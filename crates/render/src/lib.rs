//! What the viewer receives, and the rendering that produces it.
//!
//! Everything an agent writes — the Preface, every Question's and every
//! Option's text, and the Diff — is turned into sanitized HTML here, on the
//! server, before it goes anywhere near a browser. That is the whole point of
//! this crate being its own: the markdown parser, the sanitizer and the syntax
//! highlighter live on one side of the wire, and the viewer receives HTML it
//! only has to put in the page.
//!
//! The rest of what the viewer is handed lives here for the same reason, even
//! where no markdown is involved: a Timeline's Events and the outcomes of
//! answering and locking are named states the viewer says in words. Taken
//! together these types *are* the viewer's side of the wire, which is what makes
//! this the one crate the TypeScript is generated from — see the `typescript`
//! feature in the manifest.
//!
//! Nothing here knows about the store, the router or the viewer. Given a
//! [`verkstead_schema::QuestionSet`] and where the Set stands, [`set_view`]
//! hands back the [`SetView`] the viewer draws — so whatever is serving that
//! viewer, this is the one place the rendering happens.

mod answering;
mod browsing;
mod conversations;
mod profiles;
mod push;
mod repos;
mod settings;
mod sharing;
mod transcript;
mod update;
mod view;

pub use answering::{Locked, Submitted};
pub use browsing::{BrowseScope, DirectoryEntry, DirectoryListing, EntryKind};
pub use conversations::{
    AbandonedRepo, AbandonedRoadmap, Adopted, AdoptedStage, AdoptionView, AgentOutputEvent,
    AgentSession, Attached, AttachmentOrigin, AttachmentRemoved, AttachmentView, BacklogPane,
    BaseBranchChoice, BaseRecorded, BranchRename, BranchRenamed, BriefEdit, BriefEvent, BriefSaved,
    Capture, CheckRollup, Checked, Comment, CommitEvent, CommitPane, CommitRecord, CompanionAdded,
    CompanionAddition, CompanionBaseRecorded, CompanionBranchRenamed, CompanionMode,
    CompanionModeChoice, CompanionModeChosen, CompanionRefusal, CompanionRemoved, CompanionUpgrade,
    CompanionView, ConversationArchived, ConversationClosed, ConversationEntry,
    ConversationSteered, ConversationStopped, ConversationUnarchived, ConversationView,
    GrillingStarted, HandoffEvent, Lifecycle, ManualTaskEvent, Merging, MovedEvent, NewAdoption,
    NewCompanion, NewConversation, NewOrder, NoticeEvent, PinnedEvent, ProposalView,
    PullRequestCheck, PullRequestComment, PullRequestCommit, PullRequestDetails, PullRequestEvent,
    PullRequestSummary, QuestionSetEvent, RepoChoice, RepoSwitched, ResolveConflictsEvent,
    Resolved, Resumed, RoadmapPane, Screen, SessionsHere, SetRow, ShareView, ShowingArchived,
    Shown, Size, StageDocument, StageEntry, StageListEvent, StageListReached, StageSource, Started,
    SteerCompanionRefusal, SteerEvent, SteerOpened, SteerSubmission, SteerTarget, TaskDocument,
    TaskEntry, TaskListEvent, TaskListReached, TaskSource, TimelineEvent, UnreadableSetEvent,
    Watching, Worktree, agent_output_event, agent_output_pinned, backlog_pane, brief_event,
    commit_event, commit_pane, handoff_event, manual_task_event, moved_event, notice_event,
    proposal_view, pull_request_details, pull_request_event, pull_request_reached,
    question_set_event, resolve_conflicts_event, roadmap_pane, stage_list, stage_list_event,
    stage_list_reached, steer_event, task_list, task_list_event, task_list_reached,
    unreadable_set_event,
};
pub use profiles::{
    AgentType, Broken, PairingView, PickedView, ProfileAccount, ProfileChoice, ProfileChosen,
    ProfileDeleted, ProfileEdit, ProfileEntry, ProfileSaved, RepoPairingsView, RoleChoice,
};
pub use push::{PushKey, Subscribed, Subscription, Unsubscribe};
pub use repos::{
    ConflictResolutionEdit, Registered, Registration, RepoEntry, RepoRemoved, RepoView,
};
pub use settings::{
    Author, BindEntry, BuildCacheEdit, BuildCacheView, CleanupEdit, CleanupStepEdit,
    CleanupStepView, CleanupView, ConflictResolution, IgnoreRule, IgnoredCommentsEdit,
    PathResolution, PathSource, PathsView, RuleField, RuleRefused, SettingsEdit, SettingsSaved,
    SettingsView, TokenEdit, TokenSaved, Verified, WatchedPathEntry,
};
pub use sharing::{
    CommentedOn, MissedOut, SHARE_MARKER, ShareCommented, SharePublished, SharedCommit,
    SharedConversation, itemized, shared, shared_commit,
};
pub use transcript::{
    Bookkeeping, Cursor, Prose, Put, Reasoning, ToolResult, ToolUse, TranscriptView, Turn, Unread,
    rollout_cwd, statements, transcript_after, transcript_view, turns,
};
pub use update::UpdateNotice;
pub use view::{
    Answered, AskView, DiffView, OptionView, QuestionView, RepoDiffView, SetReading, SetView,
    Standing, UnreadableSet,
};

pub mod diff;
pub mod markdown;
pub use view::set_view;

// The highlighter is shared by the two renderers above and wanted by nobody
// else: what it produces reaches the page through them, already marked up. The
// one thing outside it that has any business with the highlighter is starting
// it up — see [`warm_highlighter`].
mod highlight;

/// Build the syntax definitions now rather than on the first request that needs
/// them — see [`highlight::warm`]. Blocking, and worth a thread of its own.
pub fn warm_highlighter() {
    highlight::warm();
}

// Where the TypeScript is written from, which is a test and nothing else: the
// bindings are generated by running one, never by building anything.
#[cfg(all(test, feature = "typescript"))]
mod typescript;
