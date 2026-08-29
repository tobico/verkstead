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

use crate::{DiffView, PairingView, PickedView, ProfileChoice, RepoEntry, Standing};

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

    /// Beside the ladder rather than on it: the human following up on work that
    /// is already on a pull request, in a session they steered into being.
    /// Reachable from Done and from Wrapping, and leading back into the
    /// wrap-up.
    FollowUp,

    Done,

    /// Off the ladder rather than on it: the work stopped wherever it had got
    /// to. Reachable from every other state, and leading nowhere.
    Closed,
}

/// One row of the conversations sidebar.
///
/// The branch is the row's name where somebody has settled on one: a
/// Conversation has no title of its own, and of what it does have the branch is
/// the short line a human chose. A draft nobody has named carries a name
/// Verkstead invented instead, and reads *Draft* — see [`Self::branch_named`].
///
/// Where it has got to is drawn rather than worded — a turning ring for a
/// session getting on with it, the same ring empty for one that has gone quiet,
/// a dot for a Conversation that wants answering or has news on it, a dotted
/// border for a draft and a dimmed card for work that has stopped. Which is why
/// the facts below are facts and not one collapsed verdict: the row says what is
/// true of the Conversation, and which mark that comes out as is the one rule
/// the viewer keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ConversationEntry {
    pub id: i64,
    pub branch: String,

    /// Whether that name is one somebody settled on, rather than the one
    /// Verkstead prefilled the record with.
    ///
    /// A name nobody chose says nothing about the work, so a draft carrying one
    /// is drawn as *Draft* and the name itself is drawn nowhere — in the title
    /// and in what is read aloud alike. Two drafts against one Repo reading the
    /// same is what two drafts are. Once the work has started the branch is a
    /// fact rather than a plan, and a fact is named wherever it is reported.
    pub branch_named: bool,

    /// And whether the name it is carrying is still the first session's to
    /// replace.
    ///
    /// The work starting is not what makes an invented name worth drawing: the
    /// first session is told to switch the branch to something the Brief is
    /// about, and until it has the name says no more than it did while this was
    /// a draft. So the row goes on reading *Draft* while this is true, and reads
    /// the branch the moment it is not — the session renamed it, or the session
    /// ended and the name it left is the one this is called by.
    ///
    /// Always `false` where the human typed a name: there was never anything to
    /// wait for.
    pub naming: bool,

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
    /// left open, or driving that has stopped.
    ///
    /// Folded from every source before it leaves, so the viewer holds no list of
    /// them. A Draft is never one of them: it is drawn as a draft, and that is
    /// the whole of what a draft has to say.
    pub waiting: bool,

    /// Whether this one is a wrap-up that has narrowed to its checks: the review
    /// and the comments settled, the checks not, and nothing running on it.
    ///
    /// A derived condition of Wrapping rather than a state, which is why it sits
    /// beside `state` the way *blocked on you* does rather than in it. Nothing
    /// is stored for it: it is the wrap-up's own settle facts read a particular
    /// way, folded here so the row does not have to.
    ///
    /// The row draws no state in words, so what this comes out as is the label
    /// read aloud — *Waiting on checks* where the plain state word would be.
    pub waiting_on_checks: bool,

    /// Whether Verkstead has told the human something about this Conversation
    /// that they have not looked at yet.
    ///
    /// One thing writes it: the wrap-up that carries a Conversation to Done and
    /// pushes the news to the devices, in the same breath as the push. A
    /// milestone nobody was watching happen is what a mark saying *look here*
    /// is for, and a Done the human steered to themselves is what it is not.
    ///
    /// Beside `waiting` rather than folded into it, because the row draws one
    /// disc for the two and says which of them it is in the label read aloud:
    /// *something wants you* against *there is news here*. Cleared by opening
    /// the Conversation, which the browser says in a call of its own.
    pub unseen: bool,
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

    /// Whether that name is one somebody settled on, rather than the one
    /// Verkstead prefilled the record with — see
    /// [`ConversationEntry::branch_named`], which is the same fact drawn as a
    /// title.
    ///
    /// What the pane does about a draft that says no: the header reads *Draft*,
    /// and the setup card's branch field stands empty under the placeholder
    /// saying what leaving it empty means. The name itself is drawn nowhere.
    pub branch_named: bool,

    /// And whether the name it is carrying is still the first session's to
    /// replace — see [`ConversationEntry::naming`], which is the same fact drawn
    /// as a title.
    ///
    /// The header keeps reading *Draft* through it, the setup card having gone
    /// by then. What the card would have shown is settled: the branch is cut and
    /// the field is frozen, whatever the name on it turns out to be.
    pub naming: bool,

    /// The commit the work will branch from, where the human overrode the rule.
    /// `null` is the rule itself: the default branch's tip, as it stands when
    /// grilling starts — which is why there is no value here to show instead.
    pub base_commit: Option<String>,

    /// The other registered Repos this Conversation works alongside, by name.
    ///
    /// Empty is the ordinary Conversation. Beside the branch and the base
    /// because it is the same kind of fact — what the work is configured with —
    /// and settled in the same place and at the same moment: the setup card
    /// while the Brief drafts, frozen when grilling starts.
    pub companions: Vec<CompanionView>,

    pub state: Lifecycle,

    /// The Profile and model the grilling session will run under, whole rather
    /// than by id: the pane says what they are, and whether the Profile is
    /// still runnable.
    ///
    /// One of the two roles the picker offers a row that runs no session for,
    /// so this says which of three the human picked rather than whether they
    /// picked at all. A Conversation that picked *no grilling* is not grilled:
    /// its Brief goes straight to an inline implementation.
    pub grilling_pairing: PickedView,

    /// And the ones the implementation will run under. Chosen separately
    /// because it is genuinely a separate account and model.
    pub implementation_pairing: Option<PairingView>,

    /// And what the wrap-up's review session will run under, chosen separately
    /// again: reviewing is a fresh set of eyes on what was built.
    ///
    /// The other role the picker offers a row that runs no session for, so this
    /// says which of three the human picked rather than whether they picked at
    /// all.
    pub review_pairing: PickedView,

    /// Whether everything needed before the work will start is settled: every
    /// role picked — a Pairing complete and no Profile broken, or the row that
    /// runs no session — a Brief with something in it, and a Conversation still
    /// drafting.
    ///
    /// The server's rule rather than something the page works out from the
    /// fields around it. Every one of the refusals is checked again when the
    /// button is pressed — this is what decides whether to offer the button, and
    /// what it says is true only as of the moment it was read.
    pub ready_to_grill: bool,

    /// Whether to warn, where the work is started from, that this repository's
    /// dependency compiles will not be cached.
    ///
    /// True on three things at once: the Repo is a Cargo workspace — a
    /// `Cargo.toml` at its root — the shared Rust build cache is switched on,
    /// and the server found no sccache to compile through. Then a session's
    /// crate downloads are shared and its dependencies are compiled from
    /// scratch every time, which is a slow build rather than a broken one — so
    /// it is a note above the button, not a refusal on it.
    ///
    /// The server's rule rather than three fields for the page to combine, for
    /// the reason [`ConversationView::ready_to_grill`] is one: two of the three
    /// are facts about the server that nothing else on this payload carries.
    pub compiles_uncached: bool,

    /// Whether there is driving to start again: the Conversation is in a state
    /// something ought to be driving, and nothing is.
    ///
    /// What decides whether Resume is offered, and the server's rule rather
    /// than something the page works out from the fields around it — *driven*
    /// is a register of tasks that are running, and a page cannot see one.
    /// Every refusal is checked again when the button is pressed; this says
    /// only that it was worth offering as of the moment it was read.
    ///
    /// A question about a process as much as about the record, so a restarted
    /// server reads every Conversation it left mid-run as one to resume —
    /// which each of them is.
    pub ready_to_resume: bool,

    /// And whether there is driving to stop: the Conversation is in a state
    /// something ought to be driving, and it has not stopped.
    ///
    /// What decides whether Stop and Force stop are offered. Not the mirror of
    /// [`ready_to_resume`]: a Conversation between one step and the next has
    /// both, because nothing is running now and the run is going to launch
    /// something the moment it can. A quiet Conversation is one to stop as much
    /// as a busy one.
    ///
    /// Force stop is offered where this and [`working`] are both true — the
    /// stop that ends a session is worth offering only where there is one.
    ///
    /// [`ready_to_resume`]: ConversationView::ready_to_resume
    /// [`working`]: ConversationView::working
    pub ready_to_stop: bool,

    /// And whether a stop has already been asked for and is waiting for the step
    /// the run is on to finish.
    ///
    /// What takes **Stop** off the menu, the press having been made: it is
    /// recorded, the run halts the moment the step lands, and a row still
    /// offering it would be Verkstead asking for a decision it already has.
    /// Force stop is left where it is — it is the escalation from here, and the
    /// one thing a human who has changed their mind about waiting can still
    /// press.
    ///
    /// Beside [`ready_to_stop`] rather than folded into it, because the two say
    /// different things: that one is *there is a run to stop*, which is what
    /// draws Force stop, and this is *and you have already said so*.
    ///
    /// [`ready_to_stop`]: ConversationView::ready_to_stop
    pub stop_asked: bool,

    /// And whether a steer into Implementing has anything to carry on: the
    /// branch holds a backlog with work left in it, or a roadmap it has
    /// written.
    ///
    /// What decides whether the steer modal offers *carrying on* — the target
    /// itself is offered on every Conversation there is, because an instruction
    /// can always be written. Where this is false the instruction is the whole
    /// of what that target can be, so the modal requires one.
    ///
    /// The server’s rule rather than something the page works out from the
    /// fields around it: what stands is a reading of the Worktree as it is now,
    /// which a page cannot make. Read the same way everything else pinned to
    /// the Timeline is.
    ///
    /// **A Worktree that has gone is not a branch holding nothing**, and this is
    /// true there. There is no directory to read, and the steer checks one out
    /// of the branch before anything runs in it — so a Conversation stuck behind
    /// a deleted directory is offered the carrying on its branch may well hold,
    /// and what decides it in the end is the relaunch that reads the directory
    /// the steer has just made.
    ///
    /// Checked again when the modal is submitted, as every refusal here is;
    /// this says only that it was worth offering as of the moment it was read.
    pub ready_to_continue: bool,

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
    /// `null` both before grilling starts and after closing — the two ways a
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
    ///
    /// Set for every stop, however it stopped. Which of the two marks the
    /// header draws is `stopped_by_hand` below — both of them point here, a
    /// stop the human has to find being the same Notice as a stop they made
    /// themselves.
    pub blocked_on: Option<i64>,

    /// Whether that stop is the human's own press, or a row from before the
    /// two were told apart and read as one.
    ///
    /// Which of the two marks the header draws, decided here rather than in the
    /// browser: `false` is the accent *Blocked on you* badge — Verkstead pulled
    /// the brake, or a crash took the driver away — and `true` is the quiet
    /// **Stopped** label, which goes to the same Notice and says nothing about
    /// anybody waiting. The sidebar's disc follows the same rule from its own
    /// end of the wire, where the row's `waiting` has already folded it in.
    ///
    /// `false` where nothing has stopped, which is the ordinary Conversation:
    /// there is no mark to choose between.
    pub stopped_by_hand: bool,

    /// Whether the wrap-up has narrowed to its checks: the review answered, the
    /// comments dealt with, the checks alone outstanding, and nothing running in
    /// the Worktree.
    ///
    /// What the *Waiting on checks* label is drawn from, and a condition of
    /// Wrapping rather than a state of its own — the precedent is `blocked_on`
    /// above, and this sits beside `state` for the same reason. Nothing is
    /// stored for it: it is the settle facts and the register read together, at
    /// the moment the page was read.
    ///
    /// A flag rather than an Event id, because unlike a stop there is nothing to
    /// go and look at and nothing to do about it — the Notice saying so is on
    /// the record where it happened, and the label is a label.
    ///
    /// `false` in every state but Wrapping, which is where the condition is
    /// derived from and the only place it can hold.
    pub waiting_on_checks: bool,

    /// What the stop shows about the account that ran out coming back, and
    /// `null` on every stop that is not a usage window's — which is nearly all
    /// of them, and every Conversation that has not stopped.
    ///
    /// Words to draw beside Resume rather than a moment anything acts on: no
    /// stop resumes itself, so what a stopped run waits for is a press whatever
    /// stopped it. The one thing that tells a run stopped by an exhausted window
    /// from a run stopped by anything else — same card, same badge, same button.
    ///
    /// As the session printed it, because the wording is the backend's: `3pm`
    /// stays `3pm`, which is what somebody looks at their own clock for.
    pub resets: Option<String>,

    /// Whether a session is registered for this Conversation as of this read.
    ///
    /// The same fact the sidebar draws its working indicator from, said here
    /// because the Timeline has its own use for it: Force stop is offered
    /// exactly where something is running, and the states it is offered in are
    /// the ones a session may or may not be running in.
    ///
    /// A question about a process rather than about the record, so it is true
    /// only as of the moment it was read — and a restarted server has no
    /// sessions at all, so every Conversation then reads as not working, which
    /// is what each of them is.
    pub working: bool,

    /// And whether any driver of Verkstead's own is registered for it as of
    /// this read: the runner working a backlog, the driver following an inline
    /// run or a roadmap, one of the watchers a wrap-up has going — see the
    /// server's own drivers register.
    ///
    /// The same register [`ready_to_resume`] is decided against, reported raw
    /// rather than judged: this says what *is* driving and that one says what
    /// ought to be. So it is a plain `false` wherever nothing holds a
    /// registration, including the states nothing is supposed to be driving —
    /// a Closed Conversation is not one being driven, whatever the resume rule
    /// makes of it.
    ///
    /// Read for the reason [`working`] is, one register along, and true only as
    /// of the moment it was read. The pair is what says a Conversation has gone
    /// quiet all the way through: no session running *and* nothing left holding
    /// it. Which is a stronger thing than the first alone, because a driver
    /// lets go only once its task has ended — so a watcher that is off here has
    /// finished its last call to the outside world rather than merely started
    /// it.
    ///
    /// [`ready_to_resume`]: ConversationView::ready_to_resume
    /// [`working`]: ConversationView::working
    pub driven: bool,

    /// Oldest first, which is reading order and puts the Brief at the top.
    pub timeline: Vec<TimelineEvent>,

    /// Whether the human has put this Conversation away — see
    /// [`ConversationArchived`].
    ///
    /// What the actions menu offers Unarchive by, in the place Archive stands
    /// in on a Closed Conversation that is still on the list. Nothing else on
    /// the page turns on it: an archived Conversation is drawn exactly as it
    /// was, because being off the sidebar is the whole of what archiving does.
    pub archived: bool,

    /// The Events that stay in view rather than scrolling past with the record.
    ///
    /// Apart from the Timeline rather than in it, because that is what pinning
    /// *is*: the list is a record of moments, and each of these is the current
    /// state of something the work is against. Empty is the ordinary case — a
    /// Conversation with no backlog has nothing to pin.
    pub pinned: Vec<PinnedEvent>,

    /// Where the latest share of this Conversation was published, and when.
    ///
    /// `null` on every Conversation nobody has published one of, which is most
    /// of them: downloading a share leaves no trace, and this is only about the
    /// one that was put somewhere a link can reach.
    ///
    /// Replaced rather than added to. Publishing again is a fresh snapshot of a
    /// Conversation that has moved on, so what the workbench draws is where to
    /// send somebody *now* — see the store's `shares`, which says what becomes
    /// of the link it replaced.
    pub shared: Option<ShareView>,
}

/// One published share, as the workbench draws it: the link, and the moment the
/// snapshot was taken.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ShareView {
    /// The page to send somebody to, as GitHub gave it.
    pub url: String,

    /// When it was published, RFC 3339 — drawn beside the link, because a link
    /// with no date says nothing about how far the work has moved since.
    pub at: String,
}

/// One companion repo of a Conversation: which Repo, how far into it a session
/// may reach, and what its checkout comes off.
///
/// The Repo in the shape the Repo list sends one, for [`ConversationView`]'s
/// reason: the card names it and links nowhere else, and a second shape for a
/// Repo would be a second opinion about what one is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CompanionView {
    pub repo: RepoEntry,

    pub mode: CompanionMode,

    /// The branch this companion's checkout comes off, where the human named
    /// one. `null` is the same rule the Conversation's own base follows: that
    /// repository's default branch, as it stands when grilling starts.
    pub base_ref: Option<String>,

    /// What a read-write companion's branch will be called, or empty for
    /// *mirroring* — the Conversation's own branch name, followed as it is
    /// renamed. Empty on a read-only companion as well, there being no branch
    /// to name: its checkout is detached at the commit the base resolved to.
    pub branch: String,

    /// Where this companion was checked out, once grilling has made its
    /// worktree.
    ///
    /// `null` while the Conversation drafts and again once it is closed, which
    /// is the Conversation's own worktree's rule: a companion has a directory
    /// for exactly as long as the work does.
    pub worktree: Option<Worktree>,

    /// The commit its base resolved to when that checkout was made.
    ///
    /// What a read-only companion is detached at, and what a read-write one's
    /// branch was cut from. Kept beside [`Self::base_ref`] rather than instead
    /// of it, because the two say different things: the ref is the *name* the
    /// human picked and what a rename or a steer would follow, and this is where
    /// that name stood at the one moment it mattered.
    ///
    /// `null` wherever [`Self::worktree`] is, and on a checkout made before
    /// Verkstead kept the commit.
    pub base_commit: Option<String>,
}

/// How far into a companion a session may reach.
///
/// Two, and no third: a repository is there to be read, or it is there to be
/// worked in. What the word decides is the sandbox's binds and whether a branch
/// is cut for it, and neither of those has a halfway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CompanionMode {
    ReadOnly,
    ReadWrite,
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
    /// grill and closing are both this Event, because both are the work
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

    /// A Question Set whose stored body this build cannot read, drawn as itself.
    ///
    /// Its own kind rather than a flag on the one above, because there is no
    /// table to draw and nothing to answer: the row says it cannot be read, and
    /// what it opens is the stored body rather than a document. The Set is on
    /// the record and stays on it — an omission would be this build quietly
    /// deciding a decision never happened.
    UnreadableSet(UnreadableSetEvent),

    /// The handoff the grilling wrote on its way out, rendered inline like the
    /// Brief — and for the same reason: it is a document to read, and there is
    /// nothing of it a details pane would show that the Timeline does not.
    Handoff(HandoffEvent),

    /// A commit a session landed on the Conversation's branch, summarised as
    /// what it changed. Its diff is fetched separately, by the details pane —
    /// the same arrangement a Capture and a Question Set have, and for the
    /// same reason.
    Commit(CommitEvent),

    /// Something Verkstead did on its own account, rendered inline like the
    /// Brief — and for the same reason: it is a sentence to read, and there is
    /// nothing of it a details pane would show.
    ///
    /// The one Event with nothing to do about it and nobody behind it: no agent
    /// wrote it and no human pressed anything for it. It is how an unattended run
    /// says what it decided while nobody was watching.
    ///
    /// What a Verkstead of before wrote as a Pause arrives here too — see
    /// [`notice_event`]: a wait that happened is a sentence to read like any
    /// other, and what a stopped run waits on is the one Resume.
    Notice(NoticeEvent),

    /// A Manual Task a Verkstead of before set going by hand, rendered inline
    /// like the Notice above it — and for its reason: it is a thing that was
    /// asked for once, with nothing of it a details pane would add and nothing
    /// on it to press.
    ///
    /// Nothing writes another. A steer into Implementing carries the human's
    /// instruction now, and drives the Conversation with it rather than leaving
    /// the work standing beside its own session — so what is here is the record
    /// of something that happened, kept and read rather than rewritten.
    ManualTask(ManualTaskEvent),

    /// A Steer the human pressed: which state they moved the Conversation into.
    ///
    /// Drawn beside the Moved line the same move wrote rather than instead of
    /// it. The move says where the work got to and this says who put it there,
    /// and a record with only the first could never be read back for the
    /// difference between the pipeline arriving somewhere and a human deciding
    /// it should be there.
    Steer(SteerEvent),

    /// The pull request the finish step opened, at the moment it reached the
    /// Timeline.
    ///
    /// The one Event that is a [`PinnedEvent`] as well, and it is one card drawn
    /// twice rather than two cards: the sticky block keeps the pull request in
    /// view for as long as the work is on it, and this is where it happened. A
    /// record that folded it out would be a record missing the moment the work
    /// went up for review.
    PullRequest(PullRequestEvent),

    /// The backlog, at the moment it landed on the branch.
    ///
    /// A [`PinnedEvent`] as well, as the pull request above is, and drawn twice
    /// for the same reason: the sticky block keeps the list in view wherever the
    /// record is being read, and this is where the work stopped being a plan.
    ///
    /// What differs from the pull request is where the content comes from. A PR
    /// is three facts on the record; a backlog is `.tasks/` in the Worktree, read
    /// live — so this row carries the reading of the moment somebody looked, and
    /// carries none where there is no Worktree left to read.
    TaskList(TaskListReached),

    /// And the roadmap, at the moment it landed. The same arrangement one level
    /// up, read live off `docs/roadmaps/`.
    StageList(StageListReached),
}

/// The backlog on the record: where it landed, and what it says now.
///
/// The two halves come from different places on purpose. `id` and `at` are the
/// row's, stamped once when the branch first carried a backlog; `list` is the
/// Worktree's, read afresh every time the Conversation is — so the card ticks
/// along with the work while the row it sits at stays where it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct TaskListReached {
    pub id: i64,

    /// When the backlog landed, RFC 3339.
    pub at: String,

    /// The backlog as it stands, or nothing where there is none to read: a
    /// Worktree that has been taken away, or a `.tasks/` the branch has since
    /// finished with. The row stays either way — it is the record of a moment,
    /// and the moment happened.
    pub list: Option<TaskListEvent>,
}

/// The roadmap on the record: where it landed, and what it says now.
///
/// The stage lists rather than one, because a branch may have written to more
/// than one roadmap and the pinned block draws each of them. Empty where there
/// is nothing left to read, exactly as the backlog's is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct StageListReached {
    pub id: i64,

    /// When the roadmap landed, RFC 3339.
    pub at: String,

    pub roadmaps: Vec<StageListEvent>,
}

/// An Event the Timeline keeps in view rather than letting scroll past.
///
/// A fixed set — a task list, a stage list and a PR — and no manual pin or
/// unpin: what is pinned is decided by what kind of thing it is, so there is no
/// state here to flip and no route to flip it with. A tagged kind for the reason
/// [`TimelineEvent`] is one: what gets drawn turns on which kind it is.
///
/// All three are on the record as well, each at the moment it arrived there, and
/// each is one card drawn twice rather than two cards.
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
/// No id and no stamp of its own. It is read out of `.tasks/` each time the
/// Conversation is — the repository owns the files, and Verkstead never does —
/// so what it says is what the Worktree holds now rather than what it held at
/// any one moment.
///
/// Which does not keep it off the record. The moment a backlog *landed* is worth
/// stamping and is stamped — see [`TaskListReached`], which carries this reading
/// at that row — so the identity is on the row and the content is here, and the
/// card is the same card in both places.
///
/// It opens all the same, in both of the places it is drawn: what a details
/// pane shows of it is not the list again but the documents the entries name —
/// see [`BacklogPane`], which is its own request.
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

    /// Whether the task is finished, which is the entry's own checkbox. That is
    /// the done-signal the task runner turns on, and it is the list saying so
    /// rather than anything the directory beside it happens to hold — a task
    /// whose document has not been written yet is a task nobody has done.
    pub done: bool,
}

/// The backlog opened: every task document of it, rendered.
///
/// What the card cannot show. A task list's card is the entries — a number, a
/// title and a box — and each entry details a document in `.tasks/` that says
/// what the task is and what *done* means for it. That is what this is: the
/// documents themselves, in the order the backlog works them.
///
/// Its own request rather than a field on the Conversation, for the reason a
/// commit's diff is one: a Timeline is read every time an open page hears the
/// world moved, and a backlog is read whole when somebody opens it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BacklogPane {
    /// What the backlog is called: `TODO.md`'s heading, which is what the card
    /// says too. Empty where the list wrote none.
    pub feature: String,

    /// In the order the list has them, which is the order they get worked in.
    pub tasks: Vec<TaskDocument>,

    /// Whether any of these documents came out holding a Diagram, and so
    /// whether the pane carries the client-side renderer at all.
    ///
    /// Asked once of all of them, off the HTML above, exactly as a Set's own
    /// flag is — see [`crate::SetView::diagrams`]. mermaid is megabytes, and the
    /// pane that asks for the bundle is the one with something to draw with it.
    pub diagrams: bool,
}

/// One task's document as the pane draws it: the entry it belongs to, and the
/// markdown of its file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct TaskDocument {
    /// As the list writes it, zero-padding and all — `01`, the same string the
    /// card's entry carries.
    pub number: String,

    pub title: String,

    /// Whether the task is finished, which is the entry's checkbox — see
    /// [`TaskEntry::done`]. Carried on the document because a finished task
    /// still has one: its file stays in `.tasks/` until the feature is over, so
    /// the done state is something the section says about itself rather than the
    /// reason it is empty. The same way round as a stage's — see
    /// [`StageDocument::done`].
    pub done: bool,

    /// The document rendered and sanitized, or `null` where there is nothing to
    /// render. Not the ordinary end of a task's life but the list pointing at a
    /// file nobody wrote, which the pane says in words rather than drawing a
    /// gap.
    pub html: Option<String>,
}

/// What the caller of [`backlog_pane`] hands over for one entry: what the list
/// says about it, and its document as the file still holds it.
///
/// Its own type rather than the server's, because this crate reads no
/// filesystem — and rather than three parameters, two of which are strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSource {
    pub number: String,
    pub title: String,
    pub done: bool,

    /// The markdown, or `None` where the file the entry names is not there to
    /// read.
    pub markdown: Option<String>,
}

/// The roadmap opened: every stage brief of it, rendered.
///
/// What the card cannot show, one level up from [`BacklogPane`] and built the
/// same way. A stage list's card is the entries — a number, a title and a box —
/// and each entry names a brief beside `ROADMAP.md` that says what the stage is
/// for. That is what this is: the briefs themselves, in the roadmap's own order.
///
/// Its own request rather than a field on the Conversation, for the reason the
/// backlog's is one: a Timeline is read every time an open page hears the world
/// moved, and a roadmap is read whole when somebody opens it.
///
/// Named by the roadmap rather than by the Conversation, which is the one place
/// this parts company with the backlog: a Worktree has one `.tasks/` and may
/// hold any number of roadmaps, so the card says which of them it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RoadmapPane {
    /// The roadmap's directory under `docs/roadmaps/` — `mvp` — which is its
    /// identity, and what the card named to open this.
    pub name: String,

    /// `ROADMAP.md`'s own heading. Empty where it wrote none, which is when the
    /// pane falls back to the name, exactly as the card does.
    pub title: String,

    /// In the order the roadmap has them, which is the order they get worked in.
    pub stages: Vec<StageDocument>,

    /// Whether any of these briefs came out holding a Diagram, and so whether
    /// the pane carries the client-side renderer at all — asked once of all of
    /// them, as [`BacklogPane::diagrams`] is.
    pub diagrams: bool,
}

/// One stage's brief as the pane draws it: the entry it belongs to, and the
/// markdown of its file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct StageDocument {
    /// As the roadmap writes it, zero-padding and all — `01`.
    pub number: String,

    pub title: String,

    /// Whether the stage is finished, which here is the checkbox — see
    /// [`StageEntry::done`]. Carried on the document because a finished stage
    /// still has one: a brief stays where it is for ever, so the done state is
    /// something the section says about itself rather than the reason it is
    /// empty.
    pub done: bool,

    /// The brief rendered and sanitized, or `null` where there is nothing to
    /// render. Unlike a task's, that is not the ordinary end of a stage's life
    /// but a roadmap pointing at a file nobody wrote — which the pane says in
    /// words rather than drawing a gap.
    pub html: Option<String>,
}

/// What the caller of [`roadmap_pane`] hands over for one entry: what the
/// roadmap says about it, and its brief as the file holds it.
///
/// Its own type rather than the server's for [`TaskSource`]'s reason: this crate
/// reads no filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSource {
    pub number: String,
    pub title: String,
    pub done: bool,

    /// The markdown, or `None` where the brief the entry names is not there to
    /// read.
    pub markdown: Option<String>,
}

/// The roadmap as the Timeline shows it: what it is called, and every stage
/// against whether it is checked.
///
/// No id and no stamp of its own, for the reason the task list beside it has
/// none: it is read out of `docs/roadmaps/` each time the Conversation is, so
/// what it says is what the Worktree holds now. The moment the roadmap landed is
/// stamped all the same — see [`StageListReached`]. It opens, in both of the
/// places it is drawn, and what a details pane shows of it is not the list again
/// but the briefs its entries name — see [`RoadmapPane`], which is its own
/// request.
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
/// An id and a stamp of its own, unlike the task list beside it, because a pull
/// request is the whole of what it says: the finish step opened one at a moment
/// worth keeping, and the Conversation moved into Wrapping on the strength of
/// it. What is not on the record is what the PR holds — see
/// [`PullRequestDetails`], which the details pane fetches when somebody opens
/// this.

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

    /// Which repository it was opened in, where that is not the Conversation's
    /// own.
    ///
    /// `None` is the work's own repository and draws nothing, by the rule a
    /// commit's label follows: an unlabeled card means the repo the Conversation
    /// is in, and the label earns its place when the pinned block holds a
    /// companion's pull request as well.
    pub repo: Option<String>,

    /// How the checks on it were getting on the last time anything asked, or
    /// nothing where nothing has — a pull request in a repository with no CI,
    /// and one opened before Verkstead started writing this down.
    ///
    /// The aggregate rather than the checks, because what the card has room for
    /// is one icon: which of the three a suite is, and not what each of them is
    /// called.
    ///
    /// It can be stale, and on a Conversation nothing is watching any more it
    /// will be: what keeps it fresh is the checks watcher, and that stops when
    /// the wrap-up is over.
    pub checks: Option<CheckRollup>,
}

/// How a pull request's checks are getting on, taken all together.
///
/// The store's own word, carried across the wire — see the reading behind it
/// there. Three states and no fourth: *nobody has asked* is the absence of one
/// rather than a variant, which is a card with no icon on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CheckRollup {
    Passed,
    Running,
    Failed,
}

/// What is on a pull request now: the commits it carries, what GitHub is running
/// against it, and what has been said about it.
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

    /// Every check GitHub is running against it, in the order GitHub lists
    /// them. Empty where there are none, which is a repository with no CI.
    ///
    /// Each of them rather than the one word the card draws — see
    /// [`CheckRollup`]. What the card has room for is which of the three a
    /// suite is; this is the pane somebody opens to find out *which* check is
    /// red and where its run is.
    pub checks: Vec<PullRequestCheck>,
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

/// One check GitHub is running against a pull request, as the details pane
/// receives it: what it is called, how it is getting on, and where its run is.
///
/// A line of a list GitHub keeps, in the spirit [`PullRequestCommit`] is one:
/// read at the moment the pane is opened rather than written down, because a
/// suite is still running while the human is looking at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PullRequestCheck {
    /// What GitHub calls it, which is what the human calls it by.
    pub name: String,

    pub how: Checked,

    /// Where its run is, as GitHub gave it — the one thing a red check cannot
    /// be read without. Empty where GitHub gave none, which is a check drawn as
    /// its name and nothing to follow.
    pub link: String,
}

/// How one check is getting on.
///
/// The same three words as [`CheckRollup`] and not the same thing: this is one
/// check and that is a whole suite taken together. Three rather than GitHub's
/// dozen, because three is what anybody does anything about — a red one is the
/// thing to go and look at, one still running is nothing to do yet, and the
/// rest are green.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Checked {
    Passed,
    Running,
    Failed,
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

    /// What the commit said about itself, as prose alone: its Commit Summary
    /// flattened to a line with the Diagram left out, for the card to clamp —
    /// see [`crate::markdown::to_prose`].
    ///
    /// The prose and not the rendering, unlike every other document on a card.
    /// A commit's card is a button, rendered markdown cannot live inside one,
    /// and the summary is on the card to be read rather than to be read *at*:
    /// what it looks like whole is the pane's, and the card says what it says.
    ///
    /// `None` where the commit carried no summary — which is every bookkeeping
    /// commit and every commit recorded before summaries were kept — and where
    /// what it carried was a Diagram and nothing else. Both draw the card that
    /// has always been drawn.
    pub snippet: Option<String>,

    /// Which repository it landed in, where that is not the Conversation's own.
    ///
    /// `None` is the work's own repository and draws nothing: an unlabeled card
    /// means the repo the Conversation is in, and the label earns its place when
    /// a Timeline carries more than one repository's commits.
    pub repo: Option<String>,
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

    /// Whether that summary came out holding a Diagram, and so whether the pane
    /// carries the client-side renderer at all.
    ///
    /// Answered here, off the HTML above, exactly as a Set's own flag is — see
    /// [`crate::SetView::diagrams`]. It travels with the pane because it is a
    /// fact about this commit's own account of itself, and because mermaid is
    /// megabytes: the pane that asks for the bundle is the one with something to
    /// draw with it. `false` where there is no summary, there being nothing
    /// there to hold a Diagram.
    pub diagrams: bool,

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

/// A Manual Task as the page receives it: what was asked for, and when.
///
/// HTML alone, like the Notice beside it and unlike the Brief: it is a moment on
/// the record rather than a document anybody goes back and edits.
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

/// A steer as the page receives it: when, where the human sent it, and what
/// they wrote to send it there with.
///
/// The one Event that is sometimes a move and sometimes a document. A steer
/// into Wrapping or Done says nothing but the state, like the move it stands
/// above; a steer into Implementing carries the instruction the session was set
/// going on, and one into Follow-up the brief it was, which is the whole of what
/// that session was asked to do. A steer into Grilling carries a document too,
/// and that one arrives as a Brief Event of its own — it opens a round, and a
/// round starts from a Brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SteerEvent {
    pub id: i64,

    /// When it was steered, RFC 3339.
    pub at: String,

    /// The state the human moved it into.
    pub target: Lifecycle,

    /// The instruction they steered it with, rendered and sanitized on the way
    /// out as every piece of markdown on this wire is — and `None` for every
    /// steer that carried nothing written.
    pub html: Option<String>,
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

    /// Whether this Brief is done being written: the round it belongs to has
    /// been grilled, so it is the record of what that round was built from
    /// rather than a document to edit.
    ///
    /// The server's rule rather than something the page works out from the
    /// Conversation around it, as `ready_to_grill` is — and it is a fact about
    /// one Brief rather than about the Conversation, because a Conversation gets
    /// one Brief per round and what is true of them differs: an adopting
    /// Conversation's first Brief is frozen from the start — it is the stage
    /// brief, and nobody here writes it.
    pub frozen: bool,
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

/// A Question Set the Timeline cannot draw a table for, because this build
/// cannot read the body it was stored as.
///
/// What it carries is the reason and no more. There is no title — that is in the
/// body with everything else — and no standing, because a Set nobody here can
/// read is not one anybody is going to answer. The stored body itself is what
/// the details pane fetches, through the same `/api/ui/sets/{id}` a readable Set
/// is opened by: it is the same Set reached the same way, and what comes back
/// says which of the two it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct UnreadableSetEvent {
    pub id: i64,

    /// When the Set was put, RFC 3339.
    pub at: String,

    pub set_id: i64,

    /// What deserializing the stored body said. On the row rather than behind
    /// the fetch, because it is one line and it is the whole of what happened:
    /// a reader who has to open the Event to find out why a row says nothing has
    /// been told nothing by the row.
    pub why: String,
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
/// Two kinds of thing, each saying which it is: the socket is a conversation in
/// both directions, and what a watcher does to a Screen is look at it a
/// different size, or put something into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Watching {
    /// How big this watcher's window is. The latest one wins for everybody —
    /// there is one Screen however many devices are watching it — and it
    /// reaches the session's own terminal, so its interface redraws to fit.
    Resized(Size),

    /// What the watcher put in, on its way to the session's own terminal.
    ///
    /// Keystrokes and mouse reports alike: a session whose interface tracks the
    /// mouse is sent a report of every move, click and scroll over its Screen,
    /// down the path a keystroke takes, and neither commits Verkstead to
    /// anything. Whatever the terminal makes of what arrives comes back the
    /// ordinary way, in among what the session printed, because that is the one
    /// account of what happened.
    ///
    /// Text rather than a key: what a terminal takes is bytes, and the browser's
    /// own terminal has already turned a keypress into the ones a session
    /// expects.
    PutIn(String),
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
/// settled has none, one that was locked unanswered never will have, and one
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
        Standing::Waiting(_) | Standing::LockedUnanswered(_) => None,
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

/// A Question Set whose stored body this build cannot read, as an Event.
///
/// Nothing is summarised because nothing could be: what the row has to say is
/// that the record is there and this build cannot render it, which is the reason
/// and the id it is stored under.
pub fn unreadable_set_event(id: i64, at: String, set_id: i64, why: String) -> TimelineEvent {
    TimelineEvent::UnreadableSet(UnreadableSetEvent {
        id,
        at,
        set_id,
        why,
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
pub fn brief_event(id: i64, at: String, markdown: String, frozen: bool) -> TimelineEvent {
    TimelineEvent::Brief(BriefEvent {
        id,
        at,
        html: crate::markdown::to_html(&markdown),
        markdown,
        frozen,
    })
}

/// A commit as an Event: five facts git counted, and the snippet of what the
/// commit said about itself that its card clamps.
///
/// Here beside the move for the reason that one is: one place knows how a
/// Timeline is made. The snippet is rendered on the way through, which is the
/// one thing here there is anything to render — a summary of nothing but a
/// Diagram comes out empty, and a card with nothing to say says nothing.
pub fn commit_event(id: i64, at: String, commit: CommitRecord) -> TimelineEvent {
    TimelineEvent::Commit(CommitEvent {
        id,
        at,
        sha: commit.sha,
        subject: commit.subject,
        files: commit.files,
        insertions: commit.insertions,
        deletions: commit.deletions,
        snippet: commit
            .summary
            .as_deref()
            .map(crate::markdown::to_prose)
            .filter(|prose| !prose.is_empty()),
        repo: commit.repo,
    })
}

/// What the caller of [`commit_event`] hands over: the commit as the store
/// holds it.
///
/// Its own type rather than the store's, because this crate does not depend on
/// the store — and rather than seven parameters, because five of them are
/// numbers and a subject, and a call with those in the wrong order would
/// compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRecord {
    pub sha: String,
    pub subject: String,
    pub files: i64,
    pub insertions: i64,
    pub deletions: i64,

    /// The Commit Summary as the agent wrote it, or `None` where the commit
    /// carried none. Markdown, as everything an agent writes is.
    pub summary: Option<String>,

    /// What the repository it landed in is called, where that is not the
    /// Conversation's own — see [`CommitEvent::repo`].
    pub repo: Option<String>,
}

/// One commit as the details pane receives it, rendered on the way.
///
/// Here rather than in the server for the reason the Brief's rendering is: this
/// is the crate with the markdown, the diff parser and the highlighter in it. A
/// patch with nothing in it comes back as nothing to show, exactly as an empty
/// attached Diff does — and so does a summary of nothing but whitespace, which
/// the pane would otherwise draw as a gap above the diff.
pub fn commit_pane(summary: Option<&str>, patch: &str) -> CommitPane {
    let summary = summary
        .map(crate::markdown::to_html)
        .filter(|html| !html.trim().is_empty());

    CommitPane {
        // Asked of the rendered summary rather than of the message it came from,
        // for the reason a Set's own flag is: the rendering is where a fence
        // either became a Diagram or did not, and the renderer in the page reads
        // that same answer out of that same markup.
        diagrams: summary
            .as_deref()
            .is_some_and(crate::markdown::holds_diagram),
        summary,
        diff: crate::diff::to_html(patch),
    }
}

/// A backlog as the Timeline shows it. Nothing to render — a task is a number, a
/// title and whether its file is still there — and here beside the rest for the
/// reason a move is: one place knows how a Timeline is made.
///
/// The card itself rather than either placement of it, because there are two:
/// [`task_list_event`] pins it above the record and [`task_list_reached`] puts
/// it on the record where it landed, and both are handed this one reading.
pub fn task_list(feature: String, tasks: Vec<TaskEntry>) -> TaskListEvent {
    TaskListEvent { feature, tasks }
}

/// That backlog opened, rendered on the way.
///
/// Here rather than in the server for the reason the commit pane's rendering is:
/// this is the crate with the markdown parser and the sanitizer in it. A task
/// whose document is not there to read comes back with nothing to draw, and the
/// pane says so in words — the list pointing at a file nobody wrote is a thing
/// to say rather than a gap to leave, exactly as a roadmap's is.
///
/// Whether the task is done is the entry's own checkbox and travels beside the
/// document, because a done task still has one: nothing deletes a task file
/// until the feature is finished with.
///
/// A document of nothing but whitespace is the same as no document at all, which
/// is what an empty file left behind would otherwise draw: a box with a gap in
/// it.
pub fn backlog_pane(feature: String, read: Vec<TaskSource>) -> BacklogPane {
    let tasks: Vec<TaskDocument> = read
        .into_iter()
        .map(|task| TaskDocument {
            number: task.number,
            title: task.title,
            done: task.done,
            html: task
                .markdown
                .as_deref()
                .map(crate::markdown::to_html)
                .filter(|html| !html.trim().is_empty()),
        })
        .collect();

    BacklogPane {
        // Asked of the rendered documents rather than of the markdown they came
        // from, for the reason a Set's own flag is: the rendering is where a
        // fence either became a Diagram or did not, and the renderer in the page
        // reads that same answer out of that same markup.
        diagrams: tasks
            .iter()
            .filter_map(|task| task.html.as_deref())
            .any(crate::markdown::holds_diagram),
        feature,
        tasks,
    }
}

/// That backlog as the Event that gets pinned, which is where it is held in
/// view for as long as there is one.
pub fn task_list_event(list: TaskListEvent) -> PinnedEvent {
    PinnedEvent::TaskList(list)
}

/// And as the Event on the record, which is where it landed.
///
/// The stamp is the row's and the content is the Worktree's, which is the whole
/// arrangement: the row says when the branch stopped being a plan, and what is
/// drawn at it is `.tasks/` as it stands when somebody looks.
pub fn task_list_reached(id: i64, at: String, list: Option<TaskListEvent>) -> TimelineEvent {
    TimelineEvent::TaskList(TaskListReached { id, at, list })
}

/// A roadmap as the Timeline shows it. Nothing to render either — a stage is a
/// number, a title and a ticked box — and one reading behind two placements,
/// exactly as the backlog above is.
pub fn stage_list(name: String, title: String, stages: Vec<StageEntry>) -> StageListEvent {
    StageListEvent {
        name,
        title,
        stages,
    }
}

/// That roadmap opened, rendered on the way.
///
/// Here rather than in the server for [`backlog_pane`]'s reason: this is the
/// crate with the markdown parser and the sanitizer in it. A stage whose brief
/// is not there to read comes back with nothing to draw, and the pane says so
/// in words — a roadmap pointing at a file nobody wrote is a thing to say
/// rather than a gap to leave, the same way `/next-stage` refuses to guess past
/// one.
///
/// A brief of nothing but whitespace is the same as no brief at all, which is
/// what an empty file would otherwise draw: a box with a gap in it.
pub fn roadmap_pane(name: String, title: String, read: Vec<StageSource>) -> RoadmapPane {
    let stages: Vec<StageDocument> = read
        .into_iter()
        .map(|stage| StageDocument {
            number: stage.number,
            title: stage.title,
            done: stage.done,
            html: stage
                .markdown
                .as_deref()
                .map(crate::markdown::to_html)
                .filter(|html| !html.trim().is_empty()),
        })
        .collect();

    RoadmapPane {
        // Asked of the rendered briefs rather than of the markdown they came
        // from, for the reason the backlog's flag is.
        diagrams: stages
            .iter()
            .filter_map(|stage| stage.html.as_deref())
            .any(crate::markdown::holds_diagram),
        name,
        title,
        stages,
    }
}

/// That roadmap as the Event that gets pinned.
pub fn stage_list_event(list: StageListEvent) -> PinnedEvent {
    PinnedEvent::StageList(list)
}

/// And as the Event on the record, which is where the roadmap landed.
///
/// Every roadmap this branch has written to rather than one, because the pinned
/// block holds every one of them too: a branch that touched two has two cards,
/// and the record row is the same cards in their place.
pub fn stage_list_reached(id: i64, at: String, roadmaps: Vec<StageListEvent>) -> TimelineEvent {
    TimelineEvent::StageList(StageListReached { id, at, roadmaps })
}

/// A pull request as the Event that gets pinned. Nothing to render — a PR is a
/// number, a title and a URL — and here beside the rest for the reason a move
/// is: one place knows how a Timeline is made.
pub fn pull_request_event(id: i64, at: String, opened: PullRequestSummary) -> PinnedEvent {
    PinnedEvent::PullRequest(pull_request(id, at, opened))
}

/// The same pull request as the Event on the record, which is where it happened.
///
/// Made by the same call as the pinned one above, because the two are one card
/// in two places: a Timeline that built them separately could come to hand over
/// two pull requests that disagreed.
pub fn pull_request_reached(id: i64, at: String, opened: PullRequestSummary) -> TimelineEvent {
    TimelineEvent::PullRequest(pull_request(id, at, opened))
}

/// The pull request itself, which each of the two above wraps in its own kind.
fn pull_request(id: i64, at: String, opened: PullRequestSummary) -> PullRequestEvent {
    PullRequestEvent {
        id,
        at,
        number: opened.number,
        title: opened.title,
        url: opened.url,
        repo: opened.repo,

        checks: opened.checks,
    }
}

/// What the caller of [`pull_request_event`] hands over: the pull request as the
/// store holds it.
///
/// Its own type rather than the store's, because this crate does not depend on
/// the store — and rather than four parameters, three of which are strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestSummary {
    pub number: i64,
    pub title: String,
    pub url: String,

    /// What the Repo it was opened in is called, where that is not the
    /// Conversation's own — see [`PullRequestEvent::repo`].
    pub repo: Option<String>,

    /// How its checks were, as the store last wrote it down.
    pub checks: Option<CheckRollup>,
}

/// What a pull request holds, as the details pane receives it: the commit list
/// and the check list as they stand, and every comment rendered from the
/// markdown it was written in.
///
/// The comments are the only part with anything to render, and they are the part
/// that most needs it: a PR comment is markdown written by whoever can reach the
/// repository, so it is sanitized here rather than in a browser. A commit's
/// subject and a check's name are text and are put in the page as text.
pub fn pull_request_details(
    commits: Vec<PullRequestCommit>,
    comments: Vec<Comment>,
    checks: Vec<PullRequestCheck>,
) -> PullRequestDetails {
    PullRequestDetails {
        commits,
        checks,
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
/// is what was asked for once, written for somebody to read back.
pub fn manual_task_event(id: i64, at: String, instruction: &str) -> TimelineEvent {
    TimelineEvent::ManualTask(ManualTaskEvent {
        id,
        at,
        html: crate::markdown::to_html(instruction),
    })
}

/// A Steer as an Event: the state the human sent the Conversation into, and the
/// instruction they sent it with where they wrote one.
///
/// Rendered the way the Brief is, and for the same reason: it is what the human
/// asked for, written for somebody to read back.
pub fn steer_event(
    id: i64,
    at: String,
    target: Lifecycle,
    instruction: Option<&str>,
) -> TimelineEvent {
    TimelineEvent::Steer(SteerEvent {
        id,
        at,
        target,
        html: instruction.map(crate::markdown::to_html),
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

/// The order the human has just dragged the sidebar into: every Conversation
/// they can see, by id, top first.
///
/// The whole list rather than the one row that moved, because the whole list is
/// what a drag produces and what the human is looking at when they let go. A
/// move said as *this one, to there* would have to be replayed against a list
/// the server might have added to since; a list said whole is simply what they
/// meant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct NewOrder {
    pub order: Vec<i64>,
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

/// The branch to come off, or `null` to go back to the default-branch rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct BaseBranchChoice {
    /// One of the repository's own branches, local or remote-tracking, by name.
    /// Stored as the name and resolved when grilling starts, so the work comes
    /// off wherever that branch stands then.
    pub branch: Option<String>,
}

/// Which registered Repo to work alongside.
///
/// The id and nothing else: everything a companion holds beyond which Repo it
/// is has a default worth having, and a press in a menu is one decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct NewCompanion {
    pub repo_id: i64,
}

/// What became of adding one.
///
/// Every refusal is named rather than collapsed into one, because each is a
/// different sentence to put in front of the human — and two of them are about
/// what a companion *is* rather than about anything that has gone wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CompanionAdded {
    /// Added, read-only, on the default-branch rule.
    Added,

    NoSuchConversation,

    /// The Conversation is past drafting: its configuration froze when grilling
    /// started, and the setup card it was changed on is gone with it.
    NotDrafting,

    /// There is no Repo with that id — taken off the registry between the menu
    /// reading it and the press that picked one.
    NoSuchRepo,

    /// It is the Conversation's own Repo. The work is already being done in it,
    /// and a companion checkout of it would be that repository twice in one
    /// sandbox.
    OwnRepo,

    /// It is a companion of this Conversation already.
    AlreadyAdded,
}

/// And of taking one away.
///
/// No *no such companion*: a row that is not there is the state the press asked
/// for, so it comes back as [`CompanionRemoved::Removed`] like any other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CompanionRemoved {
    Removed,
    NoSuchConversation,

    /// The Conversation is past drafting, for [`CompanionAdded::NotDrafting`]'s
    /// reason.
    NotDrafting,
}

/// How far into a companion a session may reach, as the switch on its row sends
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CompanionModeChoice {
    pub mode: CompanionMode,
}

/// What became of flipping that switch.
///
/// *No such companion* is among these and is not among a removal's refusals: a
/// removal asked for a row to be gone, and a row that was never there is that.
/// A configuration asked for a row to say something, and where there is no row
/// there is nothing to say it — so the press did nothing, which is worth
/// saying rather than reporting as done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CompanionModeChosen {
    Chosen,
    NoSuchConversation,

    /// The Conversation is past drafting, for [`CompanionAdded::NotDrafting`]'s
    /// reason.
    NotDrafting,

    /// That Repo is not a companion of this Conversation — taken off between
    /// the card drawing the row and the press that configured it.
    NoSuchCompanion,
}

/// And of choosing the branch a companion's checkout comes off.
///
/// The Conversation's own [`BaseRecorded`] with the companion refusal added,
/// rather than that enum reused: the branch this is about is the companion
/// repository's own, and a Conversation and a companion of it are two
/// repositories with two lists of branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CompanionBaseRecorded {
    Recorded,
    NoSuchConversation,
    NotDrafting,

    /// That Repo is not a companion of this Conversation any more.
    NoSuchCompanion,

    /// The companion's own repository has no branch by that name — see
    /// [`BaseRecorded::NoSuchBranch`], which is the same refusal about the
    /// Conversation's own.
    NoSuchBranch,
}

/// And of naming the branch a read-write companion's work is done on.
///
/// The empty name is not refused, because empty is not a name: it is
/// *mirroring*, which is what a companion nobody has typed into is on and what
/// clearing the field goes back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CompanionBranchRenamed {
    Renamed,
    NoSuchConversation,
    NotDrafting,

    /// That Repo is not a companion of this Conversation any more.
    NoSuchCompanion,

    /// Not a name git would take for a branch, asked of git itself — see
    /// [`BranchRenamed::NotABranchName`].
    NotABranchName,
}

/// What became of an edit to a Brief.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum BriefSaved {
    Saved,
    NoSuchConversation,

    /// The Conversation is past drafting, so its Brief is frozen: a steered
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

/// What became of choosing the branch the work comes off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum BaseRecorded {
    Recorded,
    NoSuchConversation,

    /// The Conversation is past drafting, and the base commit was captured when
    /// grilling started.
    NotDrafting,

    /// That repository has no branch by that name. Refused now rather than at
    /// grill start, where it would be a failure with nobody watching — and
    /// asked of the branches themselves, because a branch is the whole of what
    /// there is to pick.
    NoSuchBranch,
}

/// What became of starting a Conversation grilling.
///
/// Every refusal is named rather than collapsed into one, because each of them
/// is something different for the human to go and do: choose a Profile, write a
/// Brief, pick another commit, deal with a branch that is already there. A
/// single "cannot start" would leave them guessing which.
///
/// A companion repo can fail in four of the same ways the Conversation's own
/// does, and which repository it was is the thing the human needs — so those
/// four are carried together under [`GrillingStarted::Companion`], named for
/// the Repo they are about. Nothing gates the button on a companion: the
/// configuration is always complete, so refusal at the start is the whole story.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum GrillingStarted {
    /// The branch and the worktree are made, and the Conversation is grilling.
    Started,

    NoSuchConversation,

    /// It is past drafting, so it has been started once already — or closed.
    NotDrafting,

    /// No Agent Profile is chosen for the grilling session.
    NoGrillingProfile,

    /// None is chosen for the implementation either. Fixed before starting
    /// rather than after, because the grilling ends by handing over to it.
    NoImplementationProfile,

    /// Nor for the review the wrap-up runs. Fixed before starting for the same
    /// reason: what the work is looked at by is settled before the work begins
    /// rather than swapped underneath it.
    NoReviewProfile,

    /// A chosen Profile's pair is not where it was left, so there is no account
    /// to run the session under.
    ProfileBroken,

    /// The Brief is empty, and the Brief is what the grilling starts from.
    /// Freezing an empty one would freeze nothing worth having.
    EmptyBrief,

    /// Git would not fetch from the Repo's remote, so what the work would come
    /// off cannot be trusted to be what the remote is holding. Refused rather
    /// than branched from refs that may be stale: being offline, or having lost
    /// an authentication, is something the human can go and fix.
    FetchFailed,

    /// Nothing in the repository answers to what the work would branch from —
    /// an overridden commit that has gone, or a default branch that has.
    NoBaseCommit,

    /// The branch is already there. Verkstead did not make it, so it will not
    /// take it over: what is on it is somebody's work.
    BranchExists,

    /// Git would not make the worktree. The reason is in the server's log — this
    /// is the one refusal with nothing for the human to correct.
    WorktreeRefused,

    /// One of the Conversation's companion repos could not be checked out, and
    /// so none of it was: the whole start is refused, naming the repository and
    /// what git could not do about it.
    Companion {
        /// What the companion Repo is called, which is what the human picked it
        /// by and what they will go and look at.
        repo: String,

        why: CompanionRefusal,
    },
}

/// Which of a companion repo's four ways of not being checked out this was.
///
/// The Conversation's own four, asked of a companion: everything git is asked
/// for one is what it is asked for the other, in the same order and for the same
/// reasons. Separate from [`GrillingStarted`] rather than four more variants of
/// it, so that the repository is named once instead of four times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum CompanionRefusal {
    /// Git would not fetch from that repository's remote, so what its checkout
    /// would come off cannot be trusted to be what the remote is holding.
    FetchFailed,

    /// Nothing in it answers to the base its checkout would come off — a branch
    /// picked while drafting that has since gone, or a default branch that has.
    NoBaseCommit,

    /// A read-write companion's branch is already there, in that repository.
    /// Verkstead did not make it, so it will not take it over.
    BranchExists,

    /// Git would not make the worktree. The reason is in the server's log.
    WorktreeRefused,
}

/// What became of pressing Resume.
///
/// Named the way [`GrillingStarted`]'s refusals are, and for a reason of its
/// own on top of theirs: Resume is never silent. Either something is running —
/// which needs no announcement, the session showing up on the Timeline — or
/// nothing is, and the one place that can say why is the answer to the press.
/// A recompute that quietly found nothing to launch is exactly the failure this
/// whole feature is replacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Resumed {
    /// Driving has started again: the stop is cleared and what the lifecycle
    /// and the branch say should be running is being launched.
    Resumed,

    NoSuchConversation,

    /// It is drafting, done or closed, so nothing was ever supposed to be
    /// driving it. None of the three is a Conversation standing still.
    NotDriven,

    /// Something is driving it already — a session, a runner, a wrap-up's
    /// watchers. The second press of the button is the first one arriving
    /// again, and starting a second driver would be two agents in one Worktree.
    AlreadyDriven,

    /// There is no Worktree on the record to work in. A Conversation past
    /// drafting is supposed to have one, so this is a record that cannot be
    /// true.
    NowhereToWork,

    /// There is one on the record, the directory it names is not a worktree any
    /// more, and it could not be made again from the branch. The one refusal
    /// here with nothing for the human to correct: the reason is in the
    /// server's log, as a worktree git refused at grill time is.
    WorktreeRefused,

    /// It says it is implementing and nothing says how the work is being built,
    /// which is another record that cannot be true: a Conversation implements
    /// because a direction was picked.
    NoDirection,

    /// The backlog it was working has nothing left in it and this branch never
    /// wrote one: there is no step to read off `.tasks/` and nothing built on
    /// the branch to carry to a pull request either. A backlog that *was*
    /// written and worked to empty is not this — that one has work on the
    /// branch, so the press has somewhere to go.
    NothingToWork,

    /// The grilling Pairing has gone, and a grilling runs under that one
    /// whatever else has happened since.
    NoGrillingPairing,

    /// And the implementation Pairing has gone, which is what every session of
    /// the work itself runs under.
    NoImplementationPairing,

    /// It says it is following the work up and nothing on its Timeline says what
    /// about: another record that cannot be true, a steer being the only way
    /// into Follow-up and one without a brief being refused.
    NoFollowUpBrief,
}

/// What clicking Steer found, which is what the modal it opens is drawn from.
///
/// The click is a press of its own rather than the first half of the submit: it
/// stops the drive before the modal opens, so that nothing new is launched while
/// the human composes and the world the modal was drawn against is the world the
/// submit arrives in. Cancel leaves the Conversation stopped with Resume on
/// offer, which is accepted rather than a bug — the click is what freezes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum SteerOpened {
    /// The drive has stopped — or there was never anything driving it — and the
    /// modal may open.
    Opened {
        /// Whether a session is still running as the modal opens.
        ///
        /// What **Interrupt current task** is offered for: the click leaves what
        /// is running exactly where it is, and the checkbox is the only way to
        /// end it where it stands. What ends it otherwise is the submit's own
        /// launch — one Worktree holds one agent, so the session a steer starts
        /// takes the Worktree from whatever is still in it — and into Done,
        /// where nothing is launched, nothing ends it at all.
        ///
        /// Where nothing is running there is nothing to interrupt, so the
        /// checkbox is not drawn at all.
        working: bool,
    },

    NoSuchConversation,
}

/// Where a steer can send a Conversation.
///
/// Draft and Closed are not among them and never will be: each has a way in of
/// its own, and a steer is for the states the work is *done in* — the four rungs
/// of the ladder, and Follow-up beside them, which has no other way in at all. A
/// target the modal offers is a target something can be set going in, which is
/// why the two that turn on a pull request are drawn out where there is none: an
/// instruction is writable anywhere and Done needs nothing, but there is no
/// wrapping up and no following up of work nobody can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum SteerTarget {
    /// A new round: the work grilled again, from whatever brief the human writes
    /// in the modal and against as much of the last interview as they ask for.
    ///
    /// The target that recreates the most, because it is the one reachable from
    /// the states that hold the least. A Draft has neither branch nor Worktree
    /// and gets both, its base commit resolved as a grill start resolves one; a
    /// closed Conversation kept its branch and lost its Worktree, and gets the
    /// branch checked out again into one.
    Grilling,

    /// The work built: either carrying on from what the branch already holds —
    /// the next task of the backlog, the roadmap it has written — or doing what
    /// the human wrote in the modal.
    ///
    /// **The instruction is what makes this a target from anywhere.** Where
    /// something stands, writing nothing carries it on and what is next is the
    /// branch’s own answer, asked exactly as every other turn of the run asks
    /// it — see [`ConversationView::ready_to_continue`], which is the rule the
    /// modal offers that by. Where nothing stands there is nothing to pick up,
    /// so an instruction is required and a submit without one is refused by
    /// name — see [`ConversationSteered::NoInstruction`].
    ///
    /// An instruction session is a driver rather than an errand: registered as
    /// driving while it runs, judged by the ordinary end-of-session rules, and
    /// on a clean finish the pipeline carries on from whatever the branch then
    /// holds. Which is why a Conversation that has never said how its work is
    /// built is recorded as building it inline as the steer lands — a state
    /// something runs in with nothing saying how is a record a pressed Resume
    /// refuses on.
    Implementing,

    /// The branch looked at again: the checks watched, the review run, the
    /// comments answered. No payload — the wrap-up's watchers work out for
    /// themselves what is left to do, which is what a pressed Resume already
    /// asks of them.
    ///
    /// Offered only where the record already holds a pull request. A wrapping
    /// Conversation is defined by the one under it, so a steer here is a move
    /// onto a pull request that is already there rather than a way of opening
    /// one — see [`ConversationSteered::NoPullRequest`].
    Wrapping,

    /// The pull request followed up on: a session started on the brief the human
    /// wrote, which answers what they asked, does what they want done about work
    /// that is already pushed, and goes on asking until they are finished.
    ///
    /// **The brief is required**, unlike either of the other written payloads:
    /// there is nothing on the branch that could stand for it, a follow-up being
    /// a thing the human wanted rather than a step of the run. A submit without
    /// one is refused by name — see [`ConversationSteered::NoFollowUpBrief`].
    ///
    /// Offered only where the record holds a pull request, as Wrapping is and
    /// refused by the same name, and only from Done and Wrapping: what a
    /// follow-up follows up is work that has been seen through, and a
    /// Conversation still building has the ordinary ways of saying what to do
    /// next.
    FollowUp,

    /// Finished with. Nothing runs, so there is no Pairing to settle and no
    /// payload to carry: a steer into Done is the move alone.
    Done,
}

impl SteerTarget {
    /// Whether work goes on in this state, which is what the rest of the modal's
    /// shape follows from.
    ///
    /// A target something runs in needs a Pairing settled and a Worktree to run
    /// in; one nothing runs in needs neither. Said once here because the page
    /// draws the picker by it and the server refuses by it, and two readings of
    /// the same question could come to different answers.
    pub fn runs(self) -> bool {
        match self {
            Self::Grilling | Self::Implementing | Self::Wrapping | Self::FollowUp => true,
            Self::Done => false,
        }
    }
}

/// What the human settled in the modal: where the Conversation goes, what runs
/// the work there, and what to do about anything still running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SteerSubmission {
    /// Which state to move it into.
    pub target: SteerTarget,

    /// Whether to end the session that is running where it stands.
    ///
    /// `false` is the default and the ordinary case: the click stopped the
    /// drive, so nothing was started after what is running, and what ends it is
    /// this steer's own launch taking the Worktree from it. `true` is the human
    /// saying they will not wait for it — which is what saves the wait where the
    /// launch would have had to queue behind it, and what ends it at all into
    /// Done, where nothing is launched.
    pub interrupt: bool,

    /// The Pairing the work runs under from here, for a target something runs
    /// in — and what is picked is recorded as the *Conversation's*, because
    /// steering re-settles what runs the work rather than picking for one
    /// session.
    ///
    /// Absent where the target runs nothing, and absent where the human left
    /// the picker on what the Conversation already had: both are a submit that
    /// changes no Pairing. A Conversation with none fixed yet — a steered draft
    /// — is why the pick is part of the modal rather than an error path, and one
    /// that arrives with neither this nor a Pairing of its own is refused by
    /// name.
    #[serde(default)]
    pub pairing: Option<ProfileChoice>,

    /// The new round's Brief, for a steer into Grilling.
    ///
    /// It lands as a Brief Event of its own, frozen the moment it does: a Brief
    /// freezes when its round leaves Draft, and a round steered into has no
    /// Draft to leave. A second Brief beside the first rather than an edit of
    /// it — what the earlier round was built from stays on the record.
    ///
    /// Absent is the ordinary case and not a refusal: the session starts on the
    /// Brief that is already there, and the steer leaves nothing but its own
    /// Event behind.
    #[serde(default)]
    pub brief: Option<String>,

    /// The hand-written work, for a steer into Implementing.
    ///
    /// It lands as the Steer Event's own body and a session is started on it —
    /// a driver of the Conversation rather than an errand beside it, so what
    /// follows a clean finish is whatever the branch then holds.
    ///
    /// **Required where nothing stands to be carried on**, and optional beside
    /// carrying on where something does: a branch with a backlog left in it has
    /// an answer to what is next, and a branch with nothing on it has none. A
    /// submit that names Implementing with neither is refused by name — see
    /// [`ConversationSteered::NoInstruction`].
    ///
    /// Whitespace alone is nothing written, exactly as the brief above it: a
    /// textarea somebody tabbed through is not an instruction.
    ///
    /// Nothing anywhere else reads it. A target that starts no session has
    /// nothing to write an instruction for.
    #[serde(default)]
    pub instruction: Option<String>,

    /// The brief, for a steer into Follow-up.
    ///
    /// It lands as the Steer Event's own body, exactly as the instruction above
    /// it does, and the session started on it opens the follow-up: it answers
    /// what the brief asks, does what it asks for, and asks the human what else
    /// there is until they say there is nothing.
    ///
    /// **Required**, which is what makes it the one written payload with no
    /// quiet meaning. Nothing on the branch could stand in for it — a follow-up
    /// is not a step of the run to be picked up — so a submit that names
    /// Follow-up without one is refused by name; see
    /// [`ConversationSteered::NoFollowUpBrief`].
    ///
    /// Whitespace alone is nothing written, as everywhere else here.
    #[serde(default)]
    pub follow_up: Option<String>,

    /// Whether the session is primed with everything the human has already
    /// answered.
    ///
    /// The digest a relaunched grilling assembles for itself — every answered
    /// Question Set of the Conversation, in the order it was asked — offered
    /// here as a choice rather than always sent. A fresh brief is often the
    /// point of the steer, and priming it with the whole of the last interview
    /// would be steering into the argument that has just been left behind.
    ///
    /// Nothing anywhere else reads it: a target that starts no grilling starts
    /// nothing to prime.
    #[serde(default)]
    pub digest: bool,

    /// The registered Repos to put into the sandbox the sessions to come run
    /// in, each with what a setup row would have said about it.
    ///
    /// Sandbox setup rather than a property of one state, which is why it rides
    /// every target work goes on in rather than one of them. Empty is the
    /// ordinary case and the whole of what most steers carry.
    ///
    /// **Adding only.** Nothing here may name a companion the Conversation
    /// already has — one that does is refused rather than obeyed, because the
    /// frozen set only widens and what a session was once given is never taken
    /// back mid-Conversation. Which is also why there is no list beside this
    /// one for taking a companion away.
    ///
    /// Nothing anywhere else reads it: a target nothing runs in has no sandbox
    /// to set up.
    #[serde(default)]
    pub added: Vec<CompanionAddition>,

    /// And the companions already there that the steer opens up: read-only
    /// until now, read-write from here, each with what the branch cut in it is
    /// called.
    ///
    /// **Upgrading only, which is why there is no mode on the rows.** Read-only
    /// is not something this can ask for and neither is removal, so a
    /// downgrade cannot be spelled here at all — what a session was once given
    /// is never taken back mid-Conversation. Nothing here may name a companion
    /// that is read-write already, or a Repo the Conversation has not got: both
    /// are refused rather than obeyed, the first being a row with nothing left
    /// to open and the second a page arguing with the record.
    ///
    /// Nothing anywhere else reads it, for [`Self::added`]'s reason: a target
    /// nothing runs in has no sandbox to open up.
    #[serde(default)]
    pub upgraded: Vec<CompanionUpgrade>,
}

/// One registered Repo a steer puts on a Conversation, with everything a setup
/// row would have settled about it.
///
/// The same four facts a drafting companion is configured with — which Repo,
/// how far into it the work may reach, what its checkout comes off, and what a
/// read-write one's branch is called — because this is the one other moment
/// those questions can be asked: the setup rows have gone by the time anything
/// is steered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CompanionAddition {
    pub repo_id: i64,

    pub mode: CompanionMode,

    /// The branch of that repository's own the checkout comes off, or `null`
    /// for the rule the Conversation's own base follows: that repository's
    /// default branch, as origin holds it at the moment of the steer.
    pub base_ref: Option<String>,

    /// What a read-write one's branch is to be called, or empty for
    /// *mirroring* — the Conversation's own branch name. Empty on a read-only
    /// one as well, its checkout being detached and holding no branch.
    pub branch: String,
}

/// One companion of a Conversation the steer opens up: read-only until now,
/// read-write from here.
///
/// **Two fields rather than four, and the missing two are the point.** There is
/// no mode, because there is one direction — a row that could carry read-only
/// would be a row that could take back what a session was given. And there is
/// no base: what the upgrade comes off is the base already on the row, picked
/// while the Conversation drafted, re-resolved at this moment because the
/// companion is joining the work now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CompanionUpgrade {
    pub repo_id: i64,

    /// What the branch cut in it is to be called, or empty for *mirroring* —
    /// the Conversation's own branch name, exactly as at draft time.
    pub branch: String,
}

/// What became of submitting one.
///
/// Named the way [`GrillingStarted`]'s refusals are, and nothing here is about
/// the state the Conversation was in: the human has looked at the work and said
/// where it goes, so the source is not something to be refused for. What is left
/// to be wrong about is the *target* — a state whose work cannot be set going
/// from what the record holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ConversationSteered {
    /// Moved: the Steer Event is on the Timeline beside the move, and any stop
    /// the click wrote is gone.
    Steered,

    NoSuchConversation,

    /// Wrapping or Follow-up was named for a Conversation whose work is on no
    /// pull request.
    ///
    /// A wrapping Conversation is defined by the one under it — the store writes
    /// the move and the pull-request row as one act — so there is no wrapping up
    /// to steer into here, and nothing for a follow-up to follow up either. The
    /// modal does not offer either target on such a Conversation; this is the
    /// same rule asked again on arrival, the way every named refusal here is.
    NoPullRequest,

    /// Implementing was named with nothing written, for a Conversation with
    /// nothing on its branch to carry on: no backlog with work left in it, and
    /// no roadmap it has written.
    ///
    /// One or the other, never neither. A steer into Implementing either picks
    /// up what stands or does what the human wrote, so a branch where nothing
    /// stands and a modal where nothing was written is a session with no job.
    /// The modal requires the instruction on such a Conversation rather than
    /// offering the submit and refusing it — see
    /// [`ConversationView::ready_to_continue`], which is what it draws that by
    /// — and this is the same rule asked again on arrival.
    NoInstruction,

    /// Follow-up was named with no brief written.
    ///
    /// The one written payload that is always required. A steer into
    /// Implementing with nothing written carries on what the branch holds and a
    /// steer into Grilling with nothing written grills the Brief that is there;
    /// a follow-up is neither the run's next step nor a round of it, so an empty
    /// one is a session with nothing to follow up.
    NoFollowUpBrief,

    /// Grilling was named with no brief written, for a Conversation whose newest
    /// Brief is empty.
    ///
    /// The rule a pressed *Start grilling* is refused by — see
    /// [`GrillingStarted::EmptyBrief`] — asked of the other way in. A grilling
    /// starts from a Brief and a round steered into is frozen where it lands, so
    /// an empty one is an interview about nothing that nothing can go back and
    /// edit. Reachable on a Draft alone in practice: everything past drafting
    /// was grilled out of a Brief somebody wrote.
    EmptyBrief,

    /// Nothing says which account and model the work runs under from here:
    /// neither a Pairing picked in the modal nor one the Conversation already
    /// had.
    NoPairing,

    /// The Pairing picked names a Profile that is not there — it was removed
    /// between the list the modal read and the pick it made from it.
    NoSuchProfile,

    /// Or a model that Profile does not list, for the same reason.
    NoSuchModel,

    /// The branch has never been made and nothing in the repository answers to
    /// what it would come off.
    ///
    /// A Draft alone, which is the one source with no branch behind it: what it
    /// branches from is the base the human fixed while drafting, or the Repo's
    /// default branch where they fixed none, and a repository that has since
    /// lost either is one they can point at another.
    NoBaseCommit,

    /// What the record names is not a Worktree any more, and git would not make
    /// it again from the branch.
    WorktreeRefused,

    /// One of the Repos the modal named is not on the registry — taken off it
    /// between the list the modal read and the submit that named it.
    ///
    /// The one companion refusal with no repository in it, because there is no
    /// repository to name: a Repo that is not registered is a row Verkstead
    /// knows nothing about but the id the page sent.
    NoSuchCompanionRepo,

    /// A companion could not be put into the sandbox, and this is which one and
    /// why.
    ///
    /// The repository said, because *which one* is the whole of what the human
    /// needs — the same reason [`GrillingStarted::Companion`] says it. Nothing
    /// is made and nothing is moved for any of these: every question a
    /// companion turns on is asked in front of the session that gets ended, the
    /// stop that gets cleared and the worktree that gets rebuilt, so a steer
    /// refused here is a press that did not happen.
    Companion {
        /// The Repo's registered name.
        repo: String,

        why: SteerCompanionRefusal,
    },
}

/// Which of a companion's ways of not being delivered by a steer this was.
///
/// [`CompanionRefusal`]'s four asked again at the other moment a companion is
/// checked out, and four more that only a steer can meet: the setup card
/// catches those the moment a row is pressed, and a steer is where the same
/// questions are asked past drafting, with nothing in front of them but the
/// submit.
///
/// Two of the four are about the *set* rather than about git, and they come in
/// a pair because a steer does two things to it: an add is refused where the
/// Repo is a companion already, and an upgrade is refused where it is not one
/// yet — or where it is already as open as a companion gets.
///
/// A vocabulary of its own rather than more variants of the grill start's, for
/// the reason [`SteerTarget`] is not [`crate::Lifecycle`]: what a grilling can
/// be refused for and what a steer can be refused for are different lists, and
/// one list would say each press can be refused for things it never could.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum SteerCompanionRefusal {
    /// The Repo named is the Conversation's own. It is already the work's
    /// repository, so adding it beside itself would be a second checkout of it
    /// in one sandbox.
    OwnRepo,

    /// It is a companion of this Conversation already.
    ///
    /// Not an add that quietly changes the row it found: the frozen set only
    /// widens, so a submit naming one that is already there is a page arguing
    /// with the record rather than a change to obey.
    AlreadyAdded,

    /// An upgrade named a Repo that is not a companion of this Conversation at
    /// all — its own repository, or one nothing ever put in.
    ///
    /// The mirror of [`Self::AlreadyAdded`], and refused for its reason: a
    /// steer opens up a row that is there, and one that is not there is a page
    /// arguing with the record rather than a change to obey.
    NotACompanion,

    /// An upgrade named a companion that is read-write already, which is as
    /// open as a companion gets.
    ///
    /// Obeying it would be re-cutting a branch over work that has been
    /// committed to one — the taking-back the whole of this is written to
    /// prevent — so it is refused rather than done again.
    AlreadyReadWrite,

    /// Git would not fetch from that repository's remote, so what its checkout
    /// would come off cannot be trusted to be what the remote is holding.
    FetchFailed,

    /// Nothing in it answers to the base its checkout would come off.
    NoBaseCommit,

    /// A read-write companion's branch is already there, in that repository.
    /// Verkstead did not make it, so it will not take it over.
    BranchExists,

    /// Git would not make the worktree. The reason is in the server's log.
    WorktreeRefused,
}

/// What became of pressing Adopt.
///
/// Named the way [`GrillingStarted`]'s refusals are, and for the same reason: a
/// human is at the workbench pressing the button, and each of these is
/// something different for them to go and do. What is decided while nobody is
/// watching says itself on a Timeline instead — see the server's `continuing`
/// module, which starts the same stage by the other route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Adopted {
    /// The branch and the worktree are made, the stage brief is the Brief, and
    /// the Conversation is implementing the stage.
    Adopted,

    NoSuchConversation,

    /// It is past drafting, so it has been adopted once already — or closed.
    NotDrafting,

    /// It is adopting nothing, which is every Conversation that began with a
    /// Brief and a grilling. There is no roadmap here to take a stage from.
    NotAdopting,

    /// No Agent Profile is chosen for the grilling. Carried by an adopted stage
    /// rather than run under: every stage after it inherits both Profiles from
    /// its predecessor, and a Conversation steered into a second round is
    /// grilled.
    NoGrillingProfile,

    /// And none is chosen for the implementation, which is what the stage's own
    /// work runs under.
    NoImplementationProfile,

    /// Nor for the review, which is what looks at what the stage built — and
    /// which every stage after this one inherits along with the other two.
    NoReviewProfile,

    /// A chosen Profile's pair is not where it was left, so there is no account
    /// to run the session under.
    ProfileBroken,

    /// Git would not fetch from the Repo's remote, so the roadmap at the base
    /// commit cannot be trusted to be the roadmap origin is holding. Refused
    /// rather than adopted off refs that may be stale: being offline, or having
    /// lost an authentication, is something the human can go and fix.
    FetchFailed,

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

    /// A companion repo could not be checked out beside the stage's own, and
    /// this is which one and why.
    ///
    /// A Conversation adopting a roadmap is drafting like any other, so its
    /// setup card configures companions like any other — and adoption is the
    /// other press that takes a Draft past drafting, so it checks them out
    /// exactly as a grill start does. Which is why this is
    /// [`GrillingStarted::Companion`] word for word: the same four questions
    /// asked of the same repositories at the other door.
    Companion {
        /// The Repo's registered name.
        repo: String,

        why: CompanionRefusal,
    },
}

/// What became of pressing Stop or Force stop.
///
/// One answer for both presses, because they ask for the same thing and differ
/// only in what they will wait for: the run is to stop, and nothing is to be
/// started for this Conversation until Resume is pressed. [`Stopped`] and
/// [`Stopping`] are the two ways that is now true — see each.
///
/// Named the way [`Resumed`]'s refusals are, and for the same reason: a press
/// that quietly did nothing would leave the human watching a run they thought
/// they had stopped.
///
/// [`Stopped`]: ConversationStopped::Stopped
/// [`Stopping`]: ConversationStopped::Stopping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ConversationStopped {
    /// It has stopped: the stop is written, the Notice is on the Timeline, and
    /// nothing more will be launched. Force stop always answers this, and so
    /// does a Stop pressed with nothing running to see out.
    Stopped,

    /// It is stopping: the session running now runs to its own end, and the
    /// Conversation stops before anything else is started. What Stop answers
    /// where there was something to see out.
    Stopping,

    /// It has stopped already, so the stop standing is the one that explains it.
    /// Getting going again is Resume's, not a second stop's.
    AlreadyStopped,

    /// It is drafting, done or closed, so nothing was ever driving it and there
    /// is nothing to stop.
    NotDriven,

    NoSuchConversation,
}

/// What became of closing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ConversationClosed {
    /// Closed: the worktree is gone and the branch is not.
    ///
    /// Said too where the worktree would not go — a directory git no longer
    /// reads as a worktree is logged and left, rather than standing between the
    /// human and the end of the Conversation. See [`crate::ConversationView`]'s
    /// worktree, which is `None` from here on either way.
    Closed,

    /// It was closed already, which is not an error — what was asked for holds
    /// either way.
    AlreadyClosed,

    NoSuchConversation,
}

/// And what became of archiving one: putting a Closed Conversation away, so the
/// sidebar stops drawing it.
///
/// Reversible, which is why nothing here is confirmed and why the refusals are
/// so mild: the worst of them says the human asked for something that is already
/// true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ConversationArchived {
    /// Archived: it is off the list, and everything else about it is where it
    /// was.
    Archived,

    /// It was archived already, which is not an error — what was asked for holds
    /// either way.
    AlreadyArchived,

    /// It has not been closed, so there is nothing to put away. A Conversation
    /// still being worked on belongs on the list it is being worked from.
    NotClosed,

    NoSuchConversation,
}

/// And what became of taking one back out, which is the way back from it.
///
/// One refusal fewer than archiving has: there is no state a Conversation can
/// be in that is the wrong one to put back on the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ConversationUnarchived {
    /// Unarchived: it is on the list again, for good.
    Unarchived,

    /// It was not archived, which is not an error — what was asked for holds
    /// either way.
    NotArchived,

    NoSuchConversation,
}

/// Whether the sidebar is drawing what the human has archived.
///
/// Their standing choice rather than this device's: it is read back off the
/// server on every load, and what is sent when the toggle is flipped is the
/// position it has been put in rather than the flip itself — a switch says
/// where it stands, and saying it twice says the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ShowingArchived {
    /// On: the archived Conversations are on the list, in their ordinary
    /// places. Off: they are not drawn at all.
    pub showing: bool,
}
