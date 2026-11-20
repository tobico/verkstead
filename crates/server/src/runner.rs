//! The runner: what the host does around the sessions a Conversation's work is
//! carried by, once a direction is picked.
//!
//! **What is next is no longer decided here.** A backlog is worked by the
//! **Backlog runner** Activity and a roadmap's stages are started by the
//! **Stage runner**, both components of the built-in Activities plugin — see
//! [`backlog_runner`], [`crate::continuing`] and [`crate::activities`]. They
//! read the boxes and the declarations off the Repo themselves, through the
//! host functions [`crate::lending`] lends them. What stays here is what those
//! host functions and the other drivers stand on: launching a session and
//! watching it to its signal, checking a signal against the Repo, and carrying
//! work on to its pull request.
//!
//! **What a finished step looks like** is [`Landing`]: an entry ticked in
//! `TODO.md`, or a path gone from the Worktree or arrived in it — and, either
//! way, committed as it stands. A box ticked but not committed is a session
//! still mid-task, and a commit is the one report an agent cannot half make.
//! The read never takes `index.lock` — everything here goes through
//! [`crate::repos::git`], which passes `--no-optional-locks`, because what it is
//! reading is a repository a session is committing in, and a read that tripped
//! the session's own `git add` would break the step it is reading.
//!
//! **When a step is over is the session's to say** — see [`crate::done`], and
//! ADR-0018. Every session here is an ordinary interactive agent, which idles
//! when its work is over rather than exiting, so something has to end it, and
//! what ends it is its `verkstead done`. The landing is what that signal is
//! checked against, at that moment and only then: a signal the branch does not
//! bear out is refused, and a session that commits and then waits on its tests,
//! or asks a question, is a session still at work however long it is quiet.
//! Once the signal is taken the session is ended when it is next idle, and has
//! been for [`Pace::grace`], so what it says on the way out is kept — see
//! [`signalled_and_idle`].
//!
//! What differs between the kinds of session is only what their signal is
//! checked against — see [`crate::done::Evidence`]. A step's is its landing, an
//! inline run's or an instruction's a new commit, a follow-up's the human's own
//! *Nothing else* mark. A propose-then-fix session — the wrap-up's review, and
//! each batch of comments — reports through nothing but its own words, and a
//! fix through a check GitHub is asked about once it is over, so theirs is
//! checked against nothing of its own: see [`proposing`] and [`address`].
//!
//! A step whose session exits by itself without landing it stops the run where
//! it is, and what it stops at is a **stop**: the Conversation records that
//! nothing is driving it any more, and a Notice carrying the evidence goes on
//! the Timeline — see [`crate::stopping`]. The repository is asked once first,
//! because an agent that landed its step and then exited has done the step. The
//! run does not go round again from a stop; getting going is a press of Resume,
//! because a runner that relaunched a step nothing had moved would spend an
//! account on the same failure with nobody watching.
//!
//! **And a session that never signals is spoken to, and never stopped over.**
//! One idle with nothing open, no wait declared and no signal given has not
//! finished as far as anything here can tell: it is sitting there with the turn
//! over and no way of knowing that the screen it printed to has nobody in front
//! of it. So it is told, and where it will not answer the human is told
//! instead, with the session left running for them — see [`crate::rescues`],
//! which is one loop over every driver here and takes the signal as its
//! parameter.
//!
//! **The pull request is the one thing that gets a second go.** Every run here
//! ends on one — a backlog's finish step, an inline implementation, a roadmap's
//! own session — and each of them commits its work and then pushes and opens the
//! pull request after the commit. A session that signals before the pull request
//! is open is refused for it, so it puts that right in the same turn; but one
//! that exits by itself can still land everything it was sent for and stop short
//! of the one act that makes the work readable, leaving it built, committed and
//! unreviewable. That is not a step to run again: it is one push and one
//! `gh pr create`, so it is asked for on its own, by a session sent for nothing
//! else, and what follows is GitHub asked again and the ordinary stop where the
//! answer has not changed. See [`to_a_pull_request`].
//!
//! **And Resume takes the same go**, which is what makes pressing it worth
//! anything here: a run that stopped at its push is a Conversation whose work is
//! built and whose branch is on nothing, and the press finds exactly that and
//! sends for the pull request again. What Resume must never do is guess — an
//! empty `.tasks/` is a finished backlog or one that never landed, and those are
//! opposite situations — so the branch is read for which it is. See
//! [`nothing_left`].

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use verkstead_render::Resumed;
use verkstead_schema::{Direction, Nudge};

use crate::AppState;
use crate::drivers::Driving;
use crate::github;
use crate::investigations::Investigation;
use crate::lending::Lent;
use crate::repos::git;
use crate::sessions::{Ended, Idle};
use crate::skills;
use crate::store;
use crate::tasks::{BACKLOG, TODO};

/// How fast the runner works a backlog.
///
/// Public because [`crate::Agents`] carries one and a caller standing a server
/// up chooses it — see [`Agents::pace`](crate::Agents) for why that is a choice
/// at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pace {
    /// How often the Worktree is asked whether the step has landed.
    ///
    /// A commit arrives minutes apart at best, so this is not a race to notice
    /// one: it is how long the next task waits to start. Two seconds costs a
    /// short git read a couple of times a minute.
    pub poll: Duration,

    /// How long a session that has said it is done must have printed nothing
    /// before it is ended.
    ///
    /// Counted from the signal as well as from the last thing printed, so the
    /// closing words an agent writes once `verkstead done` comes back reach the
    /// Transcript rather than being cut off under it — see
    /// [`signalled_and_idle`]. Nothing is read off the branch while it runs: the
    /// signal was checked when it was given, and this is only the pause that
    /// keeps the last few lines.
    pub grace: Duration,

    /// How often a wrapping Conversation's pull request is asked about.
    ///
    /// Here rather than beside the backlog's two because it is the same kind of
    /// choice one phase along, and a caller standing a server up sets all of
    /// them at once — see [`crate::checks::ASKED_EVERY`] for what it costs.
    pub checks: Duration,

    /// And how long a session must have been idle — with nothing open, no wait
    /// declared and no Done signal given — before the Rescue speaks to it; see
    /// [`crate::rescues`].
    ///
    /// The Rescue's grace, and the whole of what this is: no session is ended on
    /// quiet any more, so the only thing silence buys is a line typed into the
    /// session. Named for the propose-then-fix sessions it was once the ending of
    /// as well — see ADR-0018 for why that ending went.
    ///
    /// Distinctly longer than [`Pace::grace`], because a line typed into a
    /// session still at work costs it a turn: a minute of silence is the shortest
    /// an agent still at work reliably breaks.
    pub proposing: Duration,

    /// And how long a session that has just been stirred — launched, handed an
    /// answer, or typed a rescue into — is given to say its first word before
    /// silence alone is enough to rescue it.
    ///
    /// The ceiling on the hold-off in [`crate::rescues`]. What carries an
    /// answer to a session is a chain Verkstead cannot see a single hop of —
    /// the CLI's long poll returning, the harness noticing, the model taking
    /// its turn, the first bytes drawn — and a chain slower than
    /// [`Pace::proposing`] used to look exactly like a session that had gone
    /// quiet without asking. So a stirred session is left alone until it
    /// speaks. This is where that ends: one that has said nothing at all for
    /// this long is one that died mid-wait, and it is rescued having never
    /// spoken.
    ///
    /// Several times [`Pace::proposing`], because it is the outer bound on a
    /// wake rather than a measure of one. The ordinary case is the session
    /// speaking within a second or two of the answer; all this decides is how
    /// long one that never will is left before Verkstead says so.
    pub waking: Duration,

    /// And how long a session judged on the screen it draws may print nothing
    /// before it is idle whatever that screen says.
    ///
    /// The long-stop behind a signature read off the screen — see
    /// [`crate::sessions::Idle`]. Five minutes: minutes rather than seconds,
    /// because a backend that repaints leaves gaps a few seconds long in the
    /// middle of its work and a session reaped inside one would be reaped at
    /// work. What it is for is a signature that has drifted, which is a session
    /// nothing else here catches — Rescue's precondition is idle, the ending a
    /// signal earns waits on the same judgement, and no session carries a cap on
    /// its life —
    /// so what this decides is not how fast that is noticed but whether it ever
    /// is.
    ///
    /// Nothing to a session judged on what it prints, which is measured by
    /// [`crate::sessions`]'s own three seconds and never by this.
    pub long_stop: Duration,

    /// And how long a wrap-up's review waits before it takes the Worktree.
    ///
    /// Zero in a server, where nothing is waiting for anything: this is a seam
    /// the suite reaches for, and the one span here that is not a choice about
    /// how fast to work. A wrap-up starts its review and its watchers together,
    /// and the poll that runs before the review has the Worktree is the poll
    /// that can read a pull request nobody has written on yet — the window a
    /// comment left during the review used to be settled away in. Nothing can
    /// land in that window on purpose, so a test that covers it holds it open
    /// instead. See [`crate::comments::once`], which settles nothing before the
    /// review for exactly that reason.
    pub reviewing: Duration,

    /// And how often a Done Conversation's pull requests are asked about, which
    /// is the slow sweep that takes over from the watcher above — see
    /// [`crate::merges`].
    ///
    /// Here beside the rest for [`Pace::stalls`]'s reason: a caller standing a
    /// server up chooses how often Verkstead looks at things, and a pull request
    /// waiting to be merged is one of the things it looks at.
    pub merges: Duration,

    /// And how often the archived Conversations are looked over for one with
    /// a cleanup due on it — see [`crate::cleanup`].
    ///
    /// Here for [`Pace::merges`]'s reason again, and slower than any of the
    /// rest: what it is watching is a clock counted in days.
    pub cleanup: Duration,

    /// And how often the languages' stores are swept under their sizes — see
    /// [`crate::eviction`].
    ///
    /// Here for [`Pace::merges`]'s reason again. How often a pass comes due
    /// rather than when it runs: one due while anything runs waits for the
    /// first moment nothing does.
    pub evicting: Duration,

    /// And how often a stage held before its finish looks at whether the chain
    /// below it has settled — see [`crate::joins`].
    ///
    /// Here beside the rest for [`Pace::checks`]'s reason: a caller standing a
    /// server up chooses how often Verkstead looks at things, and the chain a
    /// stage is waiting to join is one of the things it looks at.
    pub joins: Duration,

    /// And how often every Conversation is looked over for one that has
    /// Stalled — see [`crate::stalls`].
    ///
    /// Here beside the rest for [`Pace::checks`]s reason rather than because a
    /// sweep is anything the runner does: a caller standing a server up chooses
    /// how often Verkstead looks at things, and a stall is one of the things it
    /// looks for.
    pub stalls: Duration,

    /// And how long the roadmaps being driven go unlooked-at for a stage waiting
    /// on a place the whole server has, where **nothing frees one** — see
    /// [`crate::places`].
    ///
    /// Here for [`Pace::stalls`]'s reason again, and it is the one of these that
    /// decides how long work waits rather than how long a report does: what comes
    /// of a look is a stage started. The backstop rather than the pace, though —
    /// what ordinarily brings a look about is a place coming free, which this
    /// says nothing about and does not wait for.
    pub places: Duration,

    /// And how often a stage away from home looks for a report its home has not
    /// taken yet — see [`crate::reporting`].
    ///
    /// The backstop rather than the pace, as [`Pace::places`] is: a move is told
    /// home the moment it is made, and home coming back is heard on its Nudge
    /// stream. What this catches is a home that answered neither.
    pub reports: Duration,

    /// And how often a sleeping device that an open placement Set of
    /// Verkstead's is waiting on is asked whether it is awake — see
    /// [`crate::own_sets::wait`].
    ///
    /// The backstop rather than the pace, as [`Pace::reports`] is: a dial
    /// finding a member answering hurries the look. What this catches is a
    /// machine that woke while nothing here happened to dial it.
    pub wakes: Duration,
}

impl Default for Pace {
    fn default() -> Pace {
        Pace {
            poll: Duration::from_secs(2),
            grace: Duration::from_secs(5),
            checks: crate::checks::ASKED_EVERY,
            proposing: Duration::from_secs(60),
            waking: Duration::from_secs(300),
            long_stop: Duration::from_secs(300),
            stalls: crate::stalls::SWEPT_EVERY,
            places: crate::places::LOOKED_AT_EVERY,
            reports: crate::reporting::LOOKED_AT_EVERY,
            wakes: crate::own_sets::WAKES_LOOKED_FOR_EVERY,
            joins: crate::joins::LOOKED_AT_EVERY,
            merges: crate::merges::SWEPT_EVERY,
            cleanup: crate::cleanup::SWEPT_EVERY,
            evicting: crate::eviction::SWEPT_EVERY,
            reviewing: Duration::ZERO,
        }
    }
}

/// The step a grilling's own session is carrying on into, or a roadmap's
/// session is launched for: the three a pick leaves to the runner rather than to
/// an Activity. A task and the finish are the **Backlog runner**'s — see
/// [`backlog_runner`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    /// Break the work down, which is where a task-list Conversation starts:
    /// there is no backlog yet, and the session writing one is already running.
    ///
    Planning,

    /// Write the handoff, which is the whole of what an inline grilling has left
    /// to do once the human has picked: the work itself runs under the other
    /// Profile in a session of its own, so everything this one settled has to be
    /// written down before it ends.
    ///
    /// Carries the path the document is watched for, which is outside the
    /// Worktree — Verkstead's own directory rather than the project's, so that
    /// no `git add -A` after it can sweep the document into the human's
    /// repository. See [`crate::handoffs`].
    Handoff(PathBuf),

    /// Write the roadmap, which is where a roadmap Conversation starts and
    /// finishes: the stages it plans are Conversations of their own.
    ///
    /// Carries the commit the branch came off, because what says this step is
    /// over is a roadmap on the branch *that was not there before* — see
    /// [`Landing::Roadmap`].
    Staging(String),
}

/// What says a step is over: a path in the Worktree that has to have gone, or
/// one that has to have arrived — and, either way, be committed as it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Landing {
    Gone(PathBuf),
    Arrived(PathBuf),

    /// The entry with this number, ticked off in `.tasks/TODO.md` and committed
    /// as it stands. What says a task is done — the same box the pinned card
    /// draws, so the list the human is watching and the step the runner is
    /// waiting on cannot disagree.
    Ticked(u32),

    /// A roadmap this branch has written, committed as it stands. The commit
    /// the branch came off, because `docs/roadmaps/` is a directory a
    /// repository often has already — a path arriving would read as landed
    /// before the session had written a line.
    Roadmap(String),

    /// The handoff document, written in the Conversation's own directory outside
    /// the Worktree. An absolute path rather than one inside it, and nothing
    /// asked of git: what version control would say about a file the repository
    /// has never heard of is nothing at all.
    Handoff(PathBuf),
}

impl Step {
    /// What would say this step is over.
    fn landing(&self) -> Landing {
        match self {
            // The plan commit is what puts the backlog under version control,
            // so the backlog being there and committed is the breakdown done.
            Step::Planning => Landing::Arrived(todo()),
            // And the handoff is the document itself, in a directory no commit
            // reaches: nothing puts it under version control, so its being there
            // is the whole of the signal.
            Step::Handoff(path) => Landing::Handoff(path.clone()),
            // The roadmap commit is what puts the stages under version control.
            Step::Staging(base) => Landing::Roadmap(base.clone()),
        }
    }

    /// Whether the step's session is done only once its branch has a pull
    /// request open as well: a roadmap's own session, which the run ends on. See [`crate::done::Ends`].
    fn ends(&self) -> crate::done::Ends {
        match self {
            Step::Staging(_) => crate::done::Ends::OnAPullRequest,
            Step::Planning | Step::Handoff(_) => crate::done::Ends::WithItsWork,
        }
    }

    /// What the step was, in the words the Timeline draws.
    ///
    fn what(&self) -> String {
        match self {
            Step::Planning => "breaking the work down into a backlog".to_owned(),
            Step::Handoff(_) => "writing the handoff for the session that builds".to_owned(),
            Step::Staging(_) => "staging the work into a roadmap".to_owned(),
        }
    }
}

/// The backlog's list, as a path inside a Worktree.
fn todo() -> PathBuf {
    Path::new(BACKLOG).join(TODO)
}

/// Run a grilling under the **Interview** Activity, and carry the Conversation
/// on wherever the Direction picked says.
///
/// The component launches the grilling session on `verkstead/grilling` under
/// the Grilling role, with `prompt` — the [`skills`] builder's words, a fresh
/// grilling's or a relaunch's with its digest — and waits on it expecting
/// `picked`: nothing before the human picks, and after it the picked
/// Direction's artifact, which [`tail`] says. The proposals the session puts are
/// the host's to settle, as ever — see
/// [`crate::conversations::settle_a_proposal`]. Breaking down and staging go on
/// inside the same session, so the interview is one session and launches no
/// second.
///
/// The three Directions differ in what the artifact is and in where landing it
/// leaves the Conversation, which is this function's mapping until a Process
/// says it (stage 11). A backlog is the start of a run — the tasks it names are
/// worked one fresh session each, under the Profile that builds — so the
/// Conversation goes on to Implementing. The handoff an inline pick asks for
/// leaves it in the same place, one session rather than a run standing on the
/// other side of it. A roadmap is the whole of this Conversation's own work,
/// because the building belongs to the Stages it plans, so the same session
/// carries the branch to a pull request and the Conversation goes straight on to
/// wrapping that up.
///
/// `lent` is the host the component is lent, carrying what only an in-process
/// caller can say: that nothing else is driving a grilling just started, or the
/// Turn a relaunch took and the Sets it holds open.
///
/// **A grilling is driven by its session until the human picks** — see
/// [`crate::drivers`] — so `driving`, the press's registration, is held across
/// the launch alone, and a session that dies before a pick leaves a
/// Conversation the stall sweep finds. The pick is what takes a registration of
/// the host's own, held through to wherever the Direction leaves the
/// Conversation.
///
/// **It offers the Directions declared for the Conversation** — all three until
/// a Process narrows them (stage 11), which the suite stands in for through
/// [`crate::followers::Followers::declare`] — handed to the component as its
/// `offers` start fact, read back by the host for what the chooser draws, and
/// said to the session under its prompt where they are fewer than three.
///
/// Whether a session was launched at all, which is what a relaunch holding Sets
/// open owes them on: see [`crate::grillings::again`]. Answered once the
/// interview is over.
pub(crate) async fn interview(
    state: AppState,
    conversation_id: i64,
    driving: Driving,
    prompt: String,
    lent: Lent,
) -> bool {
    let Some((shipped, activity)) = built_in(INTERVIEW) else {
        tracing::error!(
            conversation_id,
            "this build carries no Interview Activity, so no grilling session was started"
        );
        return false;
    };

    let offers = state.followers.declared(conversation_id);
    let words: Vec<&str> = offers.iter().map(|direction| direction.word()).collect();

    let (outcome, seen) = crate::activities::run_seeing(
        &state,
        conversation_id,
        shipped.files(),
        &activity,
        &[],
        vec![
            ("prompt".to_owned(), skills::offering(prompt, &offers)),
            ("offers".to_owned(), words.join(",")),
        ],
        lent.doing("grilling the work").launching(driving),
    )
    .await;

    let Some(writing) = seen.event_id else {
        return false;
    };

    // Taken as the pick was, so there is no moment between the session being
    // seen out and the move where nothing drives the Conversation.
    let driving = seen
        .driving
        .unwrap_or_else(|| state.drivers.driving(conversation_id));

    // The Outcome, or — where a stop landed as the artifact did, and took the
    // Outcome away — the Direction whose artifact was written: a step that
    // landed is landed, so it is taken, and the launch past it is what asks
    // after the stop.
    let picked = match outcome.as_deref() {
        Some(outcome) => outcome.strip_prefix("direction:"),
        None => seen.picked.as_deref(),
    };

    let direction = match picked {
        Some("inline") => Direction::Inline,
        Some("task-list") => Direction::TaskList,
        Some("roadmap") => Direction::Roadmap,
        // Steered elsewhere, which is the steer's to carry on; or gone before
        // anything was picked, which leaves the Conversation to the stall sweep
        // as a grilling session dying always has; or stopped.
        _ => return true,
    };

    tracing::info!(
        conversation_id,
        event_id = writing,
        ?direction,
        "a grilling session wrote what the pick asked for and has gone quiet since, so it is being ended",
    );

    state.sessions.end(conversation_id).await;

    match direction {
        Direction::Inline => handed_off(state, conversation_id, driving).await,
        Direction::TaskList => broken_down(state, conversation_id, driving).await,
        Direction::Roadmap => {
            if let Some(base) = base(&state, conversation_id).await {
                staged(&state, conversation_id, &base, writing).await;
            }
        }
    }

    true
}

/// The id the **Interview** Activity has in the built-in Activities plugin's
/// manifest.
const INTERVIEW: &str = "interview";

/// What the grilling session's Done signal is checked against once `direction`
/// is picked: the step its artifact is, read for the Conversation as it stands.
///
/// `None` where there is nothing to check it against — no Worktree left, or a
/// roadmap pick on a Conversation with no base commit, which nothing could say a
/// roadmap is this branch's own against.
pub(crate) async fn tail(
    state: &AppState,
    conversation_id: i64,
    direction: Direction,
) -> Option<Tail> {
    let step = match direction {
        Direction::Inline => Step::Handoff(
            crate::handoffs::Handoffs::under(&state.data_dir).document(conversation_id),
        ),
        Direction::TaskList => Step::Planning,
        Direction::Roadmap => Step::Staging(base(state, conversation_id).await?),
    };

    let worktree = worktree(state, conversation_id).await?;
    let landing = step.landing();

    Some(Tail {
        evidence: crate::done::Evidence::Landed {
            worktree: worktree.clone(),
            landing: landing.clone(),
        },
        ends: step.ends(),
        doing: step.what(),
        worktree,
        landing,
    })
}

/// A picked Direction's artifact, as the wait on the grilling session holds its
/// Done signal to it — see [`tail`].
pub(crate) struct Tail {
    pub(crate) evidence: crate::done::Evidence,
    pub(crate) ends: crate::done::Ends,

    /// What the rescue and a stop say the session is doing.
    pub(crate) doing: String,

    /// Where the artifact lands, asked once more of a session that ended.
    pub(crate) worktree: PathBuf,
    pub(crate) landing: Landing,
}

/// Carry a task-list Conversation on from its committed backlog: the run.
///
/// The plan commit is the end of the planning as well as the start of the run, so
/// the Conversation moves here: grilling until then and implementing afterwards
/// — see [`crate::conversations::grilling_over`]. No handoff is taken or written
/// on this path: the backlog is what the grilling settled, committed to the
/// branch, and every session that works it reads the repository rather than a
/// summary of a conversation it never had.
///
/// Returns when there is nothing left to run: the backlog worked through, a
/// Conversation that has gone, or a step whose session ended without landing it.
/// Idle rather than looping — a runner that relaunched a session for a step
/// nothing had moved would be a machine spending an account on the same failure
/// over and over, with nobody watching. What it leaves behind for the human is a
/// stop, and the Notice saying what stopped.
///
/// `driving` is the registration that says this Conversation is being driven,
/// taken by whoever started the interview rather than here — see
/// [`crate::drivers`] for why it is handed over rather than taken again.
async fn broken_down(state: AppState, conversation_id: i64, driving: Driving) {
    // The backlog is on the branch, so the record says where that happened —
    // before the move, because it is what the move is being made on the
    // strength of.
    crate::conversations::backlog_landed(&state, conversation_id).await;

    crate::conversations::grilling_over(&state, conversation_id).await;

    backlog_runner(state, conversation_id, driving).await
}

/// Start driving a stalled implementation again, from wherever the repository
/// now stands.
///
/// What Resume means where the Conversation it was pressed on is Implementing —
/// see [`crate::resume`], which is what decides that and hands it here. Which
/// run stopped is the direction's to say, and each of the three picks up from
/// what the repository now holds: the backlog off `.tasks/`, an inline run in a
/// fresh session, a roadmap off what the branch has written.
///
/// Nothing is decided from what stopped, because it knows nothing worth having:
/// a run that stopped with nothing running left no step to read and no session's
/// last words to go on. What there is, is the repository — which is the same
/// thing every turn of an ordinary run asks.
///
/// The reading here is the second one: the press has already asked what there is
/// to work in order to refuse by name where there is nothing. Read again rather
/// than carried, because a spawn is a moment later and where an agent is about
/// to be let loose is the one thing that must not be guessed at.
///
/// The registration is handed on rather than taken again, so a Conversation the
/// human has just pressed Resume on is driven from that moment rather than from
/// whenever the launch gets round to it — see [`crate::drivers`].
pub(crate) async fn implementing_again(state: AppState, conversation_id: i64, driving: Driving) {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(conversation_id, "there is no Conversation left to work in");
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to work in failed");
            return;
        }
    };

    // A Conversation is implementing because a direction was chosen, so a
    // missing one is a record that cannot be true — and the one thing that says
    // which run it is that stopped.
    let Some(direction) = conversation.direction else {
        tracing::error!(
            conversation_id,
            "a Conversation that says it is implementing has no direction recorded, \
             so there is nothing to say what stalled"
        );
        return;
    };

    let Some(working_in) = conversation.worktree else {
        tracing::info!(
            conversation_id,
            "the Conversation has no Worktree left, so nothing was started again"
        );
        return;
    };

    match direction {
        Direction::Inline => inline_again(state, conversation_id, driving).await,
        Direction::Roadmap => roadmap_again(state, conversation_id, &working_in, driving).await,
        Direction::TaskList => {
            let base = conversation.base_commit.as_deref();

            // A list, ticked or not, is the **Backlog runner**'s: it asks the
            // boxes what is next exactly as every other turn of the run does —
            // a stage's backlog included, it being a backlog like any other by
            // the time there is one.
            if anything_to_work(&working_in).await {
                return backlog_runner(state, conversation_id, driving).await;
            }

            // An empty one is three situations rather than one. A stage whose
            // planning never landed is answered here, asked before GitHub is:
            // a stage that has planned nothing has pushed nothing and opened
            // nothing, so its empty backlog is a run that has not started —
            // see [`stage_to_plan`]. The other two are a breakdown that never
            // landed and a feature that is finished with, and nothing can tell
            // those apart until [`nothing_left`] asks GitHub.
            if stage_to_plan(&state, conversation_id, &working_in, base).await {
                // A stage the human is still being asked where to start has
                // not started, and its open Set is the wait — a restart
                // reaching it included. See [`crate::placing::stage_waiting`].
                if crate::placing::stage_waiting(&state, conversation_id).await {
                    return;
                }

                tracing::info!(
                    conversation_id,
                    "a stage's backlog was never planned, so the planning is being run again",
                );

                return backlog_runner(state, conversation_id, driving).await;
            }

            nothing_left(state, conversation_id, &working_in, base, driving).await
        }
    }
}

/// What to make of a backlog with nothing left in it, which is decided by asking
/// GitHub what the branch is on and the branch what it has written.
///
/// [`inline_again`]'s question, asked at the other end of the run. An inline
/// implementation asks it before spending a session, because a branch that is
/// already on a pull request has nothing left to implement. A backlog asks it
/// once it has nothing left to work, and the reasoning arrives at the same
/// place: the finish step that emptied it is the step that opens the pull
/// request, so a branch on one is a run whose ending got most of the way through.
///
/// Which is one of the cases this is written for. Recording the pull request and
/// moving the Conversation into Wrapping is one transaction — see
/// [`store::record_pull_request`] — and a Conversation whose ending failed
/// somewhere after the push is left implementing a backlog that is empty, with
/// the work on a pull request nothing has written down. Every way back in used
/// to refuse it: Resume for the empty backlog, a steer into Wrapping for the
/// pull request it had no record of. So the run asks GitHub rather than the
/// record, because GitHub is the one that knows.
///
/// Three answers, exactly as [`inline_again`] has them:
///
/// - a pull request, and [`crate::wrapping::opened`] records it and starts the
///   wrap-up, finishing the ending that did not finish;
/// - [`github::Trouble::NoPullRequest`], which is the ending that got nowhere
///   near as far, and what happens about it is below;
/// - anything else, which is `gh` unable to answer at all, and that stops,
///   saying which trouble it was.
///
/// **An empty backlog and no pull request is two situations, and the branch tells
/// them apart.** A branch that has written a backlog since it came off its base
/// has been worked through and finished with — the finish commit is what took
/// `.tasks/` away — so the work is built and the one thing missing is the push
/// that never happened, and a session is sent for it exactly as an ending that
/// stopped short gets one. A branch that has written none never had a breakdown
/// land on it at all: there is nothing built to open a pull request for, and
/// pressing on would be pushing an empty branch. That one stops, and it is the
/// only way out of here that still does.
///
/// Which is what makes Resume worth pressing on a run that stopped at its push.
/// The press comes back through here, the branch still says the work is written,
/// and the go is taken again — a human who has just logged `gh` in gets the
/// pipeline finished rather than the Notice they were already looking at. See
/// [`asked_for_a_pull_request`], which is the same move the automatic ending
/// makes, and [`backlog_written`], which is the reading that separates the two.
///
/// The stop is a stop rather than a line in the log, because what is on the other
/// side of doing nothing here is the stall sweep finding the Conversation undriven
/// a minute later and stopping it with a worse sentence than this one.
async fn nothing_left(
    state: AppState,
    conversation_id: i64,
    working_in: &Path,
    base: Option<&str>,
    driving: Driving,
) {
    let Some((_repo_id, branch, found)) = crate::wrapping::asked(&state, conversation_id).await
    else {
        return;
    };

    // Held until the wrap-up's watchers have registrations of their own, or
    // until the stop is written: dropping first would leave a moment where a
    // sweep could find the Conversation undriven and stop it all over again.
    let _driving = driving;

    match found {
        Ok(opened) => {
            tracing::info!(
                conversation_id,
                number = opened.number,
                "the backlog is worked out and the branch is on a pull request nothing \
                 recorded, so this wraps it up rather than working it again",
            );

            // With no Event to read a tail off: the session that opened the pull
            // request is long gone, and what it said is already on the Timeline
            // above whatever stopped the run.
            crate::wrapping::opened(&state, conversation_id, None).await
        }
        Err(github::Trouble::NoPullRequest) if wrote_a_backlog(working_in, base).await => {
            tracing::info!(
                conversation_id,
                "the backlog was written, worked out and left on no pull request, so a \
                 session is being sent to open one",
            );

            asked_for_a_pull_request(&state, conversation_id).await
        }
        Err(github::Trouble::NoPullRequest) => {
            tracing::info!(
                conversation_id,
                "there is no backlog left to work, none was ever written here, and the \
                 branch is on no pull request",
            );

            stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "working out what is left of the backlog",
                "There's nothing in `.tasks/` to work, nothing on this branch ever wrote \
                 a backlog, and there's no pull request to wrap up. The breakdown never \
                 landed, so nothing built here can be carried anywhere.",
                None,
            )
            .await;
        }
        Err(trouble) => {
            tracing::warn!(
                conversation_id,
                branch,
                why = trouble.why(),
                "GitHub cannot be asked what an emptied backlog's branch is on, so nothing \
                 was started again",
            );

            stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "working out what is left of the backlog",
                &trouble.why(),
                None,
            )
            .await;
        }
    }
}

/// Run an inline implementation again — or, where the branch is already on a
/// pull request, wrap that up instead.
///
/// The question in front of the session is the one [`roadmap_again`] asks a
/// phase earlier: an inline implementation ends on a pull request, so a branch
/// that already has one has nothing left to implement. Two ways it gets there,
/// and neither of them is noticed by anything else — a session that pushed and
/// opened the pull request before it died, and a human who opened one by hand
/// off the halt's own advice. Asked of GitHub rather than of the branch,
/// because a pull request is GitHub's fact and the branch cannot say whether
/// there is one.
///
/// So `gh` is asked first, and what comes back decides between three things:
///
/// - a pull request, and the wrap-up takes it from here without a session being
///   spent on work that is already done;
/// - [`github::Trouble::NoPullRequest`], which is the ordinary case — the work
///   really is unfinished, so a fresh session builds it exactly as before;
/// - anything else, which is `gh` unable to answer at all, and that halts. A
///   session launched into it could only dead-end on the same missing thing
///   when it came to push, and the halt is what reaches the human on their
///   phone.
async fn inline_again(state: AppState, conversation_id: i64, driving: Driving) {
    // A run the human is still being asked where to start has not started, and
    // its open Set is the wait — a restart reaching it included. See
    // [`crate::placing::inline_waiting`].
    if crate::placing::inline_waiting(&state, conversation_id).await {
        return;
    }

    let Some((_, branch, found)) = crate::wrapping::asked(&state, conversation_id).await else {
        return;
    };

    match found {
        Ok(opened) => {
            tracing::info!(
                conversation_id,
                number = opened.number,
                "the work is already on a pull request, so this wraps it up rather than \
                 building it again"
            );

            // With no Event to read a tail off: there is no session behind this
            // one, the last of them having gone before the human pressed
            // Resume.
            crate::wrapping::opened(&state, conversation_id, None).await
        }
        Err(github::Trouble::NoPullRequest) => follow_inline(state, conversation_id, driving).await,
        Err(trouble) => {
            tracing::warn!(
                conversation_id,
                branch,
                why = trouble.why(),
                "GitHub cannot be asked whether the work is on a pull request, so nothing \
                 was started again",
            );

            crate::wrapping::stopped(&state, conversation_id, &trouble.why(), None).await
        }
    }
}

/// Write the roadmap again — or, where the branch already has one, look for the
/// pull request it opened.
///
/// Two cases, one phase earlier than the finish's: a roadmap Conversation's own
/// work is one session, and a run that stopped after it had written the roadmap
/// stopped on the question of what became of it rather than on the writing. So
/// the press is worth something either way — a roadmap that is not written is
/// written by a fresh session, and one that is written but on no pull request is
/// sent for the pull request. See [`to_a_pull_request`].
async fn roadmap_again(state: AppState, conversation_id: i64, working_in: &Path, driving: Driving) {
    let Some(base) = base(&state, conversation_id).await else {
        return;
    };

    let staged = {
        let worktree = working_in.to_owned();
        let base = base.clone();

        tokio::task::spawn_blocking(move || !crate::stages::touched(&worktree, &base).is_empty())
            .await
            .unwrap_or(false)
    };

    if staged {
        tracing::info!(
            conversation_id,
            "the roadmap is written, so this looks for the pull request again"
        );

        // Which is this path's own sighting of the landing: the run that wrote
        // the roadmap stopped before anything saw it out, so the row it never
        // got is written now. A second sighting writes nothing — see
        // [`store::record_roadmap`].
        crate::conversations::roadmap_landed(&state, conversation_id, Some(working_in), &base)
            .await;

        return to_a_pull_request(&state, conversation_id, None).await;
    }

    let Some(launched) = launch_step(
        &state,
        conversation_id,
        &Step::Staging(base.clone()),
        "verkstead/staging",
        |documents| skills::staging(documents.skills, &documents.brief),
    )
    .await
    else {
        return;
    };

    follow_roadmap(state, conversation_id, base, launched, driving).await
}

/// Work the backlog a wrap-up's review split its findings out into, from its
/// first task to the pull request the branch already has.
///
/// The one entry into a run that is not a direction being followed. What
/// launched it is a review the human answered by splitting work out — see
/// [`crate::review`] — and the Conversation has just been sent back down the
/// ladder to Implementing to build it. So which direction was picked for it in
/// the first place is beside the point: what is next is `.tasks/`, exactly as it
/// is for every other turn of a run, and the finish that follows the last task
/// wraps the Conversation up a second time.
///
/// The registration is taken here rather than by the caller, because the caller
/// is the review's own task and is about to end: a gap between the two would be
/// a Conversation the stall sweep found with nothing driving it.
///
/// And the landing is stamped here for the same reason it is stamped at the
/// other two: this is where a backlog is known to be on the branch. It is a
/// landing like any other — the review wrote `.tasks/` and committed it — and a
/// Conversation implemented inline has never had one before, so without this
/// its list is pinned above a record that says nothing about where it came
/// from. One that already carries a row keeps the row it has: a list lands
/// once, and [`store::record_backlog`] answers a second sighting by saying so.
pub(crate) fn build_the_split_out(state: &AppState, conversation_id: i64) {
    let driving = state.drivers.driving(conversation_id);
    let state = state.clone();

    tokio::spawn(async move {
        crate::conversations::backlog_landed(&state, conversation_id).await;

        backlog_runner(state, conversation_id, driving).await
    });
}

/// Whether `worktree` holds a backlog, committed as it stands.
///
/// What says the work a review split out has landed, asked by exactly the rule a
/// breakdown's own step is judged by — the list being there and git having
/// nothing pending for it. A `TODO.md` written and not committed is a session
/// still mid-write, and a wrap-up that read that as done would send the
/// Conversation back to build a backlog that is about to be swept away.
pub(crate) async fn backlog_landed(worktree: &Path) -> bool {
    check(worktree, &Landing::Arrived(todo())).await
}

/// Work a backlog under the **Backlog runner** Activity, from whichever box
/// `.tasks/TODO.md` says is next to the pull request — or, a roadmap stage
/// with no list yet, from its planning.
///
/// The component reads the boxes, launches each task's session and the
/// finish's, places a task that wants a platform through the host, holds a
/// stage's finish for its chain through the host, and stops
/// over a step that did not land or an entry naming no task file — all of it
/// read off the Repo, so a Resume or a restart is this started again. What it
/// ends in is mapped here, until a Process says it as data:
///
/// - **`finished`** is the finish landed, its session ended already, so the
///   pull request is recorded and the wrap-up starts — or, where the finish
///   left none, one is sent for: see [`to_a_pull_request`].
/// - **`steered`**, or a stop, has said where the Conversation went already,
///   and nothing is done here.
///
/// The registration it is handed is held across the component and whatever its
/// Outcome sets going, as every driver here holds one.
pub(crate) async fn backlog_runner(state: AppState, conversation_id: i64, driving: Driving) {
    // As every launch here always has: a server with no skills starts no
    // session of any kind, and says so in its log.
    if state.sessions.skills().is_none() {
        tracing::error!(
            conversation_id,
            "this server has no skills to send a session into, so no backlog was worked"
        );
        return;
    }

    let shipped = crate::plugins::built_in::activities();
    let Some(activity) = shipped_activity(&shipped, BACKLOG_RUNNER) else {
        tracing::error!(
            conversation_id,
            "this build carries no Backlog runner Activity, so no backlog was worked"
        );

        return stop(
            &state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            "working the backlog",
            "This build of Verkstead can't work a backlog.",
            None,
        )
        .await;
    };

    let options = stage(&state, conversation_id).await.into_iter().collect();

    let ran = crate::activities::run(
        &state,
        conversation_id,
        &driving,
        shipped.files(),
        &activity,
        &[],
        options,
        "working the backlog",
        crate::continuing::Brought::Settle,
    )
    .await;

    let Some(ran) = ran else {
        return;
    };

    if ran.outcome == "finished" {
        // Held across the wrap-up's start, or the session sent for a pull
        // request: dropping first would leave a moment where a sweep could
        // find the Conversation undriven.
        let _driving = driving;

        to_a_pull_request(&state, conversation_id, ran.writing).await;
    }
}

/// The `stage` option the **Backlog runner** is started with where
/// `conversation_id` is a roadmap stage: the branch its own was cut from, empty
/// where it came off the default branch. What its planning is told, and what
/// says a stage with no list is one still to plan. A record that will not be
/// read is no stage, which plans nothing.
async fn stage(state: &AppState, conversation_id: i64) -> Option<(String, String)> {
    match store::stacks_on(&state.pool, conversation_id).await {
        Ok(stage) => Some(("stage".to_owned(), stage?.unwrap_or_default())),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a Conversation is a roadmap stage failed");
            None
        }
    }
}

/// The id the **Backlog runner** Activity has in the built-in Activities
/// plugin's manifest.
const BACKLOG_RUNNER: &str = "backlog-runner";

/// The Activity called `id` in the built-in Activities plugin, as this build
/// ships it.
fn shipped_activity(
    shipped: &crate::plugins::built_in::BuiltIn,
    id: &str,
) -> Option<crate::plugins::manifest::Activity> {
    shipped
        .manifest(crate::platform::Platform::HERE)
        .ok()?
        .activities
        .into_iter()
        .find(|activity| activity.id == id)
}

/// Carry work that is built and committed on to the pull request it ends on —
/// and where the session that should have opened one did not, ask for one before
/// anything stops.
///
/// **Every run here ends on a pull request**, and each of them ends on it the
/// same way: the session commits the last of the work and then follows the
/// repository's own finish sequence to push and open one. A backlog's finish
/// step, an inline implementation, a roadmap's own session — three endings, one
/// shape. So they have one failure too: a session that landed its commit and
/// stopped short of the push leaves the work built, committed and unreviewable,
/// and the stop that used to follow said exactly that and left the human to open
/// the pull request themselves.
///
/// **So the missing thing is asked for by a session of its own.** It is the
/// cheapest ask there is — the work is committed, and what is left is a push and
/// a `gh pr create` — and it is the one thing the run cannot go on without. A
/// fresh session rather than the one that stopped short: that one is over by the
/// time this is asked, and a context that already stopped short of the last step
/// is not the one to send back to it. See [`skills::submitting`], which says the
/// work is already built and that opening the pull request is the whole of the
/// job.
///
/// **Only where GitHub says there is no pull request.** A `gh` that is missing,
/// logged out, or looking at a repository with no remote is a wall a session
/// would walk into at exactly the same place, so those stop where they always
/// did — the reasoning [`inline_again`] follows a phase earlier. The answer is
/// handed on to [`crate::wrapping::record`] whole, which makes of it what it
/// always has.
///
/// `writing` is the Timeline Event the session that stopped short printed into,
/// so that a stop written before anything else runs carries the tail of what *it*
/// last said, and `None` where there is no session left to read one off.
async fn to_a_pull_request(state: &AppState, conversation_id: i64, writing: Option<i64>) {
    let Some((repo_id, branch, found)) = crate::wrapping::asked(state, conversation_id).await
    else {
        return;
    };

    if !matches!(found, Err(github::Trouble::NoPullRequest)) {
        return crate::wrapping::record(
            state,
            conversation_id,
            repo_id,
            &branch,
            found,
            writing,
            crate::wrapping::Door::TheMove,
        )
        .await;
    }

    tracing::warn!(
        conversation_id,
        branch,
        "the work is committed and on no pull request, so a session is being sent to open one",
    );

    asked_for_a_pull_request(state, conversation_id).await
}

/// Send one session for the pull request the work should already be on, and make
/// of what it leaves what everything else here makes of it.
///
/// The move the automatic endings and a pressed Resume share, which is why it is
/// a function: a run that stopped short of its push and a human pressing Resume
/// on one are the same Conversation wanting the same thing, and answering them
/// differently would mean the press was worth less than the run. See
/// [`to_a_pull_request`] for the first and [`nothing_left`] for the second — and
/// [`over`], which is a **Tinker**'s ending, and [`follow_up_again`], whose
/// component reads a press on one that got this far as that ending again.
///
/// **Once per go.** What follows the session is GitHub asked again, and what it
/// says then is the whole of it: a pull request wraps the Conversation up, and no
/// pull request stops it in the words it has always stopped in — over the session
/// that was sent to open one, whose last words are where the reason there is
/// still none is written down. Verkstead does not go round a second time by
/// itself, because two agents that both stopped short of the same push is
/// something for the human to look at. What they have then is Resume, and a press
/// is another go through here: the work is still built, so there is still exactly
/// one thing to ask for.
async fn asked_for_a_pull_request(state: &AppState, conversation_id: i64) {
    let Some(writing) = submitted(state, conversation_id).await else {
        return;
    };

    crate::wrapping::opened(state, conversation_id, Some(writing)).await
}

/// Run the one session sent to open a pull request the finish step did not, and
/// wait until it is over.
///
/// **Ended because it said it was done** — see [`crate::done`] — and once it is
/// next idle after that. What this session is sent to do happens on GitHub
/// rather than in the repository, so there is no path to check the signal
/// against and no commit it has to make — a branch that only wanted pushing is
/// one it finishes without writing a line. What holds is what every signal has:
/// nothing left uncommitted, and a Set of its own still open locked as it is
/// taken. Nothing ends it on quiet, so one that waits on a slow push is left to
/// finish it.
///
/// **And one that will not ask is spoken to**, until it signals — see
/// [`crate::rescues`], which tells the human where it will not answer and leaves
/// the session running for them to move.
///
/// The Timeline Event it printed into, or `None` where nothing ran: no session
/// could be started, or the run was stopped from outside while this one did. Both
/// of those have already said whatever there was to say.
async fn submitted(state: &AppState, conversation_id: i64) -> Option<i64> {
    let documents = documents(state, conversation_id).await?;

    // Every caller is a driver holding a registration of its own already; this
    // one is held across the session alone, as the Activity's host asks.
    let driving = state.drivers.driving(conversation_id);

    // No signal given and nothing open, with the session sitting there: spoken
    // to, and the human told where it will not answer — see [`crate::rescues`].
    //
    // Nothing on the branch for the signal to be checked against: what this
    // session does is GitHub's to say, so GitHub is what it is checked against.
    // See [`crate::done::Evidence::Nothing`] and [`crate::done::Ends`].
    let ran = one_session(
        state,
        conversation_id,
        &driving,
        Sent {
            skill: "verkstead/submitting",
            prompt: skills::submitting(
                documents.skills,
                &documents.brief,
                documents.handoff(),
                None,
            ),
            evidence: crate::done::Evidence::Nothing,
            ends: crate::done::Ends::OnAPullRequest,
            building: None,
            doing: "opening the pull request",
        },
    )
    .await?;

    let (event_id, ended) = ran.seen();

    match ended {
        // Said it is done and gone quiet since. Ended rather than left holding
        // the Worktree, and what it did is GitHub's to say.
        None => {
            tracing::info!(
                conversation_id,
                event_id,
                "the session sent to open the pull request said it is done and has gone \
                 quiet since, so it is being ended",
            );

            state.sessions.end(conversation_id).await;
        }
        // Verkstead ended it — the human closed the Conversation or force-stopped
        // it, or the account it was spending ran out of window. The stop is on the
        // record already, so there is nothing to ask GitHub about. See
        // [`crate::sessions::Ended::on_purpose`].
        Some(ended) if ended.on_purpose() => {
            tracing::info!(
                conversation_id,
                event_id,
                "the session sent to open the pull request was stopped from outside, so \
                 nothing is asked about it",
            );

            return None;
        }
        // And one that ended by itself is read by what it left on GitHub rather
        // than by how it exited: an agent that opened the pull request and then
        // fell over has done the job, and one that exited cleanly having done
        // nothing has not. The ask that follows this is what tells them apart.
        Some(_) => {}
    }

    Some(event_id)
}

/// Start the session that builds from the handoff the grilling session wrote.
///
/// The inline counterpart to [`broken_down`], and the same move with a
/// different artifact: the session that settled the work is the one that wrote
/// down what it settled, after the pick rather than before it, so what the human
/// said beside the pick is part of what the handoff says.
///
/// Then the three things the far side of an inline pick needs, in the order they
/// have to happen in. The handoff goes on the Timeline, which is where a
/// Conversation's documents live and the one moment this one is certainly
/// finished. The Conversation moves, so that what says it is being built has
/// everything the grilling left beside it. And the work starts, in a fresh
/// session under the implementation Profile — fresh because the two run as
/// accounts the Conversation fixed separately and a session cannot change the
/// one it is running as, which is the whole reason the handoff exists.
///
/// The registration is held across all of it — the move, and the run on the
/// other side of it — so there is no moment between the handoff landing and the
/// implementation session starting where the Conversation reads as one nothing
/// is driving.
async fn handed_off(state: AppState, conversation_id: i64, driving: Driving) {
    crate::conversations::hand_over(&state, conversation_id).await;
    crate::conversations::grilling_over(&state, conversation_id).await;

    // And an inline run whose proposal wants one platform this device is not is
    // moved to one that is before its session starts, the way a declared task
    // is: the handoff is on the record by now, so it crosses with it, and the
    // far end's arrival presses its own Resume, which launches the run there
    // without asking again — see [`crate::placing::inline`]. Where the human is
    // asked where it runs instead, their answer launches it.
    if !crate::placing::inline(&state, conversation_id).await.here() {
        return;
    }

    follow_inline(state, conversation_id, driving).await
}

/// Launch an inline implementation session and see it out, and carry the Conversation on to
/// wrapping the pull request it opened — or stop the run if it ends having
/// landed nothing.
///
/// The whole of the work in one session, so there is no next step to launch and
/// nothing to poll a Worktree for: what says an inline session did anything is
/// what it committed, which the branch watcher is putting on the Timeline while
/// it runs.
///
/// **Ended because it said it was done**, which is [`instructed`]'s rule on a
/// session of the same shape — see [`crate::done`]. What the signal is checked
/// against is a commit, there being no path to watch and a commit being the one
/// report an agent cannot half make; a signal with nothing committed since the
/// session began is refused. Nothing ends the session on the commit itself: work
/// does not stop at the first one, and a session that commits a slice and then
/// waits on its tests is a session still at work. Once the signal is taken the
/// session is ended when it is next idle, so what it says on the way out is
/// kept. Waiting for the process to exit instead would be waiting for something
/// that never comes: every session here is an interactive agent that idles when
/// its work is done.
///
/// **And one that will not ask is spoken to**, until it signals. Idle with no
/// signal given and nothing put to the human is an inline run come to nothing,
/// or finished and not said, with a process still holding the Worktree — a
/// Conversation nobody can move, driven so nothing sweeps it and silent so
/// nothing says so. Where it will not answer the human is told, and the session
/// left running, as every other driver here does it. See [`crate::rescues`].
///
/// Committing is measured against what was already there rather than against
/// zero, which is what makes a second go answerable: a first attempt that
/// committed twice and then died leaves two commits behind, and a second that
/// commits nothing has committed nothing. What such a second session is sent for
/// is the pull request, so that is what its signal is checked against instead —
/// see [`crate::done`]'s `missing`.
///
/// What follows a session that landed something is the same ending a backlog's
/// finish step has: the session followed the repository's own review process
/// before it signalled, so the branch is pushed and on a pull request by then,
/// and [`to_a_pull_request`] is what finds that pull request and
/// moves the Conversation on — or, where the session stopped short of the push,
/// sends for the one thing missing before anything stops. An inline
/// implementation is work like any other work and goes for review like any other
/// work.
///
/// Which is why landing nothing is not the end of it either. A second session on
/// the branch — the one [`inline_again`] launches where GitHub has no pull
/// request yet — is sent to check work that is already built and carry it to
/// one, and has nothing to commit when it finds nothing left to finish. So a
/// clean session that committed nothing is asked about before it is called an
/// empty one, and the pull request is what tells the two apart.
///
/// The registration it is handed is held until the session is over and whatever
/// it left behind has been read, so an inline run is a driven Conversation for
/// exactly as long as somebody is watching it.
async fn follow_inline(state: AppState, conversation_id: i64, driving: Driving) {
    // Read before the session starts, so it is where the run stood before this
    // session rather than somewhere that includes what it goes on to do.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let Some(documents) = documents(&state, conversation_id).await else {
        return;
    };

    // One that is idle with nothing open and no signal given is the whole of an
    // inline run come to nothing, or finished and not said, with a process
    // still holding the Worktree and nothing on the page to press. Spoken to,
    // and the human told where it will not answer — see [`crate::rescues`].
    //
    // What the session's Done signal is checked against: a commit past where the
    // run stood, and a pull request open on the branch. See [`crate::done`].
    //
    // What an inline session that has done its work looks like from here: it
    // said so, the branch bore that out, and it has been idle since. Nothing on
    // the branch ends it, because work does not stop at the first commit — a
    // session may commit a slice and then wait on its tests. See
    // [`signalled_and_idle`].
    let Some(ran) = one_session(
        &state,
        conversation_id,
        &driving,
        Sent {
            skill: "verkstead/implementing",
            prompt: skills::implementing(documents.skills, &documents.brief, documents.handoff()),
            evidence: crate::done::Evidence::Committed { already },
            ends: crate::done::Ends::OnAPullRequest,
            building: Some(crate::memos::Building::Inline),
            doing: "implementing the work inline",
        },
    )
    .await
    else {
        return;
    };

    let (event_id, ended) = ran.seen();

    // Said it is done over a commit, and gone quiet since, which is an inline
    // implementation done. The session is ended rather than waited out, and what
    // follows is the ending a landed run has always had: the skill carried the
    // branch to a pull request on its way out, and [`to_a_pull_request`] is what
    // finds it — or sends for it, and then stops naming what it still could not
    // find.
    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            "an inline session said it is done and has gone quiet since, so it is being ended",
        );

        state.sessions.end(conversation_id).await;

        return to_a_pull_request(&state, conversation_id, Some(event_id)).await;
    };

    // Verkstead ended it — the human closed the Conversation or force-stopped
    // it, or the account it was spending ran out of window. Whichever it was,
    // the stop is already on the record and nothing was left to land, so there
    // is nothing to ask about. Answered before the branch is read, since a run
    // stopped from outside has committed nothing and would otherwise read as a
    // session that did nothing. See [`crate::sessions::Ended::on_purpose`].
    if ended.on_purpose() {
        tracing::info!(
            conversation_id,
            event_id,
            "the inline run was stopped from outside, so nothing is asked about it"
        );
        return;
    }

    // Read after the session is over, which is after its relay has waited out the
    // final sweep of the branch: a session's last act is usually a commit, and it
    // lands a poll after the process that made it has gone.
    let landed = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed > already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what an inline session committed failed");
            return;
        }
    };

    let how = match (ended.badly(), landed) {
        // Ended well and committed something, which is an inline implementation
        // done. What becomes of the work from here is the wrap-up phase's, and
        // the session's own Timeline Event goes with it, so that a halt written
        // there carries the tail of what it last said — which is where the
        // reason it opened no pull request is usually written down.
        (None, true) => {
            tracing::info!(
                conversation_id,
                event_id,
                "an inline session has landed its work"
            );

            to_a_pull_request(&state, conversation_id, Some(event_id)).await;
            return;
        }
        // Exited cleanly having committed nothing at all. An interactive agent
        // that decides there is nothing to do exits zero, so this is exactly the
        // case a status could not have caught.
        //
        // But it is also what the *second* session on a branch looks like when
        // it does its job. The skill sends one that finds the work already built
        // to check it over and carry it to the pull request — see
        // [`inline_again`], which is what launches it — and a session that finds
        // nothing left to finish has nothing left to commit either. So GitHub is
        // asked before this is called nothing: a pull request on the branch is
        // that session having done the whole of what it was sent for.
        (None, false) => match crate::wrapping::asked(&state, conversation_id).await {
            // Nothing committed, and GitHub saying there is nothing on the
            // branch either, which is the session that really did do nothing.
            // Said in its own words rather than `gh`'s: what is wrong here is
            // the empty session, and the missing pull request is only how it
            // shows.
            //
            // A branch that could not be asked about at all falls the same way,
            // that being in the log already and no more of an answer than it
            // was.
            Some((_, _, Err(github::Trouble::NoPullRequest))) | None => {
                "The session ended without committing anything.".to_owned()
            }
            // Anything else is what the landed arm above makes of it, made in
            // the one place that knows how: a pull request found is a wrap-up,
            // and a `gh` that cannot answer at all is a halt naming what the
            // human can go and fix.
            Some(_) => {
                tracing::info!(
                    conversation_id,
                    event_id,
                    "an inline session committed nothing, so its pull request is what says \
                     whether it did anything"
                );

                crate::wrapping::opened(&state, conversation_id, Some(event_id)).await;
                return;
            }
        },
        // Ended badly, whether or not it got some of the way: the human is owed
        // the telling either way, and what it committed is on the Timeline above
        // the Notice for them to read.
        (Some(badly), _) => badly,
    };

    stop(
        &state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "implementing the work inline",
        &how,
        Some(event_id),
    )
    .await;
}

/// Do the one thing the human wrote when they steered the Conversation into
/// Implementing, and then carry the pipeline on from what the branch holds.
///
/// **A driver rather than an errand beside the work.** The registration it is
/// handed says the Conversation is being driven for as long as this runs, so
/// nothing sweeps it as standing still; it is judged by the rules every other
/// session here is judged by, so one that ends badly stops the Conversation
/// with the ordinary Notice; and what follows a clean finish is [`onwards`]
/// rather than nothing.
///
/// **Ended because it said it was done**, checked against a commit — see
/// [`crate::done`]. An instruction can ask for anything, so there is no path to
/// watch and no done file to read — what there is, is a commit, which is the
/// one report an agent cannot half make, and a signal with nothing committed
/// since the session began is refused. Work does not always stop at the commit,
/// so nothing ends the session on one; once its signal is taken it is ended when
/// it is next idle.
///
/// **And a session that commits nothing stops the Conversation**, exactly as an
/// inline implementation that commits nothing does. An interactive agent that
/// decides there is nothing to do exits zero, so a clean exit is not by itself a
/// report that anything happened — and the pipeline reads the branch to decide
/// what is next, so a branch nothing was written to is one there is nothing
/// honest to carry on from.
///
/// **Including one that never ends at all.** Idle, with no signal given and
/// nothing put to the human, is the same instruction come to nothing with a
/// process still holding the Worktree — so it is spoken to, and the human told
/// where it will not answer. See [`crate::rescues`].
pub(crate) async fn instructed(
    state: AppState,
    conversation_id: i64,
    instruction: String,
    driving: Driving,
) {
    // Read before the session starts, so it is where the branch stood before the
    // instruction rather than somewhere that includes what it goes on to do.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let Some(documents) = documents(&state, conversation_id).await else {
        return;
    };

    // No signal given and nothing asked, with the session sitting there: an
    // instruction that has come to nothing, or finished and not said, and
    // nobody to say so to. Spoken to, and the human told where it will not
    // answer — see [`crate::rescues`].
    //
    // What the session's Done signal is checked against: a commit past where the
    // branch stood. See [`crate::done`].
    let Some(ran) = one_session(
        &state,
        conversation_id,
        &driving,
        Sent {
            skill: "verkstead/instruction",
            prompt: skills::instruction(
                documents.skills,
                &documents.brief,
                documents.handoff(),
                &instruction,
            ),
            evidence: crate::done::Evidence::Committed { already },
            ends: crate::done::Ends::WithItsWork,
            building: Some(crate::memos::Building::Instruction),
            doing: "doing what the instruction said",
        },
    )
    .await
    else {
        return;
    };

    let (event_id, ended) = ran.seen();

    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            "an instruction session said it is done and has gone quiet since, so it is being \
             ended",
        );

        state.sessions.end(conversation_id).await;

        return onwards(state, conversation_id, event_id, driving).await;
    };

    // The session is over on its own account. It may have committed as its last
    // act and exited before a poll caught it, which is the ordinary shape of a
    // session that finishes rather than idles — so the record is asked once more
    // before this is read as an instruction that came to nothing. Asked
    // whichever way it ended, exactly as a step's landing is: what was committed
    // is committed, and an agent that did the work and then fell over on its way
    // out has left the human nothing to decide about.
    let landed = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed > already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what an instruction session committed failed");
            return;
        }
    };

    if landed {
        return onwards(state, conversation_id, event_id, driving).await;
    }

    // Verkstead ended it — the human closed the Conversation or force-stopped it
    // out from under the instruction, or the account it was spending ran out of
    // window. Each of the three has already written the stop this would
    // otherwise write. See [`crate::sessions::Ended::on_purpose`].
    if ended.on_purpose() {
        tracing::info!(
            conversation_id,
            event_id,
            "the instruction session was stopped from outside, so nothing is asked about it",
        );
        return;
    }

    // How it ended, where the ending itself was the problem; otherwise the
    // nothing it committed is, which is the case no exit status could have
    // shown.
    let how = ended
        .badly()
        .unwrap_or_else(|| "The session ended without committing anything.".to_owned());

    stop(
        &state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "doing what the instruction said",
        &how,
        Some(event_id),
    )
    .await;
}

/// Carry the pipeline on from whatever the branch holds, once an instruction
/// session has done what it was sent to do.
///
/// The point of an instruction session being a driver at all: the human wrote
/// one thing, and what happens after it is the machine's rather than theirs.
/// What that is, is read off the branch and nowhere else, exactly as every other
/// turn of a run reads it.
///
/// **The backlog first**, because a list with work left in it is an answer to
/// what is next that nothing else can give — and it is the answer even where the
/// branch is already on a pull request, which is the shape a wrap-up that split
/// its findings out into tasks leaves behind. The finish that follows the last
/// task wraps the pull request up again, so nothing is skipped by going this way
/// round.
///
/// **Then the pull request**, asked of `gh` the way the finish step asks — see
/// [`crate::wrapping::opened`], which finds it, moves the Conversation into
/// Wrapping over the record it already had, and starts the wrap-up's watchers
/// afresh.
///
/// Of `gh` rather than of the record, which is the difference between wrapping a
/// Conversation up and stopping it a second time. A record is what Verkstead
/// wrote down; a pull request is GitHub's fact. Where the two disagree it is
/// because the writing down failed — and a Conversation whose ending failed
/// after the push is exactly the Conversation a human steers an instruction into
/// to get it moving. Asking the record would tell it what it already believes.
///
/// **And a stop where the branch holds neither**, because there is nothing left
/// that could be started and a Conversation left implementing with nothing
/// driving it would be one the stall sweep stopped a minute later with a worse
/// sentence. A `gh` that cannot answer stops it too, saying which trouble it
/// was: a session launched into that could only dead-end on the same missing
/// thing. `writing` is the Event the session printed into, so the Notice
/// carries the tail of what it last said.
async fn onwards(state: AppState, conversation_id: i64, writing: i64, driving: Driving) {
    let Some(worktree) = worktree(&state, conversation_id).await else {
        return;
    };

    if anything_to_work(&worktree).await {
        tracing::info!(
            conversation_id,
            "the instruction is done and the backlog holds more, so the run carries on",
        );

        return backlog_runner(state, conversation_id, driving).await;
    }

    // Held until the wrap-up's own watchers have registrations of their own, or
    // until the stop is written: dropping first would leave a moment where a
    // sweep could find the Conversation undriven and stop it all over again.
    let _driving = driving;

    let Some((_repo_id, branch, found)) = crate::wrapping::asked(&state, conversation_id).await
    else {
        return;
    };

    match found {
        Ok(opened) => {
            tracing::info!(
                conversation_id,
                number = opened.number,
                "the instruction is done and the branch is on a pull request, so it is \
                 wrapped up again",
            );

            crate::wrapping::opened(&state, conversation_id, Some(writing)).await;
        }
        Err(github::Trouble::NoPullRequest) => {
            tracing::info!(
                conversation_id,
                "the instruction is done and the branch holds nothing to carry on with",
            );

            stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "carrying the work on from what the instruction left",
                "The branch holds no backlog to work and no pull request to wrap up.",
                Some(writing),
            )
            .await;
        }
        Err(trouble) => {
            tracing::warn!(
                conversation_id,
                branch,
                why = trouble.why(),
                "GitHub cannot be asked what the branch an instruction left is on",
            );

            stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "carrying the work on from what the instruction left",
                &trouble.why(),
                Some(writing),
            )
            .await;
        }
    }
}

/// One session sent under the **One session** Activity: what it is sent to do,
/// in the shape the Activity's options take it.
struct Sent<'a> {
    /// `<pack>/<skill>`, which the Activity's block declares.
    skill: &'a str,
    /// The prompt in the words the [`skills`] builder wrote it in.
    prompt: String,
    evidence: crate::done::Evidence,
    ends: crate::done::Ends,
    /// What its Memos and previews are permitted by, where it builds anything.
    building: Option<crate::memos::Building>,
    /// What the rescue says it is doing.
    doing: &'a str,
}

/// What a session run under the **One session** Activity came to, where its
/// caller has anything left to do about it.
enum Ran {
    /// It gave its Done signal, had it taken, and has been idle since: the
    /// Timeline Event it printed into.
    Done(i64),
    /// It ended without one, and how.
    Refused(i64, Ended),
}

impl Ran {
    /// The Event it printed into, and how it ended — `None` where its Done
    /// signal was taken — which is how a driver read [`Lent::see`] before.
    fn seen(self) -> (i64, Option<Ended>) {
        match self {
            Ran::Done(event_id) => (event_id, None),
            Ran::Refused(event_id, ended) => (event_id, Some(ended)),
        }
    }
}

/// Run one session under the **One session** Activity, the built-in Activities
/// plugin's component, and say what it came to.
///
/// The way the inline build, the `submitting` step and an `instruction` steer
/// launch their session: the component launches it under the skill, expecting
/// the evidence, with the prompt written here, and answers `done` or `refused`
/// — and what either means is still the caller's, which reads how the session
/// ended back off the host.
///
/// `None` where nothing is left for the caller: no session launched — the
/// component stopped the Conversation over the host's refusal, or this server
/// has no skills, which is said in its log as every launch here always has —
/// or the Conversation was stopped or steered elsewhere while it ran.
async fn one_session(
    state: &AppState,
    conversation_id: i64,
    _driving: &Driving,
    sent: Sent<'_>,
) -> Option<Ran> {
    if state.sessions.skills().is_none() {
        tracing::error!(
            conversation_id,
            skill = sent.skill,
            "this server has no skills to send a session into, so none was started"
        );
        return None;
    }

    let Some((shipped, activity)) = built_in(ONE_SESSION) else {
        tracing::error!(
            conversation_id,
            "this build carries no One session Activity, so no session was started"
        );

        stop(
            state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            sent.doing,
            "This build of Verkstead can't run a session.",
            None,
        )
        .await;

        return None;
    };

    let mut options = vec![
        ("skill".to_owned(), sent.skill.to_owned()),
        ("role".to_owned(), "implementation".to_owned()),
        ("prompt".to_owned(), sent.prompt),
        (
            "evidence".to_owned(),
            crate::lending::evidence_option(&sent.evidence),
        ),
        (
            "ends".to_owned(),
            crate::lending::ends_option(sent.ends).to_owned(),
        ),
    ];

    if let Some(building) = sent.building {
        options.push(("building".to_owned(), building.option()));
    }

    let (outcome, seen) = crate::activities::run_seeing(
        state,
        conversation_id,
        shipped.files(),
        &activity,
        &[],
        options,
        Lent::new(state, conversation_id).doing(sent.doing),
    )
    .await;

    let event_id = seen.event_id?;

    match outcome.as_deref() {
        Some("done") => Some(Ran::Done(event_id)),
        Some("refused") => Some(Ran::Refused(event_id, seen.ended.unwrap_or(Ended::Unknown))),
        _ => None,
    }
}

/// Run a wrap-up under the **Wrap-up** Activity, and land the Conversation
/// wherever its Outcome says.
///
/// The way in for every start of a wrap-up — a pull request recorded, a
/// restart, a Resume, the resolve press — through [`crate::wrapping::wrap_up`],
/// which holds the registration this is handed. The checks, the comments and
/// the review are still watched by the host beside it (see
/// [`crate::wrapping::watching`]); what the component watches itself is each
/// pull request's landing, and what the host's record says of the rest.
///
/// - **`against`** is the base a **Review** or **Fix Merge Issues** taken up
///   over a bare branch is owed its pull request against: the component is
///   handed the `submitting` session's prompt and opens it itself — see
///   [`crate::conversations::take_up`].
/// - **The Process's narrowing is its options**: Fix Merge Issues turns the
///   review and the comments off, which the component settles at once — see
///   [`crate::conversations::narrows_the_wrap_up`].
///
/// And the Outcomes, mapped here until a Process says them as data:
///
/// - **`settled`** is Done, the news, the share and the next stage, as the rule
///   always ended a wrap-up — see [`crate::settling::settled`]. A rule that is
///   no longer true by the time it is applied starts the component again.
/// - **`merged`** is the same Done without the rule: every pull request has
///   landed, and nothing is left for the wrap-up to wait on.
/// - **`closed`** is a stop: a pull request closed unmerged is the human's to
///   decide about.
/// - **`given-up`** writes the stop the host gave up with, which is exactly the
///   stop it always wrote — see [`crate::wrapping::give_up`].
/// - **`steered`**, or a stop, has said where the Conversation went already.
pub(crate) async fn wrap_up(
    state: AppState,
    conversation_id: i64,
    against: Option<String>,
    _driving: Driving,
) {
    let Some((shipped, activity)) = built_in(WRAP_UP) else {
        tracing::error!(
            conversation_id,
            "this build carries no Wrap-up Activity, so nothing is wrapping up"
        );

        stop(
            &state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            "wrapping up",
            "This build of Verkstead can't wrap up.",
            None,
        )
        .await;

        return;
    };

    let process = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.process,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to wrap up failed");
            return;
        }
    };

    let mut options = vec![
        (
            "poll".to_owned(),
            state.sessions.pace().checks.as_millis().to_string(),
        ),
        ("attempts".to_owned(), crate::checks::ATTEMPTS.to_string()),
    ];

    if crate::conversations::narrows_the_wrap_up(process) {
        options.push(("review".to_owned(), "off".to_owned()));
        options.push(("comments".to_owned(), "off".to_owned()));
    }

    if let Some(against) = against {
        let Some(documents) = documents(&state, conversation_id).await else {
            return;
        };

        options.push((
            "prompt".to_owned(),
            skills::submitting(
                documents.skills,
                &documents.brief,
                documents.handoff(),
                Some(&against),
            ),
        ));
    }

    let outcome = {
        let _running = state.wrap_ups.running(conversation_id);

        // Its dispatches try for the Turn rather than wait for it, and hold it
        // for as long as their sessions run — see [`Lent::trying`].
        crate::activities::run_seeing(
            &state,
            conversation_id,
            shipped.files(),
            &activity,
            &[],
            options,
            Lent::new(&state, conversation_id)
                .doing("wrapping up")
                .trying(),
        )
        .await
        .0
    };

    // A stop handed over as it ended some other way is still the stop: it is
    // what the host's watchers wrote before the wrap-up was an Activity. Under
    // a stop of the Conversation's own, it is let go.
    let outcome = match outcome {
        Some(_) if state.wrap_ups.given_up(conversation_id) => Some("given-up".to_owned()),
        Some(outcome) => Some(outcome),
        None => {
            state.wrap_ups.let_go(conversation_id);
            None
        }
    };

    match outcome.as_deref() {
        Some("settled") => crate::settling::settled(&state, conversation_id, true).await,
        Some("merged") => crate::settling::settled(&state, conversation_id, false).await,
        Some("closed") => {
            stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "wrapping up",
                "A pull request this work is on was closed without being merged. Reopen \
                 it and press **Resume** to carry on wrapping up.",
                None,
            )
            .await
        }
        Some("given-up") => crate::wrapping::given_up(&state, conversation_id).await,
        _ => {}
    }
}

/// The id the **Wrap-up** Activity has in the built-in Activities plugin's
/// manifest.
const WRAP_UP: &str = "wrap-up";

/// The id the **One session** Activity has in the built-in Activities plugin's
/// manifest.
const ONE_SESSION: &str = "one-session";

/// The built-in Activities plugin, and its Activity whose id is `id`.
fn built_in(
    id: &str,
) -> Option<(
    crate::plugins::built_in::BuiltIn,
    crate::plugins::manifest::Activity,
)> {
    let shipped = crate::plugins::built_in::activities();
    let activity = shipped
        .manifest(crate::platform::Platform::HERE)
        .ok()?
        .activities
        .into_iter()
        .find(|activity| activity.id == id)?;

    Some((shipped, activity))
}

/// Run a follow-up's rounds under the **Follow-up rounds** Activity, and land
/// the Conversation wherever its Outcome says.
///
/// The way in for a steer into Follow-up, for a **Tinker** start, and — through
/// [`follow_up_again`] — for a follow-up picked up again. The component is the
/// built-in Activities plugin's, and it reaches the server through the host
/// functions alone: it reads what to follow up on off the Timeline, writes the
/// session's prompt, launches it expecting the human's Nothing-else mark, and
/// reads the branches at the end. What it answers is an Outcome, and where that
/// leaves the Conversation is decided here — a Process says it as data from
/// stage 11 on.
///
/// **Nothing is kept for it between starts.** What it is handed beyond the
/// start facts is two readings of the Timeline as it now stands, which a
/// component has no way to make for itself: the rounds already answered inside
/// this follow-up, as the digest a fresh session is primed with — see
/// [`crate::follow_ups`] — and, for a follow-up on no pull request, whether
/// the newest of them carries the Nothing-else mark. Both are empty for a
/// follow-up just opened, and both are what a restarted one derives where it
/// was from.
///
/// - **On a pull request**, either Outcome is the follow-up over: back to the
///   wrap-up it was opened over — see [`back_to_the_wrap_up`], whose `pushed`
///   is read against where the commits stood before the session launched, which
///   is this host's reading and never the component's.
/// - **`committed` with none** is a Tinker that built something, here or in a
///   companion: a `submitting` session is sent for its pull request. Which is
///   also what a Tinker whose ending sent for one and got none comes to when it
///   is picked up again: a marked round with work standing ends `committed` at
///   once rather than starting another round.
/// - **`nothing` with none** is a Tinker that built nothing anywhere: Done.
/// - **`steered`**, or a stop, has said where the Conversation went already,
///   and nothing is done here.
///
/// The registration it is handed is held across the component and whatever its
/// Outcome sets going, as every driver here holds one.
pub(crate) async fn follow_up_rounds(state: AppState, conversation_id: i64, driving: Driving) {
    // A server with no skills installed starts no session of any kind, and
    // says so in its log rather than on the Timeline, as every launch here
    // always has: the component would otherwise stop the Conversation over a
    // skill it was never going to be handed.
    if state.sessions.skills().is_none() {
        tracing::error!(
            conversation_id,
            "this server has no skills to send a session into, so no follow-up was started"
        );
        return;
    }

    // Read before the session starts, so what it lands is this follow-up's own:
    // whether the wrap-up's checks go back to waiting turns on whether *this*
    // follow-up pushed anything, and a Conversation on a pull request has a run
    // of commits behind it already.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let settled = match crate::follow_ups::settled(&state, conversation_id).await {
        Ok(settled) => settled.unwrap_or_default(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the rounds a follow-up has been through failed");
            return;
        }
    };

    let mut options = vec![("settled".to_owned(), settled)];

    // Said only of a follow-up on no pull request, which is the one whose
    // ending can stand owed: one on a pull request always holds commits past
    // its base, so a marked round there would end it at once — and a restart
    // between the mark and the work that round asked for would carry that work
    // back to the wrap-up undone. Picked up again, it runs another session,
    // which finds the mark and ends on it.
    if marked(&state, conversation_id, store::Lifecycle::FollowUp).await
        && owed_ending(&state, conversation_id).await
    {
        options.push(("marked".to_owned(), "true".to_owned()));
    }

    let shipped = crate::plugins::built_in::activities();
    let Some(activity) = shipped_activity(&shipped, FOLLOW_UP_ROUNDS) else {
        tracing::error!(
            conversation_id,
            "this build carries no Follow-up rounds Activity, so no follow-up was started"
        );

        return stop(
            &state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            "following the work up",
            "This build of Verkstead can't run follow-up rounds.",
            None,
        )
        .await;
    };

    let outcome = crate::activities::run(
        &state,
        conversation_id,
        &driving,
        shipped.files(),
        &activity,
        &[],
        options,
        "following the work up",
        crate::continuing::Brought::Settle,
    )
    .await;

    let committed = match outcome.as_ref().map(|ran| ran.outcome.as_str()) {
        Some("committed") => true,
        Some("nothing") => false,
        _ => return,
    };

    over(&state, conversation_id, already, committed, driving).await
}

/// Whether the Conversation's work is on no pull request, which is the only
/// follow-up whose ending can stand owed — see [`follow_up_rounds`]. Anything
/// the record will not say reads as *no*, which starts another round.
async fn owed_ending(state: &AppState, conversation_id: i64) -> bool {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => on_a_pull_request(state, &conversation).await == Some(false),
        Ok(None) => false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation a follow-up is on failed");
            false
        }
    }
}

/// The id the **Follow-up rounds** Activity has in the built-in Activities
/// plugin's manifest.
const FOLLOW_UP_ROUNDS: &str = "follow-up-rounds";

/// Pick a follow-up up again, on a press of Resume or a restart: the
/// **Follow-up rounds** Activity started over, on the Timeline as it now
/// stands — see [`follow_up_rounds`].
///
/// **A question the session before left standing is locked first.** A Blocking
/// Ask outlives the session that asked it, and nobody is ever handed somebody
/// else's — so a question left over from the follow-up that died is one the
/// human could answer for ever with nothing reading it, and one this follow-up
/// would never end while it stood. Locked unanswered as the fresh session
/// starts, which is what a relaunched grilling does with its own. Only here: a
/// steer that opens a follow-up displaces what it displaces itself.
///
/// **Nor is this always another round.** A **Tinker** whose rounds are over
/// sits in Follow-up while the `submitting` session sent for its pull request
/// runs, and goes on sitting there where that session left none — `gh` logged
/// out, a push that failed. The component reads that off its start — the newest
/// round marked, work standing on the branches — and ends `committed` at once,
/// which with no pull request sends for one again. Which is the promise
/// [`asked_for_a_pull_request`] makes in its own words: *what they have then is
/// Resume, and a press is another go through here*.
pub(crate) async fn follow_up_again(state: AppState, conversation_id: i64, driving: Driving) {
    left_open(&state, conversation_id).await;

    follow_up_rounds(state, conversation_id, driving).await
}

/// The human has said there is nothing else: end the session, and land the
/// Conversation wherever this follow-up ends.
///
/// **Two endings, and the pull request is what tells them apart.** A follow-up
/// steered into is something taken up about work that is already on one, so where
/// it ends is where it started — see [`back_to_the_wrap_up`]. A **Tinker** starts
/// straight into Follow-up on a branch nobody has opened anything on, and what
/// its rounds committed is the whole of what decides: anything at all means the
/// pull request is sent for and the ordinary wrap-up runs over what is opened,
/// and nothing anywhere means Done.
///
/// **The branches decide, and not the `pushed` flag.** `pushed` is *more
/// commits than when this session launched*, and a follow-up picked up again
/// reads that baseline afresh, so a Tinker that commits, loses its session, is
/// relaunched and then ends on a round that committed nothing reads
/// `pushed: false`. Landing Done on that would leave the work on a branch with
/// no pull request and nothing watching it. So `committed` is what stands past
/// the commits the branches were cut from — the Conversation's own and every
/// read-write companion's — which the **Follow-up rounds** Activity reads and
/// answers as its Outcome; see [`follow_up_rounds`]. `pushed` goes on meaning
/// what it has always meant.
///
/// The session is ended first, because the Worktree is about to be handed to
/// whatever comes next: a review or a `submitting` session queueing behind an
/// agent that has nothing left to do would wait for a session Verkstead is
/// finished with.
async fn over(
    state: &AppState,
    conversation_id: i64,
    already: i64,
    committed: bool,
    driving: Driving,
) {
    tracing::info!(
        conversation_id,
        "the human has nothing else, so the follow-up is over and its session is being ended",
    );

    state.sessions.end(conversation_id).await;

    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to land a follow-up on",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation a follow-up was on failed");
            return;
        }
    };

    let Some(on_a_pull_request) = on_a_pull_request(state, &conversation).await else {
        return;
    };

    if on_a_pull_request {
        return back_to_the_wrap_up(state, conversation_id, already, driving).await;
    }

    if committed {
        tracing::info!(
            conversation_id,
            "the follow-up built something, here or in a companion, and the branch is on no \
             pull request, so a session is being sent to open one",
        );

        // Held across the session and the wrap-up it starts, which is what
        // [`asked_for_a_pull_request`] leaves behind it: dropping first would
        // leave a moment where a sweep could find the Conversation undriven.
        let _driving = driving;

        return asked_for_a_pull_request(state, conversation_id).await;
    }

    finished(state, conversation_id, driving).await
}

/// Land a follow-up back in the wrap-up it was opened over.
///
/// **Where a follow-up ends is where it started.** It is something taken up
/// about work that is already on a pull request, so what is left when it is over
/// is that pull request and a wrap-up to see it out — over whatever the branch
/// now holds, with the review left settled and the fix attempts forgotten, which
/// is exactly what a steer into Wrapping recomputes. *Back to Done* is that
/// wrap-up's own settling rule and nothing decided here.
///
/// **The checks go back to waiting where the follow-up pushed.** `already` is
/// where the Conversation's commits stood before the session started, so
/// standing past it now is a follow-up that gave GitHub a new run to make up its
/// mind about — and a settle standing over it is yesterday's green, which the
/// settling loop could reach Done on before the checks watcher's first poll had
/// looked. A follow-up that was questions and answers alone lands with
/// everything settled and passes straight through to Done. A marker that will
/// not read counts as
/// *pushed*, which is the right way round: the cost is one poll of GitHub, and
/// the cost the other way is a wrap-up finished over a suite nobody watched.
///
/// Which is the one place `pushed` is asked for at all, and the reason it is not
/// what a Tinker's ending reads — see [`over`].
async fn back_to_the_wrap_up(
    state: &AppState,
    conversation_id: i64,
    already: i64,
    driving: Driving,
) {
    let pushed = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed > already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a follow-up had committed failed");
            true
        }
    };

    match store::follow_up_over(&state.pool, conversation_id, pushed).await {
        Ok(store::Ending::Wrapped) => {}
        Ok(outcome) => {
            tracing::info!(
                conversation_id,
                ?outcome,
                "there was no follow-up left to end, so nothing was moved",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "landing a follow-up back in its wrap-up failed");
            return;
        }
    }

    tracing::info!(
        conversation_id,
        pushed,
        "the follow-up is over, so the Conversation is wrapping up again",
    );

    // The Timeline has a move on it and the card reads differently, and an open
    // page should say so without being reloaded.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    // Held until the wrap-up's watchers have registrations of their own, which
    // is what [`crate::wrapping::watching`] takes as it spawns them: dropping
    // first would leave a moment where a sweep could find the Conversation
    // undriven and stop what has just been started.
    let _driving = driving;

    crate::checks::afresh(state.clone(), conversation_id).await;
}

/// And land one in Done, because its rounds built nothing anywhere.
///
/// A Tinker that was questions and answers alone: there is no pull request to
/// carry anything to and nothing to carry — in its own repository or in any
/// companion beside it — so there is nothing for a wrap-up to be about and
/// nothing to dispatch. The move and the line on the Timeline are
/// the whole of it, and the Worktree stays as it is for any Done Conversation.
///
/// Nothing is pushed to the devices and nothing is stamped unseen either, which
/// is what parts this from a wrap-up settling: the human is the one who ended
/// this, a moment ago, by ticking **Nothing else** — see `crate::settling`, whose
/// news is for a milestone nobody was there for.
async fn finished(state: &AppState, conversation_id: i64, driving: Driving) {
    // Held until the move is written, which is what every driver here holds it
    // for: dropping first would leave a moment where a sweep could find the
    // Conversation undriven and stop it over a follow-up that had finished.
    let _driving = driving;

    match store::follow_up_done(&state.pool, conversation_id).await {
        Ok(store::Ending::Finished) => {}
        Ok(outcome) => {
            tracing::info!(
                conversation_id,
                ?outcome,
                "there was no follow-up left to end, so nothing was moved",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "finishing a follow-up that built nothing failed");
            return;
        }
    }

    tracing::info!(
        conversation_id,
        "the follow-up built nothing anywhere and its branch is on no pull request, so the \
         work is done",
    );

    // The Timeline has a move on it and the card reads differently, and an open
    // page should say so without being reloaded.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });
}

/// See out an investigating session: the one session an Investigating has,
/// run under the **Follow-up rounds** Activity configured for it, and the
/// driver the Conversation is held up by for as long as it runs.
///
/// `investigation` is what the session is started on — the question this
/// Investigating was opened with, and the rounds it has already been through
/// where it is being picked up again. See [`crate::investigations`], which is
/// where a press of Resume reads both back from.
///
/// **Follow-up rounds, not an Activity of its own** (ADR-0030, *Activity*):
/// the component started with the skill `verkstead/investigating`, the
/// evidence `investigated` and the prompt written here — the question under
/// *What I want found out* and nowhere else, as a Tinker's follow-up brief
/// is, an investigation building nothing. The component reads no question off
/// the Timeline for it, a follow-up's reading knowing nothing of a steer into
/// Investigating or an Investigate's Brief.
///
/// **A driver rather than an errand beside the work**, exactly as a follow-up's
/// session is: the registration it is handed says the Conversation is being
/// driven for as long as this runs, so nothing sweeps it as standing still
/// while the human is composing an answer on a phone.
///
/// **What ends it is the human saying there is nothing else**: the newest round
/// they answered carries the mark, the session says it is done, and the
/// Conversation lands back where it came from — see [`found_out`]. A signal
/// without the mark is refused so that the next round goes to them as a Set,
/// and the refusal is worded for an investigation rather than for a follow-up;
/// a signal over whatever scratch the Worktree holds is taken, that being what
/// the session was sent to write. That is the evidence's to hold rather than
/// the state's — see [`crate::done::Evidence::Investigated`], which carries
/// both. A session that finishes on a round already marked ends the same way;
/// see [`Lent`]'s reading of a gone session.
///
/// **Either Outcome is the investigation over.** The component reads the
/// branches as it does for a follow-up, `committed` or `nothing`, and an
/// investigation lands where it came from whichever it is, as it always has:
/// nothing it committed is carried anywhere by it.
///
/// **A session that is gone is a stop**, which the component raises in an
/// investigation's words, any question it left the human holding going off
/// with it. And one that will not ask is spoken to, the human told where it
/// will not answer, by the host's wait.
pub(crate) async fn investigating(
    state: AppState,
    conversation_id: i64,
    investigation: Investigation,
    driving: Driving,
) {
    // What the session before this one left standing, where there was one. A
    // Blocking Ask outlives the session that asked it, and nobody is ever
    // handed somebody else's — so a question left over from the session that
    // died is one the human could answer for ever with nothing reading it.
    // Locked unanswered as the fresh session starts, which is what a relaunched
    // grilling and a relaunched follow-up both do with their own.
    if investigation.again {
        left_open(&state, conversation_id).await;
    }

    let Some(documents) = documents(&state, conversation_id).await else {
        return;
    };

    let prompt = skills::investigating(
        documents.skills,
        &investigation.brief,
        &investigation.settled,
    );

    let Some((shipped, activity)) = built_in(FOLLOW_UP_ROUNDS) else {
        tracing::error!(
            conversation_id,
            "this build carries no Follow-up rounds Activity, so no investigation was started"
        );

        return stop(
            &state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            INVESTIGATING,
            "This build of Verkstead can't run an investigation.",
            None,
        )
        .await;
    };

    let options = vec![
        ("skill".to_owned(), "verkstead/investigating".to_owned()),
        (
            "evidence".to_owned(),
            crate::lending::evidence_option(&crate::done::Evidence::Investigated),
        ),
        ("prompt".to_owned(), prompt),
    ];

    let outcome = crate::activities::run(
        &state,
        conversation_id,
        &driving,
        shipped.files(),
        &activity,
        &[],
        options,
        INVESTIGATING,
        crate::continuing::Brought::Settle,
    )
    .await;

    // `driving` is held until whatever this writes is written, and handed to
    // the ending, because what the Conversation lands in may be a state
    // something has to go on driving.
    if let Some("committed" | "nothing") = outcome.as_ref().map(|ran| ran.outcome.as_str()) {
        found_out(&state, conversation_id, driving).await;
    }
}

/// What an investigating session is said to be doing, by the rescue and by a
/// stop over it.
const INVESTIGATING: &str = "finding out what was asked";

/// The human has nothing else they want found out: end the session, and land the
/// Conversation back where the investigation came from.
///
/// **Where it came from, which is one of two things.** An Investigate
/// Conversation — one whose Process is Investigate, which ran Draft to
/// Investigating — ends **Done**: there is nowhere for it to go back to, the
/// investigation being the whole of the work. One *steered* into Investigating
/// goes back to the state the steer found it in, with nothing else about it
/// changed — its settles and its pull request exactly as they were, because
/// nothing here touches either. Which of the two this is, and which state, is
/// [`crate::investigations::landing`]'s to say: it reads a fact somebody recorded
/// beside the Steer Event rather than inferring one from the Timeline's shape.
///
/// A landing that cannot be read is a landing in Done, which is the safe way
/// round: an investigation that ended is over whatever the record will say about
/// it, and Done is the state nothing has to be driving. Reading it *before* the
/// move, so that a store that will not answer is a Conversation left in
/// Investigating with its session still there rather than one moved nowhere in
/// particular.
///
/// **Nothing is dispatched for a landing in Done**, there being nothing to drive:
/// no wrap-up, no watchers, no `submitting` session and no pull request sent for.
/// An investigation writes nothing down on the branch, so there is nothing for a
/// wrap-up to be about and nothing to carry anywhere, and the Worktree stays as it
/// does for any Done Conversation — with whatever scratch the session left in it,
/// which every Set's Diff has already shown.
///
/// **And for any other landing, whatever a pressed Resume would be driving it
/// with.** A Conversation put back into Wrapping or Implementing with nothing
/// driving it is exactly what the stall sweep raises a stop about, so the ending
/// starts the drive rather than leaving it to be found — see
/// [`crate::resume::landed`], which is the press's own recompute and already
/// exhaustive over the states. The registration is held across it and handed on,
/// as every driver here hands one on.
///
/// **Nothing is pushed to the devices and nothing is stamped unseen either**,
/// which is what parts this from a wrap-up settling: the human ended this
/// themselves a moment ago by ticking **Nothing else**. See [`finished`], which is
/// the same ending a Tinker that built nothing gets, for the same reason.
///
/// The session is ended first, as a follow-up's is, so that nothing is left
/// holding the Worktree while the move is written.
async fn found_out(state: &AppState, conversation_id: i64, driving: Driving) {
    let landing = match crate::investigations::landing(&state.pool, conversation_id).await {
        Ok(landing) => landing,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading where an investigation came from failed, so it is ending Done");
            crate::investigations::Landing {
                state: store::Lifecycle::Done,
                steered: None,
            }
        }
    };

    tracing::info!(
        conversation_id,
        ?landing,
        "the human has nothing else they want found out, so the investigation is over and its \
         session is being ended",
    );

    state.sessions.end(conversation_id).await;

    match store::investigation_over(&state.pool, conversation_id, landing.state).await {
        Ok(store::Investigated::Landed) => {}
        Ok(outcome) => {
            tracing::info!(
                conversation_id,
                ?outcome,
                "there was no investigation left to end, so nothing was moved",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "landing an investigation that is over failed");
            return;
        }
    }

    tracing::info!(
        conversation_id,
        ?landing,
        "the investigation is over, so the Conversation is back where it came from",
    );

    // The Timeline has a move on it and the card reads differently, and an open
    // page should say so without being reloaded.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    // An Investigate Conversation, or one steered in from a Draft or from a close:
    // Done is where the work has got to and nothing is supposed to be driving it.
    // Its scratch stays where the session wrote it, as it does for any Done
    // Conversation — the Worktree goes with the close, and nothing is going to
    // work in there before then.
    if landing.state == store::Lifecycle::Done {
        drop(driving);
        return;
    }

    // Every other landing hands the checkouts back to sessions that commit and
    // push, so what the investigation wrote in them comes out first — after the
    // move, which is what says this investigation is the one that ended, and
    // before the driver below, which is the first thing that would work in there.
    // See [`crate::investigations::tidied`].
    crate::investigations::tidied(state, conversation_id, landing.steered).await;

    // Held across the recompute and handed to whatever it starts, which is what
    // [`crate::resume::landed`] takes it for: dropping first would leave a moment
    // where a sweep could find the Conversation in a driven state with nothing
    // driving it.
    match crate::resume::landed(state, conversation_id, driving).await {
        Ok(Resumed::Resumed) => {}
        // Nothing was started, and the Conversation is left in a state something
        // is supposed to be driving. Said here rather than written down: there is
        // nobody in front of this to answer, and the stall sweep is a minute away
        // with a sentence that names the state — see [`crate::stalls::sweeping`].
        Ok(refusal) => {
            tracing::error!(
                conversation_id,
                ?landing,
                ?refusal,
                "an investigation landed where it came from and nothing could be started to \
                 drive it",
            );
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, ?landing, "starting to drive what an investigation landed in failed");
        }
    }
}

/// Whether the Conversation's own work is on a pull request, or `None` where the
/// record would not say.
///
/// The Conversation's own repository's, which is the pull request that makes a
/// wrap-up: a companion's is something a wrap-up covers rather than something to
/// end into. A steer into Follow-up is refused without one, so this is exactly
/// the question *is this a Tinker's branch* — asked of the record, because a
/// Tinker that has already been round once is on a pull request and ends the
/// ordinary way.
///
/// Asked by the ending — see [`over`] — which lands nothing it cannot be sure
/// about, so `None` there lands nothing at all.
async fn on_a_pull_request(state: &AppState, conversation: &store::Conversation) -> Option<bool> {
    match store::pull_request(&state.pool, conversation.id, conversation.repo.id).await {
        Ok(found) => Some(found.is_some()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = conversation.id, "reading whether a follow-up's work is on a pull request failed");
            None
        }
    }
}

/// Take off any Question Set the follow-up left standing on the Conversation.
///
/// Verkstead reaching for the lock on the human's behalf because it knows
/// something they cannot see — that there is nobody behind the question any
/// more — exactly as a wrap-up closes what its gone session left open. See
/// [`crate::review::closed`].
///
/// The Conversation's rather than the session's, which is what the follow-up's
/// own rule asks everywhere: what matters is whether the human is left holding a
/// question, and a question is one whoever put it up.
///
/// Asked at both ends of a follow-up that lost its session: as the stop over one
/// is raised, and again as a fresh session is started over one nothing stopped —
/// a restart leaves no stop behind, so what a dead session left standing is
/// still standing when the next server picks the follow-up up.
pub(crate) async fn left_open(state: &AppState, conversation_id: i64) {
    let standing = match store::open_set(&state.pool, conversation_id).await {
        Ok(standing) => standing,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a follow-up left open failed");
            return;
        }
    };

    if let Some(set_id) = standing {
        crate::review::closed(state, conversation_id, set_id).await;
    }
}

/// Whether anything on the Conversation is still waiting on the human.
///
/// The Conversation's rather than any one session's, which is the question both
/// its readers are really asking: what matters is whether the human is left
/// holding something to answer, and something to answer is one whoever put it
/// up. See [`crate::rescues::watched`], which is the other reader
/// — a session with a question standing in front of the human is not one to
/// prod, whichever session wrote it.
///
/// A Deferred Ask is not one of them and neither is a Set the human closed
/// unanswered: both are Sets nobody is idling on. See [`store::open_set`].
///
/// A store that will not answer reads as *open*, which is the right way round
/// for what it decides: on the other side is a session being ended and a
/// Conversation moved, and doing either over a question nobody has answered
/// would take the answer away from the agent that asked for it.
pub(crate) async fn open(state: &AppState, conversation_id: i64) -> bool {
    match store::open_set(&state.pool, conversation_id).await {
        Ok(open) => open.is_some(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a Conversation was still asking the human anything failed");
            true
        }
    }
}

/// Whether the newest round the human answered inside `within`'s own window
/// carries the Nothing-else mark.
///
/// `within` is the state the rounds are being had in — [`Lifecycle::FollowUp`]
/// for a follow-up, [`Lifecycle::Investigating`] for an investigation — because
/// each windows its rounds by the newest move into itself and a Conversation can
/// have been through both. See [`store::nothing_else`].
///
/// A store that will not answer reads as *not marked*, which leaves the rounds
/// running: the same way round as [`open`], read from the other side.
pub(crate) async fn marked(
    state: &AppState,
    conversation_id: i64,
    within: store::Lifecycle,
) -> bool {
    match store::nothing_else(&state.pool, conversation_id, within).await {
        Ok(marked) => marked,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, ?within, "reading whether the rounds were over failed");
            false
        }
    }
}

/// Follow the grilling session as it stages the work into a roadmap.
///
/// See a roadmap session out, and carry the Conversation on to wrapping the
/// pull request it opened.
///
/// The whole of a roadmap Conversation's own work in one session — the stages it
/// plans are Conversations of their own — so there is no next step to launch.
/// What there is, is the same ending a backlog's last step has: the session
/// commits the roadmap and then follows the repository's own finish sequence, so
/// the branch is pushed and on a pull request by the time it signals. A
/// roadmap is work like any other work and goes for review like any other work.
///
/// So the ladder for a roadmap Conversation is Grilling to Wrapping, with nothing
/// in between. Implementing is where an agent is building the work, and on a
/// roadmap the building belongs to the Stages: this Conversation's own work is
/// the planning, which is the grilling carrying on. The move is
/// [`crate::wrapping::opened`]'s, made as the pull request is recorded — and a
/// roadmap that was committed and never pushed gets the go every other ending
/// gets, the roadmap being this Conversation's whole work and a pull request
/// being what it is finished by. See [`to_a_pull_request`].
///
/// No handoff anywhere in it, and none in a task list either. A handoff is for
/// a context boundary the work actually crosses, and a roadmap crosses none:
/// what the grilling settled is in the stage briefs it committed, and each Stage
/// is a Conversation with a grilling of its own.
///
/// `launched` is the session Resume launched to commit the roadmap — the
/// grilling session's own is seen out by the interview instead, see
/// [`interview`]. `base` is the commit the branch came
/// off, which is what says a roadmap on it is one this branch wrote — see
/// [`Landing::Roadmap`].
///
/// A session that ends without writing one stops the run, the way every other
/// step does.
async fn follow_roadmap(
    state: AppState,
    conversation_id: i64,
    base: String,
    launched: (Lent, u64),
    _driving: Driving,
) {
    let Some(writing) = see_out(
        &state,
        conversation_id,
        Step::Staging(base.clone()),
        launched,
    )
    .await
    else {
        return;
    };

    staged(&state, conversation_id, &base, writing).await;
}

/// Carry a roadmap Conversation on from its committed roadmap to wrapping the
/// pull request the same session opened, `writing` being the Event that session
/// printed into.
async fn staged(state: &AppState, conversation_id: i64, base: &str, writing: i64) {
    // The roadmap is on the branch, so the record says where that happened —
    // before the pull request the same session went on to open, which is the
    // order the two happened in. Which roadmap this Conversation wrote is
    // settled in the same call, off the Worktree it was written in.
    let worktree = worktree(state, conversation_id).await;

    crate::conversations::roadmap_landed(state, conversation_id, worktree.as_deref(), base).await;

    to_a_pull_request(state, conversation_id, Some(writing)).await;
}

/// What a session that proposes and then fixes left behind — the wrap-up's one
/// review, and each batch of comments answered after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reviewed {
    /// It finished: it read what it was sent to read, put what it would do to the
    /// human, landed what they accepted, and ran out of things to say. One that
    /// found nothing worth raising finishes the same way, having said so as the
    /// last thing it printed.
    ///
    /// Which is nearly always Verkstead ending it on its Done signal rather than
    /// the session exiting — an interactive agent idles when its work is done.
    /// Both are this: see [`proposing`].
    Done,

    /// It ended, and not well. Which is not a session with nothing left to do —
    /// this is one that did not finish.
    Stopped {
        /// How it ended, in the words the Notice records.
        how: String,

        /// The Timeline Event it was printing into.
        writing: i64,
    },

    /// Nothing to say about it: no session could be started, or the run was
    /// stopped from outside while it ran.
    Nothing,
}

/// Run the one review session a wrap-up gets, and wait until it is over.
///
/// One session however many pull requests the work ended up on: `on` is every one
/// of them, so that the session reading the work whole knows where the whole of
/// it is — see [`crate::review::across`].
///
/// `said` is what was written on those pull requests before this started, which
/// the caller reads inside the Turn it is holding and records as addressed — so
/// this session is the one that proposes about it, and nothing else is sent to.
pub(crate) async fn review(
    state: &AppState,
    conversation_id: i64,
    on: Option<String>,
    said: Option<String>,
) -> Reviewed {
    proposing(
        state,
        conversation_id,
        "verkstead/reviewing",
        store::Role::Review,
        |documents| {
            skills::reviewing(
                documents.skills,
                &documents.brief,
                documents.handoff(),
                on.as_deref(),
                said.as_deref(),
            )
        },
        "review",
    )
    .await
}

/// Run one batch session, and wait until it is over.
///
/// The review's shape exactly — it proposes about what was said, waits on the
/// human, and lands what they accepted — about a batch of comments rather than
/// about the branch. `said` is that batch, which the caller reads and records as
/// addressed inside the Turn it is holding.
pub(crate) async fn respond(state: &AppState, conversation_id: i64, said: &str) -> Reviewed {
    proposing(
        state,
        conversation_id,
        "verkstead/responding",
        store::Role::Implementation,
        |documents| {
            skills::responding(
                documents.skills,
                &documents.brief,
                documents.handoff(),
                said,
            )
        },
        "batch session",
    )
    .await
}

/// Run one session that proposes and then fixes, and wait until it is over.
///
/// **Ended because it said it was done** — see [`crate::done`], and ADR-0018 —
/// and once it is next idle after that, so what it says on the way out is kept.
/// One of these puts what it would do to the human in the middle of its work and
/// has the rest of that work to do afterwards, so there is no landing to check
/// the signal against and no commit it must make: a review that fixes nothing is
/// a review that did its job. What holds is what every signal has — nothing
/// left uncommitted, and a Set of its own still open locked as it is taken.
///
/// **Nothing ends one on quiet.** It used to be ended on a minute of silence
/// with nothing of its own open, that being the only report there was, and a
/// review that started a background check run and ended its turn was ended
/// mid-review — leaving what it had got to standing as the whole of it. Now a
/// quiet one is left running, however long the quiet, and a session idling on
/// a Blocking Ask of its own is left alone as it always was: `verkstead ask`
/// blocks for as long as the human takes.
///
/// **And one that will not ask is spoken to**, until it signals — see
/// [`crate::rescues`], which tells the human where it will not answer and leaves
/// the session running.
///
/// A session ended on its signal **is a session that finished**: it is
/// [`Reviewed::Done`], exactly as one that saw itself out cleanly is. Where the
/// session ends first, how it ended is read exactly as an inline run's is:
/// cleanly means it did what it was sent to do, and anything else means it did
/// not. What to do about either is the caller's, and both callers ask the same
/// further question first: whether anything the human accepted was left
/// unlanded. See [`crate::review`] and [`crate::responding`].
///
/// `what` names the session in the log, which is the one place the two differ
/// here.
async fn proposing(
    state: &AppState,
    conversation_id: i64,
    skill: &str,
    role: store::Role,
    prompt: impl FnOnce(&Documents<'_>) -> String,
    what: &'static str,
) -> Reviewed {
    let Some(documents) = documents(state, conversation_id).await else {
        return Reviewed::Nothing;
    };

    // No signal given and nothing open, with the session sitting there: a
    // review or a batch nobody can move on. Spoken to, and the human told where
    // it will not answer — see [`crate::rescues`]. The wrap-up dispatching it
    // holds the Turn already.
    let mut lent = Lent::new(state, conversation_id).turn_held().doing(what);

    // Nothing for the signal to be checked against beyond the rules every signal
    // has: a review that finds nothing, or a batch that asks for nothing, is
    // still one done. See [`crate::done::Evidence::Nothing`].
    let Some(session) = lent
        .start(
            skill,
            role,
            prompt(&documents),
            crate::done::Evidence::Nothing,
            crate::done::Ends::WithItsWork,
        )
        .await
    else {
        return Reviewed::Nothing;
    };

    let event_id = lent.event_id(session);

    let Some(ended) = lent.see(session).await else {
        tracing::info!(
            conversation_id,
            event_id,
            what,
            "the session said it is done and has gone quiet since, so it is being ended",
        );

        state.sessions.end(conversation_id).await;

        return Reviewed::Done;
    };

    // Verkstead ended it — the human closed or force-stopped the Conversation
    // out from under the wrap-up, or the account ran out of window. Either way
    // the stop is already on the record, so there is nothing to ask about. See
    // [`crate::sessions::Ended::on_purpose`].
    if ended.on_purpose() {
        tracing::info!(
            conversation_id,
            event_id,
            what,
            "the session was stopped from outside, so nothing is asked about it"
        );
        return Reviewed::Nothing;
    }

    match ended.badly() {
        Some(how) => Reviewed::Stopped {
            how,
            writing: event_id,
        },
        None => Reviewed::Done,
    }
}

/// Whether the Conversation's commits stand past the `already` a session started
/// over.
///
/// The store rather than git, unlike a backlog step's landing: the branch
/// watcher is sweeping this branch for as long as the session runs and putting
/// what lands on the Timeline, and asking it costs one small read where asking
/// git costs a process. A reader that has to see a commit made a moment ago
/// sweeps first — see [`crate::commits::sweep_now`].
///
/// **Past rather than more than**, because a sweep takes commits off the
/// Timeline as well as putting them on — see [`store::commits_landed`], which is
/// what makes a marker rather than a count the thing to compare. A session that
/// rebased the branch or amended its commit left the same work under new shas,
/// which is a session that committed: counted, it would come back to the number
/// it started at, and this would wait for ever on a session that had done
/// exactly what it was sent for.
///
/// A store that will not answer reads as *nothing new*, which is the right way
/// round for what this decides: a Done signal refused, which the session can
/// simply give again. See [`crate::done::Evidence::Committed`].
pub(crate) async fn committed_since(state: &AppState, conversation_id: i64, already: i64) -> bool {
    match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed > already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a session committed failed");
            false
        }
    }
}

/// Stop the run: record the stop and put what stopped on the Timeline.
///
/// `decided` is who stopped it, which is what a restart reads and what decides
/// whether a phone is told — see [`crate::stopping::Decided`]. A step whose session
/// ended without landing it is Verkstead's own: it pulled the brake, and it does
/// not spend an account on the same failure again unasked.
///
/// `writing` is the Timeline Event the session that failed was printing into, and
/// `None` where there is no session left to read one off — a restart, which kills
/// every session it had and leaves the Conversation's row behind.
///
/// Nothing is refused for. By the time this runs the session is gone and the step
/// has not landed, and a stop that could not be recorded is a run stopped with
/// nothing saying so — which is a thing to see in the log, and the same thing
/// either way: the runner returns.
pub(crate) async fn stop(
    state: &AppState,
    conversation_id: i64,
    decided: crate::stopping::Decided<'_>,
    what: &str,
    how: &str,
    writing: Option<i64>,
) {
    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        decided,
        what,
        how,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a run stopped and the stop saying so could not be recorded"
        );
    }
}

/// See one step's session out: end it once it has said it is done and gone
/// quiet since, and say whether the step landed at all.
///
/// What its Done signal is checked against is the step's own [`Landing`],
/// written down for the length of the watch — see [`crate::done`]. A session
/// that lands its step and says nothing is left running, and the rescue is what
/// speaks to it.
///
/// `None` is a session that is over with its step not done. That is a crash, a
/// hang given up on, or an agent that stopped short — which of them is not
/// something to guess at here, and none of them is a reason to launch the same
/// step again on its own. The run stops, and it is the human who decides whether
/// the step gets another run.
///
/// **A session that hangs is one of those**, and until it is spoken to it is
/// none of them: it has not crashed and it has not stopped short, it is sitting
/// there with the turn finished. So it is told what it cannot see from inside —
/// and where it will not answer the human is told, the session left running.
/// See [`crate::rescues`], whose done-indicator here is the
/// session's own Done signal.
///
/// `Some` is the Timeline Event the session printed into. The step landed, and
/// what comes after it may still want the session's own last words — the finish
/// step's does, because a stop over what the finish left behind is explained from
/// them.
async fn see_out(
    state: &AppState,
    conversation_id: i64,
    step: Step,
    (mut lent, session): (Lent, u64),
) -> Option<i64> {
    let event_id = lent.event_id(session);

    let landing = step.landing();

    let worktree = worktree(state, conversation_id).await?;

    // What the session's Done signal is checked against — its landing, which it
    // was launched expecting — is written down for as long as it is
    // being seen out, and taken away with this if the watch is cancelled, which
    // a later pick does. See [`crate::done`]. A session that has not said it is
    // done and is not asking about anything is spoken to, and the human told
    // where it will not answer — see [`crate::rescues`].
    let ended = lent.see(session).await;

    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            step = ?step,
            "a session said its step is done and has gone quiet since, so it is being ended",
        );

        state.sessions.end(conversation_id).await;
        return Some(event_id);
    };

    // The session is over. It may have landed its step as its last act and
    // exited before a poll caught it, which is the ordinary shape of a session
    // that finishes rather than idles — so the Worktree is asked once more
    // before this is read as a run that has stopped. Asked whichever way the
    // session ended: a step that landed is landed, and an agent that did the
    // work and then fell over on its way out has not left the human anything to
    // decide about.
    if check(&worktree, &landing).await {
        return Some(event_id);
    }

    // Verkstead ended it — the human closed the Conversation out from under
    // the run, so the worktree has gone and the step reads as not landed
    // whatever it did; or they force-stopped it; or the account it was spending
    // ran out of window. Every one of the three has already written the stop
    // this would otherwise write, so the backlog stops here without asking. See
    // [`crate::sessions::Ended::on_purpose`].
    if ended.on_purpose() {
        tracing::info!(
            conversation_id,
            event_id,
            step = ?step,
            "the run was stopped from outside, so the backlog stops here without asking",
        );
        return None;
    }

    tracing::warn!(
        conversation_id,
        event_id,
        step = ?step,
        "a session ended without finishing its step, so the backlog stops here",
    );

    // How it ended, where the ending itself was the problem; otherwise the step
    // not landing is, which is the case no exit status could have shown — an
    // interactive agent that decides there is nothing to do exits zero.
    let how = ended
        .badly()
        .unwrap_or_else(|| "The session ended without finishing the step.".to_owned());

    stop(
        state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        &step.what(),
        &how,
        Some(event_id),
    )
    .await;

    None
}

/// Wait until the session has given its Done signal and been accepted, and is
/// idle again since.
///
/// Idle by the judgement everything here reads, which is its backend's own — see
/// [`crate::sessions::Idle`] — and [`Pace::grace`] out since the signal too, so the
/// closing words an agent prints once the command comes back reach the
/// Transcript rather than being cut off under it. Output arriving in the
/// meantime lengthens the wait rather than ending it.
///
/// Nothing on the branch is read here. The signal was checked against the
/// repository when it was given, and that is the only reading there is: a
/// session that lands its step and says nothing is a session still at work, and
/// the rescue is what speaks to it.
pub(crate) async fn signalled_and_idle(signal: crate::done::Signal, idle: &Idle, pace: Pace) {
    signal.arrived().await;

    let signalled = Instant::now();

    loop {
        let owed = pace
            .grace
            .saturating_sub(idle.for_how_long())
            .max(pace.grace.saturating_sub(signalled.elapsed()));

        if owed.is_zero() {
            return;
        }

        tokio::time::sleep(owed).await;
    }
}

/// Whether `landing` has landed, off the runtime's threads: a directory read and
/// a `git status` of one path.
pub(crate) async fn check(worktree: &Path, landing: &Landing) -> bool {
    let worktree = worktree.to_owned();
    let landing = landing.clone();

    match tokio::task::spawn_blocking(move || landed(&worktree, &landing)).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, "asking a Worktree whether a step had landed failed");
            false
        }
    }
}

/// The same, blocking: the path is where it should be, and git has nothing
/// pending for it.
///
/// A repository that will not answer reads as *not landed*, which is the right
/// way round for the one thing this decides: a session is ended on the strength
/// of this, and a git that was briefly busy is no reason to end one.
fn landed(worktree: &Path, landing: &Landing) -> bool {
    lacking(worktree, landing).is_none()
}

/// What `landing` is still missing, in words an agent can act on, or `None`
/// where it has landed — see [`crate::done`], which refuses a Done signal with
/// exactly this.
pub(crate) async fn missing(worktree: &Path, landing: &Landing) -> Option<String> {
    let worktree = worktree.to_owned();
    let landing = landing.clone();

    match tokio::task::spawn_blocking(move || lacking(&worktree, &landing)).await {
        Ok(missing) => missing,
        Err(error) => {
            tracing::error!(error = ?error, "asking a Worktree what a step is missing failed");
            Some("the repository could not be read, so try again in a moment".to_owned())
        }
    }
}

/// The same, blocking, and the one reading [`landed`] is made of.
fn lacking(worktree: &Path, landing: &Landing) -> Option<String> {
    let (path, wanted) = match landing {
        Landing::Gone(path) => (path, false),
        Landing::Arrived(path) => (path, true),
        // Not a path at all but a line inside one, so what is asked is the
        // list: the entry's box ticked, and the commit that ticked it landed.
        Landing::Ticked(number) => {
            let ticked = crate::tasks::entries(&worktree.join(BACKLOG)).is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry.number == *number && entry.checked)
            });

            return match (ticked, pending(worktree, &todo())) {
                (true, Some(false)) => None,
                (false, _) => Some(format!(
                    "task {number}'s box in `{}` is not ticked and committed",
                    todo().display(),
                )),
                (true, _) => Some(format!(
                    "task {number}'s box in `{}` is ticked but not committed",
                    todo().display(),
                )),
            };
        }
        // Not a path this branch was told about but one it went and wrote, so
        // what is asked is which roadmaps it has touched — the same reading the
        // pinned stage list is drawn by, so the list the human is watching and
        // the step the runner is waiting on cannot disagree.
        Landing::Roadmap(base) => {
            let touched = crate::stages::touched(worktree, base);

            if touched.is_empty() {
                return Some(format!(
                    "this branch has not written a roadmap under `{}`",
                    crate::stages::ROADMAPS,
                ));
            }

            if pending(worktree, Path::new(crate::stages::ROADMAPS)) != Some(false) {
                return Some(format!(
                    "the roadmap under `{}` is not committed",
                    crate::stages::ROADMAPS,
                ));
            }

            // And the third thing asked of a landed roadmap: that what it
            // declares is a roadmap something could run. Written and committed is
            // not enough where the lines declare badly — a declaration on some
            // lines and not others, an `after` naming no stage of the roadmap, a
            // platform that is not one, or a cycle — because the session that
            // wrote it is the one that can put it right, and it is alive until
            // this signal is taken. See [`crate::declarations::judge`], whose
            // sentence is said as it is: what the scheduler will show is the same
            // one.
            return crate::stages::misdeclared(worktree, &touched);
        }
        // And this one is not in the Worktree at all, so there is no commit to
        // wait for: the document being there with something in it is the whole
        // of it, read by exactly the rule that will take it.
        Landing::Handoff(path) => {
            // Named the way the skill names it rather than by this path, which
            // is where the document is from outside the session and nowhere the
            // session can see.
            return (!crate::handoffs::written(path)).then(|| {
                "the handoff document has not been written where the grilling skill says to \
                 write it"
                    .to_owned()
            });
        }
    };

    if worktree.join(path).exists() != wanted {
        return Some(match wanted {
            true => format!("`{}` has not been written", path.display()),
            false => format!("`{}` has not been taken away", path.display()),
        });
    }

    match pending(worktree, path) {
        Some(false) => (),
        _ => {
            return Some(match wanted {
                true => format!("`{}` is not committed", path.display()),
                false => format!("taking `{}` away is not committed", path.display()),
            });
        }
    }

    // And a backlog that has landed is asked what a roadmap is asked beside its
    // landing: whether the platforms its lines name are platforms. Refused here
    // rather than at launch, because the session that wrote the list is the one
    // alive to put it right, and nobody who wrote it is there when a task starts.
    // See [`crate::declarations::judge_backlog`].
    if wanted && *path == todo() {
        let list = std::fs::read_to_string(worktree.join(path)).ok()?;

        return crate::declarations::judge_backlog(&list);
    }

    None
}

/// Whether git has any pending change for `path`, or `None` where it cannot say.
///
/// One path rather than the whole Worktree: what is being asked is whether the
/// commit for *this* step has landed, and a session that is part way through the
/// next piece of work would make a whole-Worktree answer say no forever.
fn pending(worktree: &Path, path: &Path) -> Option<bool> {
    let path = path.to_string_lossy().into_owned();

    // `--` rather than `--end-of-options`: what follows it is a pathspec, which
    // is what this is asking about, and it is git's own name for a path.
    let said = git(worktree, &["status", "--porcelain", "--", &path])?;

    Some(!said.trim().is_empty())
}

/// The stage whose backlog was never planned, and what its branch was made on
/// top of.
///
/// The one step a stage has that no other Conversation has: its first, run in
/// the fork of next-stage, which is what writes the `.tasks/` every step after
/// it works through. It is launched as the stage is made and by nothing else —
/// [`crate::continuing`] where the stage before it settled, and
/// [`crate::conversations::adopt`] where a human adopted the roadmap — so a
/// planning session that died before it committed leaves a Conversation
/// implementing a backlog that was never written, with nothing in Verkstead that
/// would ever write one. That is a stage stuck for good, and it is stuck under
/// both readings that exist for getting a run going again: [`nothing_left`]
/// stops it, and a pressed Resume refuses it by name.
///
/// So it is asked for here. `true` is that stage, and `false` every other
/// Conversation: one that is not a stage, one whose backlog has been written
/// already, and one with no base commit to read a branch's writing against.
///
/// Asked of the repository rather than of the record, by the rule the rest of
/// this module reads a run's position by — what a branch has written is the
/// branch's own to say, and it is the same answer however the Conversation got
/// here. A git that will not answer says *written*, which is the right way round
/// for the one thing this decides: a stage planned a second time over a backlog
/// that is already there would be an agent let loose on somebody else's work.
pub(crate) async fn stage_to_plan(
    state: &AppState,
    conversation_id: i64,
    worktree: &Path,
    base: Option<&str>,
) -> bool {
    // No base commit is a Conversation that never branched, which is not a stage
    // — a stage is made by branching — and is not something a branch's own
    // writing can be read against either.
    let Some(base) = base else {
        return false;
    };

    match store::stacks_on(&state.pool, conversation_id).await {
        // Whether this is a stage at all; what it was cut from is the
        // component's to be told — see [`backlog_runner`].
        Ok(Some(_)) => {}
        Ok(None) => return false,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading whether a Conversation is a roadmap stage failed",
            );

            return false;
        }
    }

    !wrote_a_backlog(worktree, Some(base)).await
}

/// [`backlog_written`] as the two things that turn on it ask it: off the
/// runtime's threads, and *written* wherever it cannot be read at all.
///
/// A git that will not answer and a Conversation with no base commit are the same
/// unreadable branch, and every caller wants the same thing said about one. A
/// stage read as unplanned would be planned again over somebody else's backlog;
/// an emptied backlog read as never written would stop a run that has work on the
/// branch and one push to go, and refuse the press that would have finished it.
/// *Written* is the careful answer to all three.
pub(crate) async fn wrote_a_backlog(worktree: &Path, base: Option<&str>) -> bool {
    let Some(base) = base else {
        return true;
    };

    let worktree = worktree.to_owned();
    let base = base.to_owned();

    tokio::task::spawn_blocking(move || backlog_written(&worktree, &base))
        .await
        .unwrap_or(true)
}

/// Whether this branch has written a backlog since `base`.
///
/// Two questions, as [`crate::stages::touched`] asks two, because git answers
/// them separately: what the history holds — every commit since the base that
/// added or changed a file under `.tasks/` — and what is in the Worktree that no
/// commit has taken yet. A backlog that was written and then finished with is in
/// the first, because the plan commit that added it is; one a session wrote and
/// died before committing is in the second. A stage that never planned is in
/// neither.
///
/// A deletion is not a writing, in either question. Verkstead clears a list a
/// branch inherited from its base before any session runs, so a branch whose
/// whole `.tasks/` story since the base is a removal has planned nothing — and
/// reading that removal as a plan would leave a stage with a list nobody wrote,
/// nothing to work, and no way ever to be planned again. So the history counts
/// only commits that add or change a file there, and the Worktree counts only
/// pending additions and modifications: a staged or unstaged deletion is neither.
///
/// Since `base` rather than over the whole history, because a stage's branch is
/// stacked on the branch of the stage before it: the predecessor's backlog and
/// the finish that emptied it are commits this branch is descended from, and a
/// reading that counted those would say every stage had planned already.
///
/// A repository that will not answer says *written* — see [`stage_to_plan`] for
/// why that is the safe way round.
fn backlog_written(worktree: &Path, base: &str) -> bool {
    let since = format!("{base}..HEAD");

    // `--` rather than `--end-of-options`: what follows it is a pathspec, which
    // is git's own name for a path, and the base is a commit Verkstead resolved
    // itself rather than anything a human typed here.
    let committed = git(
        worktree,
        &[
            "log",
            "--format=%H",
            "--diff-filter=AM",
            &since,
            "--",
            BACKLOG,
        ],
    );

    let uncommitted = git(worktree, &["status", "--porcelain", "--", BACKLOG]);

    match (committed, uncommitted) {
        (Some(committed), Some(uncommitted)) => {
            !committed.trim().is_empty() || uncommitted.lines().any(writes)
        }
        _ => true,
    }
}

/// Whether one `git status --porcelain` line is a file put there rather than
/// taken away.
///
/// A line is `XY path`: what the index holds against the base, and what the
/// working tree holds against the index. A deletion is a `D` in one of them, an
/// untouched half is a space, and everything else — an addition, a modification,
/// a rename, an untracked file's `?` — is something written that is still there
/// to read. So a line counts wherever either letter is neither.
fn writes(line: &str) -> bool {
    line.bytes()
        .take(2)
        .any(|code| !matches!(code, b'D' | b' '))
}

/// Whether `.tasks/` has anything left in it to work: a list, ticked or not.
///
/// The one thing about a backlog anybody outside the **Backlog runner** asks:
/// Resume refuses by name where there is nothing to launch a session for, and
/// a run taken up again over an empty one asks GitHub what became of it rather
/// than starting the component — see [`implementing_again`]. Which box is next
/// is the component's own reading. A list is read by the same rule the pinned
/// Event is drawn by — see [`crate::tasks`].
pub(crate) async fn anything_to_work(worktree: &Path) -> bool {
    let backlog = worktree.join(BACKLOG);

    match tokio::task::spawn_blocking(move || crate::tasks::entries(&backlog).is_some()).await {
        Ok(listed) => listed,
        Err(error) => {
            tracing::error!(error = ?error, "reading whether there is a backlog failed");
            false
        }
    }
}

/// The Pairing a session on `role` is launched under, of what the Conversation
/// has settled — or `None` where there is no account to launch one under at
/// all.
///
/// The Review role reads its own and **falls back to the Implementation Pairing
/// where nothing was ever picked for it**. Not a default: the picker is
/// answered before the work starts, and a Conversation started since the role
/// existed always has one. What it is for is the two ways of reaching a wrap-up
/// with the role never picked, neither of which the human can put right — the
/// pickers freeze when the work starts, and there is no review picker anywhere
/// else:
///
/// - a Conversation written before there was a Review role, whose column the
///   migration deliberately leaves empty rather than inventing a choice nobody
///   made — see the store's `migrations`; and
/// - a Draft steered into a state that settles only what builds, which leaves
///   the review role untouched because nothing reviews there — see
///   [`crate::steering`].
///
/// Both of them were reviewed by whatever built them before there was a Review
/// Pairing at all, so that is what reviews them now. Without it a wrap-up like
/// theirs waits for ever on a review nothing can start: the launch would fail,
/// the review would never settle, and what the human would see is a stall.
///
/// **A role picked away never falls back.** *No review* is a settled choice
/// rather than an empty picker, and what it settles is that no session runs —
/// so it is `None` here, and nothing reaches here for it anyway: such a wrap-up
/// settles its review without launching anything. See [`crate::review`].
///
/// Every other role is the Implementation Pairing, which is what the work
/// itself carrying on runs under — which is every launch here but the review.
pub(crate) fn under(
    role: store::Role,
    review: &store::Picked,
    implementation: Option<&store::Pairing>,
) -> Option<store::Pairing> {
    match role {
        store::Role::Review => match review {
            store::Picked::Skipped => None,
            picked => picked
                .pairing()
                .cloned()
                .or_else(|| implementation.cloned()),
        },
        _ => implementation.cloned(),
    }
}

/// What a prompt is written from: where this server installed the skills, and
/// the work's two documents.
///
/// Read by the driver because the prompt is the driver's — see
/// [`crate::lending`], which adds what every session is told under it. Read
/// back every time rather than held across the run: a backlog takes hours.
struct Documents<'a> {
    skills: &'a skills::Skills,
    brief: String,
    handoff: Option<String>,
}

impl Documents<'_> {
    fn handoff(&self) -> Option<&str> {
        self.handoff.as_deref()
    }
}

/// The documents `conversation_id`'s next prompt is written from, or `None`
/// where there is nothing to write one from — a server that installed no skills
/// runs no sessions to send anywhere.
async fn documents(state: &AppState, conversation_id: i64) -> Option<Documents<'_>> {
    let Some(skills) = state.sessions.skills() else {
        tracing::error!(
            conversation_id,
            "this server has no skills to send a session into, so none was started"
        );
        return None;
    };

    match crate::conversations::documents(&state.pool, conversation_id).await {
        Ok((brief, handoff)) => Some(Documents {
            skills,
            brief,
            handoff,
        }),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what the work is failed");
            None
        }
    }
}

/// The host a step's session is launched under: what the rescue says it is
/// doing.
fn lent_for(state: &AppState, conversation_id: i64, step: &Step) -> Lent {
    Lent::new(state, conversation_id).doing(&step.what())
}

/// What a step's session is expected to leave, which is its [`Landing`] in the
/// Worktree, or `None` where there is no Worktree left.
async fn expected(
    state: &AppState,
    conversation_id: i64,
    step: &Step,
) -> Option<crate::done::Evidence> {
    let landing = step.landing();
    let worktree = worktree(state, conversation_id).await?;

    Some(crate::done::Evidence::Landed { worktree, landing })
}

/// Launch a session on `step` under `skill`, expecting its landing.
async fn launch_step(
    state: &AppState,
    conversation_id: i64,
    step: &Step,
    skill: &str,
    prompt: impl FnOnce(&Documents<'_>) -> String,
) -> Option<(Lent, u64)> {
    let evidence = expected(state, conversation_id, step).await?;
    let documents = documents(state, conversation_id).await?;
    let mut lent = lent_for(state, conversation_id, step);

    let session = lent
        .start(
            skill,
            store::Role::Implementation,
            prompt(&documents),
            evidence,
            step.ends(),
        )
        .await?;

    Some((lent, session))
}

/// Where the Conversation's work is being done, or `None` where there is nowhere
/// left to work — a closed Conversation, or one that has gone.
async fn worktree(state: &AppState, conversation_id: i64) -> Option<PathBuf> {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => match conversation.worktree {
            Some(worktree) => Some(worktree),
            None => {
                tracing::info!(
                    conversation_id,
                    "the Conversation has no Worktree left, so the runner stops"
                );
                None
            }
        },
        Ok(None) => {
            tracing::error!(conversation_id, "there is no Conversation left to work in");
            None
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to work in failed");
            None
        }
    }
}

/// The commit the Conversation's branch came off, or `None` where there is none
/// to read.
///
/// Written at grill start, whether or not the human overrode one, so a
/// Conversation with a Worktree has one. What it answers here is which roadmaps
/// on the branch are the branch's own — see [`crate::stages::touched`].
async fn base(state: &AppState, conversation_id: i64) -> Option<String> {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => match conversation.base_commit {
            Some(base) => Some(base),
            None => {
                tracing::error!(
                    conversation_id,
                    "the Conversation has no base commit, so what its branch wrote cannot be read"
                );
                None
            }
        },
        Ok(None) => {
            tracing::error!(conversation_id, "there is no Conversation left to work in");
            None
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to work in failed");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};

    use super::*;

    /// A worktree with a backlog in it: `TODO.md` as `list` writes it and a task
    /// document per file, committed as the breaking-down session would have left
    /// it.
    fn worktree(list: &str, files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);

        let backlog = path.join(BACKLOG);
        std::fs::create_dir_all(&backlog).unwrap();
        std::fs::write(backlog.join(TODO), list).unwrap();

        for file in files {
            std::fs::write(backlog.join(file), "# a task\n").unwrap();
        }

        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: plan rate-limiting tasks"]);

        dir
    }

    /// The list of a three-task backlog, with the first `done` of them ticked
    /// off — which is what a run part way through looks like.
    fn list(done: usize) -> String {
        ["01: First", "02: Second", "03: Third"]
            .iter()
            .enumerate()
            .map(|(at, entry)| match at < done {
                true => format!("- [x] {entry}\n"),
                false => format!("- [ ] {entry}\n"),
            })
            .collect::<String>()
    }

    /// The three task documents that backlog names.
    const DOCUMENTS: [&str; 3] = ["01-first.md", "02-second.md", "03-third.md"];

    /// Tick entry `number` off in the worktree's list and commit it, which is
    /// what a session finishing a task does.
    fn finish(path: &Path, number: &str) {
        let list = path.join(BACKLOG).join(TODO);
        let ticked = std::fs::read_to_string(&list)
            .unwrap()
            .replace(&format!("- [ ] {number}:"), &format!("- [x] {number}:"));

        std::fs::write(&list, ticked).unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "feat: a task"]);
    }

    /// Whether `path` holds a list, the synchronous half of
    /// [`anything_to_work`].
    fn listed(path: &Path) -> bool {
        crate::tasks::entries(&path.join(BACKLOG)).is_some()
    }

    fn run(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git {args:?} failed");

        String::from_utf8(output.stdout).unwrap()
    }

    /// A Worktree with no list has nothing to work, both ways round: one that
    /// was never broken down, and one whose finish commit took `.tasks/` away.
    /// A list with every box ticked still has the finish to run.
    #[tokio::test]
    async fn only_a_worktree_with_no_list_has_nothing_to_work() {
        let dir = worktree(&list(3), &DOCUMENTS);

        assert!(anything_to_work(dir.path()).await);

        std::fs::remove_dir_all(dir.path().join(BACKLOG)).unwrap();

        assert!(!anything_to_work(dir.path()).await);
        assert!(!anything_to_work(Path::new("/nonexistent")).await);
    }

    /// The done-signal, and the half of it that matters most: the box is
    /// ticked, but the commit that ticked it has not landed, so the session is
    /// still mid-task.
    #[test]
    fn an_entry_ticked_but_not_committed_is_a_session_still_working() {
        let dir = worktree(&list(0), &DOCUMENTS);
        let path = dir.path();
        let landing = Landing::Ticked(1);
        let todo = path.join(BACKLOG).join(TODO);

        assert!(!landed(path, &landing), "the box is not ticked yet");

        std::fs::write(&todo, list(1)).unwrap();

        assert!(
            !landed(path, &landing),
            "ticked, and the tick is not committed",
        );

        run(path, &["add", "-A"]);

        assert!(!landed(path, &landing), "staged is not committed either");

        run(path, &["commit", "-m", "feat: count the requests"]);

        assert!(landed(path, &landing), "ticked and committed");
    }

    /// The other way round, which is the breakdown's signal: the plan is written
    /// and only counts once it is under version control.
    #[test]
    fn a_backlog_written_but_not_committed_is_a_breakdown_still_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);
        std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "first"]);

        let landing = Landing::Arrived(todo());

        assert!(!landed(path, &landing), "there is no backlog at all yet");

        std::fs::create_dir_all(path.join(BACKLOG)).unwrap();
        std::fs::write(path.join(BACKLOG).join(TODO), "# Rate limiting\n").unwrap();

        assert!(
            !landed(path, &landing),
            "written, and untracked is not committed",
        );

        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: plan rate-limiting tasks"]);

        assert!(landed(path, &landing), "written and committed");
    }

    /// A committed backlog naming a platform that is not one has not landed: the
    /// session that wrote it is refused naming the task, and fixing the line and
    /// committing lets it through. The same landing ends a breakdown and a
    /// stage's planning session, so this is both of theirs.
    #[test]
    fn a_backlog_naming_a_platform_that_is_not_one_has_not_landed() {
        let line = |platform: &str| {
            format!(
                "# Rate limiting\n\n\
                 - [ ] 01: Count — [details](01-count.md)\n\
                 - [ ] 02: Refuse — [details](02-refuse.md) — on windows\n\
                 - [ ] 03: Report — [details](03-report.md) — on {platform}\n",
            )
        };

        let dir = worktree(
            &line("freebsd"),
            &["01-count.md", "02-refuse.md", "03-report.md"],
        );
        let path = dir.path();

        let why = lacking(path, &Landing::Arrived(todo()))
            .expect("a platform that is not one is refused");

        assert!(why.contains("task 03"), "{why}");
        assert!(why.contains("freebsd"), "{why}");
        assert!(why.contains("linux, macos, windows"), "{why}");

        std::fs::write(path.join(BACKLOG).join(TODO), line("macos")).unwrap();

        assert!(
            lacking(path, &Landing::Arrived(todo())).is_some(),
            "fixed and not committed is not landed either",
        );

        run(
            path,
            &["commit", "-am", "chore: plan rate-limiting tasks on macos"],
        );

        assert_eq!(lacking(path, &Landing::Arrived(todo())), None);
    }

    /// A repository at the moment a roadmap stage's branch is made on top of the
    /// stage before it: that one planned a backlog, worked it, and finished with
    /// it, so `.tasks/` is all through the history and gone from the tree.
    ///
    /// Returns the worktree and the commit the stage's branch stands on, which
    /// is what a stage's writing is read against.
    fn stacked() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);
        std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "first"]);

        let backlog = path.join(BACKLOG);
        std::fs::create_dir_all(&backlog).unwrap();
        std::fs::write(backlog.join(TODO), "# Visibility\n").unwrap();
        std::fs::write(backlog.join("01-first.md"), "# a task\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: plan Visibility tasks"]);

        std::fs::remove_dir_all(&backlog).unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: finish Visibility"]);

        let base = run(path, &["rev-parse", "HEAD"]).trim().to_owned();

        (dir, base)
    }

    /// The other shape a branch is cut on: a base that still carries a `.tasks/`
    /// of its own, which the new branch inherits and Verkstead clears.
    ///
    /// A stage whose predecessor stopped part way leaves exactly this — the list
    /// it was working, committed, and nothing that finished with it.
    fn inherited() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);
        std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "first"]);

        let backlog = path.join(BACKLOG);
        std::fs::create_dir_all(&backlog).unwrap();
        std::fs::write(backlog.join(TODO), "# Visibility\n").unwrap();
        std::fs::write(backlog.join("01-first.md"), "# a task\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: plan Visibility tasks"]);

        let base = run(path, &["rev-parse", "HEAD"]).trim().to_owned();

        (dir, base)
    }

    /// The reading the whole recovery turns on: a stage that was made and then
    /// lost its planning session has written no backlog, however much of one is
    /// behind it in the history it stacks on.
    #[test]
    fn a_stage_that_never_planned_has_written_no_backlog() {
        let (dir, base) = stacked();

        assert!(
            !backlog_written(dir.path(), &base),
            "the backlog in the history is the stage before this one's",
        );
    }

    /// And the moment the planning lands, which is what stops it being planned
    /// twice: the commit is this branch's own, so it counts.
    #[test]
    fn a_backlog_this_stage_committed_is_written() {
        let (dir, base) = stacked();
        let path = dir.path();

        let backlog = path.join(BACKLOG);
        std::fs::create_dir_all(&backlog).unwrap();
        std::fs::write(backlog.join(TODO), "# Pipeline\n").unwrap();

        assert!(
            backlog_written(path, &base),
            "written and not committed is still written — a session is mid-plan",
        );

        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: plan Pipeline tasks"]);

        assert!(backlog_written(path, &base), "written and committed");
    }

    /// And the shape the clearing commit leaves, which must not be read as a
    /// plan: the branch inherited a list from its base and Verkstead took it
    /// away, so the only `.tasks/` commit on this branch is a removal. A stage
    /// read as planned here would have nothing to work and no way back.
    #[test]
    fn a_removal_is_not_a_backlog_this_branch_wrote() {
        let (dir, base) = inherited();
        let path = dir.path();

        std::fs::remove_dir_all(path.join(BACKLOG)).unwrap();
        run(path, &["add", "-A"]);
        run(
            path,
            &["commit", "-m", "chore: clear the inherited task list"],
        );

        assert!(
            !backlog_written(path, &base),
            "the list this branch deleted belonged to the branch before it, and \
             deleting one plans nothing",
        );
    }

    /// The same thing before the clearing is committed: the removal is staged and
    /// no commit on this branch has touched `.tasks/` at all. A pending deletion
    /// is not a session mid-plan.
    #[test]
    fn a_staged_deletion_is_not_a_backlog_being_written() {
        let (dir, base) = inherited();
        let path = dir.path();

        run(path, &["rm", "-r", "-q", "--", BACKLOG]);

        assert!(
            !backlog_written(path, &base),
            "what is pending here is the list going away, not one arriving",
        );
    }

    /// The case that must never be read as a stage to plan again: the backlog
    /// was written, worked to empty, and the finish took `.tasks/` away. The
    /// tree looks exactly like a stage that planned nothing, and the history is
    /// what tells them apart.
    #[test]
    fn a_backlog_this_stage_finished_with_is_still_written() {
        let (dir, base) = stacked();
        let path = dir.path();

        let backlog = path.join(BACKLOG);
        std::fs::create_dir_all(&backlog).unwrap();
        std::fs::write(backlog.join(TODO), "# Pipeline\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: plan Pipeline tasks"]);

        std::fs::remove_dir_all(&backlog).unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: finish Pipeline"]);

        assert!(!listed(path), "nothing left to work");

        assert!(
            backlog_written(path, &base),
            "this stage planned, and a finished backlog is not an unplanned one",
        );
    }

    /// And the other thing the same reading decides: which of two situations an
    /// empty backlog is, when Resume is pressed on a branch that is on no pull
    /// request. A branch that worked its backlog to empty has written one and
    /// finished with it, so the work is built and the push is the only thing
    /// left; one that never had a breakdown land on it has written none and has
    /// nothing built to carry anywhere. The Worktree looks the same either way —
    /// no `.tasks/` at all — and the history is the whole of the difference. See
    /// [`nothing_left`].
    #[test]
    fn an_emptied_backlog_and_one_that_never_landed_are_told_apart_by_the_branch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);

        std::fs::write(path.join("README.md"), "# A repository\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: the repository as it stood"]);

        let base = run(path, &["rev-parse", "HEAD"]).trim().to_owned();

        assert!(!listed(path), "there is nothing to work");
        assert!(
            !backlog_written(path, &base),
            "and nothing on this branch ever wrote a backlog, so there is nothing built \
             to send for a pull request",
        );

        let backlog = path.join(BACKLOG);
        std::fs::create_dir_all(&backlog).unwrap();
        std::fs::write(backlog.join(TODO), "# Rate limiting\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: plan rate-limiting tasks"]);

        std::fs::remove_dir_all(&backlog).unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: finish rate limiting"]);

        assert!(
            !listed(path),
            "the finish took the list away, so there is nothing to work here either",
        );
        assert!(
            backlog_written(path, &base),
            "but this branch wrote a backlog and finished with it, which is a run that \
             got as far as its push",
        );
    }

    /// A repository git will not answer about says *written*, because the one
    /// thing this decides is whether to plan a stage again — and planning one
    /// over a backlog that is already there is the failure worth being careful
    /// about.
    #[test]
    fn a_repository_that_will_not_answer_says_written() {
        assert!(backlog_written(Path::new("/nonexistent"), "HEAD"));
    }

    /// The same rule for the roadmap step, and the reason it cannot be a path
    /// arriving: `docs/roadmaps/` is a directory a repository often has already,
    /// so what says the step landed is a roadmap *this branch wrote*.
    #[test]
    fn a_roadmap_the_repository_already_had_is_not_this_branchs_work() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);

        let roadmaps = path.join(crate::stages::ROADMAPS);
        std::fs::create_dir_all(roadmaps.join("public-release")).unwrap();
        std::fs::write(
            roadmaps.join("public-release").join(crate::stages::INDEX),
            "# Public release\n\n- [x] 01: Done\n",
        )
        .unwrap();

        run(path, &["add", "-A"]);
        run(
            path,
            &["commit", "-m", "docs: the roadmap that was already here"],
        );

        let base = run(path, &["rev-parse", "HEAD"]).trim().to_owned();
        let landing = Landing::Roadmap(base);

        assert!(
            !landed(path, &landing),
            "a roadmap the branch came off is not one it wrote",
        );

        std::fs::create_dir_all(roadmaps.join("mvp")).unwrap();
        std::fs::write(
            roadmaps.join("mvp").join(crate::stages::INDEX),
            "# MVP roadmap\n\n- [ ] 01: Workbench\n",
        )
        .unwrap();

        assert!(
            !landed(path, &landing),
            "written, and untracked is not committed",
        );

        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "docs: stage the mvp roadmap"]);

        assert!(landed(path, &landing), "written and committed");
    }

    /// And the third thing asked of a landed roadmap: that what it declares is a
    /// roadmap something could run.
    ///
    /// Written and committed is not enough where the lines declare badly, and
    /// this is the moment to say so — the session that wrote the roadmap is at a
    /// terminal until its signal is taken, and nothing after it would ever be
    /// sent to fix a declaration. An undeclared roadmap is no fault at all, that
    /// being every roadmap written before any of this.
    #[test]
    fn a_roadmap_that_declares_badly_has_not_landed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);
        std::fs::write(path.join("README.md"), "# Somewhere\n").unwrap();
        run(path, &["add", "-A"]);
        run(
            path,
            &["commit", "-m", "chore: the commit the branch came off"],
        );

        let base = run(path, &["rev-parse", "HEAD"]).trim().to_owned();
        let landing = Landing::Roadmap(base);

        let index = path
            .join(crate::stages::ROADMAPS)
            .join("parallel-stages")
            .join(crate::stages::INDEX);
        std::fs::create_dir_all(index.parent().unwrap()).unwrap();

        let staged = |list: &str| {
            std::fs::write(&index, list).unwrap();
            run(path, &["add", "-A"]);
            run(path, &["commit", "-m", "docs: stage the roadmap"]);
        };

        staged(
            "# Parallel stages\n\n\
             - [ ] 01: Reading — [brief](01-reading.md) — no dependencies\n\
             - [ ] 02: Scheduling — [brief](02-scheduling.md) — after 04\n",
        );

        let why = lacking(path, &landing).expect("a roadmap naming a stage that is not there");
        assert!(why.contains("stage 02"), "{why}");
        assert!(why.contains("after 04"), "{why}");

        staged(
            "# Parallel stages\n\n\
             - [ ] 01: Reading — [brief](01-reading.md) — no dependencies\n\
             - [ ] 02: Scheduling — [brief](02-scheduling.md) — after 01 — on windows\n",
        );

        assert_eq!(
            lacking(path, &landing),
            None,
            "written, committed and declaring a graph something could run",
        );

        staged(
            "# Parallel stages\n\n\
             - [ ] 01: Reading — [brief](01-reading.md)\n\
             - [ ] 02: Scheduling — [brief](02-scheduling.md)\n",
        );

        assert_eq!(
            lacking(path, &landing),
            None,
            "and a roadmap declaring nothing at all is every roadmap written before this",
        );
    }

    /// A Done signal refused over a step that has not landed says which half is
    /// missing, in words the agent can act on: the box, or the commit.
    ///
    /// The list is named the way its own platform writes a path, because that is
    /// what the agent reading the refusal is looking at — so what is expected
    /// here is written the same way rather than with a separator of its own.
    #[test]
    fn what_a_task_is_missing_is_said_by_which_half_it_is() {
        let dir = worktree(&list(0), &DOCUMENTS);
        let path = dir.path();
        let named = todo().display().to_string();

        assert_eq!(
            lacking(path, &Landing::Ticked(1)),
            Some(format!(
                "task 1's box in `{named}` is not ticked and committed"
            )),
        );

        let list = path.join(BACKLOG).join(TODO);
        let ticked = std::fs::read_to_string(&list)
            .unwrap()
            .replace("- [ ] 01:", "- [x] 01:");
        std::fs::write(&list, ticked).unwrap();

        assert_eq!(
            lacking(path, &Landing::Ticked(1)),
            Some(format!(
                "task 1's box in `{named}` is ticked but not committed"
            )),
        );

        run(path, &["commit", "-am", "feat: a task"]);

        assert_eq!(lacking(path, &Landing::Ticked(1)), None);

        assert_eq!(
            lacking(path, &Landing::Gone(todo())),
            Some(format!("`{named}` has not been taken away")),
        );
    }

    /// The repository being polled is one a session is committing in, and the
    /// moment a poll is most likely to land on is the moment it commits — which
    /// is exactly when `index.lock` is held.
    ///
    /// So the poll reads through it rather than waiting for it or taking one of
    /// its own: a poll that took the lock would be one that made the session's
    /// own `git commit` fail, on a machine with nobody watching.
    #[test]
    fn the_poll_reads_through_a_lock_a_session_is_holding() {
        let dir = worktree(&list(0), &DOCUMENTS);
        let path = dir.path();

        finish(path, "01");

        // What a session mid-commit has left in the repository.
        let lock = path.join(".git/index.lock");
        std::fs::write(&lock, "").unwrap();

        assert!(
            landed(path, &Landing::Ticked(1)),
            "a locked repository is still a repository to read",
        );
        assert!(
            lock.exists(),
            "the session's lock is still the session's: nothing here took it or cleared it",
        );
    }

    /// What each step turns on. The plan arrives, the roadmap is written since
    /// the base, and the handoff is a document outside the Worktree.
    #[test]
    fn every_step_has_something_to_watch_for() {
        assert_eq!(
            Step::Planning.landing(),
            Landing::Arrived(Path::new(".tasks/TODO.md").to_owned()),
        );
        assert_eq!(
            Step::Staging("d41f8a3b".to_owned()).landing(),
            Landing::Roadmap("d41f8a3b".to_owned()),
        );
        assert_eq!(
            Step::Handoff(PathBuf::from("/srv/verkstead/handoffs/7/handoff.md")).landing(),
            Landing::Handoff(PathBuf::from("/srv/verkstead/handoffs/7/handoff.md")),
        );
    }

    /// The handoff is the one landing with no repository in it. Nothing puts the
    /// document under version control — it is written outside the checkout on
    /// purpose — so what says the step is over is the document being there with
    /// something in it, and a Worktree with everything committed says nothing
    /// about it either way.
    #[test]
    fn the_handoff_lands_outside_the_worktree_entirely() {
        let dir = worktree(&list(3), &DOCUMENTS);
        let elsewhere = tempfile::tempdir().unwrap();
        let document = elsewhere.path().join("handoff.md");
        let landing = Landing::Handoff(document.clone());

        assert!(!landed(dir.path(), &landing), "nothing is written yet");

        std::fs::write(&document, "  \n").unwrap();
        assert!(
            !landed(dir.path(), &landing),
            "and a document of nothing hands nothing over",
        );

        std::fs::write(&document, "# What we settled\n").unwrap();
        assert!(landed(dir.path(), &landing));

        assert_eq!(
            git(dir.path(), &["status", "--porcelain"]),
            Some(String::new()),
            "with the Worktree untouched throughout: the handoff is Verkstead's \
             document rather than the project's",
        );
    }

    /// One Pairing, named by its model so that a test can tell two apart.
    fn pairing(model: &str) -> store::Pairing {
        store::Pairing {
            profile: store::Profile {
                id: "1".to_owned(),
                name: Some(model.to_owned()),
                account: store::Account::claude(
                    PathBuf::from("/data/claude"),
                    PathBuf::from("/data/claude.json"),
                ),
                models: vec![model.to_owned()],
                memory: true,
                unbound: None,
                minted: None,
            },
            model: Some(model.to_owned()),
        }
    }

    /// Each role runs under the account picked for it, which is the ordinary
    /// Conversation: the review reads its own and everything else reads what
    /// builds.
    #[test]
    fn a_session_runs_under_the_account_picked_for_its_role() {
        let review = store::Picked::Under(pairing("reviewing"));
        let building = pairing("building");

        assert_eq!(
            under(store::Role::Review, &review, Some(&building)),
            Some(pairing("reviewing")),
        );
        assert_eq!(
            under(store::Role::Implementation, &review, Some(&building)),
            Some(pairing("building")),
            "the work itself carrying on is the account that built it",
        );
    }

    /// A review with nothing picked for it runs under what built the work,
    /// which is what reviewed it before there was a role of its own.
    ///
    /// The two ways of getting here are a Conversation from before the Review
    /// role existed and a Draft steered into a state that settles only what
    /// builds, and neither can be put right by the human: the pickers froze
    /// when the work started. Without this the launch fails, the review never
    /// settles, and the wrap-up waits for ever.
    #[test]
    fn a_review_nobody_picked_an_account_for_runs_under_what_built_the_work() {
        let building = pairing("building");

        assert_eq!(
            under(
                store::Role::Review,
                &store::Picked::Nothing,
                Some(&building)
            ),
            Some(pairing("building")),
        );
    }

    /// And a review picked away runs under nothing at all.
    ///
    /// *No review* is a settled choice rather than an empty picker, so it is
    /// the one thing here that must never reach for another account: falling
    /// back would run the review the human turned off.
    #[test]
    fn a_review_picked_away_falls_back_to_nothing() {
        assert_eq!(
            under(
                store::Role::Review,
                &store::Picked::Skipped,
                Some(&pairing("building")),
            ),
            None,
        );
    }

    /// And with nothing to build under there is nothing to launch, whichever
    /// role is asking.
    #[test]
    fn a_conversation_with_no_implementation_pairing_launches_nothing() {
        for role in [store::Role::Review, store::Role::Implementation] {
            assert_eq!(under(role, &store::Picked::Nothing, None), None, "{role:?}");
        }
    }
}
