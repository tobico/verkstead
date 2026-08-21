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
//! Nothing here handles a session that crashes or hangs: a step whose session
//! ends without landing it stops the run where it is, and what becomes of that
//! is the Interruption stage's.

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
}

impl Default for Pace {
    fn default() -> Pace {
        Pace {
            poll: Duration::from_secs(2),
            grace: Duration::from_secs(5),
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

    /// Work this task file, the lowest-numbered one left.
    Task(PathBuf),

    /// Finish the feature: every task is done and only `TODO.md` is left.
    Finish,

    /// There is no backlog. Nothing to run, and nothing to poll for.
    Nothing,
}

/// What says a step is over: a path in the Worktree that has to have gone, or
/// one that has to have arrived — and, either way, be committed as it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Landing {
    Gone(PathBuf),
    Arrived(PathBuf),
}

impl Step {
    /// What would say this step is over, or `None` where it is not a step to
    /// run.
    fn landing(&self) -> Option<Landing> {
        match self {
            // The plan commit is what puts the backlog under version control,
            // so the backlog being there and committed is the breakdown done.
            Step::Planning => Some(Landing::Arrived(todo())),
            // Finishing a task is what deletes its file.
            Step::Task(file) => Some(Landing::Gone(file.clone())),
            // And the finish commit removes `TODO.md` with the rest of `.tasks/`.
            Step::Finish => Some(Landing::Gone(todo())),
            Step::Nothing => None,
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
/// over and over, with nobody watching.
pub(crate) async fn follow(state: AppState, conversation_id: i64, planning: Session) {
    if !see_out(&state, conversation_id, Step::Planning, planning).await {
        return;
    }

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

        let Some(session) = launch(&state, conversation_id).await else {
            return;
        };

        if !see_out(&state, conversation_id, step, session).await {
            return;
        }
    }
}

/// See one step's session out: end it once the step has landed and the session
/// has gone quiet, and say whether the step landed at all.
///
/// `false` is a session that is over with its step not done. That is a crash, a
/// hang given up on, or an agent that stopped short — which of them is not
/// something to guess at here, and none of them is a reason to launch the same
/// step again. The run stops, and what the human is shown is the Interruption
/// stage's to draw.
async fn see_out(state: &AppState, conversation_id: i64, step: Step, mut session: Session) -> bool {
    let Some(landing) = step.landing() else {
        return false;
    };

    let Some(worktree) = worktree(state, conversation_id).await else {
        return false;
    };

    // Taken before the session is waited on: the two are asked about together
    // below, and the clock is shared with the relay rather than owned by the
    // handle.
    let quiet = session.quiet.clone();
    let pace = state.sessions.pace();

    let landed = tokio::select! {
        _ = session.ended() => false,
        _ = landed_and_quiet(&worktree, &landing, &quiet, pace) => true,
    };

    if landed {
        tracing::info!(
            conversation_id,
            event_id = session.event_id,
            step = ?step,
            "a step has landed and its session has gone quiet, so it is being ended",
        );

        state.sessions.end(conversation_id).await;
        return true;
    }

    // The session is over. It may have landed its step as its last act and
    // exited before a poll caught it, which is the ordinary shape of a session
    // that finishes rather than idles — so the Worktree is asked once more
    // before this is read as a run that has stopped.
    if check(&worktree, &landing).await {
        return true;
    }

    tracing::warn!(
        conversation_id,
        event_id = session.event_id,
        step = ?step,
        "a session ended without finishing its step, so the backlog stops here",
    );

    false
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
async fn launch(state: &AppState, conversation_id: i64) -> Option<Session> {
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
        Ok((brief, handoff)) => skills::next_task(&brief, handoff.as_deref()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what the work is failed");
            return None;
        }
    };

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
        assert_eq!(Step::Nothing.landing(), None);
    }
}
