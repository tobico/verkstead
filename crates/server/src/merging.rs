//! The **Merged List**: one sidebar for the cluster, held in this device's
//! memory and ordered by **Rank** alone (ADR-0020, *The opened device relays*).
//!
//! **The hub keeps each member's list rather than fetching it per load.** What
//! it holds is that member's own `/api/ui/conversations`, read over the
//! **Relay**, and what refreshes it is that member's own Nudges. Fanning out on
//! every read was the other shape and was rejected: it costs a round trip per
//! device per refresh — and the sidebar is re-read constantly, a session
//! talking moving a badge on one row of it — and a member that is switched off
//! would stall the whole list behind a dial down its addresses.
//!
//! **When it is read** is three moments, and they are the same moment said
//! three ways: once at the start, once whenever the Nudge stream to that member
//! is taken up, and again whenever a Nudge announced under that member is one
//! the sidebar would re-read on locally. The take-up is not a case of its own —
//! [`crate::relaying::freshness`] announces [`Nudge::Everything`] under the
//! device it has just taken a stream up to, which is one of the kinds below.
//! The start is, and has to be: the streams are taken up in a task spawned
//! before this one subscribes, so the first round of take-ups is news this
//! device says to nobody.
//!
//! **Which kinds** is [`re_read`], and it is the viewer's own table said in
//! Rust: `conversations`, `conversation`, `set` and `everything` are the kinds
//! that invalidate the sidebar's list on a page, so they are the kinds that
//! refresh a member's list here. A member printing a line of transcript is not
//! one of them, and it is the kind that arrives twice a second.
//!
//! **A member that answers nothing holds whatever it last said.** A read that
//! could not be made leaves the held list exactly as it is — the rows stay on
//! the merged list, drawn dimmed by the flag the row carries — and a member
//! never reached holds nothing and contributes no rows at all. Which is the
//! whole of why the lists are held rather than asked for: there is something to
//! draw of a laptop whose lid is shut.
//!
//! **And the merge is the ranks and nothing else.** Every rank carries the
//! device that issued it, so the keys are distinct cluster-wide and the merged
//! order is total with no tiebreaker — which is what makes the list read the
//! same from every device in the cluster (ADR-0020, *Ranks*). The rank rides
//! out on the row for exactly this: see [`ConversationEntry::rank`].
//!
//! **This device's own rows carry a device too**, so that the list reads as one
//! list rather than as this device's work with somebody else's mixed in — and
//! no row carries one where there is no cluster, a lone device having nothing
//! to say about whose work it is drawing. Which of the two it is is the
//! membership's to decide and so the server's, rather than the page's.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio_stream::StreamExt;
use verkstead_render::{ConversationEntry, RowDevice};
use verkstead_schema::Nudge;
use verkstead_store::Member;

use crate::AppState;
use crate::device::Unrelayed;
use crate::relaying::{Call, Streamed};

/// The path a member's own sidebar list is read at, which is the path this
/// device serves its own at: the namespace is one router mounted twice, so the
/// list a member answers here and the list this device answers a browser are
/// the same endpoint seen from the two ends.
const LIST: &str = "/api/ui/conversations";

/// The most a member's list may be before it is dropped rather than held:
/// **four megabytes**.
///
/// A row is a branch name, a repo name, a rank and a handful of flags — some
/// two hundred bytes — so four megabytes is twenty thousand Conversations,
/// which is more than one person has. What it is for is the other case: a
/// machine on the far end of a relay that answers and then writes without
/// stopping, which this device holds one read of per member. The same bound the
/// Nudge stream takes against the same machine — see
/// [`crate::relaying::freshness`].
const MOST_A_LIST_IS: usize = 4 * 1024 * 1024;

/// The lists this device holds of its members: the last one each of them
/// answered, by **Device Id**.
///
/// **In memory rather than in the store**, because none of it is this device's
/// record: every row in here is a Conversation of somebody else's, and a table
/// of them would be a second account of work this device is not doing — one to
/// be kept true across a restart of the machine that owns it. What a restart
/// costs instead is the streams being taken up again, which reads every one of
/// these back.
///
/// A lock rather than a channel, because what the readers want is the latest
/// answer rather than the history: the refresher writes one member's list and
/// the sidebar reads the lot of them. A plain mutex rather than a read-write
/// one, because the reading writes too — it is where a departed member's rows
/// are dropped, see [`MemberLists::held_of`].
#[derive(Debug, Clone, Default)]
pub(crate) struct MemberLists(Arc<Mutex<HashMap<String, Vec<ConversationEntry>>>>);

impl MemberLists {
    /// Nothing held of anybody, which is every start and every router stood up
    /// without a cluster.
    pub(crate) fn none() -> MemberLists {
        MemberLists::default()
    }

    /// Hold what `device` just answered, replacing whatever it last said.
    ///
    /// Wholesale rather than merged, because a list is the answer to one
    /// question: a Conversation closed and swept on that machine is a row that
    /// has to go, and a merge would keep it for ever.
    fn keep(&self, device: &str, rows: Vec<ConversationEntry>) {
        self.held().insert(device.to_owned(), rows);
    }

    /// What is held of each of `members`, and nothing of anybody else.
    ///
    /// **The membership is what prunes this.** A device that has been unlinked
    /// is not a member of this cluster, so its rows are nobody's — and this is
    /// the reading that has the membership in hand, the merge being made
    /// against it. So the lists a departed device left are dropped here rather
    /// than held until the server restarts.
    fn held_of(&self, members: &[Member]) -> Vec<(Member, Vec<ConversationEntry>)> {
        let mut held = self.held();

        held.retain(|device, _| members.iter().any(|member| &member.device == device));

        members
            .iter()
            .filter_map(|member| {
                held.get(&member.device)
                    .map(|rows| (member.clone(), rows.clone()))
            })
            .collect()
    }

    /// The table itself, for the two callers above.
    ///
    /// Never held across an await, there being none to hold it across: both of
    /// them take it, do a little arithmetic over a handful of rows, and give it
    /// back.
    fn held(&self) -> MutexGuard<'_, HashMap<String, Vec<ConversationEntry>>> {
        self.0.lock().expect("nothing panics holding this")
    }
}

/// Hold every member's list fresh, for as long as this server runs.
///
/// Spawned rather than waited on, and beside the sweeps a start makes: what it
/// does first is read every member, some of which are laptops that are shut,
/// and it never returns.
///
/// Nothing at all on a router with no identity: a Verkstead stood up without a
/// Data Directory is linked to nothing and has nobody to hold a list of.
pub(crate) fn refreshing(state: &AppState) {
    if state.devices.is_none() {
        return;
    }

    let state = state.clone();

    tokio::spawn(async move { held(state).await });
}

/// The loop itself: every member read once, and then whatever a Nudge says to
/// read again.
///
/// **Subscribed before the first round of reads**, so that a Nudge landing
/// while this device is still working down its members is one it hears rather
/// than one that slips past it — the same order the Nudge stream's own endpoint
/// takes for the same reason.
async fn held(state: AppState) {
    let mut moved = state.nudges.subscribe();

    read_every_member(&state).await;

    loop {
        match moved.recv().await {
            Ok(moved) => {
                // This device's own news says nothing about anybody's list but
                // this device's, and that one is read off the store at the
                // moment the sidebar is drawn.
                let Some(device) = moved.device else { continue };

                if re_read(&moved.moved) {
                    read_member(&state, &device).await;
                }
            }

            // Fallen behind, which is a burst this device saw the middle of
            // none of: what it missed is unknowable, so every member is read
            // again. The same reaction the viewer's own stream makes of the
            // same thing, one channel further in.
            Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                tracing::debug!(
                    missed,
                    "the merged list fell behind the Nudges, so every member's list is read again",
                );
                read_every_member(&state).await;
            }

            // Nobody is announcing any more, which is a server that is going
            // away: there is nothing left for a list to be drawn on.
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        }
    }
}

/// Whether a Nudge is one the sidebar's own list is re-read on.
///
/// **The viewer's table said in Rust** — see `standsFor` in `web/src/nudge.ts`,
/// where `["conversations"]` is named by exactly these four kinds. The two
/// tables are the one decision: a kind that moves a page's list is a kind that
/// moves the list this device holds, and a kind that moves neither costs
/// nothing to say nothing about.
///
/// `everything` is in it twice over, in a sense: it is what a page reads back
/// when it cannot say what it missed, and it is what the stream to a member
/// announces the moment it is taken up — which is the read this whole module
/// waits for at every start and after every laptop that comes back.
fn re_read(moved: &Nudge) -> bool {
    matches!(
        moved,
        Nudge::Conversations | Nudge::Conversation { .. } | Nudge::Set { .. } | Nudge::Everything,
    )
}

/// Read every member's list, one after another.
///
/// Sequential rather than at once, because the lists are held rather than
/// waited on: nothing is drawn behind this, and a member that is switched off
/// costs the dial's own patience whether it is the first or the last. What it
/// buys is one relay in flight from this device at a time.
async fn read_every_member(state: &AppState) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let members = match devices.membership().rows().await {
        Ok(members) => members,
        Err(why) => {
            tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so no member's list was \
                 read for the merged list",
            );
            return;
        }
    };

    for member in members {
        read_member(state, &member.device).await;
    }
}

/// Read one member's list and hold what it said.
///
/// **A read that could not be made leaves the held list alone.** A member that
/// is switched off, a link that is down, a machine that has moved: its rows
/// stay on the merged list from the last time it answered, drawn dimmed by the
/// flag the row carries. A member never reached holds nothing and so draws
/// nothing, which is the same rule seen from its other end.
async fn read_member(state: &AppState, device: &str) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let answered = match devices.relay(device, asking()).await {
        Ok(answered) => answered,
        Err(why) => {
            tracing::debug!(
                device,
                "a member's Conversations could not be read for the merged list, so what it last \
                 said stands: {}",
                unrelayed(&why),
            );
            return;
        }
    };

    if !answered.status().is_success() {
        tracing::warn!(
            device,
            status = %answered.status(),
            "a member refused the read of its Conversations, so what it last said stands",
        );
        return;
    }

    match bounded(answered).await {
        Ok(body) => match serde_json::from_slice::<Vec<ConversationEntry>>(&body) {
            Ok(rows) => state.merged.keep(device, rows),

            // A member running a Verkstead this one cannot read the list of.
            // What it last said stands, which is the same stance a read that
            // never landed takes: this device has no better account of that
            // machine's work than the one it already holds.
            Err(why) => tracing::warn!(
                device,
                "a member answered a list of Conversations this device cannot read, so what it \
                 last said stands: {why}",
            ),
        },

        Err(why) => tracing::warn!(
            device,
            "a member's Conversations could not be read to the end: {why:#}",
        ),
    }
}

/// What a dial that was never made is said in the log as: the four findings, in
/// as many words.
///
/// The browser's own sentences for the same four are [`crate::relaying`]'s —
/// each carries a status code and names the device, those being an answer to a
/// call somebody made. This is the other reader: a line about a read nobody
/// asked for.
fn unrelayed(why: &Unrelayed) -> String {
    match why {
        Unrelayed::ThisDevice => "the id is this device's own".to_owned(),
        Unrelayed::NoSuchMember => "the device is no member of this cluster".to_owned(),
        Unrelayed::Unreachable(why) => format!("{why:#}"),
        Unrelayed::Unreadable(why) => {
            format!("the devices this one is linked to could not be read: {why:#}")
        }
    }
}

/// The call the list is read with: the member's own `/api/ui/conversations`.
///
/// The same [`Call`] a browser's relayed request is put over, because it is the
/// same dial. What is different is who asked: nobody. This is one of the two
/// calls in the namespace this device makes of its own accord — the other being
/// the Nudge stream it holds — so it carries no header of a browser's and no
/// body at all.
fn asking() -> Call {
    let mut headers = axum::http::HeaderMap::new();

    headers.insert(
        axum::http::header::ACCEPT,
        axum::http::HeaderValue::from_static("application/json"),
    );

    Call {
        method: reqwest::Method::GET,
        onwards: LIST.to_owned(),
        headers,
        body: Streamed::Nothing,
    }
}

/// What a member answered, up to [`MOST_A_LIST_IS`] of it.
///
/// Read chunk by chunk rather than in one call, so the bound is applied as the
/// bytes arrive: a body read whole and then measured is a body this device has
/// already held.
async fn bounded(answered: reqwest::Response) -> anyhow::Result<Vec<u8>> {
    let mut body = answered.bytes_stream();
    let mut held: Vec<u8> = Vec::new();

    while let Some(chunk) = body.next().await {
        held.extend_from_slice(&chunk?);

        if held.len() > MOST_A_LIST_IS {
            anyhow::bail!(
                "the list is longer than the {MOST_A_LIST_IS} bytes this device will hold of one",
            );
        }
    }

    Ok(held)
}

/// The merged list: this device's own rows and every member's, ordered by rank.
///
/// `own` is what [`crate::ui`] read off this store a moment ago, in the order
/// the store put it in. What comes back is that list where there is no cluster
/// — untouched, every row carrying no device — and the merge where there is
/// one.
///
/// **A stable sort over the ranks and nothing else.** Two ranks that are there
/// are distinct by construction, so there is nothing for a tiebreaker to do;
/// the stability is for the one case where a rank is absent, which is a
/// database the rewrite has not reached and whose rows come out first in the
/// order their own device answered them.
pub(crate) async fn merged(
    state: &AppState,
    own: Vec<ConversationEntry>,
) -> Vec<ConversationEntry> {
    let Some(devices) = state.devices.as_ref() else {
        return own;
    };

    let members = match devices.membership().rows().await {
        Ok(members) => members,

        // A membership that cannot be read is not a cluster that has dissolved,
        // and this device's own work is what it has to draw either way: the
        // sidebar answers the rows it is sure of rather than failing over the
        // ones it is not.
        Err(why) => {
            tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so the sidebar is this \
                 device's own rows alone",
            );
            return own;
        }
    };

    if members.is_empty() {
        return own;
    }

    // This device's own, which says whose they are now that there is somebody
    // else's beside them.
    let this = RowDevice {
        id: None,
        name: devices.name(),
        os: devices.os(),
        reachable: true,
    };

    let mut rows: Vec<ConversationEntry> = own
        .into_iter()
        .map(|row| ConversationEntry {
            device: Some(this.clone()),
            ..row
        })
        .collect();

    for (member, held) in state.merged.held_of(&members) {
        // The name, the OS word and whether the last dial got through are this
        // device's own readings of that machine rather than anything the list
        // said: a member answers its own Conversations and has no idea who is
        // asking, so what a row over there carried was no device at all.
        let whose = RowDevice {
            id: Some(member.device.clone()),
            name: member.name.clone(),
            os: member.os.clone(),
            reachable: member.reachable,
        };

        rows.extend(held.into_iter().map(|row| ConversationEntry {
            device: Some(whose.clone()),
            ..row
        }));
    }

    rows.sort_by(|one, other| one.rank.cmp(&other.rank));

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four kinds the sidebar's own list is re-read on, and one that is not
    /// — the kind that arrives twice a second while a session talks.
    #[test]
    fn a_members_list_is_read_again_on_what_the_sidebar_reads_on() {
        assert!(re_read(&Nudge::Conversations));
        assert!(re_read(&Nudge::Conversation { conversation: 4 }));
        assert!(re_read(&Nudge::Set { conversation: 4 }));
        assert!(re_read(&Nudge::Everything));

        assert!(!re_read(&Nudge::Transcript { conversation: 4 }));
        assert!(!re_read(&Nudge::Screen { conversation: 4 }));
        assert!(!re_read(&Nudge::Repos));
        assert!(!re_read(&Nudge::Discovered));
    }
}
