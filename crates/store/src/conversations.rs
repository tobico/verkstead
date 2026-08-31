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
//! Timeline rather than in it would have to be moved into it later, and a
//! steered round adds a second Brief Event rather than editing the first —
//! which a column could not hold at all.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use sqlx::SqlitePool;
use verkstead_schema::{Direction, QuestionSet, SetCreated};

use super::wrap_up::{WaitingOn, settled_when};

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
/// [`Lifecycle::Closed`] is off the ladder rather than on it: closing is the
/// work stopping wherever it was, which is why it is reachable from all of them
/// and leads nowhere. [`Lifecycle::FollowUp`] is beside it rather than on it
/// too, being somewhere the human puts a Conversation whose work is already
/// pushed — and it leads back into the wrap-up it came off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    /// The Brief is being written, and with it everything else about the
    /// Conversation: the branch name and the base commit alike. Where every
    /// Conversation starts, and the one state nothing comes back to — a second
    /// round opens where it is steered, past drafting already.
    Draft,

    /// A grilling session is running against it.
    Grilling,

    /// The work is being done.
    Implementing,

    /// The work is on a PR and the wrap-up loop has it.
    Wrapping,

    /// The human is following that pull request up: a session of their own,
    /// asking and doing whatever they want taken up about work already pushed.
    ///
    /// The one state with no way in but a steer, and the one that is not a rung
    /// of the ladder: it hangs off the wrap-up rather than following it, and
    /// where it leads back to is Wrapping.
    FollowUp,

    /// Finished. A steer is the way back in: one into [`Lifecycle::Grilling`]
    /// opens a second round with a Brief of its own — see
    /// [`steer_conversation`].
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
            Self::FollowUp => "follow-up",
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
            "follow-up" => Self::FollowUp,
            "done" => Self::Done,
            "closed" | "aborted" => Self::Closed,
            other => bail!("a Conversation is in the unknown state {other:?}"),
        })
    }

    /// Whether a stored word reads as this state, tolerating one this Verkstead
    /// does not understand.
    ///
    /// The compare for the two presses that have to work on a row whose state
    /// word has gone bad — closing it, and refusing to archive it — where
    /// [`Self::read`]'s error would be the whole conversation locked away
    /// behind the one column nothing can get past. A word that will not parse
    /// is not the state being asked about, which is the answer both of them
    /// want: it is not Closed, so the close goes ahead and heals the row, and
    /// it is not Closed, so an archive on its own says `NotClosed` rather than
    /// putting a possibly-live worktree out of sight.
    ///
    /// Nowhere else. Everything that needs to *know* the state still reads it,
    /// because a reader told Draft about a word nobody can parse would act on a
    /// guess — see [`load_conversation`], which still refuses.
    pub(crate) fn reads_as(word: &str, state: Self) -> bool {
        Self::read(word).is_ok_and(|read| read == state)
    }
}

/// A Conversation as the store holds it, with the Repo it is attached to read
/// back beside it — there is no Conversation without one, and everything done
/// about a Conversation is done inside that repository.
///
/// The Pairings are read back the same way, because whether a Conversation
/// is ready to grill turns on what they are rather than on which ids they hold:
/// a Profile whose pair has gone is not something to launch a session under, and
/// the id alone cannot say so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    pub id: i64,
    pub created_at: String,
    pub repo: super::Repo,

    /// The branch the work will be done on: the name somebody settled on, or
    /// the one prefilled at creation while nobody has.
    pub branch: String,

    /// Whether that name is one somebody settled on rather than the random one
    /// Verkstead prefilled the record with.
    ///
    /// Kept in the record rather than read off the name's shape: a name
    /// Verkstead invented is nothing to show the human, so a draft still
    /// carrying one is drawn as a Draft with an empty branch field. Typing one
    /// settles it, and clearing the field hands it back — the prefill is still
    /// where it was, so it stands again.
    pub branch_named: bool,

    /// Whether the branch is still waiting to be named by the session that was
    /// told to name it.
    ///
    /// Set where the work starts on a name Verkstead invented, because that is
    /// where the first session is told to switch to an appropriate one. It says
    /// nothing about the name itself: what it says is that nobody has settled
    /// for the one the record holds yet, which is why the Conversation goes on
    /// being drawn as a Draft after it has stopped being one.
    ///
    /// Put down two ways and both of them final: the session renames the branch
    /// and the record follows it — see [`follow_branch`] — or the session ends
    /// having left the name alone, and the name it left is the Conversation's.
    /// Always `false` where the human typed a name, there being nothing to wait
    /// for.
    pub naming: bool,

    /// The commit to branch from, where the human named one. `None` is not a
    /// missing value: it is the rule that the default branch's tip at grill
    /// start is what gets used, which is a thing to resolve then rather than a
    /// commit to record now.
    pub base_commit: Option<String>,

    pub state: Lifecycle,

    /// The Profile and model the grilling session runs under, once they are
    /// chosen.
    ///
    /// One of the two roles that can be picked away altogether — see
    /// [`super::Picked`]. A Conversation whose human picked *no grilling* is
    /// never grilled: its Brief goes straight to an inline implementation.
    pub grilling_pairing: super::Picked,

    /// And the ones the implementation runs under. A separate choice because it
    /// is genuinely a separate account and model — and because the
    /// implementation session cannot simply carry the grilling one on.
    pub implementation_pairing: Option<super::Pairing>,

    /// And what the wrap-up's review session runs under. A third choice of its
    /// own for the reason the second is one: reviewing is a fresh set of eyes on
    /// what was built, so the account that looks at the work is picked apart
    /// from the account that built it.
    ///
    /// The other role that can be picked away altogether — see
    /// [`super::Picked`]. A Conversation whose human picked *no review* wraps up
    /// without a review session, which is a settled choice rather than a Pairing
    /// missing.
    pub review_pairing: super::Picked,

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

    /// The other registered Repos this Conversation works alongside, by the
    /// Repo's name — see [`super::companions`].
    ///
    /// Empty is the ordinary Conversation: one repository is what most work
    /// needs. Carried on the Conversation rather than fetched beside it,
    /// because everything that acts on a Conversation acts on its companions
    /// too — the sandbox it is worked in, the prompt its sessions are given,
    /// and the summary of what it was set up with.
    pub companions: Vec<super::Companion>,
}

/// Where a sidebar row's state word got to: the state it names, or the word
/// itself where this Verkstead has never heard of it.
///
/// The one read of that column that does not fail on a word it cannot parse.
/// Everything else refuses, because everything else acts on the answer — but
/// the sidebar is the way to every Conversation there is, so one row written by
/// a Verkstead from the future, or by hand, used to take the whole list with
/// it and leave the human with no route to any of them but a URL out of their
/// own history.
///
/// The word travels rather than being swallowed here: what to draw for it is
/// the wire's business, and the store has no log to put it in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowState {
    /// A word this Verkstead knows.
    Known(Lifecycle),

    /// And one it does not, kept as it was stored so that whoever draws the row
    /// can say what it found.
    Unknown(String),
}

impl RowState {
    /// The state where the word named one, and nothing where it did not.
    ///
    /// What every reader of this list but the sidebar wants: the sweeps decide
    /// whether to drive or to look for a stall, and a row whose state nobody
    /// can read is one neither should touch.
    pub fn known(&self) -> Option<Lifecycle> {
        match self {
            Self::Known(state) => Some(*state),
            Self::Unknown(_) => None,
        }
    }
}

/// One row of the conversations sidebar, drawn without reading a Timeline.
///
/// The branch is the row's name where somebody has settled on one. A
/// Conversation has no title of its own — the domain gives it a Repo, a Brief,
/// a branch and a base commit and nothing else — and of those the branch is the
/// one short line a human chose, which is what a list is read by. A name
/// Verkstead invented is not one of those, which is what [`Self::branch_named`]
/// says about the name beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationRow {
    pub id: i64,
    pub branch: String,

    /// Whether that name is one somebody settled on — see
    /// [`Conversation::branch_named`], which is the same fact about the same
    /// record.
    pub branch_named: bool,

    /// And whether the name it is carrying is still the first session's to
    /// replace — see [`Conversation::naming`], which is the same fact about the
    /// same record.
    ///
    /// What keeps the row reading *Draft* through the first minutes of the work,
    /// and what stops it reading that for ever.
    pub naming: bool,

    /// What the Repo is called, which is the only thing about it a row shows.
    pub repo: String,

    pub state: RowState,

    /// Whether anything about this Conversation is waiting on the human.
    ///
    /// One fact folded from every source there is, rather than a list the row's
    /// reader is left to weigh: what the sidebar says is *this one wants you*,
    /// and which source said so is the Conversation's own page to show. The
    /// sources are [`waits_on_the_human`]'s, all of them in the one query — the
    /// same fold that page reads through [`waiting`], so the two cannot come to
    /// disagree.
    pub waiting: bool,

    /// Whether its wrap-up has narrowed to its checks — see
    /// [`super::narrowed_to_checks`], which is the same reading of the same
    /// facts, asked of one Conversation instead of the list.
    ///
    /// Half of what the row says as *Waiting on checks*. The other half is that
    /// nothing is running on it, which the caller already reads once for the
    /// whole sidebar rather than per row.
    pub narrowed_to_checks: bool,

    /// Whether Verkstead has told the human something about this Conversation
    /// they have not looked at yet — see [`super::stamp_unseen`], which is the
    /// one thing that writes it.
    ///
    /// Read here rather than asked for per row, the way the two above are: the
    /// mark is one `EXISTS` over a table with a row per Conversation at most.
    pub unseen: bool,
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

/// And the words it holds for the two rows that fix where a list landed on the
/// branch: the backlog, and the roadmap.
///
/// Constants for a third reason again — [`landed`] asks whether a Timeline
/// already carries one, and that question is put to the `kind` column rather
/// than to an Event.
const TASK_LIST: &str = "task-list";
const STAGE_LIST: &str = "stage-list";

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
    ///
    /// And beside the summary, what that session was launched under — see
    /// [`super::RanUnder`]. `None` for a session started before Verkstead wrote
    /// that down, and for one that was paired with nothing.
    AgentOutput(super::Summary, Option<super::RanUnder>),

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

    /// A Manual Task: the instruction a human typed at the end of the Timeline
    /// for a one-off session to carry out.
    ///
    /// Markdown in the `body` column like the Brief and the handoff, because
    /// that is what it is — one document, written by a human for an agent to
    /// read. Nothing is joined in beside it: a Manual Task is its instruction,
    /// and what the session it started did lands as the Events that work lands
    /// as.
    ///
    /// **Read rather than written.** Nothing puts another on a Timeline — a
    /// steer into Implementing carries the human's instruction now, and drives
    /// the Conversation with it — and nothing rewrote the ones that are there.
    /// The kind stays because the rows do: ADR-0006's rule is that the record
    /// is kept and read as it was written.
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

    /// The human pressed **Resolve conflicts** on a finished Conversation's pull
    /// request, and this is where they did it.
    ///
    /// A kind of its own rather than a [`Event::Steer`] into Wrapping, because
    /// the two are different acts and a Timeline that drew them the same could
    /// never be read back for which happened. A steer into Wrapping opens the
    /// branch to be read again — the review's settle goes with it, and a review
    /// session runs. This deliberately does not: the work was reviewed and
    /// carried to Done on the strength of that review, and a base that has moved
    /// underneath it since is not a reason to read the same branch twice.
    ///
    /// No body, and nothing joined in beside it. Where it goes is always
    /// Wrapping — there is nowhere else a conflict is resolved — so the
    /// [`Event::Moved`] line under it says the whole of where, and the
    /// pull requests it was about are the ones the record says conflicted at
    /// that moment. What the row keeps is the deciding: somebody read a conflict
    /// on work Verkstead had finished with and asked for another round.
    ///
    /// See `crate::resolving` in the server, and [`resolve_conflicts`].
    ResolveConflicts,

    /// The backlog landed on the branch, and this is where that happened.
    ///
    /// No body, and nothing joined in beside it either. What the Timeline draws
    /// at this row is `.tasks/` as the Worktree holds it *now* — the same live
    /// reading the pinned block is drawn from — so what the row keeps is the
    /// position alone: the moment the work stopped being a plan and became a
    /// list to work through.
    ///
    /// One per Conversation. A backlog lands once, and everything the run does
    /// to it afterwards moves the files it is read from rather than the record
    /// — see [`landed`].
    TaskList,

    /// And the roadmap landed on the branch, which is the same thing one level
    /// up: written once, and drawn from a live reading of `docs/roadmaps/`.
    StageList,
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

    /// How it was asked, which is what tells one still waiting on the human from
    /// a blocking one: both are something to answer, and only one of them has a
    /// session standing still behind it — see [`super::deferrals`].
    pub ask: super::Ask,
}

impl Event {
    /// The word the `kind` column holds. `'static`, so the one statement that
    /// wants the word without an Event to hand can ask for it and let the Event
    /// go.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Brief(_) => "brief",
            Self::Moved(_) => "moved",
            Self::AgentOutput(..) => "agent-output",
            Self::QuestionSet(_) => QUESTION_SET,
            Self::Handoff(_) => "handoff",
            Self::Commit(_) => "commit",
            Self::PullRequest(_) => PULL_REQUEST,
            Self::Pause(_) => super::pauses::PAUSE,
            Self::Notice(_) => "notice",
            Self::ManualTask(_) => "manual-task",
            Self::Steer(..) => "steer",
            Self::ResolveConflicts => "resolve-conflicts",
            Self::TaskList => TASK_LIST,
            Self::StageList => STAGE_LIST,
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
            Self::AgentOutput(..) => Cow::Borrowed(""),
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
            // Nothing either: where it goes is always Wrapping, so the move
            // under it says the whole of where, and what this row keeps is
            // the deciding.
            Self::ResolveConflicts => Cow::Borrowed(""),
            // Nothing, and for neither of those reasons: there is no content
            // here to hold anywhere. The row fixes a position, and the card
            // drawn at it is read off the Worktree — see the variants.
            Self::TaskList | Self::StageList => Cow::Borrowed(""),
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
        ran_under: Option<super::RanUnder>,
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
                // Not asked for the same way: a Capture is written in the same
                // transaction as the Event and a pairing was not always written
                // at all, so an Event without one is a session from before this
                // was recorded rather than a database somebody has been in.
                ran_under,
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
            "resolve-conflicts" => Self::ResolveConflicts,
            TASK_LIST => Self::TaskList,
            STAGE_LIST => Self::StageList,
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

/// What became of choosing one of a Conversation's Pairings — or of picking a
/// role away from it altogether.
///
/// A drafting refusal among them, like the Brief and the branch name and for
/// the same reason: every Pairing is fixed when the work starts. The
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

    /// It is past drafting, so every Pairing is fixed.
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

/// What became of landing a follow-up back in the wrap-up it was opened over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// Landed: the Conversation is wrapping up again and the move is on its
    /// Timeline, with the checks put back to waiting where the follow-up
    /// pushed anything.
    Wrapped,

    /// It is not following anything up, so there is no follow-up here to end —
    /// closed out from under the session, or steered somewhere else while this
    /// was deciding.
    NotFollowingUp,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// What became of pressing **Resolve conflicts** on a finished Conversation's
/// pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolving {
    /// Recorded: the Conversation is wrapping up again, the press and the move
    /// are on its Timeline, and every pull request the record says conflicts is
    /// something the wrap-up waits on once more.
    Wrapping,

    /// It is not Done, so there is nothing here to send back to a wrap-up — it
    /// is wrapping up already, with the watchers on it, or it has been closed
    /// since the pane was drawn.
    NotDone,

    /// Nothing on it conflicts, so there is nothing to resolve. The button is
    /// drawn off the same recorded fact, so this is a press made against a
    /// reading that has moved on.
    NothingConflicts,

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
///
/// A Conversation's branch is two columns rather than one: `branch` is the name
/// Verkstead prefilled it with, and `named_branch` the one somebody settled on
/// where anybody has. Which is why handing a name back is the prefill standing
/// again rather than another name invented — see [`Conversation::branch_named`].
///
/// And a third column beside them for the stretch between: `naming` says the
/// work has started on a name Verkstead invented and the first session has been
/// told to pick a real one — see [`Conversation::naming`].
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS conversations (
             id                        INTEGER PRIMARY KEY AUTOINCREMENT,
             repo_id                   INTEGER NOT NULL REFERENCES repos(id),
             created_at                TEXT NOT NULL,
             branch                    TEXT NOT NULL,
             named_branch              TEXT,
             naming                    INTEGER NOT NULL DEFAULT 0,
             base_commit               TEXT,
             state                     TEXT NOT NULL,
             grilling_profile_id       INTEGER REFERENCES profiles(id),
             implementation_profile_id INTEGER REFERENCES profiles(id),
             review_profile_id         INTEGER REFERENCES profiles(id)
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
    // as a lock hangs off a Set: there is no migration machinery here and
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

    // The model half of a Conversation's Pairings, one row per role. A
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

    // Which of a Conversation's roles the human picked away: one row per role
    // that runs no session at all. A table of its own for the reason the model
    // half is in one — there is no migration machinery here and `conversations`
    // is STRICT and left alone — and it needs none besides: a database written
    // before this arrives with the table empty, which is every Conversation
    // having picked no such thing, and that is exactly what they had.
    //
    // Apart from the Profile column rather than a value inside it, because a
    // skip is not a Profile: the column says which account a role runs under and
    // this says the role runs nothing. A role with a row here has no Profile —
    // picking one takes the row away and picking the row takes the Profile away,
    // in the one write — so the two cannot disagree about what was picked.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skipped_roles (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             role            TEXT NOT NULL,
             PRIMARY KEY (conversation_id, role)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the skipped roles table")?;

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
/// A name somebody settled on, which is what a stage's own slug is and what a
/// test naming its Conversation means. A name nobody has had to think of yet
/// goes in through [`start_unnamed_conversation`] instead.
///
/// `None` means there is no Repo on the registry with that id — one that never
/// existed, and one somebody has taken away, which are one answer because
/// neither is a repository new work may be put in. The insert selects from
/// `repos` rather than trusting the id, so a Conversation cannot come to hang
/// off a repository that was never registered — SQLite does not enforce a
/// foreign key unless it is asked to, and a row that named nothing would be a
/// Conversation with nowhere to work. A Repo that has been taken off the
/// registry falls out of that same `SELECT`, so the removal holds against a
/// press made from a list that has not heard about it yet.
///
/// The Brief goes in with it, in the same transaction: the Brief is the first
/// Event, and a Conversation whose Timeline was empty because the second insert
/// failed would be one the human could not write anything into.
pub async fn start_conversation(
    pool: &SqlitePool,
    repo_id: i64,
    branch: &str,
) -> Result<Option<i64>> {
    started(pool, repo_id, branch, Named::Settled, None).await
}

/// The same, on a name Verkstead invented rather than one anybody settled on.
///
/// What the New conversation button starts: the record needs a branch name from
/// the moment it exists, and nobody has thought of one yet. It is Verkstead's
/// until the human types one — the row reads *Draft* until then, and the name
/// itself is shown nowhere.
pub async fn start_unnamed_conversation(
    pool: &SqlitePool,
    repo_id: i64,
    branch: &str,
) -> Result<Option<i64>> {
    started(pool, repo_id, branch, Named::Prefilled, None).await
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
    started(pool, repo_id, branch, Named::Prefilled, Some(roadmap)).await
}

/// Whose the branch name a Conversation is started on is.
///
/// Two states rather than a bare `bool` at three call sites, because what the
/// argument decides is what the human is shown: a settled name is the row's
/// title, and a prefilled one is drawn nowhere at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Named {
    /// A name somebody chose: a stage's slug, or the name a test gave its
    /// Conversation.
    Settled,

    /// A name Verkstead invented, because the record needs one and nobody has
    /// thought of one yet.
    Prefilled,
}

/// What all three of them do: the row, its empty Brief, and the adoption mark
/// where there is one to write.
///
/// All of it in one transaction. A Conversation whose Timeline was empty
/// because the second insert failed would be one the human could not write
/// anything into, and one that lost its mark to a third would be a Draft drawn
/// on the wrong page.
async fn started(
    pool: &SqlitePool,
    repo_id: i64,
    branch: &str,
    named: Named,
    adopts: Option<&str>,
) -> Result<Option<i64>> {
    let mut tx = super::writing(pool, "starting a Conversation").await?;

    // The registry is asked in the insert's own `SELECT` rather than before it,
    // for the reason the path's uniqueness is left to the index: a look taken
    // first is a look something can get past. A Repo that has been taken off the
    // registry falls out here, so a sidebar that has not heard about the removal
    // cannot start work in a repository Verkstead has stopped offering.
    //
    // The name goes in both columns where it is settled: the prefill is what
    // stands if the name is ever handed back, and a Conversation started on a
    // name somebody chose has that name to fall back on and no other.
    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO conversations
             (repo_id, created_at, branch, named_branch, base_commit, state)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?, NULL, ?
         FROM repos
         WHERE id = ? AND id NOT IN (SELECT repo_id FROM unregistered_repos)
         RETURNING id",
    )
    .bind(branch)
    .bind((named == Named::Settled).then_some(branch))
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

/// Whether anything about a Conversation is waiting on the human, said as SQL
/// about a Conversation row aliased `c`.
///
/// The rule itself, in one place. Two readings of it draw from it: the sidebar's
/// list folds it per row inside [`conversations`]'s own query, and [`waiting`]
/// asks it of the one Conversation a page is drawn for. Written twice they would
/// be two rules the day one of them was edited, and a Conversation whose row
/// said it wanted the human while its own page said it wanted nobody is a
/// Verkstead that cannot be believed about either.
///
/// An `OR` over the sources, in the order they appear below:
///
/// - A **Question Set with no Response and no lock** — an ask left open.
///   Blocking and Deferred alike: what draws the human is that there is
///   something answerable, not whether the asking session is idling on it.
/// - A **stop that came from outside the human**, which is a Conversation
///   nothing is driving any more and which goes again only when they say so —
///   Verkstead's own brake, an account out of window, a driver a crash took
///   away. Their own press is not one of them: it stops the run just the same
///   and waits for the same press, but a mark saying *look here* about
///   something they did themselves is what makes the marks worth ignoring. See
///   [`super::Decision::waits_on_the_human`], which is that rule, and
///   `stops::waited_on`, which is it said as the condition below. A column on
///   the row rather than a subselect, so the whole list costs one query.
///
/// A grilling waiting on its closing proposal is the first of them and not a
/// source of its own: the proposal rides a Question Set, and an unanswered Set
/// is already an ask left open.
///
/// A **Draft** is none of them, whatever else is true of it: it is waiting on
/// the human in the ordinary sense, and the sidebar says so by drawing it as a
/// draft rather than by marking it as an ask.
///
/// **Closed** is none of them either, and for the opposite reason: nothing is
/// waiting because nothing is left. Closing shuts the Sets it found open — see
/// the server's `conversations::close` — so what this excludes is mostly the
/// stop the Conversation carried, which stays on the record as history. A
/// **Done** Conversation is not excluded: its Sets are still answerable, and an
/// answerable ask is still an ask.
fn waits_on_the_human() -> String {
    format!(
        "c.state NOT IN ('{draft}', '{closed}') AND (
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
             OR ({stopped})
         )",
        draft = Lifecycle::Draft.stored(),
        closed = Lifecycle::Closed.stored(),
        stopped = super::stops::waited_on(),
    )
}

/// The same question about one Conversation, which is what its own page asks.
///
/// [`waits_on_the_human`] said of a single row, so the page and the sidebar row
/// can only ever agree. A Conversation that is not there waits on nobody: the
/// page has nothing to draw either way, and an error where a `false` will do
/// would be a read failing over a Conversation that has gone.
pub async fn waiting(pool: &SqlitePool, conversation_id: i64) -> Result<bool> {
    let waiting: Option<bool> = sqlx::query_scalar(&format!(
        "SELECT ({waiting}) FROM conversations c WHERE c.id = ?",
        waiting = waits_on_the_human(),
    ))
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!("reading whether Conversation {conversation_id} waits on the human")
    })?;

    Ok(waiting.unwrap_or(false))
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
/// `waiting` is folded inside the query rather than by the caller, because
/// every source of it is a read of this database and the sidebar is one list: a
/// caller folding them itself would be issuing a query per row for facts a
/// subselect already has. The rule is [`waits_on_the_human`], which is where the
/// sources are set out and which the Conversation's own page reads through
/// [`waiting`].
///
/// `narrowed_to_checks` rides along in the same query for the same reason: it is
/// a reading of the wrap-up's settle facts — see [`super::narrowed_to_checks`],
/// which asks it of one Conversation — and a caller folding it itself would be
/// issuing a query per row for something a subselect already has.
///
/// And `unseen` rides along for the same reason a third time: whether Verkstead
/// has told the human something about this Conversation that they have not
/// looked at yet — see [`super::stamp_unseen`]. Not one of the waiting sources
/// above, because the two say different things and the row says which in words:
/// *something wants you* against *there is news here*.
///
/// What the human has archived is not here at all, unless they have asked to be
/// shown it — see [`super::archive_conversation`] and
/// [`super::showing_archived`]. Archiving is the one thing that takes a
/// Conversation off this list, and it takes it off nothing else: its Timeline,
/// its branch and its own page are where they were.
///
/// The toggle is read inside the query rather than handed in, because it is a
/// fact about this list and this is the one thing that draws it: a caller given
/// the choice would be a second place to get it wrong, and there is no other way
/// the sidebar should ever be read.
///
/// A row whose state word this Verkstead does not know is still on the list,
/// carrying the word — see [`RowState`]. Every other read of that column
/// refuses one, and this one cannot afford to: the list is the only route to a
/// Conversation's own page, so one bad row failing it would leave the human
/// with nothing to press on any of them.
pub async fn conversations(pool: &SqlitePool) -> Result<Vec<ConversationRow>> {
    /// The columns in the order the query below selects them.
    type Row = (i64, String, bool, bool, String, String, bool, bool, bool);

    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT c.id, COALESCE(c.named_branch, c.branch),
                c.named_branch IS NOT NULL AS branch_named,
                c.naming,
                r.name, c.state,
                ({waiting}) AS waiting,
                c.state = 'wrapping'
                  AND EXISTS (
                      SELECT 1 FROM wrap_up_settled w
                      WHERE w.conversation_id = c.id AND w.waiting_on = 'review'
                  )
                  AND EXISTS (
                      SELECT 1 FROM wrap_up_settled w
                      WHERE w.conversation_id = c.id AND w.waiting_on = 'comments'
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM wrap_up_settled w
                      WHERE w.conversation_id = c.id AND w.waiting_on = 'checks'
                  ) AS narrowed_to_checks,
                EXISTS (
                    SELECT 1 FROM unseen_conversations u
                    WHERE u.conversation_id = c.id
                ) AS unseen
         FROM conversations c
         JOIN repos r ON r.id = c.repo_id
         LEFT JOIN placements m ON m.conversation_id = c.id
         WHERE EXISTS (SELECT 1 FROM shown_archives)
            OR NOT EXISTS (
                   SELECT 1 FROM archived_conversations a WHERE a.conversation_id = c.id
               )
         ORDER BY m.place IS NULL DESC, m.place, c.id DESC",
        waiting = waits_on_the_human(),
    ))
    .fetch_all(pool)
    .await
    .context("listing the Conversations")?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                branch,
                branch_named,
                naming,
                repo,
                state,
                waiting,
                narrowed_to_checks,
                unseen,
            )| ConversationRow {
                id,
                branch,
                branch_named,
                naming,
                repo,
                // The one place a state word that will not parse is carried
                // rather than refused. This list is the way to every
                // Conversation there is, so one bad row used to take all of
                // them off the page — see [`RowState`].
                state: match Lifecycle::read(&state) {
                    Ok(state) => RowState::Known(state),
                    Err(_) => RowState::Unknown(state),
                },
                waiting,
                narrowed_to_checks,
                unseen,
            },
        )
        .collect())
}

/// How much work is on one Repo, counted by whether it is over.
///
/// **Live** is everything still going, a Draft included: the Conversation is on
/// this repository now, and somebody may be waiting on it. **Finished** is Done
/// and Closed together — work that ended, however it ended.
///
/// What the human archived is counted like anything else. Archiving takes a
/// Conversation off the sidebar and off nothing else, so a Repo that carried
/// twenty Conversations carried twenty whatever is being shown at the moment.
///
/// Counted by reading the states back rather than by asking SQL which of them
/// are over: the words in the column are [`Lifecycle`]'s, one of them is a
/// spelling only [`Lifecycle::read`] knows about, and a list of words in a query
/// would be a second opinion about what "finished" means.
pub async fn work_on_repo(pool: &SqlitePool, repo_id: i64) -> Result<Work> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT state, COUNT(*)
         FROM conversations
         WHERE repo_id = ?
         GROUP BY state",
    )
    .bind(repo_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("counting the Conversations on Repo {repo_id}"))?;

    let mut work = Work {
        live: 0,
        finished: 0,
    };

    for (state, count) in rows {
        match Lifecycle::read(&state)? {
            Lifecycle::Done | Lifecycle::Closed => work.finished += count,
            _ => work.live += count,
        }
    }

    Ok(work)
}

/// The two counts [`work_on_repo`] answers with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    /// Conversations still going — everything that is neither Done nor Closed.
    pub live: i64,

    /// And the ones that are over.
    pub finished: i64,
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
        bool,
        bool,
        Option<String>,
        String,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        i64,
        String,
        String,
        String,
    );

    let row: Option<Row> = sqlx::query_as(
        "SELECT c.id, c.created_at,
                COALESCE(c.named_branch, c.branch),
                c.named_branch IS NOT NULL AS branch_named,
                c.naming,
                c.base_commit, c.state,
                c.grilling_profile_id, c.implementation_profile_id,
                c.review_profile_id,
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
        branch_named,
        naming,
        base_commit,
        state,
        grilling_profile_id,
        implementation_profile_id,
        review_profile_id,
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
        branch_named,
        naming,
        base_commit: base_commit.filter(|commit| !commit.is_empty()),
        state: Lifecycle::read(&state)?,
        grilling_pairing: picked(pool, id, Role::Grilling, grilling_profile_id).await?,
        implementation_pairing: pairing(pool, id, Role::Implementation, implementation_profile_id)
            .await?,
        review_pairing: picked(pool, id, Role::Review, review_profile_id).await?,
        worktree: worktree(pool, id).await?,
        direction: direction(pool, id).await?,
        adopting: adopting(pool, id).await?,
        companions: super::companions(pool, id).await?,
    }))
}

/// Everything closing a Conversation needs to know about it, and nothing else.
///
/// Which is: that there is one, and where every directory it was given to work
/// in stands. The state it is in does not come into it — closing is reachable
/// from all of them — and neither does the direction, the pairings, the base
/// commit or a companion's mode.
///
/// See [`closable`] for why that is a read of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closable {
    /// The Repo the worktree below was cut from, which is the directory git is
    /// asked to remove it from.
    pub repo: std::path::PathBuf,

    /// And the worktree, where grilling has made one.
    pub worktree: Option<std::path::PathBuf>,

    /// The same pair again for every companion, plus what the Repo is called —
    /// a worktree that will not go is logged, and a log naming a path and no
    /// repository is a line nobody can act on.
    pub companions: Vec<ClosableCompanion>,
}

/// One companion of a [`Closable`]: where its repository is, what it is called,
/// and the directory it was checked out into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosableCompanion {
    pub repo: std::path::PathBuf,
    pub name: String,
    pub worktree: Option<std::path::PathBuf>,
}

/// The same Conversation [`load_conversation`] reads, cut down to what a close
/// acts on — and read without parsing a single stored word.
///
/// A sibling read rather than a flag on the other one, because the difference
/// is not how much is fetched but what it is allowed to refuse. The full read
/// parses the state, the direction and each companion's mode, and any of the
/// three going bad takes the close with it — which is exactly backwards: a
/// Conversation whose record has gone strange is the one a human most needs to
/// be able to end. This one has nothing to parse, so there is nothing in it to
/// refuse but a Conversation that is not there.
///
/// Every other reader still goes through [`load_conversation`], and should: a
/// reader that is going to *act* on the state wants to be told the word is bad
/// rather than handed a guess.
pub async fn closable(pool: &SqlitePool, id: i64) -> Result<Option<Closable>> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT r.path, w.path
         FROM conversations c
         JOIN repos r ON r.id = c.repo_id
         LEFT JOIN worktrees w ON w.conversation_id = c.id
         WHERE c.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading what closing Conversation {id} would remove"))?;

    let Some((repo, worktree)) = row else {
        return Ok(None);
    };

    let companions: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT r.path, r.name, w.path
         FROM companions c
         JOIN repos r ON r.id = c.repo_id
         LEFT JOIN companion_worktrees w
                ON w.conversation_id = c.conversation_id AND w.repo_id = c.repo_id
         WHERE c.conversation_id = ?
         ORDER BY r.name, r.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the companion checkouts of Conversation {id}"))?;

    Ok(Some(Closable {
        repo: std::path::PathBuf::from(repo),
        worktree: worktree.map(std::path::PathBuf::from),
        companions: companions
            .into_iter()
            .map(|(repo, name, worktree)| ClosableCompanion {
                repo: std::path::PathBuf::from(repo),
                name,
                worktree: worktree.map(std::path::PathBuf::from),
            })
            .collect(),
    }))
}

/// Every directory a Conversation's record still names as a checkout: its own
/// worktree, and one per companion it was given.
///
/// The keep-set an orphan sweep decides by, which is why it is one read of both
/// tables rather than two reads a caller joins. A row here is the whole fact:
/// closing deletes both a Conversation's own worktree row and its companions'
/// — see [`close_conversation`] — and archiving is refused for a Conversation
/// that is not already closed, so *a live Conversation still works in it* and
/// *there is a row for it* are the same statement. Nothing here parses a state
/// word, and nothing needs to.
///
/// Which includes a Conversation that is Done. Done is not Closed: its
/// directory is still where a Follow-up steer will pick the work up, and only
/// closing gives one back.
///
/// Every path there is rather than a page of them, because what asks is
/// deciding what to delete: a keep-set that stopped short would be a keep-set
/// that named live work as an orphan.
pub async fn recorded_worktrees(pool: &SqlitePool) -> Result<Vec<PathBuf>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT path FROM worktrees
         UNION
         SELECT path FROM companion_worktrees",
    )
    .fetch_all(pool)
    .await
    .context("listing the worktrees the Conversations are still working in")?;

    Ok(rows
        .into_iter()
        .map(|(path,)| PathBuf::from(path))
        .collect())
}

/// One of a Conversation's Pairings: the Profile its column names, and the
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

/// The whole of what a Conversation has settled about one of its roles: a
/// Pairing, the row that runs no session, or nothing yet.
///
/// The skip is read first because it is the stronger fact: picking it takes the
/// Profile away in the same write, so a row here is what the human last picked
/// whatever the column says.
async fn picked(
    pool: &SqlitePool,
    conversation: i64,
    role: Role,
    profile_id: Option<i64>,
) -> Result<super::Picked> {
    if skipped(pool, conversation, role).await? {
        return Ok(super::Picked::Skipped);
    }

    Ok(match pairing(pool, conversation, role, profile_id).await? {
        Some(pairing) => super::Picked::Under(pairing),
        None => super::Picked::Nothing,
    })
}

/// Whether the human picked this role away.
async fn skipped(pool: &SqlitePool, conversation: i64, role: Role) -> Result<bool> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM skipped_roles WHERE conversation_id = ? AND role = ?")
            .bind(conversation)
            .bind(role.stored())
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading whether Conversation {conversation} skipped a role")
            })?;

    Ok(row.is_some())
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

/// Which of the three roles a Pairing is being chosen for.
///
/// The word the `pairing_models` table holds, and the column the Profile half
/// goes in — the two halves of one choice, so the role names both rather than
/// letting a caller pass one and forget the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Grilling,
    Implementation,
    Review,
}

impl Role {
    /// Every one of them, in the order the work goes through them: what the
    /// memory is written from and read back into, so that adding a role is the
    /// variant and nothing else.
    pub(crate) const ALL: [Self; 3] = [Self::Grilling, Self::Implementation, Self::Review];

    pub(crate) fn stored(self) -> &'static str {
        match self {
            Self::Grilling => "grilling",
            Self::Implementation => "implementation",
            Self::Review => "review",
        }
    }

    pub(crate) fn column(self) -> &'static str {
        match self {
            Self::Grilling => "grilling_profile_id",
            Self::Implementation => "implementation_profile_id",
            Self::Review => "review_profile_id",
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

/// And the one the wrap-up's review session will run under.
pub async fn set_review_pairing(
    pool: &SqlitePool,
    id: i64,
    profile_id: i64,
    model: Option<&str>,
) -> Result<Chosen> {
    choose(pool, id, Role::Review, profile_id, model).await
}

/// Or pick the row that says there is to be no review at all.
///
/// A choice like the ones above it and refused on the same terms: made while the
/// Conversation drafts, fixed when the work starts, and a Conversation that has
/// picked it is as ready to start as one that picked a Pairing.
///
/// The Profile column and the model row go with it, so what is left is the one
/// fact — this role runs nothing — rather than that beside an account nobody
/// will launch.
pub async fn skip_review(pool: &SqlitePool, id: i64) -> Result<Chosen> {
    skip(pool, id, Role::Review).await
}

/// And the row that says there is to be no grilling at all.
///
/// The same choice one role along, and it says more than the review one does:
/// what a Conversation that picked it starts is an inline implementation on the
/// Brief, so the press that would have begun an interview begins the work — see
/// [`start_building`].
pub async fn skip_grilling(pool: &SqlitePool, id: i64) -> Result<Chosen> {
    skip(pool, id, Role::Grilling).await
}

/// Record that a role runs no session at all.
///
/// [`choose`]'s shape and [`choose`]'s refusals, because it is the same act:
/// the human picking one row of the one list, on a Conversation that is still
/// drafting.
async fn skip(pool: &SqlitePool, id: i64, role: Role) -> Result<Chosen> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(match refusal {
            Edited::NoSuchConversation => Chosen::NoSuchConversation,
            _ => Chosen::NotDrafting,
        });
    }

    let mut tx = super::writing(pool, "picking a role away from a Conversation").await?;

    sqlx::query(&format!(
        "UPDATE conversations SET {} = NULL WHERE id = ?",
        role.column()
    ))
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("clearing the Profile Conversation {id} had chosen"))?;

    sqlx::query("DELETE FROM pairing_models WHERE conversation_id = ? AND role = ?")
        .bind(id)
        .bind(role.stored())
        .execute(&mut *tx)
        .await
        .with_context(|| format!("clearing the model Conversation {id} had paired"))?;

    sqlx::query(
        "INSERT INTO skipped_roles (conversation_id, role) VALUES (?, ?)
         ON CONFLICT (conversation_id, role) DO NOTHING",
    )
    .bind(id)
    .bind(role.stored())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("picking a role away from Conversation {id}"))?;

    tx.commit()
        .await
        .with_context(|| format!("picking a role away from Conversation {id}"))?;

    Ok(Chosen::Chosen)
}

/// Record one of the Pairings, both halves of it.
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

    let mut tx = super::writing(pool, "choosing a Profile for a Conversation").await?;

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

    // And the skip, because a Pairing picked is the row that runs no session
    // unpicked: the two are rows of one list, and a role left holding both would
    // be a Conversation that had picked twice.
    sqlx::query("DELETE FROM skipped_roles WHERE conversation_id = ? AND role = ?")
        .bind(id)
        .bind(role.stored())
        .execute(&mut **tx)
        .await
        .with_context(|| format!("clearing the role Conversation {id} had picked away"))?;

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
/// Responses and their locks hang off the same Event rows, and asking per
/// Set would be a read for every Question the human has ever been put.
///
/// `archivings` is where a lock is stored: the name it went under before
/// locking was called locking, kept because there is no migration machinery
/// here to rename a table with.
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
        Option<String>,
    );

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT e.id, e.at, e.kind, e.body,
                q.id, q.body, r.submitted_at, r.body, a.archived_at AS locked_at,
                c.sha, c.subject, c.files, c.insertions, c.deletions, cr.name
         FROM timeline_events e
         JOIN conversations v ON v.id = e.conversation_id
         LEFT JOIN set_events s ON s.event_id = e.id
         LEFT JOIN question_sets q ON q.id = s.set_id
         LEFT JOIN responses r ON r.set_id = s.set_id
         LEFT JOIN archivings a ON a.set_id = s.set_id
         LEFT JOIN commits c ON c.event_id = e.id
         LEFT JOIN repos cr ON cr.id = c.repo_id AND cr.id <> v.repo_id
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

    // And how each of the Sets above was asked, for the arithmetic again — see
    // [`super::deferrals::stored_on_timeline`]. Cheaper than any of them: one
    // indexed column, and most Conversations have no stored ask at all, which
    // is what a Set this does not name comes back as.
    let stored = super::deferrals::stored_on_timeline(pool, conversation_id).await?;

    // And what each of those sessions ran under, for the arithmetic again and
    // at the Capture summaries' cost: one row per session, and a Timeline with
    // no session on it answers with nothing.
    let mut ran_under = super::session_pairings::on_timeline(pool, conversation_id).await?;

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
                locked_at,
                sha,
                subject,
                files,
                insertions,
                deletions,
                repo,
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
                        // And absent for every commit in the Conversation's own
                        // repository, which is what the join above says: a label
                        // is drawn where repos mix and nowhere else.
                        repo,
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
                        settlement: settled(set_id, answered_at, answer, locked_at)?,
                        ask: stored.get(&set_id).copied().unwrap_or(super::Ask::Blocking),
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
                event: Event::read(
                    &kind,
                    body,
                    summary,
                    ran_under.remove(&id),
                    set,
                    commit,
                    pull_request,
                    pause,
                )?,
            })
        })
        .collect()
}

/// How a Set on the Timeline was settled, out of the two rows that can settle
/// one, or `None` while it is still waiting on the human.
///
/// The Response wins where both are somehow there: the answering is the
/// decision, and a decision is not something a lock can take back.
fn settled(
    set_id: i64,
    answered_at: Option<String>,
    answer: Option<String>,
    locked_at: Option<String>,
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

    Ok(locked_at.map(|locked_at| {
        super::Settlement::LockedUnanswered(super::SetLocked { set_id, locked_at })
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
/// `ask` is which of the three kinds it is, and the row that records a stored
/// one is written in this same transaction: a Set that was stored a moment
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

    let mut tx = super::writing(pool, "putting a Question Set").await?;

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

    if ask.deferred_shaped() {
        super::deferrals::defer(&mut tx, id, ask.idled()).await?;
    }

    tx.commit().await.context("putting a Question Set")?;

    // The stored kinds say so in the reply, which is what tells the CLI there
    // is nothing to wait on — see [`verkstead_schema::SetCreated`]. Said by the
    // server rather than assumed by the CLI, because which channel a Set was
    // asked on is the backend's fact and the backend is what the server knows.
    Ok(Some(SetCreated {
        id,
        created_at,
        stored: ask.deferred_shaped(),
    }))
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

/// The newest Set this wrap has put to the human, where it has asked anything.
///
/// The review's own Set until a batch of comments asks something after it, and
/// that batch's from then on — see [`proposals`], which is where *this wrap's*
/// is worked out.
///
/// The newest rather than the review's own, because nothing on the record says
/// which session asked one and nothing has to. One Worktree holds one agent and
/// nothing advances past a stop, so the Set the session that is running put up
/// is the last one there is for as long as anything is asking about it.
pub async fn last_proposal(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    Ok(proposals(pool, conversation_id)
        .await?
        .last()
        .map(|proposal| proposal.set_id))
}

/// And the newest one a *batch* session put up, where a batch has put one up at
/// all.
///
/// The half of [`last_proposal`] that is nobody else's. A wrap-up's proposals
/// come in one order and only one — the review is the session a wrap-up starts
/// with, and no batch is dispatched until it has settled — so the moment the
/// review settled is the line between them: put up before it and the Set is the
/// review's own, after it and it is a batch's. See [`super::settled_when`].
///
/// Which matters because *the review's Set is the newest proposal until a batch
/// asks anything*, and how a review ended is the report of the session that ran
/// it rather than something to work out from its Set afterwards. A wrap-up whose
/// review settled having landed nothing it was answered for leaves a Set that
/// reads as owing work, and it is not the batch half's to carry out.
///
/// `None` where the review has not settled, which is every wrap-up that has not
/// got as far as a batch: nothing is dispatched about what was said until the
/// review is over.
pub async fn last_batch_proposal(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    let Some(settled) = settled_when(pool, conversation_id, WaitingOn::Review).await? else {
        return Ok(None);
    };

    Ok(proposals(pool, conversation_id)
        .await?
        .into_iter()
        .filter(|proposal| proposal.asked_at > settled)
        .map(|proposal| proposal.set_id)
        .next_back())
}

/// Every Set this wrap has put to the human with somebody idling on it, oldest
/// first.
///
/// **Every one of them**, because a wrap-up's asks are all of them proposals:
/// the review reads the branch and proposes what to do about what it found, and
/// a batch session proposes what to do about what was said on the pull request.
/// Nothing marks one as such and nothing needs to — a Set that says which kind
/// of ask it was would be a second record to keep true, and the one that could
/// disagree with the session that asked it.
///
/// Which widens what counts: a Set some other session of this wrap put up is one
/// of these too, and a Set left standing behind one is read as this wrap's ask
/// with nobody behind it. That is the safe way round for what hangs on it —
/// what is on the other side is a run stopping, and a question nobody is coming
/// back to answer is worth stopping over whoever asked it.
///
/// **Idled, though, and never Deferred**, which is the one thing that width must
/// not swallow. A Deferred Ask idles nobody: the session that sent one carried
/// straight on, its Answers reach a later session by design, and it is
/// unanswered for as long as the human likes without anything being owed. So a
/// Deferred Set is not a proposal left standing — and reading one as such would
/// stop the run over a question that was working exactly as it was meant to,
/// and close it on the human's behalf into the bargain. A store-and-nudge ask
/// is on the other side of that line, stored though it is: a session is idling
/// on it with its turn ended, so it is a proposal like any other. This is the
/// same question [`unanswered_set_since`] asks of a quiet session, and the two
/// have to answer it the same way: a Set that holds no session open holds no
/// wrap-up open either.
///
/// **This wrap's**, because a Conversation can wrap up more than once: a review
/// that splits its findings out into a backlog leaves Wrapping to build them and
/// comes back for a second wrap, and the first wrap's proposals are answered and
/// done with. Counting them would be a second review that never ran, because the
/// review it found asking was last month's. So the window opens at the newest
/// move into Wrapping — and where there has been no such move, at the start of
/// the Timeline, which is every Conversation that has not got that far.
///
/// **And only the ones still standing.** A Set locked unanswered is one nobody
/// is ever going to answer, which is what Verkstead closes a proposal whose
/// session is gone as — see [`super::lock_set`]. Counting one would be the
/// same mistake the other way about: the review it found asking is a question
/// nothing is left to act on, so no fresh reading of the branch could ever be
/// recognised as the review of this wrap.
async fn proposals(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Proposal>> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT q.id, q.created_at
         FROM question_sets q
         JOIN set_events s ON s.set_id = q.id
         JOIN timeline_events e ON e.id = s.event_id
         LEFT JOIN archivings a ON a.set_id = q.id
         LEFT JOIN deferrals d ON d.set_id = q.id
         WHERE e.conversation_id = ?
           AND a.set_id IS NULL
           AND (d.set_id IS NULL OR d.idled)
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

    Ok(rows
        .into_iter()
        .map(|(set_id, asked_at)| Proposal { set_id, asked_at })
        .collect())
}

/// One proposal, as the reads above tell them apart: which Set it is, and when
/// it was put to the human.
///
/// The moment is what says which half of a wrap-up put it up — see
/// [`last_batch_proposal`] — and nothing on the Set itself does.
struct Proposal {
    /// The Set the proposal is on.
    set_id: i64,

    /// When it was asked, as the Set's own row records it.
    asked_at: String,
}

/// A Question Set of this Conversation's that arrived after `event_id` and is
/// still waiting to be answered, or `None` where none is.
///
/// Unanswered *and* unlocked: a Set the human closed without answering is one
/// nothing is coming for, so it is settled as much as an answered one is.
///
/// The Event id is what makes it *whose* Set. Nothing else on the record says
/// which session asked one, and nothing has to: one Worktree holds one agent, so
/// every Set that landed after a session's own Event is that session's. What
/// asks is a driver deciding whether a quiet session is finished — a session
/// idling on a Blocking Ask prints nothing for hours, and quiet alone would reap
/// it mid-question.
///
/// The Sets somebody is idling on, for that same reason read the other way: a
/// Deferred Ask idles nobody, so a session that has gone quiet behind one has
/// finished rather than being mid-question, and a driver that waited on it would
/// wait for as long as the human took to answer something nothing was waiting
/// for.
///
/// A store-and-nudge ask is one somebody is idling on, whatever the row beside
/// it looks like: the session that sent one has ended its turn and is waiting
/// for the nudge, so ending it on quiet would leave the Response with nothing to
/// nudge — see [`super::Ask`].
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
           AND r.set_id IS NULL AND a.set_id IS NULL
           AND (d.set_id IS NULL OR d.idled)
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

/// A Question Set of this Conversation's that is still waiting to be answered,
/// whoever asked it.
///
/// [`unanswered_set_since`] widened to the whole Timeline, which is the same
/// question asked without a session to ask it *of*: what a follow-up's rule
/// wants to know is whether the human is left holding a question, and a
/// question is one of those whoever put it up.
///
/// Every Timeline Event's id is positive, so opening the window at zero leaves
/// nothing out.
///
/// The Sets somebody is idling on, and never a Deferred one, exactly as the read
/// it is made of: a Deferred Ask idles nobody and holds nothing open, so a
/// follow-up that waited on one would be waiting on a question that was working
/// exactly as it was meant to.
pub async fn open_set(pool: &SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    unanswered_set_since(pool, conversation_id, 0).await
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

/// Where a Set is read: the Conversation it was asked from, and the Timeline
/// Event it landed on.
///
/// The two halves a path to a Set is made of. A Set has no page of its own — it
/// is read in the details pane of its Event — so anything that has to say where
/// a Set is, a push notification above all, needs the pair rather than the id.
///
/// [`asked_from`] answers the first half alone, for the callers whose question
/// stops at which Conversation this belongs to. `None` here means what it means
/// there: a broken record rather than a Set with nowhere to be, since [`ask`]
/// writes the Set, its Event and the row joining them in one transaction.
pub async fn opened_at(pool: &SqlitePool, set_id: i64) -> Result<Option<(i64, i64)>> {
    let found: Option<(i64, i64)> = sqlx::query_as(
        "SELECT e.conversation_id, e.id
         FROM set_events s
         JOIN timeline_events e ON e.id = s.event_id
         WHERE s.set_id = ?",
    )
    .bind(set_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("looking for the Timeline Event Question Set {set_id} landed on"))?;

    Ok(found)
}

/// Where a Conversation stands, and nothing else about it.
///
/// For the readers whose whole question is the state: whether the Set on the
/// page in front of the human is a follow-up's, above all. The whole
/// [`Conversation`] is a join across the Repo and every Pairing, which is more
/// of the store read than one word is worth.
pub async fn state(pool: &SqlitePool, id: i64) -> Result<Option<Lifecycle>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    row.map(|(state,)| Lifecycle::read(&state)).transpose()
}

/// Rewrite the Brief of the round a drafting Conversation is in.
///
/// The Brief Event is edited in place rather than added to, and it is the
/// *newest* of them: the frozen-Brief rule the design states — a steered round
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

/// Name the branch a drafting Conversation's work will be done on, or hand the
/// naming of it back to Verkstead.
///
/// Whether the name is one git would take is decided above the store, where git
/// itself is asked — this records what it is given.
///
/// `None` is the field cleared: the settled name goes and the prefill Verkstead
/// started the Conversation with stands again, which is the name that was there
/// before anybody typed. Not a branch called nothing, and not a fresh name
/// invented either — the one that has been sitting in the record all along.
///
/// Refused once the branch has been made as well as off the state — see
/// [`branch_made`]. Drafting says as much on its own now that a second round
/// opens where it is steered rather than back in Draft; the branch is asked
/// about all the same, because what the work branched from is a fact from the
/// moment there is a branch and no field of the human's rewrites one.
pub async fn rename_branch(pool: &SqlitePool, id: i64, branch: Option<&str>) -> Result<Edited> {
    if let Some(refusal) = not_drafting(pool, id).await? {
        return Ok(refusal);
    }

    if branch_made(pool, id).await? {
        return Ok(Edited::NotDrafting);
    }

    sqlx::query("UPDATE conversations SET named_branch = ? WHERE id = ?")
        .bind(branch)
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("renaming the branch of Conversation {id}"))?;

    Ok(Edited::Saved)
}

/// Settle for the branch name a Conversation is carrying, the session that was
/// told to replace it having ended without doing so.
///
/// The other end of the naming instruction, and the one that has to be there:
/// without it a session that read the instruction and left the name alone would
/// leave the Conversation reading *Draft* for the rest of its life. What it
/// settles is nothing about the name — whose it is and what it is are both
/// exactly what they were — only that nobody is waiting for another one.
///
/// Written after every session rather than after the first, because the first is
/// the only one it can ever find anything to write: nothing sets this but the
/// start of the work — see [`Conversation::naming`].
pub async fn settle_naming(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("UPDATE conversations SET naming = 0 WHERE id = ? AND naming = 1")
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("settling for the branch name of Conversation {id}"))?;

    Ok(())
}

/// The branch a Conversation's work is on right now, and nothing else about it.
///
/// [`load_conversation`] answers this too, along with everything else a
/// Conversation is. This is for the readers that ask over and over — the commit
/// sweep looks every couple of seconds while a session runs, and the one thing
/// that can have moved under it is this name.
///
/// `None` is no such Conversation.
pub async fn conversation_branch(pool: &SqlitePool, id: i64) -> Result<Option<String>> {
    let branch: Option<(String,)> =
        sqlx::query_as("SELECT COALESCE(named_branch, branch) FROM conversations WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading the branch of Conversation {id}"))?;

    Ok(branch.map(|(branch,)| branch))
}

/// Follow the Conversation's branch to the name a session renamed it to.
///
/// Not [`rename_branch`]: that is the human naming a branch that has not been
/// cut yet, and it is refused from the moment there is one. This is the other
/// direction — the branch has been cut, worked on and renamed in git, and the
/// record is catching up with what the repository now says. So it is refused
/// for nothing: whether this is a rename or a broken checkout is decided by
/// reading git, above the store, and by the time it gets here the branch has
/// already moved.
///
/// Whose the name is does not change with it. A Conversation that started on a
/// name Verkstead invented is still on one Verkstead is responsible for after a
/// session picked a better one, and one the human typed is still theirs; what
/// moves is the name itself, in whichever of the two columns is holding it.
///
/// What does end here is the waiting. A rename is the answer to the naming
/// instruction, so a Conversation still holding one is done holding it and the
/// name it has just moved to is what it is called from now on — see
/// [`Conversation::naming`]. A rename nobody was waiting for writes the same
/// nothing over the nothing already there.
pub async fn follow_branch(pool: &SqlitePool, id: i64, branch: &str) -> Result<()> {
    sqlx::query(
        "UPDATE conversations
         SET branch = ?,
             named_branch = CASE WHEN named_branch IS NULL THEN NULL ELSE ? END,
             naming = 0
         WHERE id = ?",
    )
    .bind(branch)
    .bind(branch)
    .bind(id)
    .execute(pool)
    .await
    .with_context(|| format!("following the renamed branch of Conversation {id}"))?;

    Ok(())
}

/// Record the commit a drafting Conversation branches from, or `None` to put it
/// back on the default-branch rule.
///
/// `None` is the ordinary case and not a cleared field: the design says the base
/// commit is the default branch's tip *at grill start*, so while drafting there
/// is no value to hold — only whether the human has overridden the rule.
///
/// Refused once the branch has been made, for [`rename_branch`]'s reason: the rule
/// resolved to a commit when the work branched, and a second round carries on
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
pub(crate) async fn not_drafting(pool: &SqlitePool, id: i64) -> Result<Option<Edited>> {
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
/// checkout are made, and forgotten only by closing.
///
/// Asked beside the state rather than instead of it, because the two answer
/// different questions: drafting is where the Conversation has got to, and this
/// is whether there is a branch behind it. What is refused off this is the
/// branch name and the base commit — a record that has both settled is not one
/// a field should rewrite, whatever its state column says.
pub(crate) async fn branch_made(pool: &SqlitePool, id: i64) -> Result<bool> {
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
/// It is also where the Repo remembers what it was started with — see
/// [`super::pairings::remember`] — because this is the moment the three roles
/// stop being changeable and become what the work is actually running under.
///
/// `companions` is where each of the Conversation's companion repos was checked
/// out, in the same transaction and for the same reason: they were made against
/// git before this was called, and a Conversation saying it is grilling without
/// saying where they went would be one nothing could bind into a sandbox and
/// nothing would come back and remove. Empty is the ordinary Conversation, which
/// has none.
pub async fn start_grilling(
    pool: &SqlitePool,
    id: i64,
    base_commit: &str,
    worktree: &Path,
    companions: &[super::CompanionWorktree],
) -> Result<Grilling> {
    start(pool, id, base_commit, worktree, companions, None).await
}

/// And the same start on a Conversation whose human picked *no grilling*: the
/// branch, the worktree, the base commit and the memory exactly as above, and
/// the Conversation lands Implementing rather than Grilling.
///
/// One press, two landings, and which of them is a fact about what was picked
/// rather than a second kind of start — see [`skip_grilling`]. Everything the
/// server did against git before calling either is the same work, so the record
/// of it is the same record.
///
/// The direction goes down with the move, because there is no grilling left to
/// propose one: what a Brief taken straight to the work is, is an inline
/// implementation, and a Conversation implementing with no direction is a record
/// nothing could resume — see [`pick_direction`], which is how the other way in
/// writes the same row.
pub async fn start_building(
    pool: &SqlitePool,
    id: i64,
    base_commit: &str,
    worktree: &Path,
    companions: &[super::CompanionWorktree],
) -> Result<Grilling> {
    start(
        pool,
        id,
        base_commit,
        worktree,
        companions,
        Some(Direction::Inline),
    )
    .await
}

/// What the two of them do, which is the same thing but for where it leaves the
/// Conversation.
///
/// `building` is the direction a start that skips the grilling records, and its
/// being there is also what says which state to land in: a start with a
/// direction has nothing to grill and is already building.
async fn start(
    pool: &SqlitePool,
    id: i64,
    base_commit: &str,
    worktree: &Path,
    companions: &[super::CompanionWorktree],
    building: Option<Direction>,
) -> Result<Grilling> {
    let worktree = super::repos::text(worktree)?;

    let landing = match building {
        Some(_) => Lifecycle::Implementing,
        None => Lifecycle::Grilling,
    };

    let mut tx = super::writing(pool, "starting a Conversation's work").await?;

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

    // And whether the branch still has to be named, which is settled by the same
    // press for the same reason the base commit is: the name Verkstead invented
    // was a prefill while this was a Draft and is the branch the work is on from
    // here, so this is the moment the first session inherits the job of picking
    // a better one. A Conversation the human named has nothing to wait for.
    sqlx::query(
        "UPDATE conversations
         SET base_commit = ?, state = ?, naming = (named_branch IS NULL)
         WHERE id = ?",
    )
    .bind(base_commit)
    .bind(landing.stored())
    .bind(id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("moving Conversation {id} to {landing:?}"))?;

    if let Some(direction) = building {
        sqlx::query(
            "INSERT INTO directions (conversation_id, direction) VALUES (?, ?)
             ON CONFLICT (conversation_id) DO UPDATE SET direction = excluded.direction",
        )
        .bind(id)
        .bind(direction_stored(direction))
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording how Conversation {id}'s work is being built"))?;
    }

    // Written over whatever is there rather than inserted: a record that somehow
    // holds a worktree already is corrected to the one just made, where an
    // insert would fail on it.
    sqlx::query(
        "INSERT INTO worktrees (conversation_id, path) VALUES (?, ?)
         ON CONFLICT(conversation_id) DO UPDATE SET path = excluded.path",
    )
    .bind(id)
    .bind(worktree)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the worktree of Conversation {id}"))?;

    super::companions::record_worktrees(&mut tx, id, companions).await?;

    moved(&mut tx, id, landing).await?;

    // And what it is being started with, against its Repo, so the next
    // Conversation started on that Repo arrives with every picker filled. In
    // this transaction because this is the moment the Pairings are fixed: a
    // memory written a moment later could be of a choice that never ran.
    super::pairings::remember(&mut tx, id).await?;

    tx.commit()
        .await
        .context("starting a Conversation's work")?;

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
///
/// And closing one whose state word this Verkstead cannot read works, which is
/// the one read of that column written to tolerate a bad word — see
/// [`Lifecycle::reads_as`]. The write below is unconditional, so the row comes
/// out of it holding `closed`: closing a Conversation nobody could read is also
/// what repairs it.
pub async fn close_conversation(pool: &SqlitePool, id: i64) -> Result<Closing> {
    let mut tx = super::writing(pool, "closing a Conversation").await?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Closing::NoSuchConversation);
    };

    // Tolerantly, unlike every other compare against this column: a word this
    // Verkstead cannot parse is not Closed, so the close goes ahead — and the
    // unconditional write below leaves `closed` where the bad word was, which
    // is the row healed. That is the point rather than a side effect. A
    // Conversation whose state column has gone bad is exactly the one a human
    // most needs to be able to end, and refusing here would be the one column
    // nothing could get past.
    if Lifecycle::reads_as(&state, Lifecycle::Closed) {
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

    // And every companion's beside it, for the same reason and by the same rule:
    // the directories are gone by the time this runs, and the branches their
    // read-write companions were worked on stay where they are.
    super::companions::forget_worktrees(&mut tx, id).await?;

    moved(&mut tx, id, Lifecycle::Closed).await?;

    tx.commit().await.context("closing a Conversation")?;

    Ok(Closing::Closed)
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
    let mut tx = super::writing(pool, "acting on a picked direction").await?;

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
    let mut tx = super::writing(pool, "starting the implementation").await?;

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
    let mut tx = super::writing(pool, "building the split-out work").await?;

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

/// Land a follow-up back in the wrap-up it was opened over, because the human
/// has said there is nothing else.
///
/// The way out of Follow-up, and the only one there is short of a steer. A
/// follow-up is something taken up about work that is already on a pull request,
/// so where it ends is where it started: the wrap-up carries on over whatever
/// the branch now holds, and *back to Done* is that wrap-up's own settling rule
/// rather than anything decided here — see [`finish_wrap_up`].
///
/// Refused for anything but Follow-up, as every move here is refused outside the
/// state it leaves: a Conversation closed or steered out from under the session
/// is not one to wrap up.
///
/// **The checks go back to waiting where the follow-up pushed**, in the same
/// transaction as the move. A follow-up that committed has given GitHub a new
/// run to make up its mind about, and the settle standing over it is yesterday's
/// green: without this the wrap-up's settling loop could reach Done in the gap
/// before the checks watcher's first poll saw the new run. `pushed` is the
/// caller's to know — it is what the Conversation recorded while the session ran
/// — and a pure question-and-answer follow-up that committed nothing lands with
/// everything settled and passes straight through.
///
/// The review's settle is deliberately left alone either way, which is the one
/// place this parts company with [`implement_again`]. *Settled once and stays
/// settled* is a rule about one wrap, and this is the same wrap: the human read
/// the branch, said what they wanted about it, and watched it done. A second
/// review of what they have just been through would be Verkstead reading over
/// their shoulder.
///
/// One transaction, as every move is: a Conversation that says Wrapping always
/// has the move on its Timeline to say when it got there.
pub async fn follow_up_over(pool: &SqlitePool, id: i64, pushed: bool) -> Result<Ending> {
    let mut tx = super::writing(pool, "ending a follow-up").await?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Ending::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::FollowUp {
        return Ok(Ending::NotFollowingUp);
    }

    if pushed {
        // Every pull request the work ended up on, rather than the one. A
        // Conversation ends on one per repository it was worked in and each has
        // a suite of its own, so a follow-up that pushed is a wrap-up whose
        // checks are all of them running again — and one left settled would be a
        // wrap-up finishing on a green nobody re-earned.
        let opened: Vec<(i64,)> =
            sqlx::query_as("SELECT repo_id FROM pull_requests WHERE conversation_id = ?")
                .bind(id)
                .fetch_all(&mut *tx)
                .await
                .with_context(|| format!("reading which pull requests Conversation {id} is on"))?;

        for (repo_id,) in opened {
            super::wrap_up::unsettle(&mut tx, id, super::WaitingOn::Checks(repo_id)).await?;
        }
    }

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Wrapping.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("moving Conversation {id} back to wrapping up"))?;

    moved(&mut tx, id, Lifecycle::Wrapping).await?;

    tx.commit().await.context("ending a follow-up")?;

    Ok(Ending::Wrapped)
}

/// Send a Done Conversation back to wrapping up, because the human pressed
/// **Resolve conflicts** on a pull request that will not merge.
///
/// The one way back up the ladder that is not a steer. A wrap-up ends when
/// GitHub can merge every pull request the work is on — and a base goes on
/// moving under a branch nobody is working on, so a Conversation Verkstead
/// finished with a week ago is a Conversation whose pull request conflicts
/// today. The sweep after Done writes that fact down and dispatches nothing
/// about it; this is the human reading it and asking for another round.
///
/// Refused for anything but Done, as every move here is refused outside the
/// state it leaves: a Conversation already wrapping up has the watchers on it
/// and needs no press, and one that has been closed since the pane was drawn is
/// not one to start work in.
///
/// **And refused where nothing conflicts**, which is the refusal this move has
/// and no other does. What is being asked for is a resolution, so a press that
/// found nothing to resolve would put the Conversation back to Wrapping to sit
/// there settled and sail to Done again — a round trip with a Notice's worth of
/// noise on the Timeline and nothing done. The button is drawn off the same
/// fact; this is the same rule asked again on arrival.
///
/// **The review's settle is left standing**, which is the whole difference
/// between this and a steer into Wrapping. A steer deliberately opens the branch
/// to be read again; this does not — the work was reviewed and carried to Done,
/// and a base that moved underneath it is not a reason to read it a second time.
/// So the wrap-up that starts here finds its review settled and runs no session
/// for it.
///
/// **What goes back to waiting is the merge**, and only on the pull requests the
/// record says conflict. A settle left standing over a conflict would be a
/// wrap-up that reached Done again on the first turn of its settling loop,
/// before the checks watcher's first poll had asked GitHub anything — so the
/// fact the press was offered off is the fact the wrap-up waits on. The checks
/// and the comments settle from the answers this round's own polls get, so
/// neither is touched.
///
/// Two Events, in the order the moment happened in and in a steer's own shape:
/// the human's line first, because somebody decided this, and the Moved line
/// under it because that is what came of the deciding. A long Timeline that said
/// only *Done → Wrapping* could never be read back for who put it there.
///
/// **A kind of its own rather than a Steer**, though, which is the difference
/// that matters when it is read back. A steer into Wrapping may carry no
/// instruction either, so the two would be the same row and the same line on the
/// page — and they are not the same act: a steer opens the branch to be read
/// again and this deliberately leaves the review's settle standing. Which of
/// them happened is the whole of what a reader wants to know months later, so
/// the record says which. See [`Event::ResolveConflicts`].
///
/// One transaction, as every move is: a Conversation that says Wrapping always
/// has the move on its Timeline to say when it got there.
pub async fn resolve_conflicts(pool: &SqlitePool, id: i64) -> Result<Resolving> {
    let mut tx = super::writing(pool, "resolving a conflict on a finished Conversation").await?;

    let row: Option<(String,)> = sqlx::query_as("SELECT state FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading the state of Conversation {id}"))?;

    let Some((state,)) = row else {
        return Ok(Resolving::NoSuchConversation);
    };

    if Lifecycle::read(&state)? != Lifecycle::Done {
        return Ok(Resolving::NotDone);
    }

    let conflicted = super::pull_requests::conflicted(&mut tx, id).await?;

    if conflicted.is_empty() {
        return Ok(Resolving::NothingConflicts);
    }

    for repo_id in conflicted {
        super::wrap_up::unsettle(&mut tx, id, super::WaitingOn::Mergeable(repo_id)).await?;
    }

    let pressed = Event::ResolveConflicts;

    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
    )
    .bind(id)
    .bind(pressed.kind())
    .bind(pressed.body().into_owned())
    .execute(&mut *tx)
    .await
    .with_context(|| format!("putting a resolve press on the Timeline of Conversation {id}"))?;

    sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
        .bind(Lifecycle::Wrapping.stored())
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("moving Conversation {id} back to wrapping up"))?;

    moved(&mut tx, id, Lifecycle::Wrapping).await?;

    tx.commit()
        .await
        .context("resolving a conflict on a finished Conversation")?;

    Ok(Resolving::Wrapping)
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
/// it is what came of the act.
///
/// **The Steer carries what the human wrote**, where a target takes anything
/// written: the instruction a steer into Implementing sends a session off with,
/// and the brief a steer into Follow-up does, are the Event's own body, so
/// reading the Event back is reading the job that was set. See [`Event::Steer`] for how the two are held in the one column.
///
/// **A third where the steer opens a round**: the Brief the human wrote for it,
/// under the move rather than above it, because the move is where the round
/// boundary falls and the Brief belongs to the round that starts there. Frozen
/// where it lands, the round it opens being past drafting, and a second Brief
/// Event beside the first rather than an edit of it: what the earlier round was
/// built from stays on the record.
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
/// Empty is the ordinary case twice over: a target nothing runs in has no
/// Pairing to settle, and a human who left the picker on what the Conversation
/// already had has changed none. More than one is a target whose sessions run
/// under more than one role — a wrap-up builds its fixes and reviews the work —
/// which the human's one pick settles together.
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
/// **And the companions the steer widened the set with and the ones it opened
/// up**, their rows and their checkouts, and a line under the Steer naming what
/// came in and what was opened. Past drafting is exactly where this writes, so
/// none of it goes through the setup card's own guarded writes — see
/// [`super::companions::join`] and [`super::companions::open_up`]. One direction
/// only: nothing here takes a companion away, puts one back to read-only, or
/// writes an add over a row that is already there.
///
/// Nothing about the run is touched, and what has to stop running is stopped
/// before this is called — see the server's `steering` module, which is the only
/// caller.
pub async fn steer_conversation(pool: &SqlitePool, id: i64, steer: Steer<'_>) -> Result<Steering> {
    let Steer {
        target,
        pairings,
        brief,
        instruction,
        direction,
        worktree,
        base_commit,
        companions,
        opened,
        checkouts,
        said,
    } = steer;

    let mut tx = super::writing(pool, "steering a Conversation").await?;

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

    // And what came into the sandbox with it, directly under the human's own
    // line. The Steer says a person moved this; this says which repositories
    // moved with it, and it belongs to the Steer rather than to the move — what
    // a Conversation is configured with is read on the Brief's details pane ever
    // after, and this is what says when the set changed and who changed it.
    if let Some(said) = said {
        let notice = Event::Notice(said.to_owned());

        sqlx::query(
            "INSERT INTO timeline_events (conversation_id, at, kind, body)
             VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)",
        )
        .bind(id)
        .bind(notice.kind())
        .bind(notice.body().into_owned())
        .execute(&mut *tx)
        .await
        .with_context(|| format!("saying what a steer of Conversation {id} took in"))?;
    }

    // Before the move rather than after it, because what it asks about is where
    // the steer found the Conversation. A Draft steered into a state something
    // runs in is work starting on a branch nobody has named, exactly as a grill
    // start is — so its first session is the one told to name it. Every other
    // source has had its first session already, whatever its branch ended up
    // being called.
    //
    // Into Done it is not asked at all: nothing is started there, so nothing
    // would ever be along to answer it and the Conversation would read *Draft*
    // for the rest of its life. What it is called is the name its branch was cut
    // on, which is the only name it is ever going to have.
    if target != Lifecycle::Done {
        sqlx::query(
            "UPDATE conversations SET naming = (named_branch IS NULL)
             WHERE id = ? AND state = ?",
        )
        .bind(id)
        .bind(Lifecycle::Draft.stored())
        .execute(&mut *tx)
        .await
        .with_context(|| format!("leaving the branch of Conversation {id} to be named"))?;
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

    // And the companions the steer widened the set with, the ones it opened up,
    // and where every checkout it made went — the ones just added, the ones
    // whose detached directory was replaced, and every one the record held with
    // no directory behind it. All in the move's own transaction, for the reason
    // the Worktree above is: a Conversation that said it had moved without
    // saying which repositories moved with it, and how far into each the work
    // now reaches, would be one nothing could bind into a sandbox correctly and
    // nothing would come back and remove.
    super::companions::join(&mut tx, id, companions).await?;
    super::companions::open_up(&mut tx, id, opened).await?;
    super::companions::record_worktrees(&mut tx, id, checkouts).await?;

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
    // its wrap-up bookkeeping goes with it: a round that inherited the one before
    // it would reach Wrapping with everything wrap-up waits on already settled,
    // and would be over the moment it arrived. See
    // [`super::wrap_up::forget_the_round`].
    if target == Lifecycle::Grilling {
        super::wrap_up::forget_the_round(&mut tx, id).await?;
    }

    moved(&mut tx, id, target).await?;

    // The new round's Brief under the move, because the move is where the round
    // boundary falls and the Brief under it belongs to the round that starts
    // there. Frozen from the moment it lands — the round it opens is past
    // drafting, which is the only state [`save_brief`] will edit one in — and a
    // second Brief Event beside the first rather than an edit of it.
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

    for pairing in pairings {
        if !settle(
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
/// Everything but the target is absent in the ordinary case, and each absence
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

    /// What the work runs under from here, where they picked something new —
    /// one entry per role the state steered into runs its sessions under, so a
    /// target that both builds and reviews settles both from the one pick.
    ///
    /// Empty is the ordinary case twice over: a target nothing runs in settles
    /// nothing, and a human who left the picker on what the Conversation
    /// already had has changed nothing.
    pub pairings: &'a [Settling<'a>],

    /// The new round's Brief, for a steer that opens one.
    pub brief: Option<&'a str>,

    /// What the human wrote to steer it with: the instruction a steer into
    /// Implementing carries, or the brief a steer into Follow-up does. Either
    /// lands as the Steer Event's own body rather than beside it.
    ///
    /// Not a Brief, however alike the three look on the page. A Brief is what a
    /// round is grilled *about*; this is what one session was set going on,
    /// said by the human at the moment they steered — so it belongs to the
    /// steer, and reading the Event back is reading what they asked for.
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

    /// The companions the steer is putting on, which are rows this writes and
    /// nothing else could.
    ///
    /// Past drafting is where a steer writes, so this does not go through the
    /// setup card's own guarded writes — see [`super::companions::join`]. Empty
    /// is the ordinary case: most steers widen nothing.
    pub companions: &'a [super::Joining<'a>],

    /// And the companions it opened up, which are rows this moves and nothing
    /// else could.
    ///
    /// The same absent guard for the same reason, and one direction only: each
    /// of these goes to read-write with the branch it was given, and nothing
    /// here puts one back — see [`super::companions::open_up`]. Empty is the
    /// ordinary case: most steers open nothing up.
    pub opened: &'a [super::Opening<'a>],

    /// And where every companion checkout the steer made went, which is the ones
    /// just added, the ones it opened up, and every one the record held with no
    /// directory behind it.
    ///
    /// In the same transaction as the move, for the reason the Conversation's
    /// own Worktree is: one that said it had moved without saying where its
    /// companions went would be one nothing could bind into a sandbox and
    /// nothing would come back and remove.
    pub checkouts: &'a [super::CompanionWorktree],

    /// What the steer has to say about the set it widened, for the Timeline.
    ///
    /// Under the Steer Event rather than beside it: the Steer is the human's
    /// own line — *I moved this* — and this is what came into the sandbox with
    /// it. `None` is a steer that widened nothing, which is a steer with nothing
    /// to announce.
    pub said: Option<&'a str>,
}

/// A Pairing a steer settles: which of the roles, and both halves of the
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
/// grilling round rather than one ever — a steered round grills again, and its
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

/// What became of stamping the row that fixes where a list landed.
///
/// Three answers rather than a `bool`, because the middle one is not a failure
/// and is not a write either: a run that is seen out twice, or one taken up
/// again after a stop, reaches the same landing a second time and finds the row
/// already on the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landed {
    /// The row is on the Timeline, written now.
    Stamped,

    /// It was already there, so nothing was written. A list lands once.
    Already,

    /// There is no Conversation with that id.
    NoSuchConversation,
}

/// Put the row that says a list landed on this Conversation's branch — the
/// backlog, or the roadmap — on its Timeline.
///
/// The row carries nothing: what the Timeline draws at it is the list as the
/// Worktree holds it now, read at the moment somebody looks. What it fixes is
/// *where* — the moment the plan became something to work through, which the
/// runner is watching for anyway in order to move the Conversation on.
///
/// One transaction, and the looking is inside it, which is what makes a second
/// go safe: the state is read and acted on without a gap for another to write
/// through — the pattern [`super::record_pull_request`] uses one Event along.
///
/// Nothing is backfilled. A Conversation whose backlog landed before there was a
/// row to stamp has none, and draws its list in the pinned block alone.
async fn landed(pool: &SqlitePool, id: i64, event: Event) -> Result<Landed> {
    let kind = event.kind();

    let mut tx = super::writing(pool, "recording what landed on a Conversation").await?;

    let conversation: Option<(i64,)> = sqlx::query_as("SELECT id FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("reading Conversation {id}"))?;

    if conversation.is_none() {
        return Ok(Landed::NoSuchConversation);
    }

    let stamped: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM timeline_events WHERE conversation_id = ? AND kind = ? LIMIT 1",
    )
    .bind(id)
    .bind(kind)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("looking for the {kind} row of Conversation {id}"))?;

    if stamped.is_some() {
        return Ok(Landed::Already);
    }

    sqlx::query(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, '')",
    )
    .bind(id)
    .bind(kind)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("putting the {kind} row of Conversation {id} on its Timeline"))?;

    tx.commit()
        .await
        .with_context(|| format!("recording that {kind} landed on Conversation {id}"))?;

    Ok(Landed::Stamped)
}

/// The backlog landed: the breakdown committed `.tasks/`, and there is a list to
/// work through from here.
///
/// Written where the runner sees that landing, which is the same moment it moves
/// the Conversation on — see `crate::runner` in the server.
pub async fn record_backlog(pool: &SqlitePool, id: i64) -> Result<Landed> {
    landed(pool, id, Event::TaskList).await
}

/// And the roadmap landed: the staging session committed `docs/roadmaps/`, and
/// the stages it names are what the effort is now against.
pub async fn record_roadmap(pool: &SqlitePool, id: i64) -> Result<Landed> {
    landed(pool, id, Event::StageList).await
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
///
/// `companions` is where the stage's inherited companion repos were checked
/// out, written in this transaction for [`start_grilling`]'s reason: a stage
/// that said it was implementing with companions nothing had checked out would
/// be one nothing could bind into a sandbox and nothing would come back and
/// remove.
pub async fn start_stage(
    pool: &SqlitePool,
    id: i64,
    base_commit: &str,
    worktree: &Path,
    stacks_on: Option<&str>,
    companions: &[super::CompanionWorktree],
) -> Result<Staged> {
    let worktree = super::repos::text(worktree)?;

    let mut tx = super::writing(pool, "starting a stage").await?;

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

    super::companions::record_worktrees(&mut tx, id, companions).await?;

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
