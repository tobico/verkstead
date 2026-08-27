//! Watching a wrapping Conversation's pull request go green, and fixing it when
//! it does not.
//!
//! The finish step opened the pull request and pushed the branch; GitHub is now
//! running whatever the repository runs. Nobody is watching it. So Verkstead
//! asks the host's `gh` how the checks are getting on, for as long as the
//! Conversation is Wrapping and no longer — see [`crate::github::checks`].
//!
//! Three answers and three different things to do. Checks still running are
//! nothing to do at all. Checks that pass settle one of the things wrap-up is
//! waiting on. A check that fails dispatches a **fix session**: a fresh session
//! under the Conversation's implementation Profile, inside the bundled
//! addressing skill, given the failure as its feedback. It commits and pushes as
//! that skill says, with no gate in front of either, and the branch watcher puts
//! what it committed on the Timeline.
//!
//! **Two attempts, then it stops asking the machine and starts asking the
//! human.** A check that is still red after [`ATTEMPTS`] fix sessions stops the
//! run and nothing further is dispatched for it: the human reads which checks
//! failed and what the last session said, off the Notice. The count
//! is per check rather than per Conversation — a suite where one job fails and
//! is fixed and then a different one fails has not spent its attempts — and it
//! is kept in the store, so a restarted server does not start the counting
//! again.
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

/// How many fix sessions one check gets before the human is asked instead.
///
/// Two, which is one automatic go and one more after it did not work. A third
/// would be a machine spending an account on the same failure with nobody
/// watching, which is the whole thing a stop exists instead of.
const ATTEMPTS: i64 = 2;

/// Watch `conversation_id`'s checks until it stops wrapping up.
///
/// Returns when there is nothing left to watch: the Conversation has moved on or
/// gone, or driving that stopped. Idle rather than looping, for the
/// runner's reason — a watcher that kept dispatching sessions at a check nothing
/// was going to fix would be spending an account on the same failure over and
/// over.
///
/// Nothing here is refused for. This runs unattended with nobody watching, and
/// what it has to say it says on the Timeline or in the log.
pub(crate) async fn watch(state: AppState, conversation_id: i64) {
    // The Timeline Event the last fix session printed into, so that a stop
    // written here carries the tail of what it said — which is where the reason
    // it could not fix the check is usually written down.
    let mut writing = None;

    loop {
        match once(&state, conversation_id, writing).await {
            Watching::Again(said) => writing = said,
            Watching::Done(why) => {
                tracing::info!(
                    conversation_id,
                    why,
                    "the checks are no longer being watched"
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
async fn once(state: &AppState, conversation_id: i64, writing: Option<i64>) -> Watching {
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

    let opened = match store::pull_request(&state.pool, conversation_id, conversation.repo.id).await
    {
        Ok(Some(opened)) => opened,
        // A Conversation wrapping up has a pull request — recording one *is* the
        // move — so this is a record that has been got at rather than a wrap-up
        // to carry on with.
        Ok(None) => return Watching::Done("the Conversation has no pull request to watch"),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the pull request to watch failed");
            return Watching::Again(writing);
        }
    };

    let asked = {
        let gh = state.github.clone();
        let repo = conversation.repo.path.clone();
        let number = opened.number;

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
                number = opened.number,
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
        settle(state, conversation_id, checks.len()).await;
        return Watching::Again(writing);
    }

    // Not green, whether that is a red check or a suite that has not finished.
    // Said before anything is dispatched, because a fix session pushes a commit
    // and a commit is a new run to wait on.
    unsettle(state, conversation_id).await;

    if failed.is_empty() {
        tracing::debug!(
            conversation_id,
            number = opened.number,
            "the checks are still running, which is nothing to do",
        );
        return Watching::Again(writing);
    }

    fix(state, conversation_id, &failed, writing).await
}

/// Dispatch a fix session for the failed checks that have attempts left, or ask
/// the human where none has.
async fn fix(
    state: &AppState,
    conversation_id: i64,
    failed: &[Check],
    writing: Option<i64>,
) -> Watching {
    let mut fixable = Vec::new();

    for check in failed {
        match store::fix_attempts(&state.pool, conversation_id, &check.name).await {
            Ok(spent) if spent < ATTEMPTS => fixable.push(check),
            Ok(_) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, check = check.name, "reading what a check had been given failed");
                return Watching::Again(writing);
            }
        }
    }

    // Every failed check has had its two goes, so the machine has nothing left to
    // try and the human is asked instead.
    if fixable.is_empty() {
        return ask(state, conversation_id, failed, writing).await;
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
    let Some(_turn) = state.sessions.try_turn(conversation_id) else {
        tracing::debug!(
            conversation_id,
            "something else is working in the Worktree, so the checks are looked at again later",
        );
        return Watching::Again(writing);
    };

    // Counted as the session is dispatched rather than as it ends, so that an
    // attempt spent by a server that then restarted is one the next server does
    // not spend again.
    for check in &fixable {
        if let Err(error) =
            store::record_fix_attempt(&state.pool, conversation_id, &check.name).await
        {
            tracing::error!(error = ?error, conversation_id, check = check.name, "counting a fix session failed");
            return Watching::Again(writing);
        }
    }

    tracing::info!(
        conversation_id,
        checks = ?fixable.iter().map(|check| &check.name).collect::<Vec<_>>(),
        "a check failed, so a fix session is starting on it",
    );

    // One session for however many checks are red, rather than one each. They are
    // one run of one suite over one branch, and two agents in one Worktree would
    // be two agents editing each other's files.
    let said = crate::runner::address(state, conversation_id, &feedback(&fixable)).await;

    Watching::Again(said.or(writing))
}

/// Stop asking the machine: stop the run, and put what failed on the Timeline.
///
/// The evidence is what makes the stop readable without opening a terminal —
/// which checks failed, where their runs are, and the tail of what the last fix
/// session said, which [`crate::stopping::stop`] reads off `writing`.
///
/// [`store::Decision::Deliberate`]: every fix session the branch was allowed has
/// been spent, and Verkstead stopping there is a decision. A restart that
/// started the fixing over would spend them all again on checks that are still
/// red for whatever reason they were red the first time.
async fn ask(
    state: &AppState,
    conversation_id: i64,
    failed: &[Check],
    writing: Option<i64>,
) -> Watching {
    let how = format!(
        "{} after {ATTEMPTS} fix sessions each:\n\n{}",
        match failed.len() {
            1 => "a check is still failing".to_owned(),
            many => format!("{many} checks are still failing"),
        },
        listed(failed),
    );

    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "getting the pull request's checks green again",
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
/// The names and the links and nothing else. What the check actually said is in
/// its run rather than in anything `gh pr view` will hand over, and the
/// addressing skill sends the session to run the failing thing itself — a fix
/// written from a summary of a log is a guess.
fn feedback(failed: &[&Check]) -> String {
    let failed: Vec<Check> = failed.iter().map(|check| (*check).clone()).collect();

    format!(
        "These checks are failing on the pull request this branch is on. Find out what each of \
         them is actually complaining about, fix the cause, and push the fix so they run \
         again.\n\n{}",
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

/// Record that the checks are green, so wrap-up has one less thing to wait on.
async fn settle(state: &AppState, conversation_id: i64, checks: usize) {
    if let Err(error) =
        store::settle_wrap_up(&state.pool, conversation_id, store::WaitingOn::Checks).await
    {
        tracing::error!(error = ?error, conversation_id, "recording that the checks are green failed");
        return;
    }

    tracing::debug!(conversation_id, checks, "the checks are green");
}

/// And that they are not, which is a red suite, one still running, and a `gh`
/// that could not say.
async fn unsettle(state: &AppState, conversation_id: i64) {
    if let Err(error) =
        store::unsettle_wrap_up(&state.pool, conversation_id, store::WaitingOn::Checks).await
    {
        tracing::error!(error = ?error, conversation_id, "putting the checks back to waiting failed");
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

    /// What a fix session is told: which checks are red and where their runs
    /// are, so it can go and read the real failure rather than a summary.
    #[test]
    fn a_fix_session_is_told_which_checks_failed_and_where_they_are() {
        let rust = check(
            "Rust",
            "https://github.com/tobico/verkstead/actions/runs/1/job/2",
        );
        let told = feedback(&[&rust]);

        assert!(
            told.contains("Rust") && told.contains("/actions/runs/1/job/2"),
            "the name and the run: {told}",
        );
        assert!(
            told.contains("push"),
            "and that the fix has to reach the pull request: {told}",
        );
    }

    /// A check GitHub gave no link for is still a check to fix, and the line
    /// says its name rather than trailing an empty dash.
    #[test]
    fn a_check_with_no_run_to_link_to_is_listed_by_name_alone() {
        assert_eq!(listed(&[check("Rust", "")]), "- Rust");
    }
}
