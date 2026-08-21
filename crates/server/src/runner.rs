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
//! A step whose session ends without landing it stops the run where it is, and
//! what it stops at is an Interruption: the evidence goes on the Timeline and the
//! human picks a remedy — see [`crate::interruptions`]. The run does not go round
//! again while one is open, which is checked here as well as enforced by the
//! store's index, because the check the runner makes is the one that decides
//! whether to spend an account on a step nobody has looked at yet.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::AppState;
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
}

impl Default for Pace {
    fn default() -> Pace {
        Pace {
            poll: Duration::from_secs(2),
            grace: Duration::from_secs(5),
            checks: crate::checks::ASKED_EVERY,
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
}

impl Step {
    /// What would say this step is over, or `None` where it is not a step to
    /// run.
    fn landing(&self) -> Option<Landing> {
        match self {
            // The plan commit is what puts the backlog under version control,
            // so the backlog being there and committed is the breakdown done.
            Step::Planning | Step::PlanningStage => Some(Landing::Arrived(todo())),
            // Finishing a task is what deletes its file.
            Step::Task(file) => Some(Landing::Gone(file.clone())),
            // And the finish commit removes `TODO.md` with the rest of `.tasks/`.
            Step::Finish => Some(Landing::Gone(todo())),
            // The roadmap commit is what puts the stages under version control.
            Step::Staging(base) => Some(Landing::Roadmap(base.clone())),
            Step::Nothing => None,
        }
    }

    /// The step as an Interruption records it — which is what a retry launches
    /// again.
    ///
    /// Which task it was is not part of it, and need not be: a retry runs the
    /// fork that reads `.tasks/` and takes the lowest number left, which is the
    /// same answer the run was already working from.
    ///
    /// [`Step::Nothing`] is not a step a session was launched for, so it can
    /// never be one an Interruption is raised about — it is the answer that ends
    /// the run rather than one that runs anything.
    fn stored(&self) -> Option<store::Step> {
        match self {
            Step::Planning => Some(store::Step::Planning),
            Step::PlanningStage => Some(store::Step::Stage),
            Step::Task(_) => Some(store::Step::Task),
            Step::Finish => Some(store::Step::Finish),
            Step::Staging(_) => Some(store::Step::Roadmap),
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

/// Work `conversation_id`'s backlog to empty, starting from the session that is
/// writing it.
///
/// `planning` is the breakdown session the task-list direction has just
/// launched. It is the run's first step rather than something that happened
/// before the run: it is an ordinary interactive session and will idle once its
/// plan is committed, so something has to see it out — and what sees a session
/// out is exactly this.
///
/// Returns when there is nothing left to run: the backlog worked through, a
/// Conversation that has gone, or a step whose session ended without landing it.
/// Idle rather than looping — a runner that relaunched a session for a step
/// nothing had moved would be a machine spending an account on the same failure
/// over and over, with nobody watching. What it leaves behind for the human is an
/// Interruption, which is where the run picks up again if they retry.
///
pub(crate) async fn follow(state: AppState, conversation_id: i64, planning: Session) {
    work(state, conversation_id, Step::Planning, planning).await
}

/// Run a step again because the human asked for it, and go on working the
/// backlog from there.
///
/// The step is decided and the session launched *here* rather than by whoever
/// took the remedy, and that is the whole reason this exists: which step a
/// session is for has to be settled before the session is started. A driver that
/// launched first and read `.tasks/` afterwards would be racing the agent it had
/// just let loose — a breakdown that commits quickly would have its own backlog
/// read back as the step it was working.
///
/// `note` is what the human wrote alongside the retry, which reaches the agent as
/// part of its prompt — see [`skills::retrying`].
pub(crate) async fn retry(state: AppState, conversation_id: i64, step: store::Step, note: String) {
    let Some(working_in) = worktree(&state, conversation_id).await else {
        return;
    };

    // What to run again, and what watching it means. A retried planning step is
    // the breakdown over again; a retried task or finish is whatever `.tasks/`
    // now has left, which is the same answer the fork the session runs will come
    // to. Inline is not a backlog step at all and is followed on its own.
    let (step, prompt) = match step {
        store::Step::Planning => (Step::Planning, Prompt::BreakingDown),
        // The same first step by the other route, and the same care about where
        // the branch stands: what it was made on top of was decided once, when
        // it was made, and is read back rather than decided again — a retry that
        // came to a different answer would be a session told to stack on
        // something nobody had stacked it on.
        store::Step::Stage => (
            Step::PlanningStage,
            Prompt::PlanningStage(stacked_on(&state, conversation_id).await.flatten()),
        ),
        store::Step::Task | store::Step::Finish => match decide(&working_in).await {
            Step::Nothing => {
                // Nothing left to work, which for a retried finish is the
                // ordinary case rather than a dead end: the finish commit landed
                // and what failed was finding the pull request it opened. So the
                // retry is that question asked again — of a `gh` that has since
                // been logged in, or of a PR the human opened by hand.
                if step == store::Step::Finish {
                    tracing::info!(
                        conversation_id,
                        "the backlog is finished, so the retry looks for the pull request again"
                    );

                    return crate::wrapping::opened(&state, conversation_id, None).await;
                }

                tracing::info!(
                    conversation_id,
                    "the backlog the retried step belonged to has gone, so nothing was launched"
                );
                return;
            }
            step => (step, Prompt::NextTask),
        },
        store::Step::Inline => {
            let Some(session) = launch(&state, conversation_id, Prompt::Implementing, &note).await
            else {
                return;
            };

            return follow_inline(state, conversation_id, session).await;
        }
        // Nor is this one. A roadmap Conversation has one step of its own, so a
        // retry is that step again — unless the roadmap already landed, which is
        // the same case a retried finish has: what failed was finding the pull
        // request it opened, and the retry is that question asked again.
        store::Step::Roadmap => {
            let Some(base) = base(&state, conversation_id).await else {
                return;
            };

            let staged = {
                let worktree = working_in.clone();
                let base = base.clone();

                tokio::task::spawn_blocking(move || {
                    !crate::stages::touched(&worktree, &base).is_empty()
                })
                .await
                .unwrap_or(false)
            };

            if staged {
                tracing::info!(
                    conversation_id,
                    "the roadmap is written, so the retry looks for the pull request again"
                );

                return crate::wrapping::opened(&state, conversation_id, None).await;
            }

            let Some(session) = launch(&state, conversation_id, Prompt::Staging, &note).await
            else {
                return;
            };

            return follow_roadmap(state, conversation_id, base, session).await;
        }
        // Not a backlog step at all: the backlog was finished before this
        // Conversation ever had a pull request to have checks on. Retrying it is
        // the fix sessions starting over, which is the watcher's own to do — see
        // [`crate::checks::retried`]. Nothing is launched from here, because what
        // to dispatch is decided by asking GitHub rather than by reading
        // `.tasks/`.
        store::Step::Checks => return crate::checks::retried(state, conversation_id).await,
        // Nor is this one, and for the same reason one step further round: what a
        // retried review runs is the review again, in a session as fresh as the
        // first — which is [`crate::review`]'s to launch, because it is the one
        // thing that knows what to do with what the review comes back with.
        store::Step::Review => return crate::review::retried(state, conversation_id).await,
    };

    tracing::info!(conversation_id, step = ?step, "a retried step is starting in a fresh session");

    let Some(session) = launch(&state, conversation_id, prompt, &note).await else {
        return;
    };

    work(state, conversation_id, step, session).await
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
    let Some(session) = launch(
        &state,
        conversation_id,
        Prompt::PlanningStage(stacked_on),
        "",
    )
    .await
    else {
        return;
    };

    work(state, conversation_id, Step::PlanningStage, session).await
}

/// Work a backlog from `first` to empty.
///
/// `first` is the step the session it is handed is running, decided before that
/// session was started — see [`retry`] for why that ordering is the whole of it.
async fn work(state: AppState, conversation_id: i64, first: Step, session: Session) {
    let mut ran = first;
    let mut session = session;

    loop {
        let Some(writing) = see_out(&state, conversation_id, ran.clone(), session).await else {
            return;
        };

        // The finish step is the last one a backlog has, and landing it is not
        // the end of the run: what the finish did was push and open a pull
        // request, and the Conversation moves on to wrapping that up. Asked here
        // rather than after the loop, because this is the one place that knows
        // *which* step just landed.
        if ran == Step::Finish {
            crate::wrapping::opened(&state, conversation_id, Some(writing)).await;
            return;
        }

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

        // Asked before anything is launched, because that is what *the run does
        // not advance past an Interruption* means: the store's index makes two
        // open ones impossible, and this makes the second session impossible. A
        // run whose step landed while an older Interruption was still open would
        // otherwise carry on with the human still being asked about it.
        if stopped(&state, conversation_id).await {
            return;
        }

        tracing::info!(conversation_id, step = ?step, "a fresh session is starting on the next step");

        let Some(started) = launch(&state, conversation_id, Prompt::NextTask, "").await else {
            return;
        };

        ran = step;
        session = started;
    }
}

/// See an inline implementation session out, and stop the run at an Interruption
/// if it ends having landed nothing.
///
/// The whole of the work in one session, so there is no next step to launch and
/// nothing to poll a Worktree for: what says an inline session did anything is
/// what it committed, which the branch watcher is putting on the Timeline while
/// it runs.
///
/// Landing is measured against what was already there rather than against zero,
/// which is what makes a retry answerable: a first attempt that committed twice
/// and then died leaves two commits behind, and a retry that commits nothing has
/// still landed nothing.
pub(crate) async fn follow_inline(state: AppState, conversation_id: i64, mut session: Session) {
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

    // Verkstead ended it, which for an inline run means the human aborted the
    // Conversation: nothing was left to land because they stopped it. Answered
    // before the branch is read, since an aborted run has committed nothing and
    // would otherwise read as a session that did nothing.
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
        // done. What becomes of the work from here is the wrap-up phase's.
        (None, true) => {
            tracing::info!(
                conversation_id,
                event_id,
                "an inline session has landed its work"
            );
            return;
        }
        // Exited cleanly having committed nothing at all. An interactive agent
        // that decides there is nothing to do exits zero, so this is exactly the
        // case a status could not have caught.
        (None, false) => "the session ended without committing anything".to_owned(),
        // Ended badly, whether or not it got some of the way: the human is owed
        // the choice either way, and what it committed is on the Timeline above
        // the Interruption for them to read.
        (Some(badly), _) => badly,
    };

    stop(
        &state,
        conversation_id,
        store::Step::Inline,
        "implementing the work inline",
        &how,
        event_id,
    )
    .await;
}

/// See a roadmap session out, and carry the Conversation on to wrapping the
/// pull request it opened.
///
/// The whole of a roadmap Conversation's own work in one session — the stages it
/// plans are Conversations of their own — so there is no next step to launch.
/// What there is, is the same ending a backlog's last step has: the fork commits
/// the roadmap and then follows the repository's own finish sequence, so the
/// branch is pushed and on a pull request by the time the session goes quiet. A
/// roadmap is work like any other work and goes for review like any other work.
///
/// `base` is the commit the branch came off, which is what says a roadmap on it
/// is one this branch wrote — see [`Landing::Roadmap`].
///
/// A session that ends without writing one stops the run at an Interruption, the
/// way every other step does, and the human's remedies mean what they always
/// mean.
pub(crate) async fn follow_roadmap(
    state: AppState,
    conversation_id: i64,
    base: String,
    session: Session,
) {
    let Some(writing) = see_out(&state, conversation_id, Step::Staging(base), session).await else {
        return;
    };

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
/// Nothing is refused for and no Interruption is raised. A fix session that ends
/// having done nothing is not by itself something to ask the human about: what
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
        "",
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

/// What a review session left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reviewed {
    /// It put its findings to the human. The Set is on the Timeline, and what
    /// becomes of each finding is theirs to say.
    Asked,

    /// It ended, cleanly, having asked nothing at all: the review found nothing
    /// worth raising. What it thought is the last thing it printed, which is on
    /// the Timeline as its own Event.
    FoundNothing,

    /// It ended without asking anything, and not well. Which is not a review
    /// that found nothing — this is a review that did not happen.
    Stopped {
        /// How it ended, in the words an Interruption records.
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
/// Ended on **asked**, where a fix session is ended on committed: a review
/// reports by putting its findings to the human, and the ask is the one report it
/// cannot half make. There is no grace period after it, unlike every other
/// session here — `verkstead ask` blocks until the human answers, so a review
/// that has asked is a session doing nothing but wait, and what it is waiting for
/// is not its to act on.
///
/// A session that ends without asking is read off how it ended, exactly as an
/// inline run is: cleanly means it found nothing, and anything else means the
/// review did not happen. Nothing is refused for and no Interruption is raised
/// here — what to do about each of those is [`crate::review`]'s.
pub(crate) async fn review(state: &AppState, conversation_id: i64) -> Reviewed {
    let Some(mut session) = launch(state, conversation_id, Prompt::Reviewing, "").await else {
        return Reviewed::Nothing;
    };

    let event_id = session.event_id;
    let pace = state.sessions.pace();

    let ended = tokio::select! {
        ended = session.ended() => Some(ended),
        _ = asked(state, conversation_id, pace) => None,
    };

    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            "the review has put its findings to the human, so its session is being ended",
        );

        state.sessions.end(conversation_id).await;
        return Reviewed::Asked;
    };

    // The session is over. Asking may have been its last act — the CLI is a
    // process of its own, and a Set can land as the session around it goes — so
    // the Timeline is asked once more before this is read as a review that raised
    // nothing.
    if asked_already(state, conversation_id).await {
        return Reviewed::Asked;
    }

    // Verkstead ended it, which here means the human aborted the Conversation out
    // from under the wrap-up. There is nothing to ask them about: they have just
    // answered.
    if ended.on_purpose() {
        tracing::info!(
            conversation_id,
            event_id,
            "the review was stopped from outside, so nothing is asked about it"
        );
        return Reviewed::Nothing;
    }

    match ended.badly() {
        Some(how) => Reviewed::Stopped {
            how,
            writing: event_id,
        },
        None => Reviewed::FoundNothing,
    }
}

/// Wait until the review's Set is on the Timeline.
///
/// The store rather than anything of the session's, for the reason a fix
/// session's commits are read there: the Set arrives through Verkstead itself,
/// and a Set carrying findings *is* the review's — see
/// [`store::review_asked`].
async fn asked(state: &AppState, conversation_id: i64, pace: Pace) {
    loop {
        tokio::time::sleep(pace.poll).await;

        if asked_already(state, conversation_id).await {
            return;
        }
    }
}

/// Whether it has. A store that will not answer reads as *not yet*, which is the
/// right way round for the one thing this decides: a session is ended on the
/// strength of it.
async fn asked_already(state: &AppState, conversation_id: i64) -> bool {
    match store::review_asked(&state.pool, conversation_id).await {
        Ok(asked) => asked.is_some(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether the review had asked failed");
            false
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
                return;
            }

            tokio::time::sleep(owed).await;
        }
    }
}

/// Whether an Interruption is already holding this run up.
///
/// A store that will not answer reads as *stopped*, which is the right way round
/// for the one thing this decides: what is on the other side of it is launching
/// an agent, and a runner that could not tell whether the human was still being
/// asked something should wait rather than spend an account guessing.
async fn stopped(state: &AppState, conversation_id: i64) -> bool {
    match store::open_interruption(&state.pool, conversation_id).await {
        Ok(Some(event_id)) => {
            tracing::info!(
                conversation_id,
                event_id,
                "the run is blocked on the human, so nothing was launched"
            );
            true
        }
        Ok(None) => false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a run was blocked failed");
            true
        }
    }
}

/// Stop the run: put what went wrong on the Timeline for the human to answer.
///
/// Nothing is refused for. By the time this runs the session is gone and the step
/// has not landed, and an Interruption that could not be raised is a run stopped
/// with nothing saying so — which is a thing to see in the log, and the same
/// thing either way: the runner returns.
async fn stop(
    state: &AppState,
    conversation_id: i64,
    step: store::Step,
    what: &str,
    how: &str,
    writing: i64,
) {
    if let Err(error) =
        crate::interruptions::raise(state, conversation_id, step, what, how, Some(writing)).await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a run stopped and the Interruption saying so could not be raised"
        );
    }
}

/// See one step's session out: end it once the step has landed and the session
/// has gone quiet, and say whether the step landed at all.
///
/// `None` is a session that is over with its step not done. That is a crash, a
/// hang given up on, or an agent that stopped short — which of them is not
/// something to guess at here, and none of them is a reason to launch the same
/// step again on its own. The run stops at an Interruption, and it is the human
/// who decides whether the step gets another run.
///
/// `Some` is the Timeline Event the session printed into. The step landed, and
/// what comes after it may still want the session's own last words — the finish
/// step's does, because an Interruption raised about what the finish left behind
/// is answered from them.
async fn see_out(
    state: &AppState,
    conversation_id: i64,
    step: Step,
    mut session: Session,
) -> Option<i64> {
    let event_id = session.event_id;

    let (Some(landing), Some(stored)) = (step.landing(), step.stored()) else {
        return None;
    };

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

    // Verkstead ended it, which here means the human aborted the Conversation
    // out from under the run: the worktree has gone, so the step reads as not
    // landed whatever it did. There is nothing to ask them about — they have
    // just answered.
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

    stop(state, conversation_id, stored, &step.what(), &how, event_id).await;

    None
}

/// Wait until `landing` has landed *and* the session has been quiet for the
/// grace period.
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
                return;
            }

            tokio::time::sleep(owed).await;
        }
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
    /// Verkstead's fork of to-tasks, which writes the backlog.
    BreakingDown,

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

    /// The addressing skill, carrying the feedback the fix session is for.
    ///
    /// The one prompt that has something in it, because it is the one session
    /// launched *about* something rather than about the work as a whole: the
    /// other three are told where the work is written down and read it for
    /// themselves.
    Addressing(String),

    /// The reviewing skill, which the one session a wrap-up starts with runs
    /// inside.
    Reviewing,
}

/// Start a fresh session on the next step, under the Conversation's
/// implementation Profile.
///
/// Which step it is is not said: the bundled fork reads `.tasks/` and picks the
/// same one this did, by the same rule. Verkstead decides the step to know what
/// to watch for, not to hand it over — a runner that named the file would be a
/// second opinion about a question the skill is already asking.
///
/// `note` is what the human wrote when they asked for a step to be tried again,
/// and is empty for every session a run launches of its own accord.
///
/// The Conversation is read back every time rather than held across the run: a
/// backlog takes hours, and where an agent is about to be let loose is the one
/// thing that must not be guessed at.
async fn launch(
    state: &AppState,
    conversation_id: i64,
    inside: Prompt,
    note: &str,
) -> Option<Session> {
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

    let Some(profile) = conversation.implementation_profile.clone() else {
        tracing::error!(
            conversation_id,
            "the implementation Profile is gone, so no session was started"
        );
        return None;
    };

    let prompt = match crate::conversations::documents(&state.pool, conversation_id).await {
        Ok((brief, handoff)) => {
            let handoff = handoff.as_deref();

            let prompt = match &inside {
                Prompt::BreakingDown => skills::breaking_down(&brief, handoff),
                Prompt::PlanningStage(stacked_on) => {
                    skills::next_stage(&brief, stacked_on.as_deref())
                }
                Prompt::Staging => skills::staging(&brief, handoff),
                Prompt::NextTask => skills::next_task(&brief, handoff),
                Prompt::Implementing => skills::implementing(&brief, handoff),
                Prompt::Addressing(feedback) => skills::addressing(&brief, handoff, feedback),
                Prompt::Reviewing => skills::reviewing(&brief, handoff),
            };

            skills::retrying(&prompt, note)
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what the work is failed");
            return None;
        }
    };

    // One Worktree holds one agent. Every session a run launches of its own
    // accord follows one this has already ended, but a retry follows one that
    // died — and a register still holding a relay that has not finished unwinding
    // would be two agents editing each other's files.
    state.sessions.end(conversation_id).await;

    match state
        .sessions
        .start(&state.pool, &state.nudges, &conversation, &profile, &prompt)
        .await
    {
        Ok(session) => session,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "a task session could not be started");
            None
        }
    }
}

/// Where the Conversation's work is being done, or `None` where there is nowhere
/// left to work — an aborted Conversation, or one that has gone.
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

/// What a stage Conversation's branch was made on top of.
///
/// Both layers mean something — see [`store::stacks_on`] — and a store that will
/// not answer reads as the outer `None`: what turns on it is a sentence in a
/// prompt, and a session told nothing about stacking will read the repository
/// rather than stack on a branch this guessed at.
async fn stacked_on(state: &AppState, conversation_id: i64) -> Option<Option<String>> {
    match store::stacks_on(&state.pool, conversation_id).await {
        Ok(stacked_on) => stacked_on,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a stage's branch stands on failed");
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
        assert_eq!(Step::Nothing.landing(), None);
    }
}
