//! Starting a Conversation and changing what the human may change about one:
//! everything between a workbench form and a row in the store.
//!
//! Two of the three edits are decided against the repository rather than taken
//! on trust, and git is the one asked both times — whether a name is one it
//! would take for a branch, and whether the repository has the branch the work
//! is to come off. Refused here rather than at grill start, where a bad name or
//! a branch that is not there would be a failure with nobody watching.
//!
//! Starting the grilling is where a Conversation stops being a record and gets
//! somewhere to work — see [`start_grilling`] — and where the session that does
//! the work is launched in it. Adopting is the same moment by the other door:
//! a roadmap Verkstead did not write has its next stage started here, with the
//! human's press standing in for the predecessor that would otherwise have
//! started it — see [`adopt`]. Closing is where both are given back: the
//! session ends, and then the worktree goes.

use std::path::{Path, PathBuf};

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{
    Adopted, BaseRecorded, BranchRenamed, BriefSaved, CompanionAdded, CompanionRemoved,
    ConversationClosed, GrillingStarted, PairingView, Started, Worktree,
};
use verkstead_schema::{Direction, Nudge};

use crate::AppState;
use crate::handoffs::Handoffs;
use crate::repos::git;
use crate::skills;
use crate::stages::Startable;
use crate::store;
use crate::worktrees;

/// Start a Conversation against a registered Repo, on a branch name nobody has
/// had to think of yet.
///
/// The name is the server's because the record is: a prefill the browser
/// invented would be a name the server never saw, and the human may well leave
/// it as it is.
///
/// The two Pairings are prefilled the same way, off what the Repo was last
/// grilled with — see [`prefill`].
pub(crate) async fn start(state: &AppState, repo_id: i64) -> Result<Started> {
    Ok(
        match store::start_conversation(&state.pool, repo_id, &branch_name()).await? {
            Some(id) => {
                prefill(state, id, repo_id).await;
                Started::Started { id }
            }
            None => Started::NoSuchRepo,
        },
    )
}

/// Start a Conversation to adopt `roadmap` in a registered Repo with.
///
/// The same start as any other, with the roadmap written beside it: that mark
/// is what draws the adoption-shaped page, and it is the only thing about the
/// roadmap Verkstead keeps. The branch name is the server's here too, and it is
/// discarded at the press — a stage is worked on its own slug — so what it does
/// is name the row in the sidebar until then.
///
/// The roadmap is taken as the notice gave it. Whether it is still there with a
/// stage to start is a question about a repository at a commit, and it is asked
/// where the page is drawn and asked again when Adopt is pressed: a roadmap
/// somebody finished between the notice and the click is a thing to say on the
/// page rather than a start to refuse.
pub(crate) async fn start_adopting(
    state: &AppState,
    repo_id: i64,
    roadmap: &str,
) -> Result<Started> {
    Ok(
        match store::start_adoption(&state.pool, repo_id, &branch_name(), roadmap).await? {
            Some(id) => {
                prefill(state, id, repo_id).await;
                Started::Started { id }
            }
            None => Started::NoSuchRepo,
        },
    )
}

/// Fill a new Conversation's two pickers with what its Repo was last grilled
/// with.
///
/// A default and not a lock: both are still the human's to change, and changing
/// one before pressing Start Grilling is what the Repo remembers next — the
/// memory is written from the Conversation at grill start, whatever it says by
/// then.
///
/// **Each half is judged before it is applied**, against the same reading the
/// pane gives a chosen Pairing: a Profile whose pair has gone, or which no
/// longer lists the model it was remembered with, is a Pairing that would fail
/// to start a session, and a picker prefilled with one would be worse than a
/// picker left empty. What does not survive the judging is simply not applied,
/// which leaves that picker exactly as a Repo with no memory leaves it.
///
/// Nothing here refuses the start. The Conversation exists by the time this
/// runs, and answering the button with a failure would say that it does not —
/// so a memory that could not be read is logged and the human gets the empty
/// pickers they would have got anyway.
async fn prefill(state: &AppState, id: i64, repo_id: i64) {
    if let Err(error) = remembered(state, id, repo_id).await {
        tracing::warn!(
            error = ?error,
            conversation_id = id,
            "the Repo's remembered Pairings could not be applied to a new Conversation"
        );
    }
}

/// What [`prefill`] does, with somewhere for a store error to go.
async fn remembered(state: &AppState, id: i64, repo_id: i64) -> Result<()> {
    let remembered = store::remembered_pairings(&state.pool, repo_id).await?;

    if let Some((profile_id, model)) = usable(&state.watched, remembered.grilling).await? {
        store::set_grilling_pairing(&state.pool, id, profile_id, Some(&model)).await?;
    }

    if let Some((profile_id, model)) = usable(&state.watched, remembered.implementation).await? {
        store::set_implementation_pairing(&state.pool, id, profile_id, Some(&model)).await?;
    }

    Ok(())
}

/// A remembered Pairing as something to prefill a picker with, or `None` where
/// it is not one any more.
///
/// Read as a row rather than trusted as a pair of ids, which is the reading
/// [`start_grilling`] gives the Pairings it is about to launch under: whether
/// the Profile's pair is still where it was left is a question for the Watched
/// Paths, and whether it still lists the model is a question for the Profile's
/// own list.
async fn usable(
    watched: &crate::watched::WatchedPaths,
    remembered: Option<store::Pairing>,
) -> Result<Option<(i64, String)>> {
    let Some(model) = remembered
        .as_ref()
        .and_then(|pairing| pairing.model.clone())
    else {
        return Ok(None);
    };

    let Some(pairing) = crate::profiles::pairing(watched, remembered).await? else {
        return Ok(None);
    };

    Ok(
        (pairing.profile.broken.is_none() && pairing.profile.models.contains(&model))
            .then_some((pairing.profile.id, model)),
    )
}
/// Finish what answering a Question Set started, and say what it did to the
/// Conversation it was asked from.
///
/// The store hands back what happened and says nothing about it — see
/// [`store::Taken`] — so this is where a proposal settled becomes a line in the
/// log. All of the outcomes are unremarkable but one, which cannot happen — and
/// that is exactly why it is worth saying when it does.
///
/// An accepted proposal settles the Direction, and the session that proposed is
/// the one that produces what it asked for.
///
/// **Whatever a pick asks for is written there, by the session that proposed.**
/// That session is idling on the ask and the Response is on its way back to it,
/// so nothing is ended and nothing is launched: what Verkstead does is arm the
/// session already running, which sees the artifact out and carries the
/// Conversation on from there — see [`write_the_artifact`]. The Conversation
/// stays grilling until then, because the grilling is what is still happening.
///
/// Which artifact differs, and inline's is the handoff itself: the work runs
/// under the other Profile in a session of its own, so everything the grilling
/// settled has to be written down before it ends — and written *after* the pick,
/// shaped by whatever the human said beside it.
///
/// Nothing here is refused for: by the time this runs the Response is stored and
/// the store has recorded the pick. What a session that could not be picked up
/// leaves behind is something to see in the log, and no more than that. A stop
/// is written about a run that stopped — see [`crate::stopping`] — and this is not
/// one.
pub(crate) async fn settle_a_proposal(
    state: &AppState,
    set_id: i64,
    proposed: Option<store::Proposed>,
) {
    use store::{Directing, Proposed};

    let picked = match proposed {
        // The ordinary Set, carrying no proposal at all — which is nearly every
        // Set a grilling asks.
        None => return,
        Some(Proposed::SentBack) => {
            tracing::info!(
                set_id,
                "a wrap-up proposal was not accepted; the grilling carries on with the Response"
            );
            return;
        }
        Some(Proposed::Accepted {
            direction,
            directing: Directing::Writing,
        }) => {
            tracing::info!(
                set_id,
                ?direction,
                "a wrap-up proposal was accepted with a direction picked on it"
            );
            direction
        }
        Some(Proposed::Accepted {
            directing: Directing::NotGrilling,
            ..
        }) => {
            tracing::debug!(
                set_id,
                "a wrap-up proposal was accepted for a Conversation that had already left grilling"
            );
            return;
        }
        Some(Proposed::Accepted {
            directing: Directing::NoSuchConversation,
            ..
        }) => {
            tracing::error!(
                set_id,
                "a wrap-up proposal names a Conversation that is not there, so nothing was armed"
            );
            return;
        }
    };

    // Which Conversation the grilling was of. Read back rather than passed in:
    // one of the two endpoints that take a Response knows and the other does
    // not, and a Set is on exactly one Timeline either way.
    let conversation_id = match store::asked_from(&state.pool, set_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::error!(set_id, "an accepted proposal is on no Timeline");
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, set_id, "reading which Conversation a proposal was asked from failed");
            return;
        }
    };

    if !write_the_artifact(state, conversation_id, picked).await {
        // A Conversation grilling with nothing grilling it, which is a thing to
        // see in the log: the pick is recorded, so the human's answer stands, and
        // there is nothing here to stop over — no session ran and went wrong.
        tracing::error!(
            conversation_id,
            ?picked,
            "the grilling session is not running, so nothing is watching for what the pick asked for"
        );
    }
}

/// Arm the watcher that follows the grilling session as it writes the picked
/// Direction's artifact.
///
/// Nothing is started: the session is the one that proposed, it is idling on the
/// blocking ask the Response is being delivered through, and it goes on from
/// there with the whole thread still in its context. What is armed here is the
/// watcher — the artifact landing, plus quiet, is what ends the session and moves
/// the Conversation on. Which artifact each direction ends on is
/// [`crate::runner::follow_the_tail`]'s.
///
/// **Armed through the register, so exactly one watcher is live.** A pick lets
/// the agent proceed and never makes it: it may come back with another Set
/// instead, with a fresh proposal on it if it wants the direction reconsidered,
/// and a pick on that one supersedes. The watcher the earlier pick armed is
/// watching for the wrong artifact from that moment, so arming cancels it — see
/// [`crate::followers`].
///
/// Whether a watcher was armed. `false` is a Conversation with no session
/// running, which has nothing to arm one *on*: the pick is recorded, so the
/// human's answer stands, and what to make of a pick with nothing grilling it is
/// said by the caller rather than here.
async fn write_the_artifact(state: &AppState, id: i64, direction: Direction) -> bool {
    let Some(session) = state.sessions.following(id) else {
        return false;
    };

    // Taken here rather than inside the watcher, so the registration is on
    // before the task that holds it exists: a watcher is what follows a grilling
    // session, and the whole of what carries the Conversation from the pick
    // through the move that follows it — see [`crate::drivers`].
    let driving = state.drivers.driving(id);

    state.followers.arm(
        id,
        tokio::spawn(crate::runner::follow_the_tail(
            state.clone(),
            id,
            direction,
            session,
            driving,
        )),
    );

    true
}

/// Record that the grilling is over and the work is being built.
///
/// Called once a grilling's own tail has landed and the session has been seen
/// out — the backlog a task list writes, or the handoff an inline build is
/// primed from — which is the one moment the artifact it was writing is
/// certainly finished.
///
/// A roadmap tail has no counterpart here. Its session goes on past the artifact
/// to the pull request, and what records *that* is the move it makes — see
/// [`crate::wrapping::opened`].
///
/// Nothing is refused for: the artifact is in hand by the time this runs, and a
/// move that would not record is something to see in the log.
pub(crate) async fn grilling_over(state: &AppState, id: i64) {
    match store::start_implementing(&state.pool, id).await {
        Ok(store::Implementing::Started) => {
            tracing::info!(
                conversation_id = id,
                "a grilling's tail has landed, so the work is being built"
            )
        }
        Ok(store::Implementing::NotGrilling) => tracing::debug!(
            conversation_id = id,
            "a grilling's tail landed for a Conversation that had already left grilling"
        ),
        Ok(store::Implementing::NoSuchConversation) => tracing::error!(
            conversation_id = id,
            "a grilling's tail landed for a Conversation that is not there"
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "the work could not be recorded as being built")
        }
    }
}

/// Put the row that says the backlog landed on the Conversation's Timeline.
///
/// Called where the runner sees the landing, which is the moment it moves the
/// Conversation on: the plan commit is what puts `.tasks/` under version
/// control, and from here there is a list to work through. What the row fixes is
/// the position — the card drawn at it is read off the Worktree when somebody
/// looks, exactly as the pinned one is.
///
/// Refusing for nothing, and its own call rather than part of the move beside
/// it. A Conversation implementing without a row is a Conversation whose backlog
/// landed before there were rows — which is every one of them from before this,
/// and none of them is broken by it.
pub(crate) async fn backlog_landed(state: &AppState, id: i64) {
    landed(id, store::record_backlog(&state.pool, id).await, "backlog")
}

/// And the row that says the roadmap landed, which is the same thing one level
/// up: the staging session committed `docs/roadmaps/`, and the stages it names
/// are what the effort is against from here.
pub(crate) async fn roadmap_landed(state: &AppState, id: i64) {
    landed(id, store::record_roadmap(&state.pool, id).await, "roadmap")
}

/// What the two above do with what the store answered, which is the same thing
/// twice: say what happened, and carry on either way.
fn landed(id: i64, stamped: Result<store::Landed>, what: &str) {
    match stamped {
        Ok(store::Landed::Stamped) => {
            tracing::info!(
                conversation_id = id,
                what,
                "a list has landed, so it is on the record"
            )
        }
        Ok(store::Landed::Already) => tracing::debug!(
            conversation_id = id,
            what,
            "a list landed a second time, and the record already says where it landed"
        ),
        Ok(store::Landed::NoSuchConversation) => tracing::error!(
            conversation_id = id,
            what,
            "a list landed for a Conversation that is not there"
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, what, "a list landing could not be recorded")
        }
    }
}

/// Put the handoff the grilling wrote on the Timeline, refusing for nothing —
/// which is [`take_handoff`] with the one thing its caller does about a failure
/// done where the reason for it is.
///
/// The inline tail alone, because inline is the one direction whose work crosses
/// into a session of its own: what a task list or a roadmap settled is the
/// artifact it committed, read by whoever comes next out of the repository. And
/// it is called where the Conversation is about to move, so a handoff that could
/// not be read is a thing to see in the log rather than something to stop for —
/// what it costs is the document beside the work, and the work itself is where
/// it is either way.
pub(crate) async fn hand_over(state: &AppState, id: i64) {
    if let Err(error) = take_handoff(state, id).await {
        tracing::error!(error = ?error, conversation_id = id, "the handoff could not be put on the Timeline");
    }
}

/// Take the handoff document the grilling wrote and put it on the Timeline.
///
/// Taken after the session has ended, so that what is read is a finished
/// document rather than one still being written. A grilling that never wrote one
/// leaves nothing to take, which is a thing to note and not a failure — see
/// [`crate::handoffs::Handoffs::take`].
async fn take_handoff(state: &AppState, conversation_id: i64) -> Result<()> {
    let handoffs = Handoffs::under(&state.data_dir);

    let written = tokio::task::spawn_blocking(move || handoffs.take(conversation_id)).await?;

    let Some(written) = written else {
        tracing::warn!(
            conversation_id,
            "a grilling ended without writing a handoff, so what follows has the Brief alone"
        );
        return Ok(());
    };

    if !store::record_handoff(&state.pool, conversation_id, &written).await? {
        tracing::error!(
            conversation_id,
            "a handoff was written for a Conversation that is not there"
        );
    }

    Ok(())
}

/// Save what the human has written into the Brief.
pub(crate) async fn save_brief(pool: &SqlitePool, id: i64, markdown: &str) -> Result<BriefSaved> {
    Ok(match store::save_brief(pool, id, markdown).await? {
        store::Edited::Saved => BriefSaved::Saved,
        store::Edited::NoSuchConversation => BriefSaved::NoSuchConversation,
        store::Edited::NotDrafting => BriefSaved::NotDrafting,
    })
}

/// Name the branch the work will be done on.
///
/// Whether the name is usable is git's to say, not a list of forbidden
/// characters here: the branch this names is one git will be asked to create,
/// and the only opinion that will matter then is the one being asked now.
pub(crate) async fn rename_branch(
    pool: &SqlitePool,
    id: i64,
    branch: &str,
) -> Result<BranchRenamed> {
    let branch = branch.trim().to_owned();

    if !tokio::task::spawn_blocking({
        let branch = branch.clone();
        move || is_branch_name(&branch)
    })
    .await?
    {
        return Ok(BranchRenamed::NotABranchName);
    }

    Ok(match store::rename_branch(pool, id, &branch).await? {
        store::Edited::Saved => BranchRenamed::Renamed,
        store::Edited::NoSuchConversation => BranchRenamed::NoSuchConversation,
        store::Edited::NotDrafting => BranchRenamed::NotDrafting,
    })
}

/// Record the branch the work comes off, or put the Conversation back on the
/// default-branch rule.
///
/// What is stored is the name, not the commit it stands at: the choice is one of
/// the repository's branches, and what the human means by picking one is *come
/// off whatever is on it when this starts* — so it is resolved at grill start
/// and not before. Blank counts as clearing it — a choice unmade is the human
/// taking the override away, not naming a branch called nothing.
///
/// Refused unless the repository really has a branch by that name, asked of the
/// branches themselves rather than of `rev-parse`: a sha or a tag resolves and
/// is still not something this stores, there being no way to pick one.
pub(crate) async fn set_base_branch(
    pool: &SqlitePool,
    id: i64,
    asked: Option<&str>,
) -> Result<BaseRecorded> {
    let asked = asked.map(str::trim).filter(|asked| !asked.is_empty());

    if let Some(branch) = asked {
        // The repository to ask is the Conversation's own, so the Conversation
        // has to be there before there is anywhere to ask.
        let Some(conversation) = store::load_conversation(pool, id).await? else {
            return Ok(BaseRecorded::NoSuchConversation);
        };

        // Past drafting is answered here rather than by the store below, so that
        // a Conversation whose base was frozen months ago is told *that* rather
        // than told about a branch the repository has since lost. The store
        // asks again all the same: this read and that write are not one moment.
        if conversation.state != store::Lifecycle::Draft {
            return Ok(BaseRecorded::NotDrafting);
        }

        let branch = branch.to_owned();
        let known = tokio::task::spawn_blocking(move || {
            worktrees::branches(&conversation.repo.path).contains(&branch)
        })
        .await?;

        if !known {
            return Ok(BaseRecorded::NoSuchBranch);
        }
    }

    Ok(match store::set_base_commit(pool, id, asked).await? {
        store::Edited::Saved => BaseRecorded::Recorded,
        store::Edited::NoSuchConversation => BaseRecorded::NoSuchConversation,
        store::Edited::NotDrafting => BaseRecorded::NotDrafting,
    })
}

/// Add a registered Repo for the work to run alongside.
///
/// Thin over the store, and deliberately so: what a companion may be is decided
/// where the rows are — its own Repo and one already added are questions about
/// the table rather than about git — and nothing is made on disk until grilling
/// starts.
pub(crate) async fn add_companion(
    pool: &SqlitePool,
    id: i64,
    repo_id: i64,
) -> Result<CompanionAdded> {
    Ok(match store::add_companion(pool, id, repo_id).await? {
        store::Adding::Added => CompanionAdded::Added,
        store::Adding::NoSuchConversation => CompanionAdded::NoSuchConversation,
        store::Adding::NotDrafting => CompanionAdded::NotDrafting,
        store::Adding::NoSuchRepo => CompanionAdded::NoSuchRepo,
        store::Adding::OwnRepo => CompanionAdded::OwnRepo,
        store::Adding::AlreadyAdded => CompanionAdded::AlreadyAdded,
    })
}

/// And take one away again, for as long as the Conversation is still drafting.
pub(crate) async fn remove_companion(
    pool: &SqlitePool,
    id: i64,
    repo_id: i64,
) -> Result<CompanionRemoved> {
    Ok(match store::remove_companion(pool, id, repo_id).await? {
        store::Removing::Removed => CompanionRemoved::Removed,
        store::Removing::NoSuchConversation => CompanionRemoved::NoSuchConversation,
        store::Removing::NotDrafting => CompanionRemoved::NotDrafting,
    })
}

/// Give a drafting Conversation somewhere to work: a branch off its base commit
/// and a worktree of its Repo, and the move onto the Timeline that says so.
///
/// Everything that has to be true is checked here, each refused by its own name,
/// because each is something different for the human to go and do. They are
/// checked in the order they can be: the record first, then the Profiles, then
/// the Brief, then the repository — so the answers that cost a git call are
/// asked only once the cheap ones have passed.
///
/// The order of the doing is the other way round from the recording. Git makes
/// the branch and the worktree, and only then does the store hear about it: a
/// row saying a Conversation is grilling when nothing was ever checked out would
/// be a Conversation nothing could start and nothing would clean up. The reverse
/// — a worktree on disk that the store does not know about — is a directory to
/// tidy, which is the lesser of the two.
///
/// The session comes last, for the same reason and read the same way: a
/// Conversation that is grilling with nothing grilling it is a thing to look at
/// and start again, and one that had launched an agent nothing had recorded
/// would be an agent nobody could see or stop. It is also the one part of this
/// that failing does not refuse — the branch is made, the Brief is frozen, and a
/// session that would not start is logged, leaving a Conversation that is
/// grilling with a Timeline that says so and no session on it. Not a stop
/// either: the human is at the button they have just pressed, and what a stop is
/// for is telling them about a run that stopped while nobody was watching. The
/// sweep is what finds this one, a minute later — see [`crate::stalls`].
///
/// **The fetch comes before the resolving**, whether the human picked a base or
/// left it to the rule: a remote-tracking branch is only ever as fresh as the
/// last fetch, and an unpicked base means origin's default branch rather than
/// this checkout's copy of it — so the work comes off what the remote is
/// holding now rather than off wherever the local branch was last left. A
/// repository with no remote has nothing to fetch and nothing to be stale
/// against, and is never refused for it.
///
/// **A Conversation that already has a worktree makes neither.** Its branch has
/// been worked and the directory is where that work was done, so what this does
/// for one is resolve the commit it branched from and start the session: a start
/// that made a branch would start the work over. Nothing the pipeline does
/// reaches this any more — a second round opens where it is steered, past
/// drafting already — so it is asked of the record rather than assumed away.
///
/// The whole state rather than the four pieces of it this needs: what starting a
/// grilling reaches is most of what the server holds — the store, the boundary,
/// the data directory, the sessions and whoever is watching them — and a
/// parameter list of that length says less than the one name does.
pub(crate) async fn start_grilling(state: &AppState, id: i64) -> Result<GrillingStarted> {
    let pool = &state.pool;
    let watched = &state.watched;

    let Some(conversation) = store::load_conversation(pool, id).await? else {
        return Ok(GrillingStarted::NoSuchConversation);
    };

    if conversation.state != store::Lifecycle::Draft {
        return Ok(GrillingStarted::NotDrafting);
    }

    // Read as rows rather than judged off the ids, which is the same reading the
    // pane gets — a Profile whose pair has gone is not one to launch a session
    // under, and the id alone cannot say so.
    let grilling = crate::profiles::pairing(watched, conversation.grilling_pairing.clone()).await?;
    let implementation =
        crate::profiles::pairing(watched, conversation.implementation_pairing.clone()).await?;

    if let Some(refusal) = unready(grilling.as_ref(), implementation.as_ref()) {
        return Ok(refusal.grilling());
    }

    // Kept rather than only judged: it is what the session about to start is
    // primed with, and it is frozen from the moment the Conversation moves.
    let brief = brief(pool, id).await?;

    if brief.trim().is_empty() {
        return Ok(GrillingStarted::EmptyBrief);
    }

    // What the work branches from, resolved here and nowhere earlier: what the
    // human picked is a branch, and what they meant by picking it is wherever it
    // stands at this moment. Without one it is the default branch, which is the
    // same rule by another name — read off origin, once the fetch below has
    // made origin's copy of it current.
    let picked = conversation.base_commit.clone();
    let default = conversation.repo.default_branch.clone();

    let repo = conversation.repo.path.clone();
    let branch = conversation.branch.clone();

    // Where the work goes on. A Conversation that already has one works where it
    // has always worked and there is nothing here to make; one that has none is
    // given the name a first grilling chooses.
    let worked_in = conversation.worktree.clone();
    let path = worked_in.clone().unwrap_or_else(|| {
        worktrees::worktree_path(&state.data_dir, id, &conversation.repo.name, &branch)
    });

    // The filesystem and git halves together, off the runtime: a worktree of a
    // large repository is not a quick call, and every part of this blocks.
    let made = tokio::task::spawn_blocking({
        let path = path.clone();
        move || {
            // A Conversation that has a worktree resolves the commit its branch
            // was cut from and stops there: the branch is taken because this
            // Conversation took it, the checkout is already where the work will
            // happen, and a base that was frozen when the work started is not
            // something a fetch could freshen.
            if worked_in.is_some() {
                let named = picked.unwrap_or(default);

                return worktrees::resolve(&repo, &named).ok_or(GrillingStarted::NoBaseCommit);
            }

            // Before anything resolves, because a remote-tracking ref is only as
            // fresh as the last fetch — and a branch made off a stale one is
            // work starting from wherever this checkout happened to be left.
            // Refused rather than carried on with: the human is at the button,
            // and offline or an authentication that has gone is theirs to fix.
            if let worktrees::Fetched::Failed(said) = worktrees::fetch(&repo) {
                tracing::error!(
                    said,
                    repo = %repo.display(),
                    "fetching a Repo's remotes failed, so its grilling is not being started",
                );

                return Err(GrillingStarted::FetchFailed);
            }

            // A picked base resolves exactly as picked; the fetch only means a
            // picked remote-tracking branch stands where it now stands. An
            // unpicked one is the default branch as origin holds it.
            let named = match picked {
                Some(picked) => picked,
                None => worktrees::default_ref(&repo, &default),
            };

            let Some(commit) = worktrees::resolve(&repo, &named) else {
                return Err(GrillingStarted::NoBaseCommit);
            };

            if worktrees::branch_exists(&repo, &branch) {
                return Err(GrillingStarted::BranchExists);
            }

            match worktrees::add(&repo, &path, &branch, &commit) {
                true => Ok(commit),
                false => Err(GrillingStarted::WorktreeRefused),
            }
        }
    })
    .await?;

    let commit = match made {
        Ok(commit) => commit,
        Err(refusal) => return Ok(refusal),
    };

    match store::start_grilling(pool, id, &commit, &path).await? {
        store::Grilling::NoSuchConversation => return Ok(GrillingStarted::NoSuchConversation),
        store::Grilling::NotDrafting => return Ok(GrillingStarted::NotDrafting),
        store::Grilling::Started => {}
    }

    // From here the Conversation says it is being grilled, and the thing that
    // will say so is a session that does not exist yet. So a registration stands
    // in for it across the launch, which is the slowest part of this: a sweep
    // that looked in between would find a Conversation grilling with nothing
    // grilling it, and stop the run under a press the human is still standing
    // at. Held to the
    // end of this rather than handed on — what drives a grilling from there is
    // its session — and what it leaves behind where the launch fails is a stall
    // for the next sweep to find. See [`crate::drivers`] and [`crate::stalls`].
    let _driving = state.drivers.driving(id);

    // Read back rather than assembled from what was just recorded: what the
    // session runs against is the Conversation as it now stands, worktree and
    // all, and the one thing that must not be guessed at is where an agent is
    // about to be let loose.
    let Some(conversation) = store::load_conversation(pool, id).await? else {
        return Ok(GrillingStarted::NoSuchConversation);
    };

    // Only the grilling Profile. The implementation one is fixed before starting
    // because the grilling ends by handing over to it — that hand-over is a
    // later stage's, and this session is not run under it.
    //
    // Logged rather than raised: by here the branch is made and the Brief is
    // frozen, and answering the button with a failure would say that none of it
    // happened.
    //
    // What it is started on is the Brief under the line that sends it into the
    // bundled grilling skill: a sandbox has no global `CLAUDE.md` to say what a
    // session is for, so the prompt is where it is said — see [`crate::skills`].
    if let Some(pairing) = conversation.grilling_pairing.clone()
        && let Err(error) = state
            .sessions
            .start(
                pool,
                &state.nudges,
                &conversation,
                &pairing,
                &skills::grilling(&brief),
            )
            .await
    {
        tracing::error!(error = ?error, conversation_id = id, "a grilling session could not be started");
    }

    Ok(GrillingStarted::Started)
}

/// Take a roadmap Verkstead did not write and start its next stage: one press,
/// and a drafting Conversation becomes the stage's own, on the stage's own
/// branch, with a planning session running in it.
///
/// The human's press stands in for the settling predecessor that starts every
/// other stage — see [`crate::continuing`], which does the same job at the other
/// end of a roadmap. So what this does is [`crate::continuing::start`]'s
/// sequence with the two differences adoption has: there is no predecessor
/// Conversation, so nothing stacks and the branch comes off the base commit; and
/// there is a human at the workbench, so what stops it is answered to the button
/// by name rather than said as a notice to nobody.
///
/// **Every refusal is named, and they are checked cheap-first**, which is
/// [`start_grilling`]'s order and for its reason: each of them is something
/// different for the human to go and do, and the record's own state and its
/// Profiles are answered before anything that costs a git call. Nothing is
/// created and nothing is checked out for any of them.
///
/// **The fetch comes before the resolving**, as it does at a grill start: an
/// unpicked base means the default branch as origin holds it rather than this
/// checkout's copy of it, so what is adopted is judged against origin's tip. A
/// fetch git would not make refuses the press by name; a repository with no
/// remote has nothing to be stale against and is never refused for it.
///
/// **The stage is read again, here, at whatever the base resolves to.** What the
/// page showed is a reading of a moment ago, and a roadmap is a document in a
/// repository that anybody may have moved since — ticked the last box, taken the
/// branch, started the stage by hand. Which is why *the roadmap has finished* is
/// among the refusals at all.
///
/// **Then git, and then the store**, which is the order [`start_grilling`] does
/// the same job in and for the same reason: a row saying a stage is under way
/// with nothing checked out is a Conversation nothing can run and nothing will
/// clean up, where a directory the store does not know about is a directory to
/// tidy.
///
/// The Timeline gets both records — the stage brief as the Brief, and what was
/// adopted from where — and the planning session comes last, exactly as it does
/// for a stage a predecessor started. From the plan commit onwards there is
/// nothing new: that commit touches the roadmap, so the stage after this one is
/// carried on by the path that was already there.
pub(crate) async fn adopt(state: &AppState, id: i64) -> Result<Adopted> {
    let pool = &state.pool;
    let watched = &state.watched;

    let Some(conversation) = store::load_conversation(pool, id).await? else {
        return Ok(Adopted::NoSuchConversation);
    };

    if conversation.state != store::Lifecycle::Draft {
        return Ok(Adopted::NotDrafting);
    }

    // Adopting is how a stage's work *started*, so it is not a thing to do twice
    // — and what says it has happened is the worktree, adoption being what made
    // one.
    if conversation.worktree.is_some() {
        return Ok(Adopted::NotDrafting);
    }

    // The one thing about the roadmap that is Verkstead's, and the whole of what
    // makes this Conversation an adopting one. Everything else about it is read
    // back out of the repository.
    let Some(roadmap) = conversation.adopting.clone() else {
        return Ok(Adopted::NotAdopting);
    };

    // Both Profiles, before anything that costs a git call — the cheap answers
    // first, which is the order [`start_grilling`] checks the same pair in. Read
    // as rows rather than judged off the ids, because a Profile whose pair has
    // gone is not one to run a session under and the id alone cannot say so.
    //
    // Both, rather than only the one the work runs under: a stage inherits both
    // from its predecessor, so what this one is adopted with is what every stage
    // after it starts with.
    let grilling = crate::profiles::pairing(watched, conversation.grilling_pairing.clone()).await?;
    let implementation =
        crate::profiles::pairing(watched, conversation.implementation_pairing.clone()).await?;

    if let Some(refusal) = unready(grilling.as_ref(), implementation.as_ref()) {
        return Ok(refusal.adopting());
    }

    // Where the stage branches from. The override where the human fixed one —
    // which is how an unmerged predecessor is stacked on, that being their move
    // rather than Verkstead's — and the default branch as origin holds it where
    // they did not; which of the two it is, is settled inside the read below,
    // because the fetch that makes origin's copy current happens there.
    let picked = conversation.base_commit.clone();
    let default = conversation.repo.default_branch.clone();

    let repo = conversation.repo.path.clone();

    // The reading, off the runtime's threads: fetching, resolving a commit and
    // reading a roadmap out of a git directory are all blocking calls.
    let read = tokio::task::spawn_blocking({
        let repo = repo.clone();

        move || {
            // Before anything resolves. The human is at this button, so a fetch
            // git would not make refuses the press by name rather than adopting
            // a stage judged against refs that may be a week old — being offline
            // or having lost an authentication is theirs to go and fix. A
            // repository with no remote has nothing to fetch and is never
            // refused for it.
            if let worktrees::Fetched::Failed(said) = worktrees::fetch(&repo) {
                tracing::error!(
                    said,
                    repo = %repo.display(),
                    "fetching a Repo's remotes failed, so its roadmap is not being adopted",
                );

                return Err(Adopted::FetchFailed);
            }

            let named = match picked {
                Some(picked) => picked,
                None => worktrees::default_ref(&repo, &default),
            };

            let Some(commit) = worktrees::resolve(&repo, &named) else {
                return Err(Adopted::NoBaseCommit);
            };

            // The same rule the notice was drawn by and the page was drawn by,
            // asked here at the base commit — and asked again, rather than
            // taken from either, because a roadmap is a document anybody may
            // have moved since. Which clause refused it is the answer to the
            // button: each of them is a different thing to go and do about it.
            match crate::stages::startable(&repo, &commit, &roadmap) {
                Startable::Stage(abandoned) => Ok((commit, named, abandoned.stage)),
                Startable::NoRoadmap => Err(Adopted::NoRoadmap),
                Startable::Complete => Err(Adopted::RoadmapComplete),
                Startable::InFlight => Err(Adopted::StageInFlight),
                Startable::NoBrief => Err(Adopted::NoBrief),
                Startable::BranchTaken => Err(Adopted::BranchExists),
            }
        }
    })
    .await?;

    // `named` comes back out rather than being worked out again up here: what an
    // unpicked base resolved through is decided inside, after the fetch, and the
    // Timeline is owed the name the branch actually came off.
    let (commit, named, stage) = match read {
        Ok(read) => read,
        Err(refusal) => return Ok(refusal),
    };

    // The stage's own slug, as the unattended start names one — the brief's
    // filename without its number. The name the Conversation was started under
    // was the server's invention for a row in the sidebar, and it is discarded
    // here: a stage is worked on the branch its roadmap will annotate it with.
    let branch = stage.branch();
    let path = worktrees::worktree_path(&state.data_dir, id, &conversation.repo.name, &branch);

    let made = tokio::task::spawn_blocking({
        let path = path.clone();
        let branch = branch.clone();
        let commit = commit.clone();

        move || worktrees::add(&repo, &path, &branch, &commit)
    })
    .await?;

    if !made {
        return Ok(Adopted::WorktreeRefused);
    }

    // And now the store, in the order the record is read in: the branch it is on,
    // the Brief it works from, and then the move that freezes both. Adoption
    // never stacks — there is no predecessor Conversation to stand on, and
    // standing on an unmerged one is the base commit the human fixed above.
    store::rename_branch(pool, id, &branch).await?;
    store::save_brief(pool, id, &stage.brief).await?;

    match store::start_stage(pool, id, &commit, &path, None).await? {
        store::Staged::Started => {}
        store::Staged::NoSuchConversation => return Ok(Adopted::NoSuchConversation),
        store::Staged::NotDrafting => return Ok(Adopted::NotDrafting),
    }

    // What was adopted, from where, and where its branch came off — on the
    // Conversation's own Timeline, because that is the only Timeline there is:
    // adoption has no predecessor Conversation for the human to have been
    // watching.
    if let Err(error) = store::note(pool, id, &adopted(&stage, &branch, &named)).await {
        tracing::error!(error = ?error, conversation_id = id, "recording what was adopted failed");
    }

    tracing::info!(
        conversation_id = id,
        branch,
        label = stage.label,
        roadmap = stage.roadmap,
        "a roadmap stage was adopted and has started",
    );

    // A Conversation moved and the sidebar's notice has one roadmap fewer in it,
    // and an open page should say so without being reloaded. Two things moved
    // and they are said as two: the notice is drawn off the Repos, which is not
    // where this Conversation is drawn from.
    state
        .nudges
        .announce(Nudge::Conversation { conversation: id });
    state.nudges.announce(Nudge::Repos);

    tokio::spawn(crate::runner::plan_stage(state.clone(), id, None));

    Ok(Adopted::Adopted)
}

/// What an adopting Conversation's Timeline is told: which stage of which
/// roadmap was adopted, and where its branch came off.
///
/// [`crate::continuing::begun`]'s wording, with the two things adoption changes
/// taken out of it. *With nobody asked* goes, because somebody did: a human
/// pressed this. And only the came-off half is ever said, because an adopted
/// stage has no predecessor Conversation to stack on — where its branch stands
/// is the base commit, which is the human's to fix and theirs alone.
fn adopted(stage: &crate::stages::Stage, branch: &str, from: &str) -> String {
    // The brief named rather than linked, as the unattended start names it: it
    // is a path in a Worktree the workbench has no route to, and a link that
    // went nowhere would be worse than the path itself.
    format!(
        "Stage {} of the `{}` roadmap — *{}* — was adopted from `{}`. Its branch `{branch}` came \
         off `{from}`: an adopted stage has no Conversation before it to stack on, so where it \
         stands is the base commit this one was fixed to.",
        stage.label, stage.roadmap, stage.title, stage.brief_path,
    )
}

/// Stop a Conversation wherever it has got to: its session ended, its worktree
/// removed, its branch left where it is.
///
/// The session goes first and is waited on, because what follows it is the
/// removal of the directory it is working in: an agent still writing files into
/// a worktree git is taking away is the one way this could leave a mess neither
/// end knows about.
///
/// The worktree then goes before the record does, for the reason the branch is
/// made before one: what is recorded is what happened. A Conversation that said
/// it had stopped while its directory was still on disk would be one nothing
/// would ever come back and remove.
pub(crate) async fn close(state: &AppState, id: i64) -> Result<ConversationClosed> {
    let pool = &state.pool;

    let Some(conversation) = store::load_conversation(pool, id).await? else {
        return Ok(ConversationClosed::NoSuchConversation);
    };

    state.sessions.end(id).await;

    if let Some(path) = conversation.worktree.clone() {
        let repo = conversation.repo.path.clone();

        let removed = tokio::task::spawn_blocking(move || worktrees::remove(&repo, &path)).await?;

        if !removed {
            tracing::error!(
                conversation_id = id,
                "a Conversation's worktree could not be removed"
            );
            return Ok(ConversationClosed::WorktreeStuck);
        }
    }

    // And the directory beside it, with whatever the sessions put there. It is
    // given back for the reason the worktree is: it is somewhere a Conversation
    // was given to work, and the Conversation has stopped. Whatever it held that
    // was worth keeping is on the Timeline already.
    let handoffs = Handoffs::under(&state.data_dir);
    tokio::task::spawn_blocking(move || handoffs.remove(id)).await?;

    Ok(match store::close_conversation(pool, id).await? {
        store::Closing::Closed => ConversationClosed::Closed,
        store::Closing::AlreadyClosed => ConversationClosed::AlreadyClosed,
        store::Closing::NoSuchConversation => ConversationClosed::NoSuchConversation,
    })
}

/// Why these two Pairings are not something to run the work under, or `None`
/// where they are.
///
/// The same rule [`crate::profiles::ready_to_grill`] answers for the pane, said
/// the other way round: that one says whether to offer the button, this one says
/// what was wrong when it was pressed. Each is named separately, because
/// choosing a Pairing and mending a broken Profile are different jobs.
///
/// A Profile chosen with no model beside it is unpaired and reads here as
/// nothing chosen, which is what it is: the pick to make again is the whole
/// Pairing, Profile and model together.
///
/// The rule rather than either button's answer, because both buttons ask it:
/// starting a grilling and adopting a stage each want both Pairings fixed
/// before they will do anything, and each says so in its own words — see
/// [`Unready::grilling`] and [`Unready::adopting`].
fn unready(
    grilling: Option<&PairingView>,
    implementation: Option<&PairingView>,
) -> Option<Unready> {
    let Some(grilling) = grilling.filter(|pairing| pairing.model.is_some()) else {
        return Some(Unready::NoGrillingProfile);
    };

    let Some(implementation) = implementation.filter(|pairing| pairing.model.is_some()) else {
        return Some(Unready::NoImplementationProfile);
    };

    [grilling, implementation]
        .into_iter()
        .any(|pairing| pairing.profile.broken.is_some())
        .then_some(Unready::ProfileBroken)
}

/// What is wrong with a Conversation's pair of Profiles, before it is put in
/// the words of whichever press asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unready {
    NoGrillingProfile,
    NoImplementationProfile,
    ProfileBroken,
}

impl Unready {
    /// Said to the press that starts a grilling.
    fn grilling(self) -> GrillingStarted {
        match self {
            Unready::NoGrillingProfile => GrillingStarted::NoGrillingProfile,
            Unready::NoImplementationProfile => GrillingStarted::NoImplementationProfile,
            Unready::ProfileBroken => GrillingStarted::ProfileBroken,
        }
    }

    /// And to the press that adopts a stage.
    fn adopting(self) -> Adopted {
        match self {
            Unready::NoGrillingProfile => Adopted::NoGrillingProfile,
            Unready::NoImplementationProfile => Adopted::NoImplementationProfile,
            Unready::ProfileBroken => Adopted::ProfileBroken,
        }
    }
}

/// The Brief the round a Conversation is in started from.
///
/// The *last* Brief rather than the first, and searched for rather than taken
/// from either end: a Conversation gets one Brief per round — a steered one
/// adds a second rather than editing the first — and the round about to be
/// grilled is the one at the bottom of the Timeline.
///
/// Asked by the two ways into a grilling, so that both are refused on an empty
/// one: [`start_grilling`] above, and the `steering` module's own refusals for
/// a steer that opens a round without writing a brief for it.
pub(crate) async fn brief(pool: &SqlitePool, id: i64) -> Result<String> {
    Ok(store::timeline(pool, id)
        .await?
        .into_iter()
        .rev()
        .find_map(|event| match event.event {
            store::Event::Brief(markdown) => Some(markdown),
            _ => None,
        })
        .unwrap_or_default())
}

/// The two documents a session that builds the work is primed with: the Brief,
/// and the handoff where a grilling wrote one.
///
/// One name and one read of the Timeline for the pair, because they are always
/// wanted together — an inline session, a breakdown and every task session take
/// exactly these two.
///
/// Both are the *last* of their kind rather than the first, and for one reason:
/// a Conversation gets a Brief and a handoff per round — a steered one adds a
/// second Brief rather than editing the first — and what a session about to build
/// is primed with is the round it is building.
pub(crate) async fn documents(pool: &SqlitePool, id: i64) -> Result<(String, Option<String>)> {
    let timeline = store::timeline(pool, id).await?;

    let brief = timeline
        .iter()
        .rev()
        .find_map(|event| match &event.event {
            store::Event::Brief(markdown) => Some(markdown.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let handoff = timeline.iter().rev().find_map(|event| match &event.event {
        store::Event::Handoff(markdown) => Some(markdown.clone()),
        _ => None,
    });

    Ok((brief, handoff))
}

/// A Conversation's worktree as the viewer receives it: where it is, and whether
/// it is still there.
///
/// The look at the filesystem happens here rather than in the store, because
/// whether a directory exists is not something a database knows — and it is
/// worth knowing: a worktree deleted by hand should read as a Conversation with
/// a problem rather than as an obscure failure from whatever next works in it.
pub(crate) async fn worktree(path: Option<PathBuf>) -> Result<Option<Worktree>> {
    let Some(path) = path else {
        return Ok(None);
    };

    let missing = tokio::task::spawn_blocking({
        let path = path.clone();
        move || !path.is_dir()
    })
    .await?;

    Ok(Some(Worktree {
        // Stored as UTF-8 in the first place — a path that is not cannot be
        // recorded — so nothing is lost putting it back on the wire.
        path: path.to_string_lossy().into_owned(),
        missing,
    }))
}

/// Whether everything needed before grilling starts is settled, as the pane
/// reads it: the two Pairings, and a Brief with something in it.
///
/// Answered against what the endpoint has already read rather than by loading
/// the Conversation again — and it deliberately says nothing about the branch or
/// the base commit, which are decided against git when the button is pressed.
pub(crate) fn ready_to_grill(
    state: store::Lifecycle,
    grilling: Option<&PairingView>,
    implementation: Option<&PairingView>,
    brief: &str,
) -> bool {
    state == store::Lifecycle::Draft
        && !brief.trim().is_empty()
        && crate::profiles::ready_to_grill(grilling, implementation)
}

/// Whether git would take this as a branch name.
///
/// Asked as the full ref the branch would become rather than through
/// `--branch`: that form is judged by the rules alone, with no repository to
/// judge it in and nothing resolved on the way — `--branch` would read `@{-1}`
/// as whichever branch was checked out where the server happens to be running,
/// and hand back a name nobody typed.
///
/// The leading dash is the one rule added here, because it is not one of git's:
/// `refs/heads/-x` is a perfectly well-formed ref, and a branch called `-x` is
/// one every git command line would read as an option.
///
/// The directory is the server's own and means nothing — this asks about the
/// spelling of a name, and there is no repository involved in the answer.
fn is_branch_name(branch: &str) -> bool {
    !branch.is_empty()
        && !branch.starts_with('-')
        && git(
            Path::new("."),
            &["check-ref-format", &format!("refs/heads/{branch}")],
        )
        .is_some()
}

/// A branch name to start a Conversation under, until the human names it
/// themselves.
///
/// Two words rather than a hash: it is a branch name, so it is going to be typed
/// and read aloud, and the whole point of prefilling one is that the human need
/// not stop and think of anything before they start writing the brief.
fn branch_name() -> String {
    /// Colours, weather and temper — words that pair with anything below and
    /// carry no meaning about the work, so a name that is left as it is never
    /// says something untrue about the branch.
    const QUALITIES: [&str; 32] = [
        "amber", "ashen", "autumn", "brisk", "cobalt", "copper", "crisp", "dusky", "eager",
        "ember", "flint", "frosted", "gilded", "hushed", "indigo", "ivory", "lucid", "mellow",
        "misty", "nimble", "ochre", "opal", "quiet", "rustic", "sable", "scarlet", "slate",
        "sunlit", "teal", "umber", "verdant", "wistful",
    ];

    /// Birds, weather and landscape, for the same reason.
    const THINGS: [&str; 32] = [
        "anchor", "beacon", "bramble", "cedar", "cinder", "coppice", "curlew", "delta", "eddy",
        "fathom", "gable", "harbour", "heron", "kestrel", "lantern", "meadow", "orchard", "otter",
        "pennant", "quarry", "ridge", "rookery", "sable", "shale", "sparrow", "thicket", "thistle",
        "tundra", "vale", "willow", "wren", "zephyr",
    ];

    // A generator that could not answer is not a reason to refuse to start a
    // Conversation: the name is a prefill the human is free to replace, so an
    // unlucky machine gets the first pair rather than an error.
    let picked = getrandom::u64().unwrap_or(0);

    let quality = QUALITIES[(picked % QUALITIES.len() as u64) as usize];
    let thing = THINGS[((picked / QUALITIES.len() as u64) % THINGS.len() as u64) as usize];

    format!("{quality}-{thing}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A prefilled name has to be one git would take, or the human is handed a
    /// branch that cannot be created and no reason why.
    #[test]
    fn every_prefilled_branch_name_is_one_git_would_take() {
        for _ in 0..64 {
            let name = branch_name();
            assert!(is_branch_name(&name), "git refused {name:?}");
        }
    }

    #[test]
    fn a_prefilled_name_is_two_words_a_human_can_say() {
        let name = branch_name();
        let (quality, thing) = name.split_once('-').expect("two words and a hyphen");

        assert!(!quality.is_empty() && !thing.is_empty());
        assert!(name.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
    }

    #[test]
    fn the_names_git_refuses_are_refused_here() {
        for refused in [
            "",
            " ",
            "has space",
            "two..dots",
            "ends/",
            "-dash",
            "with~tilde",
            "@{-1}",
        ] {
            assert!(!is_branch_name(refused), "{refused:?} should be refused");
        }
    }

    #[test]
    fn an_ordinary_branch_name_is_taken() {
        for taken in ["main", "rate-limiting", "feature/rate-limiting", "v2"] {
            assert!(is_branch_name(taken), "{taken:?} should be taken");
        }
    }
}
