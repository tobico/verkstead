//! The auto-advancing task runner: once there is a backlog, it works itself.
//!
//! A fresh session per task, launched the moment the one before it has landed,
//! with no gate between them and nobody asked. That is the whole of it — and
//! everything difficult about it is in the two questions it has to answer from
//! the repository alone, because the sessions are ordinary interactive ones and
//! the repository is the only thing they report through.
//!
//! **What is next** is [`next_step`]: the lowest-numbered entry of `TODO.md`
//! whose box is not ticked, or the finish step once every box is. Read off the
//! Worktree by the same rule the pinned Event is drawn by — see [`crate::tasks`]
//! — so the list the human is watching and the list the runner is working are
//! one list. An unticked entry naming a file nobody wrote is neither, and stops
//! the run rather than putting a session at nothing to work from.
//!
//! **When a step is over** is [`Landing`]: an entry ticked in `TODO.md`, or a
//! path gone from the Worktree or arrived in it — and, either way, committed as
//! it stands. A box ticked but not committed is a session still mid-task, and a
//! commit is the one report an agent cannot half make. The poll never takes
//! `index.lock` — everything here
//! goes through [`crate::repos::git`], which passes `--no-optional-locks`,
//! because what it is reading is a repository a session is committing in and a
//! watcher that tripped the session's own `git add` would break the step it is
//! waiting for.
//!
//! A session is then ended on **done plus quiet**, never on done alone. Work
//! does not always stop at the commit — a message, a summary, the tidying after
//! — so the session is ended only once it has printed nothing for the grace
//! period, and anything it prints in the meantime puts the whole grace back on
//! the clock. A session that keeps talking is never killed blind.
//!
//! **Every kind of session is ended by Verkstead**, and none of them by itself:
//! they are ordinary interactive agents, which idle when their work is done
//! rather than exiting, so waiting to see one exit is waiting for something that
//! never comes. What differs between the kinds is what *done* is read off. A
//! propose-then-fix session — the wrap-up's review, and each batch of comments —
//! reports through nothing but its own words, so its rule is quiet with nothing
//! pending: see [`proposing`].
//!
//! A step whose session ends without landing it stops the run where it is, and
//! what it stops at is a **stop**: the Conversation records that nothing is
//! driving it any more, and a Notice carrying the evidence goes on the Timeline
//! — see [`crate::stopping`]. The run does not go round again from there; getting
//! going is a press of Resume, because a runner that relaunched a step nothing
//! had moved would spend an account on the same failure with nobody watching.
//!
//! **And one whose session never ends at all** is spoken to before it is stopped
//! over. A session that goes idle with nothing open and nothing landed has not
//! ended and has not finished: it is sitting there with the turn over and no way
//! of knowing that the screen it printed to has nobody in front of it. So it is
//! told, twice, and then stopped where it stands — see [`crate::rescues`], which
//! is one loop over every driver here and takes what *done* is read off as its
//! parameter.
//!
//! **The pull request is the one thing that gets a second go.** Every run here
//! ends on one — a backlog's finish step, an inline implementation, a roadmap's
//! own session — and each of them commits its work and then pushes and opens the
//! pull request after the commit. So each of them can land everything it was
//! sent for and still stop short of the one act that makes the work readable,
//! leaving it built, committed and unreviewable. That is not a step to run
//! again: it is one push and one `gh pr create`, so it is asked for on its own,
//! by a session sent for nothing else, and what follows is GitHub asked again
//! and the ordinary stop where the answer has not changed. See
//! [`to_a_pull_request`].
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

use verkstead_schema::{Direction, Nudge};

use crate::AppState;
use crate::drivers::Driving;
use crate::follow_ups::FollowUp;
use crate::github;
use crate::repos::git;
use crate::sessions::{Idle, Session};
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

    /// How long a session must have printed nothing, its step landed, before it
    /// is ended.
    pub grace: Duration,

    /// How often a wrapping Conversation's pull request is asked about.
    ///
    /// Here rather than beside the backlog's two because it is the same kind of
    /// choice one phase along, and a caller standing a server up sets all of
    /// them at once — see [`crate::checks::ASKED_EVERY`] for what it costs.
    pub checks: Duration,

    /// And how long a session that proposes and then fixes must have printed
    /// nothing, with nothing of its own left for the human to answer, before it
    /// is ended — see [`proposing`].
    ///
    /// Distinctly longer than [`Pace::grace`], because it is carrying more
    /// weight. A backlog step is ended on quiet *and* a landing read off the
    /// repository, so quiet is the second of two signals; one of these reports
    /// through nothing but its own words, and quiet is the only signal there
    /// is. Cutting a review off early leaves what it had got to standing as the
    /// whole of it, which on the Timeline reads as a review that found nothing
    /// — and a minute of silence is the shortest an agent still at work
    /// reliably breaks.
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
    /// nothing else here catches — Rescue's precondition is quiet, every ender
    /// gates on the same judgement, and no session carries a cap on its life —
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

    /// And how often every Conversation is looked over for one that has
    /// Stalled — see [`crate::stalls`].
    ///
    /// Here beside the rest for [`Pace::checks`]s reason rather than because a
    /// sweep is anything the runner does: a caller standing a server up chooses
    /// how often Verkstead looks at things, and a stall is one of the things it
    /// looks for.
    pub stalls: Duration,
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
            reviewing: Duration::ZERO,
        }
    }
}

/// What the runner does next, read off the Worktree and nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    /// Break the work down, which is where a task-list Conversation starts:
    /// there is no backlog yet, and the session writing one is already running.
    ///
    /// Never what [`next_step`] answers — a backlog that is not there says
    /// nothing about whether it is about to be written or has just been worked
    /// through. It is the step the runner is handed rather than one it decides.
    Planning,

    /// The same first step for a roadmap stage, which starts where a task-list
    /// Conversation does and gets there another way: nobody chose a direction,
    /// the stage before it settling is what started it, and the fork that writes
    /// its backlog re-grounds a brief rather than breaking down a handoff.
    ///
    /// Watched for exactly what a breakdown is watched for, the backlog being
    /// the same backlog. What differs is which fork wrote it.
    PlanningStage,

    /// Write the handoff, which is the whole of what an inline grilling has left
    /// to do once the human has picked: the work itself runs under the other
    /// Profile in a session of its own, so everything this one settled has to be
    /// written down before it ends.
    ///
    /// Carries the path the document is watched for, which is outside the
    /// Worktree — Verkstead's own directory rather than the project's, so that
    /// no `git add -A` after it can sweep the document into the human's
    /// repository. See [`crate::handoffs`].
    ///
    /// Never what [`next_step`] answers, for the reason [`Step::Planning`] never
    /// is: it is the step the runner is handed rather than one it decides.
    Handoff(PathBuf),

    /// Work this task: the lowest-numbered entry whose box is not ticked, and
    /// the file in `.tasks/` that says what it is.
    ///
    /// Both, because the two answer different halves of the step. The number is
    /// what says it is over — the entry's own box, ticked — and the file is what
    /// the Notice names when it is not.
    Task { number: u32, file: PathBuf },

    /// An entry that is not ticked and names no file: a hand-edited backlog, or
    /// a breakdown that stopped part way through writing one.
    ///
    /// Nothing to run. A session launched at it would have no task document to
    /// read and nothing to tell it where to stop, so the run stops instead and a
    /// Notice names the entry — see [`nothing_to_work_from`], which is the one
    /// answer, and both places that ask what is next reach for it: [`carry_on`]
    /// working a backlog, and [`backlog_again`] resuming one.
    Broken { label: String },

    /// Finish the feature: every entry is ticked, and what is left is taking
    /// `.tasks/` away.
    Finish,

    /// Write the roadmap, which is where a roadmap Conversation starts and
    /// finishes: the stages it plans are Conversations of their own.
    ///
    /// Carries the commit the branch came off, because what says this step is
    /// over is a roadmap on the branch *that was not there before* — see
    /// [`Landing::Roadmap`]. Never what [`next_step`] answers, for the reason
    /// [`Step::Planning`] never is: it is the step the runner is handed.
    Staging(String),

    /// There is no backlog. Nothing to run, and nothing to poll for.
    Nothing,
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
    /// What would say this step is over, or `None` where it is not a step to
    /// run.
    fn landing(&self) -> Option<Landing> {
        match self {
            // The plan commit is what puts the backlog under version control,
            // so the backlog being there and committed is the breakdown done.
            Step::Planning | Step::PlanningStage => Some(Landing::Arrived(todo())),
            // And the handoff is the document itself, in a directory no commit
            // reaches: nothing puts it under version control, so its being there
            // is the whole of the signal.
            Step::Handoff(path) => Some(Landing::Handoff(path.clone())),
            // Finishing a task is what ticks its entry off in the list.
            Step::Task { number, .. } => Some(Landing::Ticked(*number)),
            // And the finish commit removes `TODO.md` with the rest of `.tasks/`.
            Step::Finish => Some(Landing::Gone(todo())),
            // The roadmap commit is what puts the stages under version control.
            Step::Staging(base) => Some(Landing::Roadmap(base.clone())),
            // Neither of these is a step to run, so neither has anything to
            // watch for.
            Step::Broken { .. } | Step::Nothing => None,
        }
    }

    /// What the step was, in the words the Timeline draws.
    ///
    /// The sentence rather than the word above it: the two are read by different
    /// readers, and a human looking at a stopped run wants to know *which* task
    /// rather than that it was a task.
    fn what(&self) -> String {
        match self {
            Step::Planning => "breaking the work down into a backlog".to_owned(),
            Step::PlanningStage => "planning the roadmap stage into a backlog".to_owned(),
            Step::Handoff(_) => "writing the handoff for the session that builds".to_owned(),
            Step::Task { file, .. } => format!("the task in {}", file.display()),
            Step::Broken { .. } => "working the backlog".to_owned(),
            Step::Finish => "finishing the feature".to_owned(),
            Step::Staging(_) => "staging the work into a roadmap".to_owned(),
            Step::Nothing => "nothing".to_owned(),
        }
    }
}

/// The backlog's list, as a path inside a Worktree.
fn todo() -> PathBuf {
    Path::new(BACKLOG).join(TODO)
}

/// Why a run stopped at an entry with no task file, in the words the Notice
/// carries.
///
/// The entry rather than the file, because the file is the thing that is not
/// there: what the human has to go and look at is the line in `TODO.md`, and
/// either writing the document it names or ticking it off gets the run going
/// again.
fn broken(label: &str) -> String {
    format!(
        "entry {label} of `{}/{TODO}` is not ticked off and names no task file, so there is \
         nothing for a session to work from",
        BACKLOG,
    )
}

/// Stop the run at a [`Step::Broken`], with the Notice naming the entry there is
/// nothing to work from.
///
/// Both places that decide what to run next reach for this, because both can be
/// handed one: [`carry_on`] meets it as a backlog is worked, and
/// [`backlog_again`] meets it when a Resume asks the same question of the same
/// `.tasks/`. A press that launched a session where the loop would have refused
/// to would be Resume undoing the rule rather than the human's leave to try
/// again.
///
/// [`crate::stopping::Decided::Verkstead`]: nothing can be read out of the
/// backlog, so a restart looking again would find the same unreadable list. What
/// changes it is the human's, in the repository — writing the document the entry
/// names, or ticking the entry off — and Resume is what picks it up afterwards.
async fn nothing_to_work_from(state: &AppState, conversation_id: i64, step: &Step, label: &str) {
    tracing::warn!(
        conversation_id,
        label,
        "a backlog entry is not done and names no task file, so the run stops here",
    );

    stop(
        state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        &step.what(),
        &broken(label),
        None,
    )
    .await;
}

/// Follow the grilling session as it writes what the pick asked for.
///
/// Nothing is launched: the session is the one that proposed, idling on the
/// blocking ask the Response is being delivered through, and it goes on from
/// there with the whole thread still in its context. What this is, is the watcher
/// — the artifact landing, plus quiet, is what ends the session and moves the
/// Conversation on.
///
/// The three tails differ in what the artifact is and in where landing it leaves
/// the Conversation. A backlog is the start of a run — the tasks it names are
/// worked one fresh session each, under the Profile that builds — so the
/// Conversation goes on to Implementing. The handoff an inline pick asks for
/// leaves it in the same place, one session rather than a run standing on the
/// other side of it. A roadmap is the whole of this Conversation's own work,
/// because the building belongs to the Stages it plans, so the same session
/// carries the branch to a pull request and the Conversation goes straight on to
/// wrapping that up.
///
/// `driving` is the registration that says this Conversation is being driven,
/// taken by whoever armed the watcher rather than here, and held from the pick
/// through to wherever the tail leaves the Conversation — see [`crate::drivers`]
/// for why it is handed over rather than taken again. A watcher is the one thing
/// that follows a grilling session, so it is also what says a grilling picked on
/// is not standing still.
pub(crate) async fn follow_the_tail(
    state: AppState,
    conversation_id: i64,
    direction: Direction,
    session: Session,
    driving: Driving,
) {
    match direction {
        Direction::Inline => follow_handoff(state, conversation_id, session, driving).await,
        Direction::TaskList => follow_breakdown(state, conversation_id, session, driving).await,
        Direction::Roadmap => follow_staging(state, conversation_id, session, driving).await,
    }
}

/// Work `conversation_id`'s backlog to empty, starting from the session that is
/// writing it.
///
/// `writing` is the session that will commit the backlog: the grilling session
/// itself, which got the human's pick back through its blocking ask and breaks
/// the work down without leaving the context that settled it. It is the run's
/// first step rather than something that happened before the run — an ordinary
/// interactive session, which will idle once its plan is committed, so something
/// has to see it out, and what sees a session out is exactly this.
///
/// The plan commit is the end of the planning as well as the start of the run, so
/// the Conversation moves as the session is seen out: grilling until then and
/// implementing afterwards — see [`crate::conversations::grilling_over`]. No
/// handoff is taken or written on this path: the backlog is what the grilling
/// settled, committed to the branch, and every session that works it reads the
/// repository rather than a summary of a conversation it never had.
///
/// Returns when there is nothing left to run: the backlog worked through, a
/// Conversation that has gone, or a step whose session ended without landing it.
/// Idle rather than looping — a runner that relaunched a session for a step
/// nothing had moved would be a machine spending an account on the same failure
/// over and over, with nobody watching. What it leaves behind for the human is a
/// stop, and the Notice saying what stopped.
///
/// `driving` is the registration that says this Conversation is being driven,
/// taken by whoever armed the watcher or pressed Resume rather than here — see
/// [`crate::drivers`] for why it is handed over rather than taken again.
async fn follow_breakdown(
    state: AppState,
    conversation_id: i64,
    writing: Session,
    driving: Driving,
) {
    if see_out(&state, conversation_id, Step::Planning, writing)
        .await
        .is_none()
    {
        return;
    }

    // The backlog is on the branch, so the record says where that happened —
    // before the move, because it is what the move is being made on the
    // strength of.
    crate::conversations::backlog_landed(&state, conversation_id).await;

    crate::conversations::grilling_over(&state, conversation_id).await;

    carry_on(state, conversation_id, driving).await
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
        // The backlog's own answer to what is next, asked of `.tasks/` exactly
        // as every other turn of the run asks it — a stage's backlog included,
        // it being a backlog like any other by the time there is one. What is
        // next has not changed on account of nothing having been running.
        Direction::TaskList => {
            backlog_again(
                state,
                conversation_id,
                &working_in,
                conversation.base_commit.as_deref(),
                driving,
            )
            .await
        }
    }
}

/// Work a backlog again — or, where it is worked out and the branch is already
/// on a pull request, wrap that up instead.
///
/// The backlog's own answer to what is next, asked of `.tasks/` exactly as every
/// other turn of the run asks it — a stage's backlog included, it being a
/// backlog like any other by the time there is one. What is next has not changed
/// on account of nothing having been running.
///
/// An empty one is three situations rather than one. A stage whose planning
/// never landed is the one that is answered here, because it is the only one
/// with a step left to run: its backlog was never written, and the session that
/// would have written it is launched once and by nobody else — see
/// [`stage_to_plan`]. The other two are a breakdown that never landed and a
/// feature that is finished with, and nothing can tell those apart until
/// [`nothing_left`] asks GitHub.
///
/// `base` is the commit this Conversation's branch was made on, which is what
/// the first of the three is read against: what a branch has written is asked of
/// what it has written *since it branched*, the predecessor's own backlog being
/// in the history of a stage stacked on one.
async fn backlog_again(
    state: AppState,
    conversation_id: i64,
    working_in: &Path,
    base: Option<&str>,
    driving: Driving,
) {
    let step = decide(working_in).await;

    if step == Step::Nothing {
        // Asked before GitHub is, and before anything is read as an ending that
        // half happened: a stage that has planned nothing has pushed nothing and
        // opened nothing, so what an empty backlog means there is that the run
        // has not started rather than that it is over.
        if let Some(stacked_on) = stage_to_plan(&state, conversation_id, working_in, base).await {
            tracing::info!(
                conversation_id,
                "a stage's backlog was never planned, so the planning is being run again",
            );

            return plan_stage(state, conversation_id, stacked_on, driving).await;
        }

        return nothing_left(state, conversation_id, working_in, base, driving).await;
    }

    // And an entry that is not ticked and names no file is refused here exactly
    // as the loop refuses it — see [`nothing_to_work_from`]. A Resume is the
    // human's leave to try the run again rather than their leave to work a
    // backlog nothing can be read out of, and the press arriving before they
    // have fixed `TODO.md` is the ordinary way this is met: the Notice that
    // stopped the run is what they have just read.
    //
    // Held until the stop is written, for [`nothing_left`]'s reason: dropping
    // the registration first would leave a moment where a sweep could find the
    // Conversation undriven and stop it with a worse sentence.
    if let Step::Broken { label } = &step {
        let _driving = driving;

        nothing_to_work_from(&state, conversation_id, &step, label).await;
        return;
    }

    tracing::info!(conversation_id, step = ?step, "a stopped run is being taken up again");

    let Some(session) = launch_in_turn(&state, conversation_id, Prompt::NextTask).await else {
        return;
    };

    work(state, conversation_id, step, session, driving).await
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
                "there is nothing in `.tasks/` to work, nothing on this branch ever wrote \
                 a backlog, and there is no pull request to wrap up — the breakdown never \
                 landed, so there is nothing built here to carry anywhere",
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
        Err(github::Trouble::NoPullRequest) => {
            let Some(session) = launch_in_turn(&state, conversation_id, Prompt::Implementing).await
            else {
                return;
            };

            follow_inline(state, conversation_id, session, driving).await
        }
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
        crate::conversations::roadmap_landed(&state, conversation_id).await;

        return to_a_pull_request(&state, conversation_id, None).await;
    }

    let Some(session) = launch_in_turn(&state, conversation_id, Prompt::Staging).await else {
        return;
    };

    follow_roadmap(state, conversation_id, base, session, driving).await
}

/// Plan a roadmap stage and then work the backlog it writes, from the first task
/// to the pull request.
///
/// Where [`follow`] is handed the session that is already running, this launches
/// one: a stage is started by the stage before it settling rather than by
/// anything a human pressed, so there is nobody to have launched it — see
/// [`crate::continuing`]. Everything after that is the same run a task list has,
/// because from the plan commit onwards it *is* one.
///
/// Started as the stage is made — by the stage before it settling, or by a human
/// adopting the roadmap it belongs to — and started again by a run taken up over
/// a stage that has no backlog. That second way in is what makes the first
/// recoverable: a planning session that died before it committed leaves a stage
/// whose backlog nothing else would ever write, so what is read there is a run
/// that never began rather than one that is worked out — see [`stage_to_plan`].
///
/// `stacked_on` is the branch this stage's branch was made on top of, which the
/// fork is told because it is the one thing about a stage the repository does not
/// say.
///
/// `driving` is the registration that says this Conversation is being driven,
/// taken by whoever is starting the planning rather than here — the stage being
/// made, or a run that has found the planning never happened. Handed over rather
/// than taken at this end, for the reason every other driver here hands one
/// over: the launch is the slow part, and a gap in the middle of one is a
/// Conversation a sweep reads as standing still. See [`crate::drivers`].
pub(crate) async fn plan_stage(
    state: AppState,
    conversation_id: i64,
    stacked_on: Option<String>,
    driving: Driving,
) {
    let Some(session) =
        launch_in_turn(&state, conversation_id, Prompt::PlanningStage(stacked_on)).await
    else {
        return;
    };

    work(
        state,
        conversation_id,
        Step::PlanningStage,
        session,
        driving,
    )
    .await
}

/// Work a backlog from `first` to empty.
///
/// `first` is the step the session it is handed is running, decided before that
/// session was started: which step a session is for has to be settled before the
/// session is, or a driver would be racing the agent it had just let loose.
///
/// The registration it is handed is held for the whole run and let go as it
/// returns, which is what makes the quiet gaps between one step's session and
/// the next read as a Conversation being driven rather than as one standing
/// still.
async fn work(
    state: AppState,
    conversation_id: i64,
    first: Step,
    session: Session,
    driving: Driving,
) {
    let Some(writing) = see_out(&state, conversation_id, first.clone(), session).await else {
        return;
    };

    // A stage's first step is the one that writes its backlog, and landing that
    // is the same moment [`follow_breakdown`] records one step earlier: the
    // branch now carries a list to work through. Asked here for the reason the
    // finish is asked here — this is the one place that knows *which* step just
    // landed.
    if first == Step::PlanningStage {
        crate::conversations::backlog_landed(&state, conversation_id).await;
    }

    // The finish step is the last one a backlog has, and landing it is not the
    // end of the run: what the finish did was push and open a pull request, and
    // the Conversation moves on to wrapping that up. Asked here rather than
    // afterwards, because this is the one place that knows *which* step just
    // landed.
    if first == Step::Finish {
        to_a_pull_request(&state, conversation_id, Some(writing)).await;
        return;
    }

    carry_on(state, conversation_id, driving).await
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

        carry_on(state, conversation_id, driving).await
    });
}

/// Build the work of a Conversation whose human picked *no grilling*, from the
/// press that gave it a branch to the pull request it ends on.
///
/// [`build_the_split_out`]'s shape and the other entry into a run that is not a
/// direction being followed — except that here the direction *is* recorded, the
/// start having written it: a Brief taken straight to the work is an inline
/// implementation, so what this launches and what it does with the session is
/// [`follow_handoff`]'s second half exactly. What it skips is the first half,
/// there being no grilling session to see out and no handoff for it to have
/// written.
///
/// The registration is taken here rather than by the caller, because the caller
/// is the press and is about to answer the human: a gap between the two would be
/// a Conversation the stall sweep found with nothing driving it.
pub(crate) fn build_the_ungrilled(state: &AppState, conversation_id: i64) {
    let driving = state.drivers.driving(conversation_id);
    let state = state.clone();

    tokio::spawn(async move {
        let Some(session) = launch_in_turn(&state, conversation_id, Prompt::Implementing).await
        else {
            return;
        };

        follow_inline(state, conversation_id, session, driving).await
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

/// Work whatever the backlog has left, one fresh session per step, until it is
/// empty.
///
/// Where [`work`] is handed a session that is already running its step, this
/// starts from the repository: what is next is read off `.tasks/` and a session
/// is launched for it. Split out because the two entries into a run differ only
/// in that first session — the breakdown's own, or the grilling session that
/// wrote the backlog in its place.
///
/// The registration is held for the whole loop and let go as it returns, which
/// is what makes the quiet gaps between one step's session and the next read as
/// a Conversation being driven rather than as one standing still.
async fn carry_on(state: AppState, conversation_id: i64, _driving: Driving) {
    loop {
        let Some(worktree) = worktree(&state, conversation_id).await else {
            return;
        };

        let step = decide(&worktree).await;

        if step == Step::Nothing {
            tracing::info!(
                conversation_id,
                "the backlog is worked through, so the runner has nothing left to launch"
            );
            return;
        }

        // An entry that is not ticked and names no file: there is nothing for a
        // session to read and nothing to tell it where to stop, so the run stops
        // here and the Notice names the entry — see [`nothing_to_work_from`],
        // which a Resume asking the same question reaches for too.
        if let Step::Broken { label } = &step {
            nothing_to_work_from(&state, conversation_id, &step, label).await;
            return;
        }

        tracing::info!(conversation_id, step = ?step, "a fresh session is starting on the next step");

        let Some(started) = launch_in_turn(&state, conversation_id, Prompt::NextTask).await else {
            return;
        };

        let Some(writing) = see_out(&state, conversation_id, step.clone(), started).await else {
            return;
        };

        if step == Step::Finish {
            to_a_pull_request(&state, conversation_id, Some(writing)).await;
            return;
        }
    }
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
        return crate::wrapping::record(state, conversation_id, repo_id, &branch, found, writing)
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
/// [`to_a_pull_request`] for the first and [`nothing_left`] for the second.
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
/// **Ended on quiet with nothing of its own open**, which is the review's rule
/// rather than a step's, and here for a reason of its own: what this session is
/// sent to do happens on GitHub rather than in the repository, so there is no
/// path to watch and no commit it has to make — a branch that only wanted pushing
/// is one it finishes without writing a line. What is left is silence, and every
/// session here is an interactive agent that idles when its work is done rather
/// than exiting. Anything it prints puts the whole grace back on the clock, and a
/// Set of its own left open holds it for as long as the human takes.
///
/// **No rescue on this one**, alone among the sessions here. The rescue is for a
/// session nothing else can move on from — but this is already the second go at
/// the same missing thing, and what follows it either way is GitHub asked and the
/// Conversation wrapped up or stopped. Prodding it a third time would put the
/// Notice off rather than save the human from it.
///
/// The Timeline Event it printed into, or `None` where nothing ran: no session
/// could be started, or the run was stopped from outside while this one did. Both
/// of those have already said whatever there was to say.
async fn submitted(state: &AppState, conversation_id: i64) -> Option<i64> {
    let mut session = launch_in_turn(state, conversation_id, Prompt::Submitting).await?;

    let event_id = session.event_id;
    let idle = session.idle.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        () = quiet_and_nothing_asked(state, conversation_id, event_id, &idle, pace) => None,
    };

    match ended {
        // Quiet with nothing of its own open, which is as much as a session whose
        // work happened on GitHub can report from in here. Ended rather than left
        // holding the Worktree, and what it did is GitHub's to say.
        None => {
            tracing::info!(
                conversation_id,
                event_id,
                "the session sent to open the pull request has gone quiet, so it is \
                 being ended",
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

/// Follow the grilling session as it writes the handoff, and start the session
/// that builds from it.
///
/// The inline counterpart to [`follow_breakdown`], and the same move with a
/// different artifact: the session that settled the work is the one that writes
/// down what it settled, after the pick rather than before it, so what the human
/// said beside the pick is part of what the handoff has to say. Nothing is
/// launched here either — the session is already running, idling on the ask the
/// Response came back through.
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
/// A session that goes quiet without writing one stops, the way every other step
/// does: nothing is driving the Conversation and a Notice says so, and starting
/// it again is Resume's.
///
/// The registration is held across all of it — the watch, the move, and the run
/// on the other side of it — so there is no moment between the handoff landing
/// and the implementation session starting where the Conversation reads as one
/// nothing is driving.
async fn follow_handoff(state: AppState, conversation_id: i64, writing: Session, driving: Driving) {
    let handoffs = crate::handoffs::Handoffs::under(&state.data_dir);
    let step = Step::Handoff(handoffs.document(conversation_id));

    if see_out(&state, conversation_id, step, writing)
        .await
        .is_none()
    {
        return;
    }

    crate::conversations::hand_over(&state, conversation_id).await;
    crate::conversations::grilling_over(&state, conversation_id).await;

    let Some(session) = launch_in_turn(&state, conversation_id, Prompt::Implementing).await else {
        return;
    };

    follow_inline(state, conversation_id, session, driving).await
}

/// See an inline implementation session out, and carry the Conversation on to
/// wrapping the pull request it opened — or stop the run if it ends having
/// landed nothing.
///
/// The whole of the work in one session, so there is no next step to launch and
/// nothing to poll a Worktree for: what says an inline session did anything is
/// what it committed, which the branch watcher is putting on the Timeline while
/// it runs.
///
/// **Ended on committed plus quiet**, which is [`instructed`]'s rule on a
/// session of the same shape: there is no path to watch, and a commit is the one
/// report an agent cannot half make. Work does not always stop at the commit —
/// the push and the pull request come after one — so the session is ended only
/// once it has printed nothing for the grace, and anything it prints puts the
/// whole grace back on the clock. Waiting for the process to exit instead would
/// be waiting for something that never comes: every session here is an
/// interactive agent that idles when its work is done.
///
/// **And one that will not ask is spoken to**, on the same commit. Idle with
/// nothing committed and nothing put to the human is the whole of an inline run
/// come to nothing with a process still holding the Worktree — a Conversation
/// nobody can move, driven so nothing sweeps it and silent so nothing says so.
/// Told twice and then stopped where it stands, as every other driver here does
/// it. See [`crate::rescues`].
///
/// Landing is measured against what was already there rather than against zero,
/// which is what makes a second go answerable: a first attempt that committed
/// twice and then died leaves two commits behind, and a second that commits
/// nothing has still landed nothing.
///
/// What follows a session that landed something is the same ending a backlog's
/// finish step has: the session followed the repository's own review process on
/// its way out, so the branch is pushed and on a pull request by the time it
/// goes quiet, and [`to_a_pull_request`] is what finds that pull request and
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
async fn follow_inline(
    state: AppState,
    conversation_id: i64,
    mut session: Session,
    _driving: Driving,
) {
    let event_id = session.event_id;

    // Taken before the waiting starts, so it is a count of what the run had
    // landed before this session rather than including what it goes on to do.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let idle = session.idle.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        // What an inline session that has done its work looks like from here:
        // more on the branch than it started with, and nothing printed for the
        // grace. Waited for rather than left to the exit, because an
        // interactive agent idles when its work is done rather than exiting —
        // which is [`instructed`]'s rule, on a session whose whole job is the
        // same shape.
        () = committed_and_quiet(&state, conversation_id, already, &idle, pace) => None,
        // And one that is idle with nothing committed and nothing put to the
        // human: the whole of an inline run come to nothing, with a process
        // still holding the Worktree and nothing on the page to press. Told
        // twice and then stopped where it stands, the commit being its
        // done-indicator — see [`crate::rescues`].
        () = crate::rescues::until_it_will_not_ask(
            &state,
            conversation_id,
            event_id,
            &idle,
            pace,
            crate::rescues::Done::Committed { already },
        ) => {
            tracing::warn!(
                conversation_id,
                event_id,
                "the inline session went quiet without committing anything or asking \
                 about it, so the Conversation stops here",
            );

            state.sessions.end(conversation_id).await;

            return stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "implementing the work inline",
                crate::rescues::WOULD_NOT_ASK,
                Some(event_id),
            )
            .await;
        }
    };

    // Committed and gone quiet, which is an inline implementation done. The
    // session is ended rather than waited out, and what follows is the ending a
    // landed run has always had: the skill carried the branch to a pull request
    // on its way out, and [`to_a_pull_request`] is what finds it — or sends for
    // it, and then stops naming what it still could not find.
    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            "an inline session has committed and gone quiet, so it is being ended",
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
                "the session ended without committing anything".to_owned()
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
/// **Ended on committed plus quiet**, which is a fix session's rule rather than
/// a backlog step's, and for its reason: an instruction can ask for anything, so
/// there is no path to watch and no done file to read — what there is, is a
/// commit, which is the one report an agent cannot half make. Work does not
/// always stop at the commit, so the session is ended only once it has printed
/// nothing for the grace period, and anything it prints puts the whole grace
/// back on the clock.
///
/// **And a session that commits nothing stops the Conversation**, exactly as an
/// inline implementation that commits nothing does. An interactive agent that
/// decides there is nothing to do exits zero, so a clean exit is not by itself a
/// report that anything happened — and the pipeline reads the branch to decide
/// what is next, so a branch nothing was written to is one there is nothing
/// honest to carry on from.
///
/// **Including one that never ends at all.** Idle, with nothing committed and
/// nothing put to the human, is the same instruction come to nothing with a
/// process still holding the Worktree — so it is told twice and then stopped in
/// the same words, the commit being its done-indicator. See [`crate::rescues`].
pub(crate) async fn instructed(
    state: AppState,
    conversation_id: i64,
    instruction: String,
    driving: Driving,
) {
    // Taken before the session starts, so it is a count of what the branch
    // carried before the instruction rather than one that includes what it goes
    // on to do.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let Some(mut session) =
        launch_in_turn(&state, conversation_id, Prompt::Instruction(instruction)).await
    else {
        return;
    };

    let event_id = session.event_id;
    let idle = session.idle.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        () = committed_and_quiet(&state, conversation_id, already, &idle, pace) => None,
        // Nothing committed and nothing asked, with the session sitting there:
        // an instruction that has come to nothing and nobody to say so to.
        // Told twice and then stopped where it stands, which is the same ending
        // a step that would not land gets — see [`crate::rescues`].
        () = crate::rescues::until_it_will_not_ask(
            &state,
            conversation_id,
            event_id,
            &idle,
            pace,
            crate::rescues::Done::Committed { already },
        ) => {
            let _driving = driving;

            tracing::warn!(
                conversation_id,
                event_id,
                "the instruction session went quiet without committing anything or asking \
                 about it, so the Conversation stops here",
            );

            state.sessions.end(conversation_id).await;

            return stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "doing what the instruction said",
                crate::rescues::WOULD_NOT_ASK,
                Some(event_id),
            )
            .await;
        }
    };

    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            "an instruction session has committed and gone quiet, so it is being ended",
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
        .unwrap_or_else(|| "the session ended without committing anything".to_owned());

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

        return carry_on(state, conversation_id, driving).await;
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
                "the branch holds no backlog to work and no pull request to wrap up",
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

/// See out a follow-up's session, and land the Conversation back in its wrap-up
/// once the human says there is nothing else.
///
/// `follow_up` is what the session is started on: the brief a steer into
/// Follow-up carried, and — where this is a follow-up being picked up again
/// rather than opened — the rounds it has already been through. See
/// [`crate::follow_ups`], which is where a press of Resume reads both back from.
///
/// **A driver rather than an errand beside the work**, exactly as an instruction
/// session is: the registration it is handed says the Conversation is being
/// driven for as long as this runs, so nothing sweeps it as standing still while
/// the human is composing an answer on a phone.
///
/// **Nothing is watched for on the branch.** A follow-up is rounds of asking
/// rather than a step with a landing: what it commits is the human's to have
/// asked for, and a round that was a question and an answer commits nothing at
/// all. So there is no committed-and-quiet to end it on and no artifact to read.
///
/// **What ends it is the human's own mark**, on the newest round they answered,
/// with the session idle and nothing left open — see [`nothing_else_and_quiet`],
/// which is the three of those waited on together. Then the session is ended and
/// the Conversation goes back to Wrapping over the pull request it was opened
/// about, with the wrap-up's watchers started over whatever the branch now
/// holds. *Back to Done* is the wrap-up's own settling rule and nothing this
/// decides — see [`over`].
///
/// **And a session that is gone is a stop**, which is the responding rule: no
/// other session is ever sent to finish somebody else's, so what it had got to,
/// what it was about to do and what it made of the last answer are all beyond
/// asking. The Notice says what happened and any question it left the human
/// holding goes off with it.
///
/// **Unless it is gone because it had finished**, which the mark is what says.
/// A session whose last act was the round the human marked *Nothing else* is a
/// follow-up that is over rather than one nobody is left to have, and an agent
/// that finishes its turn and exits is the ordinary shape of that — so the mark
/// and the open Set are read again where the session ends first, and a
/// follow-up that reads as finished lands in the wrap-up instead of stopping.
/// The same reading [`instructed`] gives its commits at the same point.
///
/// **So is one that will not ask.** A session that goes idle without a Set open
/// leaves the human holding a Conversation they can neither answer nor end, so
/// it is spoken to — twice, and then stopped where it stands. Nothing it left
/// open goes off with that one: there being nothing open is half of what said it
/// was stuck.
pub(crate) async fn following_up(
    state: AppState,
    conversation_id: i64,
    follow_up: FollowUp,
    driving: Driving,
) {
    // Taken before the session starts, so what it lands is counted as this
    // follow-up's own: whether the wrap-up's checks go back to waiting turns on
    // whether *this* follow-up pushed anything, and a Conversation on a pull
    // request has a run of commits behind it already.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    // What the session before this one left standing, where there was one. A
    // Blocking Ask outlives the session that asked it, and nobody is ever handed
    // somebody else's — so a question left over from the follow-up that died is
    // one the human could answer for ever with nothing reading it, and one this
    // follow-up would never end while it stood. Locked unanswered as the fresh
    // session starts, which is what a relaunched grilling does with its own.
    if follow_up.again {
        left_open(&state, conversation_id).await;
    }

    let Some(mut session) =
        launch_in_turn(&state, conversation_id, Prompt::FollowingUp(follow_up)).await
    else {
        return;
    };

    let event_id = session.event_id;
    let idle = session.idle.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        () = nothing_else_and_quiet(&state, conversation_id, &idle, pace) => None,
        // The session is still there and still saying nothing, having been asked
        // twice to say it where the human would hear. So it is ended here rather
        // than waited on any longer, and the stop written over it is one the
        // human presses Resume on — which starts a fresh follow-up session on
        // the same brief. Nothing it left open goes off with it: there being
        // nothing open is half of what said it was stuck.
        () = crate::rescues::until_it_will_not_ask(
            &state,
            conversation_id,
            event_id,
            &idle,
            pace,
            crate::rescues::Done::NothingElse,
        ) => {
            let _driving = driving;

            state.sessions.end(conversation_id).await;

            return stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "following the work up",
                crate::rescues::WOULD_NOT_ASK,
                Some(event_id),
            )
            .await;
        }
    };

    let Some(ended) = ended else {
        return over(&state, conversation_id, already, driving).await;
    };

    // Verkstead ended it — the human closed the Conversation or force-stopped it,
    // or the account it was spending ran out of window. Each has already written
    // the stop this would otherwise write. See
    // [`crate::sessions::Ended::on_purpose`].
    if ended.on_purpose() {
        tracing::info!(
            conversation_id,
            event_id,
            "the follow-up session was stopped from outside, so nothing is said about it",
        );
        return;
    }

    // The session is over on its own account, which is not by itself a follow-up
    // left unfinished. The human may have said there was nothing else and the
    // agent gone before the grace beside this had run out — which is the
    // ordinary shape of a session that finishes its turn rather than idling,
    // [`crate::sessions::Ended::Well`] being what an interactive agent with
    // nothing left to do exits as. So the record is asked once more before this
    // is read as a follow-up nobody is left to have, exactly as an instruction
    // session's commits are asked for again where it ends first.
    //
    // The same two questions [`nothing_else_and_quiet`] asks, and asked in the
    // same order: a Set still standing is the human holding a question, and one
    // is worth closing and stopping over whatever the newest answer said. Both
    // read the safe way round for this — a store that will not answer reads as
    // open and as not marked — so a record that cannot be asked leaves the stop
    // below exactly as it was.
    if !open(&state, conversation_id).await && marked(&state, conversation_id).await {
        tracing::info!(
            conversation_id,
            event_id,
            "the follow-up session finished on a round the human had already \
             marked, so the follow-up is over rather than gone",
        );

        return over(&state, conversation_id, already, driving).await;
    }

    // Held until the stop is written, which is what every driver here holds it
    // for: dropping first would leave a moment where a sweep could find the
    // Conversation undriven and stop it with a worse sentence.
    let _driving = driving;

    // And anything it left the human holding goes off as the stop is raised.
    // The session that asked is gone and no other is ever handed somebody else's
    // ask, so a Set left standing would keep the card blocked on you over a
    // question nobody is behind. See [`crate::responding`], whose rule this is.
    left_open(&state, conversation_id).await;

    // How it ended, where the ending itself was the problem; otherwise the
    // ending is the whole of it, a session that has finished with the follow-up
    // still running being exactly as gone as one that fell over.
    let how = match ended.badly() {
        Some(how) => format!("{how}, so {NOBODY_FOLLOWING_UP}"),
        None => format!("the follow-up session finished, so {NOBODY_FOLLOWING_UP}"),
    };

    stop(
        &state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "following the work up",
        &how,
        Some(event_id),
    )
    .await;
}

/// What a stop over a gone follow-up session says beyond how it went.
///
/// [`store::Decision::Verkstead`], as every stop written here is: what to do
/// about it is the human's, and steering is what they have.
const NOBODY_FOLLOWING_UP: &str = "nobody is left to ask you anything or to act on what you say, and any question it had \
     put to you has been closed unanswered";

/// The human has said there is nothing else: end the session, and put the
/// Conversation back in the wrap-up it was opened over.
///
/// **Where a follow-up ends is where it started.** It is something taken up
/// about work that is already on a pull request, so what is left when it is over
/// is that pull request and a wrap-up to see it out — over whatever the branch
/// now holds, with the review left settled and the fix attempts forgotten, which
/// is exactly what a steer into Wrapping recomputes. *Back to Done* is that
/// wrap-up's own settling rule and nothing decided here.
///
/// **The checks go back to waiting where the follow-up pushed.** `already` is
/// what the Conversation had committed before the session started, so more than
/// it now is a follow-up that gave GitHub a new run to make up its mind about —
/// and a settle standing over it is yesterday's green, which the settling loop
/// could reach Done on before the checks watcher's first poll had looked. A
/// follow-up that was questions and answers alone lands with everything settled
/// and passes straight through to Done. A count that will not read counts as
/// *pushed*, which is the right way round: the cost is one poll of GitHub, and
/// the cost the other way is a wrap-up finished over a suite nobody watched.
///
/// The session is ended first, because the Worktree is about to be handed to the
/// wrap-up's own watchers: a review queueing behind an agent that has nothing
/// left to do would wait for a session Verkstead is finished with.
async fn over(state: &AppState, conversation_id: i64, already: usize, driving: Driving) {
    tracing::info!(
        conversation_id,
        "the human has nothing else, so the follow-up is over and its session is being ended",
    );

    state.sessions.end(conversation_id).await;

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
async fn left_open(state: &AppState, conversation_id: i64) {
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

/// Wait until the follow-up is over: the human has marked their newest answer
/// *Nothing else*, nothing is left open on the Conversation, and the session has
/// printed nothing for [`Pace::proposing`].
///
/// All three, and none of them is enough alone. **The mark alone** would end a
/// follow-up in the middle of the work the last round asked for — the human
/// answers and the agent goes off and does it, which is the whole point of the
/// state. **Quiet alone** would reap a session idling on a Blocking Ask, which is
/// a session doing exactly what it should: the ask blocks for as long as the
/// human takes, and that may be the next morning. **Nothing open alone** would end
/// every follow-up the moment it started, none of them having asked anything yet.
///
/// The grace is asked first because it is the cheap half, exactly as
/// [`quiet_and_nothing_asked`] asks it: a session still talking is not one to ask
/// the store about, and anything it prints puts the whole grace back on the
/// clock. An answer arriving does the same — a session that has just been told
/// what to do has everything it asked for and nothing done yet — so the grace
/// runs again from the last time a Set was open, and this returns only once both
/// are spent.
///
/// **The mark is read last and every time round**, which is what makes the
/// latest Response the one that decides: a Set asked after an end-marked one puts
/// the follow-up back to running through the open-Set arm above, and its own
/// answer is what this reads when it comes.
///
/// **And a follow-up whose human never says *nothing else* is rescued rather
/// than waited on for ever.** That is this condition read the other way round —
/// quiet, nothing open and no mark — and it is watched for beside this rather
/// than here, by the one loop that watches for it in every state. See
/// [`crate::rescues::until_it_will_not_ask`], which takes the mark as its
/// done-indicator.
async fn nothing_else_and_quiet(state: &AppState, conversation_id: i64, idle: &Idle, pace: Pace) {
    // When a Set of the Conversation's was last seen open. An answer arriving is
    // something the session has just been given to act on, and one that has just
    // been given something has had no time to act on it yet — so the grace runs
    // again from here. `None` while it has asked nothing at all.
    let mut asked: Option<Instant> = None;

    loop {
        let owed = pace.proposing.saturating_sub(idle.for_how_long());

        if !owed.is_zero() {
            tokio::time::sleep(owed).await;
            continue;
        }

        if open(state, conversation_id).await {
            asked = Some(Instant::now());
            tokio::time::sleep(pace.poll).await;
            continue;
        }

        let owed = asked
            .map(|at| pace.proposing.saturating_sub(at.elapsed()))
            .unwrap_or_default();

        if !owed.is_zero() {
            tokio::time::sleep(owed).await;
            continue;
        }

        if marked(state, conversation_id).await {
            return;
        }

        tokio::time::sleep(pace.poll).await;
    }
}

/// Whether anything on the Conversation is still waiting on the human.
///
/// The Conversation's rather than any one session's, which is the question both
/// its readers are really asking: what matters is whether the human is left
/// holding something to answer, and something to answer is one whoever put it
/// up. See [`crate::rescues::until_it_will_not_ask`], which is the other reader
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

/// Whether the newest round the human answered carries the Nothing-else mark.
///
/// A store that will not answer reads as *not marked*, which leaves the
/// follow-up running: the same way round as [`open`], read from the other side.
pub(crate) async fn marked(state: &AppState, conversation_id: i64) -> bool {
    match store::nothing_else(&state.pool, conversation_id).await {
        Ok(marked) => marked,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a follow-up was over failed");
            false
        }
    }
}

/// Follow the grilling session as it stages the work into a roadmap.
///
/// The roadmap's counterpart to [`follow_breakdown`], and the same move: the
/// session that settled the work writes the artifact without leaving the context
/// that settled it, so nothing is launched here and what is followed is the
/// session already running.
///
/// The commit the branch came off is read here rather than handed in, because
/// this entry has nobody to have decided it — the pick arrives on a Response and
/// what it arms is a watcher, not a session. Without one there is nothing that
/// could say a roadmap in the Worktree is this branch's own, so the session is
/// left running rather than followed.
async fn follow_staging(state: AppState, conversation_id: i64, writing: Session, driving: Driving) {
    let Some(base) = base(&state, conversation_id).await else {
        return;
    };

    follow_roadmap(state, conversation_id, base, writing, driving).await
}

/// See a roadmap session out, and carry the Conversation on to wrapping the
/// pull request it opened.
///
/// The whole of a roadmap Conversation's own work in one session — the stages it
/// plans are Conversations of their own — so there is no next step to launch.
/// What there is, is the same ending a backlog's last step has: the session
/// commits the roadmap and then follows the repository's own finish sequence, so
/// the branch is pushed and on a pull request by the time it goes quiet. A
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
/// `writing` is the session that will commit the roadmap: the grilling session
/// itself, or a fresh one Resume launched. `base` is the commit the branch came
/// off, which is what says a roadmap on it is one this branch wrote — see
/// [`Landing::Roadmap`].
///
/// A session that ends without writing one stops the run, the way every other
/// step does.
async fn follow_roadmap(
    state: AppState,
    conversation_id: i64,
    base: String,
    session: Session,
    _driving: Driving,
) {
    let Some(writing) = see_out(&state, conversation_id, Step::Staging(base), session).await else {
        return;
    };

    // The roadmap is on the branch, so the record says where that happened —
    // before the pull request the same session went on to open, which is the
    // order the two happened in.
    crate::conversations::roadmap_landed(&state, conversation_id).await;

    to_a_pull_request(&state, conversation_id, Some(writing)).await;
}

/// Run one fix session about `feedback`, and wait until it is over.
///
/// The Timeline Event it printed into, or `None` where nothing could be
/// launched. What it committed is not returned and is not counted here: the
/// watcher that dispatched this asks GitHub what became of it, and a fix session
/// that pushed nothing is answered by the check still being red rather than by
/// anything read off the branch.
///
/// Ended on **committed plus quiet**, the way a backlog step is ended on landed
/// plus quiet, and for the same two reasons. A commit is the one report an agent
/// cannot half make, so it is what says the fix is done; and work does not always
/// stop at the commit — the push that puts it on the pull request comes after
/// one — so the session is ended only once it has printed nothing for the grace
/// period, with anything it prints putting the whole grace back on the clock.
///
/// Nothing is refused for and nothing is stopped. A fix session that ends having done
/// nothing is not by itself something to stop over: what
/// wrap-up is watching is the check, and the human is asked once the machine has
/// had its two goes at it — see [`crate::checks`].
///
/// **A session that will not ask is still spoken to**, which is the one thing
/// the rescue does here that it does everywhere — see [`crate::rescues`]. What
/// it does *not* do here is write the stop the other callers write: a fix
/// session is one of two goes at one check, and the state that dispatched it has
/// a stop of its own for when they run out. So the rescue ends the session and
/// the wrap-up carries on from the check, which is still red.
pub(crate) async fn address(state: &AppState, conversation_id: i64, feedback: &str) -> Option<i64> {
    // Taken before the session starts, so it is a count of what the branch
    // carried before this fix rather than one that includes it.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return None;
        }
    };

    let mut session = launch(
        state,
        conversation_id,
        Prompt::Addressing(feedback.to_owned()),
    )
    .await?;

    let event_id = session.event_id;
    let idle = session.idle.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        _ = committed_and_quiet(state, conversation_id, already, &idle, pace) => None,
        // Idle, with nothing committed and nothing put to the human, which is a
        // fix nobody can move on. Told twice and then ended where it stands; the
        // stop, where there is to be one, is the wrap-up's own once the branch
        // has had its two goes.
        () = crate::rescues::until_it_will_not_ask(
            state,
            conversation_id,
            event_id,
            &idle,
            pace,
            crate::rescues::Done::Committed { already },
        ) => {
            tracing::warn!(
                conversation_id,
                event_id,
                "the fix session went quiet without committing anything or asking about it, \
                 so it is being ended and the check looked at again",
            );

            state.sessions.end(conversation_id).await;

            return Some(event_id);
        }
    };

    if ended.is_none() {
        tracing::info!(
            conversation_id,
            event_id,
            "a fix session has committed and gone quiet, so it is being ended",
        );

        state.sessions.end(conversation_id).await;
    }

    Some(event_id)
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
    /// Which is nearly always Verkstead ending it on quiet with nothing pending
    /// rather than the session exiting — an interactive agent idles when its work
    /// is done. Both are this: see [`proposing`].
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
        Prompt::Reviewing { on, said },
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
        Prompt::Responding(said.to_owned()),
        "batch session",
    )
    .await
}

/// Run one session that proposes and then fixes, and wait until it is over.
///
/// **Ended on quiet with nothing pending**, which is a rule of its own. Every
/// other session here reports through the repository and is ended on what landed
/// plus quiet; one of these puts what it would do to the human in the middle of
/// its work and has the rest of that work to do afterwards, so there is no
/// landing to watch for and no commit it must make — a review that fixes nothing
/// is a review that did its job. What is left is silence, and every session is an
/// interactive agent that idles when its work is done rather than exiting, so
/// waiting to see one exit is waiting for something that never comes.
///
/// So it is quiet for [`Pace::proposing`] *and* nothing of its own left for the
/// human to answer, or it is left where it is. `verkstead ask` blocks for as long
/// as they take, and a session idling on a Blocking Ask is one working rather
/// than one stuck — quiet alone would reap it mid-question, however carefully the
/// grace was chosen. A Deferred Ask idles nobody and so holds nothing open: its
/// Answers reach a later session by design, and waiting on one would be waiting
/// for the human to answer something nothing was waiting for. Anything the
/// session prints puts the whole grace back on the clock, so one still talking is
/// never cut off. The Turn the caller is holding keeps the Worktree this
/// session's across the whole of it, the wait on the human included.
///
/// A session ended that way **is a session that finished**: it is
/// [`Reviewed::Done`], exactly as one that saw itself out is — as long as it said
/// something. A session ended on its own quiet is one being taken at its word,
/// and one that never said a word gave none, so that is a stop rather than a
/// review: see [`SAID_NOTHING`]. Where the session
/// ends first, how it ended is read exactly as an inline run's is: cleanly means
/// it did what it was sent to do, and anything else means it did not. Nothing is
/// refused for and nothing is stopped here — what to do about either of those is
/// the caller's, and both callers ask the same further question first: whether
/// anything the human accepted was left unlanded. See [`crate::review`] and
/// [`crate::responding`].
///
/// `what` names the session in the log, which is the one place the two differ
/// here.
async fn proposing(
    state: &AppState,
    conversation_id: i64,
    inside: Prompt,
    what: &'static str,
) -> Reviewed {
    let Some(mut session) = launch(state, conversation_id, inside).await else {
        return Reviewed::Nothing;
    };

    let event_id = session.event_id;
    let idle = session.idle.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => ended,
        () = quiet_and_nothing_asked(state, conversation_id, event_id, &idle, pace) => {
            // Whether that was a session finishing or one that never got going,
            // which is the whole of what its own words can be asked. Ended
            // either way — a session with nothing to say is not one to leave
            // holding the Worktree — and the difference is in what is made of
            // it.
            let said_anything = idle.said_anything();

            tracing::info!(
                conversation_id,
                event_id,
                what,
                said_anything,
                "the session has gone quiet with nothing of its own open, so it is \
                 being ended",
            );

            state.sessions.end(conversation_id).await;

            if !said_anything {
                return Reviewed::Stopped {
                    how: SAID_NOTHING.to_owned(),
                    writing: event_id,
                };
            }

            return Reviewed::Done;
        }
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

/// How a session that was ended on quiet without ever having said a word is
/// recorded.
///
/// Read as ending badly rather than as ending, which is the one place this rule
/// needs a second signal. Every other ending here pairs quiet with something the
/// session produced — a commit, a backlog, a handoff — so a session that came up
/// and did nothing satisfies none of them and stops the run. Quiet with nothing
/// pending is satisfied by pure silence, and a review is exactly the session
/// whose whole report is its own words: taking silence as *it found nothing* would
/// settle the review and carry a wrap-up to Done over a branch nobody read, with
/// nothing on the Timeline saying so.
///
/// So it is the human's to look at, and the Notice already says the rest of it —
/// what the last session said, which is nothing at all.
const SAID_NOTHING: &str =
    "the session never said anything, so nothing here says the work was done";

/// Wait until the session has printed nothing for [`Pace::proposing`] *and* has
/// no Question Set of its own still waiting to be answered.
///
/// Both, and neither on its own is enough. Quiet alone would reap a session
/// idling on a Blocking Ask, which is a session doing exactly what it should —
/// the ask blocks until the human answers, and that may be the next morning.
/// Nothing-open alone would end every session the moment it started, none of them
/// having asked anything yet.
///
/// The grace is asked first because it is the cheap half: a session still talking
/// is not one to ask the store about. And anything it prints puts the whole grace
/// back on the clock — see [`crate::sessions::Idle`] — so a session mid-sentence
/// is never one this ends, however long it goes on for.
///
/// An Answer arriving does the same, which is the other half of that rule: a
/// session that has just been told what to do has everything it asked for and
/// nothing done yet, and reaping it in the moment between the Response landing
/// and the agent finding its voice again would throw away the whole of what the
/// human decided. So the grace runs again from the last time an ask of its own
/// was open, and the session is ended only once both are spent.
async fn quiet_and_nothing_asked(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    idle: &Idle,
    pace: Pace,
) {
    // When it was last seen with an ask of its own open, and `None` while it has
    // asked nothing at all.
    let mut asked: Option<Instant> = None;

    loop {
        let owed = pace.proposing.saturating_sub(idle.for_how_long());

        if !owed.is_zero() {
            tokio::time::sleep(owed).await;
            continue;
        }

        if asking(state, conversation_id, event_id).await {
            asked = Some(Instant::now());
            tokio::time::sleep(pace.poll).await;
            continue;
        }

        let owed = asked
            .map(|at| pace.proposing.saturating_sub(at.elapsed()))
            .unwrap_or_default();

        if !owed.is_zero() {
            tokio::time::sleep(owed).await;
            continue;
        }

        return;
    }
}

/// Whether the session is idling on an ask of its own that nothing has
/// answered.
///
/// Its own, which is what the Event id says: nothing on the record names the
/// session a Set was asked by, and nothing has to — one Worktree holds one agent,
/// so every Set that landed after this session's Event is this session's. A
/// Deferred Ask is not one of them, and neither is a Set the human closed
/// unanswered: both are Sets nobody is idling on. A store-and-nudge one is,
/// stored though it is — the session that sent it has its turn ended and is
/// waiting to be nudged. See [`store::unanswered_set_since`].
///
/// A store that will not answer reads as *asking*, which is the right way round
/// for the one thing this decides: a session is ended on the strength of it, and
/// ending one mid-question would take the answer away from an agent that asked
/// for it.
async fn asking(state: &AppState, conversation_id: i64, event_id: i64) -> bool {
    match store::unanswered_set_since(&state.pool, conversation_id, event_id).await {
        Ok(open) => open.is_some(),
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading whether a session that proposes was still asking failed"
            );
            true
        }
    }
}

/// Wait until the Conversation has more commits than `already` *and* the session
/// has been quiet for the grace period.
///
/// The store rather than git, unlike a backlog step's landing: the branch watcher
/// is sweeping this branch for as long as the session runs and putting what lands
/// on the Timeline, so the Timeline is where a fresh commit shows up first — and
/// asking it costs one small read where asking git costs a process.
async fn committed_and_quiet(
    state: &AppState,
    conversation_id: i64,
    already: usize,
    idle: &Idle,
    pace: Pace,
) {
    loop {
        tokio::time::sleep(pace.poll).await;

        if !committed_since(state, conversation_id, already).await {
            continue;
        }

        loop {
            let owed = pace.grace.saturating_sub(idle.for_how_long());

            if owed.is_zero() {
                break;
            }

            tokio::time::sleep(owed).await;
        }

        return;
    }
}

/// Whether the Conversation has more commits on it than the `already` a session
/// started over.
///
/// The store rather than git, for [`committed_and_quiet`]'s reason: the branch
/// watcher is sweeping this branch for as long as the session runs and putting
/// what lands on the Timeline, so the Timeline is where a fresh commit shows up
/// first.
///
/// A store that will not answer reads as *nothing new*, which is the right way
/// round for both things this decides — a session ended, and a session left
/// alone rather than spoken to. See [`crate::rescues::Done::Committed`], which
/// is the other reader.
pub(crate) async fn committed_since(
    state: &AppState,
    conversation_id: i64,
    already: usize,
) -> bool {
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
async fn stop(
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

/// See one step's session out: end it once the step has landed and the session
/// has gone quiet, and say whether the step landed at all.
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
/// twice, and then ended where it stands and stopped over like any other step
/// that did not land. See [`crate::rescues`], whose done-indicator here is the
/// step's own [`Landing`].
///
/// `Some` is the Timeline Event the session printed into. The step landed, and
/// what comes after it may still want the session's own last words — the finish
/// step's does, because a stop over what the finish left behind is explained from
/// them.
async fn see_out(
    state: &AppState,
    conversation_id: i64,
    step: Step,
    mut session: Session,
) -> Option<i64> {
    let event_id = session.event_id;

    let landing = step.landing()?;

    let worktree = worktree(state, conversation_id).await?;

    // Taken before the session is waited on: the two are asked about together
    // below, and the clock is shared with the relay rather than owned by the
    // handle.
    let idle = session.idle.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        _ = landed_and_quiet(&worktree, &landing, &idle, pace) => None,
        // The step is not landing and the session is not asking about it: it has
        // gone idle with nothing open and nothing on the branch, which is a run
        // nobody can move. Told twice and then stopped where it stands — see
        // [`crate::rescues`], whose loop this is one of five callers of.
        () = crate::rescues::until_it_will_not_ask(
            state,
            conversation_id,
            event_id,
            &idle,
            pace,
            crate::rescues::Done::Landed {
                worktree: worktree.clone(),
                landing: landing.clone(),
            },
        ) => {
            tracing::warn!(
                conversation_id,
                event_id,
                step = ?step,
                "a session went quiet without finishing its step or asking about it, so the \
                 backlog stops here",
            );

            state.sessions.end(conversation_id).await;

            stop(
                state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                &step.what(),
                crate::rescues::WOULD_NOT_ASK,
                Some(event_id),
            )
            .await;

            return None;
        }
    };

    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            step = ?step,
            "a step has landed and its session has gone quiet, so it is being ended",
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
        .unwrap_or_else(|| "the session ended without finishing the step".to_owned());

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

/// Wait until `landing` has landed and the session has been quiet for the grace
/// period.
///
/// Two loops rather than one condition, because the second is not a poll: once
/// the step is done, what is left is sleeping out whatever quiet is still owed
/// and looking again. Output arriving in the meantime lengthens the wait rather
/// than ending it, and there is no cap on how long that may go on for.
async fn landed_and_quiet(worktree: &Path, landing: &Landing, idle: &Idle, pace: Pace) {
    loop {
        tokio::time::sleep(pace.poll).await;

        if !check(worktree, landing).await {
            continue;
        }

        loop {
            let owed = pace.grace.saturating_sub(idle.for_how_long());

            if owed.is_zero() {
                break;
            }

            tokio::time::sleep(owed).await;
        }

        return;
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

            return ticked && pending(worktree, &todo()) == Some(false);
        }
        // Not a path this branch was told about but one it went and wrote, so
        // what is asked is which roadmaps it has touched — the same reading the
        // pinned stage list is drawn by, so the list the human is watching and
        // the step the runner is waiting on cannot disagree.
        Landing::Roadmap(base) => {
            return !crate::stages::touched(worktree, base).is_empty()
                && pending(worktree, Path::new(crate::stages::ROADMAPS)) == Some(false);
        }
        // And this one is not in the Worktree at all, so there is no commit to
        // wait for: the document being there with something in it is the whole
        // of it, read by exactly the rule that will take it.
        Landing::Handoff(path) => return crate::handoffs::written(path),
    };

    if worktree.join(path).exists() != wanted {
        return false;
    }

    pending(worktree, path) == Some(false)
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
/// So it is asked for here. `Some` is that stage, carrying what
/// [`crate::skills::next_stage`] has to be told — the branch this stage's branch
/// stacks on, or `None` inside where it came off the default branch. `None` is
/// every other Conversation: one that is not a stage, one whose backlog has been
/// written already, and one with no base commit to read a branch's writing
/// against.
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
) -> Option<Option<String>> {
    // No base commit is a Conversation that never branched, which is not a stage
    // — a stage is made by branching — and is not something a branch's own
    // writing can be read against either.
    let base = base?;

    let stacked_on = match store::stacks_on(&state.pool, conversation_id).await {
        // The outer answer is whether this is a stage at all, and the inner one
        // is what its branch was made on top of. Only the outer one decides
        // anything here; the inner is carried through to the fork, which is the
        // one thing about a stage the repository does not say.
        Ok(stacked_on) => stacked_on?,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading whether a Conversation is a roadmap stage failed",
            );

            return None;
        }
    };

    (!wrote_a_backlog(worktree, Some(base)).await).then_some(stacked_on)
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
/// them separately: what the history holds — every commit that touched
/// `.tasks/`, the finish step's own deletion of it included — and what is in the
/// Worktree that no commit has taken yet. A backlog that was written and then
/// finished with is in the first; one a session wrote and died before committing
/// is in the second. A stage that never planned is in neither.
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
    let committed = git(worktree, &["log", "--format=%H", &since, "--", BACKLOG]);
    let uncommitted = git(worktree, &["status", "--porcelain", "--", BACKLOG]);

    match (committed, uncommitted) {
        (Some(committed), Some(uncommitted)) => {
            !committed.trim().is_empty() || !uncommitted.trim().is_empty()
        }
        _ => true,
    }
}

/// Whether `.tasks/` has anything left in it to work.
///
/// The one thing about a backlog anybody outside the runner asks: Resume refuses
/// by name where there is nothing to launch a session for, and what *nothing*
/// means is this module's own answer rather than a second reading of the
/// directory — see [`crate::resume`]. What is left is asked again by whatever
/// the resume spawns, a moment later and for itself.
pub(crate) async fn anything_to_work(worktree: &Path) -> bool {
    decide(worktree).await != Step::Nothing
}

/// What to run next in `worktree`, off the runtime's threads.
async fn decide(worktree: &Path) -> Step {
    let worktree = worktree.to_owned();

    match tokio::task::spawn_blocking(move || next_step(&worktree)).await {
        Ok(step) => step,
        Err(error) => {
            tracing::error!(error = ?error, "deciding what to run next failed");
            Step::Nothing
        }
    }
}

/// What to run next, decided from `.tasks/` alone.
///
/// The lowest-numbered entry whose box is not ticked, because the order the
/// backlog was written in is the order its slices depend on each other. Then the
/// finish step, once every box is ticked. Then nothing — which is a `.tasks/`
/// that was never written and one that has been finished with, and there is
/// nothing for the runner to do about either.
///
/// The list decides all of it, and the directory beside it only says which file
/// the session will be working from. A backlog part way through being written
/// has entries with no files yet, and reading those as done would be the runner
/// finishing a feature nobody had started: an entry that is not ticked and names
/// nothing is [`Step::Broken`], which stops the run and says so.
fn next_step(worktree: &Path) -> Step {
    let backlog = worktree.join(BACKLOG);

    let Some(entries) = crate::tasks::entries(&backlog) else {
        return Step::Nothing;
    };

    // In the list's own order rather than sorted: `TODO.md` is written in the
    // order the tasks are meant to be worked, and renumbering somebody's backlog
    // is not the runner's to do.
    let Some(next) = entries.into_iter().find(|entry| !entry.checked) else {
        return Step::Finish;
    };

    match crate::tasks::files(&backlog).get(&next.number) {
        Some(name) => Step::Task {
            number: next.number,
            file: Path::new(BACKLOG).join(name),
        },
        None => Step::Broken { label: next.label },
    }
}

/// Which skill a session is being started inside.
///
/// Not the same question as which step it is running. The runner decides the
/// step in order to know what to watch for, and the skill is what the session is
/// *told* — a task and the finish that follows the last of them are two steps
/// and one skill, because which of them it is, is read off `.tasks/` by the fork
/// rather than handed to it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Prompt {
    /// Its fork of next-stage, which writes a roadmap stage's backlog instead —
    /// carrying what the branch was made on top of, which is the one thing about
    /// a stage the session cannot read out of the repository.
    PlanningStage(Option<String>),

    /// Its fork of to-roadmap, which writes the roadmap and carries the branch
    /// to a pull request.
    Staging,

    /// Its fork of next-task, which every session of a backlog runs — the task
    /// sessions and the finish one alike.
    NextTask,

    /// The implementation skill, which is the whole of an inline run.
    Implementing,

    /// The submitting skill, which the session sent after a finish that left no
    /// pull request runs inside.
    ///
    /// Carries nothing, like every other prompt that carries nothing: what it is
    /// about is the branch, which the session reads for itself. A prompt of its
    /// own rather than the finish step's again, because the work is built — a
    /// session told to work the next task would find no backlog and nothing to
    /// do, and the one thing left is the one thing that skill says last. See
    /// [`to_a_pull_request`].
    Submitting,

    /// The instruction skill, carrying the hand-written work a steer into
    /// Implementing sent the session off with.
    ///
    /// Its own skill rather than the implementation's, because it is one
    /// session's whole job rather than a slice of the work the documents
    /// describe — and the pipeline carries on from what it commits, which is
    /// what the skill has to say and no other says — see [`instructed`].
    Instruction(String),

    /// The addressing skill, carrying the feedback the fix session is for.
    ///
    /// One of the prompts that has something in it, because it is a session
    /// launched *about* something rather than about the work as a whole: the
    /// ones that carry nothing are told where the work is written down and read
    /// it for themselves.
    Addressing(String),

    /// The reviewing skill, which the one session a wrap-up starts with runs
    /// inside — carrying every pull request the work ended up on, and whatever
    /// was said on them before it started, which is the other half of what it has
    /// to propose about.
    Reviewing {
        /// Every pull request the work is on, where it is on more than one: one
        /// review reads the whole of the work, and this is where the whole of it
        /// is.
        on: Option<String>,

        /// What was standing on those pull requests when the review started.
        said: Option<String>,
    },

    /// The responding skill, which a session answering a batch of comments runs
    /// inside — carrying the batch, which is the whole of what it is about.
    Responding(String),

    /// The following-up skill, carrying the brief a steer into Follow-up sent
    /// the session off with — and, where this is a follow-up being picked up
    /// again, the rounds it has already been through.
    ///
    /// The instruction's shape and never its meaning: what it carries opens a
    /// conversation rather than naming one job, so the session answers it, does
    /// what it asks and goes on asking — see [`following_up`].
    FollowingUp(FollowUp),
}

impl Prompt {
    /// Which of the Conversation's Pairings a session on this prompt is
    /// launched under.
    ///
    /// The review is the one that is not the Implementation Pairing, and the
    /// line is that reviewing is a fresh set of eyes and fixing is building:
    /// the check fixes, the comment responses and the follow-ups a wrap-up
    /// dispatches are all the work itself carrying on, so they run under what
    /// built it.
    fn role(&self) -> store::Role {
        match self {
            Self::Reviewing { .. } => store::Role::Review,
            _ => store::Role::Implementation,
        }
    }
}

/// Wait for the Conversation's Worktree, and then [`launch`] into it.
///
/// What every driver here launches through except the two that are already
/// holding the Turn when they get here — a fix session and the review, both
/// dispatched by a wrap-up that took it before it decided anything.
///
/// The wait is what keeps a launch from displacing a session somebody else put
/// there. [`crate::sessions::Sessions::start`] ends whatever is registered, which
/// is exactly what a run relaunching its own step wants and exactly what must
/// not happen to a session a steer set going — the human steers in a quiet
/// moment between steps, and a run that reached the next one a second later
/// would kill it mid-sentence. A steer holds the Turn for as long as its session
/// runs, so this waits for it rather than ending it.
///
/// Held across the launch alone rather than across the session, because that is
/// the whole of what it is protecting: once a session is registered, everything
/// else that might start one can see it there.
///
/// And a stop lands here, which is why it is asked after the wait rather than
/// before: *nothing new starts* has to be true of the moment a session would be
/// started, and the wait is however long the session in front of this one took.
/// Every launch a run makes goes through here, so this is the one place that can
/// say it of all of them — which is why what it asks is
/// [`crate::stopping::stopped`] and not [`crate::stops::asked`] alone. A press that
/// is still waiting is only one of the two ways a run stops: a Force stop writes
/// its stop outright, and so does a Stop pressed in one of the quiet moments
/// between sessions. A launch that looked for the waiting press alone would walk
/// straight past both of them.
async fn launch_in_turn(state: &AppState, conversation_id: i64, inside: Prompt) -> Option<Session> {
    let _turn = state.sessions.turn(conversation_id).await;

    if crate::stopping::stopped(state, conversation_id).await {
        return None;
    }

    launch(state, conversation_id, inside).await
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
/// itself carrying on runs under — see [`Prompt::role`].
fn under(
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

/// Start a fresh session on the next step, under the Profile the prompt's own
/// role names — the review Pairing for the wrap-up's review, and the
/// implementation one for everything else. Which account that comes to is
/// [`under`]'s to say.
///
/// Which step it is is not said: the bundled fork reads `.tasks/` and picks the
/// same one this did, by the same rule. Verkstead decides the step to know what
/// to watch for, not to hand it over — a runner that named the file would be a
/// second opinion about a question the skill is already asking.
///
/// The Conversation is read back every time rather than held across the run: a
/// backlog takes hours, and where an agent is about to be let loose is the one
/// thing that must not be guessed at.
///
/// The Turn is the caller's to have taken — see [`launch_in_turn`], which is
/// what the callers that are not already holding one launch through. It cannot
/// be taken here: a wrap-up takes the Turn before it decides what to dispatch,
/// and a lock taken twice by the one task is a task waiting on itself.
async fn launch(state: &AppState, conversation_id: i64, inside: Prompt) -> Option<Session> {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(conversation_id, "there is no Conversation left to work in");
            return None;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to work in failed");
            return None;
        }
    };

    let role = inside.role();

    let paired = under(
        role,
        &conversation.review_pairing,
        conversation.implementation_pairing.as_ref(),
    );

    let Some(pairing) = paired else {
        tracing::error!(
            conversation_id,
            role = ?role,
            "the Pairing that session runs under is gone, so no session was started"
        );
        return None;
    };

    let prompt = match crate::conversations::documents(&state.pool, conversation_id).await {
        Ok((brief, handoff)) => {
            let handoff = handoff.as_deref();

            match &inside {
                Prompt::PlanningStage(stacked_on) => {
                    skills::next_stage(&brief, stacked_on.as_deref())
                }
                Prompt::Staging => skills::staging(&brief),
                Prompt::NextTask => skills::next_task(&brief, handoff),
                // The one prompt that reads a Pairing to decide what it says:
                // a Conversation whose human picked *no grilling* has no
                // handoff because there was no interview, which is a different
                // thing from a grilling that ended without writing one. Asked
                // of the record every launch rather than carried from the
                // press, so a resumed run says it too — see
                // [`skills::ungrilled`].
                Prompt::Implementing if conversation.grilling_pairing.skipped() => {
                    skills::ungrilled(&brief)
                }
                Prompt::Implementing => skills::implementing(&brief, handoff),
                Prompt::Submitting => skills::submitting(&brief, handoff),
                Prompt::Instruction(instruction) => {
                    skills::instruction(&brief, handoff, instruction)
                }
                Prompt::Addressing(feedback) => skills::addressing(&brief, handoff, feedback),
                Prompt::Reviewing { on, said } => {
                    skills::reviewing(&brief, handoff, on.as_deref(), said.as_deref())
                }
                Prompt::Responding(said) => skills::responding(&brief, handoff, said),
                Prompt::FollowingUp(follow_up) => {
                    skills::following_up(&brief, handoff, &follow_up.brief, &follow_up.settled)
                }
            }
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what the work is failed");
            return None;
        }
    };

    // And whatever the human has since answered on this Conversation's Deferred
    // Asks that no session has been told about — under everything above, which
    // is where the newest and least general thing said goes. Here rather than in
    // `Sessions::start`, because this is where every session that *builds* is
    // launched from: the one that is not launched here is the one an Answer
    // must not be spent on — see [`crate::deferrals`].
    let folding = crate::deferrals::unfolded(&state.pool, conversation_id).await;
    let prompt = folding.under(&prompt);

    // One Worktree holds one agent. Every session a run launches of its own
    // accord follows one this has already ended, but a Resume follows one that
    // died — and a register still holding a relay that has not finished unwinding
    // would be two agents editing each other's files.
    state.sessions.end(conversation_id).await;

    match state
        .sessions
        .start(&state.pool, &state.nudges, &conversation, &pairing, &prompt)
        .await
    {
        Ok(session) => {
            // Once there is a session reading them, and only then: a launch that
            // came to nothing would otherwise cost the human the one session
            // their Answers were folded into.
            if session.is_some() {
                folding.recorded(&state.pool).await;
            }

            session
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "a task session could not be started");
            None
        }
    }
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

    /// The order the backlog was written in is the order its slices depend on
    /// each other, so the lowest entry still unticked is the only thing to run
    /// next.
    #[test]
    fn the_next_step_is_the_lowest_unticked_entry() {
        let dir = worktree(&list(0), &DOCUMENTS);

        assert_eq!(
            next_step(dir.path()),
            Step::Task {
                number: 1,
                file: Path::new(BACKLOG).join("01-first.md"),
            },
        );

        finish(dir.path(), "01");

        assert_eq!(
            next_step(dir.path()),
            Step::Task {
                number: 2,
                file: Path::new(BACKLOG).join("02-second.md"),
            },
        );
    }

    /// And the file staying where it is says nothing: what a session leaves
    /// behind is a ticked entry beside a document nobody deleted.
    #[test]
    fn a_task_file_left_behind_is_not_a_task_still_to_do() {
        let dir = worktree(&list(2), &DOCUMENTS);

        assert_eq!(
            next_step(dir.path()),
            Step::Task {
                number: 3,
                file: Path::new(BACKLOG).join("03-third.md"),
            },
            "the two ticked entries are done, documents and all",
        );
    }

    /// The other way round, and the case the whole rule is for: a backlog part
    /// way through being written has entries with no documents yet, and the
    /// runner stops rather than putting a session at nothing to work from.
    #[test]
    fn an_unticked_entry_with_no_file_stops_the_run() {
        let dir = worktree(&list(0), &["01-first.md"]);

        finish(dir.path(), "01");

        assert_eq!(
            next_step(dir.path()),
            Step::Broken {
                label: "02".to_owned()
            },
        );
    }

    /// By the number rather than by the name, which is the same answer for a
    /// zero-padded backlog and a different one for a backlog that got past nine.
    #[test]
    fn a_backlog_that_got_past_nine_is_still_worked_in_order() {
        let dir = worktree(
            "- [ ] 9: Ninth\n- [ ] 10: Tenth\n",
            &["9-ninth.md", "10-tenth.md"],
        );

        assert_eq!(
            next_step(dir.path()),
            Step::Task {
                number: 9,
                file: Path::new(BACKLOG).join("9-ninth.md"),
            },
        );
    }

    /// Every box ticked is the feature built, whatever is still sitting in
    /// `.tasks/` — the finish step is what takes the directory away.
    #[test]
    fn the_finish_step_is_what_is_left_once_every_entry_is_ticked() {
        let dir = worktree(&list(3), &DOCUMENTS);

        assert_eq!(next_step(dir.path()), Step::Finish);
    }

    /// A Worktree with no backlog is nothing to run. Both ways round: one that
    /// was never broken down, and one whose finish commit took `.tasks/` away.
    #[test]
    fn a_worktree_with_no_backlog_has_nothing_to_run() {
        let dir = worktree(&list(3), &DOCUMENTS);
        std::fs::remove_dir_all(dir.path().join(BACKLOG)).unwrap();

        assert_eq!(next_step(dir.path()), Step::Nothing);
        assert_eq!(next_step(Path::new("/nonexistent")), Step::Nothing);
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

        assert_eq!(next_step(path), Step::Nothing, "nothing left to work");

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

        assert_eq!(next_step(path), Step::Nothing, "there is nothing to work");
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

        assert_eq!(
            next_step(path),
            Step::Nothing,
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

    /// What each step turns on. The plan arrives, the finish takes the list
    /// away, and a task is its own entry ticked off in it.
    #[test]
    fn every_step_but_nothing_has_something_to_watch_for() {
        assert_eq!(
            Step::Planning.landing(),
            Some(Landing::Arrived(Path::new(".tasks/TODO.md").to_owned())),
        );
        assert_eq!(
            Step::Finish.landing(),
            Some(Landing::Gone(Path::new(".tasks/TODO.md").to_owned())),
        );
        assert_eq!(
            Step::Task {
                number: 1,
                file: Path::new(".tasks/01-first.md").to_owned(),
            }
            .landing(),
            Some(Landing::Ticked(1)),
        );
        assert_eq!(
            Step::Staging("d41f8a3b".to_owned()).landing(),
            Some(Landing::Roadmap("d41f8a3b".to_owned())),
        );
        assert_eq!(
            Step::Handoff(PathBuf::from("/srv/verkstead/handoffs/7/handoff.md")).landing(),
            Some(Landing::Handoff(PathBuf::from(
                "/srv/verkstead/handoffs/7/handoff.md"
            ))),
        );
        assert_eq!(Step::Nothing.landing(), None);
        assert_eq!(
            Step::Broken {
                label: "02".to_owned()
            }
            .landing(),
            None,
            "an entry with no file is nothing to run, so there is nothing to wait for",
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
                id: 1,
                name: model.to_owned(),
                account: store::Account::Claude {
                    claude_dir: PathBuf::from("/data/claude"),
                    config_file: PathBuf::from("/data/claude.json"),
                },
                models: vec![model.to_owned()],
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
