//! The auto-advancing task runner: once there is a backlog, it works itself.
//!
//! A fresh session per task, launched the moment the one before it has landed,
//! with no gate between them and nobody asked. That is the whole of it — and
//! everything difficult about it is in the two questions it has to answer from
//! the repository alone, because the sessions are ordinary interactive ones and
//! the repository is the only thing they report through.
//!
//! **What is next** is [`next_step`]: the lowest-numbered task file left, or the
//! finish step once only `TODO.md` is. Read off the Worktree by the same rule
//! the pinned Event is drawn by — see [`crate::tasks`] — so the list the human
//! is watching and the list the runner is working are one list.
//!
//! **When a step is over** is [`Landing`]: a path gone from the Worktree, or
//! arrived in it, *and* committed as it stands. A task file deleted but not
//! committed is a session still mid-task, and a commit is the one report an
//! agent cannot half make. The poll never takes `index.lock` — everything here
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

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use verkstead_schema::Direction;

use crate::AppState;
use crate::drivers::Driving;
use crate::github;
use crate::repos::git;
use crate::sessions::{Quiet, Session};
use crate::skills;
use crate::store;
use crate::tasks::{BACKLOG, TODO, numbered};

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
            stalls: crate::stalls::SWEPT_EVERY,
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

    /// Work this task file, the lowest-numbered one left.
    Task(PathBuf),

    /// Finish the feature: every task is done and only `TODO.md` is left.
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
enum Landing {
    Gone(PathBuf),
    Arrived(PathBuf),

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
            // Finishing a task is what deletes its file.
            Step::Task(file) => Some(Landing::Gone(file.clone())),
            // And the finish commit removes `TODO.md` with the rest of `.tasks/`.
            Step::Finish => Some(Landing::Gone(todo())),
            // The roadmap commit is what puts the stages under version control.
            Step::Staging(base) => Some(Landing::Roadmap(base.clone())),
            Step::Nothing => None,
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
            Step::Task(file) => format!("the task in {}", file.display()),
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
            let step = decide(&working_in).await;

            if step == Step::Nothing {
                // Either the breakdown never landed one or the finish took the
                // last of it away, and nothing here can tell those apart. So
                // nothing is launched — which the press has already refused by
                // name, this being the reading after the spawn rather than the
                // one in front of it. See [`crate::resume`].
                tracing::info!(
                    conversation_id,
                    "there is no backlog left to work, so nothing was started again"
                );
                return;
            }

            tracing::info!(conversation_id, step = ?step, "a stopped run is being taken up again");

            let Some(session) = launch_in_turn(&state, conversation_id, Prompt::NextTask).await
            else {
                return;
            };

            work(state, conversation_id, step, session, driving).await
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
    let Some((branch, found)) = crate::wrapping::asked(&state, conversation_id).await else {
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
/// stopped on the question of what became of it rather than on the writing.
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

        return crate::wrapping::opened(&state, conversation_id, None).await;
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
/// `stacked_on` is the branch this stage's branch was made on top of, which the
/// fork is told because it is the one thing about a stage the repository does not
/// say.
pub(crate) async fn plan_stage(state: AppState, conversation_id: i64, stacked_on: Option<String>) {
    // Taken here rather than by [`crate::continuing`], which is the one place
    // that could have taken it earlier and has nothing to gain from doing so: a
    // stage is a Conversation made moments ago, and this is spawned as the last
    // thing that makes it.
    let driving = state.drivers.driving(conversation_id);

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
        crate::wrapping::opened(&state, conversation_id, Some(writing)).await;
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

        tracing::info!(conversation_id, step = ?step, "a fresh session is starting on the next step");

        let Some(started) = launch_in_turn(&state, conversation_id, Prompt::NextTask).await else {
            return;
        };

        let Some(writing) = see_out(&state, conversation_id, step.clone(), started).await else {
            return;
        };

        if step == Step::Finish {
            crate::wrapping::opened(&state, conversation_id, Some(writing)).await;
            return;
        }
    }
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
/// Landing is measured against what was already there rather than against zero,
/// which is what makes a second go answerable: a first attempt that committed
/// twice and then died leaves two commits behind, and a second that commits
/// nothing has still landed nothing.
///
/// What follows a session that landed something is the same ending a backlog's
/// finish step has: the session followed the repository's own review process on
/// its way out, so the branch is pushed and on a pull request by the time it
/// goes quiet, and [`crate::wrapping::opened`] is what finds that pull request
/// and moves the Conversation on. An inline implementation is work like any
/// other work and goes for review like any other work.
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
    let already = match store::recorded_commits(&state.pool, conversation_id).await {
        Ok(recorded) => recorded.len(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let ended = session.ended().await;

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
    let landed = match store::recorded_commits(&state.pool, conversation_id).await {
        Ok(recorded) => recorded.len() > already,
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

            crate::wrapping::opened(&state, conversation_id, Some(event_id)).await;
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
            Some((_, Err(github::Trouble::NoPullRequest))) | None => {
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
pub(crate) async fn instructed(
    state: AppState,
    conversation_id: i64,
    instruction: String,
    driving: Driving,
) {
    // Taken before the session starts, so it is a count of what the branch
    // carried before the instruction rather than one that includes what it goes
    // on to do.
    let already = match store::recorded_commits(&state.pool, conversation_id).await {
        Ok(recorded) => recorded.len(),
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
    let quiet = session.quiet.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        () = committed_and_quiet(&state, conversation_id, already, &quiet, pace) => None,
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
    let landed = match store::recorded_commits(&state.pool, conversation_id).await {
        Ok(recorded) => recorded.len() > already,
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
/// **And a stop where the branch holds neither**, because there is nothing left
/// that could be started and a Conversation left implementing with nothing
/// driving it would be one the stall sweep stopped a minute later with a worse
/// sentence. `writing` is the Event the session printed into, so the Notice
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

    match store::pull_request(&state.pool, conversation_id).await {
        Ok(Some(_)) => {
            tracing::info!(
                conversation_id,
                "the instruction is done and the branch is on a pull request, so it is \
                 wrapped up again",
            );

            crate::wrapping::opened(&state, conversation_id, Some(writing)).await;
        }
        Ok(None) => {
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
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether the branch is on a pull request failed");
        }
    }
}

/// See out the session a steer into Follow-up started, and stop the
/// Conversation when it is over.
///
/// **A driver rather than an errand beside the work**, exactly as an instruction
/// session is: the registration it is handed says the Conversation is being
/// driven for as long as this runs, so nothing sweeps it as standing still while
/// the human is composing an answer on a phone.
///
/// **Nothing is watched for.** A follow-up is rounds of asking rather than a
/// step with a landing: what it commits is the human's to have asked for, and a
/// round that was a question and an answer commits nothing at all. So there is
/// no committed-and-quiet to end it on and no artifact to read — the session
/// runs until it is over, and the ending is what this waits for.
///
/// **And what its ending means is not settled here yet.** A follow-up is over
/// when the human says there is nothing else, which is a thing they have no way
/// to say so far; until they do, a session that has ended is a Conversation with
/// nobody following anything up, so the run stops and says so and the human is
/// left with the Timeline and the Steer button. What replaces this is the rule
/// that reads their answer and lands the Conversation back in the wrap-up.
pub(crate) async fn following_up(
    state: AppState,
    conversation_id: i64,
    brief: String,
    driving: Driving,
) {
    let Some(mut session) =
        launch_in_turn(&state, conversation_id, Prompt::FollowingUp(brief)).await
    else {
        return;
    };

    let event_id = session.event_id;
    let ended = session.ended().await;

    // Held until the stop is written, which is what every driver here holds it
    // for: dropping first would leave a moment where a sweep could find the
    // Conversation undriven and stop it with a worse sentence.
    let _driving = driving;

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

    // How it ended, where the ending itself was the problem; otherwise the
    // ending is the whole of it, there being nothing yet that carries a
    // follow-up on from a session that has finished.
    let how = ended
        .badly()
        .unwrap_or_else(|| "the follow-up session finished".to_owned());

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
/// [`crate::wrapping::opened`]'s, made as the pull request is recorded.
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

    crate::wrapping::opened(&state, conversation_id, Some(writing)).await;
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
pub(crate) async fn address(state: &AppState, conversation_id: i64, feedback: &str) -> Option<i64> {
    // Taken before the session starts, so it is a count of what the branch
    // carried before this fix rather than one that includes it.
    let already = match store::recorded_commits(&state.pool, conversation_id).await {
        Ok(recorded) => recorded.len(),
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
    let quiet = session.quiet.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        _ = committed_and_quiet(state, conversation_id, already, &quiet, pace) => None,
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
/// `said` is what was written on the pull request before this started, which the
/// caller reads inside the Turn it is holding and records as addressed — so this
/// session is the one that proposes about it, and nothing else is sent to.
pub(crate) async fn review(
    state: &AppState,
    conversation_id: i64,
    said: Option<String>,
) -> Reviewed {
    proposing(state, conversation_id, Prompt::Reviewing(said), "review").await
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
    let quiet = session.quiet.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => ended,
        () = quiet_and_nothing_asked(state, conversation_id, event_id, &quiet, pace) => {
            // Whether that was a session finishing or one that never got going,
            // which is the whole of what its own words can be asked. Ended
            // either way — a session with nothing to say is not one to leave
            // holding the Worktree — and the difference is in what is made of
            // it.
            let said_anything = quiet.said_anything();

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
/// back on the clock — see [`crate::sessions::Quiet`] — so a session mid-sentence
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
    quiet: &Quiet,
    pace: Pace,
) {
    // When it was last seen with an ask of its own open, and `None` while it has
    // asked nothing at all.
    let mut asked: Option<Instant> = None;

    loop {
        let owed = pace.proposing.saturating_sub(quiet.for_how_long());

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

/// Whether the session has a Blocking Ask of its own that nothing has answered.
///
/// Its own, which is what the Event id says: nothing on the record names the
/// session a Set was asked by, and nothing has to — one Worktree holds one agent,
/// so every Set that landed after this session's Event is this session's. A
/// Deferred Ask is not one of them, and neither is a Set the human closed
/// unanswered: both are Sets nobody is idling on. See
/// [`store::unanswered_set_since`].
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
    quiet: &Quiet,
    pace: Pace,
) {
    loop {
        tokio::time::sleep(pace.poll).await;

        match store::recorded_commits(&state.pool, conversation_id).await {
            Ok(recorded) if recorded.len() > already => {}
            // Nothing new, or a store that would not answer — which reads as
            // nothing new for the reason a repository that will not answer reads
            // as *not landed*: a session is ended on the strength of this.
            Ok(_) => continue,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading what a fix session committed failed");
                continue;
            }
        }

        loop {
            let owed = pace.grace.saturating_sub(quiet.for_how_long());

            if owed.is_zero() {
                break;
            }

            tokio::time::sleep(owed).await;
        }

        return;
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
    let quiet = session.quiet.clone();
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        _ = landed_and_quiet(&worktree, &landing, &quiet, pace) => None,
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
async fn landed_and_quiet(worktree: &Path, landing: &Landing, quiet: &Quiet, pace: Pace) {
    loop {
        tokio::time::sleep(pace.poll).await;

        if !check(worktree, landing).await {
            continue;
        }

        loop {
            let owed = pace.grace.saturating_sub(quiet.for_how_long());

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
async fn check(worktree: &Path, landing: &Landing) -> bool {
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
/// The lowest-numbered task file left, because the order the backlog was written
/// in is the order its slices depend on each other. Then the finish step, once
/// the only thing left is the list itself. Then nothing — which is a `.tasks/`
/// that was never written and one that has been finished with, and there is
/// nothing for the runner to do about either.
fn next_step(worktree: &Path) -> Step {
    let backlog = worktree.join(BACKLOG);

    let mut left: Vec<(u32, String)> = match std::fs::read_dir(&backlog) {
        Ok(listed) => listed
            .flatten()
            .filter_map(|file| {
                let name = file.file_name().to_string_lossy().into_owned();
                numbered(&name).map(|number| (number, name))
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    // By the number rather than by the name, so `9-` comes before `10-` where a
    // backlog was written without zero-padding. The name breaks the tie, which
    // is only ever two files claiming one number.
    left.sort();

    if let Some((_, name)) = left.into_iter().next() {
        return Step::Task(Path::new(BACKLOG).join(name));
    }

    if backlog.join(TODO).is_file() {
        return Step::Finish;
    }

    Step::Nothing
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
    /// inside — carrying whatever was said on the pull request before it started,
    /// which is the other half of what it has to propose about.
    Reviewing(Option<String>),

    /// The responding skill, which a session answering a batch of comments runs
    /// inside — carrying the batch, which is the whole of what it is about.
    Responding(String),

    /// The following-up skill, carrying the brief a steer into Follow-up sent
    /// the session off with.
    ///
    /// The instruction's shape and never its meaning: what it carries opens a
    /// conversation rather than naming one job, so the session answers it, does
    /// what it asks and goes on asking — see [`following_up`].
    FollowingUp(String),
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

/// Start a fresh session on the next step, under the Conversation's
/// implementation Profile.
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

    let Some(pairing) = conversation.implementation_pairing.clone() else {
        tracing::error!(
            conversation_id,
            "the implementation Pairing is gone, so no session was started"
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
                Prompt::Implementing => skills::implementing(&brief, handoff),
                Prompt::Instruction(instruction) => {
                    skills::instruction(&brief, handoff, instruction)
                }
                Prompt::Addressing(feedback) => skills::addressing(&brief, handoff, feedback),
                Prompt::Reviewing(said) => skills::reviewing(&brief, handoff, said.as_deref()),
                Prompt::Responding(said) => skills::responding(&brief, handoff, said),
                Prompt::FollowingUp(follow_up) => skills::following_up(&brief, handoff, follow_up),
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

    /// A worktree with a backlog in it: `TODO.md` and whichever task files are
    /// still to do, committed as a session that finished the ones before them
    /// would have left it.
    fn worktree(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--initial-branch", "main"]);
        run(path, &["config", "user.email", "test@verkstead.invalid"]);
        run(path, &["config", "user.name", "Verkstead Test"]);

        let backlog = path.join(BACKLOG);
        std::fs::create_dir_all(&backlog).unwrap();
        std::fs::write(backlog.join(TODO), "# Rate limiting\n").unwrap();

        for file in files {
            std::fs::write(backlog.join(file), "# a task\n").unwrap();
        }

        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "chore: plan rate-limiting tasks"]);

        dir
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
    /// each other, so the lowest number left is the only thing to run next.
    #[test]
    fn the_next_step_is_the_lowest_numbered_task_file_left() {
        let dir = worktree(&["03-third.md", "01-first.md", "02-second.md"]);

        assert_eq!(
            next_step(dir.path()),
            Step::Task(Path::new(BACKLOG).join("01-first.md")),
        );

        std::fs::remove_file(dir.path().join(BACKLOG).join("01-first.md")).unwrap();

        assert_eq!(
            next_step(dir.path()),
            Step::Task(Path::new(BACKLOG).join("02-second.md")),
        );
    }

    /// By the number rather than by the name, which is the same answer for a
    /// zero-padded backlog and a different one for a backlog that got past nine.
    #[test]
    fn a_backlog_that_got_past_nine_is_still_worked_in_order() {
        let dir = worktree(&["9-ninth.md", "10-tenth.md"]);

        assert_eq!(
            next_step(dir.path()),
            Step::Task(Path::new(BACKLOG).join("9-ninth.md")),
        );
    }

    /// `TODO.md` is the list rather than a task, and the runner has to be able
    /// to tell them apart — it is what is left when every task is done.
    #[test]
    fn the_finish_step_is_what_is_left_once_the_task_files_have_gone() {
        let dir = worktree(&[]);

        assert_eq!(next_step(dir.path()), Step::Finish);
    }

    /// A Worktree with no backlog is nothing to run. Both ways round: one that
    /// was never broken down, and one whose finish commit took `.tasks/` away.
    #[test]
    fn a_worktree_with_no_backlog_has_nothing_to_run() {
        let dir = worktree(&[]);
        std::fs::remove_dir_all(dir.path().join(BACKLOG)).unwrap();

        assert_eq!(next_step(dir.path()), Step::Nothing);
        assert_eq!(next_step(Path::new("/nonexistent")), Step::Nothing);
    }

    /// The done-signal, and the half of it that matters most: the file is gone,
    /// but the commit removing it has not landed, so the session is still
    /// mid-task.
    #[test]
    fn a_task_file_deleted_but_not_committed_is_a_session_still_working() {
        let dir = worktree(&["01-first.md"]);
        let landing = Landing::Gone(Path::new(BACKLOG).join("01-first.md"));

        assert!(!landed(dir.path(), &landing), "the file is still there");

        std::fs::remove_file(dir.path().join(BACKLOG).join("01-first.md")).unwrap();

        assert!(
            !landed(dir.path(), &landing),
            "deleted, and the deletion is not committed",
        );

        run(dir.path(), &["add", "-A"]);

        assert!(
            !landed(dir.path(), &landing),
            "staged is not committed either",
        );

        run(dir.path(), &["commit", "-m", "feat: count the requests"]);

        assert!(landed(dir.path(), &landing), "gone and committed");
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
        let dir = worktree(&["01-first.md"]);
        let path = dir.path();

        std::fs::remove_file(path.join(BACKLOG).join("01-first.md")).unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "-m", "feat: count the requests"]);

        // What a session mid-commit has left in the repository.
        let lock = path.join(".git/index.lock");
        std::fs::write(&lock, "").unwrap();

        assert!(
            landed(path, &Landing::Gone(Path::new(BACKLOG).join("01-first.md"))),
            "a locked repository is still a repository to read",
        );
        assert!(
            lock.exists(),
            "the session's lock is still the session's: nothing here took it or cleared it",
        );
    }

    /// Which path each step turns on. The plan arrives, everything else goes.
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
            Step::Task(Path::new(".tasks/01-first.md").to_owned()).landing(),
            Some(Landing::Gone(Path::new(".tasks/01-first.md").to_owned())),
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
    }

    /// The handoff is the one landing with no repository in it. Nothing puts the
    /// document under version control — it is written outside the checkout on
    /// purpose — so what says the step is over is the document being there with
    /// something in it, and a Worktree with everything committed says nothing
    /// about it either way.
    #[test]
    fn the_handoff_lands_outside_the_worktree_entirely() {
        let dir = worktree(&[]);
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
}
