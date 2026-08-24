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
//! Brief was the only kind there was to begin with; agent output, Question Sets
//! and commits are variants added beside it, and a Timeline shaped around its
//! first Event would have had to be taken apart to hold the second.

use serde::{Deserialize, Serialize};
use verkstead_schema::Direction;

#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::{DiffView, PairingView, RepoEntry, Standing};

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
///
/// Where it has got to is drawn rather than worded — a turning ring for a
/// session getting on with it, the same ring empty for one that has gone quiet,
/// a dot for a Conversation that wants answering, a dotted border for a draft
/// and a dimmed card for work that has stopped. Which is why the facts below are
/// facts and not one collapsed verdict: the row says what is true of the
/// Conversation, and which mark that comes out as is the one rule the viewer
/// keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ConversationEntry {
    pub id: i64,
    pub branch: String,

    /// What the Repo this Conversation is against is called.
    pub repo: String,

    pub state: Lifecycle,

    /// Whether a session is running on this Conversation right now.
    ///
    /// The server's own registry of running processes and nothing else, so a
    /// server that restarted says no about work it is no longer doing.
    pub working: bool,

    /// And whether that session has stopped printing — the same quiet the
    /// agent-output row's mark is drawn from, said here so that a card and the
    /// row it opens tell the same truth about the same session.
    ///
    /// Always `false` where nothing is working, which is what keeps the two a
    /// pair rather than a contradiction: idle is a thing a *running* session
    /// is. Which mark it comes out as is the viewer's, and the rule there is
    /// unchanged — waiting still wins over both.
    pub idle: bool,

    /// Whether something about this Conversation is waiting on the human: an ask
    /// left open, or a run stopped on an Interruption.
    ///
    /// Folded from every source before it leaves, so the viewer holds no list of
    /// them. A Draft is never one of them: it is drawn as a draft, and that is
    /// the whole of what a draft has to say.
    pub waiting: bool,
}

/// One Repo's notice under the new-conversation box: the roadmaps in it that
/// nothing is driving.
///
/// One notice per Repo with its roadmaps inside, rather than one per roadmap —
/// what the human reads first is which repository has work left lying about,
/// and a repository with three of them is one thing to look at rather than
/// three.
///
/// Nothing here is stored. Every field is read off the repository at the moment
/// the list is drawn, which is why a roadmap somebody has since picked up
/// simply stops appearing rather than having to be taken off anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AbandonedRepo {
    /// Which Repo, by the id a Conversation is started against.
    pub repo_id: i64,

    /// And what it is called, which is what the notice says.
    pub repo: String,

    /// The roadmaps in it with a stage that could be started now. Never empty:
    /// a Repo with nothing to adopt has no notice at all.
    pub roadmaps: Vec<AbandonedRoadmap>,
}

/// One abandoned roadmap, named with the stage that would be adopted.
///
/// The stage is the lowest-numbered unchecked one, which is the roadmap's own
/// order rather than anybody's choice — see the abandoned rule in the server's
/// `stages` module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AbandonedRoadmap {
    /// Its directory name under `docs/roadmaps/` — `mvp`. The roadmap's
    /// identity, here as everywhere else.
    pub name: String,

    /// What the roadmap calls itself in its heading, or empty where it has
    /// none. Prose about itself, riding along beside the name.
    pub title: String,

    /// The next stage's number as the roadmap writes it — `04`.
    pub stage: String,

    /// And what that stage is called.
    pub stage_title: String,
}

/// What a Conversation is adopting, as its own page draws it: the roadmap it
/// was started for, and the stage adopting would start.
///
/// Read off the repository at the Conversation's base commit every time the
/// page is, rather than kept: the roadmap is the repository's document, and
/// only the name of it is Verkstead's. So a base commit the human overrides is
/// answered by the stage that is next *there*, which is the whole reason the
/// stage is not carried over from the notice that was clicked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AdoptionView {
    /// Its directory name under `docs/roadmaps/` — `mvp`. The one thing about
    /// the roadmap that is stored, and the one thing that is true whatever the
    /// base commit says.
    pub roadmap: String,

    /// What the roadmap calls itself in its heading at that commit, or empty
    /// where it has none — and where the roadmap is not there to read.
    pub title: String,

    /// The stage adopting would start: the lowest-numbered unchecked one, read
    /// at the base commit.
    ///
    /// `null` where there is none to start there — the roadmap is finished, or
    /// gone, or its next stage is somebody else's already. The press says which
    /// of those it is; this is only what the page can name.
    pub stage: Option<AdoptedStage>,
}

/// The stage an adoption would start, named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AdoptedStage {
    /// Its number as the roadmap writes it — `04`.
    pub label: String,

    /// And what the roadmap calls it.
    pub title: String,

    /// Where its brief is in the repository, which is the document the work
    /// starts from.
    pub brief_path: String,

    /// The branch the stage would be worked on: its own slug, as the unattended
    /// path names one. The Conversation's server-invented name is discarded at
    /// the press, so this is the name to say rather than that one.
    pub branch: String,
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

    /// The Profile and model the grilling session will run under, whole rather
    /// than by id: the pane says what they are, and whether the Profile is
    /// still runnable.
    pub grilling_pairing: Option<PairingView>,

    /// And the ones the implementation will run under. Chosen separately
    /// because it is genuinely a separate account and model.
    pub implementation_pairing: Option<PairingView>,

    /// Whether everything needed before grilling will start is settled: both
    /// Pairings complete and neither Profile broken, a Brief with something in
    /// it, and a Conversation still drafting.
    ///
    /// The server's rule rather than something the page works out from the
    /// fields around it. Every one of the refusals is checked again when the
    /// button is pressed — this is what decides whether to offer the button, and
    /// what it says is true only as of the moment it was read.
    pub ready_to_grill: bool,

    /// What this Conversation is adopting, where it is adopting anything.
    ///
    /// `null` is the ordinary Conversation, which begins with a Brief and a
    /// grilling. Anything else is one started from the abandoned-roadmaps
    /// notice, and it is what puts the page on the adoption shape: the roadmap
    /// and its stage named, both Profiles and the base commit to fix, and one
    /// Adopt press — no Brief to write and no grilling to start.
    pub adopting: Option<AdoptionView>,

    /// The worktree the grilling was given to work in, once there is one.
    ///
    /// `null` both before grilling starts and after aborting — the two ways a
    /// Conversation has none, which are the same fact about it.
    pub worktree: Option<Worktree>,

    /// The latest pick: how the human most recently said the work should be
    /// built, on a proposal Set of this Conversation's.
    ///
    /// `null` until a proposal has been put to them and picked on. What the page
    /// *draws* of the choice is the answered Set on the Timeline, which is where
    /// it was made; this is the fact about the Conversation itself.
    pub direction: Option<Direction>,

    /// Which Event the Conversation is blocked on, or `null` where nothing is
    /// stopping it.
    ///
    /// What the *blocked on you* badge is drawn from. The Event id and not a
    /// flag, so that a header saying the work has stopped can point at the thing
    /// that stopped it — a Timeline is long by the time a run gets far enough to
    /// stop, and *blocked on you* with nowhere to go would be a badge the human
    /// had to go hunting behind.
    ///
    /// *Blocked on you* is a badge on an active state and never a state of its
    /// own, which is why this sits beside `state` rather than in it.
    pub blocked_on: Option<i64>,

    /// Which of this Conversation's sessions the human has the keyboard of, or
    /// `null` where it is Verkstead's.
    ///
    /// The Hold, said as the Event of the session it was taken on: the workbench
    /// draws the hand-back control on that session's Screen, and a Hold with no
    /// session to name would be one nobody could give back.
    ///
    /// Beside `blocked_on` rather than folded into it, though a Hold sets that
    /// too. What the badge says is *the work has stopped and it is your move*,
    /// and what this says is *which move* — where an Interruption is answered
    /// with a Remedy, a Hold is answered by handing the keyboard back.
    ///
    /// Never on the Timeline, however long it lasts: the Timeline records the
    /// work rather than the watching. This is a fact about now, read off the
    /// running server every time the Conversation is.
    pub held: Option<i64>,

    /// Whether a session is registered for this Conversation as of this read.
    ///
    /// The same fact the sidebar draws its working indicator from, said here
    /// because the Timeline has its own use for it: the Manual Task composer is
    /// offered exactly where nothing is running, and the states it is offered in
    /// are the ones a session may or may not be running in.
    ///
    /// A question about a process rather than about the record, so it is true
    /// only as of the moment it was read — and a restarted server has no
    /// sessions at all, so every Conversation then reads as not working, which
    /// is what each of them is.
    pub working: bool,

    /// Oldest first, which is reading order and puts the Brief at the top.
    pub timeline: Vec<TimelineEvent>,

    /// The Events that stay in view rather than scrolling past with the record.
    ///
    /// Apart from the Timeline rather than in it, because that is what pinning
    /// *is*: the list is a record of moments, and each of these is the current
    /// state of something the work is against. Empty is the ordinary case — a
    /// Conversation with no backlog has nothing to pin.
    pub pinned: Vec<PinnedEvent>,
}

/// The grilling's closing proposal as the Set it rides draws it: which direction
/// was recommended, and why.
///
/// The rationale arrives as HTML like every other piece of agent markdown on this
/// wire — the parser and the sanitizer are the server's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ProposalView {
    /// The direction the agent recommends, which the chooser marks.
    pub direction: Direction,

    /// Why, rendered and sanitized by the server on the way out.
    pub rationale_html: String,
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

    /// What a session printed, summarised. The whole of it is fetched
    /// separately, by the details pane and only when one is opened — see
    /// [`Capture`].
    AgentOutput(AgentOutputEvent),

    /// A Question Set the session put to the human, summarised as the table of
    /// what was asked against what was decided. The whole document is fetched
    /// separately, by the details pane, from the same endpoint the standalone
    /// Set page reads.
    QuestionSet(QuestionSetEvent),

    /// The handoff the grilling wrote on its way out, rendered inline like the
    /// Brief — and for the same reason: it is a document to read, and there is
    /// nothing of it a details pane would show that the Timeline does not.
    Handoff(HandoffEvent),

    /// A commit a session landed on the Conversation's branch, summarised as
    /// what it changed. Its diff is fetched separately, by the details pane —
    /// the same arrangement a Capture and a Question Set have, and for the
    /// same reason.
    Commit(CommitEvent),

    /// A run that stopped: what Verkstead noticed, the evidence it gathered, and
    /// the three remedies — offered on the Event itself, because the Timeline is
    /// where the human looks.
    ///
    /// The one kind of Event that carries its whole self rather than a summary
    /// with the rest behind a fetch. Its evidence is four short strings, gathered
    /// once and bounded when it was — see [`InterruptionEvent::tail`] — where a
    /// Capture and a diff are megabytes. A second request for what the
    /// remedies are being chosen against would be a page that could draw the
    /// buttons before it could say what they were for.
    Interruption(InterruptionEvent),

    /// Something Verkstead did on its own account, rendered inline like the
    /// Brief — and for the same reason: it is a sentence to read, and there is
    /// nothing of it a details pane would show.
    ///
    /// The one Event with nothing to do about it and nobody behind it: no agent
    /// wrote it and no human pressed anything for it. It is how an unattended run
    /// says what it decided while nobody was watching.
    Notice(NoticeEvent),

    /// A Manual Task the human set going by hand, rendered inline like the
    /// handoff — and for the same reason: it is what they asked for, in their
    /// own words, with nothing of it a details pane would add.
    ///
    /// What the session it started went on to do is not here. That lands as the
    /// Events any work lands as — what it printed, what it asked, what it
    /// committed — so this is the instruction alone, which is the part of a
    /// Manual Task nothing else on the Timeline records.
    ManualTask(ManualTaskEvent),
}

/// One of the three things the human can do about an Interruption.
///
/// Roadrunner's remedies and roadrunner's meanings. In every case the repository
/// is left as the session left it: none of the three reverts, resets or stashes
/// anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Remedy {
    /// Run the step again in a fresh session, told whatever the human wrote
    /// alongside.
    Retry,

    /// Verkstead stops driving, so the human can take the step on themselves.
    TakeOver,

    /// The run ends here.
    Abort,
}

/// A run that stopped, as the Timeline shows it: what went wrong, what the
/// evidence was, and how the human settled it.
///
/// The evidence is a reading of a Worktree and a session at the moment they went
/// wrong, taken then and kept — both move on, and a git status read when the page
/// looked would be a status of whatever happened next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct InterruptionEvent {
    pub id: i64,

    /// When the run stopped, RFC 3339.
    pub at: String,

    /// Which step failed, in words — "task 03 of the backlog".
    pub what: String,

    /// How it ended: the exit status, or that it ended without landing anything.
    pub how: String,

    /// What git made of the Worktree, as `git status` said it. Empty where the
    /// repository would not answer, or where there was nothing pending.
    pub git_status: String,

    /// The tail of what the session said: its own prose off the Transcript, or
    /// what it printed with the terminal's control sequences taken out where it
    /// kept no log. The tail and not the whole: what went wrong is at the end,
    /// and the whole of it is on the Timeline already as the session's own
    /// Event. Empty where it said nothing at all.
    pub tail: String,

    /// How the human settled it, or `null` while it is still open — which is the
    /// state the run is stopped in, and what the remedies are drawn for.
    pub settled: Option<RemedyTaken>,
}

/// How an Interruption was settled: which remedy, whatever the human wrote
/// alongside it, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RemedyTaken {
    pub remedy: Remedy,

    /// What the human wrote alongside the choice — "try again but leave the
    /// migration alone". Empty where they wrote nothing, which is the ordinary
    /// case for the two remedies that launch nothing.
    pub note: String,

    /// When they chose, RFC 3339.
    pub at: String,
}

/// The remedy the human is choosing, and what they want said alongside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RemedyChoice {
    pub remedy: Remedy,

    /// What to tell the fresh session, for a retry. Carried for the other two as
    /// well and recorded either way: a human who wrote why they were taking over
    /// has said something worth keeping on the record, even though nothing reads
    /// it back to an agent.
    pub note: String,
}

/// What became of choosing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum RemedySettled {
    /// Recorded, and the remedy has been acted on.
    Settled,

    /// This Conversation has no such Interruption — an Event id that belongs to
    /// another Conversation names nothing.
    NoSuchInterruption,

    /// It was settled before this arrived — from another device, or by a second
    /// press. Not an error and not something to act on twice: the first choice
    /// stands.
    AlreadySettled,
}

/// An Event the Timeline keeps in view rather than letting scroll past.
///
/// A fixed set — a task list, a stage list and a PR — and no manual pin or
/// unpin: what is pinned is decided by what kind of thing it is, so there is no
/// state here to flip and no route to flip it with. A tagged kind for the reason
/// [`TimelineEvent`] is one: what gets drawn turns on which kind it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum PinnedEvent {
    /// The backlog the Conversation's Worktree holds, and how far through it the
    /// work has got.
    TaskList(TaskListEvent),

    /// The roadmap the Conversation's Worktree holds, and how far through its
    /// stages the effort has got.
    StageList(StageListEvent),

    /// The pull request the finish step opened, which is what the work is being
    /// wrapped up on.
    PullRequest(PullRequestEvent),
}

/// The backlog as the Timeline shows it: what the work is called, and every
/// task against whether it is done.
///
/// No id and no stamp, unlike every Event in the record. It is read out of
/// `.tasks/` each time the Conversation is — the repository owns the files, and
/// Verkstead never does — so what it says is what the Worktree holds now rather
/// than what it held at a moment worth stamping. Nothing opens it either: the
/// whole of a task list is the list, which is why the design gives it no details
/// pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct TaskListEvent {
    /// What the backlog is called: `TODO.md`'s heading, which is the feature
    /// name the breaking-down session picked. Empty where it wrote none.
    pub feature: String,

    /// In the order the list has them, which is the order they get worked in.
    pub tasks: Vec<TaskEntry>,
}

/// One task of a backlog: the number it answers to, what it is called, and
/// whether it is done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct TaskEntry {
    /// As the list writes it, zero-padding and all — `01`. A Timeline that
    /// renumbered the backlog would be showing its own list rather than the
    /// repository's.
    pub number: String,

    pub title: String,

    /// Whether the task is finished, which is the task file having gone from
    /// `.tasks/`. That is the done-signal the task runner turns on, and a
    /// checkbox is how an entry is written rather than what says it is done.
    pub done: bool,
}

/// The roadmap as the Timeline shows it: what it is called, and every stage
/// against whether it is checked.
///
/// No id and no stamp, for the reason the task list beside it has none: it is
/// read out of `docs/roadmaps/` each time the Conversation is, so what it says
/// is what the Worktree holds now rather than what it held at a moment worth
/// stamping. Nothing opens it either — the whole of a stage list is the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct StageListEvent {
    /// The roadmap's directory under `docs/roadmaps/` — `mvp` — which is its
    /// identity: what a stage's brief sits beside, and what whoever starts the
    /// next stage is pointed at.
    pub name: String,

    /// `ROADMAP.md`'s own heading, which is prose the roadmap wrote about
    /// itself. Empty where it wrote none.
    pub title: String,

    /// In the order the roadmap has them, which is the order they get worked in.
    pub stages: Vec<StageEntry>,
}

/// One stage of a roadmap: the number it answers to, what it is called, and
/// whether it is done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct StageEntry {
    /// As the roadmap writes it, zero-padding and all — `01`.
    pub number: String,

    pub title: String,

    /// Whether the stage is finished, which here *is* the checkbox: a stage's
    /// brief stays where it is for ever, being the record of what the stage was
    /// for, so there is no file going away to read it off. The other way round
    /// from a task — see [`TaskEntry::done`].
    pub done: bool,
}

/// The pull request as the Timeline shows it: what it is called and what number
/// it answers to, with a way out to GitHub itself.
///
/// An id and a stamp, unlike the task list beside it, because this one *is* on
/// the record: the finish step opened a pull request at a moment worth keeping,
/// and the Conversation moved into Wrapping on the strength of it. What is not
/// on the record is what the PR holds — see [`PullRequestDetails`], which the
/// details pane fetches when somebody opens this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PullRequestEvent {
    pub id: i64,

    /// When it reached the Timeline, RFC 3339 — which is when Verkstead found
    /// it rather than when GitHub opened it, the two being a finish step apart.
    pub at: String,

    /// The number GitHub gave it, which is what everybody calls it by.
    pub number: i64,

    /// Its title, which is the feature name the finish step gave it.
    pub title: String,

    /// The whole URL, because merging is the human's act and this is the way to
    /// where they do it.
    pub url: String,
}

/// What is on a pull request now: the commits it carries, and what has been said
/// about it.
///
/// Its own request rather than a field on the Conversation, for the reason a
/// commit's diff is one — and for a further reason of its own: reading this is
/// asking GitHub, over the network, through the host's `gh`. A Timeline that
/// carried it would make an API call every time an open page heard the world
/// moved.
///
/// Fetched rather than remembered, in the same spirit the task list is read off
/// the Worktree: a PR is being worked on while the human is looking at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PullRequestDetails {
    /// Oldest first, as GitHub lists them, which is the order they landed.
    pub commits: Vec<PullRequestCommit>,

    pub comments: Vec<PullRequestComment>,
}

/// One commit of a pull request: what it is, and what it was called.
///
/// Not a [`CommitEvent`]: that is a commit Verkstead watched land on the branch
/// and put on the Timeline, with counts of what it moved and a diff behind it.
/// This is a line of a list GitHub keeps, and the two can differ — a branch that
/// was rebased after the PR opened has commits on the PR that are on no
/// Timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PullRequestCommit {
    /// The full hash, as everything on this wire carries one: the page shortens
    /// it for reading.
    pub sha: String,

    /// The first line of the commit message.
    pub subject: String,
}

/// One comment on a pull request: who said it, when, and what they said.
///
/// The body arrives rendered, like everything else an outsider wrote — a comment
/// is markdown, and it is markdown from the public internet, so it is sanitized
/// on this side of the wire rather than put in a page raw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PullRequestComment {
    /// The login of whoever left it. Empty where the account has gone, which is
    /// a comment to draw rather than a reason to refuse the pane.
    pub author: String,

    /// When it was made, RFC 3339, as GitHub stamped it.
    pub at: String,

    pub html: String,
}

/// A commit as the Timeline shows it: what it was called, and how much of the
/// repository it moved.
///
/// The line and not what is behind it. A commit's summary and its diff are what
/// the details pane fetches — see [`CommitPane`] — and the Timeline is re-read
/// every time an open page hears the world moved.
///
/// There is no state here and no action on it. Commits are viewable and nothing
/// else: the design gives them no per-commit review, because feedback about the
/// work consolidates in the wrap-up phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CommitEvent {
    pub id: i64,

    /// When it reached the Timeline, RFC 3339. Which is when Verkstead noticed
    /// it rather than when it was committed — the two are a poll apart, and the
    /// Timeline is a record of what Verkstead saw happen.
    pub at: String,

    /// The full hash. Full, because what it takes for a short one to be
    /// unambiguous grows with the repository — the page shortens it for
    /// reading, which is a different thing from recording one shortened.
    pub sha: String,

    /// The first line of the commit message.
    ///
    /// It comes from the Event because it cannot come from the diff: the diff
    /// arrives headerless, which is what lets it be rendered by the same
    /// renderer an attached Diff is.
    pub subject: String,

    pub files: i64,
    pub insertions: i64,
    pub deletions: i64,
}

/// One commit, as the details pane receives it: what it said about itself, and
/// what it changed.
///
/// Its own request rather than a field on the Conversation, for the reason a
/// Capture is: a Timeline is read every time an open page hears the world
/// moved, and a commit is read whole when somebody opens the one Event it
/// belongs to.
///
/// The diff is rendered with the folds and the highlighting an attached Diff
/// already gets, because it is the same renderer on the same kind of input — see
/// [`crate::diff`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CommitPane {
    /// The Commit Summary, rendered and sanitized like every other document an
    /// agent wrote — `null` where the commit carried none, which is every
    /// bookkeeping commit and every commit recorded before summaries were kept.
    pub summary: Option<String>,

    /// `null` where the commit changed nothing a diff can show, which is a merge
    /// or an empty commit. A commit the repository no longer has is not this: it
    /// is a 404, because there is nothing there to draw a pane about.
    pub diff: Option<DiffView>,
}

/// The handoff document as the page receives it.
///
/// HTML alone, unlike the Brief, which travels as its source as well: the Brief
/// is the one document on this wire the human edits, and this one nobody does.
/// It is the agent's account of what was settled, fixed at the moment the
/// grilling ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct HandoffEvent {
    pub id: i64,

    /// When it reached the Timeline, RFC 3339.
    pub at: String,

    /// Rendered and sanitized by the server on the way out, as every piece of
    /// agent markdown on this wire is.
    pub html: String,
}

/// A notice as the page receives it: what Verkstead did, and when.
///
/// HTML alone, like the handoff and unlike the Brief: nobody edits it. Rendered
/// on the way out all the same, because a notice says which stage it started and
/// what the branch is called, and those are worth setting in a code span rather
/// than running into the prose around them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct NoticeEvent {
    pub id: i64,

    /// When it was said, RFC 3339.
    pub at: String,

    /// Rendered and sanitized by the server on the way out, as every piece of
    /// markdown on this wire is.
    pub html: String,
}

/// A Manual Task as the page receives it: what the human asked for, and when.
///
/// HTML alone, like the handoff and unlike the Brief: it is a moment on the
/// record rather than a document anybody goes back and edits — what a second
/// thought produces is a second Manual Task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ManualTaskEvent {
    pub id: i64,

    /// When it was asked for, RFC 3339.
    pub at: String,

    /// Rendered and sanitized by the server on the way out, as every piece of
    /// markdown on this wire is.
    pub html: String,
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

/// A session's output as the Timeline shows it: how far its conversation has
/// got, the last thing that was said, and whether more is coming.
///
/// The summary and not the Capture. A grilling session prints megabytes over
/// an hour, and the Timeline is re-read every time an open page hears the world
/// moved — so what a Conversation carries is these two lines, and the Capture
/// is fetched by the pane that shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AgentOutputEvent {
    pub id: i64,

    /// When the session started, RFC 3339.
    pub at: String,

    /// How many lines it has printed.
    pub lines: i64,

    /// How many turns its conversation has taken, as the Transcript pane draws
    /// them — which is the metric the row and the pane show.
    ///
    /// `null` where the session keeps no log, and the two places that show this
    /// show nothing at all rather than a zero: a session with no Transcript has
    /// no turns to be wrong about. Not the same as a count of none, which is a
    /// session that has a log and has not said anything into it yet.
    pub turns: Option<i64>,

    /// The last thing the agent said, off its own log — or, where it kept none,
    /// the last line it printed with the terminal's control sequences taken
    /// out. Empty where it has said nothing yet.
    pub latest: String,

    /// Whether the session writing this is still running.
    ///
    /// Not something the record holds: a running session is a process, and what
    /// knows about one is the server that started it. A Verkstead that has been
    /// restarted has no sessions, which is why this is read off what is running
    /// rather than off what was written.
    pub running: bool,

    /// And whether that session has stopped printing — quiet long enough for
    /// the mark to say it is sitting there rather than working.
    ///
    /// Beside `running` rather than instead of it, because the two are
    /// different questions and a page draws three answers from them: no mark,
    /// a turning ring, and a still one. Always `false` where nothing is
    /// running, which is what makes those three the only ones there are.
    ///
    /// Computed on every read, off the same clock that ends a session that has
    /// gone quiet — so a page opened onto a session that has been idle for an
    /// hour says so at once rather than waiting to be told.
    pub idle: bool,
}

/// A Question Set as the Timeline shows it: what it was called, the table of
/// what was asked against what was decided, and where it stands.
///
/// The table and not the Set. The design gives a Question Set a summary of
/// number, question and answer in the Timeline and the whole document in the
/// details pane, and the two are different sizes: a Set carries a Preface, every
/// Option of every Question and the whole uncommitted Diff of the repository it
/// was asked from, and the Timeline is re-read every time an open page hears the
/// world moved.
///
/// `set_id` is what the details pane fetches the document by — the same
/// `/api/ui/sets/{id}` the standalone page reads, because it is the same Set
/// reached another way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct QuestionSetEvent {
    pub id: i64,

    /// When the Set was put, RFC 3339.
    pub at: String,

    pub set_id: i64,

    pub title: String,

    /// One row per Question and Sub-question, in the order the agent asked
    /// them.
    pub rows: Vec<SetRow>,

    /// Whether it is still waiting on the human, and what became of it if not.
    /// The same verdict the Set's own page carries, from the same registry of
    /// held waits — this is a Timeline the human answers from.
    pub standing: Standing,
}

/// One row of a Question Set's Timeline table: the number it answers to, what
/// was asked, and what was decided.
///
/// Plain words in both columns rather than the rendered HTML the Set page draws.
/// A row is one line in a list of Events — the markup would have to come back
/// out to fit, which means a parser on the browser's side of the wire, the one
/// thing rendering on the server is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SetRow {
    /// `Q7` for a Question, `Q7a` for a Sub-question.
    pub name: String,

    /// Whether this row is a Sub-question, so the table can indent it under the
    /// Question it belongs to.
    pub nested: bool,

    /// What was asked, as plain words.
    pub question: String,

    /// What was decided, as plain words: the Option that was chosen, whatever
    /// was written, or both. Empty where nothing was — a Question left open, a
    /// Heading that asked nothing, or a Set still waiting on the human. Which of
    /// those it is, the Set's `standing` says.
    pub answer: String,
}

/// One session's Capture, whole, as the details pane receives it.
///
/// Byte for byte, control sequences and all: what a terminal was sent is what a
/// session said, and a Capture that had been tidied up would be a record of
/// something else.
///
/// Not quite all of it is the agent's own word. What bwrap says when it will
/// not start is said on the terminal Verkstead began it on, so it arrives here
/// too, where it happened and in among whatever else was printed. On a session
/// that never started it is the only account of why — which makes the Capture
/// the place for it, being where somebody looking at a session that said
/// nothing is already looking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Capture {
    pub text: String,
}

/// One session's Screen: the grid its Capture leaves on a terminal.
///
/// Not the bytes and not a picture of them — the escape sequences that would
/// paint the grid as it stands, which is what the terminal in the details pane
/// is fed. The server holds the terminal that decided them and hands over the
/// repaint; the browser's copy is a window onto that one rather than a second
/// opinion about it (ADR 0007).
///
/// The size comes with it because a repaint means nothing without one: the same
/// sequences put a session's display in different places on a grid of a
/// different width.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Screen {
    pub repaint: String,
    pub columns: u16,
    pub rows: u16,
}

/// What the server says down a live Screen's socket.
///
/// Watching a running session is the one place the viewer is sent something
/// rather than fetching it, so the two things it can be sent say which they are
/// rather than being told apart by shape. A repaint arrives first and whenever
/// the grid has been resized under everybody; what the session printed arrives
/// as it prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Shown {
    /// The whole grid as it stands — see [`Screen`]. The first thing a watcher
    /// is sent, so that one attaching halfway through a session sees the
    /// session rather than the rest of it.
    Painted(Screen),

    /// What the session has printed since the last thing said here, to be
    /// written on the terminal the repaint painted.
    Printed(String),
}

/// And what a watcher says back up it.
///
/// Three kinds of thing, each saying which it is: the socket is a conversation
/// in both directions, and what a watcher does to a Screen is look at it a
/// different size, type into it, or move a mouse over it.
///
/// The last two carry the same thing — bytes on their way to the session's own
/// terminal — and are told apart for one reason, which is the Hold. Typing
/// takes it and mousing never does, so which of the two the human did has to
/// survive the crossing rather than be guessed at from the bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Watching {
    /// How big this watcher's window is. The latest one wins for everybody —
    /// there is one Screen however many devices are watching it — and it
    /// reaches the session's own terminal, so its interface redraws to fit.
    Resized(Size),

    /// What the human typed, on its way to the session's own terminal.
    ///
    /// The first of these takes the Hold: from then on Verkstead records and
    /// nothing else until the keyboard is handed back — see the Hold in
    /// `CONTEXT.md`. Whatever the terminal makes of it comes back the ordinary
    /// way, in among what the session printed, because that is the one account
    /// of what happened.
    ///
    /// Text rather than a key: what a terminal takes is bytes, and the browser's
    /// own terminal has already turned a keypress into the ones a session
    /// expects.
    Typed(String),

    /// What the mouse did, on its way to the same terminal.
    ///
    /// A session whose interface tracks the mouse is sent a report of every
    /// move, click and scroll over its Screen, down the path a keystroke takes
    /// — so one of these is a keystroke in every respect but the one that
    /// matters here: **it never takes the Hold**. The Hold is the human
    /// deliberately intervening, and a cursor crossing a live Screen is not
    /// that.
    ///
    /// Written through whether the Conversation is held or not, exactly as
    /// [`Watching::Typed`] is: a human mid-intervention uses the mouse as much
    /// as the keyboard.
    Moused(String),
}

/// What handing a Conversation's keyboard back came to.
///
/// The one way a Hold ends, and it ends by being pressed: no timeout, no release
/// on the socket dropping, because Verkstead resuming over a half-finished
/// intervention is worse than a stalled run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum HandedBack {
    /// The Hold is over: Verkstead has the Conversation again, and whatever the
    /// human left is judged by the ordinary end-of-session rules.
    HandedBack,

    /// There was no Hold to end. The same answer arriving twice — a second
    /// device, or a press repeated — rather than a refusal.
    NotHeld,
}

/// How big a Screen is, in characters.
///
/// Named for the Screen rather than for the window it was measured in, because
/// that is what it becomes: a watcher reports the size of the pane it drew, and
/// the latest one is the size the Screen and the session's own terminal are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Size {
    pub columns: u16,
    pub rows: u16,
}

/// A move as an Event. Nothing to render — see [`MovedEvent`] — but built here
/// beside the Brief so that one place knows how a Timeline is made.
pub fn moved_event(id: i64, at: String, state: Lifecycle) -> TimelineEvent {
    TimelineEvent::Moved(MovedEvent { id, at, state })
}

/// The grilling's proposal as the Set carrying it needs it, with the rationale
/// rendered on the way.
///
/// Here rather than in the server for the reason the Brief's rendering is: this
/// is the crate with the markdown parser in it.
pub fn proposal_view(proposal: &verkstead_schema::Proposal) -> ProposalView {
    ProposalView {
        direction: proposal.direction,
        rationale_html: crate::markdown::to_html(&proposal.rationale),
    }
}

/// A session's output as an Event. Nothing to render either — the summary was
/// worked out as the output arrived — and here for the same reason as the move.
pub fn agent_output_event(
    id: i64,
    at: String,
    lines: i64,
    turns: Option<i64>,
    latest: String,
    running: bool,
    idle: bool,
) -> TimelineEvent {
    TimelineEvent::AgentOutput(AgentOutputEvent {
        id,
        at,
        lines,
        turns,
        latest,
        running,
        // Idle is a thing a running session is, and the caller reads the two
        // off different places — so the pair is made consistent here rather
        // than at each of them.
        idle: running && idle,
    })
}

/// A Question Set as an Event, summarised on the way.
///
/// The Answers come out of `standing` rather than beside it: a Set that has not
/// settled has none, one that was archived unanswered never will have, and one
/// that was answered carries them — so the one field decides both what the table
/// says and how it reads.
pub fn question_set_event(
    id: i64,
    at: String,
    set_id: i64,
    set: &verkstead_schema::QuestionSet,
    standing: Standing,
) -> TimelineEvent {
    let response = match &standing {
        Standing::Answered(answered) => Some(&answered.response),
        Standing::Waiting(_) | Standing::ArchivedUnanswered(_) => None,
    };

    TimelineEvent::QuestionSet(QuestionSetEvent {
        id,
        at,
        set_id,
        title: set.title.clone(),
        rows: asked(set, response),
        standing,
    })
}

/// The Set's Questions and Sub-questions as the Timeline's table, in the order
/// the agent asked them, each against whatever became of it.
fn asked(
    set: &verkstead_schema::QuestionSet,
    response: Option<&verkstead_schema::Response>,
) -> Vec<SetRow> {
    let mut rows = Vec::new();

    for question in &set.questions {
        rows.push(SetRow {
            name: question.name().to_owned(),
            nested: false,
            question: crate::markdown::to_plain(&question.text),
            // A Heading has no Answer and never will: it heads its
            // Sub-questions rather than asking anything. Nothing is done about
            // that here — there is no entry to find, so the column comes out
            // empty on its own.
            answer: decided(response, question.name(), &question.options),
        });

        for subquestion in &question.subquestions {
            let name = subquestion.name(question);

            rows.push(SetRow {
                answer: decided(response, &name, &subquestion.options),
                name,
                nested: true,
                question: crate::markdown::to_plain(&subquestion.text),
            });
        }
    }

    rows
}

/// What became of one question, as the one line the table gives it: the Option
/// that was chosen, whatever was written, or both.
///
/// Empty where nothing was decided, which the row is drawn from rather than
/// worded here — a question left open on an answered Set and one on a Set nobody
/// has reached yet are the same emptiness, and the Set's standing is what tells
/// them apart.
fn decided(
    response: Option<&verkstead_schema::Response>,
    name: &str,
    options: &[verkstead_schema::QuestionOption],
) -> String {
    let Some(answer) = response.and_then(|response| {
        response
            .answers
            .iter()
            .find(|answer| answer.label.trim() == name)
    }) else {
        return String::new();
    };

    let chosen = answer
        .selected
        .and_then(|n| options.iter().find(|option| option.n == n))
        .map(|option| crate::markdown::to_plain(&option.text));

    let said = answer
        .free_text
        .as_deref()
        .map(str::trim)
        .filter(|said| !said.is_empty())
        .map(crate::markdown::to_plain);

    // Both where the human picked an Option and said why, which is the ordinary
    // shape of an Answer that carries a qualification.
    match (chosen, said) {
        (Some(chosen), Some(said)) => format!("{chosen} — {said}"),
        (Some(only), None) | (None, Some(only)) => only,
        (None, None) => String::new(),
    }
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

/// A commit as an Event. Nothing to render — the summary is five facts git
/// counted — and here beside the move for the reason that one is: one place
/// knows how a Timeline is made.
pub fn commit_event(id: i64, at: String, commit: CommitSummary) -> TimelineEvent {
    TimelineEvent::Commit(CommitEvent {
        id,
        at,
        sha: commit.sha,
        subject: commit.subject,
        files: commit.files,
        insertions: commit.insertions,
        deletions: commit.deletions,
    })
}

/// What the caller of [`commit_event`] hands over: the commit as the store
/// holds it.
///
/// Its own type rather than the store's, because this crate does not depend on
/// the store — and rather than six parameters, because five of them are numbers
/// and a subject, and a call with those in the wrong order would compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSummary {
    pub sha: String,
    pub subject: String,
    pub files: i64,
    pub insertions: i64,
    pub deletions: i64,
}

/// One commit as the details pane receives it, rendered on the way.
///
/// Here rather than in the server for the reason the Brief's rendering is: this
/// is the crate with the markdown, the diff parser and the highlighter in it. A
/// patch with nothing in it comes back as nothing to show, exactly as an empty
/// attached Diff does — and so does a summary of nothing but whitespace, which
/// the pane would otherwise draw as a gap above the diff.
pub fn commit_pane(summary: Option<&str>, patch: &str) -> CommitPane {
    CommitPane {
        summary: summary
            .map(crate::markdown::to_html)
            .filter(|html| !html.trim().is_empty()),
        diff: crate::diff::to_html(patch),
    }
}

/// A backlog as the Event that gets pinned. Nothing to render — a task is a
/// number, a title and whether its file is still there — and here beside the
/// rest for the reason a move is: one place knows how a Timeline is made.
pub fn task_list_event(feature: String, tasks: Vec<TaskEntry>) -> PinnedEvent {
    PinnedEvent::TaskList(TaskListEvent { feature, tasks })
}

/// A roadmap as the Event that gets pinned. Nothing to render either — a stage
/// is a number, a title and a ticked box.
pub fn stage_list_event(name: String, title: String, stages: Vec<StageEntry>) -> PinnedEvent {
    PinnedEvent::StageList(StageListEvent {
        name,
        title,
        stages,
    })
}

/// A pull request as the Event that gets pinned. Nothing to render — a PR is a
/// number, a title and a URL — and here beside the rest for the reason a move
/// is: one place knows how a Timeline is made.
pub fn pull_request_event(id: i64, at: String, opened: PullRequestSummary) -> PinnedEvent {
    PinnedEvent::PullRequest(PullRequestEvent {
        id,
        at,
        number: opened.number,
        title: opened.title,
        url: opened.url,
    })
}

/// What the caller of [`pull_request_event`] hands over: the pull request as the
/// store holds it.
///
/// Its own type rather than the store's, because this crate does not depend on
/// the store — and rather than three parameters, two of which are strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestSummary {
    pub number: i64,
    pub title: String,
    pub url: String,
}

/// What a pull request holds, as the details pane receives it: the commit list
/// as it stands, and every comment rendered from the markdown it was written in.
///
/// The comments are the only part with anything to render, and they are the part
/// that most needs it: a PR comment is markdown written by whoever can reach the
/// repository, so it is sanitized here rather than in a browser.
pub fn pull_request_details(
    commits: Vec<PullRequestCommit>,
    comments: Vec<Comment>,
) -> PullRequestDetails {
    PullRequestDetails {
        commits,
        comments: comments
            .into_iter()
            .map(|comment| PullRequestComment {
                author: comment.author,
                at: comment.at,
                html: crate::markdown::to_html(&comment.markdown),
            })
            .collect(),
    }
}

/// What the caller of [`pull_request_details`] hands over for each comment: what
/// GitHub said about it, with the body still as its author wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub author: String,
    pub at: String,
    pub markdown: String,
}

/// A run that stopped, as an Event. Nothing to render — the evidence is four
/// strings read off a Worktree and a terminal, and neither git status nor a
/// session's last words are markdown — and here beside the rest for the reason a
/// move is: one place knows how a Timeline is made.
pub fn interruption_event(id: i64, at: String, stopped: Stopped) -> TimelineEvent {
    TimelineEvent::Interruption(InterruptionEvent {
        id,
        at,
        what: stopped.what,
        how: stopped.how,
        git_status: stopped.git_status,
        tail: stopped.tail,
        settled: stopped.settled,
    })
}

/// What the caller of [`interruption_event`] hands over: the Interruption as the
/// store holds it.
///
/// Its own type rather than the store's, because this crate does not depend on
/// the store — and rather than five parameters, because four of them are strings
/// and a call with those in the wrong order would compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stopped {
    pub what: String,
    pub how: String,
    pub git_status: String,
    pub tail: String,
    pub settled: Option<RemedyTaken>,
}

/// The handoff as an Event, rendered on the way — the same rendering the Brief
/// gets, because it is the same kind of thing: markdown somebody wrote for
/// somebody else to read.
pub fn handoff_event(id: i64, at: String, markdown: &str) -> TimelineEvent {
    TimelineEvent::Handoff(HandoffEvent {
        id,
        at,
        html: crate::markdown::to_html(markdown),
    })
}

/// A notice as an Event, rendered the same way and for the same reason: it is a
/// sentence somebody has to be able to read.
pub fn notice_event(id: i64, at: String, markdown: &str) -> TimelineEvent {
    TimelineEvent::Notice(NoticeEvent {
        id,
        at,
        html: crate::markdown::to_html(markdown),
    })
}

/// A Manual Task as an Event, rendered the same way and for the same reason: it
/// is what the human asked for, written for somebody to read back.
pub fn manual_task_event(id: i64, at: String, instruction: &str) -> TimelineEvent {
    TimelineEvent::ManualTask(ManualTaskEvent {
        id,
        at,
        html: crate::markdown::to_html(instruction),
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

/// Starting one to adopt a roadmap with: which Repo, and which of its roadmaps.
///
/// What the notice sends when a roadmap in it is clicked. The roadmap is named
/// by its directory under `docs/roadmaps/`, which is its identity here as
/// everywhere else — the stage is not sent, because which stage is next is the
/// roadmap's own answer at whatever commit the Conversation ends up branching
/// from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct NewAdoption {
    pub repo_id: i64,
    pub roadmap: String,
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

/// What the human typed into the Manual Task composer: the instruction, and the
/// Agent Profile to run it under.
///
/// The Profile travels with the instruction rather than being read off the
/// Conversation, because the pick is one-off. The composer starts on the
/// Conversation's implementation Profile and a different choice belongs to this
/// submission alone — it never becomes the Conversation's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ManualTaskSubmission {
    /// What to do, in the human's own markdown. Nothing here interprets it — it
    /// goes on the Timeline whole and into the prompt whole.
    pub instruction: String,

    /// Which saved Profile the one-off session runs as.
    pub profile_id: i64,

    /// And which of that Profile's models it runs on. The composer prefills the
    /// Conversation's implementation Pairing and otherwise demands a pick:
    /// there is no default model anywhere.
    pub model: String,
}

/// What became of submitting one.
///
/// Named the way [`GrillingStarted`]'s refusals are, and for the same reason:
/// each of them is something different for the human to go and do, and a single
/// "cannot start" would leave them guessing which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ManualTaskStarted {
    /// The instruction is on the Timeline and a session is running on it.
    Started,

    NoSuchConversation,

    /// It is drafting or aborted, so it has no Worktree for a session to run in.
    /// The two states the composer is never offered in.
    NowhereToWork,

    /// A session was registered when this arrived, so the composer that was
    /// pressed was stale. Nothing is queued: an instruction written against a
    /// world that has since moved may no longer be the thing to do.
    AlreadyRunning,

    /// Nothing was typed, and an instruction is the whole of what a Manual Task
    /// is.
    EmptyInstruction,

    /// The picked Profile has gone — deleted between the page being drawn and
    /// the press.
    NoSuchProfile,

    /// It is still there and no longer lists the model picked beside it — its
    /// list was edited between the page being drawn and the press.
    NoSuchModel,

    /// The instruction is on the Timeline and no session could be started for
    /// it. The reason is in the server's log, as a worktree git refused is: this
    /// is the one refusal with nothing for the human to correct.
    NotStarted,
}

/// What became of pressing Adopt.
///
/// Named the way [`GrillingStarted`]'s refusals are, and for the same reason: a
/// human is at the workbench pressing the button, and each of these is
/// something different for them to go and do. What is decided while nobody is
/// watching says itself on a Timeline instead — see the server's `continuing`
/// module, which starts the same stage by the other route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Adopted {
    /// The branch and the worktree are made, the stage brief is the Brief, and
    /// the Conversation is implementing the stage.
    Adopted,

    NoSuchConversation,

    /// It is past drafting, so it has been adopted once already — or aborted.
    NotDrafting,

    /// It is adopting nothing, which is every Conversation that began with a
    /// Brief and a grilling. There is no roadmap here to take a stage from.
    NotAdopting,

    /// No Agent Profile is chosen for the grilling. Carried by an adopted stage
    /// rather than run under: every stage after it inherits both Profiles from
    /// its predecessor, and a Conversation that is reopened is grilled.
    NoGrillingProfile,

    /// And none is chosen for the implementation, which is what the stage's own
    /// work runs under.
    NoImplementationProfile,

    /// A chosen Profile's pair is not where it was left, so there is no account
    /// to run the session under.
    ProfileBroken,

    /// Nothing in the repository answers to what the stage would branch from —
    /// an overridden commit that has gone, or a default branch that has.
    NoBaseCommit,

    /// No roadmap by that name is readable at the base commit, or what is there
    /// plans nothing.
    NoRoadmap,

    /// Every stage of it is ticked. The roadmap finished — between the notice
    /// being drawn and the button being pressed, if it had a stage a moment
    /// ago.
    RoadmapComplete,

    /// The next stage names a brief that cannot be read at the base commit. The
    /// roadmap's own to fix: starting the stage after it instead would be
    /// Verkstead deciding to skip work.
    NoBrief,

    /// The next stage is annotated with a branch that still exists, so somebody
    /// or something is already on it.
    StageInFlight,

    /// The stage's own slug branch is already there. Verkstead did not make it
    /// for this, so it will not take it over — and a branch git would not answer
    /// about counts as one that is there.
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
