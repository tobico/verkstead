//! The sockets over the hop: a Conversation terminal, a session's Screen and a
//! Code pane's watcher, held open through the device the browser opened
//! (ADR-0020, *The opened device relays*).
//!
//! **A socket is not a call, and this is where the hop stops being one.**
//! [`super`] carries a request and hands an answer back; the three attach
//! endpoints answer `101` and then the connection *is* the endpoint. So what
//! this does is put the same upgrade to the member, hand the member's own `101`
//! to the browser, and then join the two connections together — a socket on
//! each side of this device, living and dying as one.
//!
//! **Byte for byte rather than frame for frame.** Nothing here knows what a
//! WebSocket frame is: the browser's bytes are copied to the member and the
//! member's back to the browser, and that is the whole of it. Two things came
//! of weighing the other shape — a WebSocket client half in the binary, reading
//! frames off one side and writing them onto the other. It would put a second
//! protocol implementation in the tree for traffic this device has no business
//! reading; and a frame re-written by this hop is a frame this hop can get
//! wrong, where a byte copied cannot. What a terminal carries is a human's
//! keystrokes and what the shell printed, and neither is any of this device's
//! concern.
//!
//! **Which is also why this device signs nothing.** The browser's
//! `Sec-WebSocket-Key` crosses with everything else, so the
//! `Sec-WebSocket-Accept` the browser checks is the one the *member* computed —
//! the handshake is between the two ends, and this hop is in the middle of it
//! rather than a party to it. The same goes for a subprotocol and for an
//! extension nobody here has heard of: what the member agreed to is what the
//! browser is told.
//!
//! **Both closes cross**, which is the load-bearing half of this module. A
//! browser that goes away — a tab shut, a laptop whose lid came down mid-edit —
//! has to take the far socket with it, or a file watcher goes on running on the
//! other machine over a pane nobody is looking at. And a member that goes away
//! has to close the browser's, or the pane sits attached to nothing with no way
//! to find out. The watcher is what makes this worth spelling out: it is the one
//! attachment with a cost on the other machine and no traffic to notice its
//! silence by. See [`crossing`], which is where either end going ends both.
//!
//! **And a refusal at the far end is still a refusal.** A terminal number that
//! is not live and a Conversation that device has no record of are answered as
//! they are locally — the member says what it says, this hop hands it back
//! untouched, and the browser's dial fails rather than opening onto a socket
//! that says nothing. Nothing is upgraded on this side until the member has
//! upgraded on that one.

use axum::http::header::{CONNECTION, UPGRADE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use verkstead_schema::ApiError;

use crate::ui::refused;

/// Whether the browser is asking for the protocol to change rather than making
/// a call — which is every one of the three attach endpoints, and nothing else
/// in the namespace.
///
/// Read the way [`axum::extract::ws::WebSocketUpgrade`] reads it, because it is
/// the extractor at the far end of this hop: `Connection` naming `upgrade`
/// among its tokens, and an `Upgrade` header to say what to. Which protocol is
/// not this hop's business — the member is the one that has to agree to it — so
/// the value itself is only carried.
pub(super) fn upgrading(headers: &HeaderMap) -> bool {
    headers.contains_key(UPGRADE) && names_upgrade(headers)
}

/// `Connection` naming `upgrade`, which is a list: a browser writes
/// `Connection: Upgrade` and a proxy in front of one may write
/// `keep-alive, Upgrade`.
fn names_upgrade(headers: &HeaderMap) -> bool {
    headers
        .get_all(CONNECTION)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
}

/// The browser's half of the hop, taken out of the connection.
///
/// **Taken before the request goes anywhere**, and taken whether or not the
/// member turns out to answer: this is a fact about the socket the browser is
/// on, and the moment to find out that there is no upgrading it is before a
/// second connection exists to leak. Where the member refuses, it is dropped
/// unused and the browser gets the refusal.
///
/// `None` is a connection no upgrade can be had over, which on this listener is
/// something no browser does — the workbench is HTTP/1.1 to the browser and to
/// the `tailscale serve` in front of it alike. Said as a refusal rather than
/// ignored, for the reason axum's own extractor says it: a socket that opened
/// and carried nothing would be a worse way to find out.
pub(super) fn taken(extensions: &mut axum::http::Extensions) -> Option<OnUpgrade> {
    extensions.remove::<OnUpgrade>()
}

/// And what is said where there is none.
pub(super) fn not_upgradable() -> Response {
    refused(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiError::new(
            "this connection cannot be upgraded, so a member's socket cannot be relayed over it",
        ),
    )
}

/// The headers an upgrade travels to the member under: [`super::forwarded`]'s,
/// with the two that say it is one put back.
///
/// `Connection` and `Upgrade` are hop-by-hop headers, and [`super::KEPT_BACK`]
/// holds every one of them back for the reason it gives — forwarding one would
/// be describing this hop's socket to a machine on the other side of another.
/// These two are the exception, and they are the exception because here they are
/// not describing this hop's socket: they *are* the request. A member that got
/// them stripped would see an ordinary `GET` of an attach endpoint, and answer
/// it the way axum answers one — a `400` about a header that never arrived.
///
/// Carried as the browser wrote them rather than re-spelled, which is this
/// hop's whole promise. A `Connection: keep-alive, Upgrade` is passed on entire:
/// the far end reads that list the same way [`names_upgrade`] does, and a value
/// this device tidied would be a value this device could tidy wrongly.
pub(super) fn asking(headers: &HeaderMap) -> HeaderMap {
    let mut asking = super::forwarded(headers);

    for name in [CONNECTION, UPGRADE] {
        for value in headers.get_all(&name) {
            asking.append(name.clone(), value.clone());
        }
    }

    asking
}

/// The member's answer to an upgrade, as this device's own: either the `101`
/// that opens a socket, or whatever else it said.
///
/// **A `101` is handed back headers and all**, `Connection` and `Upgrade`
/// included — the two this hop strips from an ordinary answer, and for the
/// mirror of the reason [`asking`] puts them back. Here they are the answer
/// rather than a description of the member's socket, and the browser's own
/// client is waiting to read them. Nothing is re-framed either: a `101` carries
/// no body and no length, so the whole of the member's header map crosses as it
/// stands, `Sec-WebSocket-Accept` and any subprotocol with it.
///
/// **And anything else is an ordinary answer.** A refusal is what the browser
/// asked for, so it gets the member's own — see this module's documentation. The
/// browser's upgrade is dropped unused, which leaves the connection exactly what
/// it was: one that answered a request and may answer another.
pub(super) fn bridged(taking: OnUpgrade, answered: reqwest::Response) -> Response {
    if answered.status() != StatusCode::SWITCHING_PROTOCOLS {
        return super::handed_back(answered);
    }

    // Built before the answer is moved into the bridge below, and built from the
    // answer's own headers: see above.
    let switching = (StatusCode::SWITCHING_PROTOCOLS, answered.headers().clone()).into_response();

    // **A task rather than an await**, because the browser's half cannot be had
    // until the `101` below has reached it: hyper hands an upgraded connection
    // over once the response it answers is written, so a hop that waited here
    // would be waiting on something this return is what causes. The member's
    // half is waited for in the same place for symmetry rather than necessity.
    tokio::spawn(async move {
        let far = match answered.upgrade().await {
            Ok(far) => far,
            Err(why) => {
                tracing::debug!(
                    %why,
                    "a member answered a relayed socket and then would not hand the \
                     connection over, so nothing was bridged",
                );
                return;
            }
        };

        let near = match taking.await {
            Ok(near) => TokioIo::new(near),
            Err(why) => {
                tracing::debug!(
                    %why,
                    "a browser's half of a relayed socket could not be taken, so the \
                     member's is let go of",
                );
                return;
            }
        };

        crossing(near, far).await;
    });

    switching
}

/// The two connections joined, until either end has finished with the other.
///
/// **Whichever direction ends, ends both.** The copies are raced rather than
/// waited on together: a read that reached the end of one side is that side
/// gone, and what this module exists to guarantee is that the other side hears
/// about it. Waiting for both to finish would leave a half-closed browser
/// holding a watcher open on another machine, which is the exact cost *Both
/// closes have to cross* is about.
///
/// **And the ending is a shutdown rather than a drop**, on both sides. The one
/// whose read ended has nothing to hear it; the other is a live connection with
/// a socket at the end of it, and a write half shut down is a `FIN` it reads as
/// the end — which is what a browser's WebSocket client turns into a close, and
/// what lets go of a watcher at the far end. Dropping alone would do it too, but
/// only as whatever the operating system makes of an abandoned socket.
///
/// What is *in* the bytes is nobody's business here, a close frame included: the
/// direction that ended is the direction that delivered it, so a member that
/// closed its socket politely has already had its frame copied by the time this
/// tears the pair down.
async fn crossing(
    near: impl AsyncRead + AsyncWrite + Send + 'static,
    far: impl AsyncRead + AsyncWrite + Send + 'static,
) {
    let (mut from_browser, mut to_browser) = tokio::io::split(near);
    let (mut from_member, mut to_member) = tokio::io::split(far);

    let ended = tokio::select! {
        ended = tokio::io::copy(&mut from_browser, &mut to_member) => ended,
        ended = tokio::io::copy(&mut from_member, &mut to_browser) => ended,
    };

    if let Err(why) = ended {
        // Ordinary rather than exceptional: a browser that went away without
        // saying so is a reset read off one of the two, and it is exactly the
        // ending this races for.
        tracing::debug!(%why, "a relayed socket ended without being closed");
    }

    let _ = to_member.shutdown().await;
    let _ = to_browser.shutdown().await;
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;
    use tokio::io::{AsyncReadExt, DuplexStream};

    use super::*;

    /// How much each of the two in-memory pipes below will hold at once.
    ///
    /// Small on purpose, and smaller than what the tests push through it: what
    /// they are about is a *stream* rather than a message, so the copy has to
    /// go round more than once for the claim to be worth anything.
    const PIPE: usize = 8 * 1024;

    /// And how much is pushed through it, which is past that several times over.
    const PLENTY: usize = 96 * 1024;

    /// The browser's own headers, as a dial for one of the three attach
    /// endpoints arrives with them.
    fn dialling() -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert("connection", HeaderValue::from_static("Upgrade"));
        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        headers.insert("sec-websocket-version", HeaderValue::from_static("13"));
        headers.insert(
            "sec-websocket-key",
            HeaderValue::from_static("dGhlIHNhbXBsZSBub25jZQ=="),
        );
        headers.insert("cookie", HeaderValue::from_static("workbench_key=secret"));

        headers
    }

    /// A socket is told from a call by the two headers, and either of them alone
    /// is not one.
    #[test]
    fn a_socket_is_told_from_a_call_by_the_two_headers() {
        assert!(upgrading(&dialling()), "a browser dialling an attach");

        let mut headers = HeaderMap::new();
        assert!(!upgrading(&headers), "an ordinary read");

        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        assert!(
            !upgrading(&headers),
            "a header nothing asked the connection to act on is not an upgrade",
        );

        headers.insert("connection", HeaderValue::from_static("keep-alive"));
        assert!(!upgrading(&headers), "and nor is a connection kept alive");
    }

    /// And `Connection` is a list, whatever wrote it.
    #[test]
    fn a_connection_naming_more_than_one_thing_still_names_the_upgrade() {
        let mut headers = HeaderMap::new();

        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        headers.insert(
            "connection",
            HeaderValue::from_static("keep-alive, Upgrade"),
        );

        assert!(upgrading(&headers));
    }

    /// The two headers that say it is a socket travel, where the ordinary hop
    /// holds both of them back — and the Workbench Key's cookie still does not.
    #[test]
    fn the_headers_that_say_it_is_a_socket_travel() {
        let asking = asking(&dialling());

        assert_eq!(asking.get("connection").unwrap(), "Upgrade");
        assert_eq!(asking.get("upgrade").unwrap(), "websocket");
        assert_eq!(
            asking.get("sec-websocket-key").unwrap(),
            "dGhlIHNhbXBsZSBub25jZQ==",
            "the key the member's accept is computed from is the browser's own",
        );
        assert!(
            !asking.contains_key("cookie"),
            "this device's Workbench Key must not reach a member, socket or no socket",
        );

        assert!(
            !super::super::forwarded(&dialling()).contains_key("connection"),
            "an ordinary call still holds the connection's own headers back",
        );
    }

    /// And `Connection` crosses as it was written rather than tidied.
    #[test]
    fn a_connection_header_crosses_as_it_was_written() {
        let mut headers = dialling();
        headers.insert(
            "connection",
            HeaderValue::from_static("keep-alive, Upgrade"),
        );

        assert_eq!(
            asking(&headers).get("connection").unwrap(),
            "keep-alive, Upgrade",
        );
    }

    /// Two ends joined by [`crossing`], as the two ends of a relayed socket:
    /// what the *browser* holds, and what the *member* holds.
    ///
    /// In-memory pipes rather than sockets, because what [`crossing`] promises
    /// is about the two streams it was handed and nothing about how they were
    /// got: bytes across and an end that crosses. That the streams come off a
    /// real upgrade and a real dial is `tests/bridging.rs`'s subject, over a
    /// real member.
    fn joined() -> (DuplexStream, DuplexStream) {
        let (browser, near) = tokio::io::duplex(PIPE);
        let (far, member) = tokio::io::duplex(PIPE);

        tokio::spawn(crossing(near, far));

        (browser, member)
    }

    /// Bytes to push through it: distinct all the way along, so that a copy
    /// that repeated or dropped a stretch of them is a failure rather than a
    /// coincidence.
    fn plenty(from: u8) -> Vec<u8> {
        (0..PLENTY)
            .map(|at| from.wrapping_add((at % 251) as u8))
            .collect()
    }

    /// What a browser types reaches the member, and what the member prints
    /// reaches the browser — byte for byte, and in more than one read's worth.
    ///
    /// Which is the whole of what a terminal asks of this hop: a shell inside the
    /// far Sandbox is the one socket carrying traffic both ways, and a Screen is
    /// the same machinery pointed at an agent.
    #[tokio::test]
    async fn bytes_cross_both_ways_and_arrive_as_they_were_sent() {
        let (browser, member) = joined();

        let (mut browser_reads, mut browser_writes) = tokio::io::split(browser);
        let (mut member_reads, mut member_writes) = tokio::io::split(member);

        let typed = plenty(1);
        let printed = plenty(128);

        let mut heard = vec![0; PLENTY];
        let mut shown = vec![0; PLENTY];

        // All four at once: the pipes hold a fraction of this each, so a write
        // waited out before its read would wedge rather than fail.
        tokio::try_join!(
            async {
                browser_writes.write_all(&typed).await?;
                browser_writes.flush().await
            },
            async {
                member_writes.write_all(&printed).await?;
                member_writes.flush().await
            },
            async { member_reads.read_exact(&mut heard).await.map(|_| ()) },
            async { browser_reads.read_exact(&mut shown).await.map(|_| ()) },
        )
        .expect("both ends should have said what they said and heard what was said");

        assert_eq!(heard, typed, "the member heard something else");
        assert_eq!(shown, printed, "the browser was shown something else");
    }

    /// A browser that goes away takes the member's socket with it.
    ///
    /// The load-bearing one: a Code pane's watcher costs the other machine a
    /// watch per Worktree and says nothing down its socket, so a close that did
    /// not cross would leave one running over a pane nobody is looking at.
    #[tokio::test]
    async fn a_browser_that_goes_away_closes_the_members_socket() {
        let (browser, mut member) = joined();

        drop(browser);

        assert_eq!(
            member.read(&mut [0; 1]).await.unwrap(),
            0,
            "the member's end should have been closed behind the browser",
        );
    }

    /// And a member that goes away closes the browser's, so the pane is not left
    /// attached to nothing with no way to find out.
    #[tokio::test]
    async fn a_member_that_goes_away_closes_the_browsers_socket() {
        let (mut browser, member) = joined();

        drop(member);

        assert_eq!(
            browser.read(&mut [0; 1]).await.unwrap(),
            0,
            "the browser's end should have been closed behind the member",
        );
    }

    /// And what the member said on its way out is said before the end crosses,
    /// rather than dropped along with the socket.
    ///
    /// Which is what makes a close *frame* reach the browser: it is bytes down
    /// the same stream, so the direction that ended is the direction that
    /// delivered it.
    #[tokio::test]
    async fn what_was_said_before_an_end_arrives_ahead_of_it() {
        let (mut browser, mut member) = joined();

        member.write_all(b"goodbye").await.unwrap();
        member.flush().await.unwrap();
        drop(member);

        let mut last = [0; 7];
        browser.read_exact(&mut last).await.unwrap();

        assert_eq!(&last, b"goodbye");
        assert_eq!(
            browser.read(&mut [0; 1]).await.unwrap(),
            0,
            "and then the end of it",
        );
    }
}
