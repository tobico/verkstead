//! Watching a wrapping Conversation's pull requests go green, and fixing them
//! when they do not.
//!
//! The finish step opened the pull requests and pushed the branches; GitHub is
//! now running whatever each repository runs. Nobody is watching them. So
//! Verkstead asks the host's `gh` how the checks are getting on, for as long as
//! the Conversation is Wrapping and no longer — see [`crate::github::checks`].
//!
//! **One watcher per pull request**, because a suite is a fact about a pull
//! request rather than about a Conversation: a Conversation ends on one per
//! repository it was worked in, each with its own checks running against its own
//! branch, and each asked about in its own repository — `#7` means something else
//! in another one, or nothing. [`watching`] starts one for every pull request on
//! the record, and [`crate::wrapping::covering`] starts one for each companion's
//! as it finds it.
//!
//! Three answers and three different things to do. Checks still running are
//! nothing to do at all. Checks that pass settle one of the things wrap-up is
//! waiting on. A check that fails dispatches a **fix session**: a fresh session
//! under the Conversation's implementation Profile, inside the bundled
//! addressing skill, given the failure as its feedback. It commits and pushes as
//! that skill says, with no gate in front of either, and the branch watcher puts
//! what it committed on the Timeline.
//!
//! **The fix session is told which repository, which pull request and where to
//! work.** A session starts in the Conversation's own worktree and `gh` reads
//! its repository from wherever it runs, so one sent at a companion's pull
//! request would otherwise ask the wrong repository how its checks were getting
//! on. Every companion's worktree is bound into the sandbox already, so the
//! directory the feedback names is one the session can simply work in — see
//! [`feedback`], and the bundled addressing skill, which is written for a
//! session that may be sent outside the worktree it starts in.
//!
//! **Two attempts per check per pull request, then it stops asking the machine
//! and starts asking the human.** A check that is still red after [`ATTEMPTS`]
//! fix sessions stops the run and nothing further is dispatched for it: the
//! human reads which pull request would not go green, which checks failed and
//! what the last session said, off the Notice. The count is per check rather
//! than per Conversation — a suite where one job fails and is fixed and then a
//! different one fails has not spent its attempts — and per pull request beside
//! it, the same check name red on two of them being two different failures. It
//! is kept in the store, so a restarted server does not start the counting
//! again.
//!
//! **Sessions still run one at a time.** Two red pull requests do not collide:
//! a fix session takes the Conversation's Turn, and the watcher that cannot get
//! it comes back to its pull request on the next poll rather than queueing
//! behind whatever is in there.
//!
//! **A check that goes red while the review holds the Worktree is the review's
//! to fix**, not a session of its own. The review session takes the Worktree
//! before it asks and keeps it until it ends — which is across the human's
//! answering — so a fix session dispatched at a red check would find nothing to
//! take and queue behind hours of waiting. It does not queue: the reviewing
//! skill sends the woken session to read the pull request's own check state once
//! the answers arrive and fix whatever is failing beside the findings they
//! accepted, before its push. That spends none of the check's [`ATTEMPTS`],
//! because nothing here dispatched it: the count is what stops an unattended
//! loop, and this fix rides work the human just approved. Whatever is still red
//! after that push is red in front of a free Worktree, and the flow below is
//! the flow it meets.
//!
//! A `gh` that cannot answer changes nothing at all — it does not settle, it
//! does not unsettle, and it dispatches nothing. That is the only honest reading
//! of it: Verkstead does not know how the checks are, and neither *green* nor
//! *red* is a thing to conclude from not knowing.

use std::time::Duration;

use crate::AppState;
use crate::github::{Check, Checked};
use crate::store;
use crate::wrapping::{Watched, named};

/// How many fix sessions one check gets before the human is asked instead.
///
/// Two, which is one automatic go and one more after it did not work. A third
/// would be a machine spending an account on the same failure with nobody
/// watching, which is the whole thing a stop exists instead of.
const ATTEMPTS: i64 = 2;

/// Watch every pull request `conversation_id` is on, one watcher each.
///
/// What a wrap-up starts with, and what a server coming back up and a Resume
/// start again — each of which starts the whole of a wrap-up rather than some of
/// it. The pull requests are read here rather than passed in, for the reason
/// every other watcher reads the record: what this is looking at is a wrap-up
/// with nothing running, whatever put it there.
///
/// One task each, so that a suite nobody can ask about holds up nothing else,
/// and awaited together, so that the whole of this counts as one driver of the
/// Conversation for as long as any of them is going — see
/// [`crate::wrapping::watching`].
///
/// A companion's pull request found after this started gets its own watcher
/// where it is recorded, that being the moment there is one to watch: see
/// [`crate::wrapping::covering`].
pub(crate) async fn watching(state: AppState, conversation_id: i64) {
    let opened = match store::pull_requests(&state.pool, conversation_id).await {
        Ok(opened) => opened,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which pull requests to watch failed");
            return;
        }
    };

    let watchers: Vec<_> = opened
        .into_iter()
        .map(|(repo, _)| tokio::spawn(watch(state.clone(), conversation_id, repo.id)))
        .collect();

    for watcher in watchers {
        if let Err(error) = watcher.await {
            tracing::error!(error = ?error, conversation_id, "a checks watcher ended badly");
        }
    }
}

/// Watch the checks on the pull request `conversation_id` opened in `repo_id`,
/// until it stops wrapping up.
///
/// Returns when there is nothing left to watch: the Conversation has moved on or
/// gone, that repository has no pull request on the record any more, or driving
/// stopped. Idle rather than looping, for the runner's reason — a watcher that
/// kept dispatching sessions at a check nothing was going to fix would be
/// spending an account on the same failure over and over.
///
/// Nothing here is refused for. This runs unattended with nobody watching, and
/// what it has to say it says on the Timeline or in the log.
pub(crate) async fn watch(state: AppState, conversation_id: i64, repo_id: i64) {
    // The Timeline Event the last fix session printed into, so that a stop
    // written here carries the tail of what it said — which is where the reason
    // it could not fix the check is usually written down.
    let mut writing = None;

    loop {
        match once(&state, conversation_id, repo_id, writing).await {
            Watching::Again(said) => writing = said,
            Watching::Done(why) => {
                tracing::info!(
                    conversation_id,
                    repo_id,
                    why,
                    "a pull request's checks are no longer being watched"
                );
                return;
            }
        }

        tokio::time::sleep(state.sessions.pace().checks).await;
    }
}

/// Forget what a Conversation's checks have already been tried, and watch them
/// again.
///
/// What Resume does to a Conversation that stopped on its checks. The attempts
/// go first: the human has read the Notice of what stopped and asked for another
/// go, and a count left standing would be a watcher that stopped all over again on
/// its next poll without dispatching anything.
pub(crate) async fn afresh(state: AppState, conversation_id: i64) {
    if let Err(error) = store::forget_fix_attempts(&state.pool, conversation_id).await {
        tracing::error!(error = ?error, conversation_id, "forgetting what a Conversation's checks had been given failed");
        return;
    }

    tracing::info!(
        conversation_id,
        "the checks are being tried again from no attempts spent"
    );

    // The whole wrap-up rather than the checks alone: the review stopped being
    // run when the stop was written, because nothing advances past one, and a
    // Conversation that came back with its checks watched and nobody reading its
    // branch would wait on a review that was never going to happen.
    //
    // And its review from the start, which is the same forgetting as the
    // attempts above: a review found already asking is what the press is an
    // answer to, so it is read past rather than stopped over a second time. See
    // [`crate::review::afresh`].
    crate::wrapping::watching(&state, conversation_id, crate::wrapping::Reviewing::Afresh);
}

/// What one look at the checks decided.
enum Watching {
    /// Look again after the interval, carrying whichever Timeline Event the last
    /// fix session printed into.
    Again(Option<i64>),

    /// Stop watching, for this reason.
    Done(&'static str),
}

/// Take one look: ask GitHub how the checks are, and do whatever that means.
async fn once(
    state: &AppState,
    conversation_id: i64,
    repo_id: i64,
    writing: Option<i64>,
) -> Watching {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return Watching::Done("there is no Conversation left to watch"),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to watch failed");
            return Watching::Again(writing);
        }
    };

    // The one thing that ends the watching by itself. Everything a Conversation
    // leaves Wrapping for — Done, or closed from the menu — arrives here as the
    // same fact: this is not a wrap-up any more.
    if conversation.state != store::Lifecycle::Wrapping {
        return Watching::Done("the Conversation is not wrapping up any more");
    }

    // Asked before anything is dispatched, for the runner's reason: *the run does
    // not advance past a stop* means no session is launched while the human is
    // the only thing that can start one. However the run stopped — the stop
    // written here included, and an account out of window included — it is the
    // one question [`crate::stopping::stopped`] answers.
    if crate::stopping::stopped(state, conversation_id).await {
        return Watching::Done("driving has stopped");
    }

    let opened = match store::pull_request(&state.pool, conversation_id, repo_id).await {
        Ok(Some(opened)) => opened,
        // A Conversation wrapping up has a pull request in the repository whose
        // watcher this is — a watcher is started where one is recorded and never
        // before — so this is a record that has been got at rather than a wrap-up
        // to carry on with.
        Ok(None) => return Watching::Done("that repository has no pull request to watch"),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, repo_id, "reading the pull request to watch failed");
            return Watching::Again(writing);
        }
    };

    // Which repository to ask in and which checkout its work is done in, read off
    // the Conversation every poll rather than held: a companion taken away is one
    // there is nowhere left to ask about.
    let Some(watched) = crate::wrapping::watched(&conversation, repo_id, opened.number) else {
        return Watching::Done("there is no repository left to ask about that pull request in");
    };

    let asked = {
        let gh = state.github.clone();
        let repo = watched.repo.path.clone();
        let number = watched.number;

        // Off the runtime's threads: this is a process, and one that goes to the
        // network.
        tokio::task::spawn_blocking(move || crate::github::checks(&gh, &repo, number)).await
    };

    let checks = match asked {
        Ok(Ok(checks)) => checks,
        // GitHub could not be asked. Nothing is concluded from that and nothing is
        // touched — not the settlement either way, and certainly not a fix
        // session: *Verkstead does not know* is a third thing beside green and
        // red, and each of the two ways of guessing at it is wrong on its own
        // terms. The next poll asks again, of a `gh` that may by then have been
        // logged in.
        Ok(Err(trouble)) => {
            tracing::warn!(
                conversation_id,
                repo = watched.repo.name,
                number = watched.number,
                why = trouble.why(),
                "the checks could not be asked about, so the wrap-up goes on waiting",
            );

            return Watching::Again(writing);
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking gh about the checks failed");
            return Watching::Again(writing);
        }
    };

    let failed: Vec<Check> = checks
        .iter()
        .filter(|check| check.how == Checked::Failed)
        .cloned()
        .collect();

    let running = checks.iter().any(|check| check.how == Checked::Running);

    // Green, which includes a pull request with no checks on it at all: a
    // repository with no CI is nothing for a wrap-up to wait on, and waiting for
    // a check that is never coming would be a Conversation that never finished.
    if failed.is_empty() && !running {
        settle(state, conversation_id, &watched, checks.len()).await;
        return Watching::Again(writing);
    }

    // Not green, whether that is a red check or a suite that has not finished.
    // Said before anything is dispatched, because a fix session pushes a commit
    // and a commit is a new run to wait on.
    unsettle(state, conversation_id, &watched).await;

    if failed.is_empty() {
        tracing::debug!(
            conversation_id,
            repo = watched.repo.name,
            number = watched.number,
            "the checks are still running, which is nothing to do",
        );
        return Watching::Again(writing);
    }

    fix(state, conversation_id, &watched, &failed, writing).await
}

/// Dispatch a fix session for the failed checks that have attempts left, or ask
/// the human where none has.
async fn fix(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
    failed: &[Check],
    writing: Option<i64>,
) -> Watching {
    let mut fixable = Vec::new();

    for check in failed {
        match store::fix_attempts(&state.pool, conversation_id, watched.repo.id, &check.name).await
        {
            Ok(spent) if spent < ATTEMPTS => fixable.push(check),
            Ok(_) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, check = check.name, "reading what a check had been given failed");
                return Watching::Again(writing);
            }
        }
    }

    // Every failed check has had its two goes, so the machine has nothing left to
    // try and the human is asked instead.
    if fixable.is_empty() {
        return ask(state, conversation_id, watched, failed, writing).await;
    }

    // One agent in one Worktree. Tried rather than waited for, and taken before
    // an attempt is counted: what else is in there is the review session or a
    // finding the human accepted, both of which take as long as they take, and a
    // fix session queued behind one would be dispatched about a suite nobody has
    // looked at since. Looking again in half a minute costs nothing and asks
    // GitHub afresh.
    //
    // Which is also why nothing is counted for a check the review ends up fixing
    // itself: an attempt is spent where a session is dispatched, and none is
    // here. The turn being taken is the whole of what folds this check into the
    // woken session, and the two attempts are still there for it afterwards.
    // Which is also what keeps two red pull requests from colliding: the second
    // watcher finds the Turn taken, and comes back to its own suite on the next
    // poll rather than queueing behind a session about somebody else's.
    let Some(_turn) = state.sessions.try_turn(conversation_id) else {
        tracing::debug!(
            conversation_id,
            repo = watched.repo.name,
            "something else is working in the Worktree, so the checks are looked at again later",
        );
        return Watching::Again(writing);
    };

    // Counted as the session is dispatched rather than as it ends, so that an
    // attempt spent by a server that then restarted is one the next server does
    // not spend again.
    for check in &fixable {
        if let Err(error) =
            store::record_fix_attempt(&state.pool, conversation_id, watched.repo.id, &check.name)
                .await
        {
            tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, check = check.name, "counting a fix session failed");
            return Watching::Again(writing);
        }
    }

    tracing::info!(
        conversation_id,
        repo = watched.repo.name,
        number = watched.number,
        checks = ?fixable.iter().map(|check| &check.name).collect::<Vec<_>>(),
        "a check failed, so a fix session is starting on it",
    );

    // One session for however many checks are red, rather than one each. They are
    // one run of one suite over one branch, and two agents in one Worktree would
    // be two agents editing each other's files.
    let said = crate::runner::address(state, conversation_id, &feedback(watched, &fixable)).await;

    Watching::Again(said.or(writing))
}

/// Stop asking the machine: stop the run, and put what failed on the Timeline.
///
/// The evidence is what makes the stop readable without opening a terminal —
/// which pull request would not go green, which of its checks failed, where
/// their runs are, and the tail of what the last fix session said, which
/// [`crate::stopping::stop`] reads off `writing`.
///
/// The pull request is named in the step as well as in the reason, a Conversation
/// having more than one: what stopped is *this* suite rather than the checks in
/// general, and the Notice's own first line is where that is read.
///
/// [`store::Decision::Deliberate`]: every fix session the branch was allowed has
/// been spent, and Verkstead stopping there is a decision. A restart that
/// started the fixing over would spend them all again on checks that are still
/// red for whatever reason they were red the first time.
async fn ask(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
    failed: &[Check],
    writing: Option<i64>,
) -> Watching {
    let how = format!(
        "{} on {} after {ATTEMPTS} fix sessions each:\n\n{}",
        match failed.len() {
            1 => "a check is still failing".to_owned(),
            many => format!("{many} checks are still failing"),
        },
        named(watched),
        listed(failed),
    );

    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        &format!("getting the checks green on {}", named(watched)),
        &how,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "the checks would not go green and the stop saying so could not be recorded"
        );
    }

    Watching::Done("the checks would not go green, so the human is being asked")
}

/// The failed checks as the fix session is told about them.
///
/// The names and the links and nothing else about what they said. What a check
/// actually complained about is in its run rather than in anything `gh pr view`
/// will hand over, and the addressing skill sends the session to run the failing
/// thing itself — a fix written from a summary of a log is a guess.
///
/// What it *is* told beside them is where to work. A session starts in the
/// Conversation's own worktree and both `git` and `gh` read their repository from
/// wherever they run, so one sent at a companion's pull request would otherwise
/// ask the wrong repository how its checks were getting on. Every worktree is
/// bound into the sandbox at the path named here, so it is a directory the
/// session can simply work in.
///
/// Named the same way whichever repository it is, the Conversation's own
/// included: a session is told where it is working rather than left to infer
/// that it has not been sent anywhere.
fn feedback(watched: &Watched, failed: &[&Check]) -> String {
    let failed: Vec<Check> = failed.iter().map(|check| (*check).clone()).collect();

    format!(
        "These checks are failing on {}. Work in that repository's worktree, at `{}` — both \
         `git` and `gh` read the repository from wherever they are run, so a check asked about \
         anywhere else is a different repository's.\n\nFind out what each of them is actually \
         complaining about, fix the cause, and push the fix so they run again.\n\n{}",
        named(watched),
        watched.worktree.display(),
        listed(&failed),
    )
}

/// One check per line: what it is called, and where its run is.
fn listed(checks: &[Check]) -> String {
    checks
        .iter()
        .map(|check| match check.link.is_empty() {
            true => format!("- {}", check.name),
            false => format!("- {} — {}", check.name, check.link),
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Record that this pull request's checks are green, so wrap-up has one less
/// thing to wait on.
///
/// One of however many it is waiting on: a Conversation ends on a pull request
/// per repository it was worked in, and every one of them has to be green before
/// the wrap-up is over — see [`store::finish_wrap_up`].
async fn settle(state: &AppState, conversation_id: i64, watched: &Watched, checks: usize) {
    if let Err(error) = store::settle_wrap_up(
        &state.pool,
        conversation_id,
        store::WaitingOn::Checks(watched.repo.id),
    )
    .await
    {
        tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "recording that the checks are green failed");
        return;
    }

    tracing::debug!(
        conversation_id,
        repo = watched.repo.name,
        number = watched.number,
        checks,
        "the checks are green",
    );
}

/// And that they are not, which is a red suite, one still running, and a `gh`
/// that could not say.
async fn unsettle(state: &AppState, conversation_id: i64, watched: &Watched) {
    if let Err(error) = store::unsettle_wrap_up(
        &state.pool,
        conversation_id,
        store::WaitingOn::Checks(watched.repo.id),
    )
    .await
    {
        tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "putting the checks back to waiting failed");
    }
}

/// How often the checks are asked about.
///
/// A CI run takes minutes, so this is not a race to notice one finishing: it is
/// how long a green suite waits to settle and how long a red one waits for its
/// fix session. Thirty seconds costs two `gh` calls a minute per wrapping
/// Conversation, and there are rarely more than a handful.
pub(crate) const ASKED_EVERY: Duration = Duration::from_secs(30);

#[cfg(test)]
mod tests {
    use super::*;

    fn check(name: &str, link: &str) -> Check {
        Check {
            name: name.to_owned(),
            how: Checked::Failed,
            link: link.to_owned(),
        }
    }

    /// A companion's pull request, which is the one a fix session has to be sent
    /// somewhere for.
    fn watched() -> Watched {
        Watched {
            repo: store::Repo {
                id: 2,
                path: std::path::PathBuf::from("/watched/askance"),
                name: "askance".to_owned(),
                default_branch: "main".to_owned(),
            },
            number: 7,
            worktree: std::path::PathBuf::from("/state/worktrees/rate-limiting-askance"),
        }
    }

    /// What a fix session is told: which checks are red and where their runs
    /// are, so it can go and read the real failure — and which pull request in
    /// which repository they are red on, which is what says where to read it.
    #[test]
    fn a_fix_session_is_told_which_checks_failed_and_where_they_are() {
        let rust = check(
            "Rust",
            "https://github.com/tobico/verkstead/actions/runs/1/job/2",
        );
        let told = feedback(&watched(), &[&rust]);

        assert!(
            told.contains("Rust") && told.contains("/actions/runs/1/job/2"),
            "the name and the run: {told}",
        );
        assert!(
            told.contains("push"),
            "and that the fix has to reach the pull request: {told}",
        );
        assert!(
            told.contains("#7") && told.contains("askance"),
            "which pull request, in which repository: {told}",
        );
        assert!(
            told.contains("/state/worktrees/rate-limiting-askance"),
            "and the worktree to work in, `gh` reading its repository from wherever \
             it is run: {told}",
        );
    }

    /// A check GitHub gave no link for is still a check to fix, and the line
    /// says its name rather than trailing an empty dash.
    #[test]
    fn a_check_with_no_run_to_link_to_is_listed_by_name_alone() {
        assert_eq!(listed(&[check("Rust", "")]), "- Rust");
    }
}
