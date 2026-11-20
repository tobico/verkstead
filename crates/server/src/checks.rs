//! What the host still does for the wrap-up's **checks** watcher, which runs
//! inside the Wrap-up component (`plugins/activities/wrap-up`) and reaches the
//! server through the `activity` world's host functions alone.
//!
//! **The component decides; the host remembers and words.** Every round the
//! component asks the Forge for each pull request's checks, settles the wrap-up
//! on them, dispatches a fix session at a red one and a resolution session at a
//! conflict, and gives the wrap-up up once the machine has had its goes. What
//! is here is what only the host can do for it: the readings written down for
//! the card ([`remember`], [`marked`]), what origin holds a branch at
//! ([`pushed`]), the conflict's goes counted in the store, what a resolution
//! session is told ([`resolution`]), and whether a stop would spend another
//! pull request's goes ([`give_up`]).
//!
//! **Two attempts per check per pull request, then it stops asking the machine
//! and starts asking the human** — [`ATTEMPTS`], which the host hands the
//! component as its `attempts` option. The count is kept in the store, per
//! check name and per pull request, so a restarted server does not start the
//! counting again. A conflict's goes are counted per repository, which for a
//! stack is per stack.
//!
//! **And the stop waits for the rest of them.** A stop is the Conversation's
//! rather than one pull request's, so a pull request out of goes waits until
//! every other one has gone green or run out as well — see [`owed_elsewhere`],
//! read off what has been counted, which a restarted server has too.
//!
//! **A green suite is only ever green about one commit.** GitHub's record runs
//! behind the branch for a while after a push, so the component holds the head
//! a green rollup names against what origin is holding this pull request's own
//! branch at — [`pushed`] — and settles nothing where the two differ.
//!
//! **All of this stops at Done**, which is where [`crate::merges`] takes over:
//! what that sweep does with a conflict is write it down, nothing more.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use verkstead_schema::Nudge;

use crate::AppState;
use crate::github::{Check, Checked, Credential, Mergeable};
use crate::plugins::component::worlds::category::activity::verkstead::plugin::activity_types as world;
use crate::repos::git;
use crate::store;
use crate::unseen::Unseen;
use crate::wrapping::{Watched, named};

/// How many fix sessions one check gets before the human is asked instead, and
/// how many resolution sessions a conflict gets. The Wrap-up component is
/// handed it as its `attempts` option.
///
/// Two, which is one automatic go and one more after it did not work. A third
/// would be a machine spending an account on the same failure with nobody
/// watching, which is the whole thing a stop exists instead of.
pub(crate) const ATTEMPTS: i64 = 2;

/// Forget what a Conversation's checks and conflicts have already been tried,
/// and watch them again.
///
/// What Resume does to a wrap-up that stopped. The attempts go first: the human
/// has read the Notice of what stopped and asked for another go, and a count
/// left standing would be a watcher that stopped all over again on its next poll
/// without dispatching anything.
///
/// **Both counts**, because either of them is what the Notice they read could
/// have been about: a wrap-up stops on the checks that would not go green or on
/// the pull request that would not merge, and a press that forgave only one of
/// them would be a Resume that stopped again on the other. See
/// [`store::forget_fix_attempts`], which is where the two are taken away
/// together.
pub(crate) async fn afresh(state: AppState, conversation_id: i64) {
    if let Err(error) = store::forget_fix_attempts(&state.pool, conversation_id).await {
        tracing::error!(error = ?error, conversation_id, "forgetting what a Conversation's checks and conflicts had been given failed");
        return;
    }

    tracing::info!(
        conversation_id,
        "the checks and the conflicts are being tried again from no attempts spent"
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

/// `pull_request`, as the wrap-up watches it: the Repo it is in, the branch the
/// record says its work is on, and the checkout that work is done in — or the
/// sentence a call naming it is refused with.
async fn watched(
    state: &AppState,
    conversation_id: i64,
    pull_request: &world::PullRequest,
) -> Result<Watched, String> {
    let named = crate::forge::named(state, conversation_id, pull_request).await?;

    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return Err("There is no Conversation left to ask about.".to_owned()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation a pull request is watched on failed");
            return Err("The Conversation couldn't be read.".to_owned());
        }
    };

    let opened = match store::pull_request_numbered(
        &state.pool,
        conversation_id,
        named.repo.id,
        named.number,
    )
    .await
    {
        Ok(Some(opened)) => opened,
        Ok(None) => {
            return Err(format!(
                "Pull request #{} of `{}` isn't on the record.",
                named.number, named.repo.name
            ));
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, repo = named.repo.name, number = named.number, "reading a watched pull request failed");
            return Err("The pull request couldn't be read.".to_owned());
        }
    };

    crate::wrapping::watched(&conversation, named.repo.id, &opened).ok_or_else(|| {
        format!(
            "There's no checkout left to work on pull request #{} of `{}` in.",
            named.number, named.repo.name
        )
    })
}

/// What origin holds `pull_request`'s branch at, where that can be told — the
/// `pushed-head` host function. See [`pushed_head`].
pub(crate) async fn pushed(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
) -> Option<String> {
    let watched = watched(state, conversation_id, &pull_request).await.ok()?;
    let credential = state
        .github
        .as_author(watched.repo.author.as_deref())
        .credential();

    // Off the runtime's threads: this is a process, and one that goes to the
    // network.
    tokio::task::spawn_blocking(move || {
        pushed_head(&watched.worktree, watched.head.as_deref(), &credential)
    })
    .await
    .unwrap_or_default()
}

/// How many resolution sessions the conflicts in `pull_request`'s Repo have
/// had — the `conflict-attempts` host function.
pub(crate) async fn conflict_attempts(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
) -> Result<i64, String> {
    let named = crate::forge::named(state, conversation_id, &pull_request).await?;

    store::conflict_fix_attempts(&state.pool, conversation_id, named.repo.id)
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, conversation_id, repo = named.repo.name, "reading what a conflict had been given failed");
            "The resolution attempts couldn't be read.".to_owned()
        })
}

/// Count one more, as a resolution session is dispatched — the
/// `count-conflict-attempt` host function. Counted as the session is
/// dispatched rather than as it ends, so that an attempt spent by a server
/// that then restarted is one the next server does not spend again.
pub(crate) async fn count_conflict_attempt(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
) -> Result<i64, String> {
    let named = crate::forge::named(state, conversation_id, &pull_request).await?;

    store::record_conflict_fix_attempt(&state.pool, conversation_id, named.repo.id)
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, conversation_id, repo = named.repo.name, "counting a resolution session failed");
            "The resolution attempt couldn't be counted.".to_owned()
        })
}

/// What a session sent at `pull_request`'s conflict is told — the
/// `resolution` host function.
///
/// **A stack gets one session for the whole of it**, and that is the one place
/// the configured strategy is not read: a fix low in a stack moves every branch
/// above it, so the session is told the ordered branches from the bottom and
/// told to sync them with `gh stack sync` — see [`syncing`]. And because that
/// extension is a separate install, it is asked for *before* a go is spent: a
/// Sandbox without it stops the run with a Notice naming it, and the answer is
/// that stop's sentence. See [`unsyncable`] and [`crate::stacks::missing`].
///
/// A lone pull request is told the strategy the settings file names, read as
/// the session is dispatched rather than held from anywhere: a resolution
/// configured a minute ago is what the next conflict is resolved by. See
/// [`resolving`].
pub(crate) async fn resolution(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
    writing: Option<i64>,
) -> Result<String, String> {
    let watched = watched(state, conversation_id, &pull_request).await?;

    // Which pull requests of this repository the Conversation holds, in order
    // from the bottom — one where nothing was walked, and the chain where one
    // was.
    let stack = store::stack(&state.pool, conversation_id, watched.repo.id)
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "reading the stack a conflicting pull request is one of failed");
            "The stack couldn't be read.".to_owned()
        })?;

    match chained(&stack) {
        Some(branches) => match crate::stacks::missing(state, conversation_id).await {
            Some(why) => Err(unsyncable(state, conversation_id, &watched, &why, writing).await),
            None => Ok(syncing(&watched, &stack, &branches)),
        },
        None => Ok(resolving(
            &watched,
            state.settings.config().conflict_resolution(),
        )),
    }
}

/// Give the wrap-up up over `pull_request` — the `give-up` host function —
/// unless another of the Conversation's pull requests still has a go coming,
/// which is `false` and nothing written. See [`owed_elsewhere`].
///
/// [`crate::wrapping::give_up`] writes the stop as the Activity answers
/// `given-up`, with the tail of `writing` — the last session dispatched, which
/// is where the reason it could not fix the thing is usually written down.
pub(crate) async fn give_up(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
    what: String,
    how: String,
    writing: Option<i64>,
) -> bool {
    let Ok(watched) = watched(state, conversation_id, &pull_request).await else {
        return false;
    };

    if owed_elsewhere(state, conversation_id, &watched).await {
        tracing::debug!(
            conversation_id,
            repo = watched.repo.name,
            number = watched.number,
            "this pull request has had its goes and another still has one, so the run is \
             not stopped over it yet",
        );

        return false;
    }

    crate::wrapping::give_up(
        state,
        conversation_id,
        crate::wrapping::GaveUp { what, how, writing },
    )
    .await;

    true
}

/// Write down whether GitHub can merge this pull request, for the card's mark,
/// telling the open pages where that is news. `UNKNOWN` — GitHub still
/// working it out — writes nothing, and what stands is the last thing it did
/// say. Settling the wrap-up on it is the component's.
pub(crate) async fn marked(
    state: &AppState,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
    mergeable: Mergeable,
) {
    let merging = match mergeable {
        Mergeable::Cleanly => store::Merging::Cleanly,
        Mergeable::Conflicting => store::Merging::Conflicting,
        Mergeable::Unknown => return,
    };

    match store::record_merging(&state.pool, conversation_id, repo_id, number, merging).await {
        Ok(true) => state.nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        }),
        Ok(false) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, repo_id, number, "recording whether the pull request merges failed");
        }
    }
}

/// What origin is holding this pull request's branch on, asked as part of the
/// poll.
///
/// `head` is the branch the record says the pull request's work is on, and it is
/// what is asked about: a repository wrapping up a **stack** has several branches
/// through the one Worktree, so the checkout answers for at most one of them and
/// a rollup held against the wrong branch is a green suite either refused or
/// believed for no reason. `None` is a pull request recorded before Verkstead
/// wrote the head down, and there the checkout stands in — which is the branch
/// such a row was on, the Conversation's own being the only pull request it was
/// possible for it to be.
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
fn pushed_head(worktree: &Path, head: Option<&str>, credential: &Credential) -> Option<String> {
    let checked_out;

    let branch = match head {
        Some(head) => head,
        None => {
            checked_out = git(worktree, &["symbolic-ref", "--short", "HEAD"])?;
            checked_out.trim()
        }
    };

    // One line of `<commit>\trefs/heads/<branch>`, or nothing at all where origin
    // is not holding that branch — which `ls-remote` reports by saying nothing
    // rather than by failing. Asked as the Repo's Author, the way the fetch
    // before a cut is: a private repository under the second Author's account
    // answers that Author and nobody else.
    let said = credential
        .on(&mut Command::new("git"))
        .args(["ls-remote", "origin", &format!("refs/heads/{branch}")])
        .unseen()
        .current_dir(worktree)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())?;

    Some(
        String::from_utf8_lossy(&said.stdout)
            .split_whitespace()
            .next()?
            .to_owned(),
    )
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
///
/// Written against the pull request it is a suite of rather than against the
/// Conversation, because that is what it is a suite of: `repo_id` and `number`
/// together are which one, and a Conversation with a read-write companion or a
/// stack has as many suites as it has pull requests.
pub(crate) async fn remember(
    state: &AppState,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
    checks: &[Check],
) {
    // A pull request with no checks on it at all is not passing and is not
    // failing: there is nothing to say about a repository with no CI, and a
    // green tick would be one this suite never earned. So nothing is written
    // down, and the card draws no icon.
    let Some(rollup) = rollup(checks) else {
        return;
    };

    match store::record_check_rollup(&state.pool, conversation_id, repo_id, number, rollup).await {
        Ok(true) => state.nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        }),
        Ok(false) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, repo_id, number, "recording how the checks are failed");
        }
    }
}

/// The one word a whole suite comes to, or nothing where there is no suite.
///
/// Red first, then unfinished, then green — see [`store::Rollup`], which is
/// where that order is argued. The same reading the Wrap-up component makes of the same
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

/// Whether another of this Conversation's pull requests still has a go coming to
/// it, which is what a pull request out of goes waits for before the run is
/// stopped over it.
///
/// A go of either kind. A pull request is out of goes when the machine has
/// nothing left to try on it at all — neither a fix for a check that will not go
/// green nor a resolution for a base it will not merge — and one that is still
/// owed a resolution is as much a reason to hold a stop as one owed a fix.
///
/// The checks' attempts are counted per pull request because the same check name
/// red on two of them is two different failures, and one spending the other's
/// would stop a run that still had somewhere to go. The conflict's are counted
/// per repository, that being per stack — see
/// [`store::conflict_fix_attempts`] — so a pull request of a stack is asked
/// about its own suite and its repository's conflicts. A stop is the Conversation's
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
async fn owed_elsewhere(state: &AppState, conversation_id: i64, watched: &Watched) -> bool {
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
        // This one, whose goes are what the caller has just run out of. By the
        // pull request rather than by the repository: the others of a stack are
        // in this same one and each has goes of its own.
        if (repo.id, opened.number) == (watched.repo.id, watched.number) {
            continue;
        }

        // And one there is nowhere left to ask about, which is a repository
        // taken off the registry mid-wrap-up. Its own watcher stopped on that
        // same fact, so a go it is owed is one nothing will ever spend — and
        // waiting for it would be a stop the human never got.
        if crate::wrapping::watched(&conversation, repo.id, &opened).is_none() {
            continue;
        }

        let number = opened.number;
        let repo_id = repo.id;

        // Its checks, where they have not gone green.
        if !settled.contains(&store::WaitingOn::Checks { repo_id, number }) {
            match store::most_fix_attempts(&state.pool, conversation_id, repo_id, number).await {
                Ok(spent) if spent < ATTEMPTS => return true,
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, repo = repo.name, number, "reading what a pull request had been given failed");
                }
            }
        }

        // And its conflict, where GitHub has not said it merges. Counted per
        // repository, so a pull request of the same stack as the caller's reads
        // the same count back — which is the answer that count is meant to give:
        // one resolution is one act over the whole chain.
        if !settled.contains(&store::WaitingOn::Mergeable { repo_id, number }) {
            match store::conflict_fix_attempts(&state.pool, conversation_id, repo_id).await {
                Ok(spent) if spent < ATTEMPTS => return true,
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, repo = repo.name, number, "reading what a pull request's conflict had been given failed");
                }
            }
        }
    }

    false
}

/// Stop asking the machine about a stack nothing here can sync, and say so on
/// the Timeline.
///
/// The opposite of a conflict given up on in one respect: that is written once
/// the machine has had its goes, and this before it has had any. What
/// is missing is the extension rather than a resolution, and no number of
/// sessions would find it — so the goes are left where they are and the human
/// is asked straight away.
///
/// **The Notice names the extension and the command that installs it.** A stop
/// saying only that something did not work would be one somebody had to open a
/// terminal to understand; what this is for is the human reading the one line
/// and knowing the one thing to run. See [`crate::stacks::INSTALL`]. Answers
/// the Notice's reason, which is what the `resolution` host function is
/// refused with.
async fn unsyncable(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
    why: &str,
    writing: Option<i64>,
) -> String {
    let how = format!(
        "{} is one of a stack, and a stack is synced with the `gh {extension}` extension \
         rather than merged into branch by branch. A session here can't run it. {why} \
         Install it with `{install}` and press **Resume**. Nothing has been spent on the \
         conflict, and the goes are still there once the extension is.",
        crate::stopping::opening(&named(watched)),
        extension = crate::stacks::EXTENSION,
        install = crate::stacks::INSTALL,
    );

    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        &format!("syncing the stack {} is one of", named(watched)),
        &how,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "the stack could not be synced and the stop saying so could not be recorded"
        );
    }

    how
}

/// What a resolution session is told: which pull request will not merge, where
/// to work, and what to do about it — in the words of the strategy this
/// repository resolves conflicts by.
///
/// Where to work for the fix session's reason, and named the same way whichever
/// repository it is: `git` reads its repository from wherever it runs, so a
/// merge made in the wrong checkout is a change to work nobody asked about.
///
/// **The strategy is named rather than left to the session.** The two are
/// genuinely different acts on the branch — a merge leaves every pushed commit
/// where it is, a rebase writes them again and force-pushes over what reviewers
/// have read — so a session left to pick would be picking something the human
/// has a setting for. Which is why the merge says *not a rebase* and the rebase
/// says *with a lease*: each has to say the thing the other would have done.
fn resolving(watched: &Watched, resolution: store::ConflictResolution) -> String {
    let how = match resolution {
        store::ConflictResolution::Merge => {
            "Merge the pull request's base branch into the branch that worktree is on, resolve \
             every conflict, run the repository's tests, then commit the merge and push it. A \
             merge rather than a rebase: nothing here force-pushes, so whatever has been read \
             or stacked on this branch goes on standing."
        }
        store::ConflictResolution::Rebase => {
            "Rebase the branch that worktree is on onto the pull request's base branch, resolve \
             every conflict as the rebase reaches it, run the repository's tests, then \
             force-push the rebased branch with `--force-with-lease`. A rebase rather than a \
             merge: this repository is configured for one, so the branch is rewritten and the \
             lease is what stops the push landing over work that arrived while you were \
             resolving."
        }
    };

    format!(
        "GitHub cannot merge {} into its base branch: the branch and the base have both \
         changed the same lines since they parted. Work in that repository's worktree, at \
         `{}` — `git` reads the repository from wherever it is run, so a resolution made \
         anywhere else is a different repository's.\n\n{how}\n\nA conflict is two changes to \
         reconcile. Neither side is the one to keep — taking the branch's hunk or the base's \
         wholesale would throw away work somebody did, so read both and write what they both \
         meant.",
        named(watched),
        watched.worktree.display(),
    )
}

/// The branches of `stack` from the bottom, where it is a stack whose branches
/// are all on the record — and `None` where it is not one.
///
/// Two things are being asked at once, and they are one question: whether there
/// is more than one pull request here, and whether every one of them says which
/// branch it is on. A session told to sync a stack is told it as a list of
/// branches, so a chain with a row that never recorded its head is one nothing
/// could be told to sync — and that is a row written before Verkstead wrote
/// heads down, which is to say before there were stacks at all. It falls back
/// on the configured strategy, which is what such a Conversation has always
/// had.
///
/// The order is [`store::stack`]'s, which is the chain read off the rows: the
/// bottom first, because that is the end a sync works from and the end a human
/// reads a stack from.
fn chained(stack: &[store::PullRequest]) -> Option<Vec<&str>> {
    if stack.len() < 2 {
        return None;
    }

    let branches: Vec<&str> = stack
        .iter()
        .filter_map(|opened| opened.head.as_deref())
        .collect();

    (branches.len() == stack.len()).then_some(branches)
}

/// What a session sent at a **stack** is told: which pull request will not
/// merge, which chain it is one of, where to work, and to sync the whole thing.
///
/// [`resolving`]'s twin, and what it says instead of a strategy is the one
/// thing the strategy cannot say. A stack is a chain of branches each based on
/// the one under it, and a fix low in one moves every branch above it — so
/// merging a base into each branch in turn is not the same act done several
/// times, it is the act `gh stack`'s own documentation warns against. The
/// extension cascades the rebase and force-pushes the chain atomically, which
/// is why what the session is told is the extension rather than git.
///
/// **The registry is per worktree**, and this Conversation's was made minutes
/// ago by the take-up, so there may be nothing in it: the session is told to
/// adopt the chain where `gh stack view` finds none. The branches are named in
/// order from the bottom, which is the order a chain is adopted in and the
/// order a human checks it against, and the command is named with them: one
/// `gh stack init` takes a chain of any depth, bottom first, which
/// `docs/agents/git-workflow.md` now documents from what the extension was
/// measured doing rather than from what its own documentation says. A prompt
/// that left the argument list to the session would be a session working out
/// again what the repository has already written down.
///
/// **And what this checkout holds is what an adoption adopts.** The take-up cut
/// its Worktree on the Conversation's own branch, and the rest of the chain may
/// be nowhere but `origin/…`: adopting a branch that is not here *creates* it
/// rather than refusing, and the sync force-pushes whatever was adopted — so a
/// branch invented empty here is the real one on origin overwritten, and in
/// this workbench that branch is another Conversation's work. So the session is
/// told to fetch and have every branch of the chain locally at origin's commit
/// before it adopts anything, and to read the adopted chain back before it
/// syncs.
///
/// **And what the sync backs out on is the session's to resolve.** `gh stack
/// sync` refuses rather than half-rebases when a branch conflicts, and `gh
/// stack rebase` is the verb that stops in it — so the session is told both,
/// and told that the resolution is the point rather than the sync succeeding.
///
/// Where to work for the fix session's reason, and named the same way whichever
/// repository it is. Every branch of the stack is a branch of *this*
/// repository — that is what the walk refuses a fork for — so they are all
/// reached through the one Worktree.
fn syncing(watched: &Watched, stack: &[store::PullRequest], branches: &[&str]) -> String {
    let listed: Vec<String> = stack
        .iter()
        .map(|opened| match opened.head.as_deref() {
            Some(head) => format!("#{} (`{head}`)", opened.number),
            None => format!("#{}", opened.number),
        })
        .collect();

    format!(
        "GitHub cannot merge {} into its base branch, and it is one of a stack of {deep} \
         pull requests in that repository — from the bottom: {listed}. A fix low in a stack \
         moves every branch above it, so the whole chain is yours to put right rather than \
         the one pull request.\n\n\
         Work in that repository's worktree, at `{worktree}` — `git` and `gh` both read the \
         repository from wherever they are run, and every branch of this stack is in that \
         one repository, so the whole job is done in that one directory.\n\n\
         Sync the stack with the `gh {extension}` extension rather than merging or rebasing \
         by hand: it cascade-rebases each branch onto its updated parent and force-pushes \
         them atomically, which is what keeps the chain a chain. Its registry is kept per \
         worktree and this worktree is new, so run `gh {extension} view` first.\n\n\
         Where it knows of no stack, the chain is yours to adopt — and what this checkout \
         holds is what gets adopted. Run `git fetch origin`, and make sure there is a \
         local branch at origin's commit for every one of {init}, bottom first, before \
         you adopt anything: adopting a branch this checkout has not got creates it empty \
         rather than refusing, and the sync below force-pushes whatever was adopted, so a \
         branch invented here is the real one on origin overwritten — and these branches \
         are other people's work. Then adopt the whole chain with one command, bottom \
         first — `gh {extension} init {adopting}` — and read it back with \
         `gh {extension} view` before you sync anything: a chain that came back wrong is \
         one to stop at rather than to force-push.\n\n\
         Then `gh stack sync`. Where it reports a conflict it backs out rather than leaving \
         a branch half-rebased, and `gh stack rebase` is what walks the chain again and \
         stops in the conflict for you to resolve. Resolving it is the job: a conflict is \
         two changes to reconcile, and taking one side's hunk wholesale throws away work \
         somebody did. Then run the repository's tests over the top branch, and make sure \
         every branch of the stack is pushed before you finish — the sync force-pushes \
         them, which is what a stack is rebased and re-linked by, and is the one place \
         force-pushing is what was asked for.",
        named(watched),
        deep = stack.len(),
        listed = listed.join(", "),
        worktree = watched.worktree.display(),
        extension = crate::stacks::EXTENSION,
        init = branches
            .iter()
            .map(|branch| format!("`{branch}`"))
            .collect::<Vec<_>>()
            .join(", "),
        adopting = branches.join(" "),
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
    use std::process::{Command, Stdio};

    use super::*;

    fn check(name: &str, link: &str) -> Check {
        Check {
            name: name.to_owned(),
            how: Checked::Failed,
            link: link.to_owned(),
        }
    }

    /// A checkout with an origin holding two branches, which is a **stack**: the
    /// chain's lower branch and the one based on it, both pushed, in the one
    /// worktree.
    ///
    /// Handed back with the directories that keep them alive, and with what
    /// origin is holding each branch on.
    fn stacked() -> (tempfile::TempDir, tempfile::TempDir, String, String) {
        let origin = tempfile::tempdir().unwrap();
        run(
            origin.path(),
            &["init", "--bare", "--initial-branch", "main"],
        );

        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path();

        run(worktree, &["init", "--initial-branch", "main"]);
        run(
            worktree,
            &["config", "user.email", "test@verkstead.invalid"],
        );
        run(worktree, &["config", "user.name", "Verkstead Test"]);
        run(
            worktree,
            &[
                "remote",
                "add",
                "origin",
                &origin.path().display().to_string(),
            ],
        );

        std::fs::write(worktree.join("README.md"), "# a repository\n").unwrap();
        run(worktree, &["add", "README.md"]);
        run(worktree, &["commit", "-m", "first"]);

        run(worktree, &["checkout", "-b", "rate-limiting"]);
        std::fs::write(worktree.join("low.md"), "the branch below\n").unwrap();
        run(worktree, &["add", "low.md"]);
        run(worktree, &["commit", "-m", "the branch below"]);
        run(worktree, &["push", "--quiet", "origin", "HEAD"]);

        let low = head(worktree);

        run(worktree, &["checkout", "-b", "rate-limiting-2"]);
        std::fs::write(worktree.join("high.md"), "the branch above\n").unwrap();
        run(worktree, &["add", "high.md"]);
        run(worktree, &["commit", "-m", "the branch above"]);
        run(worktree, &["push", "--quiet", "origin", "HEAD"]);

        let high = head(worktree);

        (origin, dir, low, high)
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

    fn head(dir: &Path) -> String {
        run(dir, &["rev-parse", "HEAD"]).trim().to_owned()
    }

    /// What a green rollup is held against is what origin is holding on *this*
    /// pull request's branch, whichever branch the Worktree happens to be on.
    ///
    /// The whole reason the head is written down: a repository wrapping up a stack
    /// has several pull requests through the one checkout, so a rollup held against
    /// whatever `HEAD` points at would be a green suite refused on two branches out
    /// of three — or believed on the strength of another branch's commit.
    #[test]
    fn what_was_pushed_is_read_off_the_pull_requests_own_branch() {
        let (_origin, dir, low, high) = stacked();
        let worktree = dir.path();

        assert_ne!(low, high, "the two branches are at different commits");

        // The Worktree is left on the top of the stack, which is where `gh stack
        // checkout` leaves one — and the branch below is still a pull request with
        // a suite of its own.
        assert_eq!(
            pushed_head(worktree, Some("rate-limiting"), &Credential::host()).as_deref(),
            Some(low.as_str()),
            "the branch below the checkout is asked about by name",
        );
        assert_eq!(
            pushed_head(worktree, Some("rate-limiting-2"), &Credential::host()).as_deref(),
            Some(high.as_str()),
            "and so is the one the checkout is on",
        );

        // And a commit in hand is not a commit GitHub could have run anything
        // against: what is asked for is what origin holds rather than what the
        // Worktree has got to.
        std::fs::write(worktree.join("more.md"), "not pushed yet\n").unwrap();
        run(worktree, &["add", "more.md"]);
        run(worktree, &["commit", "-m", "not pushed yet"]);

        assert_ne!(head(worktree), high);
        assert_eq!(
            pushed_head(worktree, Some("rate-limiting-2"), &Credential::host()).as_deref(),
            Some(high.as_str()),
            "an unpushed commit is not what a rollup is held against",
        );

        // A pull request recorded before Verkstead wrote the head down has none,
        // and there the checkout stands in — which is the branch such a row was on.
        assert_eq!(
            pushed_head(worktree, None, &Credential::host()).as_deref(),
            Some(high.as_str()),
            "with no head on the record, the branch the Worktree is on is asked about",
        );

        // And a branch origin has never heard of is *Verkstead cannot tell*, which
        // holds nothing up.
        assert_eq!(
            pushed_head(worktree, Some("rate-limiting-3"), &Credential::host()),
            None
        );
    }

    /// And origin is asked as the Repo's Author, through the same helper the
    /// fetch before a cut goes through: the token it was handed, and no other.
    #[test]
    fn what_was_pushed_is_asked_as_the_repos_author() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), &["init", "--initial-branch", "main"]);

        let (url, offered) = crate::stand_ins::remote_recording_logins();
        run(dir.path(), &["remote", "add", "origin", &url]);

        assert_eq!(
            pushed_head(
                dir.path(),
                Some("rate-limiting"),
                &Credential::token("ghp_thehomeone"),
            ),
            None,
            "a remote that turns the login away is *Verkstead cannot tell*",
        );
        assert_eq!(
            *offered.lock().unwrap(),
            vec!["x-access-token:ghp_thehomeone".to_owned()],
        );
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
                author: None,
            },
            number: 7,
            head: Some("rate-limiting".to_owned()),
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

    /// What a resolution session is told: which pull request will not merge,
    /// which repository's worktree to do it in, and what *resolving* means here
    /// — merge the base in, and neither side thrown away.
    #[test]
    fn a_resolution_session_is_told_which_pull_request_will_not_merge_and_how_to_fix_it() {
        let told = resolving(&watched(), store::ConflictResolution::Merge);

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

    /// And what it is told where the human has asked for a rebase instead: the
    /// other act on the branch, named as the other act, and the force-push it
    /// takes said with the lease that makes it safe to make unattended.
    ///
    /// The strategy has to be *in the words* rather than implied by them. A
    /// session told to merge and left to rebase would rewrite a branch nobody
    /// asked it to, and one told to rebase and left to merge would leave the
    /// history the human configured this for.
    #[test]
    fn a_resolution_session_configured_for_a_rebase_is_told_to_rebase() {
        let told = resolving(&watched(), store::ConflictResolution::Rebase);

        assert!(
            told.contains("#7") && told.contains("askance"),
            "which pull request, in which repository: {told}",
        );
        assert!(
            told.contains("/state/worktrees/rate-limiting-askance"),
            "and the worktree to do it in: {told}",
        );
        assert!(
            told.contains("Rebase the branch") && told.contains("rather than a merge"),
            "the strategy is the one that rewrites the branch, said as what it is \
             and as what it is not: {told}",
        );
        assert!(
            told.contains("--force-with-lease"),
            "and the push a rewritten branch takes, leased rather than outright: \
             {told}",
        );
        assert!(
            told.contains("two changes to reconcile"),
            "and neither side is the one to keep, which is the same either way: \
             {told}",
        );
    }

    /// One recorded pull request of a chain, as [`store::stack`] hands it back.
    fn link(number: i64, head: &str, base: &str) -> store::PullRequest {
        store::PullRequest {
            number,
            title: format!("Stage {number}"),
            url: format!("https://github.com/tobico/askance/pull/{number}"),
            head: Some(head.to_owned()),
            base: Some(base.to_owned()),
            repo: None,
        }
    }

    /// A stack is more than one pull request with a branch on every row, and
    /// the branches come back in the order the rows were in — which is from the
    /// bottom.
    #[test]
    fn a_stack_is_every_branch_of_it_from_the_bottom() {
        let stack = [
            link(5, "stage-01", "main"),
            link(6, "stage-02", "stage-01"),
            link(7, "rate-limiting", "stage-02"),
        ];

        assert_eq!(
            chained(&stack),
            Some(vec!["stage-01", "stage-02", "rate-limiting"]),
        );
    }

    /// A lone pull request is not a stack, and neither is a chain a row of
    /// which never recorded the branch it is on.
    ///
    /// The second is a row written before there were stacks. What a session is
    /// told to sync is a list of branches, so a chain that cannot be written
    /// out as one is a conflict for the configured strategy rather than for the
    /// extension.
    #[test]
    fn a_lone_pull_request_and_a_chain_missing_a_branch_are_not_stacks() {
        assert_eq!(chained(&[link(7, "rate-limiting", "main")]), None);

        let forgotten = [
            store::PullRequest {
                head: None,
                ..link(5, "stage-01", "main")
            },
            link(7, "rate-limiting", "stage-01"),
        ];

        assert_eq!(chained(&forgotten), None);
    }

    /// What a session sent at a stack is told: which pull request will not
    /// merge, the whole chain from the bottom, the worktree all of it is
    /// reached through, and the extension that syncs it rather than a strategy.
    #[test]
    fn a_stack_session_is_told_the_chain_and_to_sync_it() {
        let stack = [
            link(5, "stage-01", "main"),
            link(6, "stage-02", "stage-01"),
            link(7, "rate-limiting", "stage-02"),
        ];
        let branches = chained(&stack).expect("three pull requests, each with a branch");
        let told = syncing(&watched(), &stack, &branches);

        assert!(
            told.contains("#7") && told.contains("askance"),
            "which pull request, in which repository: {told}",
        );
        assert!(
            told.contains("stack of 3")
                && told.contains("#5 (`stage-01`)")
                && told.contains("#6 (`stage-02`)")
                && told.contains("#7 (`rate-limiting`)"),
            "and the whole chain, from the bottom: {told}",
        );
        assert!(
            told.contains("/state/worktrees/rate-limiting-askance"),
            "and the one worktree every branch of it is reached through: {told}",
        );
        assert!(
            told.contains("gh stack sync"),
            "synced rather than merged into branch by branch: {told}",
        );
        assert!(
            told.contains("`stage-01`, `stage-02`, `rate-limiting`")
                && told.contains("bottom first"),
            "the chain is named in the order it is adopted in, bottom branch \
             first: {told}",
        );
        assert!(
            told.contains("git fetch origin") && told.contains("origin's commit"),
            "and this checkout is made to hold every branch of it before \
             anything is adopted, an adoption creating what it cannot find: \
             {told}",
        );
        assert!(
            told.contains("gh stack init stage-01 stage-02 rate-limiting"),
            "and the one command that adopts a chain of that depth, with the \
             whole of it as its argument list: {told}",
        );
        assert!(
            told.contains("gh stack rebase"),
            "and the verb that stops in what the sync backed out on: {told}",
        );
        assert!(
            told.contains("two changes to reconcile"),
            "resolved rather than taken one side of, as every conflict here is: \
             {told}",
        );
        assert!(
            told.contains("tests") && told.contains("pushed"),
            "and the suite run and the chain pushed before it is finished: {told}",
        );
        assert!(
            !told.contains("Merge the pull request's base branch")
                && !told.contains("Rebase the branch"),
            "and nothing of the configured strategy, which a stack is not resolved \
             by: {told}",
        );
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
