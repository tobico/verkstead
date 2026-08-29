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

use crate::conversations::{CommitPane, ConversationView, TimelineEvent};
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

    /// The pane behind every commit on that Timeline, in its order.
    ///
    /// Carried for the reason the sheets are: the workbench fetches a commit
    /// when somebody opens one, and a share has nothing to fetch with. So the
    /// whole of every one of them rides in the file — the Commit Summary
    /// rendered, and the diff parsed, highlighted and folded per file.
    ///
    /// No cap on any of it, and nothing summarised on the way out. What a
    /// colleague is being shown is the work, and a patch cut off at a size is
    /// a different document from the one the human reviewed.
    pub commits: Vec<SharedCommit>,

    /// When the share was taken, RFC 3339.
    ///
    /// A share is a snapshot of a moment rather than a window onto a
    /// Conversation that goes on moving, so the moment is on the file: the
    /// reader is owed the date of the thing in their hands, and sharing again
    /// makes another one rather than freshening this.
    pub exported_at: String,
}

/// One commit as a share carries it: the pane the workbench would have fetched,
/// beside the Event whose card opens it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SharedCommit {
    /// Which Timeline Event this is the pane of.
    ///
    /// The Event rather than the hash, because that is what the card opening it
    /// is known by — and a Conversation works in more than one repository, so a
    /// hash is not a name for one commit here either.
    pub id: i64,

    /// What the workbench's details pane draws: the Commit Summary rendered,
    /// and the diff with every fold and every colour already in it.
    ///
    /// The endpoint's own rendering rather than a second one, so that a
    /// colleague reading a patch and the human who reviewed it are reading one
    /// drawing of it.
    pub pane: CommitPane,

    /// Whether the repository still had the commit when the share was taken.
    ///
    /// `false` is a commit git can no longer show — rebased away, collected, or
    /// in a repository that has moved out from under Verkstead — and the pane
    /// says the diff is not in the file rather than that the commit changed
    /// nothing. The workbench answers that case with a 404, which a share
    /// cannot: one commit nobody can read is no reason to refuse the export of
    /// everything around it, and what the Timeline says about it — the subject,
    /// the hash, how much it moved — is on the card either way.
    pub held: bool,
}

/// One commit as a share carries it, rendered.
///
/// `patch` is what the repository said when it was asked for one, and `None` is
/// a repository that would not say — which is the whole of what
/// [`SharedCommit::held`] records.
///
/// The summary is rendered either way. It was kept by the sweep that recorded
/// the commit rather than read back out of git, so a commit that has gone still
/// has its own account of itself to read.
pub fn shared_commit(id: i64, summary: Option<&str>, patch: Option<&str>) -> SharedCommit {
    SharedCommit {
        id,
        // No patch renders as no diff, which is also what a merge or an empty
        // commit renders as — the flag beside it is what tells the reader which
        // of the two kinds of nothing they are looking at.
        pane: crate::conversations::commit_pane(summary, patch.unwrap_or_default()),
        held: patch.is_some(),
    }
}

/// Curate a Conversation for sharing: the Events that board, the sheets and the
/// diffs behind the ones that open, and a record with nothing left on it to act
/// on.
///
/// `sets` and `commits` are every Question Set and every commit the caller
/// rendered. What comes back holds the ones still on the curated Timeline and
/// no others: which Events board is this module's rule, and a bundle carrying
/// the sheet of a Set whose row was taken off would be carrying what the reader
/// was not meant to have.
pub fn shared(
    conversation: ConversationView,
    sets: Vec<SetView>,
    commits: Vec<SharedCommit>,
    exported_at: String,
) -> SharedConversation {
    let timeline: Vec<TimelineEvent> = conversation
        .timeline
        .into_iter()
        .filter(boards)
        .map(frozen)
        .collect();

    let boarded: Vec<i64> = timeline.iter().filter_map(asked).collect();
    let landed: Vec<i64> = timeline.iter().filter_map(committed).collect();

    SharedConversation {
        sets: sets
            .into_iter()
            .filter(|set| boarded.contains(&set.id))
            .collect(),
        commits: commits
            .into_iter()
            .filter(|commit| landed.contains(&commit.id))
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

            // And where a share of this Conversation was last published, which
            // is the workbench's fact about the record rather than part of it:
            // a reader already holds a share, and one carrying the link to
            // another would be handing on a URL nobody meant to give them.
            shared: None,

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

/// And which Event a commit's pane belongs to, on the one kind that has one.
fn committed(event: &TimelineEvent) -> Option<i64> {
    match event {
        TimelineEvent::Commit(commit) => Some(commit.id),
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

/// What became of publishing a share: where it went, or why it did not go.
///
/// A publish is Verkstead's own write to GitHub rather than a session's, and
/// every way it can fail is something for the human to go and do — which is why
/// each is named rather than folded into one refusal. Two of the three are about
/// the token on the settings page, and the page is where they are answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum SharePublished {
    /// It is up, at this link and as of this moment.
    Published {
        share: crate::conversations::ShareView,
    },

    /// No token is configured, so there is nobody to publish as.
    ///
    /// A refusal rather than a fallback to whatever login the host's `gh` has,
    /// which is the one place Verkstead's reach into GitHub differs between
    /// reading and writing: a pull request read as the host is a question asked
    /// twice, and a gist *written* as the host is a file in an account nobody
    /// chose, under a login the human may not even be able to find it in.
    NoToken,

    /// There is one, and gists are not among what GitHub will let it do — the
    /// `gist` scope, which a token issued for reading repositories does not
    /// carry. Fixed by re-issuing it with that ticked and saving it again.
    NoGistScope,

    /// Something else, in `gh`'s or git's own words.
    Refused { why: String },
}
