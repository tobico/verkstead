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
//! **And the stop waits for the rest of them.** A stop is the Conversation's
//! rather than one pull request's — nothing is dispatched past one — so a
//! watcher that wrote one the moment its own pull request ran out would spend
//! the other's goes for it, which is the thing counting per pull request exists
//! to stop. Whichever of them got the Turn first would be the one that got its
//! two. So a pull request out of goes waits, and the human is asked once every
//! one of them has gone green or run out as well — see [`owed_elsewhere`].
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
//! **The same poll reads whether the pull request merges at all.** GitHub says
//! it in the answer the rollup comes back in, so it costs no second call — and
//! a branch its base has moved under conflicts however green its suite is. Three
//! words again, and three different things: *MERGEABLE* settles one of the
//! things wrap-up waits on, *CONFLICTING* puts it back to waiting, and *UNKNOWN*
//! — GitHub still working it out, which is most of the first moments after a
//! push — does neither. So a conflicted pull request holds its Conversation in
//! Wrapping, which is also what closes the race where a conflict appears just as
//! the last suite goes green. See [`merging`].
//!
//! **And a conflict gets a session of its own**, the check fixes' shape end to
//! end: the same dispatch under the same Pairing, the Conversation's Turn taken
//! before anything is counted, [`ATTEMPTS`] goes counted in the store as each
//! session starts, and the same waiting for every other pull request before a
//! stop is written. What it is told is to merge the base branch in, resolve the
//! conflicts, run the tests and push — a merge rather than a rebase, so that
//! nothing is force-pushed and nothing stacked on the branch breaks. See
//! [`resolve`] and [`resolving`].
//!
//! A conflict is the whole of what such a poll *dispatches*. A branch nothing
//! can land is not a branch worth getting a check green on, and the resolution's
//! own push is what puts the suite in front of the next poll anyway. The suite
//! is still read and still settled on either way, though — the two are different
//! facts about the same branch, and a green one left unsettled through a
//! conflict would be one another pull request's stop went on waiting for.
//!
//! A `gh` that cannot answer changes nothing at all — it does not settle, it
//! does not unsettle, and it dispatches nothing. That is the only honest reading
//! of it: Verkstead does not know how the checks are, and neither *green* nor
//! *red* is a thing to conclude from not knowing. Nor whether it merges: that is
//! the same *UNKNOWN* by another route.
//!
//! **And a green suite is only ever green about one commit.** GitHub answers a
//! pull request as its own record stands, and that record runs behind the branch
//! for a while after a push: a rollup read in that window is the suite of the
//! commit before it, reported green, and a wrap-up that took it for the branch's
//! would reach Done over work nothing has ever checked. So the one answer that
//! ends something is the one that has to earn it. Green settles the wrap-up only
//! where the head GitHub named beside it is what origin is holding, and only
//! where a pull request that has reported a suite before is still reporting one —
//! GitHub takes a commit a moment before it creates the runs for it, and in
//! between it names the new commit and reports nothing against it, which is the
//! same answer a repository with no CI gives. Both are readings of the same fact:
//! *the run for what was pushed has not been reported yet* is not *there is
//! nothing to wait for*. Neither holds anything up where it cannot be told —
//! a `gh` that answered without a head and a checkout with no origin to ask are
//! the third thing again, and the rollup stands on its own.

use std::path::Path;
use std::time::Duration;

use verkstead_schema::Nudge;

use crate::AppState;
use crate::github::{Check, Checked, Mergeable};
use crate::repos::git;
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

    // Whether this pull request has ever reported a check, which is what tells a
    // repository with no CI from a run that has not been created yet — see
    // [`once`].
    let mut reported = false;

    loop {
        match once(&state, conversation_id, repo_id, writing, &mut reported).await {
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
    reported: &mut bool,
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

    let suite = match asked {
        Ok(Ok(suite)) => suite,
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

    // Written down before anything is decided about it, because the card draws
    // it and the card outlives the watching: this is the one place anything asks
    // GitHub how the checks are while a wrap-up is running, and what it learned
    // would otherwise go no further than the settle below.
    remember(state, conversation_id, &suite.checks).await;

    // And whether the pull request merges at all, which came back in the same
    // answer. Read before the checks rather than after, because it is a fact
    // about the branch rather than about the suite: every way the reading of the
    // checks below ends — green, red, still running, a rollup about the wrong
    // commit — is one this has to have been said on.
    let merges = merging(state, conversation_id, &watched, suite.mergeable).await;

    // The suite read and settled on, and nothing dispatched about it yet.
    // Settled whatever the merge said, because the two are different facts about
    // the same branch: a green suite left unsettled through a conflict is one
    // another pull request's stop would go on waiting for long after this
    // pull request had run out of anywhere to go — see [`owed_elsewhere`].
    let checked = checking(state, conversation_id, &watched, &suite, reported).await;

    // And a conflict is the whole of what this look dispatches, whatever the
    // suite turned out to be. A branch nothing can land is not one worth getting
    // a check green on — that fix would be work nobody could use — and the
    // resolution's own push is what puts the suite in front of the next poll
    // anyway.
    if merges == Some(store::Merging::Conflicting) {
        return resolve(state, conversation_id, &watched, writing).await;
    }

    match checked {
        Checking::NothingToDo => Watching::Again(writing),
        Checking::Failed(failed) => fix(state, conversation_id, &watched, &failed, writing).await,
    }
}

/// What one reading of the checks came to, once the wrap-up has been settled or
/// unsettled on it.
///
/// Reading the suite and acting on it are two steps rather than one so that a
/// conflicted pull request gets the first without the second: its checks are
/// read and written down like any other pull request's, and what is dispatched
/// at it is a resolution rather than a fix.
enum Checking {
    /// Nothing to dispatch about: green, still running, or a rollup about a
    /// commit that is not what origin is holding.
    NothingToDo,

    /// These checks are red.
    Failed(Vec<Check>),
}

/// Read how the suite is getting on and settle the wrap-up on it, without
/// dispatching anything.
///
/// `reported` is whether this pull request has ever reported a check, which the
/// watcher remembers across polls and this updates — see the green branch below,
/// which is the one place it changes an answer.
async fn checking(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
    suite: &crate::github::Suite,
    reported: &mut bool,
) -> Checking {
    // Whether anything has ever run against this pull request, which is the one
    // thing that tells a repository with no CI from a run that has not been
    // created yet — see the green branch below.
    *reported |= !suite.checks.is_empty();

    let failed: Vec<Check> = suite
        .checks
        .iter()
        .filter(|check| check.how == Checked::Failed)
        .cloned()
        .collect();

    let running = suite
        .checks
        .iter()
        .any(|check| check.how == Checked::Running);

    // Green, which includes a pull request with no checks on it at all: a
    // repository with no CI is nothing for a wrap-up to wait on, and waiting for
    // a check that is never coming would be a Conversation that never finished.
    //
    // Which is the one answer here that ends anything, so it is the one that is
    // held to what the two below ask. A red suite and one still running each
    // leave the wrap-up where it was, and a poll that read either of them of the
    // wrong commit costs a look rather than a Conversation.
    if failed.is_empty() && !running {
        // A pull request that has reported a check does not go back to reporting
        // none. GitHub creates the runs for a commit a moment after it takes the
        // commit itself, so *nothing is running against this* is also what the
        // gap between the two looks like — and it is the one gap the head below
        // cannot catch, both sides naming the same commit throughout it. What
        // tells them apart is whether anything has ever run here: a repository
        // with no CI has reported nothing from the first poll to the last, and
        // this one reported a suite a moment ago.
        //
        // Remembered for as long as this watcher runs rather than written down.
        // A server that came back up mid-wrap-up reads the gap as no CI again,
        // which is the trade the rest of a wrap-up makes about a restart too:
        // the record says what was settled rather than what was seen on the way.
        if suite.checks.is_empty() && *reported {
            tracing::debug!(
                conversation_id,
                repo = watched.repo.name,
                number = watched.number,
                "the checks have gone from a pull request that had them, so the \
                 run for the last push has not been created yet",
            );

            unsettle(state, conversation_id, watched).await;
            return Checking::NothingToDo;
        }

        // And a rollup is a fact about one commit, which is not always the one
        // that was pushed. GitHub answers this pull request as its own record
        // stands, and that record runs behind the branch for a while after a push
        // — long enough for a green suite belonging to the commit before the last
        // one to carry a wrap-up to Done over work nothing has ever checked. So
        // the head it named is held against what origin is holding, and a rollup
        // about anything else is not this branch's suite at all.
        //
        // Asked here rather than every poll, because this is the only poll whose
        // answer it changes — and it goes to the network, which the rest of a
        // look does not.
        //
        // Both halves have to be known for the question to mean anything: a `gh`
        // that answered without a head, and a checkout with no origin to ask, are
        // each *Verkstead cannot tell*. Neither is a reason to distrust a rollup
        // that is very probably the right one, so neither holds a wrap-up up.
        let pushed = {
            let worktree = watched.worktree.clone();

            // Off the runtime's threads: this is a process, and one that goes
            // to the network.
            tokio::task::spawn_blocking(move || pushed_head(&worktree))
                .await
                .unwrap_or_default()
        };

        if let (false, Some(pushed)) = (suite.head.is_empty(), pushed.as_deref()) {
            if pushed != suite.head {
                tracing::debug!(
                    conversation_id,
                    repo = watched.repo.name,
                    number = watched.number,
                    reported = suite.head,
                    pushed,
                    "the checks GitHub reported are about a commit that is not \
                     what origin is holding, so the run for what was pushed has \
                     not been reported yet",
                );

                unsettle(state, conversation_id, watched).await;
                return Checking::NothingToDo;
            }
        }

        settle(state, conversation_id, watched, suite.checks.len()).await;
        return Checking::NothingToDo;
    }

    // Not green, whether that is a red check or a suite that has not finished.
    // Said before anything is dispatched, because a fix session pushes a commit
    // and a commit is a new run to wait on.
    unsettle(state, conversation_id, watched).await;

    if failed.is_empty() {
        tracing::debug!(
            conversation_id,
            repo = watched.repo.name,
            number = watched.number,
            "the checks are still running, which is nothing to do",
        );
        return Checking::NothingToDo;
    }

    Checking::Failed(failed)
}

/// What origin is holding `worktree`'s branch on, asked as part of the poll.
///
/// Origin rather than the checkout's own HEAD, because those are different
/// commits whenever a session has committed and not yet pushed — and the question
/// a rollup has to be held against is which commit GitHub was given, not which
/// one the Worktree has got to. A wrap-up that waited on checks for an unpushed
/// commit would wait for a run nobody could ever have started.
///
/// Asked of the remote rather than read off a remote-tracking ref, because a ref
/// is only ever as fresh as the last fetch and fetching is not a free way to
/// freshen one: it writes refs and pulls objects into a repository an agent is
/// working in right now, which is what every git read here passes
/// `--no-optional-locks` to stay out of the way of. This wants one commit id, so
/// it asks for one commit id.
///
/// `None` where there is no origin to ask, where the remote could not be reached,
/// or where the branch is not on it — each of them *Verkstead cannot tell*, which
/// the caller reads as nothing to hold the rollup against rather than as a reason
/// to distrust it. A checkout with no remote is every one of this suite's own,
/// and a branch origin has never heard of is one nothing has pushed.
fn pushed_head(worktree: &Path) -> Option<String> {
    let branch = git(worktree, &["symbolic-ref", "--short", "HEAD"])?;
    let branch = branch.trim();

    // One line of `<commit>\trefs/heads/<branch>`, or nothing at all where origin
    // is not holding that branch — which `ls-remote` reports by saying nothing
    // rather than by failing.
    let said = git(
        worktree,
        &["ls-remote", "origin", &format!("refs/heads/{branch}")],
    )?;

    Some(said.split_whitespace().next()?.to_owned())
}

/// Write down how the suite is, and tell the open pages where that is news.
///
/// The watcher's, and the details pane's too: opening the pane asks GitHub the
/// same question, so it is also what freshens a rollup on a Conversation nothing
/// is watching any more.
///
/// The aggregate rather than the checks themselves: what the card has room for
/// is one icon, and which of the three afternoons this is is what a human wants
/// out of one. What every check is called and where its run is is not thrown
/// away by that — it is on GitHub, which is where a red one is read anyway.
///
/// Nudged only where the word changed. A suite that is still running says the
/// same thing every thirty seconds for as long as it takes, and a page told each
/// time would be a page re-reading a Timeline nothing had happened on.
pub(crate) async fn remember(state: &AppState, conversation_id: i64, checks: &[Check]) {
    // A pull request with no checks on it at all is not passing and is not
    // failing: there is nothing to say about a repository with no CI, and a
    // green tick would be one this suite never earned. So nothing is written
    // down, and the card draws no icon.
    let Some(rollup) = rollup(checks) else {
        return;
    };

    match store::record_check_rollup(&state.pool, conversation_id, rollup).await {
        Ok(true) => state.nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        }),
        Ok(false) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "recording how the checks are failed");
        }
    }
}

/// The one word a whole suite comes to, or nothing where there is no suite.
///
/// Red first, then unfinished, then green — see [`store::Rollup`], which is
/// where that order is argued. The same reading [`once`] above makes of the same
/// checks, so the icon on the card and the wrap-up's own patience cannot come to
/// disagree about a suite they are both looking at.
fn rollup(checks: &[Check]) -> Option<store::Rollup> {
    if checks.iter().any(|check| check.how == Checked::Failed) {
        return Some(store::Rollup::Failed);
    }

    if checks.iter().any(|check| check.how == Checked::Running) {
        return Some(store::Rollup::Running);
    }

    // Green, and only where there was something to be green: an empty suite
    // falls through to nothing at all.
    (!checks.is_empty()).then_some(store::Rollup::Passed)
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
    // try here — and the human is asked once it has nothing left to try anywhere.
    // See [`owed_elsewhere`], which is what keeps a pull request out of goes from
    // spending another one's.
    if fixable.is_empty() {
        if owed_elsewhere(state, conversation_id, watched.repo.id).await {
            tracing::debug!(
                conversation_id,
                repo = watched.repo.name,
                number = watched.number,
                "this pull request has had its goes and another still has one, so the \
                 run is not stopped over it yet",
            );

            return Watching::Again(writing);
        }

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

/// Whether another of this Conversation's pull requests still has a go coming to
/// it, which is what a pull request out of goes waits for before the run is
/// stopped over it.
///
/// A go of either kind. A pull request is out of goes when the machine has
/// nothing left to try on it at all — neither a fix for a check that will not go
/// green nor a resolution for a base it will not merge — and one that is still
/// owed a resolution is as much a reason to hold a stop as one owed a fix.
///
/// The attempts are counted per pull request because the same check name red on
/// two of them is two different failures, and one spending the other's would
/// stop a run that still had somewhere to go. A stop is the Conversation's
/// rather than one pull request's, though — nothing is dispatched past one — so
/// the first watcher to run out writing one would spend the other's goes just as
/// surely as sharing the count would, and which watcher that is is a matter of
/// which of them got the Turn first. So the human is asked once there is nowhere
/// left to go: every other pull request green, or out of goes as well.
///
/// Read off what has been counted rather than off the other watchers, which is
/// what makes it a fact a restarted server has too. A pull request nothing has
/// been dispatched about is one with its goes still in hand — whether its suite
/// is red and waiting for the Turn, or still running, or was never red at all —
/// and a wrap-up waits on a suite it has not read the end of anyway.
///
/// `false` where the record cannot be read, which is the stop this was in front
/// of going ahead: what that costs is a go, and holding a stop open on an
/// unreadable record would cost the human ever being told.
async fn owed_elsewhere(state: &AppState, conversation_id: i64, repo_id: i64) -> bool {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation whose pull requests still had a go failed");
            return false;
        }
    };

    let opened = match store::pull_requests(&state.pool, conversation_id).await {
        Ok(opened) => opened,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which pull requests still had a go failed");
            return false;
        }
    };

    let settled = match store::wrap_up_settled(&state.pool, conversation_id).await {
        Ok(settled) => settled,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which pull requests had gone green failed");
            return false;
        }
    };

    for (repo, opened) in opened {
        // This one, whose goes are what the caller has just run out of.
        if repo.id == repo_id {
            continue;
        }

        // And one there is nowhere left to ask about, which is a repository
        // taken off the registry mid-wrap-up. Its own watcher stopped on that
        // same fact, so a go it is owed is one nothing will ever spend — and
        // waiting for it would be a stop the human never got.
        if crate::wrapping::watched(&conversation, repo.id, opened.number).is_none() {
            continue;
        }

        // Its checks, where they have not gone green.
        if !settled.contains(&store::WaitingOn::Checks(repo.id)) {
            match store::most_fix_attempts(&state.pool, conversation_id, repo.id).await {
                Ok(spent) if spent < ATTEMPTS => return true,
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, repo = repo.name, "reading what a pull request had been given failed");
                }
            }
        }

        // And its conflict, where GitHub has not said it merges.
        if !settled.contains(&store::WaitingOn::Mergeable(repo.id)) {
            match store::conflict_fix_attempts(&state.pool, conversation_id, repo.id).await {
                Ok(spent) if spent < ATTEMPTS => return true,
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, repo = repo.name, "reading what a pull request's conflict had been given failed");
                }
            }
        }
    }

    false
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
/// [`store::Decision::Verkstead`]: every fix session the branch was allowed has
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

/// Write down whether GitHub can merge this pull request, and settle or unsettle
/// the wrap-up on the strength of it.
///
/// Three answers and three different things, exactly as the checks are three:
/// **MERGEABLE** settles one of the things the wrap-up waits on, **CONFLICTING**
/// puts it back to waiting, and **UNKNOWN** does neither. GitHub says the third
/// while it is still working the answer out, which is most of the first moments
/// after a push — and *not yet computed* is no more a conflict than a `gh` that
/// would not answer is a red check. So it writes nothing down either, and what
/// stands is the last thing GitHub did say.
///
/// What GitHub said, for the caller to act on — and `None` where it said
/// *UNKNOWN*, which is nothing to act on at all.
async fn merging(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
    mergeable: Mergeable,
) -> Option<store::Merging> {
    let merging = match mergeable {
        Mergeable::Cleanly => store::Merging::Cleanly,
        Mergeable::Conflicting => store::Merging::Conflicting,
        Mergeable::Unknown => {
            tracing::debug!(
                conversation_id,
                repo = watched.repo.name,
                number = watched.number,
                "GitHub has not worked out whether the pull request merges, which is \
                 nothing to conclude",
            );

            return None;
        }
    };

    // Written down before it is acted on, for the reason the rollup is: the
    // watching stops when the wrap-up is over and this is the one place that
    // asks, so a reading that went no further than the settle would be one
    // nothing could draw afterwards.
    if let Err(error) =
        store::record_merging(&state.pool, conversation_id, watched.repo.id, merging).await
    {
        tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "recording whether the pull request merges failed");
    }

    let waiting_on = store::WaitingOn::Mergeable(watched.repo.id);

    let written = match merging {
        store::Merging::Cleanly => {
            store::settle_wrap_up(&state.pool, conversation_id, waiting_on).await
        }
        store::Merging::Conflicting => {
            // Said on every poll for as long as the conflict stands, which is
            // why it is not an `info!`: what a human reads a conflict off is the
            // record rather than the log.
            tracing::debug!(
                conversation_id,
                repo = watched.repo.name,
                number = watched.number,
                "the pull request conflicts with its base, so the wrap-up goes on waiting",
            );

            store::unsettle_wrap_up(&state.pool, conversation_id, waiting_on).await
        }
    };

    if let Err(error) = written {
        tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "recording whether the wrap-up was waiting on a conflict failed");
    }

    Some(merging)
}

/// Dispatch a resolution session at a pull request that will not merge, or ask
/// the human where it has had its goes.
///
/// The check fixes' shape end to end, and deliberately so: the same dispatch,
/// the same Turn, the same two goes counted as the session starts, and the same
/// waiting for every other pull request before a stop is written. What differs
/// is what the session is told to do — see [`resolving`].
///
/// **The Turn is tried rather than waited for**, exactly as it is for a check.
/// What else is in the Worktree is the review session or a finding the human
/// accepted, and a resolution queued behind one would be dispatched about a base
/// nobody has looked at since. Coming back in half a minute costs nothing and
/// asks GitHub afresh — and nothing is counted for a poll that could not get in,
/// the count being of sessions dispatched rather than of conflicts seen.
async fn resolve(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
    writing: Option<i64>,
) -> Watching {
    let spent = match store::conflict_fix_attempts(&state.pool, conversation_id, watched.repo.id)
        .await
    {
        Ok(spent) => spent,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "reading what a conflict had been given failed");
            return Watching::Again(writing);
        }
    };

    // The machine has had its goes at this one, so nothing further is dispatched
    // for it — and the human is asked once there is nothing left to try
    // anywhere. See [`owed_elsewhere`], which is what keeps a pull request out of
    // goes from spending another's.
    if spent >= ATTEMPTS {
        if owed_elsewhere(state, conversation_id, watched.repo.id).await {
            tracing::debug!(
                conversation_id,
                repo = watched.repo.name,
                number = watched.number,
                "this pull request has had its goes at the conflict and another still \
                 has one, so the run is not stopped over it yet",
            );

            return Watching::Again(writing);
        }

        return unmergeable(state, conversation_id, watched, writing).await;
    }

    let Some(_turn) = state.sessions.try_turn(conversation_id) else {
        tracing::debug!(
            conversation_id,
            repo = watched.repo.name,
            "something else is working in the Worktree, so the conflict is looked at again later",
        );
        return Watching::Again(writing);
    };

    // Counted as the session is dispatched rather than as it ends, so that an
    // attempt spent by a server that then restarted is one the next server does
    // not spend again.
    if let Err(error) =
        store::record_conflict_fix_attempt(&state.pool, conversation_id, watched.repo.id).await
    {
        tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "counting a resolution session failed");
        return Watching::Again(writing);
    }

    tracing::info!(
        conversation_id,
        repo = watched.repo.name,
        number = watched.number,
        "the pull request will not merge, so a session is starting on the conflict",
    );

    let said = crate::runner::address(state, conversation_id, &resolving(watched)).await;

    Watching::Again(said.or(writing))
}

/// Stop asking the machine about a pull request that will not merge, and say so
/// on the Timeline.
///
/// [`ask`]'s twin, and it names the pull request in the step for the same
/// reason: a Conversation has more than one, and what stopped is *this* branch
/// against *its* base rather than merging in general.
///
/// The evidence is the tail of what the last session said, which
/// [`crate::stopping::stop`] reads off `writing` — a resolution that could not
/// be made is usually a session saying why.
async fn unmergeable(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
    writing: Option<i64>,
) -> Watching {
    let how = format!(
        "{} still conflicts with its base branch after {ATTEMPTS} resolution sessions. \
         Nothing lands until the conflict is resolved by hand.",
        named(watched),
    );

    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        &format!("resolving the conflicts on {}", named(watched)),
        &how,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "the pull request would not merge and the stop saying so could not be recorded"
        );
    }

    Watching::Done("the pull request would not merge, so the human is being asked")
}

/// What a resolution session is told: which pull request will not merge, where
/// to work, and what to do about it.
///
/// Where to work for [`feedback`]'s reason, and named the same way whichever
/// repository it is: `git` reads its repository from wherever it runs, so a
/// merge made in the wrong checkout is a change to work nobody asked about.
///
/// **Merge rather than rebase.** A rebase rewrites the branch and would have to
/// be force-pushed, which throws away whatever the pull request's reviewers have
/// already read and breaks anything stacked on top of it. A merge commit is
/// ugly and safe, and safe is what an unattended session should be. The strategy
/// is settled here rather than asked about — task 03 is what makes it
/// configurable.
fn resolving(watched: &Watched) -> String {
    format!(
        "GitHub cannot merge {} into its base branch: the branch and the base have both \
         changed the same lines since they parted. Work in that repository's worktree, at \
         `{}` — `git` reads the repository from wherever it is run, so a merge made anywhere \
         else is a different repository's.\n\nMerge the pull request's base branch into the \
         branch that worktree is on, resolve every conflict, run the repository's tests, then \
         commit the merge and push it. A merge rather than a rebase: nothing here \
         force-pushes, so whatever has been read or stacked on this branch goes on \
         standing.\n\nA conflict is two changes to reconcile. Neither side is the one to keep \
         — taking the branch's hunk or the base's wholesale would throw away work somebody \
         did, so read both and write what they both meant.",
        named(watched),
        watched.worktree.display(),
    )
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

    /// The same, for the tests that are about how a check is getting on rather
    /// than about where its run is.
    fn how(name: &str, how: Checked) -> Check {
        Check {
            how,
            ..check(name, "")
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

    /// What a resolution session is told: which pull request will not merge,
    /// which repository's worktree to do it in, and what *resolving* means here
    /// — merge the base in, and neither side thrown away.
    #[test]
    fn a_resolution_session_is_told_which_pull_request_will_not_merge_and_how_to_fix_it() {
        let told = resolving(&watched());

        assert!(
            told.contains("#7") && told.contains("askance"),
            "which pull request, in which repository: {told}",
        );
        assert!(
            told.contains("/state/worktrees/rate-limiting-askance"),
            "and the worktree to do the merge in, `git` reading its repository from \
             wherever it is run: {told}",
        );
        assert!(
            told.contains("Merge the pull request's base branch"),
            "the strategy is the one that does not rewrite the branch: {told}",
        );
        assert!(
            told.contains("rather than a rebase") && told.contains("force-push"),
            "said as what it is not, because a rebase would have to be force-pushed: \
             {told}",
        );
        assert!(
            told.contains("push"),
            "and the resolution has to reach the pull request: {told}",
        );
        assert!(
            told.contains("two changes to reconcile"),
            "and neither side is the one to keep: {told}",
        );
    }

    /// A check GitHub gave no link for is still a check to fix, and the line
    /// says its name rather than trailing an empty dash.
    #[test]
    fn a_check_with_no_run_to_link_to_is_listed_by_name_alone() {
        assert_eq!(listed(&[check("Rust", "")]), "- Rust");
    }

    /// One red check is a red suite, whatever the rest of it is doing. It is
    /// the thing to go and look at, and a card saying anything else about a
    /// suite with a failure in it would be a card sending nobody.
    #[test]
    fn a_suite_with_anything_red_in_it_is_red() {
        assert_eq!(
            rollup(&[
                how("Rust", Checked::Passed),
                how("Web", Checked::Failed),
                how("Nix", Checked::Running),
            ]),
            Some(store::Rollup::Failed),
        );
    }

    /// And nothing red with something unfinished is a suite still running:
    /// green is a thing the whole of it has to have earned.
    #[test]
    fn a_suite_with_nothing_red_and_something_unfinished_is_running() {
        assert_eq!(
            rollup(&[how("Rust", Checked::Passed), how("Web", Checked::Running)]),
            Some(store::Rollup::Running),
        );
    }

    /// Every check finished and none of them red.
    #[test]
    fn a_suite_that_has_finished_with_nothing_red_has_passed() {
        assert_eq!(
            rollup(&[how("Rust", Checked::Passed), how("Web", Checked::Passed)]),
            Some(store::Rollup::Passed),
        );
    }

    /// And a pull request with no checks on it at all says nothing rather than
    /// green: a repository with no CI has passed nothing, and the card draws no
    /// icon for it.
    #[test]
    fn a_pull_request_with_no_checks_on_it_is_not_a_green_one() {
        assert_eq!(rollup(&[]), None);
    }
}
