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

use crate::RepoEntry;

/// Where a Conversation has got to.
///
/// The whole ladder, though only [`Lifecycle::Draft`] is reachable yet: the
/// states are the domain's, and the page says which one a Conversation is in
/// rather than assuming the only one it can currently be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Lifecycle {
    Draft,
    Grilling,
    Direction,
    Implementing,
    Wrapping,
    Done,
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

    /// Oldest first, which is reading order and puts the Brief at the top.
    pub timeline: Vec<TimelineEvent>,
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
