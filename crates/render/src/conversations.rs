//! The workbench's own side of the wire: the Conversations in the sidebar, one
//! Conversation with its Timeline, and everything the human changes about a
//! Conversation while it is still drafting.
//!
//! The Brief arrives already rendered, as every piece of markdown crossing this
//! wire does — the parser and the sanitizer are the server's, and the viewer only
//! puts HTML in the page. Its source travels beside the HTML, because the Brief
//! is the one document on this wire the human edits: a page that had to unrender
//! it to fill the field would need a parser after all.
//!
//! The Timeline is a list of Events with a kind, not a Brief and a list. The
//! Brief is the only kind there is yet; agent output, Question Sets and commits
//! are variants added beside it, and a Timeline shaped around its first Event
//! would have to be taken apart to hold the second.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::{ProfileEntry, RepoEntry};

/// Where a Conversation has got to.
///
/// The whole ladder, though only the first two are reachable yet: the states are
/// the domain's, and the page says which one a Conversation is in rather than
/// assuming the only one it can currently be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Lifecycle {
    Draft,
    Grilling,
    Direction,
    Implementing,
    Wrapping,
    Done,

    /// Off the ladder rather than on it: the work stopped wherever it had got
    /// to. Reachable from every other state, and leading nowhere.
    Aborted,
}

/// One row of the conversations sidebar.
///
/// The branch is the row's name: a Conversation has no title of its own, and of
/// what it does have the branch is the short line the human chose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ConversationEntry {
    pub id: i64,
    pub branch: String,

    /// What the Repo this Conversation is against is called.
    pub repo: String,

    pub state: Lifecycle,
}

/// One Conversation, whole: what it is attached to, what the human has settled
/// about it, and everything that has happened to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ConversationView {
    pub id: i64,

    /// The Repo, in the shape the Repo list already sends one — the workbench
    /// shows the same three facts about it, and a second shape for the same
    /// thing would be a second opinion about what a Repo is.
    pub repo: RepoEntry,

    pub branch: String,

    /// The commit the work will branch from, where the human overrode the rule.
    /// `null` is the rule itself: the default branch's tip, as it stands when
    /// grilling starts — which is why there is no value here to show instead.
    pub base_commit: Option<String>,

    pub state: Lifecycle,

    /// The Agent Profile the grilling session will run under, whole rather than
    /// by id: the pane says what it is, and whether it is still runnable.
    pub grilling_profile: Option<ProfileEntry>,

    /// And the one the implementation will run under. Chosen separately because
    /// it is genuinely a separate account and model.
    pub implementation_profile: Option<ProfileEntry>,

    /// Whether everything needed before grilling will start is settled: both
    /// Profiles chosen and neither broken, a Brief with something in it, and a
    /// Conversation still drafting.
    ///
    /// The server's rule rather than something the page works out from the
    /// fields around it. Every one of the refusals is checked again when the
    /// button is pressed — this is what decides whether to offer the button, and
    /// what it says is true only as of the moment it was read.
    pub ready_to_grill: bool,

    /// The worktree the grilling was given to work in, once there is one.
    ///
    /// `null` both before grilling starts and after aborting — the two ways a
    /// Conversation has none, which are the same fact about it.
    pub worktree: Option<Worktree>,

    /// Oldest first, which is reading order and puts the Brief at the top.
    pub timeline: Vec<TimelineEvent>,
}

/// A Conversation's worktree: where it is, and whether it is still there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Worktree {
    pub path: String,

    /// Whether the directory has gone from under Verkstead.
    ///
    /// A thing to say rather than a thing to fail on later. A worktree is an
    /// ordinary directory on a machine the human also uses, and one that has
    /// been deleted by hand should read as a Conversation with a problem — not
    /// as an obscure failure from whatever next tries to work in it.
    pub missing: bool,
}

/// One entry in a Timeline.
///
/// A tagged kind rather than a struct with a nullable field per kind: what the
/// details pane draws is decided by which kind an Event is, and the stages after
/// this one add their kinds here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum TimelineEvent {
    /// The Brief, rendered inline in the Timeline — the design's table gives it
    /// no details pane, because there is nothing of it the Timeline does not
    /// already show.
    Brief(BriefEvent),

    /// The Conversation moved, and this is the state it moved to. Starting to
    /// grill and aborting are both this Event, because both are the work
    /// changing hands and the state is the only thing that differs.
    Moved(MovedEvent),
}

/// A move as the page receives it: when, and to what.
///
/// No rendered body, unlike the Brief — there is no markdown in a move. What the
/// Timeline draws is a sentence of the viewer's own making from the one state,
/// because the wording belongs to whoever is reading it rather than to the
/// record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct MovedEvent {
    pub id: i64,

    /// When it moved, RFC 3339.
    pub at: String,

    pub state: Lifecycle,
}

/// The Brief as the page receives it: rendered for reading, and as it was
/// written for editing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BriefEvent {
    pub id: i64,

    /// When it was written, RFC 3339.
    pub at: String,

    /// The markdown the human last wrote, for the field they write it in.
    pub markdown: String,

    /// The same, as HTML — rendered and sanitized by the server on the way out.
    pub html: String,
}

/// A move as an Event. Nothing to render — see [`MovedEvent`] — but built here
/// beside the Brief so that one place knows how a Timeline is made.
pub fn moved_event(id: i64, at: String, state: Lifecycle) -> TimelineEvent {
    TimelineEvent::Moved(MovedEvent { id, at, state })
}

/// The Brief as an Event, rendered on the way.
///
/// Here rather than in the server for the reason the Set's rendering is: this is
/// the crate with the markdown parser in it, and whatever serves the viewer, the
/// rendering happens in one place.
pub fn brief_event(id: i64, at: String, markdown: String) -> TimelineEvent {
    TimelineEvent::Brief(BriefEvent {
        id,
        at,
        html: crate::markdown::to_html(&markdown),
        markdown,
    })
}

/// Starting a Conversation: the Repo it is against, and nothing else.
///
/// The branch name is not the browser's to send. It is prefilled randomly, and a
/// prefill the page invented would be one the server never saw — the record is
/// the server's from the moment it exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct NewConversation {
    pub repo_id: i64,
}

/// What became of starting one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Started {
    /// Started, and this is which one — the page goes straight to it.
    Started { id: i64 },

    /// There is no Repo with that id to attach it to.
    NoSuchRepo,
}

/// A Brief as the human has just written it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BriefEdit {
    pub markdown: String,
}

/// What the branch is to be called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BranchRename {
    pub branch: String,
}

/// The commit to branch from, or `null` to go back to the default-branch rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BaseCommitOverride {
    /// Whatever names a commit in the repository — a short hash, a tag, a branch
    /// — which the server resolves before it records anything.
    pub commit: Option<String>,
}

/// What became of an edit to a Brief.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum BriefSaved {
    Saved,
    NoSuchConversation,

    /// The Conversation is past drafting, so its Brief is frozen: a reopened
    /// round adds a new Brief rather than editing this one.
    NotDrafting,
}

/// What became of naming the branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum BranchRenamed {
    Renamed,
    NoSuchConversation,

    /// The Conversation is past drafting: the branch exists by now, and renaming
    /// it is not a thing a text field does.
    NotDrafting,

    /// Not a name git would take for a branch. Asked of git itself rather than
    /// guessed at from a list of forbidden characters.
    NotABranchName,
}

/// What became of overriding the base commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum BaseRecorded {
    Recorded,
    NoSuchConversation,

    /// The Conversation is past drafting, and the base commit was captured when
    /// grilling started.
    NotDrafting,

    /// Nothing in that repository answers to what was typed. Refused now rather
    /// than at grill start, where it would be a failure with nobody watching.
    NoSuchCommit,
}

/// What became of starting a Conversation grilling.
///
/// Every refusal is named rather than collapsed into one, because each of them
/// is something different for the human to go and do: choose a Profile, write a
/// Brief, pick another commit, deal with a branch that is already there. A
/// single "cannot start" would leave them guessing which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum GrillingStarted {
    /// The branch and the worktree are made, and the Conversation is grilling.
    Started,

    NoSuchConversation,

    /// It is past drafting, so it has been started once already — or aborted.
    NotDrafting,

    /// No Agent Profile is chosen for the grilling session.
    NoGrillingProfile,

    /// None is chosen for the implementation either. Fixed before starting
    /// rather than after, because the grilling ends by handing over to it.
    NoImplementationProfile,

    /// A chosen Profile's pair is not where it was left, so there is no account
    /// to run the session under.
    ProfileBroken,

    /// The Brief is empty, and the Brief is what the grilling starts from.
    /// Freezing an empty one would freeze nothing worth having.
    EmptyBrief,

    /// Nothing in the repository answers to what the work would branch from —
    /// an overridden commit that has gone, or a default branch that has.
    NoBaseCommit,

    /// The branch is already there. Verkstead did not make it, so it will not
    /// take it over: what is on it is somebody's work.
    BranchExists,

    /// Git would not make the worktree. The reason is in the server's log — this
    /// is the one refusal with nothing for the human to correct.
    WorktreeRefused,
}

/// What became of aborting one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ConversationAborted {
    /// Stopped: the worktree is gone and the branch is not.
    Aborted,

    /// It was aborted already, which is not an error — what was asked for holds
    /// either way.
    AlreadyAborted,

    NoSuchConversation,

    /// The worktree could not be removed, so nothing was recorded: a Conversation
    /// that said it had stopped while its directory was still there would be one
    /// nothing would ever clean up.
    WorktreeStuck,
}
