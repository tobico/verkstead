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

use crate::AppState;
use crate::answer_files::OnAnswers;
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
    /// an ask it was idling on when it went is a question with nobody behind it
    /// — locked as the fresh session starts, exactly as a relaunched grilling
    /// locks what its dead interview left. See [`crate::runner::following_up`].
    ///
    /// `false` where a steer opened it, which is a launch with nothing of its
    /// own behind it: what a steer displaces is displaced by the launch itself,
    /// and it is the same steer's business rather than this one's.
    ///
    /// Not read off `settled`: a session can die before its first round comes
    /// back, and that one is being picked up too.
    pub(crate) again: bool,

    /// Whether [`brief`] is the Conversation's own Brief rather than something
    /// the human steered it with.
    ///
    /// True for the follow-up a **Tinker** start opens, where the Brief *is* the
    /// thing to follow up on: it goes under *What I want to follow up on* and
    /// nowhere else, so the session reads it once, under the heading that says
    /// act on it. False for a steer, where the documents describe work that is
    /// already on a pull request and the brief is what the human wants taken up
    /// about it — two different things, said in two places.
    ///
    /// [`brief`]: FollowUp::brief
    pub(crate) from_the_brief: bool,
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
            from_the_brief: false,
        }
    }

    /// And the one a **Tinker** start opens, on the Conversation's own Brief.
    ///
    /// The same follow-up with one thing different: nothing has been built, so
    /// there are no documents for the session to be told the work in and the
    /// Brief is the whole of what it is being asked to take up.
    pub(crate) fn priming(brief: String) -> FollowUp {
        FollowUp {
            from_the_brief: true,
            ..FollowUp::opening(brief)
        }
    }
}

/// Read back what the Conversation's follow-up was opened about and what it has
/// been through, or `None` where the Timeline holds no brief to pick up.
///
/// **Two ways in, and the newer of them wins.** A steer is one, and a **Tinker**
/// start is the other: that one writes no steer, because it is the press that
/// starts the work rather than one that takes something up about work already
/// done — so where there is no steer into Follow-up the Brief is what the
/// follow-up is about, and the rounds are what has been answered under it.
///
/// `None` is a record that cannot be true: both ways in are refused without
/// something to start from, so a Conversation standing in Follow-up with
/// neither a steer nor a Brief on its Timeline is one nothing can be started
/// for. The press that asked says so by name rather than starting a session on
/// nothing — see [`crate::resume`].
///
/// One read of the Timeline for both halves, as a relaunched grilling takes one
/// for its three: a Conversation on a pull request has a long Timeline behind
/// it, and picking a follow-up up again is no reason to read it twice.
///
/// The state rather than the pool alone, because the digest names the files put
/// on those Answers at the path this session will open them — which is a
/// question about the Data Directory as well as about the record. See
/// [`OnAnswers`].
pub(crate) async fn opened(state: &AppState, conversation_id: i64) -> Result<Option<FollowUp>> {
    let timeline = store::timeline(&state.pool, conversation_id).await?;

    let Some((opened, brief, from_the_brief)) = steered(&timeline)
        .map(|(at, brief)| (at, brief, false))
        .or_else(|| briefed(&timeline).map(|(at, brief)| (at, brief, true)))
    else {
        return Ok(None);
    };

    Ok(Some(FollowUp {
        brief: brief.to_owned(),
        // Everything answered under whichever of the two opened it, which is
        // what makes these this follow-up's rounds rather than the whole
        // Conversation's: a wrap-up's review, the grilling that settled the work
        // and the round before this one are all above it.
        settled: crate::grillings::settled(
            &timeline[opened + 1..],
            &OnAnswers::of(state, conversation_id).await,
        ),
        again: true,
        from_the_brief,
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
            store::Event::Steer(store::Lifecycle::FollowUp, Some(brief), _) => {
                Some((at, brief.as_str()))
            }
            _ => None,
        })
}

/// And where its Brief stands, for the follow-up a **Tinker** start opened.
///
/// The newest again, and for the newest's reason: a Conversation gets one Brief
/// per round, and the one a Tinker is following up on is the one at the bottom
/// of the Timeline. Read only where there is no steer above it — a Tinker
/// steered into Follow-up a second time is having the steer's follow-up, not its
/// first one all over again.
fn briefed(timeline: &[store::TimelineEvent]) -> Option<(usize, &str)> {
    timeline
        .iter()
        .enumerate()
        .rev()
        .find_map(|(at, event)| match &event.event {
            store::Event::Brief(markdown) => Some((at, markdown.as_str())),
            _ => None,
        })
}
