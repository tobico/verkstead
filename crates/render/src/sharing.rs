//! What a share carries: one Conversation's record, curated, as the file a
//! colleague opens.
//!
//! A share is read rather than worked in, and it is read by somebody who is not
//! sitting at this Verkstead — so the curation happens here, on the way out,
//! rather than in the page that draws it. Two halves to it, and both are about
//! the same thing: what a reader outside the workbench has any business with.
//!
//! **Which Events board.** The Brief, the Question Sets, the commits, the
//! steers, the moves and the Manual Tasks a Verkstead of before set going. Not
//! what a session printed, not the Notices Verkstead wrote itself, not the
//! handoff, not a Set this build could not read, and none of the pinned cards.
//! Silently: no placeholder marks the gap, because a share is a curated record
//! rather than a record with holes cut in it — a row saying *something was here*
//! would be an invitation to ask for what was deliberately left out.
//!
//! **And what is left of the Conversation around them.** Every field the
//! workbench reads to decide what may be *done* is put back to the value that
//! says *nothing* — no run to resume, no run to stop, no grilling to start,
//! nothing being adopted. The page the share is drawn with is the workbench's
//! own, so this is what makes it read-only at the source: a share cannot express
//! an action, whatever a component reused to draw it would otherwise offer.
//!
//! What the reader does have is the record and the way around it, which is the
//! whole point: the Timeline on one side, and whatever it opens on the other.

use serde::{Deserialize, Serialize};
#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::conversations::{ConversationView, TimelineEvent};
use crate::view::SetView;

/// One Conversation as a share carries it, which is what the shared file boots
/// from.
///
/// The Conversation whole rather than a shape of its own: the share is drawn by
/// the workbench's own components, so what they are handed has to be what they
/// are always handed. What differs is that this one has been through
/// [`shared`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SharedConversation {
    /// The record, curated — see [`shared`].
    pub conversation: ConversationView,

    /// The sheet of every Question Set on that Timeline, in its order.
    ///
    /// Carried rather than fetched, which is the difference between a share and
    /// the workbench: the live viewer asks for a Set when somebody opens one,
    /// and a share has nothing to ask. So the whole of every Set the record
    /// holds — the Preface, every Option of every Question, the Diff it was
    /// asked over and what was decided — rides in the file, rendered by the
    /// endpoint the workbench reads a Set through, so that a colleague's sheet
    /// and the human's are one rendering of one decision.
    ///
    /// Read-only regardless of how a Set stood when the share was taken: what
    /// makes it so is the sheet being drawn as a record — see the share's
    /// details pane — because a Set still waiting on somebody is part of the
    /// record too, and a reader with no server behind them cannot answer it.
    pub sets: Vec<SetView>,

    /// When the share was taken, RFC 3339.
    ///
    /// A share is a snapshot of a moment rather than a window onto a
    /// Conversation that goes on moving, so the moment is on the file: the
    /// reader is owed the date of the thing in their hands, and sharing again
    /// makes another one rather than freshening this.
    pub exported_at: String,
}

/// Curate a Conversation for sharing: the Events that board, the sheets behind
/// the ones that open, and a record with nothing left on it to act on.
///
/// `sets` is every Question Set the caller rendered. What comes back holds the
/// ones still on the curated Timeline and no others: which Events board is this
/// module's rule, and a bundle carrying the sheet of a Set whose row was taken
/// off would be carrying what the reader was not meant to have.
pub fn shared(
    conversation: ConversationView,
    sets: Vec<SetView>,
    exported_at: String,
) -> SharedConversation {
    let timeline: Vec<TimelineEvent> = conversation
        .timeline
        .into_iter()
        .filter(boards)
        .map(frozen)
        .collect();

    let boarded: Vec<i64> = timeline.iter().filter_map(asked).collect();

    SharedConversation {
        sets: sets
            .into_iter()
            .filter(|set| boarded.contains(&set.id))
            .collect(),
        conversation: ConversationView {
            timeline,

            // Nothing is pinned in a share. Each pinned card is the current
            // state of something the work is against — a backlog read off a
            // worktree, a pull request as GitHub has it — and a share is
            // neither of those: it is a moment, and the reader has no worktree
            // to read and nothing to open a pull request with.
            pinned: Vec::new(),

            // Every field the workbench decides an action by, said as nothing.
            // The record is what is being shared; what could be *done* about it
            // belongs to whoever has the workbench.
            ready_to_grill: false,
            ready_to_resume: false,
            ready_to_stop: false,
            stop_asked: false,
            ready_to_continue: false,
            compiles_uncached: false,

            // And what is being adopted, which is the one other thing that puts
            // a control on the record: an adopting Conversation draws the Adopt
            // press and the setup card under its Brief.
            adopting: None,

            // What is happening right now, which is nothing here: a share is a
            // file, and no session is running in it. Both of these are read as
            // of the moment a page is drawn, so a share that carried them would
            // be saying something true of a moment that has passed.
            working: false,
            driven: false,

            // And the marks that point at a stop. The Notice a stop is read
            // through does not board, so a badge pointing at one would point at
            // nothing — and *blocked on you* said to somebody who cannot act is
            // a mark asking the wrong person.
            blocked_on: None,
            stopped_by_hand: false,
            waiting_on_checks: false,
            resets: None,

            ..conversation
        },
        exported_at,
    }
}

/// Whether one Event boards.
///
/// The rule is a list rather than a judgement: what a share is for is showing
/// what was asked, answered and built, and the kinds left out are the ones that
/// are either nobody else's to read — a session's own output, the handoff
/// between two of them — or Verkstead talking to itself.
fn boards(event: &TimelineEvent) -> bool {
    match event {
        TimelineEvent::Brief(_)
        | TimelineEvent::QuestionSet(_)
        | TimelineEvent::Commit(_)
        | TimelineEvent::Steer(_)
        | TimelineEvent::Moved(_)
        | TimelineEvent::ManualTask(_) => true,

        TimelineEvent::AgentOutput(_)
        | TimelineEvent::UnreadableSet(_)
        | TimelineEvent::Handoff(_)
        | TimelineEvent::Notice(_)
        | TimelineEvent::PullRequest(_)
        | TimelineEvent::TaskList(_)
        | TimelineEvent::StageList(_) => false,
    }
}

/// Which Set an Event on the curated Timeline is about, on the one kind that is
/// about one.
fn asked(event: &TimelineEvent) -> Option<i64> {
    match event {
        TimelineEvent::QuestionSet(asked) => Some(asked.set_id),
        _ => None,
    }
}

/// And the one Event a share has to say something else about: the Brief, which
/// is frozen here whatever it was.
///
/// A Brief that has not frozen is a field the human types into, with the
/// Conversation's setup under it. Frozen, it is the document it will be read as
/// for the rest of the record's life — which is what a share of a Draft should
/// show, and the only thing a reader with no server behind them could do
/// anything with.
fn frozen(event: TimelineEvent) -> TimelineEvent {
    match event {
        TimelineEvent::Brief(brief) => TimelineEvent::Brief(crate::conversations::BriefEvent {
            frozen: true,
            ..brief
        }),
        other => other,
    }
}
