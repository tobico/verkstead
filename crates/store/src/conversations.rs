//! The Conversations a grilling session is run against: a Repo, a branch, a base
//! commit, and the Timeline everything about the work lands on.
//!
//! Nothing here executes anything. The branch and the worktree are made by the
//! server, against git and the filesystem; what this records is that they were —
//! which commit was branched from, where the worktree was put, and that the
//! Conversation has moved. A store that shelled out to git would be a store with
//! a second way to fail.
//!
//! The Timeline is its own table from the start rather than a Brief column on
//! the Conversation. The Brief is the first Event, and agent output, Question
//! Sets and commits are the same list with more in it. A Brief kept beside the
//! Timeline rather than in it would
//! have to be moved into it later, and a reopened round adds a second Brief
//! Event rather than editing the first — which a column could not hold at all.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use sqlx::SqlitePool;
use verkstead_schema::{Decided, Direction, QuestionSet, Response, Review, SetCreated};

/// The word the `direction` column holds.
///
/// The wire spelling, so the column reads as what an agent wrote and a database
/// opened by hand says something. Its own pair of functions rather than serde,
/// because what goes in a `TEXT` column is this module's business either way —
/// exactly as [`Lifecycle::stored`] is.
fn direction_stored(direction: Direction) -> &'static str {
    match direction {
        Direction::Inline => "inline",
        Direction::TaskList => "task-list",
        Direction::Roadmap => "roadmap",
    }
}

/// The direction a stored word names. An unknown one is a database written by a
/// Verkstead this one does not understand, as an unknown state is.
fn direction_read(word: &str) -> Result<Direction> {
    Ok(match word {
        "inline" => Direction::Inline,
        "task-list" => Direction::TaskList,
        "roadmap" => Direction::Roadmap,
        other => bail!("a Conversation is headed in the unknown direction {other:?}"),
    })
}

/// Where a Conversation has got to.
///
/// The ladder is the domain's rather than any one stage's invention, so the
/// states beyond the two this one reaches are here too: the rules written
/// against them — a Brief and a branch name are the human's to change only while
/// the Conversation is still drafting — need the states they refuse on behalf of
/// to exist before the stage that reaches them does.
///
/// [`Lifecycle::Closed`] is off the ladder rather than on it. Every other state
/// is somewhere the work has got to, and closing is the work stopping wherever
/// it was — which is why it is reachable from all of them and leads nowhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    /// The brief of the round it is in is being written. On a first round that
    /// is everything about the Conversation — the Brief, the branch name and the
    /// base commit alike; on a reopened one it is the Brief alone, the branch
    /// having been worked already.
    Draft,

    /// A grilling session is running against it.
    Grilling,

    /// The work is being done.
    Implementing,

    /// The work is on a PR and the wrap-up loop has it.
    Wrapping,

    /// Finished. It can be reopened with a new round, which puts it back to
    /// [`Lifecycle::Draft`] with a Brief of its own to write — see
    /// [`reopen_conversation`].
    Done,

    /// Closed, from wherever it had got to. The worktree is gone; the branch is
    /// not, because a branch is cheap and may hold work worth reading.
    Closed,
}

impl Lifecycle {
    /// The word the column holds. Lowercase and spelled out, so the table reads
    /// as something rather than as a number nobody can look up.
    pub(crate) fn stored(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Grilling => "grilling",
            Self::Implementing => "implementing",
            Self::Wrapping => "wrapping",
            Self::Done => "done",
            Self::Closed => "closed",
        }
    }

    /// The state a stored word names. A word this does not know is a database
    /// written by a Verkstead this one does not understand, which is worth
    /// saying rather than guessing past.
    ///
    /// `aborted` is read beside `closed`, because that is what the state was
    /// called while the press was Abort. A migration rewrites the rows it can
    /// reach — see [`super::migrations`] — and this reads the ones it never
    /// did: a database restored from a backup taken before it ran, or a row
    /// somebody wrote by hand.
    pub(crate) fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "draft" => Self::Draft,
            "grilling" => Self::Grilling,
            "implementing" => Self::Implementing,
            "wrapping" => Self::Wrapping,
            "done" => Self::Done,
            "closed" | "aborted" => Self::Closed,
            other => bail!("a Conversation is in the unknown state {other:?}"),
        })
    }
}

/// A Conversation as the store holds it, with the Repo it is attached to read
/// back beside it — there is no Conversation without one, and everything done
/// about a Conversation is done inside that repository.
///
/// The two Pairings are read back the same way, because whether a Conversation
/// is ready to grill turns on what they are rather than on which ids they hold:
/// a Profile whose pair has gone is not something to launch a session under, and
/// the id alone cannot say so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    pub id: i64,
    pub created_at: String,
    pub repo: super::Repo,

    /// The branch the work will be done on. Prefilled with a random name at
    /// creation and the human's to change while the Conversation is drafting.
    pub branch: String,

    /// The commit to branch from, where the human named one. `None` is not a
    /// missing value: it is the rule that the default branch's tip at grill
    /// start is what gets used, which is a thing to resolve then rather than a
    /// commit to record now.
    pub base_commit: Option<String>,

    pub state: Lifecycle,

    /// The Profile and model the grilling session runs under, once they are
    /// chosen.
    pub grilling_pairing: Option<super::Pairing>,

    /// And the ones the implementation runs under. A separate choice because it
    /// is genuinely a separate account and model — and because the
    /// implementation session cannot simply carry the grilling one on.
    pub implementation_pairing: Option<super::Pairing>,

    /// Where the Conversation's worktree was put, once grilling has made one.
    ///
    /// `None` before grilling starts and again after closing — the two ways a
    /// Conversation has no worktree, which are the same fact about it whatever
    /// put it there. Whether the directory is still on disk is not something the
    /// store can say; see [`close_conversation`] for who does.
    pub worktree: Option<PathBuf>,

    /// The latest pick: how the human most recently said the work should be
    /// built, on a proposal Set of this Conversation's.
    ///
    /// `None` is nothing picked yet rather than a direction of none — a
    /// Conversation whose grilling has not put a proposal to the human has had
    /// nothing to pick on. A later pick replaces an earlier one, because a later
    /// proposal supersedes the one before it.
    pub direction: Option<Direction>,

    /// Which roadmap this Conversation is adopting, where it is adopting one.
    ///
    /// `None` is every Conversation started from the new-conversation box: they
    /// begin with a Brief and a grilling. `Some` is one started from the
    /// abandoned-roadmaps notice, and the directory name inside is the whole of
    /// what is stored about the roadmap — see [`start_adoption`].
    pub adopting: Option<String>,
}

/// One row of the conversations sidebar, drawn without reading a Timeline.
///
/// The branch is the row's name. A Conversation has no title of its own — the
/// domain gives it a Repo, a Brief, a branch and a base commit and nothing else
/// — and of those the branch is the one short line the human chose, which is
/// what a list is read by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationRow {
    pub id: i64,
    pub branch: String,

    /// What the Repo is called, which is the only thing about it a row shows.
    pub repo: String,

    pub state: Lifecycle,

    /// Whether anything about this Conversation is waiting on the human.
    ///
    /// One fact folded from every source there is, rather than a list the row's
    /// reader is left to weigh: what the sidebar says is *this one wants you*,
    /// and which source said so is the Conversation's own page to show. The
    /// sources are [`conversations`]'s, all of them in the one query.
    pub waiting: bool,
}

/// The word the `kind` column holds for a Question Set.
///
/// A constant, alone among the kinds, because it is the one that has to be
/// written without an Event to hand: [`ask`] inserts the Event and the Set in
/// one transaction, and the Set is not yet a [`SetOnTimeline`] at the moment the
/// row is written.
const QUESTION_SET: &str = "question-set";

/// And the word it holds for the pull request the finish step opened. A
/// constant for the same kind of reason, one module along: the row it hangs off
/// is written by [`super::pull_requests`], and the kind is wanted there without
/// an Event to hand.
const PULL_REQUEST: &str = "pull-request";

/// One entry in a Conversation's Timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvent {
    pub id: i64,

    /// When it landed, RFC 3339.
    pub at: String,

    pub event: Event,
}

/// What an Event is. The kinds the stages so far produce — the rest of the
/// table in the design arrives with the stages that produce them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The Brief: the markdown the Conversation starts from, as the human last
    /// wrote it.
    Brief(String),

    /// The Conversation moved, and this is the state it moved to.
    ///
    /// One kind for every move rather than one per destination: what the
    /// Timeline is recording is that the work changed hands, and the state it
    /// changed to is the only thing that differs between one move and the next.
    /// Starting to grill and closing both land here.
    Moved(Lifecycle),

    /// A session's output, summarised. The whole of it is the Capture beside
    /// it — see [`super::captures`] — which is what the details pane shows
    /// and what this is a line of.
    ///
    /// The only Event whose body is not in the `body` column: a Capture is
    /// written a chunk at a time for as long as a session runs, and a column
    /// that was rewritten whole on every chunk would cost more the longer the
    /// session went on.
    AgentOutput(super::Summary),

    /// A Question Set the session put to the human, with however far it has got.
    ///
    /// Its body is not in the `body` column either, and for a different reason
    /// from the Capture's: a Set is already a row of its own in
    /// `question_sets`, answered through the same tables whether it is reached
    /// through a Conversation or through `curl`. A second copy on the Timeline
    /// would be a second thing to keep true.
    ///
    /// Boxed, alone among the variants: a Set is the whole of what an agent
    /// asked, and an enum every Brief and every move was as large as would cost
    /// a Timeline's worth of memory to hold the one kind that needs it.
    QuestionSet(Box<SetOnTimeline>),

    /// The handoff document the grilling session wrote on its way out.
    ///
    /// Markdown in the `body` column like the Brief, because that is what it is:
    /// a document the human reads and the implementation session is primed with.
    /// It is written outside the Worktree and taken onto the Timeline when the
    /// proposal is accepted — see `crate::handoffs` in the server — so the copy
    /// here is the only one that lasts.
    Handoff(String),

    /// A commit a session landed on the Conversation's branch, summarised.
    ///
    /// Its body is not in the `body` column either, and for the Capture's
    /// reason rather than the Set's: what a commit is worth saying about it is
    /// five separate facts, and a row of them is a row of them. The diff is in
    /// neither place — see [`super::commits`] — because the repository has it.
    Commit(super::Commit),

    /// The pull request the finish step opened, as the host's `gh` found it —
    /// see [`super::pull_requests`].
    ///
    /// Its body is not in the `body` column either, for the commit's reason:
    /// what a PR is, here, is a row of separate facts. What it is *not* is the
    /// commit list and the comments — those move for as long as the PR is open,
    /// and are fetched when somebody looks.
    PullRequest(super::PullRequest),

    /// A run waiting out an Agent Profile's window: which account ran out, when
    /// it comes back, and what ended the wait — see [`super::pauses`].
    ///
    /// Its body is not in the `body` column either, for the commit's reason:
    /// what a Pause is, is a row of separate facts. Unboxed, unlike the Question
    /// Set beside it — three short strings is no more than a Brief carries, and
    /// nothing here is gathered evidence of a size to make the enum large.
    Pause(super::Pause),

    /// Something Verkstead has to say on its own account: which stage it has
    /// started and where the branch went, or that a roadmap has no stages left
    /// to run.
    ///
    /// Markdown in the `body` column like the Brief, and the one Event no agent
    /// and no human writes. It is what unattended running is owed: a decision
    /// Verkstead made without being asked is one the human has to be able to
    /// read afterwards, and a decision only the log knows about is one nobody
    /// looking at the work will ever find.
    ///
    /// Never something to do about, whatever it says: a Notice is written
    /// after the fact, and what a run that stopped is waiting on is the stop
    /// on the Conversation rather than anything on the Timeline — see
    /// [`super::stops`].
    Notice(String),

    /// A Manual Task: the instruction the human typed at the end of the
    /// Timeline for a one-off session to carry out.
    ///
    /// Markdown in the `body` column like the Brief and the handoff, because
    /// that is what it is — one document, written by a human for an agent to
    /// read. Nothing is joined in beside it: a Manual Task is its instruction,
    /// and what the session it starts does lands as the Events that work lands
    /// as.
    ///
    /// Beside the run rather than a step of it. It moves no state, and it is not
    /// a Step — the unattended unit a done file ends — however much the two look
    /// alike from the session's end.
    ManualTask(String),

    /// A Steer: the state the human moved the Conversation into, and whatever
    /// they wrote to send it there with.
    ///
    /// Its own kind beside the [`Event::Moved`] line the move writes, and the
    /// two say different things about the same moment. A move is the machine
    /// recording where the work got to; this is the human saying they put it
    /// there, which is the one thing a Timeline of moves alone could never be
    /// read back for.
    ///
    /// The target on the first line of the `body` column, exactly as a move
    /// holds the state it went to, and the instruction under it where the
    /// human wrote one — the hand-written work a steer into Implementing
    /// carries, kept as they wrote it. A steer into Grilling carries a
    /// document too and that one is not here: it opens a round, and what a
    /// round starts from is a Brief Event like any other.
    ///
    /// One line and no instruction is every steer written before there was one
    /// to write, and it reads back as the steer it was — ADR-0006's rule, and
    /// the reason the target goes above the document rather than under it.
    Steer(Lifecycle, Option<String>),
}

/// A Question Set as its Timeline Event holds it: which Set, what it asked, and
/// how far it has got.
///
/// The whole Set rather than a summary written down beside it, unlike the
/// Capture's. What the Timeline shows of a Set is a table of its Questions
/// against their Answers, and both halves of that move — the Questions when the
/// Set arrives and the Answers when the human replies — so a stored summary
/// would be two write paths for one row. A Conversation's Sets are counted in
/// tens, and this is a `JOIN` and a `serde_json` per one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetOnTimeline {
    pub set_id: i64,

    /// What the agent asked — or the stored body where this build can no longer
    /// read it, which is a row on the Timeline like any other rather than the
    /// end of reading it. See [`super::Asked`].
    pub set: super::Asked,

    /// How it was settled, or `None` while it is still waiting on the human.
    pub settlement: Option<super::Settlement>,

    /// Whether it was a Deferred Ask, which is what tells one still waiting on
    /// the human from a blocking one: both are something to answer, and nothing
    /// is idling on this one — see [`super::deferrals`].
    pub deferred: bool,
}

impl Event {
    /// The word the `kind` column holds. `'static`, so the one statement that
    /// wants the word without an Event to hand can ask for it and let the Event
    /// go.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Brief(_) => "brief",
            Self::Moved(_) => "moved",
            Self::AgentOutput(_) => "agent-output",
            Self::QuestionSet(_) => QUESTION_SET,
            Self::Handoff(_) => "handoff",
            Self::Commit(_) => "commit",
            Self::PullRequest(_) => PULL_REQUEST,
            Self::Pause(_) => super::pauses::PAUSE,
            Self::Notice(_) => "notice",
            Self::ManualTask(_) => "manual-task",
            Self::Steer(..) => "steer",
        }
    }

    /// What goes in the `body` column beside the kind.
    ///
    /// Borrowed for every kind but one, which is what the `Cow` is for: a
    /// steer's body is the state it named with the instruction under it, which
    /// is two things joined rather than one thing held.
    fn body(&self) -> Cow<'_, str> {
        match self {
            Self::Brief(markdown) => Cow::Borrowed(markdown),
            Self::Moved(state) => Cow::Borrowed(state.stored()),
            // Nothing: what a session printed is in the Capture tables, and
            // what the Timeline shows of it is read back from there too.
            Self::AgentOutput(_) => Cow::Borrowed(""),
            // Nothing either, and for the nearer reason: a Set is a row in
            // `question_sets` already.
            Self::QuestionSet(_) => Cow::Borrowed(""),
            Self::Handoff(markdown) => Cow::Borrowed(markdown),
            // Nothing, for the Capture's reason: what a commit is, is a row
            // in `commits`.
            Self::Commit(_) => Cow::Borrowed(""),
            // Nothing either, and for the commit's reason.
            Self::PullRequest(_) => Cow::Borrowed(""),
            // Nothing either, and for the commit's reason again.
            Self::Pause(_) => Cow::Borrowed(""),
            Self::Notice(markdown) => Cow::Borrowed(markdown),
            Self::ManualTask(instruction) => Cow::Borrowed(instruction),
            // The state it was steered into, as a move holds the state it moved
            // to — with the instruction under it where the human wrote one, on
            // lines of its own so that the word above it reads back whole.
            Self::Steer(target, None) => Cow::Borrowed(target.stored()),
            Self::Steer(target, Some(instruction)) => {
                Cow::Owned(format!("{}\n{instruction}", target.stored()))
            }
        }
    }

    /// The Event a row holds, with whatever was joined in beside it — the
    /// summary for an agent-output row, the Set for a question-set one.
    ///
    /// Each of those is written in the same transaction as its Event, so one
    /// without the other is a database somebody has been in by hand — worth
    /// saying rather than reading as a session that printed nothing or a Set
    /// that asked nothing.
    ///
    /// One parameter per kind of row that can be joined in, which is what makes
    /// the list long: they are the Timeline's own columns rather than an
    /// argument list somebody chose, and gathering them into a struct would be a
    /// second shape to keep true beside the query that fills it.
    #[allow(clippy::too_many_arguments)]
    fn read(
        kind: &str,
        body: String,
        summary: Option<super::Summary>,
        set: Option<SetOnTimeline>,
        commit: Option<super::Commit>,
        pull_request: Option<super::PullRequest>,
        pause: Option<super::Pause>,
    ) -> Result<Self> {
        Ok(match kind {
            "brief" => Self::Brief(body),
            "moved" => Self::Moved(Lifecycle::read(&body)?),
            "agent-output" => Self::AgentOutput(
                summary.ok_or_else(|| anyhow!("a session's output has no Capture beside it"))?,
            ),
            QUESTION_SET => Self::QuestionSet(Box::new(
                set.ok_or_else(|| anyhow!("a Question Set Event has no Set beside it"))?,
            )),
            "handoff" => Self::Handoff(body),
            "commit" => Self::Commit(
                commit.ok_or_else(|| anyhow!("a Commit Event has no commit beside it"))?,
            ),
            PULL_REQUEST => Self::PullRequest(
                pull_request
                    .ok_or_else(|| anyhow!("a pull request Event has no pull request beside it"))?,
            ),
            super::pauses::PAUSE => {
                Self::Pause(pause.ok_or_else(|| anyhow!("a Pause Event has no Pause beside it"))?)
            }
            "notice" => Self::Notice(body),
            "manual-task" => Self::ManualTask(body),
            // The state off the first line and the instruction under it, split
            // rather than parsed: an instruction is a document and may hold
            // anything, so the only thing read out of it is where it starts.
            "steer" => match body.split_once('\n') {
                Some((target, instruction)) => {
                    Self::Steer(Lifecycle::read(target)?, Some(instruction.to_owned()))
                }
                None => Self::Steer(Lifecycle::read(&body)?, None),
            },
            other => bail!("a Timeline holds an Event of the unknown kind {other:?}"),
        })
    }
}

/// What became of an edit to a drafting Conversation.
///
/// One outcome type for the three of them — the Brief, the branch name and the
/// base commit — because they are refused for the same two reasons, and a
/// caller telling them apart would be telling apart the same sentence three
/// times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edited {
    /// Recorded.
    Saved,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// It is past drafting, so this is not the human's to change any more.
    NotDrafting,
}

/// What became of choosing one of a Conversation's two Pairings.
///
/// A drafting refusal among them, like the Brief and the branch name and for
/// the same reason: both Pairings are fixed when grilling starts. The
/// implementation one is used long after that, but what it is has to be settled
/// before the work begins rather than swapped underneath it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chosen {
    /// Recorded.
    Chosen,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// There is no Profile with that id to choose.
    NoSuchProfile,

    /// It is past drafting, so both Pairings are fixed.
    NotDrafting,
}

/// What became of starting a Conversation grilling.
///
/// Only the two refusals the store is in a position to make. Everything else
/// starting is refused for — an unchosen Profile, an empty Brief, a base commit
/// nothing answers to — is decided above it, against the Profiles and against
/// git, and is settled by the time this is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grilling {
    /// Recorded: the base commit, the worktree, the state and the Event.
    Started,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// It is past drafting, so it has been started once already — or closed.
    NotDrafting,
}

/// What became of starting a roadmap stage's Conversation working.
///
/// The same two refusals [`Grilling`] has, one phase further along and for the
/// same reason: everything else this could be refused for is settled against git
/// before the record is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staged {
    /// Recorded: the base commit, the worktree, the direction, the state and the
    /// move.
    Started,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// It is past drafting, so something has started it already.
    NotDrafting,
}

/// What became of a direction picked on a wrap-up proposal.
///
/// Driven by a Response arriving rather than by anything the human pressed, so
/// [`Directing::NotGrilling`] is an ordinary outcome and not a mistake: a second
/// proposal Set answered after the first has nothing left to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Directing {
    /// Recorded, and the Conversation is still grilling: whichever direction was
    /// picked, the session that proposed writes its artifact itself, so what
    /// moves the Conversation is that artifact landing rather than the answer.
    Writing,

    /// It was not grilling, so there was no grilling for this to end. Nothing
    /// recorded and nothing wrong.
    NotGrilling,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// What became of the work starting once a grilling's own tail has landed.
///
/// The other half of a [`Directing::Writing`] pick: the pick recorded the
/// direction and left the Conversation grilling, and this is the move that
/// follows the backlog a task list writes, or the handoff an inline tail writes.
/// A roadmap tail is the same shape one rung further along — the roadmap goes
/// for review as it lands, so what follows it is [`super::record_pull_request`]
/// rather than this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Implementing {
    /// Recorded: the Conversation is being built, and the move is on its
    /// Timeline.
    Started,

    /// It was not grilling, so there was no grilling for this to end. Nothing
    /// recorded and nothing wrong.
    NotGrilling,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// What became of reopening a finished one.
///
/// Reopening twice has no outcome of its own, unlike closing twice: the first
/// press leaves the Conversation drafting, and a second finds a state that is
/// not Done — which is [`Reopening::NotDone`], the round being open already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reopening {
    /// Reopened: the Conversation is drafting again, with a Brief of its own to
    /// write and the round boundary on its Timeline.
    Reopened,

    /// It is not Done, so there is no finished round here to open another
    /// after. Every other state is somewhere the work has got to, and Closed
    /// is off the ladder rather than on it.
    NotDone,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// What became of sending a wrapping Conversation back to be built.
///
/// The one way back down the ladder, and the only thing that takes it: a review
/// whose findings were too big to fix in one sitting splits them out as a
/// backlog, and a backlog is built rather than wrapped. What follows it is the
/// finish step and [`super::record_pull_request`] again, which is the second wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rebuilding {
    /// Recorded: the Conversation is being built again, the move is on its
    /// Timeline, and its review is back to waiting.
    Started,

    /// It is not wrapping up, so there is no wrap-up here to leave — it was
    /// closed out from under the session, or it is being built already.
    NotWrapping,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// What became of steering one.
///
/// Nothing here is about the state the Conversation was in, and that is the
/// point of a steer: the human has looked at the work and said where it goes, so
/// every source is a source — a draft, a run in flight, a Conversation Verkstead
/// has finished with. What is left to be wrong about is which Conversation was
/// named, and which Profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Steering {
    /// Recorded: the Steer Event, the state, the move on the Timeline, and the
    /// Pairing where the human picked one.
    Steered,

    /// There is no Conversation with that id.
    NoSuchConversation,

    /// The Pairing picked names a Profile that is not there — removed between
    /// the list the modal read and the pick it made from it. Nothing is moved:
    /// the move and the Pairing are one act.
    NoSuchProfile,
}

/// What became of closing one.
///
/// Closing twice is not an error, which is what [`Closing::AlreadyClosed`] is
/// for: it is a distinct outcome rather than a failure, because the thing the
/// human asked for holds either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Closing {
    /// Closed: the worktree is forgotten, the state is [`Lifecycle::Closed`],
    /// and the move is on the Timeline.
    Closed,

    /// It was closed already. Nothing to record and nothing wrong.
    AlreadyClosed,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// The tables a Conversation and its Timeline live in.
///
/// The Timeline is indexed by the Conversation it belongs to, because that is
/// the only way it is ever read: a Timeline is one Conversation's, whole and in
/// order.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS conversations (
             id                        INTEGER PRIMARY KEY AUTOINCREMENT,
             repo_id                   INTEGER NOT NULL REFERENCES repos(id),
             created_at                TEXT NOT NULL,
             branch                    TEXT NOT NULL,
             base_commit               TEXT,
             state                     TEXT NOT NULL,
             grilling_profile_id       INTEGER REFERENCES profiles(id),
             implementation_profile_id INTEGER REFERENCES profiles(id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the conversations table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS timeline_events (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             at              TEXT NOT NULL,
             kind            TEXT NOT NULL,
             body            TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the timeline_events table")?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS timeline_events_conversation
             ON timeline_events (conversation_id, id)",
    )
    .execute(pool)
    .await
    .context("indexing the Timeline by its Conversation")?;

    // The worktree hangs off a Conversation rather than being a column on it,
    // as an archiving hangs off a Set: there is no migration machinery here and
    // `conversations` is STRICT and left alone. One worktree per Conversation,
    // by the primary key — and a Conversation that has none has no row, which is
    // both the state before grilling and the state after closing.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS worktrees (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             path            TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the worktrees table")?;

    // Which Timeline Event a Question Set landed on, and so — through the Event —
    // which Conversation it was asked from. A table of its own rather than a
    // column on `question_sets` for the reason the worktree is not a column on
    // `conversations`: there is no migration machinery here and that table is
    // STRICT and left alone.
    //
    // One Set per Event and one Event per Set, by the primary key and the unique
    // index: a Set is put once, and an Event that held two of them would be a row
    // of the Timeline that could not say which it was.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS set_events (
             set_id   INTEGER PRIMARY KEY REFERENCES question_sets(id),
             event_id INTEGER NOT NULL UNIQUE REFERENCES timeline_events(id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the set_events table")?;

    // How the human chose to have the work built, once they have. A table of its
    // own for the reason the worktree is one: there is no migration machinery
    // here and `conversations` is STRICT and left alone. One direction per
    // Conversation, by the primary key — and one that has not been chosen for
    // has no row, which is the state every Conversation starts in.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS directions (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             direction       TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the directions table")?;

    // What a roadmap stage's branch was put on top of, where it was put on top
    // of anything. A table of its own for the reason the direction is one, and
    // one row per stage Conversation — written by [`start_stage`] and by nothing
    // else, because it is a decision taken once, at the moment the branch is
    // made, and never again.
    //
    // Verkstead's own fact rather than the repository's, which is why it is
    // stored where almost nothing about a roadmap is: the boxes and the briefs
    // belong to `docs/roadmaps/` and are read back off the Worktree, but *which
    // branch this one stacks on* is something Verkstead decided and the
    // repository never records.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stage_branches (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             stacks_on       TEXT
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the stage branches table")?;

    // The model half of a Conversation's two Pairings, one row per role. A
    // table of its own for the reason the direction is one: there is no
    // migration machinery here and `conversations` is STRICT and left alone —
    // so the Profile half stays in the column it has always been in and the
    // model half arrives beside it.
    //
    // One row per role by the primary key, and a role with no row is one whose
    // Pairing has no model: either nothing has been chosen for it at all, or it
    // was chosen before pairings existed, which the Profile column tells apart.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pairing_models (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             role            TEXT NOT NULL,
             model           TEXT NOT NULL,
             PRIMARY KEY (conversation_id, role)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pairing models table")?;

    // Which roadmap a drafting Conversation is adopting, where it is adopting
    // one. A table of its own for the reason the direction is one: there is no
    // migration machinery here and `conversations` is STRICT and left alone.
    // One roadmap per Conversation by the primary key, and a Conversation that
    // is adopting nothing has no row — which is every Conversation started from
    // the new-conversation box.
    //
    // The roadmap's directory name and nothing else. What that roadmap says —
    // its stages, its boxes, its briefs — is the repository's and is read back
    // off it wherever it is wanted; a copy kept here would be a second opinion
    // about a document Verkstead does not own.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS adoptions (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             roadmap         TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the adoptions table")?;

    collapse_the_direction_state(pool).await?;

    Ok(())
}

/// Take the retired Direction state out of a database written before it left the
/// ladder.
///
/// Direction was where a Conversation sat between its grilling ending and a
/// human pressing a separate chooser. The pick rides the closing Set now, so
/// nothing ever waits there — and a Conversation *found* waiting there is
/// stranded: its grilling session is over, and the chooser it was waiting on is
/// gone. Closed is what that is, which is where this puts it, with the move on
/// its Timeline as every close has one.
///
/// The retired Events go with the state. A Timeline holding a move to `direction`
/// or a `directed` Event is one this Verkstead cannot read at all — every state
/// and every kind is a word it knows or an error — and both of them are about
/// the machinery being removed rather than about the work. What says the human
/// chose is the answered proposal Set, which stays exactly where it was.
///
/// No compatibility path, which is what makes this a one-way collapse: this is a
/// single-user tool, and in-flight Conversations are finished or closed before
/// upgrading. Safe to run against a database that has been through it — after the
/// first run there is nothing left to match.
async fn collapse_the_direction_state(pool: &SqlitePool) -> Result<()> {
    // Before the state is rewritten, while the rows to close can still be found
    // by it.
    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 'moved', ?
         FROM conversations WHERE state = 'direction'",
    )
    .bind(Lifecycle::Closed.stored())
    .execute(pool)
    .await
    .context("recording the closing of every Conversation caught choosing a direction")?;

    // The worktree row goes with it, as it does at [`close_conversation`]: a
    // closed Conversation has none. The directory itself is not the store's to
    // remove, and never was.
    sqlx::query(
        "DELETE FROM worktrees
         WHERE conversation_id IN (SELECT id FROM conversations WHERE state = 'direction')",
    )
    .execute(pool)
    .await
    .context("forgetting the worktrees of the Conversations caught choosing a direction")?;

    sqlx::query("UPDATE conversations SET state = ? WHERE state = 'direction'")
        .bind(Lifecycle::Closed.stored())
        .execute(pool)
        .await
        .context("closing every Conversation caught choosing a direction")?;

    sqlx::query(
        "DELETE FROM timeline_events
         WHERE kind = 'directed' OR (kind = 'moved' AND body = 'direction')",
    )
    .execute(pool)
    .await
    .context("taking the retired Direction Events off the Timelines")?;

    Ok(())
}

/// Start a Conversation against a registered Repo, on `branch`, with an empty
/// Brief already in its Timeline.
///
/// `None` means there is no such Repo. The insert selects from `repos` rather
/// than trusting the id, so a Conversation cannot come to hang off a repository
/// that was never registered — SQLite does not enforce a foreign key unless it
/// is asked to, and a row that named nothing would be a Conversation with
/// nowhere to work.
///
/// The Brief goes in with it, in the same transaction: the Brief is the first
/// Event, and a Conversation whose Timeline was empty because the second insert
/// failed would be one the human could not write anything into.
pub async fn start_conversation(
    pool: &SqlitePool,
    repo_id: i64,
    branch: &str,
) -> Result<Option<i64>> {
    started(pool, repo_id, branch, None).await
}

/// Start a Conversation adopting `roadmap` against a registered Repo, on
/// `branch`, with an empty Brief already in its Timeline.
///
/// The same Conversation as any other, with one thing more written about it:
/// which roadmap it is adopting. That mark is what the adoption-shaped page is
/// drawn from, and it is the only thing about the roadmap that is Verkstead's —
/// its stages, its boxes and its briefs are read back off the repository
/// wherever they are wanted.
///
/// The roadmap is taken as given. Whether it is there and whether it has a
/// stage to start are questions about a repository at a commit, answered where
/// the repository is read and answered again when the human presses Adopt.
pub async fn start_adoption(
    pool: &SqlitePool,
    repo_id: i64,
    branch: &str,
    roadmap: &str,
) -> Result<Option<i64>> {
    started(pool, repo_id, branch, Some(roadmap)).await
}

/// What both of them do: the row, its empty Brief, and the adoption mark where
/// there is one to write.
///
/// All of it in one transaction. A Conversation whose Timeline was empty
/// because the second insert failed would be one the human could not write
/// anything into, and one that lost its mark to a third would be a Draft drawn
/// on the wrong page.
async fn started(
    pool: &SqlitePool,
    repo_id: i64,
    branch: &str,
    adopts: Option<&str>,
) -> Result<Option<i64>> {
    let mut tx = pool.begin().await.context("starting a Conversation")?;

    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO conversations (repo_id, created_at, branch, base_commit, state)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, NULL, ?
         FROM repos WHERE id = ?
         RETURNING id",
    )
    .bind(branch)
    .bind(Lifecycle::Draft.stored())
    .bind(repo_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("starting a Conversation on Repo {repo_id}"))?;

    let Some((id,)) = row else {
        return Ok(None);
    };

    // Empty, because nothing has been written yet. It is an Event all the same:
    // the Brief is the first thing on the Timeline whether or not it says
    // anything, and the Timeline is where the human writes it.
    //
    // An adopting Conversation has nobody to write it: its Brief is the stage
    // brief, and it arrives when the stage is adopted.
    let brief = Event::Brief(String::new());
    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
    )
    .bind(id)
    .bind(brief.kind())
    .bind(brief.body().into_owned())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("writing the Brief of Conversation {id}"))?;

    if let Some(roadmap) = adopts {
        sqlx::query("INSERT INTO adoptions (conversation_id, roadmap) VALUES (?, ?)")
            .bind(id)
            .bind(roadmap)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("recording what Conversation {id} is adopting"))?;
    }

    tx.commit().await.context("starting a Conversation")?;

    Ok(Some(id))
}

/// Every Conversation in the order the human put them in, and whether each is
/// waiting on the human.
///
/// The order is theirs: this is one person's working set, and which piece of
/// work sits at the top is something they say by dragging a row rather than
/// something a sort decides — see [`super::place_conversations`], which is where
/// what they said is kept.
///
/// What has never been placed goes above what has, newest first among itself.
/// A Conversation started a minute ago is the one thing on this list nobody has
/// had the chance to place, and putting it at the top is both the predictable
/// answer and the useful one: it arrives where it will be seen, and the hand-made
/// order underneath it is left exactly as it was.
///
/// `waiting` is an `OR` over the sources, computed here rather than by the
/// caller, because every one of them is a read of this database and the sidebar
/// is one list: a caller folding them itself would be issuing a query per row
/// for facts a subselect already has.
///
/// The sources, in the order they appear below:
///
/// - A **Question Set with no Response and no archiving** — an ask left open.
///   Blocking and Deferred alike: what draws the human is that there is
///   something answerable, not whether the asking session is idling on it.
/// - A **stop**, which is a Conversation nothing is driving any more and which
///   goes again only when the human says so — however it stopped, an account
///   out of window included. A column on the row rather than a subselect, so
///   the whole list costs one query.
///
/// A grilling waiting on its closing proposal is the first of them and not a
/// source of its own: the proposal rides a Question Set, and an unanswered Set
/// is already an ask left open.
///
/// A **Draft** is none of them, whatever else is true of it: it is waiting on
/// the human in the ordinary sense, and the sidebar says so by drawing it as a
/// draft rather than by marking it as an ask.
pub async fn conversations(pool: &SqlitePool) -> Result<Vec<ConversationRow>> {
    let rows: Vec<(i64, String, String, String, bool)> = sqlx::query_as(
        "SELECT c.id, c.branch, r.name, c.state,
                c.state <> 'draft' AND (
                    EXISTS (
                        SELECT 1 FROM set_events s
                        JOIN timeline_events e ON e.id = s.event_id
                        WHERE e.conversation_id = c.id
                          AND NOT EXISTS (
                              SELECT 1 FROM responses p WHERE p.set_id = s.set_id
                          )
                          AND NOT EXISTS (
                              SELECT 1 FROM archivings a WHERE a.set_id = s.set_id
                          )
                    )
                    OR c.stopped_at IS NOT NULL
                ) AS waiting
         FROM conversations c
         JOIN repos r ON r.id = c.repo_id
         LEFT JOIN placements m ON m.conversation_id = c.id
         ORDER BY m.place IS NULL DESC, m.place, c.id DESC",
    )
    .fetch_all(pool)
    .await
    .context("listing the Conversations")?;

    rows.into_iter()
        .map(|(id, branch, repo, state, waiting)| {
            Ok(ConversationRow {
                id,
                branch,
                repo,
                state: Lifecycle::read(&state)?,
                waiting,
            })
        })
        .collect()
}

/// One Conversation with its Repo and whichever Profiles it has chosen, or
/// `None` if there is no such Conversation.
///
/// The Profiles are fetched beside the row rather than joined into it: they are
/// each optional, they are read back whole, and two more `LEFT JOIN`s' worth of
/// columns to unpack would say nothing the two small reads do not.
pub async fn load_conversation(pool: &SqlitePool, id: i64) -> Result<Option<Conversation>> {
    /// The columns in the order the query below selects them.
    type Row = (
        i64,
        String,
        String,
        Option<String>,
        String,
        Option<i64>,
        Option<i64>,
        i64,
        String,
        String,
        String,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT c.id, c.created_at, c.branch, c.base_commit, c.state,
                c.grilling_profile_id, c.implementation_profile_id,
                r.id, r.path, r.name, r.default_branch
         FROM conversations c
         JOIN repos r ON r.id = c.repo_id
         WHERE c.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("loading Conversation {id}"))?;

    let Some((
        id,
        created_at,
        branch,
        base_commit,
        state,
        grilling_profile_id,
        implementation_profile_id,
        repo_id,
        repo_path,
        repo_name,
        default_branch,
    )) = row
    else {
        return Ok(None);
    };

    Ok(Some(Conversation {
        id,
        created_at,
        repo: super::Repo {
            id: repo_id,
            path: std::path::PathBuf::from(repo_path),
            name: repo_name,
            default_branch,
        },
        branch,
        base_commit: base_commit.filter(|commit| !commit.is_empty()),
        state: Lifecycle::read(&state)?,
        grilling_pairing: pairing(pool, id, Role::Grilling, grilling_profile_id).await?,
        implementation_pairing: pairing(pool, id, Role::Implementation, implementation_profile_id)
            .await?,
        worktree: worktree(pool, id).await?,
        direction: direction(pool, id).await?,
        adopting: adopting(pool, id).await?,
    }))
}

/// One of a Conversation's two Pairings: the Profile its column names, and the
/// model paired with it where one was.
///
/// A role with no Profile has no Pairing at all, whatever `pairing_models`
/// holds: the model half alone is nothing to run a session under.
async fn pairing(
    pool: &SqlitePool,
    conversation: i64,
    role: Role,
    profile_id: Option<i64>,
) -> Result<Option<super::Pairing>> {
    let Some(profile_id) = profile_id else {
        return Ok(None);
    };

    let Some(profile) = super::load_profile(pool, profile_id).await? else {
        return Ok(None);
    };

    let row: Option<(String,)> =
        sqlx::query_as("SELECT model FROM pairing_models WHERE conversation_id = ? AND role = ?")
            .bind(conversation)
            .bind(role.stored())
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading the model Conversation {conversation} paired"))?;

    Ok(Some(super::Pairing {
        profile,
        model: row.map(|(model,)| model),
    }))
}

/// Where a Conversation's worktree was put, if it has one.
async fn worktree(pool: &SqlitePool, id: i64) -> Result<Option<std::path::PathBuf>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT path FROM worktrees WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading the worktree of Conversation {id}"))?;

    Ok(row.map(|(path,)| std::path::PathBuf::from(path)))
}

/// How a Conversation's work is to be built, if the human has chosen yet.
async fn direction(pool: &SqlitePool, id: i64) -> Result<Option<Direction>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT direction FROM directions WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading the direction of Conversation {id}"))?;

    row.map(|(word,)| direction_read(&word)).transpose()
}

/// Which roadmap a Conversation is adopting, where it is adopting one.
///
/// A read of its own beside the row, as the worktree and the direction are:
/// almost no Conversation has one, and a `LEFT JOIN`'s worth of column would
/// say nothing this small read does not.
pub async fn adopting(pool: &SqlitePool, id: i64) -> Result<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT roadmap FROM adoptions WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading what Conversation {id} is adopting"))?;

    Ok(row.map(|(roadmap,)| roadmap))
}

/// Which of the two roles a Pairing is being chosen for.
///
/// The word the `pairing_models` table holds, and the column the Profile half
/// goes in — the two halves of one choice, so the role names both rather than
/// letting a caller pass one and forget the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Grilling,
    Implementation,
}

impl Role {
    pub(crate) fn stored(self) -> &'static str {
        match self {
            Self::Grilling => "grilling",
            Self::Implementation => "implementation",
        }
    }

    pub(crate) fn column(self) -> &'static str {
        match self {
            Self::Grilling => "grilling_profile_id",
            Self::Implementation => "implementation_profile_id",
        }
    }
}

/// Choose the Pairing the grilling session will run under.
pub async fn set_grilling_pairing(
    pool: &SqlitePool,
    id: i64,
    profile_id: i64,
    model: Option<&str>,
) -> Result<Chosen> {
    choose(pool, id, Role::Grilling, profile_id, model).await
}

/// Choose the Pairing the implementation will run under.
pub async fn set_implementation_pairing(
    pool: &SqlitePool,
    id: i64,
    profile_id: i64,
    model: Option<&str>,
) -> Result<Chosen> {
    choose(pool, id, Role::Implementation, profile_id, model).await
}

/// Record one of the two choices, both halves of it.
///
/// Refused past drafting, which is what fixes a Pairing when grilling starts:
/// what runs the work is settled before the work starts, alongside the branch,
/// the base commit and the Brief. The one thing that re-settles one afterwards
/// is a steer, and it goes through [`settle`] rather than here — see
/// [`steer_conversation`].
///
/// Whether the Profile lists the model is decided above the store, where the
/// Profile is read as a row.
///
/// `model` is `None` for the one caller that carries a Pairing across rather
/// than making one — see [`Pairing::model`] — and it takes the model row away,
/// so a re-choice cannot leave the model half of an earlier one behind.
async fn choose(
    pool: &SqlitePool,
    id: i64,
    role: Role,
    profile_id: i64,
    model: Option<&str>,
) -> Result<Chosen> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(match refusal {
            Edited::NoSuchConversation => Chosen::NoSuchConversation,
            _ => Chosen::NotDrafting,
        });
    }

    let mut tx = pool
        .begin()
        .await
        .with_context(|| format!("choosing Profile {profile_id} for Conversation {id}"))?;

    if !settle(&mut tx, id, role, profile_id, model).await? {
        return Ok(Chosen::NoSuchProfile);
    }

    tx.commit()
        .await
        .with_context(|| format!("choosing Profile {profile_id} for Conversation {id}"))?;

    Ok(Chosen::Chosen)
}

/// Write one role's Pairing, both halves of it, inside somebody else's
/// transaction.
///
/// `false` means there is no Profile with that id — the one thing this can be
/// wrong about, and the caller's to name in its own words.
///
/// Apart from [`choose`] because a steer settles a Pairing too, and settles it
/// past drafting: what runs the work is fixed when grilling starts, and steering
/// is the human re-settling it from wherever the work has got to. What is common
/// to the two is this — the Profile column and the model row, written together,
/// because a Pairing is both halves and either alone is not something to launch
/// a session with. See [`steer_conversation`].
///
/// The Profile is selected from `profiles` inside the statement rather than
/// checked first, as a Conversation's Repo is: SQLite enforces a foreign key
/// only when asked to, and a column naming a Profile that is not there is a
/// session that fails to start with nobody watching.
pub(crate) async fn settle(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: i64,
    role: Role,
    profile_id: i64,
    model: Option<&str>,
) -> Result<bool> {
    let changed = sqlx::query(&format!(
        "UPDATE conversations
         SET {} = (SELECT id FROM profiles WHERE id = ?)
         WHERE id = ? AND EXISTS (SELECT 1 FROM profiles WHERE id = ?)",
        role.column()
    ))
    .bind(profile_id)
    .bind(id)
    .bind(profile_id)
    .execute(&mut **tx)
    .await
    .with_context(|| format!("choosing Profile {profile_id} for Conversation {id}"))?
    .rows_affected();

    if changed == 0 {
        return Ok(false);
    }

    sqlx::query("DELETE FROM pairing_models WHERE conversation_id = ? AND role = ?")
        .bind(id)
        .bind(role.stored())
        .execute(&mut **tx)
        .await
        .with_context(|| format!("clearing the model Conversation {id} had paired"))?;

    if let Some(model) = model {
        sqlx::query("INSERT INTO pairing_models (conversation_id, role, model) VALUES (?, ?, ?)")
            .bind(id)
            .bind(role.stored())
            .bind(model)
            .execute(&mut **tx)
            .await
            .with_context(|| format!("pairing {model:?} with Conversation {id}'s Profile"))?;
    }

    Ok(true)
}

/// A Conversation's Timeline, oldest first — which is reading order, and which
/// puts the Brief at the top where it was written.
///
/// Ordered by id rather than by `at`: the id is handed out in the order things
/// happened, and two Events stamped in the same millisecond must not come back
/// in an arbitrary one.
///
/// A Capture's summary is read for the whole Timeline rather than per Event,
/// and no Capture itself is: a Timeline is read every time an open page looks
/// again, and what a session printed is megabytes the middle pane never shows.
///
/// A Question Set's whole body *is* joined in, which is the one place this pays
/// for a deserialization per Event — see [`SetOnTimeline`] for why there is
/// nothing cheaper to read instead. One query all the same: the Sets, their
/// Responses and their archivings hang off the same Event rows, and asking per
/// Set would be a read for every Question the human has ever been put.
pub async fn timeline(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<TimelineEvent>> {
    /// The columns in the order the query below selects them: the Event, the
    /// Set with however it was settled that is there for one kind of Event, and
    /// the commit that is there for another.
    type Row = (
        i64,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    );

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT e.id, e.at, e.kind, e.body,
                q.id, q.body, r.submitted_at, r.body, a.archived_at,
                c.sha, c.subject, c.files, c.insertions, c.deletions
         FROM timeline_events e
         LEFT JOIN set_events s ON s.event_id = e.id
         LEFT JOIN question_sets q ON q.id = s.set_id
         LEFT JOIN responses r ON r.set_id = s.set_id
         LEFT JOIN archivings a ON a.set_id = s.set_id
         LEFT JOIN commits c ON c.event_id = e.id
         WHERE e.conversation_id = ?
         ORDER BY e.id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Timeline of Conversation {conversation_id}"))?;

    // The kinds that are not joined into the query above, and the reason is
    // arithmetic rather than judgement: the query is at the sixteen columns a
    // tuple can be read back as, and there is no seventeenth position to put
    // one in. Each is one more read for the whole Timeline rather than a read
    // per Event, which is what the joins were saving.
    //
    // The Capture summaries first, which is the one of the three that is on
    // nearly every Timeline: how much a session printed, how many turns its
    // conversation took, and the last thing it said.
    let mut summaries = super::captures::on_timeline(pool, conversation_id).await?;

    // And the pull request, for the same arithmetic and a cheaper read still:
    // there is one per Conversation, and until the finish step has run there is
    // none.
    let mut pull_requests = super::pull_requests::on_timeline(pool, conversation_id).await?;

    // And the Commit Summaries, for the same arithmetic. The commit itself is
    // joined into the query above; what the agent wrote under its subject is one
    // more read, and a Timeline of bookkeeping commits answers it with nothing.
    let mut summaries_of_commits =
        super::commits::summaries_on_timeline(pool, conversation_id).await?;

    // And the Pauses, for the arithmetic again and at the pull request's cost:
    // an account running out of window is the rare Event, so this is one more
    // query that nearly always comes back with nothing.
    let mut pauses = super::pauses::on_timeline(pool, conversation_id).await?;

    // And which of the Sets above were asked deferred, for the arithmetic again
    // — see [`super::deferrals::deferred_on_timeline`]. Cheaper than any of
    // them: one indexed column, and most Conversations have no deferred Set at
    // all.
    let deferred = super::deferrals::deferred_on_timeline(pool, conversation_id).await?;

    rows.into_iter()
        .map(|row| {
            let (
                id,
                at,
                kind,
                body,
                set_id,
                set_body,
                answered_at,
                answer,
                archived_at,
                sha,
                subject,
                files,
                insertions,
                deletions,
            ) = row;

            let commit = match (sha, subject, files, insertions, deletions) {
                (Some(sha), Some(subject), Some(files), Some(insertions), Some(deletions)) => {
                    Some(super::Commit {
                        sha,
                        subject,
                        files,
                        insertions,
                        deletions,
                        // Absent for most commits, which is what a commit that
                        // said nothing about itself looks like.
                        summary: summaries_of_commits.remove(&id),
                    })
                }
                // Every column of that row is `NOT NULL`, so the only way to be
                // missing any of them is to be missing the row: this Event is
                // not a Commit.
                _ => None,
            };

            let set = set_id
                .zip(set_body)
                .map(|(set_id, body)| -> Result<SetOnTimeline> {
                    Ok(SetOnTimeline {
                        set_id,
                        // Never a failure, whatever the body turns out to hold:
                        // an unreadable Set is a row of its own and the rest of
                        // the Timeline is drawn around it — see
                        // [`super::Asked`].
                        set: super::Asked::read(body),
                        settlement: settled(set_id, answered_at, answer, archived_at)?,
                        deferred: deferred.contains(&set_id),
                    })
                })
                .transpose()?;

            // Taken out rather than looked up, because each belongs to exactly
            // one Event and the Events are walked once.
            let summary = summaries.remove(&id);
            let pull_request = pull_requests.remove(&id);
            let pause = pauses.remove(&id);

            Ok(TimelineEvent {
                id,
                at,
                event: Event::read(&kind, body, summary, set, commit, pull_request, pause)?,
            })
        })
        .collect()
}

/// How a Set on the Timeline was settled, out of the two rows that can settle
/// one, or `None` while it is still waiting on the human.
///
/// The Response wins where both are somehow there, exactly as the Archive's own
/// listing has it: the answering is the decision, and it is the one already
/// filed.
fn settled(
    set_id: i64,
    answered_at: Option<String>,
    answer: Option<String>,
    archived_at: Option<String>,
) -> Result<Option<super::Settlement>> {
    if let Some((submitted_at, body)) = answered_at.zip(answer) {
        let response = serde_json::from_str(&body).with_context(|| {
            format!("deserialising the stored Response to Question Set {set_id}")
        })?;

        return Ok(Some(super::Settlement::Answered(super::StoredResponse {
            set_id,
            submitted_at,
            response,
        })));
    }

    Ok(archived_at.map(|archived_at| {
        super::Settlement::ArchivedUnanswered(super::SetArchived {
            set_id,
            archived_at,
        })
    }))
}

/// Put a Question Set on a Conversation's Timeline, stamping it with an id and a
/// creation time.
///
/// `None` means there is no such Conversation to ask from. The Event is inserted
/// by selecting from `conversations` rather than by checking first, as a
/// Conversation's own Repo is: SQLite enforces a foreign key only when asked to,
/// and a Set attributed to a Conversation that is not there is one nobody could
/// ever reach to answer.
///
/// One transaction, because a Set stored without its Event would be a Set no
/// Timeline showed and no human would ever see — and the agent would be blocked
/// on it.
///
/// The Set is expected to have been validated already — the store is not where
/// the question grammar is enforced.
///
/// `ask` is which of the two kinds it is, and the deferral is written in this
/// same transaction where it is a Deferred Ask: a Set that was stored a moment
/// before the record of how it was asked is one that reads as blocking for that
/// moment, and what reads it in that moment is a driver deciding whether a quiet
/// session is still waiting on an Answer.
pub async fn ask(
    pool: &SqlitePool,
    conversation_id: i64,
    set: &QuestionSet,
    ask: super::Ask,
) -> Result<Option<SetCreated>> {
    let body = serde_json::to_string(set).context("serialising the Question Set")?;

    let mut tx = pool.begin().await.context("putting a Question Set")?;

    let event: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ''
         FROM conversations WHERE id = ?
         RETURNING id",
    )
    .bind(QUESTION_SET)
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("putting a Question Set to Conversation {conversation_id}"))?;

    let Some((event_id,)) = event else {
        return Ok(None);
    };

    // The same insert the standalone one was, stamped by SQLite as it assigns
    // the id so that both come from one place.
    let (id, created_at): (i64, String) = sqlx::query_as(
        "INSERT INTO question_sets (created_at, title, project, branch, body)
         VALUES (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?, ?, ?)
         RETURNING id, created_at",
    )
    .bind(&set.title)
    .bind(&set.project)
    .bind(&set.branch)
    .bind(body)
    .fetch_one(&mut *tx)
    .await
    .context("storing the Question Set")?;

    sqlx::query("INSERT INTO set_events (set_id, event_id) VALUES (?, ?)")
        .bind(id)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("putting Question Set {id} on the Timeline"))?;

    if ask == super::Ask::Deferred {
        super::deferrals::defer(&mut tx, id).await?;
    }

    tx.commit().await.context("putting a Question Set")?;

    Ok(Some(SetCreated { id, created_at }))
}

/// Whether this Set is on this Conversation's Timeline.
///
/// What makes the agents' endpoints conversation-scoped mean anything: a session
/// reaches Verkstead through its own Conversation's base URL, and a Set id that
/// belongs to another Conversation names nothing there.
pub async fn set_asked_from(pool: &SqlitePool, conversation_id: i64, set_id: i64) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT s.set_id
         FROM set_events s
         JOIN timeline_events e ON e.id = s.event_id
         WHERE s.set_id = ? AND e.conversation_id = ?",
    )
    .bind(set_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("looking for Question Set {set_id} on Conversation {conversation_id}")
    })?;

    Ok(found.is_some())
}

/// Whether this Conversation's self-review has put its findings to the human,
/// and which Set they are on.
///
/// Read off the Sets themselves rather than written down when one is asked — see
/// [`proposals`]. The **first** of them is the review's: it is the session a
/// wrap-up starts with, and the batch sessions that propose the same way about
/// what was said on the pull request are all dispatched after it has settled.
pub async fn review_asked(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    Ok(proposals(pool, conversation_id).await?.first().copied())
}

/// And the newest of them, which is whatever was last put to the human: the
/// review's own Set until a batch of comments is answered after it, and that
/// batch's from then on.
///
/// The newest rather than the batch's own, because nothing on the record says
/// which session asked one and nothing has to. One Worktree holds one agent and
/// nothing advances past a stop, so the proposal a batch session made is the
/// last one there is for as long as anything is asking about it.
pub async fn last_proposal(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    Ok(proposals(pool, conversation_id).await?.last().copied())
}

/// Every Set of this Conversation's carrying a `review` block, oldest first —
/// and only the ones this wrap asked.
///
/// A Set carrying the block *is* a proposal to fix things, which is the whole
/// reason the block is a field being there rather than a convention. A second
/// record saying which Sets were which would be a second thing to keep true, and
/// the one that could disagree.
///
/// **This wrap's**, because a Conversation can wrap up more than once: a review
/// that splits its findings out into a backlog leaves Wrapping to build them and
/// comes back for a second wrap, and the first wrap's proposals are answered and
/// done with. Counting them would be a second review that never ran, because the
/// review it found asking was last month's. So the window opens at the newest
/// move into Wrapping — and where there has been no such move, at the start of
/// the Timeline, which is every Conversation that has not got that far.
///
/// **And only the ones still standing.** A Set archived unanswered is one nobody
/// is ever going to answer, which is what Verkstead closes a proposal whose
/// session is gone as — see [`super::archive_set`]. Counting one would be the
/// same mistake the other way about: the review it found asking is a question
/// nothing is left to act on, so no fresh reading of the branch could ever be
/// recognised as the review of this wrap.
async fn proposals(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<i64>> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT q.id, q.body
         FROM question_sets q
         JOIN set_events s ON s.set_id = q.id
         JOIN timeline_events e ON e.id = s.event_id
         LEFT JOIN archivings a ON a.set_id = q.id
         WHERE e.conversation_id = ?
           AND a.set_id IS NULL
           AND e.id > COALESCE(
                   (SELECT MAX(w.id) FROM timeline_events w
                    WHERE w.conversation_id = ? AND w.kind = ? AND w.body = ?),
                   0)
         ORDER BY q.id",
    )
    .bind(conversation_id)
    .bind(conversation_id)
    .bind(Event::Moved(Lifecycle::Wrapping).kind())
    .bind(Lifecycle::Wrapping.stored())
    .fetch_all(pool)
    .await
    .with_context(|| format!("looking for Conversation {conversation_id}'s proposals"))?;

    let mut proposing = Vec::new();

    for (set_id, body) in rows {
        // A Set this build cannot read is passed over rather than failing the
        // question: it carries no `review` block anybody here could act on, and
        // one unreadable body must not be able to tell a Conversation it has no
        // review when what happened is that a field left the schema.
        let asked = super::Asked::read(body);

        let Some(set) = asked.set() else {
            continue;
        };

        if set.review.is_some() {
            proposing.push(set_id);
        }
    }

    Ok(proposing)
}

/// One finding the human said to fix, as the session that will fix it is told
/// about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixing {
    /// The finding as the review wrote it for whoever fixes it.
    pub what: String,

    /// And whatever the human wrote alongside their Answer, or empty where they
    /// wrote nothing — which is the ordinary way of agreeing with the
    /// recommendation.
    pub said: String,
}

/// The findings this Conversation's review was told to fix and nothing has
/// landed, in the order the review raised them.
///
/// Empty is the ordinary answer, and it covers every way there is nothing owed:
/// no review has asked, the Set is still waiting on the human, they declined
/// every finding, or the session that was going to fix them did so. What is left
/// is the one failure this exists for — the decisions were made and the doing did
/// not happen — and the words it hands back are the review's own, which is what
/// a session dispatched to finish the job is told.
///
/// **Landed is a commit after the Answers**, which is as much as anything here
/// can know: what the fixes are is prose the review wrote, and no reading of a
/// branch can say which commit was which finding. So this is a coarse question
/// deliberately — a review whose accepted findings landed one commit and then
/// stopped reads as landed, because a session that got that far is one that was
/// working rather than one that fell over before it started.
///
/// The two stamps are the Response's and the commit Event's, both written by
/// SQLite as this database's `now`, so comparing them as text is comparing the
/// instants they name.
pub async fn unlanded_fixes(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Fixing>> {
    let Some(set_id) = review_asked(pool, conversation_id).await? else {
        return Ok(Vec::new());
    };

    unlanded_on(pool, conversation_id, set_id).await
}

/// The same question of the newest proposal instead: what a batch session was
/// told to fix and nothing has landed.
///
/// Which is the review's own Set until a batch has been answered, and that
/// batch's from then on — see [`last_proposal`]. Asking it before any batch has
/// asked anything is safe rather than wrong: the review settles only once
/// nothing it was told to fix is owed, and no batch session is dispatched until
/// it has.
pub async fn unlanded_batch_fixes(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Fixing>> {
    let Some(set_id) = last_proposal(pool, conversation_id).await? else {
        return Ok(Vec::new());
    };

    unlanded_on(pool, conversation_id, set_id).await
}

/// What is owed on one proposal, which is the whole of what either of the two
/// above is.
async fn unlanded_on(pool: &SqlitePool, conversation_id: i64, set_id: i64) -> Result<Vec<Fixing>> {
    let Some(stored) = super::load_set(pool, set_id).await? else {
        return Ok(Vec::new());
    };

    // A Set this build cannot read carries no findings anybody here could act
    // on, and passing over it is the whole of what there is to do about one —
    // see [`super::Asked`].
    let Some(review) = stored.set.set().and_then(|set| set.review.as_ref()) else {
        return Ok(Vec::new());
    };

    let Some(answered) = super::load_response(pool, set_id).await? else {
        return Ok(Vec::new());
    };

    let fixing = decided_as(review, &answered.response, Decided::Fix);

    if fixing.is_empty() || landed_since(pool, conversation_id, &answered.submitted_at).await? {
        return Ok(Vec::new());
    }

    Ok(fixing)
}

/// The findings this Conversation's review was told to split out into a backlog
/// of their own, in the order the review raised them.
///
/// The escape hatch's half of [`unlanded_fixes`], and it reads the same record
/// the other way: a finding the human answered with the Option it named as
/// *split it out* is work for a session of its own rather than work for the
/// session that asked. Empty is the ordinary answer — a review that offered no
/// split at all, one still waiting on the human, one whose splits were declined.
///
/// **Nothing here asks whether it landed**, unlike [`unlanded_fixes`]. What says
/// a split has been carried out is a `.tasks/` backlog on the branch, and that is
/// a question about the Worktree rather than about the record — so this says what
/// was split out and its caller says whether the backlog is there.
pub async fn split_out(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Fixing>> {
    let Some(set_id) = review_asked(pool, conversation_id).await? else {
        return Ok(Vec::new());
    };

    let Some(stored) = super::load_set(pool, set_id).await? else {
        return Ok(Vec::new());
    };

    // Passed over where this build cannot read the body, for [`unlanded_on`]'s
    // reason.
    let Some(review) = stored.set.set().and_then(|set| set.review.as_ref()) else {
        return Ok(Vec::new());
    };

    let Some(answered) = super::load_response(pool, set_id).await? else {
        return Ok(Vec::new());
    };

    Ok(decided_as(review, &answered.response, Decided::Split))
}

/// The findings a Response decided one way, as the session that acts on them is
/// told about them.
///
/// One reading for both outcomes, because they are the same question asked of
/// different Options — and what the human wrote beside their Answer travels with
/// the finding either way, the schema following the pick to find it.
fn decided_as(review: &Review, response: &Response, decided: Decided) -> Vec<Fixing> {
    review
        .findings
        .iter()
        .filter(|finding| finding.decided(response) == decided)
        .map(|finding| Fixing {
            what: finding.what.trim().to_owned(),
            said: finding.said(response).to_owned(),
        })
        .collect()
}

/// Whether anything has been committed on this Conversation's branch since
/// `submitted_at`.
///
/// The commits on the Timeline rather than the branch itself, for the reason
/// every other reader of them asks the store: the branch is swept while the
/// session runs and what it finds lands here, so this is where a fresh commit
/// shows up — and asking it costs one small read where asking git costs a
/// process.
async fn landed_since(pool: &SqlitePool, conversation_id: i64, submitted_at: &str) -> Result<bool> {
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT c.event_id
         FROM commits c
         JOIN timeline_events e ON e.id = c.event_id
         WHERE c.conversation_id = ? AND e.at > ?
         LIMIT 1",
    )
    .bind(conversation_id)
    .bind(submitted_at)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("looking for what Conversation {conversation_id} committed since {submitted_at}")
    })?;

    Ok(found.is_some())
}

/// A Question Set of this Conversation's that arrived after `event_id` and is
/// still waiting to be answered, or `None` where none is.
///
/// Unanswered *and* unarchived: a Set the human closed without answering is one
/// nothing is coming for, so it is settled as much as an answered one is.
///
/// The Event id is what makes it *whose* Set. Nothing else on the record says
/// which session asked one, and nothing has to: one Worktree holds one agent, so
/// every Set that landed after a session's own Event is that session's. What
/// asks is the driver of a Manual Task — a session idling on a Blocking Ask
/// prints nothing for hours, and quiet alone would reap it mid-question.
///
/// Blocking Asks alone, for that same reason read the other way: a Deferred Ask
/// idles nobody, so a session that has gone quiet behind one has finished rather
/// than being mid-question, and a driver that waited on it would wait for as
/// long as the human took to answer something nothing was waiting for.
pub async fn unanswered_set_since(
    pool: &SqlitePool,
    conversation_id: i64,
    event_id: i64,
) -> Result<Option<i64>> {
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT q.id
         FROM question_sets q
         JOIN set_events s ON s.set_id = q.id
         JOIN timeline_events e ON e.id = s.event_id
         LEFT JOIN responses r ON r.set_id = q.id
         LEFT JOIN archivings a ON a.set_id = q.id
         LEFT JOIN deferrals d ON d.set_id = q.id
         WHERE e.conversation_id = ? AND e.id > ?
           AND r.set_id IS NULL AND a.set_id IS NULL AND d.set_id IS NULL
         ORDER BY q.id
         LIMIT 1",
    )
    .bind(conversation_id)
    .bind(event_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("looking for an unanswered Question Set of Conversation {conversation_id}'s")
    })?;

    Ok(found.map(|(set_id,)| set_id))
}

/// Which Conversation a Set was asked from, or `None` if it is on no Timeline
/// at all.
///
/// The other direction of [`set_asked_from`], and the one a Set opened by its
/// own id needs: a page reached from a push notification knows the Set and
/// nothing else, and where it leads back to is the Conversation it belongs to.
///
/// `None` cannot happen for a stored Set — [`ask`] writes the Set, its Event and
/// the row joining them in one transaction — so it is a broken record rather
/// than a Set that simply has no Conversation.
pub async fn asked_from(pool: &SqlitePool, set_id: i64) -> Result<Option<i64>> {
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT e.conversation_id
         FROM set_events s
         JOIN timeline_events e ON e.id = s.event_id
         WHERE s.set_id = ?",
    )
    .bind(set_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("looking for the Conversation Question Set {set_id} was asked from")
    })?;

    Ok(found.map(|(id,)| id))
}

/// Rewrite the Brief of the round a drafting Conversation is in.
///
/// The Brief Event is edited in place rather than added to, and it is the
/// *newest* of them: the frozen-Brief rule the design states — a reopened round
/// adds a new Brief rather than editing the old one — makes every Brief but the
/// last one a record of a round that has been built, and the drafting guard is
/// what keeps this to the round nobody has grilled yet. A Conversation on its
/// first round has one Brief, and the newest is it.
pub async fn save_brief(pool: &SqlitePool, id: i64, markdown: &str) -> Result<Edited> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(refusal);
    }

    sqlx::query(
        "UPDATE timeline_events SET body = ?
         WHERE id = (
             SELECT id FROM timeline_events
             WHERE conversation_id = ? AND kind = ?
             ORDER BY id DESC LIMIT 1
         )",
    )
    .bind(markdown)
    .bind(id)
    .bind(Event::Brief(String::new()).kind())
    .execute(pool)
    .await
    .with_context(|| format!("saving the Brief of Conversation {id}"))?;

    Ok(Edited::Saved)
}

/// Name the branch a drafting Conversation's work will be done on.
///
/// Whether the name is one git would take is decided above the store, where git
/// itself is asked — this records what it is given.
///
/// Refused once the branch has been made, which drafting alone no longer says:
/// a reopened round is drafting again on a branch that has been worked — see
/// [`branch_made`].
pub async fn rename_branch(pool: &SqlitePool, id: i64, branch: &str) -> Result<Edited> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(refusal);
    }

    if branch_made(pool, id).await? {
        return Ok(Edited::NotDrafting);
    }

    sqlx::query("UPDATE conversations SET branch = ? WHERE id = ?")
        .bind(branch)
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("renaming the branch of Conversation {id}"))?;

    Ok(Edited::Saved)
}

/// Record the commit a drafting Conversation branches from, or `None` to put it
/// back on the default-branch rule.
///
/// `None` is the ordinary case and not a cleared field: the design says the base
/// commit is the default branch's tip *at grill start*, so while drafting there
/// is no value to hold — only whether the human has overridden the rule.
///
/// Refused once the branch has been made, for [`rename_branch`]'s reason: the rule
/// resolved to a commit when the work branched, and a reopened round carries on
/// from what was built rather than branching again.
pub async fn set_base_commit(pool: &SqlitePool, id: i64, commit: Option<&str>) -> Result<Edited> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(refusal);
    }

    if branch_made(pool, id).await? {
        return Ok(Edited::NotDrafting);
    }

    sqlx::query("UPDATE conversations SET base_commit = ? WHERE id = ?")
        .bind(commit)
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("recording the base commit of Conversation {id}"))?;

    Ok(Edited::Saved)
}

/// The reason this Conversation is not the human's to edit, or `None` where it
/// is.
///
/// Read before the write rather than guarded inside it, unlike the Set tables:
/// there is one human at the workbench, and what would be raced for here is
/// their own two tabs editing one Brief. What matters is that a Conversation
/// past drafting refuses, and that a Conversation that is not there says so.
async fn not_drafting(pool: &SqlitePool, id: i64) -> Result<Option<Edited>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Some(Edited::NoSuchConversation));
    };

    Ok(match Lifecycle::read(&state)? {
        Lifecycle::Draft => None,
        _ => Some(Edited::NotDrafting),
    })
}

/// Whether this Conversation's branch has been made already.
///
/// The worktree row is what says so: it is written when the branch and the
/// checkout are made, and forgotten only by closing. Drafting used to answer
/// this on its own — a Conversation was drafting exactly until its branch
/// existed — and a reopened round is the case that separated the two: it is
/// drafting a second Brief on a branch that has already been worked.
///
/// Which is why the branch name and the base commit are refused off this as
/// well as off the state. The Brief is not: writing the new round's Brief is
/// the whole of what reopening is for.
async fn branch_made(pool: &SqlitePool, id: i64) -> Result<bool> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT conversation_id FROM worktrees WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking for the worktree of Conversation {id}"))?;

    Ok(row.is_some())
}

/// Record that a Conversation has started grilling: what it branched from, where
/// its worktree is, and that it has moved.
///
/// The branch and the worktree are already made by the time this is called —
/// the server does that, against git — so what is written here is the record of
/// work that has happened, not an instruction to do any. Which is also why it is
/// one transaction: a Conversation left saying `draft` with a worktree on disk
/// would be one nothing could start again and nothing would clean up.
///
/// `base_commit` is written whether or not the human overrode one. Where they
/// did not, the rule was the default branch's tip *at grill start* — so this is
/// the moment that rule resolves to a commit, and after it there is a fact about
/// what the work branched from rather than a rule about what it would have.
///
/// It is also where the Repo remembers what it was grilled with — see
/// [`super::pairings::remember`] — because this is the moment the two Pairings
/// stop being changeable and become what the work is actually running under.
pub async fn start_grilling(
    pool: &SqlitePool,
    id: i64,
    base_commit: &str,
    worktree: &Path,
) -> Result<Grilling> {
    let worktree = super::repos::text(worktree)?;

    let mut tx = pool.begin().await.context("starting a grilling")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Grilling::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Draft {
        return Ok(Grilling::NotDrafting);
    }

    sqlx::query(
        "UPDATE conversations
         SET base_commit = ?, state = ?
         WHERE id = ?",
    )
    .bind(base_commit)
    .bind(Lifecycle::Grilling.stored())
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("moving Conversation {id} to grilling"))?;

    // Written over whatever is there, because a second round has one already: a
    // reopened Conversation is drafting on the checkout it was reopened with, and
    // this writes the same path back — see [`reopen_conversation`].
    sqlx::query(
        "INSERT INTO worktrees (conversation_id, path) VALUES (?, ?)
         ON CONFLICT(conversation_id) DO UPDATE SET path = excluded.path",
    )
    .bind(id)
    .bind(worktree)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the worktree of Conversation {id}"))?;

    moved(&mut tx, id, Lifecycle::Grilling).await?;

    // And what it is being grilled with, against its Repo, so the next
    // Conversation started on that Repo arrives with both pickers filled. In
    // this transaction because this is the moment the Pairings are fixed: a
    // memory written a moment later could be of a choice that never ran.
    super::pairings::remember(&mut tx, id).await?;

    tx.commit().await.context("starting a grilling")?;

    Ok(Grilling::Started)
}

/// Record that a Conversation has been closed: its worktree is gone, and it has
/// stopped wherever it had got to.
///
/// The worktree is forgotten rather than remembered as removed, because there is
/// nothing left to point at — the branch it was checked out on is still there,
/// and that is the thing worth keeping. The directory itself is removed by the
/// server before this is called, for the reason the branch is created before
/// [`start_grilling`] is: the record follows the work rather than promising it.
///
/// Closing one that is closed already records nothing and is not an error. The
/// human asked for it to be closed, and it is.
pub async fn close_conversation(pool: &SqlitePool, id: i64) -> Result<Closing> {
    let mut tx = pool.begin().await.context("closing a Conversation")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Closing::NoSuchConversation);
    };

    if Lifecycle::read(&state)? == Lifecycle::Closed {
        return Ok(Closing::AlreadyClosed);
    }

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Closed.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("closing Conversation {id}"))?;

    sqlx::query("DELETE FROM worktrees WHERE conversation_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("forgetting the worktree of Conversation {id}"))?;

    moved(&mut tx, id, Lifecycle::Closed).await?;

    tx.commit().await.context("closing a Conversation")?;

    Ok(Closing::Closed)
}

/// Record that a finished Conversation has been reopened: it is drafting a
/// second Brief, on the branch the first round was built on.
///
/// Done is the one state this is reachable from. Closed is off the ladder and
/// stays there, and every other state is somewhere the work has got to — there
/// is nothing to reopen about work that is still going on.
///
/// **The frozen Brief is left exactly where it is and a new one is added.** That
/// is the whole rule the design states: the first Brief is what the first round
/// was built from, and a Timeline that had lost it would have lost why the work
/// is the shape it is. What [`save_brief`] writes from here on is the new one.
///
/// The move is written first and the Brief after it, which is reading order: the
/// move is where the round boundary falls, and the Brief under it belongs to the
/// round that starts there.
///
/// The worktree is recorded rather than made, as [`start_grilling`] records one:
/// the directory is the server's to keep or to check out again before this is
/// called. Written over whatever was there, because a Conversation that had one
/// keeps it and one whose directory had gone gets it back in the same place.
///
/// One transaction, for [`start_grilling`]'s reason: a Conversation left saying
/// `done` with a second Brief on its Timeline would be one nothing could grill
/// and nothing would tidy.
pub async fn reopen_conversation(pool: &SqlitePool, id: i64, worktree: &Path) -> Result<Reopening> {
    let worktree = super::repos::text(worktree)?;

    let mut tx = pool.begin().await.context("reopening a Conversation")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Reopening::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Done {
        return Ok(Reopening::NotDone);
    }

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Draft.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("moving Conversation {id} back to drafting"))?;

    sqlx::query(
        "INSERT INTO worktrees (conversation_id, path) VALUES (?, ?)
         ON CONFLICT(conversation_id) DO UPDATE SET path = excluded.path",
    )
    .bind(id)
    .bind(worktree)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the worktree of Conversation {id}"))?;

    // The round before this one is over and its bookkeeping goes with it, so
    // that the wrap-up of the round starting here waits on the same things from
    // nothing — see [`super::wrap_up::forget_the_round`].
    super::wrap_up::forget_the_round(&mut tx, id).await?;

    moved(&mut tx, id, Lifecycle::Draft).await?;

    // Empty, because the new round has not been written yet — exactly as the
    // first Brief is empty from the moment there is a Conversation. It is a
    // second Event and never an edit of the first: what the first round was
    // built from stays on the record beside it.
    let brief = Event::Brief(String::new());
    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
    )
    .bind(id)
    .bind(brief.kind())
    .bind(brief.body().into_owned())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("writing the new round's Brief of Conversation {id}"))?;

    tx.commit().await.context("reopening a Conversation")?;

    Ok(Reopening::Reopened)
}

/// Record the direction the human picked on a wrap-up proposal: it is the
/// Conversation's latest pick, and nothing else.
///
/// **A pick moves nothing, whichever one it is.** The grilling session that
/// proposed is the one that produces what the pick asked for — the backlog, the
/// roadmap, or the handoff an inline build is primed from — so the Conversation
/// stays Grilling until that session has: the pick informs the agent and the
/// artifact moves the machine. What follows a backlog or a handoff is
/// [`start_implementing`]; what follows the roadmap commit is the pull request
/// the same session opens, which is [`super::record_pull_request`]'s to record.
///
/// Called off the back of a Response landing rather than off anything the human
/// pressed — see [`super::submit_response`] — which is why a Conversation that is
/// not grilling is [`Directing::NotGrilling`] rather than an error. A pick that
/// arrives after the grilling has ended has nothing left to inform.
///
/// The pick is recorded as the row alone. What says on the Timeline that the
/// human chose is the answered proposal Set sitting on it, with the direction
/// on its Response — a second Event beside it would be a second record of one
/// decision.
///
/// One transaction, though there is only the one row to write: what a later pick
/// overwrites is the row a watcher is armed from, and a restart reads back.
pub async fn pick_direction(pool: &SqlitePool, id: i64, direction: Direction) -> Result<Directing> {
    let mut tx = pool.begin().await.context("acting on a picked direction")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Directing::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Grilling {
        return Ok(Directing::NotGrilling);
    }

    sqlx::query(
        "INSERT INTO directions (conversation_id, direction) VALUES (?, ?)
         ON CONFLICT (conversation_id) DO UPDATE SET direction = excluded.direction",
    )
    .bind(id)
    .bind(direction_stored(direction))
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the direction picked on Conversation {id}"))?;

    // The grilling carries on writing the artifact, so there is nothing to move.
    // The pick is committed for itself: it is what the follower watching for that
    // artifact is armed from, and what a restart reads back.
    tx.commit().await.context("acting on a picked direction")?;

    Ok(Directing::Writing)
}

/// Record that a grilling's own tail has landed and the work is being built: the
/// Conversation is implementing, and the move is on its Timeline.
///
/// What the grilling session wrote is already in hand by the time this runs —
/// the plan commit, or the handoff document taken onto the Timeline — so this is
/// the record catching up with what the session left behind rather than a
/// decision of its own.
///
/// Refused for anything but Grilling, which is the only place a tail can be
/// running. That refusal is what keeps a run that was closed, or one whose pick
/// was superseded, from moving a Conversation that has gone somewhere else since.
///
/// One transaction, as every move is: a Conversation that says Implementing
/// always has the move on its Timeline to say when it got there.
pub async fn start_implementing(pool: &SqlitePool, id: i64) -> Result<Implementing> {
    let mut tx = pool.begin().await.context("starting the implementation")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Implementing::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Grilling {
        return Ok(Implementing::NotGrilling);
    }

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Implementing.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("moving Conversation {id} to implementing"))?;

    moved(&mut tx, id, Lifecycle::Implementing).await?;

    tx.commit().await.context("starting the implementation")?;

    Ok(Implementing::Started)
}

/// Send a wrapping Conversation back to be built, because its review split work
/// out into a backlog.
///
/// The one move down the ladder there is. A review that judged its findings too
/// big to fix where it stood wrote them as `.tasks/`, and a backlog is something
/// to work a session at a time — so the Conversation goes back to Implementing
/// and comes round to Wrapping again through its finish step, which is the
/// second wrap.
///
/// Refused for anything but Wrapping, for the reason every other move is refused
/// outside the state it leaves: a Conversation closed out from under the session
/// that wrote the backlog is not one to start building.
///
/// **The review's settle goes with it**, in the same transaction. *Settled once
/// and stays settled* is a rule about one wrap rather than about the
/// Conversation — see [`super::WaitingOn::Review`] — and a settle left standing
/// would be a second wrap that reached Done having read none of what the backlog
/// built. The checks and the comments need no such thing: both are asked of
/// GitHub on every poll, so they settle from the answers the second wrap gets.
///
/// One transaction, as every move is: a Conversation that says Implementing
/// always has the move on its Timeline to say when it got there.
pub async fn implement_again(pool: &SqlitePool, id: i64) -> Result<Rebuilding> {
    let mut tx = pool.begin().await.context("building the split-out work")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Rebuilding::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Wrapping {
        return Ok(Rebuilding::NotWrapping);
    }

    super::wrap_up::unsettle(&mut tx, id, super::WaitingOn::Review).await?;

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Implementing.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("moving Conversation {id} back to implementing"))?;

    moved(&mut tx, id, Lifecycle::Implementing).await?;

    tx.commit().await.context("building the split-out work")?;

    Ok(Rebuilding::Started)
}

/// Steer a Conversation into `target`: the human's own Event, the state, and
/// the move that says it got there.
///
/// The one move with no state it is refused from. Every other call here answers
/// to a rung of the ladder — grilling starts from a draft, a rebuild leaves a
/// wrap-up — because each of them is the pipeline moving the work along its own
/// path. A steer is the human stepping outside that path, so the state it finds
/// is not something to be right or wrong: a draft, a run in flight and a
/// Conversation Verkstead has finished with are all somewhere to be steered
/// from.
///
/// Two Events, in the order the moment happened in. The Steer goes first because
/// it is the act — somebody decided this — and the Moved line follows it because
/// it is what came of the act, which is the same order a Manual Task's
/// instruction stands in above what its session went on to do.
///
/// **The Steer carries what the human wrote**, where a target takes anything
/// written: the instruction a steer into Implementing sends a session off with
/// is the Event's own body, so reading the Event back is reading the job that
/// was set. See [`Event::Steer`] for how the two are held in the one column.
///
/// **A third where the steer opens a round**: the Brief the human wrote for it,
/// under the move rather than above it, because the move is where the round
/// boundary falls and the Brief belongs to the round that starts there — which
/// is the order [`reopen_conversation`] writes those two in. Frozen where it
/// lands, the round it opens being past drafting, and a second Brief Event
/// beside the first rather than an edit of it: what the earlier round was built
/// from stays on the record.
///
/// One transaction, as every move is: a Conversation that says Done always has
/// the move on its Timeline to say when it got there, and one steered always has
/// the human's own line above it.
///
/// **And the Pairing the human picked, where they picked one.** In the same
/// transaction as the move, because it is the same act: steering re-settles what
/// runs the work rather than picking for one session, and a Conversation that
/// moved into a state something runs in without the Pairing that state's
/// sessions run under would be a move only half made. Past drafting, which is
/// the whole of why this does not go through [`set_implementation_pairing`] —
/// see [`settle`].
///
/// `None` is the ordinary case twice over: a target nothing runs in has no
/// Pairing to settle, and a human who left the picker on what the Conversation
/// already had has changed none.
///
/// **And how the work is being built, where nothing said yet.** Written only
/// over a Conversation with no direction on it: a state something runs in with
/// nothing saying how the work is built is a record a pressed Resume refuses on
/// by name, and a steer that set a session going in one would leave the
/// Conversation unable to be started again. What is already picked is left
/// exactly as it is.
///
/// **And what the steer had to make before it could move anything**, which is
/// the Worktree it is to run in and — for a Draft, which has never had one — the
/// commit its branch was cut from. Recorded here rather than made here: git and
/// the filesystem are the server's to reach, and what this writes is the record
/// of work that has already happened. See [`Steer`].
///
/// Nothing about the run is touched, and what has to stop running is stopped
/// before this is called — see the server's `steering` module, which is the only
/// caller.
pub async fn steer_conversation(pool: &SqlitePool, id: i64, steer: Steer<'_>) -> Result<Steering> {
    let Steer {
        target,
        pairing,
        brief,
        instruction,
        direction,
        worktree,
        base_commit,
    } = steer;

    let mut tx = pool.begin().await.context("steering a Conversation")?;

    let steer = Event::Steer(target, instruction.map(str::to_owned));

    // Selected from `conversations` rather than trusting the id, as every other
    // Event is written: a steer attributed to a Conversation that is not there
    // would be on nobody's Timeline — and this is also what says whether there
    // is anything here to move at all.
    let landed = sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?
         FROM conversations WHERE id = ?",
    )
    .bind(steer.kind())
    .bind(steer.body().into_owned())
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("putting a steer on the Timeline of Conversation {id}"))?
    .rows_affected();

    if landed == 0 {
        return Ok(Steering::NoSuchConversation);
    }

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(target.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("steering Conversation {id} into {}", target.stored()))?;

    // What the branch was cut from, for the one source that had no branch: a
    // Draft's column holds the base the human picked while drafting, which is a
    // *branch* until the moment something resolves it. This is that moment, as
    // [`start_grilling`] is for a Conversation that reached grilling the
    // ordinary way — and after it there is a fact about what the work branched
    // from rather than a rule about what it would have.
    if let Some(base_commit) = base_commit {
        sqlx::query("UPDATE conversations SET base_commit = ? WHERE id = ?")
            .bind(base_commit)
            .bind(id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("recording what Conversation {id} branched from"))?;
    }

    // And where the work goes on, written over whatever was there: a
    // Conversation that kept its directory is written the same path back, and
    // one that never had it — a Draft, or a closed Conversation whose Worktree
    // was deleted — is written the one the steer has just checked out.
    if let Some(worktree) = worktree {
        sqlx::query(
            "INSERT INTO worktrees (conversation_id, path) VALUES (?, ?)
             ON CONFLICT(conversation_id) DO UPDATE SET path = excluded.path",
        )
        .bind(id)
        .bind(super::repos::text(worktree)?)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording the worktree of Conversation {id}"))?;
    }

    // And how the work is built from here, for a Conversation that has never
    // said. `DO NOTHING` rather than an upsert, which is what makes the rule the
    // record's rather than the caller's: a direction already picked is the
    // human's own answer to how their work is built, and a steer that wrote over
    // it would be deciding that for them. What this is for is the Conversation
    // that reaches Implementing without ever having been grilled — a steered
    // draft, or one whose work is a hand-written instruction and nothing else —
    // because a state something runs in with nothing saying how is a record
    // Resume refuses on by name.
    if let Some(direction) = direction {
        sqlx::query(
            "INSERT INTO directions (conversation_id, direction) VALUES (?, ?)
             ON CONFLICT (conversation_id) DO NOTHING",
        )
        .bind(id)
        .bind(direction_stored(direction))
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording how Conversation {id} is being built"))?;
    }

    // A steer into Grilling opens a round, so the round before it is over and
    // its wrap-up bookkeeping goes with it — the same forgetting a reopened
    // Conversation does, for the same reason: a round that inherited the one
    // before it would reach Wrapping with everything wrap-up waits on already
    // settled, and would be over the moment it arrived. See
    // [`super::wrap_up::forget_the_round`].
    if target == Lifecycle::Grilling {
        super::wrap_up::forget_the_round(&mut tx, id).await?;
    }

    moved(&mut tx, id, target).await?;

    // The new round's Brief under the move, which is the order
    // [`reopen_conversation`] writes the two in and for its reason: the move is
    // where the round boundary falls, and the Brief under it belongs to the
    // round that starts there. Frozen from the moment it lands — the round it
    // opens is past drafting, which is the only state [`save_brief`] will edit
    // one in — and a second Brief Event beside the first rather than an edit of
    // it.
    if let Some(brief) = brief {
        let event = Event::Brief(brief.to_owned());

        sqlx::query(
            "INSERT INTO timeline_events (conversation_id, at, kind, body)
             VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
        )
        .bind(id)
        .bind(event.kind())
        .bind(event.body().into_owned())
        .execute(&mut *tx)
        .await
        .with_context(|| format!("writing the steered round's Brief of Conversation {id}"))?;
    }

    if let Some(pairing) = pairing
        && !settle(
            &mut tx,
            id,
            pairing.role,
            pairing.profile_id,
            Some(pairing.model),
        )
        .await?
    {
        return Ok(Steering::NoSuchProfile);
    }

    tx.commit().await.context("steering a Conversation")?;

    Ok(Steering::Steered)
}

/// Everything one steer writes: where it goes, and whatever the human's press
/// settled or the server had to make on the way.
///
/// One struct rather than a parameter list, because all of it is one act. A
/// steer moves the Conversation, and the Brief the human wrote, the Pairing they
/// picked and the Worktree that had to exist before any of it could run are
/// parts of that move rather than things done beside it — a Conversation left
/// wrapping under a Pairing that was not written, or grilling a round whose
/// Brief did not land, would be a move only half made.
///
/// Everything but the target is `None` in the ordinary case, and each `None`
/// says something different: no Brief is a steer into a round that starts on the
/// Brief already there, no instruction is a steer that carries on what the
/// branch already holds, no Pairing is a picker left on what the Conversation
/// already had, no direction is a Conversation that has already said how its
/// work is built, and no Worktree or base commit is a target nothing runs in or
/// a Conversation that had both already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Steer<'a> {
    /// Which state the human moved it into.
    pub target: Lifecycle,

    /// What the work runs under from here, where they picked something new.
    pub pairing: Option<Settling<'a>>,

    /// The new round's Brief, for a steer that opens one.
    pub brief: Option<&'a str>,

    /// The hand-written work a steer into Implementing carries, which lands as
    /// the Steer Event's own body rather than beside it.
    ///
    /// Not a Brief and not a Manual Task's instruction, however alike the three
    /// look on the page. A Brief is what a round is grilled *about*; this is
    /// one session's whole job, said by the human at the moment they steered —
    /// so it belongs to the steer, and reading the Event back is reading what
    /// they asked for.
    pub instruction: Option<&'a str>,

    /// How the work is being built from here, for a Conversation that has never
    /// said.
    ///
    /// Written only where there is nothing to overwrite — see
    /// [`steer_conversation`], which leaves a direction already picked exactly
    /// as it found it. What says how the work is built is the human's own pick,
    /// and a steer that rewrote one would be answering a question they had
    /// already answered.
    pub direction: Option<Direction>,

    /// Where the work goes on, for a target something runs in.
    pub worktree: Option<&'a Path>,

    /// And what its branch was cut from, where the steer is what cut it.
    pub base_commit: Option<&'a str>,
}

/// A Pairing a steer settles: which of the two roles, and both halves of the
/// choice.
///
/// The model borrowed rather than owned, this being read straight off what the
/// modal submitted and living exactly as long as the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settling<'a> {
    /// Which role's Pairing is being re-settled: the one the state steered into
    /// runs its sessions under.
    pub role: Role,

    pub profile_id: i64,

    /// One of that Profile's models. Never absent: there is no default model
    /// anywhere, so a Pairing is picked whole or not at all.
    pub model: &'a str,
}

/// Put the handoff document the grilling wrote on a Conversation's Timeline.
///
/// `false` means there is no such Conversation, by the same insert-from-select
/// every other Event is written with: a handoff attributed to a Conversation
/// that is not there would be a document on nobody's Timeline.
///
/// Written whole, every time it is called. A Conversation gets one of these per
/// grilling round rather than one ever — a reopened round grills again, and its
/// handoff is a second Event beside the first rather than a rewrite of it.
pub async fn record_handoff(pool: &SqlitePool, id: i64, markdown: &str) -> Result<bool> {
    let event = Event::Handoff(markdown.to_owned());

    let written = sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?
         FROM conversations WHERE id = ?",
    )
    .bind(event.kind())
    .bind(event.body().into_owned())
    .bind(id)
    .execute(pool)
    .await
    .with_context(|| format!("putting the handoff of Conversation {id} on its Timeline"))?
    .rows_affected();

    Ok(written > 0)
}

/// Put something Verkstead has to say on a Conversation's Timeline.
///
/// `false` means there is no such Conversation, by the same insert-from-select
/// every other Event is written with.
///
/// Written whenever there is something to say and never replacing what was said
/// before: a notice is a record of a decision at the moment it was taken, and a
/// Timeline that rewrote yesterday's would be one the human could not read back.
pub async fn note(pool: &SqlitePool, id: i64, markdown: &str) -> Result<bool> {
    let event = Event::Notice(markdown.to_owned());

    let written = sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?
         FROM conversations WHERE id = ?",
    )
    .bind(event.kind())
    .bind(event.body().into_owned())
    .bind(id)
    .execute(pool)
    .await
    .with_context(|| format!("putting a notice on the Timeline of Conversation {id}"))?
    .rows_affected();

    Ok(written > 0)
}

/// Put a Manual Task's instruction on a Conversation's Timeline.
///
/// `false` means there is no such Conversation, by the same insert-from-select
/// every other Event is written with.
///
/// The record of what was asked for by hand, written as it is asked and never
/// rewritten: a Manual Task is a moment on the Timeline like the rest of them,
/// and what its session goes on to do lands beside it as its own Events.
pub async fn record_manual_task(pool: &SqlitePool, id: i64, instruction: &str) -> Result<bool> {
    let event = Event::ManualTask(instruction.to_owned());

    let written = sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?
         FROM conversations WHERE id = ?",
    )
    .bind(event.kind())
    .bind(event.body().into_owned())
    .bind(id)
    .execute(pool)
    .await
    .with_context(|| format!("putting a manual task on the Timeline of Conversation {id}"))?
    .rows_affected();

    Ok(written > 0)
}

/// Start a roadmap stage's Conversation working: its base commit, its worktree,
/// the direction its work takes and the move that says it is under way, all at
/// once.
///
/// The one Conversation nobody starts. Every other one is drafted, grilled and
/// directed by a human at the workbench; a stage is started by the stage before
/// it settling, and what would have been settled by a grilling was settled by the
/// grilling that wrote the roadmap — the stage brief is what it settled, and it
/// arrives as the Brief. So this goes straight to Implementing: there is nothing
/// to grill and nobody to choose.
///
/// The direction is recorded with it, and it is a task list because that is the
/// pipeline a stage runs: the fork it starts in writes `.tasks/`, and the runner
/// works the backlog from there. Recorded rather than left empty, so that a
/// Conversation implementing something always says how — though nobody picked
/// it, because there was no proposal here to pick one on.
///
/// One transaction, as every move is, and the branch and the worktree exist
/// before it for the reason they do at [`start_grilling`]: the record follows the
/// work rather than promising it.
///
/// `stacks_on` is the branch this stage's branch was made on top of, and `None`
/// where it came off the default branch. Written here because this is the
/// transaction that makes the Conversation a stage, and read back by whatever
/// starts a session in it — the first one, and any the human asks for again.
pub async fn start_stage(
    pool: &SqlitePool,
    id: i64,
    base_commit: &str,
    worktree: &Path,
    stacks_on: Option<&str>,
) -> Result<Staged> {
    let worktree = super::repos::text(worktree)?;

    let mut tx = pool.begin().await.context("starting a stage")?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Staged::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Draft {
        return Ok(Staged::NotDrafting);
    }

    sqlx::query("UPDATE conversations SET base_commit = ?, state = ? WHERE id = ?")
        .bind(base_commit)
        .bind(Lifecycle::Implementing.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("moving Conversation {id} to implementing a stage"))?;

    sqlx::query("INSERT INTO worktrees (conversation_id, path) VALUES (?, ?)")
        .bind(id)
        .bind(worktree)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording the worktree of Conversation {id}"))?;

    sqlx::query(
        "INSERT INTO directions (conversation_id, direction) VALUES (?, ?)
         ON CONFLICT (conversation_id) DO UPDATE SET direction = excluded.direction",
    )
    .bind(id)
    .bind(direction_stored(Direction::TaskList))
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the direction of Conversation {id}"))?;

    sqlx::query("INSERT INTO stage_branches (conversation_id, stacks_on) VALUES (?, ?)")
        .bind(id)
        .bind(stacks_on)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording what the branch of Conversation {id} stands on"))?;

    moved(&mut tx, id, Lifecycle::Implementing).await?;

    tx.commit().await.context("starting a stage")?;

    Ok(Staged::Started)
}

/// What a stage Conversation's branch was made on top of, where it is a stage
/// at all.
///
/// Two layers of `Option` and both mean something: the outer one is *this is
/// not a stage Conversation*, and the inner is *it is, and its branch came off
/// the default branch rather than standing on another stage*.
pub async fn stacks_on(pool: &SqlitePool, id: i64) -> Result<Option<Option<String>>> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT stacks_on FROM stage_branches WHERE conversation_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading what the branch of Conversation {id} stands on"))?;

    Ok(row.map(|(stacks_on,)| stacks_on))
}

/// Put a move on a Conversation's Timeline.
///
/// Shared with [`super::pull_requests`], which moves a Conversation into
/// Wrapping in the same transaction that records what it is wrapping up: a move
/// is a move whichever module has the reason for it.
pub(crate) async fn moved(
    tx: &mut sqlx::SqliteConnection,
    id: i64,
    state: Lifecycle,
) -> Result<()> {
    let event = Event::Moved(state);

    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
    )
    .bind(id)
    .bind(event.kind())
    .bind(event.body().into_owned())
    .execute(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "recording that Conversation {id} moved to {}",
            state.stored()
        )
    })?;

    Ok(())
}

/// Move a Conversation on to another state.
///
/// The blunt instrument, for the states no stage has arrived at yet. Starting to
/// grill and closing have their own calls, because each of them is a move plus
/// everything else that has to be true at the same moment.
pub async fn set_state(pool: &SqlitePool, id: i64, state: Lifecycle) -> Result<()> {
    let changed = sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(state.stored())
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("moving Conversation {id} to {}", state.stored()))?
        .rows_affected();

    if changed == 0 {
        return Err(anyhow!("there is no Conversation {id} to move"));
    }

    Ok(())
}
