//! What an investigating session is started on, where it is read back from when
//! one has to be started again, and where the investigation goes when it ends.
//!
//! An investigation is a conversation rather than a step, and nothing it does
//! is written down on the branch — that is the whole of what makes it an
//! investigation. So what a session of one is *for* is the question it was
//! opened with, and what it has got to is the rounds the human has already
//! answered. Both of those are on the Timeline, which is the one part of an
//! investigation that outlives the session having it.
//!
//! An investigation that has lost its session is therefore picked up the way a
//! follow-up is: a fresh session on the same question, primed with what has
//! already been said — see [`crate::follow_ups`], whose shape this is, and
//! [`crate::grillings`], whose digest both of them use. A relaunch that opened
//! by asking again what the human had already answered would cost them the
//! investigation twice.
//!
//! **This Investigating's own**, which is what the window is for. A
//! Conversation can be in Investigating more than once over its life, and the
//! round before this one is finished with. So the rounds are read from the
//! newest way *into* the state downwards — which is the window the Nothing-else
//! mark is read inside as well; see `store::nothing_else`.
//!
//! **And the same way in is what says where it goes out.** An investigation ends
//! where it was entered from, so the ending reads that same newest way in and
//! takes the state written down beside it — see [`landing`]. Which is why both
//! questions are here rather than one being the runner's: they are one reading of
//! the Timeline asked twice, and two places that disagreed about which
//! Investigating this is would be a session picked up on one investigation and
//! landed at the end of another.

use anyhow::Result;
use sqlx::SqlitePool;

use crate::AppState;
use crate::answer_files::OnAnswers;
use crate::store::{self, Lifecycle};

/// What an investigating session is primed with.
///
/// One thing rather than two arguments, for the reason [`crate::follow_ups`]
/// carries its pair in one: a session is started on the question and on
/// whatever has been said about it since, and a launch that carried one without
/// the other would be either an investigation with no subject or one that had
/// forgotten its own rounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Investigation {
    /// The question the investigation was opened with, as the human wrote it:
    /// the Conversation's own Brief where a Start opened it, and the brief the
    /// human steered with where a steer did.
    ///
    /// One field for the two, because the session is told the same thing either
    /// way — it is the question — and [`crate::skills::investigating`] puts it
    /// under the one heading that says act on it.
    pub(crate) brief: String,

    /// The rounds already asked and answered inside this Investigating, as one
    /// markdown document — empty for one that is just starting.
    pub(crate) settled: String,

    /// Whether this is an investigation being picked up again rather than
    /// opened.
    ///
    /// What turns on it is what the session before this one left behind: an ask
    /// it was idling on when it died is a question with nobody behind it, so it
    /// is locked as the fresh session starts, exactly as a relaunched grilling
    /// and a relaunched follow-up lock theirs.
    pub(crate) again: bool,
}

impl Investigation {
    /// One that is starting now: the question and nothing else.
    ///
    /// What a steer makes, and what a Start will. There is nothing to prime it
    /// with — the question is the first thing said — and a heading over an
    /// empty digest would tell the session that something had been.
    pub(crate) fn opening(brief: String) -> Investigation {
        Investigation {
            brief,
            settled: String::new(),
            again: false,
        }
    }
}

/// Read back what this Conversation's investigation was opened about and what
/// it has been through, or `None` where the Timeline holds nothing to pick up.
///
/// **Two ways in, and the newer of them wins.** A steer is one, and it carries
/// the question as the Steer Event's own body. A Start on an **Investigate**
/// draft is the other: that one writes no steer, being the press that starts
/// the work rather than one that takes something up about work already done —
/// so where there is no steer into Investigating the Brief is the question, and
/// the rounds are what has been answered since the move that press wrote.
///
/// Where the Brief is the question, two Events are read rather than one,
/// because the two say different things. The Brief is the newest, for the
/// newest's reason: a Conversation gets one Brief per round. The move is where
/// the window opens, which is the same place `store::nothing_else` opens its
/// window at: the Brief was written while the Conversation was still a Draft,
/// so counting from it would take in whatever stood between the human writing
/// it and the press that started the work.
///
/// `None` is a record that cannot be true: both ways in are refused without
/// something to find out about, so a Conversation standing in Investigating
/// with neither a steer nor a Brief on its Timeline is one nothing can be
/// started for. The press that asked says so by name rather than starting a
/// session on nothing — see [`crate::resume`].
///
/// One read of the Timeline for both halves, as a relaunched follow-up takes
/// one for its two: a Conversation that has been investigated has a Timeline
/// behind it, and picking it up again is no reason to read it twice.
///
/// The state rather than the pool alone, because the digest names the files put
/// on those Answers at the path this session will open them — which is a
/// question about the Data Directory as well as about the record. See
/// [`OnAnswers`].
pub(crate) async fn opened(
    state: &AppState,
    conversation_id: i64,
) -> Result<Option<Investigation>> {
    let timeline = store::timeline(&state.pool, conversation_id).await?;

    let Some((opened, brief)) = steered(&timeline).or_else(|| briefed(&timeline)) else {
        return Ok(None);
    };

    Ok(Some(Investigation {
        brief: brief.to_owned(),
        settled: crate::grillings::settled(
            &timeline[opened + 1..],
            &OnAnswers::of(state, conversation_id).await,
        ),
        again: true,
    }))
}

/// Where this Investigating goes when the human says there is nothing else.
///
/// **Back where it came from.** The newest steer into Investigating is what this
/// Investigating came in through — the same reading [`opened`] makes for the same
/// reason, a Conversation being able to go through more than one over its life —
/// and the state that steer found it in is written down beside its Event, so the
/// ending reads it rather than working it out. See
/// [`store::SteerRecord::source`], which is where it was put and why it was put
/// anywhere at all.
///
/// **Three states are never returned to.** Two of them have a way in of their
/// own that nothing else may use: a Draft is started and a Closed Conversation is
/// steered back into life. Investigating is the third, for a reason of its own —
/// an investigation sent back to Investigating is one the human could never end,
/// the move that lands it there opening a fresh Nothing-else window that the mark
/// they have only just ticked falls outside of, so the next session asks them for
/// the same mark over again. So an Investigating steered out of any of the three
/// ends **Done**, which is where an Investigate Conversation ends. One steered
/// out of Done lands there too, by the ordinary rule rather than as a case of its
/// own.
///
/// **And an Investigating with no steer above it is an Investigate Conversation**
/// — its Process's one working state, reached by the Start that cut the branch —
/// so there is nowhere for it to go back to and it ends Done. A steer whose source
/// was never written down reads the same way, for ADR-0006's reason: the record is
/// read as it was written, and what a row that is not there says is that nobody
/// wrote one.
///
/// One read of the Timeline, as picking an investigation up again takes one.
pub(crate) async fn landing(pool: &SqlitePool, conversation_id: i64) -> Result<Lifecycle> {
    Ok(homeward(&store::timeline(pool, conversation_id).await?))
}

/// That rule over the Timeline as it stands — see [`landing`], whose whole
/// reasoning this is.
///
/// The brief is not looked at, unlike [`steered`]'s: where the work came from is
/// the record's to say whatever was written to send it there with.
fn homeward(timeline: &[store::TimelineEvent]) -> Lifecycle {
    let came_from = timeline
        .iter()
        .rev()
        .find_map(|event| match &event.event {
            store::Event::Steer(Lifecycle::Investigating, _, record) => Some(record),
            _ => None,
        })
        .and_then(|record| record.as_deref())
        .and_then(|record| record.source);

    match came_from {
        None
        | Some(Lifecycle::Draft)
        | Some(Lifecycle::Closed)
        | Some(Lifecycle::Investigating) => Lifecycle::Done,
        Some(source) => source,
    }
}

/// Where on the Timeline this investigation was steered into being, and the
/// brief the human steered it with.
///
/// The newest, because a Conversation may have been through more than one and
/// the one it is in now is the last. Carrying a brief, because that is what a
/// session is started on: a steer without one is refused, so a Steer Event with
/// nothing under its target is a record from before Investigating existed.
fn steered(timeline: &[store::TimelineEvent]) -> Option<(usize, &str)> {
    timeline
        .iter()
        .enumerate()
        .rev()
        .find_map(|(at, event)| match &event.event {
            store::Event::Steer(store::Lifecycle::Investigating, Some(brief), _) => {
                Some((at, brief.as_str()))
            }
            _ => None,
        })
}

/// And the other way in: the move a Start on an **Investigate** draft wrote,
/// with the Brief that is what its investigation is about.
///
/// The newest Brief, and the newest move into Investigating under it. Read only
/// where there is no steer above them — a Conversation steered into
/// Investigating is having the steer's investigation, not the one its Start
/// opened all over again.
fn briefed(timeline: &[store::TimelineEvent]) -> Option<(usize, &str)> {
    let brief = timeline.iter().rev().find_map(|event| match &event.event {
        store::Event::Brief(markdown) => Some(markdown.as_str()),
        _ => None,
    })?;

    let opened = timeline
        .iter()
        .enumerate()
        .rev()
        .find_map(|(at, event)| match &event.event {
            store::Event::Moved(store::Lifecycle::Investigating) => Some(at),
            _ => None,
        })?;

    Some((opened, brief))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Steer Event as the store holds it on a Timeline: where the work went, the
    /// brief it went with, and the state the press found the Conversation in.
    ///
    /// `source` is `None` for a steer recorded before that was written down, which
    /// is the one reading here that is about the record rather than about the work.
    fn steer(target: Lifecycle, source: Option<Lifecycle>) -> store::TimelineEvent {
        store::TimelineEvent {
            id: 1,
            at: "2026-09-26T12:00:00Z".to_owned(),
            event: store::Event::Steer(
                target,
                Some("Where does the 429 count come from?\n".to_owned()),
                Some(Box::new(store::SteerRecord {
                    digest: false,
                    interrupt: false,
                    pairing: store::RecordedPairing::Nothing,
                    added: Vec::new(),
                    upgraded: Vec::new(),
                    source,
                })),
            ),
        }
    }

    /// And the move a Start on an **Investigate** draft writes, which is the other
    /// way into the state and the one with no steer above it.
    fn moved(state: Lifecycle) -> store::TimelineEvent {
        store::TimelineEvent {
            id: 2,
            at: "2026-09-26T12:00:00Z".to_owned(),
            event: store::Event::Moved(state),
        }
    }

    /// An Investigate Conversation: its Process's one working state, reached by the
    /// Start that cut the branch. There is nowhere for it to go back to, so it ends
    /// Done.
    #[test]
    fn an_investigation_no_steer_opened_ends_done() {
        assert_eq!(
            homeward(&[moved(Lifecycle::Investigating)]),
            Lifecycle::Done,
        );
        assert_eq!(homeward(&[]), Lifecycle::Done, "and so does an empty one");
    }

    /// One steered into Investigating goes back to the state the steer found it in,
    /// whatever that state was: nothing else about the Conversation changed, so the
    /// wrap-up or the run it was taken out of is what is left when the question is
    /// answered.
    #[test]
    fn a_steered_investigation_goes_back_to_the_state_it_came_from() {
        for source in [
            Lifecycle::Grilling,
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
            Lifecycle::FollowUp,
        ] {
            assert_eq!(
                homeward(&[
                    moved(source),
                    steer(Lifecycle::Investigating, Some(source)),
                    moved(Lifecycle::Investigating),
                ]),
                source,
                "{source:?} is where the steer found it, so {source:?} is where it \
                 goes back to",
            );
        }
    }

    /// Three states are never returned to, so an Investigating steered out of any
    /// of them ends where an Investigate Conversation does.
    ///
    /// Two have a way in of their own that nothing else may use: a Draft is
    /// started and a Closed Conversation is steered back into life. Investigating
    /// is the third, and its reason is the human's own mark: the move that landed
    /// it back there opens a fresh Nothing-else window, so the tick that ended the
    /// investigation falls outside it and the fresh session asks for it again —
    /// which is a round of rounds nothing but a Steer or a close could ever get
    /// them out of.
    ///
    /// Done is in the list because it is the same answer read by the ordinary rule
    /// rather than as a case of its own.
    #[test]
    fn the_states_nothing_returns_to_end_done_instead() {
        for source in [
            Lifecycle::Draft,
            Lifecycle::Closed,
            Lifecycle::Investigating,
            Lifecycle::Done,
        ] {
            assert_eq!(
                homeward(&[steer(Lifecycle::Investigating, Some(source))]),
                Lifecycle::Done,
                "a steer out of {source:?}",
            );
        }
    }

    /// The newest steer into Investigating is the one this Investigating came in
    /// through: a Conversation can go through more than one over its life, and the
    /// round before this one is finished with.
    ///
    /// Which is the same reading [`opened`] makes of the same Timeline — and a
    /// steer into somewhere else is no part of either.
    #[test]
    fn the_newest_steer_into_investigating_is_what_decides() {
        assert_eq!(
            homeward(&[
                steer(Lifecycle::Investigating, Some(Lifecycle::Grilling)),
                moved(Lifecycle::Wrapping),
                steer(Lifecycle::FollowUp, Some(Lifecycle::Wrapping)),
                steer(Lifecycle::Investigating, Some(Lifecycle::Wrapping)),
                moved(Lifecycle::Investigating),
            ]),
            Lifecycle::Wrapping,
            "the grilling it was taken out of a year ago is not where this one \
             came from",
        );
    }

    /// And a steer whose source was never written down reads as one that came from
    /// nowhere, which ends Done.
    ///
    /// ADR-0006's rule: the record is read as it was written, so a row that is not
    /// there says that nobody wrote one rather than saying something about the
    /// work. Every steer written since the source was recorded has one.
    #[test]
    fn a_steer_recorded_before_the_source_was_ends_done() {
        assert_eq!(
            homeward(&[steer(Lifecycle::Investigating, None)]),
            Lifecycle::Done,
        );
    }
}
