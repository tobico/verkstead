//! What a follow-up session is primed with when one has to be started again.
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
//! finished with: its brief was answered and its Sets belong to it. So the brief
//! is read off the newest way *into* Follow-up — the steer where one opened it,
//! and the Brief itself where a **Tinker** start did — and the rounds from the
//! newest move into Follow-up down, which is the window the Nothing-else mark is
//! read inside as well; see `store::nothing_else` and [`window`].

use anyhow::Result;

use crate::AppState;
use crate::answer_files::OnAnswers;
use crate::store;

/// The rounds already asked and answered inside the Conversation's follow-up,
/// as one markdown document, or `None` where the Timeline holds no brief to
/// follow up on.
///
/// What the **Follow-up rounds** Activity is handed as its `settled` option —
/// see [`crate::activities::follow_up_rounds::run`] — and so what a fresh
/// session on a follow-up picked up again is primed with. Empty for a follow-up
/// that is just starting, which is every one a steer or a Tinker start opens.
/// The component reads the brief itself, off the same Timeline; what it cannot
/// make there is this digest, which needs each Set's schema and the Data
/// Directory's paths.
///
/// **Two ways in, and the newer of them wins.** A steer is one, and a **Tinker**
/// start is the other: that one writes no steer, because it is the press that
/// starts the work rather than one that takes something up about work already
/// done — so where there is no steer into Follow-up the Brief is what the
/// follow-up is about, and the rounds are what has been answered since the move
/// that press wrote.
///
/// `None` is a record that cannot be true: both ways in are refused without
/// something to start from, so a Conversation standing in Follow-up with
/// neither a steer nor a Brief on its Timeline is one nothing can be started
/// for. The press that asked says so by name rather than starting a session on
/// nothing — see [`crate::resume`].
///
/// The state rather than the pool alone, because the digest names the files put
/// on those Answers at the path this session will open them — which is a
/// question about the Data Directory as well as about the record. See
/// [`OnAnswers`].
pub(crate) async fn settled(state: &AppState, conversation_id: i64) -> Result<Option<String>> {
    let timeline = store::timeline(&state.pool, conversation_id).await?;

    let Some(opened) = steered(&timeline)
        .or_else(|| briefed(&timeline))
        .map(|(at, _)| at)
    else {
        return Ok(None);
    };

    // Everything answered since this follow-up was last entered, which is what
    // makes these its own rounds rather than the whole Conversation's: a
    // wrap-up's review, the grilling that settled the work and the round before
    // this one are all above it. See [`window`], which is why that is not the
    // same event as the one the brief was read off.
    Ok(Some(crate::grillings::settled(
        &timeline[window(&timeline, opened) + 1..],
        &OnAnswers::of(state, conversation_id).await,
    )))
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

/// And the other way in: the move a **Tinker** start wrote, with the Brief that
/// is what its follow-up is about.
///
/// **Two events rather than one**, because the two say different things. The
/// Brief is the subject — the newest, for the newest's reason: a Conversation
/// gets one Brief per round, and the one a Tinker is following up on is the one
/// at the bottom of the Timeline. The move into Follow-up is where the rounds
/// start being this follow-up's, which is the same place `store::nothing_else`
/// opens its window at: the Brief was written while the Conversation was still a
/// Draft, so counting from it would take in whatever stood between the human
/// writing it and the press that started the work.
///
/// Read only where there is no steer above them — a Tinker steered into Follow-up
/// a second time is having the steer's follow-up, not its first one all over
/// again.
fn briefed(timeline: &[store::TimelineEvent]) -> Option<(usize, &str)> {
    let brief = timeline.iter().rev().find_map(|event| match &event.event {
        store::Event::Brief(markdown) => Some(markdown.as_str()),
        _ => None,
    })?;

    Some((entered(timeline)?, brief))
}

/// Where this follow-up's rounds begin, given the Event its brief was read off.
///
/// **The newest move into Follow-up, where the Timeline has one.** Ordinarily
/// that is the Event directly under the steer that opened it, so this decides
/// nothing and the two answers are one. What it is here for is the case where
/// they are not: a Conversation can leave Follow-up and be put back into it
/// without a steer of its own, which is what an investigation steered out of a
/// follow-up does when the human says there is nothing else — see
/// [`crate::investigations::landing`].
///
/// Windowed from the steer instead, the relaunch would be primed with the
/// investigation's rounds as its own: Sets another session asked, under a
/// heading saying this one asked them, carrying decisions the human took about a
/// session that was told to change nothing. So the window is the one
/// `store::nothing_else` opens, which is what this module's header says it is.
///
/// Never earlier than where the brief was found, because a brief is what a
/// session is started on and the rounds under it are the rounds about it.
///
/// `None` from [`entered`] is a Timeline with no move into Follow-up on it at
/// all, which is a record from before the move was written down. Then the Event
/// the brief came off is the window, as it always was.
fn window(timeline: &[store::TimelineEvent], brief_at: usize) -> usize {
    entered(timeline).map_or(brief_at, |moved| brief_at.max(moved))
}

/// The newest move into Follow-up on this Timeline.
fn entered(timeline: &[store::TimelineEvent]) -> Option<usize> {
    timeline
        .iter()
        .enumerate()
        .rev()
        .find_map(|(at, event)| match &event.event {
            store::Event::Moved(store::Lifecycle::FollowUp) => Some(at),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Lifecycle;

    /// A move onto a Timeline, which is the Event the window opens at.
    fn moved(state: Lifecycle) -> store::TimelineEvent {
        store::TimelineEvent {
            id: 1,
            at: "2026-09-26T12:00:00Z".to_owned(),
            event: store::Event::Moved(state),
        }
    }

    /// And the human's own line above one, with the brief they steered with.
    fn steer(target: Lifecycle) -> store::TimelineEvent {
        store::TimelineEvent {
            id: 2,
            at: "2026-09-26T12:00:00Z".to_owned(),
            event: store::Event::Steer(target, Some("Say what came of it\n".to_owned()), None),
        }
    }

    /// The ordinary steer: the move is directly under it, so the window is that
    /// move and the Sets under it are this follow-up's.
    #[test]
    fn the_window_is_the_move_the_steer_wrote() {
        let timeline = [
            moved(Lifecycle::Wrapping),
            steer(Lifecycle::FollowUp),
            moved(Lifecycle::FollowUp),
        ];

        assert_eq!(window(&timeline, 1), 2);
    }

    /// And a follow-up an investigation was steered out of opens its window at
    /// the move that brought it back, not at the steer above the whole business.
    ///
    /// Otherwise the rounds the investigating session asked would be read as
    /// this one's, and a decision the human took about a session told to change
    /// nothing would arrive as something this session had already agreed to.
    #[test]
    fn a_follow_up_put_back_opens_its_window_where_it_came_back() {
        let timeline = [
            steer(Lifecycle::FollowUp),
            moved(Lifecycle::FollowUp),
            steer(Lifecycle::Investigating),
            moved(Lifecycle::Investigating),
            moved(Lifecycle::FollowUp),
        ];

        assert_eq!(
            window(&timeline, 0),
            4,
            "the investigation's rounds are above the window rather than in it",
        );
    }

    /// A **Tinker**'s own follow-up reads its brief off the Brief and its window
    /// off the move the start wrote, so the two are already the same Event.
    #[test]
    fn a_tinkers_window_is_the_move_its_brief_was_found_against() {
        let timeline = [moved(Lifecycle::FollowUp)];

        assert_eq!(window(&timeline, 0), 0);
    }

    /// And a Timeline with no move into Follow-up on it is a record from before
    /// one was written down: the Event the brief came off is the window, as it
    /// always was.
    #[test]
    fn a_record_with_no_move_into_follow_up_windows_at_the_brief() {
        let timeline = [moved(Lifecycle::Wrapping), steer(Lifecycle::FollowUp)];

        assert_eq!(window(&timeline, 1), 1);
    }
}
