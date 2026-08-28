//! What a follow-up session is started on, and where it is read back from when
//! one has to be started again.
//!
//! A follow-up is a conversation rather than a step, so nothing it is doing is
//! written down on the branch: what a session of one is *for* is the brief the
//! human steered it with, and what it has got to is the rounds they have already
//! answered. Both of those are on the Timeline — the brief as the Steer Event's
//! own body, the rounds as the Sets under it — which is the one part of a
//! follow-up that outlives the session having it.
//!
//! So a follow-up that has lost its session is picked up the way a grilling is:
//! a fresh session on the same brief, primed with what has already been said —
//! see [`crate::grillings`], whose digest this is the same digest as. A relaunch
//! that opened by asking again what the human had already answered would cost
//! them the follow-up twice.
//!
//! **This follow-up's own**, which is what the window is for. A Conversation can
//! be steered into Follow-up more than once, and the round before this one is
//! finished with: its brief was answered and its Sets belong to it. So both are
//! read from the newest steer into Follow-up down, exactly as the Nothing-else
//! mark is read inside that same window — see `store::nothing_else`.

use anyhow::Result;
use sqlx::SqlitePool;

use crate::store;

/// What a follow-up session is primed with.
///
/// One thing rather than two arguments, because the two always travel together:
/// a session is started on the brief and on whatever has been said about it
/// since, and a launch that carried one without the other would be either a
/// follow-up with no subject or one that had forgotten its own rounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FollowUp {
    /// The brief the human steered it into Follow-up with, as they wrote it.
    pub(crate) brief: String,

    /// The rounds already asked and answered inside this follow-up, as one
    /// markdown document — empty for a follow-up that is just starting, which
    /// is every one a steer launches.
    pub(crate) settled: String,

    /// Whether this is a follow-up being picked up again rather than opened.
    ///
    /// What turns on it is what the session before this one left behind. A
    /// follow-up that is being picked up had a session that died mid-round, and
    /// a Blocking Ask it left standing is a question with nobody behind it —
    /// locked as the fresh session starts, exactly as a relaunched grilling
    /// locks what its dead interview left. See [`crate::runner::following_up`].
    ///
    /// `false` where a steer opened it, which is a launch with nothing of its
    /// own behind it: what a steer displaces is displaced by the launch itself,
    /// and it is the same steer's business rather than this one's.
    ///
    /// Not read off `settled`: a session can die before its first round comes
    /// back, and that one is being picked up too.
    pub(crate) again: bool,
}

impl FollowUp {
    /// A follow-up that is starting now: the brief and nothing else.
    ///
    /// What a steer makes. There is nothing to prime it with — the steer *is*
    /// the first thing said — and a heading over an empty digest would tell the
    /// session that something had been.
    pub(crate) fn opening(brief: String) -> FollowUp {
        FollowUp {
            brief,
            settled: String::new(),
            again: false,
        }
    }
}

/// Read back what the Conversation's follow-up was opened about and what it has
/// been through, or `None` where the Timeline holds no brief to pick up.
///
/// `None` is a record that cannot be true: a steer into Follow-up is the only
/// way into the state and it is refused without a brief, so a Conversation
/// standing in Follow-up with no brief on its Timeline is one nothing can be
/// started for. The press that asked says so by name rather than starting a
/// session on nothing — see [`crate::resume`].
///
/// One read of the Timeline for both halves, as a relaunched grilling takes one
/// for its three: a Conversation on a pull request has a long Timeline behind
/// it, and picking a follow-up up again is no reason to read it twice.
pub(crate) async fn opened(pool: &SqlitePool, conversation_id: i64) -> Result<Option<FollowUp>> {
    let timeline = store::timeline(pool, conversation_id).await?;

    let Some((steered, brief)) = steered(&timeline) else {
        return Ok(None);
    };

    Ok(Some(FollowUp {
        brief: brief.to_owned(),
        // Everything answered under the steer, which is what makes these this
        // follow-up's rounds rather than the whole Conversation's: a wrap-up's
        // review, the grilling that settled the work and the round before this
        // one are all above it.
        settled: crate::grillings::settled(&timeline[steered + 1..]),
        again: true,
    }))
}

/// Where on the Timeline this follow-up was steered into being, and the brief
/// the human steered it with.
///
/// The newest, because a Conversation may have been through more than one and
/// the one it is in now is the last. Carrying a brief, because that is what a
/// session is started on: a steer without one is refused, so a Steer Event with
/// nothing under its target is a record from before Follow-up existed.
fn steered(timeline: &[store::TimelineEvent]) -> Option<(usize, &str)> {
    timeline
        .iter()
        .enumerate()
        .rev()
        .find_map(|(at, event)| match &event.event {
            store::Event::Steer(store::Lifecycle::FollowUp, Some(brief)) => {
                Some((at, brief.as_str()))
            }
            _ => None,
        })
}
