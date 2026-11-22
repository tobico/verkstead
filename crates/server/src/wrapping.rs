//! What happens the moment a Conversation's own work is finished: the pull
//! requests the session opened are found, and the Conversation moves into
//! Wrapping.
//!
//! Three endings arrive here, because three kinds of work end on a pull request.
//! A backlog worked to empty ends at its finish step, from Implementing. An
//! inline implementation ends at its one session, from Implementing too — the
//! whole of the work was that session's, so what ends it is the session ending.
//! A roadmap ends at the roadmap commit, from Grilling — the session that
//! settled the work wrote it and carried the branch on without ever leaving the
//! grilling, and there was no Implementing to leave, because on a roadmap the
//! building belongs to the Stages.
//!
//! The push and the pull request are the session's either way. It follows the
//! target repository's own review process — read out of that repository's
//! `docs/agents/git-workflow.md` by the bundled fork it is running inside —
//! because pushing and opening a PR is the project's process rather than
//! Verkstead's. What is Verkstead's is knowing that it happened, and the only way
//! to know is to ask GitHub: an agent's word for it would be the one report it
//! can most easily be wrong about.
//!
//! So this asks the host's `gh` for the PR on the Conversation's branch — see
//! [`crate::github`] — and records what it finds. Recording it *is* the move,
//! which is why nothing here sets a state: the store does both in one
//! transaction, so a Wrapping with no PR under it cannot exist.
//!
//! **On the branch as the remote has it, which is not always the name it has
//! here.** The push belongs to that same review process, so the name it goes
//! under does too: a repository whose branch-naming rule disagrees with the
//! name Verkstead cut is pushed under the rule — `modern-keystone/02-crm` here
//! and `tobi/02-crm` on GitHub — and a pull request's head is the pushed name.
//! Asked under the local one, `gh` answers that there is no pull request at
//! all, which is the one answer below that gets a session spent on it, and the
//! session finds the work already pushed and already on a PR and rightly does
//! nothing. So the branch is asked where it went before GitHub is asked
//! anything — see [`head_ref`], and [`crate::worktrees::pushed_to`] for the
//! reading itself.
//!
//! And then the companions, one repository at a time. A Conversation working
//! alongside read-write repositories ends on one pull request per repository it
//! committed in — the finish sequence the session followed extends to each of
//! them, in that repository's own words — so what moved the Conversation is
//! asked for first and every touched companion is asked about beside it. See
//! [`covering`], which is where *touched* is settled and where a companion left
//! without one stops the run.
//!
//! Nothing waits on approval. There is no gate in front of the finish and none
//! in front of the PRs: merging stays the human act, and everything up to it
//! runs unattended.
//!
//! A `gh` that cannot answer — absent, not logged in, no PR on the branch —
//! stops, leaving the Conversation where it is with the reason on the Timeline
//! as a Notice. That is the honest shape of it: the run has stopped, Verkstead
//! cannot resolve it, and what to do about it is the human's — install `gh`, log
//! in, or open the PR by hand, and resume.
//!
//! With one thing tried first, and only for the one answer a session could do
//! anything about. Every run that ends here commits its work and pushes it
//! afterwards, so any of them can land the whole of what it was sent for and
//! still stop short of the push — which leaves the work built, committed and
//! unreviewable. So *no PR on the branch* is asked for once more, by a session
//! of its own sent to push and open one, and the stop is what is left if that
//! comes back to the same missing thing. A pressed Resume takes the same go
//! rather than the Notice again. See
//! [`crate::activities::one_session::to_a_pull_request`], which is where the
//! deciding is done, and [`record`], which is this module's half of it. The
//! other troubles are walls a session would walk into in the same place, so
//! they stop as they always have.
//!
//! Which is advice Resume then has to be able to take: a pull request opened in
//! a browser is one nothing on the branch knows about, so Resume asks GitHub
//! about it before it spends anything — see [`asked`], and
//! [`crate::activities::one_session::inline_again`] for what an inline run makes
//! of the answer.

use std::path::{Path, PathBuf};

use verkstead_schema::Nudge;

use crate::AppState;
use crate::store;

/// Find the pull request `conversation_id`'s last session opened, and move the
/// Conversation on to wrapping it up.
///
/// `writing` is the Timeline Event that session printed into, so that a stop
/// written here carries the tail of what it last said — which is usually where
/// the reason it opened nothing is written down.
///
/// Nothing is refused for and nothing is returned: this runs at the end of an
/// unattended run with nobody watching, and what it has to say it says on the
/// Timeline.
pub(crate) async fn opened(state: &AppState, conversation_id: i64, writing: Option<i64>) {
    let Some((repo_id, branch, found)) = asked(state, conversation_id).await else {
        return;
    };

    record(
        state,
        conversation_id,
        repo_id,
        &branch,
        found,
        writing,
        Door::TheMove,
    )
    .await
}

/// The same for a Conversation that is wrapping up already: the pull request is
/// written beside the wrap-up rather than being what starts one.
///
/// **A Review taken up over a bare branch**, and nothing else. That one is moved
/// into Wrapping by the take-up itself, there being no pull request to move it —
/// see [`crate::conversations::take_up`] — and the session sent for one runs from
/// inside the wrap-up it is already in. So the record is the row without the move
/// over the top of it, exactly as a companion's pull request is recorded, and the
/// watchers start on what was opened.
///
/// Which is why it is a door rather than a second reading of the state: a
/// Conversation that is Wrapping has nothing left to wrap, so the move would be
/// refused as a second attempt at an ending and the pull request would go
/// unrecorded — and what the human would be looking at is a wrap-up watching
/// nothing.
pub(crate) async fn opened_beside(state: &AppState, conversation_id: i64, writing: Option<i64>) {
    let Some((repo_id, branch, found)) = asked(state, conversation_id).await else {
        return;
    };

    record(
        state,
        conversation_id,
        repo_id,
        &branch,
        found,
        writing,
        Door::Beside,
    )
    .await
}

/// Which door a pull request a session has just opened comes through.
///
/// One thing to record and two ways in, and what tells them apart is where the
/// Conversation already stands rather than anything about the pull request. See
/// [`opened`] and [`opened_beside`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Door {
    /// The move: the Conversation is not wrapping yet, and recording this is what
    /// carries it there — the ending every run comes through.
    TheMove,

    /// Beside a wrap-up that is under way already, which is a take-up over a bare
    /// branch: the row and the Event, and no move.
    Beside,
}

/// Make of an answer `gh` has already given what [`opened`] makes of its own.
///
/// The same move with the asking taken out of it, for the one caller that has
/// to look at the answer before this does: a finish step that left no pull
/// request is sent back to open one, and only a branch GitHub says has none is
/// worth spending a session on — see
/// [`crate::activities::one_session::to_a_pull_request`]. Everything else it
/// may say is this function's to record or to stop over, exactly as it always
/// was, so the deciding stays in one place and the caller hands back what it
/// was given.
///
/// `repo_id` is the registered Repo it was opened in, which is which of a
/// Conversation's pull requests this one is, and `branch` is the branch that was
/// asked about, which is what the stop is logged against. `door` is whether
/// recording this is the move — see [`Door`].
pub(crate) async fn record(
    state: &AppState,
    conversation_id: i64,
    repo_id: i64,
    branch: &str,
    found: Result<store::PullRequest, crate::forges::Trouble>,
    writing: Option<i64>,
    door: Door,
) {
    let opened = match found {
        Ok(opened) => opened,
        Err(crate::forges::Trouble::NoForge(host)) => {
            unforged(state, conversation_id, host.as_deref()).await;
            return;
        }
        Err(trouble) => {
            tracing::warn!(
                conversation_id,
                branch,
                why = trouble.why(),
                "the last session left no pull request Verkstead could find",
            );

            stopped(state, conversation_id, &trouble.why(), writing).await;
            return;
        }
    };

    // The row either way, and the move only through the door that is one — a
    // Conversation that is Wrapping already has nothing left to wrap, so asking
    // for the move there would record nothing at all.
    let recorded = match door {
        Door::TheMove => {
            store::record_pull_request(&state.pool, conversation_id, repo_id, &opened).await
        }
        Door::Beside => {
            store::record_another_pull_request(&state.pool, conversation_id, repo_id, &opened)
                .await
                .map(|there| match there {
                    true => store::Wrapping::Started,
                    false => store::Wrapping::NoSuchConversation,
                })
        }
    };

    match recorded {
        Ok(store::Wrapping::Started) => {
            tracing::info!(
                conversation_id,
                number = opened.number,
                url = opened.url,
                "the work is on a pull request, so the Conversation is wrapping up",
            );

            // The Timeline has a move on it and something new pinned above it, and
            // an open page should say so without being reloaded.
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });

            // And the devices are told, because this is one of the moments the
            // work moved on with nobody watching: the whole run from the Brief
            // to here happened unattended, and the pull request is where the
            // human's own part of it starts. Behind the record, which the store
            // has already taken — a push service that cannot be reached costs a
            // notification and never the pull request.
            // Named in the word the Repo's Forge calls one.
            let noun = match store::registered_repo(&state.pool, repo_id).await {
                Ok(Some(repo)) => crate::forges::of(&state.pool, &repo.path)
                    .await
                    .noun()
                    .to_owned(),
                _ => crate::forges::PULL_REQUEST.to_owned(),
            };

            crate::push::told(
                &state.pool,
                conversation_id,
                crate::push::News::OnAPullRequest {
                    number: opened.number,
                    noun,
                },
            );

            // And the rest of the stack, where this Conversation's Process is one
            // that walks a chain: the same walk a take-up runs at the press, from
            // the other door. What gets here is a **Fix Merge Issues** taken up
            // over a bare branch — there was nothing to walk from at the press,
            // and the `submitting` session it sent has just opened the pull
            // request there is one from now.
            //
            // A Notice of its own rather than a sentence on the take-up's, that
            // one having been written before there was anything to say; and
            // before the watchers, which is what watches what it recorded. See
            // [`crate::stacks`].
            //
            // **And the neighbours make way here too, or the run stops.** Every
            // link of a stack Verkstead built is a Conversation's, each keeping a
            // worktree with its branch checked out, and what a conflict in one
            // dispatches is a session told to run `gh stack sync` — which rebases
            // and force-pushes the whole chain. Nothing can be asked of the human
            // by now, so a neighbour that has finished is closed and one that has
            // not stops the run with a Notice naming it, rather than spending a go
            // on a sync that cannot move the branch. See
            // [`crate::conversations::neighbours_give_way`].
            let stopping = match crate::stacks::walking(state, conversation_id).await {
                Some((repo, own, walked)) => {
                    let neighbours = walked.neighbours(own);

                    let made_way = match neighbours.is_empty() {
                        true => Ok(Vec::new()),
                        false => {
                            crate::conversations::neighbours_give_way(
                                state,
                                conversation_id,
                                &repo,
                                &neighbours,
                            )
                            .await
                        }
                    };

                    // Said either way, because what the chain *is* is worth
                    // recording whether or not the run goes on: a wrap-up stopped
                    // over a neighbour reads as a wrap-up stopped over nothing
                    // unless the Timeline says what the chain was.
                    let said = crate::stacks::record(
                        state,
                        conversation_id,
                        &repo,
                        own,
                        &walked,
                        made_way.as_deref().unwrap_or_default(),
                    )
                    .await;

                    if let Some(said) = said
                        && let Err(error) = store::note(&state.pool, conversation_id, &said).await
                    {
                        tracing::error!(error = ?error, conversation_id, "recording what the stack is failed");
                    }

                    made_way.err()
                }
                None => None,
            };

            if let Some(why) = stopping {
                stopped(state, conversation_id, &why, writing).await;
                return;
            }

            // And the wrap-up itself starts here. The branch has just been
            // pushed, so GitHub is already running the checks and nobody else is
            // going to look — and nobody has read the branch at all.
            //
            // Beside a wrap-up already under way, the Activity is running — it
            // is what launched the session that opened this — and it watches
            // what was opened from here on, so nothing more starts.
            if door == Door::TheMove {
                watching(state, conversation_id, Reviewing::AsFound);
            }
        }
        // The run was stopped from outside while the last step was landing, or
        // this is a second attempt at an ending that already moved the
        // Conversation. Neither is a failure and neither is anything to record
        // twice.
        Ok(store::Wrapping::NothingToWrap) => tracing::info!(
            conversation_id,
            number = opened.number,
            "the Conversation has nothing left to wrap up, so nothing was recorded",
        ),
        Ok(store::Wrapping::NoSuchConversation) => tracing::error!(
            conversation_id,
            "there is no Conversation left to record a pull request against"
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "recording a pull request failed");

            // `{error:#}` rather than `{error}`, because what a Notice is for is
            // the human working out what to do about it. Displayed plainly, an
            // anyhow error is its outermost context and nothing else — *putting
            // a pull request on the Timeline of Conversation 43* — which names
            // the step and withholds the reason it failed. The alternate form
            // carries the chain down to what the database actually said.
            stopped(
                state,
                conversation_id,
                &format!("the pull request could not be recorded: {error:#}"),
                writing,
            )
            .await;
        }
    }
}

/// The pull request one watcher follows: where to ask GitHub about it, what to
/// call it, and where its work is done.
///
/// What both of a wrap-up's per-pull-request watchers need before they can go to
/// the network, and one thing rather than two because it is one question: a
/// Conversation ends on a pull request per repository it was worked in and as
/// many in one repository as its stack is deep, and asking GitHub about one — its
/// checks or what has been said on it — means running `gh` in that repository and
/// sending whoever answers to that repository's checkout. See [`crate::checks`]
/// and [`crate::comments`].
pub(crate) struct Watched {
    /// The registered Repo it was opened in, which is where `gh` is run and what
    /// the feedback and the Notice name it by.
    pub(crate) repo: store::Repo,

    /// The number GitHub gave it, which is what everybody calls it by — in that
    /// repository and nowhere else.
    pub(crate) number: i64,

    /// The branch its work is on, as the record holds it — see
    /// [`store::PullRequest::head`].
    ///
    /// What a green suite is held against: the rollup GitHub reports is a fact
    /// about one commit, and which commit that has to *be* is whatever origin
    /// holds on this pull request's own branch. Read off the row rather than off
    /// the checkout, because a repository wrapping up a stack has several
    /// branches through the one Worktree and the checkout answers for at most
    /// one of them.
    ///
    /// `None` on a row written before Verkstead wrote the head down, which is
    /// the one case the checkout still has to stand in for — see
    /// [`crate::checks`].
    pub(crate) head: Option<String>,

    /// The checkout its branch is on: the Conversation's own worktree, or the
    /// companion's beside it. Where a session sent at it is sent to work.
    pub(crate) worktree: PathBuf,
}

/// Where `opened`, the pull request recorded in `repo_id`, is — and where its
/// work is done.
///
/// The Conversation's own repository and Worktree, or the companion's beside it.
/// `None` where the Conversation has neither — a companion taken off it, or a
/// checkout that is gone — which is a pull request nothing can do anything
/// about.
///
/// The whole recorded pull request rather than its number, because the branch
/// comes with it: which branch this one's work is on is as much a fact off the
/// row as the number is, and neither can be read off a Worktree that a stack
/// shares.
pub(crate) fn watched(
    conversation: &store::Conversation,
    repo_id: i64,
    opened: &store::PullRequest,
) -> Option<Watched> {
    let (repo, worktree) = match conversation.repo.id == repo_id {
        true => (&conversation.repo, conversation.worktree.as_ref()?),
        false => {
            let companion = conversation
                .companions
                .iter()
                .find(|companion| companion.repo.id == repo_id)?;

            (&companion.repo, companion.worktree.as_ref()?)
        }
    };

    Some(Watched {
        repo: repo.clone(),
        number: opened.number,
        head: opened.head.clone(),
        worktree: worktree.clone(),
    })
}

/// The pull request in words, which is its number and the repository it is in.
///
/// Both halves every time: `#7` is a number in one repository and a different
/// pull request or nothing at all in another, and a Conversation now ends on one
/// per repository it was worked in.
pub(crate) fn named(watched: &Watched) -> String {
    format!(
        "pull request #{} of `{}`",
        watched.number, watched.repo.name
    )
}

/// Which review a wrap-up's watchers start.
///
/// The one thing that differs between the ways of starting a wrap-up's watchers,
/// and it differs because Resume is the human saying *have another go at all of
/// it*. The other three watchers each read the record and decide for themselves
/// whichever way this falls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reviewing {
    /// Whatever there is to see to: a branch nobody has read, or a review whose
    /// session is gone, which is a stop. What the finish step, a server coming
    /// back up and the resolve press all mean — see [`crate::review::wanted_now`].
    ///
    /// The press means it for a reason the other two do not, and deliberately:
    /// the branch *was* read, the human read the review, and the work was
    /// carried to Done on the strength of it. A base that moved underneath it
    /// since is not a reason to read the same branch twice, so what the wrap-up
    /// finds is a review already settled and runs nothing for it. See
    /// [`crate::resolving`].
    AsFound,

    /// The branch read from the start, whatever the last review left behind.
    /// What Resume means — see [`crate::review::wanted_now`].
    Afresh,
}

/// Everything a wrapping Conversation has going on: the companions it committed
/// in carried to pull requests of their own, every pull request's checks watched
/// and its comments read, its branch reviewed where nobody has read it yet, and
/// the rule that ends the whole thing waiting to be true.
///
/// One place that says what a wrap-up *is*, because everything that starts one
/// has to start the whole of it: the finish step opening the pull request, a
/// server coming back up over a Conversation it left wrapping, a Resume pressed
/// on a wrap-up that stopped — which stopped the rest too, since nothing
/// advances past an open one — and the press that sends a Done Conversation back
/// here to have a conflict resolved. See [`crate::resume`] for the middle two
/// and [`crate::resolving`] for the last.
///
/// Each of them decides for itself whether there is anything to do, so starting
/// them twice is not starting two of anything: a review that has already settled
/// returns, a second of anything queues on the Worktree behind the first and
/// finds the work done, a companion already covered is read past, and a
/// Conversation that has stopped wrapping up stops every one of them.
///
/// Which is also why a restart and a Resume both come through here rather than
/// picking their own step. What either of them is looking at is a wrap-up with
/// nothing running, and that is one situation with several possible causes: a
/// branch nobody has read, a review whose session went between its ask and the
/// answers, a batch's proposal in the same state, a companion whose pull request
/// the human has since opened by hand. Each of them asks the record what it is
/// looking at rather than being told — see [`covering`], [`crate::review::wanted_now`]
/// and [`crate::responding::unattended`]. All of it is the **Wrap-up**
/// Activity's now, which asks the host each of those things in its turn.
///
/// `reviewing` is the one thing the ways of starting them differ over, and it is
/// a Resume that makes the difference: a review already asking is something to
/// stop over where a server found it, and something to read past where the human
/// has read the Notice and asked for another go. Every other way in — the finish
/// step, a restart, the resolve press — takes the review as found, each for a
/// reason of its own.
pub(crate) fn watching(state: &AppState, conversation_id: i64, reviewing: Reviewing) {
    spawn(state, conversation_id, None, reviewing);
}

/// Start the **Wrap-up** Activity on `conversation_id`, registered as a driver
/// of it for as long as it runs — see [`crate::activities::wrap_up::run`]. What
/// ends a wrap-up is its Outcome.
///
/// `against` is the base a take-up over a bare branch is owed its pull request
/// against, which the Activity opens before it watches anything.
pub(crate) fn wrap_up(state: &AppState, conversation_id: i64, against: Option<String>) {
    spawn(state, conversation_id, against, Reviewing::AsFound);
}

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
    watching(&state, conversation_id, Reviewing::Afresh);
}

fn spawn(state: &AppState, conversation_id: i64, against: Option<String>, reviewing: Reviewing) {
    let driving = state.drivers.driving(conversation_id);

    tokio::spawn(crate::activities::wrap_up::run(
        state.clone(),
        conversation_id,
        against,
        reviewing,
        driving,
    ));
}

/// Find the pull requests the finish opened in the companions it committed in,
/// and record each one beside the Conversation's own.
///
/// The other half of [`opened`], and the reason it is the **Wrap-up**
/// Activity's first step — its `cover` host function — rather than a step
/// inside [`opened`]: what this is looking at is a wrap-up whose companions are not
/// all covered yet, and that is the same situation however it arose — a finish
/// that has just happened, a server coming back up, a human who has opened the
/// missing pull request by hand and pressed Resume. A companion already recorded
/// is read past, so running it twice records nothing twice.
///
/// **Which companions are asked about is [`committed_in`]'s**, shared with the
/// Done signal that is refused over the same thing a session earlier — see
/// [`crate::done`].
///
/// **Each one is recorded as it is found**, and the missing ones are gathered up
/// rather than stopped at. So a wrap-up that cannot find all of them still shows
/// the human the ones that exist, pinned and clickable, while they sort out the
/// one that is missing.
///
/// And a companion that was committed in and left without a pull request stops
/// the run, with a Notice naming the repository — [`stopped`]'s shape, for
/// [`stopped`]'s reason: the work ran and left no pull request, so what is wrong
/// is out here rather than in a driver that went away.
pub(crate) async fn covering(state: AppState, conversation_id: i64) {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to find the companions' pull requests for"
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation whose companions to ask about failed");
            return;
        }
    };

    // The one state anything a wrap-up does belongs to, and the stop every
    // watcher asks about before it goes to the network — see
    // [`crate::stopping::stopped`]. A run that has stopped is one nothing
    // advances past, this included.
    if conversation.state != store::Lifecycle::Wrapping
        || crate::stopping::stopped(&state, conversation_id).await
    {
        return;
    }

    let Some(committed) = committed_in(&state, &conversation).await else {
        return;
    };

    let mut missing = Vec::new();

    for companion in committed {
        // A companion on no Forge has no pull request to be missing: its work
        // is committed, and there is nobody to open one on.
        if let crate::forges::Matched::Unmatched { host } =
            crate::forges::of(&state.pool, &companion.path).await
        {
            tracing::info!(
                conversation_id,
                repo = companion.name,
                host,
                "a companion the work committed in is on no forge, so no pull request is asked for",
            );
            continue;
        }

        let asked = {
            let gh = state.github.as_author(companion.author.as_deref());
            let repo = companion.path.clone();
            let head = companion.head.clone();

            tokio::task::spawn_blocking(move || gh.pull_request(&repo, &head)).await
        };

        let found = match asked {
            Ok(found) => found,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, repo = companion.name, "asking the Forge for a companion's pull request failed");
                return;
            }
        };

        let opened = match found {
            Ok(opened) => opened,
            Err(trouble) => {
                tracing::warn!(
                    conversation_id,
                    repo = companion.name,
                    branch = companion.branch,
                    head = companion.head,
                    why = trouble.why(),
                    "the work committed in a companion Verkstead can find no pull request in",
                );

                missing.push(format!("In `{}`: {}", companion.name, trouble.why()));
                continue;
            }
        };

        match store::record_another_pull_request(
            &state.pool,
            conversation_id,
            companion.repo_id,
            &opened,
        )
        .await
        {
            Ok(true) => {
                tracing::info!(
                    conversation_id,
                    repo = companion.name,
                    number = opened.number,
                    url = opened.url,
                    "a companion the work committed in is on a pull request of its own",
                );

                // The Timeline has something new pinned on it, and an open page
                // should say so without being reloaded. Announced per pull
                // request rather than at the end, because each is recorded as it
                // is found.
                state.nudges.announce(Nudge::Conversation {
                    conversation: conversation_id,
                });
            }
            Ok(false) => tracing::error!(
                conversation_id,
                "there is no Conversation left to record a companion's pull request against"
            ),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, repo = companion.name, "recording a companion's pull request failed");
                return;
            }
        }
    }

    if missing.is_empty() {
        return;
    }

    // Asked again on the far side of the asking, which is where every wait for
    // something slow is: the answers came off a network, and a Conversation
    // steered on or closed in the meantime is no place for a Notice about a
    // pull request nobody is waiting for any more.
    if !still_going(&state, conversation_id).await {
        return;
    }

    // The tail of whatever last ran, which is where a session that opened no
    // pull request usually says why — the same thing [`opened`] is handed by the
    // finish it followed. Read back here rather than passed in, this having been
    // started by a watcher rather than by the step that ended.
    let writing = crate::stalls::said_last(&state, conversation_id).await;

    give_up(
        &state,
        conversation_id,
        GaveUp {
            what: FINDING.to_owned(),
            how: format!(
                "The work committed in a companion repo that has no pull request. {}",
                missing.join(" ")
            ),
            writing,
        },
    )
    .await;
}

/// One companion repository the work has committed in, as the two things
/// anything asking GitHub about it needs: which repository, and the name its
/// branch went to GitHub under.
pub(crate) struct Committed {
    /// The registered Repo, which is which of a Conversation's pull requests a
    /// pull request found here would be.
    pub(crate) repo_id: i64,

    /// And what it is called, which is what a refusal and a Notice name.
    pub(crate) name: String,

    /// Where it is on this machine, which is where `gh` is run.
    pub(crate) path: PathBuf,

    /// And the uuid of the Author it records, which is whose token `gh` is
    /// run with — see [`crate::forges::Forge::as_author`].
    pub(crate) author: Option<String>,

    /// The branch the work is on there, mirroring resolved.
    pub(crate) branch: String,

    /// And the name the remote is carrying that branch under, which is what a
    /// pull request's head is — see [`head_ref`].
    pub(crate) head: String,
}

/// The read-write companions this Conversation has committed in and has no pull
/// request recorded for yet, ready to be asked about.
///
/// **One reading, because its two readers have to agree.** [`covering`] stops
/// the run over a companion left without a pull request, and [`crate::done`]
/// refuses a Done signal over the same thing a session earlier — so a companion
/// the signal let through would be one the wrap-up stops the run over a moment
/// later, which is the second session the signal exists to save.
///
/// **Touched means commits beyond the base**, asked of git in the companion's
/// own repository — see [`crate::commits::touched`]. A read-only companion is
/// not asked at all, and a read-write one with nothing on its branch is ignored
/// by the whole of wrap-up: no pull request expected, nothing recorded, nothing
/// waited on.
///
/// **A pull request recorded already is read past**, which is a finish that ran
/// twice, a server that came back up, or a Resume over a wrap-up that stopped on
/// another companion. Asked before git and before GitHub, both being dearer than
/// a row.
///
/// `None` where the reading itself fell over — a record that would not be read,
/// git that would not answer. Not an empty list: what a caller does about *not
/// knowing* is its own, and the difference is the whole of why this says which
/// it is.
pub(crate) async fn committed_in(
    state: &AppState,
    conversation: &store::Conversation,
) -> Option<Vec<Committed>> {
    let conversation_id = conversation.id;
    let mut committed = Vec::new();

    for companion in &conversation.companions {
        // A read-only companion is detached and bound read-only, so nothing can
        // have landed on it and there is no branch to ask GitHub about.
        if companion.mode != store::CompanionMode::ReadWrite {
            continue;
        }

        // Mirroring resolved, which is the record's own business — see
        // [`store::Companion::branch_for`].
        let Some(branch) = companion.branch_for(&conversation.branch) else {
            continue;
        };

        match store::pull_request(&state.pool, conversation_id, companion.repo.id).await {
            Ok(Some(_)) => continue,
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, repo = companion.repo.name, "reading whether a companion's pull request was recorded failed");
                return None;
            }
        }

        let Some(base) = companion.base_commit.clone() else {
            tracing::error!(
                conversation_id,
                repo = companion.repo.name,
                "a read-write companion has no base commit, so what the work committed \
                 in it cannot be told from what it branched off"
            );
            continue;
        };

        let repo = companion.repo.path.clone();

        // Off the runtime's threads: git is a process.
        let touched = {
            let repo = repo.clone();
            let branch = branch.clone();

            match tokio::task::spawn_blocking(move || {
                crate::commits::touched(&repo, &base, &branch)
            })
            .await
            {
                Ok(touched) => touched,
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, repo = companion.repo.name, "asking git what a companion holds failed");
                    return None;
                }
            }
        };

        if !touched {
            continue;
        }

        // Under whatever name that repository's remote is carrying the branch,
        // which is the Conversation's own question asked again per companion —
        // see [`head_ref`]. A companion's finish sequence is that repository's,
        // so its naming rule is that repository's too, and two repositories may
        // well disagree about what one Conversation's branch is called.
        let head = head_ref(conversation_id, &repo, &branch).await;

        committed.push(Committed {
            repo_id: companion.repo.id,
            name: companion.repo.name.clone(),
            path: repo,
            author: companion.repo.author.clone(),
            branch,
            head,
        });
    }

    Some(committed)
}

/// Whether the Conversation is still wrapping up, which is the only state
/// anything a wrap-up dispatches belongs to.
///
/// Asked on the far side of every wait for the Worktree: a Conversation closed
/// while something queued has nowhere left to work, and a queue is where most of
/// a wrap-up's waiting happens.
///
/// A store that will not answer reads as *no*, which is the right way round for
/// the one thing this decides: on the other side of it is an agent being let
/// loose in a Worktree.
pub(crate) async fn still_going(state: &AppState, conversation_id: i64) -> bool {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state == store::Lifecycle::Wrapping,
        Ok(None) => {
            tracing::error!(conversation_id, "there is no Conversation left wrapping up");
            false
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a Conversation was wrapping up failed");
            false
        }
    }
}

/// Leave the Conversation where it is, with the reason on the Timeline.
///
/// A stop rather than a line of its own, because that is exactly what this is: a
/// run that has stopped on something Verkstead cannot resolve itself. Resume
/// once `gh` is logged in, or open the pull request by hand, or close the
/// Conversation.
///
/// [`store::Decision::Verkstead`]: the work ran and left no pull request, so what
/// is wrong is out here rather than in a driver that went away, and a restart
/// looking again would find the same missing thing.
///
/// Which is also why Resume on an inline implementation stops here rather than
/// launching over it — see [`crate::activities::one_session::inline_again`]. A
/// `gh` that cannot answer now is
/// one the session would push into later, so the account it would spend is
/// spent on the same missing thing.
pub(crate) async fn stopped(
    state: &AppState,
    conversation_id: i64,
    why: &str,
    writing: Option<i64>,
) {
    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        FINDING,
        why,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a finish step left no pull request and the stop saying so could not be recorded"
        );
    }
}

/// Land the work of `conversation_id` in Done, its Repo being on no Forge:
/// nothing is watching, because there is nothing to watch, and the Timeline
/// says why — see [`crate::forges::unmatched`].
///
/// The ending [`record`] makes for every other Repo, reached the same way: a
/// Repo whose remote's host no enabled Forge declares has no wrap-up, and the
/// next Conversation on it wraps up once a Forge answers for that host.
pub(crate) async fn unforged(state: &AppState, conversation_id: i64, host: Option<&str>) {
    let said = crate::forges::unmatched(host);

    match store::land_on_no_forge(&state.pool, conversation_id, &said).await {
        Ok(store::Unforged::Done) => {
            tracing::info!(
                conversation_id,
                host,
                "the work is finished on a repo no forge answers for, so it is done with \
                 nothing watching",
            );

            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });
        }
        Ok(store::Unforged::NothingToEnd | store::Unforged::NoSuchConversation) => {
            tracing::info!(
                conversation_id,
                "the Conversation has nothing left to end, so nothing was recorded",
            );
        }
        Err(error) => tracing::error!(
            error = ?error,
            conversation_id,
            "landing a Conversation on no forge in Done failed"
        ),
    }
}

/// What a stop over a pull request nobody can find says the run was doing.
const FINDING: &str = "finding the pull request the work ended on";

/// The stop a host function gave a wrap-up up with — what the watchers wrote
/// themselves before the wrap-up was an Activity, and what is written now as
/// the Activity's `given-up` lands.
pub(crate) struct GaveUp {
    /// What the run was doing, which the stop's Notice opens with.
    pub(crate) what: String,

    /// And why it stopped, in the Notice's words.
    pub(crate) how: String,

    /// The Timeline Event whose tail is its evidence.
    pub(crate) writing: Option<i64>,
}

/// Which Conversations have a **Wrap-up** Activity running, and the stops the
/// host functions have given each of them up with that the Activity has not
/// answered yet.
///
/// The host gives up where the component asks it to — the checks or a conflict
/// out of goes — or where `cover` finds a companion left without a pull
/// request, and the Activity answers `given-up` on its next round, which
/// [`given_up`] maps to the stop as it always stood. Where no Activity is
/// running to answer, the stop is written at once.
#[derive(Clone, Default)]
pub(crate) struct WrapUps {
    held: std::sync::Arc<std::sync::Mutex<WrapUpsHeld>>,

    /// Woken as a stop is handed over, so a round sleeping out its poll looks
    /// again at once.
    handed: std::sync::Arc<tokio::sync::Notify>,
}

#[derive(Default)]
struct WrapUpsHeld {
    running: std::collections::HashMap<i64, usize>,
    given_up: std::collections::HashMap<i64, GaveUp>,
}

/// Counts a running wrap-up for as long as it is held.
pub(crate) struct Running {
    wrap_ups: WrapUps,
    conversation_id: i64,
}

impl Drop for Running {
    fn drop(&mut self) {
        let mut held = self.wrap_ups.lock();

        if let Some(running) = held.running.get_mut(&self.conversation_id) {
            *running -= 1;

            if *running == 0 {
                held.running.remove(&self.conversation_id);
            }
        }
    }
}

impl WrapUps {
    pub(crate) fn new() -> WrapUps {
        WrapUps::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, WrapUpsHeld> {
        self.held.lock().expect("nothing panics holding it")
    }

    /// Count a wrap-up running on `conversation_id` until what this answers is
    /// dropped.
    pub(crate) fn running(&self, conversation_id: i64) -> Running {
        *self.lock().running.entry(conversation_id).or_default() += 1;

        Running {
            wrap_ups: self.clone(),
            conversation_id,
        }
    }

    /// Whether a stop is held for `conversation_id`'s wrap-up to answer.
    pub(crate) fn given_up(&self, conversation_id: i64) -> bool {
        self.lock().given_up.contains_key(&conversation_id)
    }

    /// Woken the next time a stop is handed over.
    pub(crate) async fn handed(&self) {
        self.handed.notified().await
    }

    /// Hold `gave_up` for a running wrap-up, or hand it back where none is.
    fn hand_over(&self, conversation_id: i64, gave_up: GaveUp) -> Option<GaveUp> {
        {
            let mut held = self.lock();

            if !held.running.contains_key(&conversation_id) {
                return Some(gave_up);
            }

            held.given_up.insert(conversation_id, gave_up);
        }

        self.handed.notify_waiters();
        None
    }

    fn take(&self, conversation_id: i64) -> Option<GaveUp> {
        self.lock().given_up.remove(&conversation_id)
    }

    /// Let go of a stop held for a wrap-up that ended under a stop of its own.
    pub(crate) fn let_go(&self, conversation_id: i64) {
        self.take(conversation_id);
    }
}

/// Give `conversation_id`'s wrap-up up with `gave_up`: handed to the running
/// Activity to answer `given-up` over, or written at once where none is.
pub(crate) async fn give_up(state: &AppState, conversation_id: i64, gave_up: GaveUp) {
    if let Some(gave_up) = state.wrap_ups.hand_over(conversation_id, gave_up) {
        write_the_stop(state, conversation_id, gave_up).await;
    }
}

/// Land a wrap-up's `given-up`: the stop the host gave it up with, as it was
/// always written. Nothing where another start of it has written it already.
pub(crate) async fn given_up(state: &AppState, conversation_id: i64) {
    if let Some(gave_up) = state.wrap_ups.take(conversation_id) {
        write_the_stop(state, conversation_id, gave_up).await;
    }
}

async fn write_the_stop(state: &AppState, conversation_id: i64, gave_up: GaveUp) {
    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        &gave_up.what,
        &gave_up.how,
        gave_up.writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a wrap-up was given up and the stop saying so could not be recorded"
        );
    }
}

/// Ask the host's `gh` what pull request `conversation_id`'s branch has, and
/// hand back the repository it asked in and the branch it asked about alongside
/// the answer.
///
/// Whether there is one is a question two phases ask. [`opened`] asks it at the
/// end of a run, where a pull request is what the work was carried to, and
/// Resume on an inline implementation asks it before spending a session,
/// because a branch that is already on a pull request has nothing left to
/// implement — see [`crate::activities::one_session::inline_again`]. What each
/// makes of the answer is its own,
/// so the answer is what this hands back rather than anything it does about it.
///
/// The Repo comes back with it because a pull request is recorded against one:
/// two repositories are two sets of numbers, and this is the Conversation's own.
///
/// `None` where there was nothing to ask about or the asking itself fell over,
/// both of which are in the log already: neither leaves a caller anything to
/// say.
pub(crate) async fn asked(
    state: &AppState,
    conversation_id: i64,
) -> Option<(
    i64,
    String,
    Result<store::PullRequest, crate::forges::Trouble>,
)> {
    let (repo_id, repo, author, branch) = branch(state, conversation_id).await?;
    let head = head_ref(conversation_id, &repo, &branch).await;

    // On no Forge, nothing is asked at all: there is nobody to ask, and what
    // meets this lands the work Done rather than stopping it — see [`record`].
    if let crate::forges::Matched::Unmatched { host } = crate::forges::of(&state.pool, &repo).await
    {
        return Some((repo_id, head, Err(crate::forges::Trouble::NoForge(host))));
    }

    let asked = {
        let gh = state.github.as_author(author.as_deref());
        let head = head.clone();

        // Off the runtime's threads: this is a process, and one that goes to the
        // network.
        tokio::task::spawn_blocking(move || gh.pull_request(&repo, &head)).await
    };

    match asked {
        Ok(found) => Some((repo_id, head, found)),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking the Forge for a pull request failed");
            None
        }
    }
}

/// Which name to ask GitHub about for `branch` in `repo`: the one the remote is
/// carrying it under, where that is not the name it has here.
///
/// A pull request's head is a branch on GitHub, and the branch on GitHub is
/// whatever the push named — which is the pushing session's to decide, because
/// the push follows the target repository's own review process rather than
/// Verkstead's. A repository whose branch-naming rule disagrees with the name
/// Verkstead cut gets both: the work on `modern-keystone/02-crm` here, and the
/// pull request on `tobi/02-crm` there. Asked under the local name, `gh` says
/// there is no pull request at all, which sends a finish step to open one that
/// already exists and then stops the run over the same missing thing.
///
/// So the branch is asked where it went first — see [`crate::worktrees::pushed_to`]
/// — and the local name stands wherever it has nothing to say. Which covers the
/// ordinary case twice over: a branch pushed under its own name answers with
/// that name, and a branch not pushed at all answers with nothing, and both are
/// the name that was already going to be used.
///
/// Cheap enough to ask every time: one `git for-each-ref` in the repository,
/// against a `gh` that goes to the network.
async fn head_ref(conversation_id: i64, repo: &Path, branch: &str) -> String {
    let pushed = {
        let repo = repo.to_owned();
        let branch = branch.to_owned();

        // Off the runtime's threads: git is a process too.
        tokio::task::spawn_blocking(move || crate::worktrees::pushed_to(&repo, &branch)).await
    };

    let pushed = match pushed {
        Ok(pushed) => pushed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading where a branch pushes to failed");
            None
        }
    };

    match pushed {
        Some(head) if head != branch => {
            tracing::info!(
                conversation_id,
                branch,
                head,
                "the branch is on its remote under another name, and that is the name \
                 GitHub is asked about",
            );

            head
        }
        _ => branch.to_owned(),
    }
}

/// Which registered Repo to ask `gh` in, where it is on disk, the Author it is
/// asked as, and which branch to ask about.
///
/// The repository rather than the Worktree, exactly as the branch watcher asks
/// it: the remotes and the refs are the repository's, and a Worktree may have
/// been removed by the time this runs.
async fn branch(
    state: &AppState,
    conversation_id: i64,
) -> Option<(i64, PathBuf, Option<String>, String)> {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => Some((
            conversation.repo.id,
            conversation.repo.path,
            conversation.repo.author,
            conversation.branch,
        )),
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to find a pull request for"
            );
            None
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to ask the Forge about failed");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gave_up(how: &str) -> GaveUp {
        GaveUp {
            what: "getting the checks green".to_owned(),
            how: how.to_owned(),
            writing: None,
        }
    }

    /// With no Wrap-up running to answer `given-up`, the stop comes straight
    /// back to be written, as it always was.
    #[test]
    fn a_give_up_with_no_wrap_up_running_is_handed_back() {
        let wrap_ups = WrapUps::new();

        let handed_back = wrap_ups.hand_over(7, gave_up("A check is still failing."));

        assert_eq!(
            handed_back.map(|gave_up| gave_up.how).as_deref(),
            Some("A check is still failing.")
        );
        assert!(!wrap_ups.given_up(7));
    }

    /// A running one holds it until its Outcome lands, once, and only for its
    /// own Conversation.
    #[test]
    fn a_running_wrap_up_holds_the_stop_until_its_outcome_takes_it() {
        let wrap_ups = WrapUps::new();
        let running = wrap_ups.running(7);

        assert!(
            wrap_ups
                .hand_over(7, gave_up("A check is still failing."))
                .is_none()
        );
        assert!(wrap_ups.given_up(7));
        assert!(!wrap_ups.given_up(8));

        assert_eq!(
            wrap_ups.take(7).map(|gave_up| gave_up.how).as_deref(),
            Some("A check is still failing.")
        );
        assert!(wrap_ups.take(7).is_none(), "taken once");

        drop(running);
        assert!(
            wrap_ups.hand_over(7, gave_up("Again.")).is_some(),
            "and counted no longer once it ends"
        );
    }
}
