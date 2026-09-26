//! What an investigating session is started on, and where it is read back from
//! when one has to be started again.
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

use anyhow::Result;

use crate::AppState;
use crate::answer_files::OnAnswers;
use crate::store;

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
