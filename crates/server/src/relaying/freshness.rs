//! The news coming the other way: one Nudge stream held to each member, and
//! everything down it announced here under the device it came from (ADR-0020,
//! *The opened device relays*).
//!
//! **The last thing between a remote Conversation and a local one.** A page on
//! `/devices/{device}/conversations/{id}` reads the far end through the hop in
//! [`super`] and draws what it says — and then goes stale, because nothing over
//! here had heard that a Set had been answered on that machine or that a session
//! had printed another line. This is what tells it: a member's own
//! `/api/ui/nudges` read as a stream, and each Nudge off it announced on the one
//! this device's pages are already listening to, with the member's Device Id on
//! it — see [`crate::nudge`], which is both ends of that, and [`Nudged`], which
//! is the Nudge and the device together.
//!
//! **Held by the server rather than by the browser**, which is what makes one
//! connection per member serve every page this device has open — the phone on
//! the tailnet included, which is not on the member's network at all and could
//! not hold a stream to it if it tried. It is also why the count is one per
//! member rather than one per member per tab: a browser gets six connections to
//! an origin, and a stream per device per page would spend them on news.
//!
//! **Reconnecting is the behaviour rather than an error path.** A member
//! restarts, a lid shuts, a link drops: the stream is taken up again, and one
//! that has just come back knows nothing about what it missed — so what it
//! announces first is [`Nudge::Everything`] under that device, which is *read
//! back whatever of this member is on screen*. The same reaction the browser's
//! own stream makes of a reconnect, aimed at one device's queries.
//!
//! **And a member that is not answering is not drawn about.** Nothing is
//! announced for a dial that reached nowhere: the page keeps what it last read
//! and goes stale, exactly as it does when its own stream is down, and a press
//! on it is refused by name by the hop. The row dimmed *unreachable* is the
//! dial's own doing — see [`Peers::relay`] — and the merged list that keeps a
//! member's last rows is stage 06's.
//!
//! **The membership is read again rather than held**, for the reason every other
//! reader of it does: a device linked while this server is up is one to hold a
//! stream to, and one unlinked is one to let go of. So the streams are kept
//! against the rows — a task apiece, keyed by Device Id — and the rows are read
//! every [`LOOKING_AGAIN`].

use std::collections::HashMap;
use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue, header};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use verkstead_schema::Nudge;

use crate::nudge::Nudges;
use crate::peer::Members;
use crate::peer::dialling::Peers;
use crate::relaying::{Call, Streamed};

/// What a member serves its news on, which is the same path this device serves
/// its own browser's stream at: the namespace is one router mounted twice, so
/// the stream a member holds to this device and the stream this device holds to
/// a member are the same endpoint seen from the two ends.
const STREAM: &str = "/api/ui/nudges";

/// How often the membership is read again, so that a device linked while this
/// server is up gets a stream and one unlinked loses it.
///
/// Ten seconds. A membership that moved is a handful of rows in this device's own
/// database and reading them is a `SELECT`, so what this interval costs is
/// nothing beside what it decides; what it buys is a device let in on the
/// Remote access pane drawing news within ten seconds of being let in, rather
/// than on the next restart. Nothing waits on it — the streams are held rather
/// than polled, and what arrives down one arrives the moment it is said.
const LOOKING_AGAIN: Duration = Duration::from_secs(10);

/// And how long before a stream that has ended is taken up again.
///
/// Five seconds. It is what a member that has just restarted or moved costs — a
/// dial down its addresses, and then this — and it is short enough that the page
/// in front of the human is only ever a few seconds behind a machine that is
/// there at all.
const AGAIN: Duration = Duration::from_secs(5);

/// And the longest that wait grows to, doubling for as long as nothing answers:
/// **a minute**.
///
/// **Because a member that is not there costs something every time it is
/// dialled.** A walk down its addresses, and a line in the log and a write to its
/// row saying it is unreachable — see [`Peers::relay`]. A laptop is shut for a
/// fortnight, and [`AGAIN`] over a fortnight is a quarter of a million dials and
/// as many lines about the same machine.
///
/// A minute, because what the wait costs is how late the news is when that
/// machine comes back: a page opened on a member reads it whole when it is drawn
/// and again whenever the human looks away and back, so a minute is the longest
/// the *stream* is missing rather than the longest anything is stale. The wait
/// goes back to [`AGAIN`] the moment a stream is held at all, so a member that is
/// there and merely restarting never sees this.
const AT_MOST: Duration = Duration::from_secs(60);

/// The most one frame off a member's stream may be before the stream is let go
/// of: **64 KiB**.
///
/// A Nudge is a kind, an integer and now a Device Id — some fifty bytes — so no
/// Verkstead ever comes near this. What it is for is the other case: a stream
/// that answers and then writes without ever ending a frame is a read with no
/// end and nothing to stop it, and this device holds one of these per member.
/// The same bound the outbound half of a discovery takes against a machine that
/// answers slowly and endlessly — see [`crate::peer::dialling::MOST_SAID`].
const MOST_A_FRAME_IS: usize = 64 * 1024;

/// Hold a Nudge stream to every member, for as long as this server runs.
///
/// Never returns: it is spawned at the start beside the certificate's own
/// catching-up, and the serve it is spawned from never returns either.
///
/// **A task per member and a loop over the membership.** Each task holds its own
/// member's stream and takes it up again when it ends — see [`listening`] — so
/// what is left here is bookkeeping: a row with no task gets one, and a task
/// whose row is gone is dropped. A task that has finished on its own is a member
/// whose row it could not read, and it is started again on the next pass.
pub(crate) async fn held(members: Members, peers: Peers, nudges: Nudges) {
    let mut holding: HashMap<String, JoinHandle<()>> = HashMap::new();

    loop {
        match members.rows().await {
            Ok(rows) => {
                for member in &rows {
                    let already = holding
                        .get(&member.device)
                        .is_some_and(|holder| !holder.is_finished());

                    if already {
                        continue;
                    }

                    holding.insert(
                        member.device.clone(),
                        tokio::spawn(listening(
                            member.device.clone(),
                            members.clone(),
                            peers.clone(),
                            nudges.clone(),
                        )),
                    );
                }

                // And the ones that are nobody's any more. An unlinked device is
                // not a member of this cluster, so news of it is news about
                // nothing: the stream goes, and the pages keep whatever they
                // last read of it until they are drawn again.
                holding.retain(|device, holder| {
                    let still = rows.iter().any(|member| &member.device == device);

                    if !still {
                        holder.abort();
                    }

                    still
                });
            }

            // Which leaves the streams that are up alone: they are held against
            // members this device has already read, and a database that cannot
            // be read is not a cluster that has dissolved.
            Err(why) => tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so no stream was taken up \
                 or let go of on this pass",
            ),
        }

        tokio::time::sleep(LOOKING_AGAIN).await;
    }
}

/// Hold one member's stream, taking it up again every time it ends.
///
/// Ends only where that device is no longer a member — the one thing that makes
/// a stream to it nobody's — so the task outlives a laptop that is shut, a
/// member that is upgrading and a link that drops, which is the whole point of
/// it.
///
/// **The wait between attempts doubles while nothing answers**, up to
/// [`AT_MOST`], and goes back to [`AGAIN`] the moment a stream is held at all: a
/// member that is there costs the short wait and a member that is switched off
/// costs the long one. See the two constants, which is where the reasoning is.
async fn listening(device: String, members: Members, peers: Peers, nudges: Nudges) {
    let mut waiting = AGAIN;

    loop {
        match read(&device, &members, &peers, &nudges).await {
            // Held and then ended, so the machine is there and answering:
            // whatever ended the stream is as likely to be over as not, and the
            // next one is taken up at the soonest.
            Held::Ended => waiting = AGAIN,

            // Nothing answered at all, so the wait it was dialled after is the
            // one that doubles below.
            Held::NotTakenUp => {}

            Held::NoLongerAMember => return,
        }

        tokio::time::sleep(waiting).await;

        waiting = (waiting * 2).min(AT_MOST);
    }
}

/// Why a member's stream is not being read any more.
enum Held {
    /// It was taken up and then ended: the link dropped under it, the member
    /// restarted, or it said something that is not a Nudge stream at all.
    Ended,

    /// It was never taken up, because nothing answered: the member is switched
    /// off, its addresses are all stale, or it refused the read.
    NotTakenUp,

    /// And the one ending that is final: the row is gone, so there is no member
    /// to hold a stream to and nothing its news would be about.
    NoLongerAMember,
}

/// Take up `device`'s stream and read it until it ends.
///
/// **The row is read afresh on every attempt** rather than held from the one
/// that started the task: a member moves between the LAN and the tailnet, and
/// re-issues the certificate a dial is pinned on, and a stream taken up against
/// what that row said an hour ago would be one that could never come back.
async fn read(device: &str, members: &Members, peers: &Peers, nudges: &Nudges) -> Held {
    let member = match members.rows().await {
        Ok(rows) => rows.into_iter().find(|member| member.device == device),

        // Said and tried again: a database that could not be read says nothing
        // about whether this is still a member.
        Err(why) => {
            tracing::error!(
                device,
                error = ?why,
                "the devices this one is linked to could not be read to take up a member's \
                 Nudge stream",
            );
            return Held::NotTakenUp;
        }
    };

    let Some(member) = member else {
        tracing::info!(
            device,
            "the device is no longer a member of this cluster, so its Nudge stream is let go of",
        );
        return Held::NoLongerAMember;
    };

    let answered = match peers.relay(&member, &asking()).await {
        Ok(answered) => answered,

        // Which is a member that is off, a link that is down, or a machine that
        // has moved: nothing is announced for it, and the pages go stale the way
        // they do when their own stream is down. The row is drawn unreachable by
        // the dial itself.
        Err(why) => {
            tracing::debug!(
                device,
                "a member's Nudge stream could not be taken up, and is tried again: {why:#}",
            );
            return Held::NotTakenUp;
        }
    };

    if !answered.status().is_success() {
        tracing::warn!(
            device,
            status = %answered.status(),
            "a member refused the Nudge stream this device holds to it",
        );
        return Held::NotTakenUp;
    }

    // Taken up. What this device missed while it was not listening is unknowable
    // — the far end replays nothing, and has no idea anybody was away — so the
    // first thing said under this device is the widest thing there is to say,
    // and a page drawing the member reads back what it is showing of it.
    //
    // Said on every take-up rather than only on a second one. The browser's own
    // stream tells a first open from a reconnect because the page had just read
    // the world it opened the stream over; this end knows nothing about when any
    // page last read anything, and a Nudge that reached nobody costs nothing.
    nudges.announce_of(&member.device, Nudge::Everything);

    tracing::info!(
        device,
        "this device is holding the member's Nudge stream, and announcing what comes down it \
         under its Device Id",
    );

    let mut frames = Frames::new();
    let mut body = answered.bytes_stream();

    while let Some(chunk) = body.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(why) => {
                tracing::debug!(
                    device,
                    "a member's Nudge stream stopped partway through, and is taken up again: \
                     {why:#}",
                );
                return Held::Ended;
            }
        };

        match frames.fed(&chunk) {
            Some(said) => {
                for moved in said {
                    nudges.announce_of(&member.device, moved);
                }
            }

            None => {
                tracing::warn!(
                    device,
                    most = MOST_A_FRAME_IS,
                    "a member wrote more than one frame's worth of bytes without ending one, so \
                     its Nudge stream is let go of",
                );
                return Held::Ended;
            }
        }
    }

    tracing::info!(
        device,
        "a member's Nudge stream ended, and is taken up again",
    );

    Held::Ended
}

/// The call the stream is taken up with: the member's own `/api/ui/nudges`, read
/// rather than held.
///
/// The same [`Call`] a browser's relayed request is put over, because it is the
/// same dial — this device's certificate presented, the member's fingerprint
/// pinned, and every address it advertised tried in order. What is different is
/// who asked: nobody. This is the one call in the namespace this device makes of
/// its own accord, which is why it carries no header of a browser's and no body
/// at all.
fn asking() -> Call {
    let mut headers = HeaderMap::new();

    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static("text/event-stream"),
    );

    Call {
        method: reqwest::Method::GET,
        onwards: STREAM.to_owned(),
        headers,
        body: Streamed::Nothing,
    }
}

/// A member's stream, read frame by frame out of the bytes as they arrive.
///
/// **Bytes rather than text**, because a chunk boundary falls wherever the
/// network put it and may cut a character in half; a whole frame is text, and
/// that is where this reads one.
struct Frames {
    /// What has arrived and does not end a frame yet.
    buffered: Vec<u8>,
}

impl Frames {
    fn new() -> Frames {
        Frames {
            buffered: Vec::new(),
        }
    }

    /// Whatever whole frames `chunk` completes, as the Nudges they said — or
    /// `None` where the buffer has grown past [`MOST_A_FRAME_IS`], which is a
    /// stream to let go of rather than a frame to read.
    ///
    /// **A frame that says nothing this device can read is one Nudge all the
    /// same**, and the widest one: [`Nudge::Everything`]. A member newer than
    /// this one has kinds this build has never heard of, and what this end does
    /// about a kind it cannot name is what the viewer does about one — read back
    /// everything of that device — rather than drop news on the floor. The
    /// keep-alives are not that and are skipped: a comment is a frame with
    /// nothing in it, and the far end writes one every fifteen seconds.
    fn fed(&mut self, chunk: &[u8]) -> Option<Vec<Nudge>> {
        self.buffered.extend_from_slice(chunk);

        let mut said = Vec::new();

        while let Some(end) = ends(&self.buffered) {
            let frame: Vec<u8> = self.buffered.drain(..end).collect();

            if let Some(moved) = read_frame(&frame) {
                said.push(moved);
            }
        }

        if self.buffered.len() > MOST_A_FRAME_IS {
            return None;
        }

        Some(said)
    }
}

/// Where the first whole frame in `buffered` ends, past the blank line that ends
/// it.
fn ends(buffered: &[u8]) -> Option<usize> {
    buffered
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .map(|at| at + 2)
}

/// What one frame said, where it said a Nudge at all.
///
/// The event is read as well as the data, for the reason the viewer reads it:
/// the name is what keeps whatever else may one day come down this stream from
/// being taken for a Nudge by a reader too old to know about it. Bytes that are
/// not text at all are not a frame this can judge and are skipped like a
/// keep-alive — a *kind* it cannot name is the other thing, and is the widest
/// Nudge there is.
fn read_frame(frame: &[u8]) -> Option<Nudge> {
    let frame = std::str::from_utf8(frame).ok()?;

    if !frame.lines().any(|line| line.trim_end() == "event: nudge") {
        return None;
    }

    let data = frame.lines().find_map(|line| line.strip_prefix("data: "))?;

    match serde_json::from_str::<Nudge>(data) {
        Ok(moved) => Some(moved),

        // A kind this build has never heard of, which is what a member running a
        // newer Verkstead says — so the widest reaction there is, aimed at that
        // device. See [`Frames::fed`].
        Err(why) => {
            tracing::debug!(
                "a member said something this device cannot read as a Nudge, so everything of \
                 it is announced instead: {data:?} — {why}",
            );
            Some(Nudge::Everything)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One frame at a time, as the far end writes them.
    #[test]
    fn a_frame_is_read_as_the_nudge_it_said() {
        let mut frames = Frames::new();

        assert_eq!(
            frames.fed(b"event: nudge\ndata: {\"kind\":\"set\",\"conversation\":4}\n\n"),
            Some(vec![Nudge::Set { conversation: 4 }]),
        );
    }

    /// And a frame split across two chunks is one frame: what arrives is
    /// whatever the network handed over, and a Nudge cut in half is still one
    /// Nudge.
    #[test]
    fn a_frame_split_across_chunks_is_read_whole() {
        let mut frames = Frames::new();

        assert_eq!(
            frames.fed(b"event: nudge\ndata: {\"kind\":\"scr"),
            Some(vec![])
        );
        assert_eq!(
            frames
                .fed(b"een\",\"conversation\":7}\n\nevent: nudge\ndata: {\"kind\":\"repos\"}\n\n"),
            Some(vec![Nudge::Screen { conversation: 7 }, Nudge::Repos]),
        );
    }

    /// The keep-alive is the stream's other traffic and says nothing: a comment
    /// every fifteen seconds, which must not be announced as anything.
    #[test]
    fn a_keep_alive_says_nothing() {
        let mut frames = Frames::new();

        assert_eq!(frames.fed(b":\n\n"), Some(vec![]));
    }

    /// A kind this build has never heard of is the widest Nudge there is rather
    /// than news dropped: a member may be running a newer Verkstead than this
    /// one.
    #[test]
    fn a_kind_this_device_does_not_know_is_everything_of_that_device() {
        let mut frames = Frames::new();

        assert_eq!(
            frames.fed(b"event: nudge\ndata: {\"kind\":\"whatever-comes-next\"}\n\n"),
            Some(vec![Nudge::Everything]),
        );
    }

    /// And a member that writes without ever ending a frame is let go of rather
    /// than read into this device's memory.
    #[test]
    fn a_frame_that_never_ends_lets_the_stream_go() {
        let mut frames = Frames::new();
        let endless = vec![b'x'; MOST_A_FRAME_IS + 1];

        assert_eq!(frames.fed(b"event: nudge\n"), Some(vec![]));
        assert_eq!(frames.fed(&endless), None);
    }
}
