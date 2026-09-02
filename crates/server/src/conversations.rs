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
    Adopted, BaseRecorded, BranchRenamed, BriefSaved, CompanionAdded, CompanionBaseRecorded,
    CompanionBranchRenamed, CompanionMode, CompanionModeChosen, CompanionRefusal, CompanionRemoved,
    ConversationClosed, GrillingStarted, PairingView, PickedView, RepoPairingsView, RepoSwitched,
    Started, Worktree,
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
/// The name is the server's because the record is: a name the browser invented
/// would be one the server never saw, and there has to be a branch to cut
/// whether or not the human ever types one.
///
/// It stays Verkstead's until they do, which is what the record is started
/// saying: while the Conversation drafts the workbench draws that name nowhere,
/// calls it a Draft, and leaves the branch field empty for a name of theirs.
///
/// The Pairings are prefilled the same way, off what the Repo was last
/// grilled with — see [`prefill`].
pub(crate) async fn start(state: &AppState, repo_id: i64) -> Result<Started> {
    Ok(
        match store::start_unnamed_conversation(&state.pool, repo_id, &branch_name()).await? {
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
/// discarded at the press — a stage is worked on its own name — so what it does
/// until then is stand in the record for a branch nobody has named. The row
/// reads *Draft* the whole time, which is what it is.
///
/// The roadmap is taken as the notice gave it. Whether it is still there with a
/// stage to start is a question about a repository at a commit, and it is asked
/// where the page is drawn and asked again when Adopt is pressed: a roadmap
/// somebody finished between the notice and the click is a thing to say on the
/// page rather than a start to refuse.
///
/// `base` is the branch the notice found the roadmap on, and it is fixed on the
/// Conversation here rather than left to the human: a roadmap staged on an
/// unmerged branch exists nowhere else, so a Conversation started against the
/// default branch would draw *nothing to adopt at this base commit* about the
/// very roadmap that was just clicked. `None` is the default branch, which is
/// what a Conversation with no base fixed already reads.
pub(crate) async fn start_adopting(
    state: &AppState,
    repo_id: i64,
    roadmap: &str,
    base: Option<&str>,
) -> Result<Started> {
    Ok(
        match store::start_adoption(&state.pool, repo_id, &branch_name(), roadmap).await? {
            Some(id) => {
                if let Some(base) = base {
                    fix(state, id, base).await;
                }

                prefill(state, id, repo_id).await;
                Started::Started { id }
            }
            None => Started::NoSuchRepo,
        },
    )
}

/// Fix a new adopting Conversation's base to the branch its roadmap was found
/// on.
///
/// Nothing refuses the start over this, for [`prefill`]'s reason: the
/// Conversation exists by the time this runs. What a base that failed to write
/// costs is a page saying there is nothing to adopt at the base it does have,
/// with the picker that fixes it sitting on the same page — so it is logged and
/// the human is left somewhere they can get themselves out of.
async fn fix(state: &AppState, id: i64, base: &str) {
    if let Err(error) = store::set_base_commit(&state.pool, id, Some(base)).await {
        tracing::warn!(
            error = ?error,
            conversation_id = id,
            base,
            "fixing an adopting Conversation to the branch its roadmap was found on failed",
        );
    }
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
    let prefill = pairing_prefill(state, repo_id).await?;

    // A Repo last started with no grilling is prefilled with no grilling, which
    // is the memory doing exactly what it does for a Pairing: what the human
    // last picked, ready to be changed. Nothing is judged about the row that
    // runs nothing — there is no Profile to have gone — so it is applied
    // wherever it was remembered.
    if prefill.grilling.skipped() {
        store::skip_grilling(&state.pool, id).await?;
    } else if let Some(pairing) = prefill.grilling.pairing() {
        store::set_grilling_pairing(
            &state.pool,
            id,
            pairing.profile.id,
            pairing.model.as_deref(),
        )
        .await?;
    }

    if let Some(pairing) = &prefill.implementation {
        store::set_implementation_pairing(
            &state.pool,
            id,
            pairing.profile.id,
            pairing.model.as_deref(),
        )
        .await?;
    }

    // And the same one role along: a Repo last started with no review opens its
    // next Conversation on that row too.
    if prefill.review.skipped() {
        store::skip_review(&state.pool, id).await?;
    } else if let Some(pairing) = prefill.review.pairing() {
        store::set_review_pairing(
            &state.pool,
            id,
            pairing.profile.id,
            pairing.model.as_deref(),
        )
        .await?;
    }

    Ok(())
}

/// What a Repo was last grilled with, judged as something to prefill pickers
/// with — the whole of what [`prefill`] applies, and what the compose page
/// fills its own pickers from before there is a Conversation to apply it to.
///
/// Read here rather than beside the Repo's other endpoints because the judging
/// is this module's: what a new Conversation on this Repo would arrive showing
/// is the question, and a second reading of the memory somewhere else would be
/// a second answer to it.
pub(crate) async fn pairing_prefill(state: &AppState, repo_id: i64) -> Result<RepoPairingsView> {
    let remembered = store::remembered_pairings(&state.pool, repo_id).await?;

    Ok(RepoPairingsView {
        grilling: prefilled(&state.watched, remembered.grilling).await?,
        implementation: usable(&state.watched, remembered.implementation).await?,
        review: prefilled(&state.watched, remembered.review).await?,
    })
}

/// One role's memory as a picker would show it, for the two roles that can
/// remember the row that runs no session.
///
/// The row is not judged — there is no Profile to have gone — so it comes back
/// as itself, and everything else goes through [`usable`].
async fn prefilled(
    watched: &crate::watched::WatchedPaths,
    remembered: store::Picked,
) -> Result<PickedView> {
    if remembered.skipped() {
        return Ok(PickedView::Skipped);
    }

    Ok(match usable(watched, remembered).await? {
        Some(pairing) => PickedView::Under(pairing),
        None => PickedView::Nothing,
    })
}

/// A remembered Pairing as something to prefill a picker with, or `None` where
/// it is not one any more.
///
/// Read as a row rather than trusted as a pair of ids, which is the reading
/// [`start_grilling`] gives the Pairings it is about to launch under: whether
/// the Profile's pair is still where it was left is a question for the Watched
/// Paths, and whether it still lists the model is a question for the Profile's
/// own list.
///
/// A remembered role that was picked away is `None` here too — it is nothing to
/// prefill a *Pairing* with, and its caller applies it on its own account.
///
/// What comes back is the Pairing whole, both halves settled: it is what one
/// caller writes onto a new Conversation and what the other hands to a page, and
/// neither of them should have to put the two together again.
async fn usable(
    watched: &crate::watched::WatchedPaths,
    remembered: store::Picked,
) -> Result<Option<PairingView>> {
    let Some(model) = remembered
        .pairing()
        .and_then(|pairing| pairing.model.clone())
    else {
        return Ok(None);
    };

    let Some(pairing) = crate::profiles::pairing(watched, remembered.pairing().cloned()).await?
    else {
        return Ok(None);
    };

    Ok(
        (pairing.profile.broken.is_none() && pairing.profile.models.contains(&model))
            .then_some(pairing),
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

/// Name the branch the work will be done on, or hand the naming of it back to
/// Verkstead.
///
/// Whether the name is usable is git's to say, not a list of forbidden
/// characters here: the branch this names is one git will be asked to create,
/// and the only opinion that will matter then is the one being asked now.
///
/// Blank is the field cleared rather than a name git would refuse, so it is
/// never asked about: what the human means by emptying that field is *choose
/// one for me*, and what stands again is the name Verkstead prefilled the
/// record with. Which is why it is answered as a rename like any other — the
/// name has changed hands, and the card has nothing to say about it beyond the
/// placeholder coming back.
pub(crate) async fn rename_branch(
    pool: &SqlitePool,
    id: i64,
    branch: &str,
) -> Result<BranchRenamed> {
    let branch = branch.trim().to_owned();

    if !branch.is_empty()
        && !tokio::task::spawn_blocking({
            let branch = branch.clone();
            move || is_branch_name(&branch)
        })
        .await?
    {
        return Ok(BranchRenamed::NotABranchName);
    }

    let named = (!branch.is_empty()).then_some(branch.as_str());

    Ok(match store::rename_branch(pool, id, named).await? {
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

/// Move a drafting Conversation onto another registered Repo.
///
/// Thin over the store, and for [`add_companion`]'s reason with one more behind
/// it: everything a switch decides — which repository, what its base goes back
/// to, and which companion is no longer one — is a question about the rows, and
/// nothing has been checked out to ask git about. The two refusals that matter
/// are the ones saying something *has* been settled elsewhere: a worktree, which
/// the store answers off the row it wrote, and an adoption, whose repository was
/// settled by the roadmap rather than by the human.
pub(crate) async fn switch_repo(pool: &SqlitePool, id: i64, repo_id: i64) -> Result<RepoSwitched> {
    Ok(match store::switch_repo(pool, id, repo_id).await? {
        store::Switched::Switched => RepoSwitched::Switched,
        store::Switched::NoSuchConversation => RepoSwitched::NoSuchConversation,
        store::Switched::NotDrafting => RepoSwitched::NotDrafting,
        store::Switched::Adopting => RepoSwitched::Adopting,
        store::Switched::NoSuchRepo => RepoSwitched::NoSuchRepo,
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

/// Say how far into a companion the work may reach.
///
/// Thin over the store for [`add_companion`]'s reason: what the switch decides
/// is what the sandbox binds and whether a branch is cut, and neither of those
/// happens until grilling starts.
pub(crate) async fn set_companion_mode(
    pool: &SqlitePool,
    id: i64,
    repo_id: i64,
    mode: CompanionMode,
) -> Result<CompanionModeChosen> {
    let mode = match mode {
        CompanionMode::ReadOnly => store::CompanionMode::ReadOnly,
        CompanionMode::ReadWrite => store::CompanionMode::ReadWrite,
    };

    Ok(
        match store::configure_companion(pool, id, repo_id, store::Change::Mode(mode)).await? {
            store::Configured::Saved => CompanionModeChosen::Chosen,
            store::Configured::NoSuchConversation => CompanionModeChosen::NoSuchConversation,
            store::Configured::NotDrafting => CompanionModeChosen::NotDrafting,
            store::Configured::NoSuchCompanion => CompanionModeChosen::NoSuchCompanion,
        },
    )
}

/// Record the branch a companion's checkout comes off, or put it back on the
/// default-branch rule.
///
/// The same shape as [`set_base_branch`] and for the same reasons — a name
/// rather than a commit, resolved at grill start, and refused unless the
/// repository really has a branch by it. The repository asked is the
/// *companion's* own: two Conversations against one companion are looking at
/// the same list, and neither is looking at the Conversation's.
pub(crate) async fn set_companion_base(
    pool: &SqlitePool,
    id: i64,
    repo_id: i64,
    asked: Option<&str>,
) -> Result<CompanionBaseRecorded> {
    let asked = asked.map(str::trim).filter(|asked| !asked.is_empty());

    if let Some(branch) = asked {
        // Where the Conversation has got to before the branches, so that a
        // Conversation frozen months ago is told *that* rather than told about
        // a branch the companion's repository has since lost. The store asks
        // again, this read and that write not being one moment.
        let Some(conversation) = store::load_conversation(pool, id).await? else {
            return Ok(CompanionBaseRecorded::NoSuchConversation);
        };

        if conversation.state != store::Lifecycle::Draft {
            return Ok(CompanionBaseRecorded::NotDrafting);
        }

        let Some(companion) = conversation
            .companions
            .into_iter()
            .find(|companion| companion.repo.id == repo_id)
        else {
            return Ok(CompanionBaseRecorded::NoSuchCompanion);
        };

        let branch = branch.to_owned();
        let known = tokio::task::spawn_blocking(move || {
            worktrees::branches(&companion.repo.path).contains(&branch)
        })
        .await?;

        if !known {
            return Ok(CompanionBaseRecorded::NoSuchBranch);
        }
    }

    Ok(
        match store::configure_companion(pool, id, repo_id, store::Change::Base(asked)).await? {
            store::Configured::Saved => CompanionBaseRecorded::Recorded,
            store::Configured::NoSuchConversation => CompanionBaseRecorded::NoSuchConversation,
            store::Configured::NotDrafting => CompanionBaseRecorded::NotDrafting,
            store::Configured::NoSuchCompanion => CompanionBaseRecorded::NoSuchCompanion,
        },
    )
}

/// Name the branch a read-write companion's work will be done on, or empty to
/// put it back on mirroring the Conversation's own.
///
/// Whether the name is usable is git's to say, as it is for the Conversation's
/// own branch — and the empty name is never asked about, because it is not a
/// name: it is the record holding none, which is what mirroring is.
pub(crate) async fn rename_companion_branch(
    pool: &SqlitePool,
    id: i64,
    repo_id: i64,
    branch: &str,
) -> Result<CompanionBranchRenamed> {
    let branch = branch.trim().to_owned();

    if !branch.is_empty()
        && !tokio::task::spawn_blocking({
            let branch = branch.clone();
            move || is_branch_name(&branch)
        })
        .await?
    {
        return Ok(CompanionBranchRenamed::NotABranchName);
    }

    Ok(
        match store::configure_companion(pool, id, repo_id, store::Change::Branch(&branch)).await? {
            store::Configured::Saved => CompanionBranchRenamed::Renamed,
            store::Configured::NoSuchConversation => CompanionBranchRenamed::NoSuchConversation,
            store::Configured::NotDrafting => CompanionBranchRenamed::NotDrafting,
            store::Configured::NoSuchCompanion => CompanionBranchRenamed::NoSuchCompanion,
        },
    )
}

/// Give a drafting Conversation somewhere to work: a branch off its base commit
/// and a worktree of its Repo, and the move onto the Timeline that says so.
///
/// **Two landings, and which of them is what the human picked.** A Conversation
/// with a grilling Pairing is grilled: the session that starts is the interview,
/// and what the work becomes is settled through it. One whose human picked *no
/// grilling* has settled it already — the Brief is the whole plan — so the same
/// branch, the same worktree, the same frozen Brief and the same fixed Pairings
/// leave the Conversation Implementing, with an inline session building from the
/// Brief alone. Everything above this line is the same work either way, which is
/// why it is one press and one function rather than two.
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
/// **And the branch name is settled here, not when the Conversation was
/// started.** A name Verkstead invented is a prefill: it is drawn nowhere, the
/// human never saw it, and by the time the branch is cut the repository may
/// well have one by that name already — an earlier Conversation's, since a
/// branch outlives the worktree it was worked in. So a taken prefill is
/// replaced with another free name rather than refused — free in every
/// repository it is about to be cut in at once, and free of what their remotes
/// hold as well as their own branches — and the record follows what was cut.
/// What *is* refused is a name the human typed on a branch that is already
/// there: that one they chose and meant, and it is theirs to think again about.
///
/// **Every companion repo is checked out here too**, and every one of them is
/// asked the same four questions in the same order and refused by the same
/// names — with the repository said, because *which one* is the whole of what
/// the human needs. A read-write companion is cut a branch of its own from its
/// base, exactly as the Conversation's repository is; a read-only one is checked
/// out detached at the commit its base resolved to, having nothing to commit.
///
/// **Every question is asked before any of them is answered.** The fetches, the
/// resolutions and the branch checks for the Conversation's repository and for
/// every companion come first, and only then is anything made — which is the
/// cheapest way to hold that a refused start leaves nothing behind. What is left
/// to unwind past that point is a `worktree add` that failed partway, and it is
/// unwound directory and branch together: a branch cut moments ago by a start
/// that then refused holds nothing worth keeping.
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

    // And in front of everything this press makes, on a build that runs no
    // sessions: the branch, the worktree and the frozen Brief are what a session
    // is started *into*, and making all three for a launch that cannot happen
    // would leave the human a Conversation grilling with nothing grilling it.
    // The pane draws the state rather than the button — see
    // [`crate::sessions::run_on`] — and this is that rule asked again on
    // arrival.
    if state.sessions.here().absent() {
        return Ok(GrillingStarted::NotOnWindowsYet);
    }

    // Read as rows rather than judged off the ids, which is the same reading the
    // pane gets — a Profile whose pair has gone is not one to launch a session
    // under, and the id alone cannot say so.
    let grilling = crate::profiles::picked(watched, conversation.grilling_pairing.clone()).await?;
    let implementation =
        crate::profiles::pairing(watched, conversation.implementation_pairing.clone()).await?;
    let review = crate::profiles::picked(watched, conversation.review_pairing.clone()).await?;

    if let Some(refusal) = unready(&grilling, implementation.as_ref(), &review) {
        return Ok(refusal.grilling());
    }

    // Which of the two this press is, decided once and before anything is made:
    // what it changes is where the Conversation lands and what is launched into
    // the worktree, and neither of those is a question git has to be asked.
    let grilled = !grilling.skipped();

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
    let repo_name = conversation.repo.name.clone();
    let branch = conversation.branch.clone();

    // Whether that name is one somebody chose, which is what decides whether a
    // repository already holding it is a refusal or a reason to pick again —
    // see the choosing below.
    let settled = conversation.branch_named;
    let companions = conversation.companions.clone();
    let data_dir = state.data_dir.clone();

    // Where the work goes on. A Conversation that already has one works where it
    // has always worked and there is nothing here to make; one that has none is
    // given a directory named for its branch, chosen below rather than here
    // because the branch itself is not settled until the repository has been
    // asked about the name.
    let worked_in = conversation.worktree.clone();

    // The filesystem and git halves together, off the runtime: a worktree of a
    // large repository is not a quick call, and every part of this blocks.
    let made = tokio::task::spawn_blocking({
        let branch = branch.clone();
        let checkouts = state.checkouts.clone();
        move || {
            // A Conversation that has a worktree resolves the commit its branch
            // was cut from and stops there: the branch is taken because this
            // Conversation took it, the checkout is already where the work will
            // happen, and a base that was frozen when the work started is not
            // something a fetch could freshen. Its companions were checked out
            // with it and are where they were left.
            if let Some(path) = worked_in {
                let named = picked.unwrap_or(default);

                return worktrees::resolve(&repo, &named)
                    .map(|commit| (commit, path, branch, Vec::new(), None))
                    .ok_or(GrillingStarted::NoBaseCommit);
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

            // And the name the work is cut on. A name Verkstead invented is
            // nobody's — a prefill drawn nowhere, which the human never saw and
            // could not have meant — so a repository that already has a branch
            // by it is a reason to invent another rather than a reason to stop
            // the work. Asked after the fetch, so that what the remotes hold is
            // as fresh as the remotes are.
            let branch = match settled {
                true => branch,
                false => name_to_cut(id, branch, &cut_in(&repo, &companions)),
            };

            // Which leaves the refusal to the case it was written for: a name
            // somebody typed, on a branch somebody's work is already on. That
            // one is theirs to think again about, and taking it over would be
            // Verkstead writing into work it did not start.
            if worktrees::branch_exists(&repo, &branch) {
                return Err(GrillingStarted::BranchExists);
            }

            let path = worktrees::worktree_path(&data_dir, id, &repo_name, &branch);

            // The Conversation's own checkout is the first of the list, and
            // every companion asks its way on to the end of it. Nothing is made
            // until the whole list is there.
            let mut planned = vec![Checkout {
                companion: None,
                repo,
                path: path.clone(),
                branch: Some(branch.clone()),
                commit: commit.clone(),
            }];

            for companion in companions {
                let beside =
                    plan(&data_dir, id, &branch, companion, &planned).map_err(Unmade::grilling)?;

                planned.push(beside);
            }

            // And only now, held from the first directory this makes to the
            // record naming it: a sweep of orphaned worktrees reading in
            // between would find live checkouts no record names. See
            // [`crate::AppState::checkouts`].
            //
            // Here rather than around the whole of this, which is where it
            // would read as belonging: everything above only asks questions,
            // and the fetch among them has no deadline to answer within — so a
            // remote dropping packets would hold this lock indefinitely, and
            // every close in the workbench behind it, a close being the one
            // thing that must never be held. Taken inside the blocking half and
            // handed back out of it, because the record it has to reach is on
            // the other side.
            let making = checkouts.blocking_lock_owned();

            make(&planned).map_err(Unmade::grilling)?;

            Ok((commit, path, branch, recorded(&planned), Some(making)))
        }
    })
    .await?;

    let (commit, path, cut, checkouts, making) = match made {
        Ok(made) => made,
        Err(refusal) => return Ok(refusal),
    };

    // The record catches up with the name the branch was actually cut on, which
    // is the name it was carrying wherever that one was free — which is almost
    // always, and always for a name the human typed.
    if cut != branch {
        store::reinvent_branch(pool, id, &cut).await?;
    }

    let moved = match grilled {
        true => store::start_grilling(pool, id, &commit, &path, &checkouts).await?,
        false => store::start_building(pool, id, &commit, &path, &checkouts).await?,
    };

    match moved {
        store::Grilling::NoSuchConversation => return Ok(GrillingStarted::NoSuchConversation),
        store::Grilling::NotDrafting => return Ok(GrillingStarted::NotDrafting),
        store::Grilling::Started => {}
    }

    // The record names them now, so the sweep would keep them: released here
    // rather than at the end, because what follows is a launch and holding a
    // lock across one would hold every other start behind it. Nothing to
    // release where the Conversation already had its checkouts — that made no
    // directory, so there was no window to hold.
    drop(making);

    // From here the Conversation says it is being worked on, and the thing that
    // will say so is a session that does not exist yet. So a registration stands
    // in for it across the launch, which is the slowest part of this: a sweep
    // that looked in between would find a Conversation being worked with nothing
    // working on it, and stop the run under a press the human is still standing
    // at. Held to the
    // end of this rather than handed on — what drives a grilling from there is
    // its session — and what it leaves behind where the launch fails is a stall
    // for the next sweep to find. See [`crate::drivers`] and [`crate::stalls`].
    let _driving = state.drivers.driving(id);

    // A start with no grilling in it is a run rather than an interview, so what
    // follows is the runner's: a session on the implementation skill, watched out
    // to the pull request and the wrap-up exactly as an inline implementation
    // picked at the end of a grilling is. It takes a registration of its own —
    // see [`crate::runner::build_the_ungrilled`] — so the one above can go when
    // this press does.
    if !grilled {
        crate::runner::build_the_ungrilled(state, id);

        return Ok(GrillingStarted::Started);
    }

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
    if let Some(pairing) = conversation.grilling_pairing.pairing().cloned()
        && let Some(prompt) = state
            .sessions
            .skills()
            .map(|skills| skills::grilling(skills, &brief))
        && let Err(error) = state
            .sessions
            .start(pool, &state.nudges, &conversation, &pairing, &prompt)
            .await
    {
        tracing::error!(error = ?error, conversation_id = id, "a grilling session could not be started");
    }

    Ok(GrillingStarted::Started)
}

/// One checkout a grill start is about to make: which repository, where it goes,
/// what it holds and what it came off.
///
/// The Conversation's own and each of its companions, in the one shape, because
/// from the moment they are planned they are the same thing — a worktree of a
/// registered repository. What differs between them is two fields, and both of
/// them read as what they are: a companion is named, and a checkout that holds
/// no branch is detached.
struct Checkout {
    /// The companion Repo this is a checkout of — its id and what it is called —
    /// or `None` for the Conversation's own.
    ///
    /// The id is what the record is written against, and the name is what a
    /// refusal says. Together they are the whole of what a companion's checkout
    /// needs that the Conversation's does not.
    companion: Option<(i64, String)>,

    /// The repository the worktree is made from.
    repo: PathBuf,

    /// Where the checkout goes, under the Data Directory.
    path: PathBuf,

    /// The branch to cut, or `None` for a detached checkout — which is what a
    /// read-only companion gets, having nothing to commit and no business
    /// taking a name in somebody else's repository.
    branch: Option<String>,

    /// The commit its base resolved to.
    commit: String,
}

impl Checkout {
    /// How git refusing to make this checkout is refused back: the
    /// Conversation's own repository says only that git would not, and a
    /// companion says which repository it was.
    fn refused(&self) -> Unmade {
        match &self.companion {
            Some((_, repo)) => Unmade::Companion {
                repo: repo.clone(),
                why: CompanionRefusal::WorktreeRefused,
            },
            None => Unmade::Own,
        }
    }
}

/// Why a start's checkouts could not be made, before it is put in the words of
/// whichever press asked.
///
/// [`Unready`]'s shape and for its reason: two presses take a Draft past
/// drafting — starting a grilling and adopting a stage — and each of them makes
/// the Conversation's own checkout and one per companion by the same rules. What
/// differs is only what the answer is called, so the reading is made once here
/// and spelled twice below.
enum Unmade {
    /// The Conversation's own checkout: git would not make the worktree.
    ///
    /// One case rather than four, because it is the only one of the four that
    /// reaches here. What the branch comes off and whether the name is free are
    /// asked of the Conversation's own repository before the list is built, and
    /// each press has its own words for those already.
    Own,

    /// One of its companions, named — because *which one* is the whole of what
    /// the human needs.
    Companion { repo: String, why: CompanionRefusal },
}

impl Unmade {
    /// Said to the press that starts a grilling.
    fn grilling(self) -> GrillingStarted {
        match self {
            Unmade::Own => GrillingStarted::WorktreeRefused,
            Unmade::Companion { repo, why } => GrillingStarted::Companion { repo, why },
        }
    }

    /// And to the press that adopts a stage.
    fn adopting(self) -> Adopted {
        match self {
            Unmade::Own => Adopted::WorktreeRefused,
            Unmade::Companion { repo, why } => Adopted::Companion { repo, why },
        }
    }
}

/// Ask git everything one companion's checkout turns on, and come back with what
/// it will be.
///
/// Fetch, then resolve, then check the branch — the Conversation's own
/// repository's order, for the Conversation's own repository's reasons, and each
/// failure refused by the same name with the repository said. A companion whose
/// repository has no remote has nothing to fetch and is never refused for it.
///
/// Nothing is made here. What comes back is a plan, and the making waits until
/// every companion has one: that is what lets a start that cannot deliver one
/// companion refuse without having made another.
///
/// `planned` is what the start has claimed so far, which is what stops two
/// companions being handed one directory — see [`worktrees::unclaimed_path`].
fn plan(
    data: &Path,
    id: i64,
    branch: &str,
    companion: store::Companion,
    planned: &[Checkout],
) -> Result<Checkout, Unmade> {
    let repo = companion.repo.path.clone();
    let refused = |why| Unmade::Companion {
        repo: companion.repo.name.clone(),
        why,
    };

    if let worktrees::Fetched::Failed(said) = worktrees::fetch(&repo) {
        tracing::error!(
            said,
            repo = %repo.display(),
            "fetching a companion Repo's remotes failed, so the start is not being made",
        );

        return Err(refused(CompanionRefusal::FetchFailed));
    }

    // The branch of that repository's own the human picked, or its default
    // branch as origin holds it — the rule the Conversation's base follows,
    // asked of the companion's repository.
    let named = match companion.base_ref.clone() {
        Some(picked) => picked,
        None => worktrees::default_ref(&repo, &companion.repo.default_branch),
    };

    let Some(commit) = worktrees::resolve(&repo, &named) else {
        return Err(refused(CompanionRefusal::NoBaseCommit));
    };

    // A read-write companion is cut a branch of its own: the one that was typed,
    // or the Conversation's where nothing was, which is what mirroring is. A
    // read-only one takes no name at all.
    let cut = companion.branch_for(branch);

    if let Some(cut) = &cut
        && worktrees::branch_exists(&repo, cut)
    {
        return Err(refused(CompanionRefusal::BranchExists));
    }

    // Named for the Repo and what the checkout holds, as the Conversation's own
    // is: the branch where there is one, and otherwise the base it stands at —
    // a read-only companion holds no branch to be named for.
    let holding = cut.clone().unwrap_or_else(|| named.clone());
    let claimed: Vec<PathBuf> = planned
        .iter()
        .map(|checkout| checkout.path.clone())
        .collect();
    let path = worktrees::unclaimed_path(data, id, &companion.repo.name, &holding, &claimed);

    Ok(Checkout {
        companion: Some((companion.repo.id, companion.repo.name)),
        repo,
        path,
        branch: cut,
        commit,
    })
}

/// Make every checkout of a start, or unmake the ones already made and say which
/// one would not be.
///
/// The one place either press creates anything, which is what makes *leaves
/// nothing behind* something to hold rather than something to hope for. What is
/// unwound is directory and branch together — see [`worktrees::unmake`] —
/// because a branch cut moments ago by a start that then refused holds nothing
/// worth keeping.
fn make(planned: &[Checkout]) -> Result<(), Unmade> {
    for (nth, checkout) in planned.iter().enumerate() {
        let made = match &checkout.branch {
            Some(branch) => {
                worktrees::add(&checkout.repo, &checkout.path, branch, &checkout.commit)
            }
            None => worktrees::add_detached(&checkout.repo, &checkout.path, &checkout.commit),
        };

        if made {
            continue;
        }

        // This one included, and first: an `add` that fell over may have made
        // the directory, or the branch, or neither, and what is being unwound is
        // whatever it did get as far as. The rest newest first, which is the
        // order they were made in reversed — nothing turns on it, no two of
        // these being in one repository, but a list is undone backwards.
        for done in planned[..=nth].iter().rev() {
            worktrees::unmake(&done.repo, &done.path, done.branch.as_deref());
        }

        return Err(checkout.refused());
    }

    Ok(())
}

/// Where each companion of a start was checked out and what it was cut from,
/// for the record that follows the work.
///
/// The commit as well as the directory, because a companion's base is a *name*
/// on its row and a name moves: a read-only companion is detached at whatever
/// that name came to at this moment, and this is the only thing that will ever
/// know which commit that was.
///
/// The Conversation's own is not among them: it goes on the row the store has
/// always kept for it, one per Conversation.
fn recorded(planned: &[Checkout]) -> Vec<store::CompanionWorktree> {
    planned
        .iter()
        .filter_map(|checkout| {
            let (repo_id, _) = checkout.companion.as_ref()?;

            Some(store::CompanionWorktree {
                repo_id: *repo_id,
                path: checkout.path.clone(),
                base_commit: Some(checkout.commit.clone()),
            })
        })
        .collect()
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
/// **And the companions are checked out with it**, by the same [`plan`],
/// [`make`] and [`recorded`] a grill start uses. An adopting Conversation
/// drafts like any other and its setup card configures companions like any
/// other's, and this is the other press that takes a Draft past drafting — so a
/// stage adopted without them would be a session quietly missing a repository
/// the human put there. Refused by name where one cannot be delivered, and
/// nothing left behind: the branch and every directory are unmade together.
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

    // And the same stand-down the press beside this one makes, in the same place
    // and for the same reason: what adopting makes is where a session works, and
    // a stage taken up on a build that runs none is a stage nothing will start
    // on. See [`start_grilling`].
    if state.sessions.here().absent() {
        return Ok(Adopted::NotOnWindowsYet);
    }

    // The one thing about the roadmap that is Verkstead's, and the whole of what
    // makes this Conversation an adopting one. Everything else about it is read
    // back out of the repository.
    let Some(roadmap) = conversation.adopting.clone() else {
        return Ok(Adopted::NotAdopting);
    };

    // Every Profile, before anything that costs a git call — the cheap answers
    // first, which is the order [`start_grilling`] checks the same set in. Read
    // as rows rather than judged off the ids, because a Profile whose pair has
    // gone is not one to run a session under and the id alone cannot say so.
    //
    // All of them, rather than only the one the work runs under: a stage
    // inherits every one from its predecessor, so what this one is adopted with
    // is what every stage after it starts with.
    let grilling = crate::profiles::picked(watched, conversation.grilling_pairing.clone()).await?;
    let implementation =
        crate::profiles::pairing(watched, conversation.implementation_pairing.clone()).await?;
    let review = crate::profiles::picked(watched, conversation.review_pairing.clone()).await?;

    if let Some(refusal) = unready(&grilling, implementation.as_ref(), &review) {
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

            // Both, because the stacking question below is *is this the default
            // branch or something else*, and the default's own name is half of
            // that. Resolved after the fetch, like everything else here.
            let default = worktrees::default_ref(&repo, &default);

            let named = picked.unwrap_or_else(|| default.clone());

            let Some(commit) = worktrees::resolve(&repo, &named) else {
                return Err(Adopted::NoBaseCommit);
            };

            // The same rule the notice was drawn by and the page was drawn by,
            // asked here at the base commit — and asked again, rather than
            // taken from either, because a roadmap is a document anybody may
            // have moved since. Which clause refused it is the answer to the
            // button: each of them is a different thing to go and do about it.
            match crate::stages::startable(&repo, &commit, &roadmap) {
                Startable::Stage(abandoned) => {
                    let stacks_on = predecessor(&repo, &commit, &named, &default);

                    Ok((commit, named, abandoned.stage, stacks_on))
                }
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
    let (commit, named, stage, stacks_on) = match read {
        Ok(read) => read,
        Err(refusal) => return Ok(refusal),
    };

    // The stage's own name, as the unattended start names one — its brief's
    // filename under the roadmap it belongs to. The name the Conversation was
    // started under was the server's invention for a row in the sidebar, and it
    // is discarded here: a stage is worked on the branch its roadmap will
    // annotate it with.
    let branch = stage.branch();
    let path = worktrees::worktree_path(&state.data_dir, id, &conversation.repo.name, &branch);

    // Every checkout the stage needs, planned before any of it is made: the
    // branch off the commit above, and one per companion the human configured
    // while this was drafting. An adopting Conversation is a Draft like any
    // other, so its setup card put those rows there like any other's — and
    // adoption is the other press that takes a Draft past drafting, so it is the
    // other press that owes them a directory. Without this the stage would run
    // with companions the sandbox skips in silence.
    //
    // The same [`plan`], [`make`] and [`recorded`] a grill start uses, at the
    // stage's own branch: a read-write companion mirrors that where its row
    // names nothing, so the branch cut beside the stage is the stage's own.

    let made = tokio::task::spawn_blocking({
        let path = path.clone();
        let branch = branch.clone();
        let commit = commit.clone();
        let data_dir = state.data_dir.clone();
        let companions = conversation.companions.clone();
        let checkouts = state.checkouts.clone();

        move || {
            let mut planned = vec![Checkout {
                companion: None,
                repo,
                path,
                branch: Some(branch.clone()),
                commit,
            }];

            for companion in companions {
                let beside =
                    plan(&data_dir, id, &branch, companion, &planned).map_err(Unmade::adopting)?;

                planned.push(beside);
            }

            // And only now, held from the first directory this makes to the
            // record naming it, as a grill start holds it and for its reason: a
            // directory made and not yet recorded is one a sweep would read as
            // nobody's. Here rather than around the whole of this, because
            // [`plan`] fetches once per companion and a fetch has no deadline
            // to answer within. See [`crate::AppState::checkouts`].
            let making = checkouts.blocking_lock_owned();

            make(&planned).map_err(Unmade::adopting)?;

            Ok((recorded(&planned), making))
        }
    })
    .await?;

    let (checkouts, making) = match made {
        Ok(made) => made,
        Err(refusal) => return Ok(refusal),
    };

    // And now the store, in the order the record is read in: the branch it is on,
    // the Brief it works from, and then the move that freezes both.
    store::rename_branch(pool, id, Some(branch.as_str())).await?;
    store::save_brief(pool, id, &stage.brief).await?;

    // The companion checkouts with it, in the transaction that makes it a stage
    // — the reason [`crate::continuing`] writes its own there: a Conversation
    // that said it was implementing without saying where its companions went
    // would be one nothing could bind into a sandbox and nothing would come back
    // and remove.
    match store::start_stage(pool, id, &commit, &path, stacks_on.as_deref(), &checkouts).await? {
        store::Staged::Started => {}
        store::Staged::NoSuchConversation => return Ok(Adopted::NoSuchConversation),
        store::Staged::NotDrafting => return Ok(Adopted::NotDrafting),
    }

    // Recorded, so the sweep would keep them. What follows is a Timeline and a
    // launch, and neither makes a directory.
    drop(making);

    // What was adopted, from where, and where its branch came off — on the
    // Conversation's own Timeline, because that is the only Timeline there is:
    // adoption has no predecessor Conversation for the human to have been
    // watching.
    if let Err(error) = store::note(
        pool,
        id,
        &adopted(&stage, &branch, stacks_on.as_deref(), &named),
    )
    .await
    {
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

    // Taken here rather than by the planning, which is started from more than one
    // place and takes the registration from all of them — see
    // [`crate::runner::plan_stage`].
    let driving = state.drivers.driving(id);

    tokio::spawn(crate::runner::plan_stage(
        state.clone(),
        id,
        stacks_on,
        driving,
    ));

    Ok(Adopted::Adopted)
}

/// What an adopting Conversation's Timeline is told: which stage of which
/// roadmap was adopted, and where its branch came off.
///
/// [`crate::continuing::begun`]'s wording, with the two things adoption changes
/// taken out of it. *With nobody asked* goes, because somebody did: a human
/// pressed this. What stays is both halves: an adopted stage stacks on the base
/// the human fixed it to, wherever that base is a predecessor there is anything
/// left to stack on, and the half it did not take is as much worth saying as
/// the half it did.
fn adopted(
    stage: &crate::stages::Stage,
    branch: &str,
    stacks_on: Option<&str>,
    from: &str,
) -> String {
    // The brief named rather than linked, as the unattended start names it: it
    // is a path in a Worktree the workbench has no route to, and a link that
    // went nowhere would be worse than the path itself.
    let started = format!(
        "Stage {} of the `{}` roadmap — *{}* — was adopted from `{}`.",
        stage.label, stage.roadmap, stage.title, stage.brief_path,
    );

    match stacks_on {
        Some(predecessor) => format!(
            "{started} Its branch `{branch}` stacks on `{predecessor}`, the base this \
             Conversation was fixed to, the way this repository's `{}` records.",
            crate::stages::GIT_WORKFLOW,
        ),
        None => format!(
            "{started} Its branch `{branch}` came off `{from}` and stacks on nothing: the base \
             is the default branch, or work already in it, or this repository's `{}` records no \
             way to stack a roadmap stage on the one before it.",
            crate::stages::GIT_WORKFLOW,
        ),
    }
}

/// The branch an adopted stage stacks on, where there is one to stack on.
///
/// Stacking is what a stage does when the work it builds on is finished and not
/// yet merged, and for an adopted stage that work is whatever the human fixed
/// the base to — there being no predecessor Conversation here to take it from,
/// which is the one thing adoption does differently. Four things have to hold,
/// and each of them is a way of there being nothing to stack on rather than a
/// failure:
///
/// - **The base is not the default branch.** A stage off the default branch is
///   the ordinary unstacked case: there is no predecessor, which is what
///   picking no base means.
/// - **It names a local branch.** A stack holds branches, so a base given as a
///   raw commit or as a remote-tracking ref is not one a stack could be told
///   about. Asked as [`worktrees::branch_exists`] rather than the fail-safe
///   reading, because a git that would not answer is a reason to leave the
///   bookkeeping alone rather than to invent some.
/// - **It is not already in the default branch.** A merged predecessor is work
///   that has landed, and its pull request is closed: there is nothing left for
///   a stack to hold it in.
/// - **The repository records a stacking mechanism**, at the base commit, which
///   is [`crate::stages::stacks_at`]'s question and the same one the unattended
///   path asks of a Worktree. Verkstead carries no mechanism of its own.
fn predecessor(repo: &Path, commit: &str, named: &str, default: &str) -> Option<String> {
    if named == default || !worktrees::branch_exists(repo, named) {
        return None;
    }

    if worktrees::merged(repo, commit, default) != Some(false) {
        return None;
    }

    crate::stages::stacks_at(repo, commit).then(|| named.to_owned())
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
///
/// **Every companion's worktree goes the same way, and every companion's branch
/// stays.** A companion is somewhere a Conversation was given to work and the
/// Conversation has stopped, so the directory is given back; what a read-write
/// companion committed is on its branch, which is a name and a commit and may
/// hold work worth reading.
///
/// But a worktree that will not go does not hold the close up — a companion's no
/// more than the Conversation's own. Git refuses to remove a directory it no
/// longer reads as a worktree — one hollowed out, one whose `.git` file has gone,
/// one whose repository has moved out from under it — and that is precisely the
/// state a human is trying to get out of when they press Close. Refusing them
/// would leave a Conversation nothing can ever end, which is worse than the
/// directory it was protecting. So the removal is attempted, a failure is logged
/// with the path in it, and the close is recorded regardless.
///
/// **And then the sweep this ends with takes the directory anyway.** What git's
/// polite removal could not have is exactly what [`worktrees::sweep`] is for:
/// nothing under the Data Directory outlives the record that named it, so a
/// directory the log has just been written about is one the close reclaims
/// before it returns, along with whatever earlier closes and crashes left
/// behind.
///
/// And what it was still drawing the human with goes last of all — the questions
/// it left open, in [`asked`], and the news mark it was carrying, in [`read`].
/// The record says Closed by then, which is the order the rest of this is in:
/// what has happened is written down, and then whatever outlived it is shut.
///
/// **The Conversation is read through [`store::closable`] rather than the whole
/// of it**, which is a decision about what this may be stopped by rather than
/// about how much is fetched. The full read parses the state word, the
/// direction and each companion's mode, and a stored word this Verkstead does
/// not know refuses all three — so a Conversation whose record has gone strange
/// used to be one nothing could ever end, which is the opposite of what Close
/// is for. This read parses nothing: it wants a repository path and a worktree
/// path per checkout, and the only thing it can be refused for is a
/// Conversation that is not there.
pub(crate) async fn close(state: &AppState, id: i64) -> Result<ConversationClosed> {
    let pool = &state.pool;

    let Some(conversation) = store::closable(pool, id).await? else {
        return Ok(ConversationClosed::NoSuchConversation);
    };

    state.sessions.end(id).await;

    if let Some(path) = conversation.worktree.clone() {
        let repo = conversation.repo.clone();
        let left = path.clone();

        let removed = tokio::task::spawn_blocking(move || worktrees::remove(&repo, &path)).await?;

        if !removed {
            tracing::warn!(
                conversation_id = id,
                worktree = %left.display(),
                "a Conversation's worktree could not be removed, so it was closed around it"
            );
        }
    }

    for companion in conversation.companions.clone() {
        let Some(path) = companion.worktree else {
            continue;
        };

        let repo = companion.repo.clone();
        let left = path.clone();

        let removed = tokio::task::spawn_blocking(move || worktrees::remove(&repo, &path)).await?;

        if !removed {
            tracing::warn!(
                conversation_id = id,
                repo = companion.name,
                worktree = %left.display(),
                "a companion repo's worktree could not be removed, so it was closed around it"
            );
        }
    }

    // And the directory beside it, with whatever the sessions put there. It is
    // given back for the reason the worktree is: it is somewhere a Conversation
    // was given to work, and the Conversation has stopped. Whatever it held that
    // was worth keeping is on the Timeline already.
    let handoffs = Handoffs::under(&state.data_dir);
    tokio::task::spawn_blocking(move || handoffs.remove(id)).await?;

    let closing = store::close_conversation(pool, id).await?;

    if closing == store::Closing::Closed {
        asked(state, id).await;
        read(state, id).await;
    }

    // And then the backstop under all of it: every directory under the worktrees
    // directory that no Conversation names any more. The targeted removals above
    // are still what ordinarily gives this Conversation its checkouts back —
    // this is what reclaims the ones git refused, which are exactly the ones the
    // warnings above have just been written about, along with whatever earlier
    // closes and crashes left behind. See [`worktrees::sweep`].
    //
    // After the record rather than before it: the rows this close deletes are
    // what makes its own directories orphans, and a sweep that ran first would
    // read them as live and leave them.
    worktrees::sweep(state).await;

    Ok(match closing {
        store::Closing::Closed => ConversationClosed::Closed,
        store::Closing::AlreadyClosed => ConversationClosed::AlreadyClosed,
        store::Closing::NoSuchConversation => ConversationClosed::NoSuchConversation,
    })
}

/// Lock whatever the Conversation was still asking, now that it has closed.
///
/// Closing takes away every session there will ever be, so an open Set is a
/// question with no reader and no reader coming: what the human wrote into one
/// would go nowhere, and the marks over it would go on saying somebody was
/// waiting. Locking unanswered is what that Set has always meant — see
/// [`crate::sets::lock`], which a relaunched grilling reaches for over the same
/// facts.
///
/// **Every open Set, Deferred Asks included**, which is the wider of the two
/// readings [`crate::sets::Open`] holds. A relaunch leaves a Deferred Ask
/// standing because the session after it will fold the answer in; here there is
/// no session after it, so the question is over with the Conversation.
///
/// The stop the Conversation carries is left exactly as it was. That is history
/// rather than something outstanding, and what stops it reading as *waiting on
/// you* is the Closed state itself — see [`crate::ui`], where the header's mark
/// is decided, and the sidebar's own `waiting` in the store.
///
/// Nothing is refused for, and nothing is read back: a Conversation is closed
/// whether or not the questions it left could be shut.
async fn asked(state: &AppState, id: i64) {
    let timeline = match store::timeline(&state.pool, id).await {
        Ok(timeline) => timeline,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading what a closed Conversation was asking failed");
            return;
        }
    };

    crate::sets::lock(
        state,
        id,
        &crate::sets::open(&timeline, crate::sets::Open::Either),
        "the Conversation that asked it is closed",
    )
    .await;
}

/// Take away the news mark, now that the Conversation has closed.
///
/// The other half of what [`asked`] does, over the other thing a closed
/// Conversation could still be drawing the human with. The mark means *there is
/// news here to read*, and closing is them saying the work is over wherever it
/// had got to — so there is nothing left to go and read, and a disc on the
/// Conversation they have just put away is the one that teaches them to stop
/// reading the discs.
///
/// Which is what the sidebar's own `waiting` says of a closed row already — see
/// the store's `conversations` — and the mark beside it is the same disc drawn
/// for the other reason, so the two say it together or the row goes on glowing
/// over nothing.
///
/// Cleared rather than hidden. The row is the whole of the mark, and a
/// Conversation steered back into life is not one the human has news about: what
/// they missed was the wrap-up they closed it in front of.
///
/// Nothing is refused for, and no Nudge is sent: the close announces the list
/// has moved on its own account, and every open page reads the row again on the
/// strength of that.
async fn read(state: &AppState, id: i64) {
    if let Err(error) = store::see_conversation(&state.pool, id).await {
        tracing::error!(error = ?error, conversation_id = id, "clearing the news mark on a closed Conversation failed");
    }
}

/// Why these Pairings are not something to run the work under, or `None`
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
/// starting the work and adopting a stage each want every role settled before
/// they will do anything, and each says so in its own words — see
/// [`Unready::grilling`] and [`Unready::adopting`].
fn unready(
    grilling: &PickedView,
    implementation: Option<&PairingView>,
    review: &PickedView,
) -> Option<Unready> {
    // The row that runs no session is a choice made, so it passes here as a
    // Pairing does — and leaves nothing to be broken, there being no Profile.
    let grilling = match grilling {
        PickedView::Skipped => None,
        PickedView::Under(pairing) if pairing.model.is_some() => Some(pairing),
        _ => return Some(Unready::NoGrillingProfile),
    };

    let Some(implementation) = implementation.filter(|pairing| pairing.model.is_some()) else {
        return Some(Unready::NoImplementationProfile);
    };

    let review = match review {
        PickedView::Skipped => None,
        PickedView::Under(pairing) if pairing.model.is_some() => Some(pairing),
        _ => return Some(Unready::NoReviewProfile),
    };

    [grilling, Some(implementation), review]
        .into_iter()
        .flatten()
        .any(|pairing| pairing.profile.broken.is_some())
        .then_some(Unready::ProfileBroken)
}

/// What is wrong with a Conversation's pair of Profiles, before it is put in
/// the words of whichever press asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unready {
    NoGrillingProfile,
    NoImplementationProfile,
    NoReviewProfile,
    ProfileBroken,
}

impl Unready {
    /// Said to the press that starts a grilling.
    fn grilling(self) -> GrillingStarted {
        match self {
            Unready::NoGrillingProfile => GrillingStarted::NoGrillingProfile,
            Unready::NoImplementationProfile => GrillingStarted::NoImplementationProfile,
            Unready::NoReviewProfile => GrillingStarted::NoReviewProfile,
            Unready::ProfileBroken => GrillingStarted::ProfileBroken,
        }
    }

    /// And to the press that adopts a stage.
    fn adopting(self) -> Adopted {
        match self {
            Unready::NoGrillingProfile => Adopted::NoGrillingProfile,
            Unready::NoImplementationProfile => Adopted::NoImplementationProfile,
            Unready::NoReviewProfile => Adopted::NoReviewProfile,
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

/// Whether everything needed before the work starts is settled, as the pane
/// reads it: the three roles, and a Brief with something in it.
///
/// Answered against what the endpoint has already read rather than by loading
/// the Conversation again — and it deliberately says nothing about the branch or
/// the base commit, which are decided against git when the button is pressed.
pub(crate) fn ready_to_grill(
    state: store::Lifecycle,
    grilling: &PickedView,
    implementation: Option<&PairingView>,
    review: &PickedView,
    brief: &str,
) -> bool {
    state == store::Lifecycle::Draft
        && !brief.trim().is_empty()
        && crate::profiles::ready_to_grill(grilling, implementation, review)
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

/// Colours, weather and temper — words that pair with anything below and carry
/// no meaning about the work, so a name that is left as it is never says
/// something untrue about the branch.
const QUALITIES: [&str; 32] = [
    "amber", "ashen", "autumn", "brisk", "cobalt", "copper", "crisp", "dusky", "eager", "ember",
    "flint", "frosted", "gilded", "hushed", "indigo", "ivory", "lucid", "mellow", "misty",
    "nimble", "ochre", "opal", "quiet", "rustic", "sable", "scarlet", "slate", "sunlit", "teal",
    "umber", "verdant", "wistful",
];

/// Birds, weather and landscape, for the same reason.
const THINGS: [&str; 32] = [
    "anchor", "beacon", "bramble", "cedar", "cinder", "coppice", "curlew", "delta", "eddy",
    "fathom", "gable", "harbour", "heron", "kestrel", "lantern", "meadow", "orchard", "otter",
    "pennant", "quarry", "ridge", "rookery", "sable", "shale", "sparrow", "thicket", "thistle",
    "tundra", "vale", "willow", "wren", "zephyr",
];

/// How many names the two lists spell between them, which is how far
/// [`free_branch_name`] walks before it gives up on a plain pair.
const PAIRS: u64 = QUALITIES.len() as u64 * THINGS.len() as u64;

/// The pair of words at `nth`, counted through every combination the two lists
/// hold and wrapping round at the end of them.
///
/// One number rather than two, so that a walk from anywhere reaches all of them:
/// the qualities turn over first and the things once per lap, and `PAIRS` steps
/// from any starting point is every name there is.
fn pair(nth: u64) -> String {
    let quality = QUALITIES[(nth % QUALITIES.len() as u64) as usize];
    let thing = THINGS[((nth / QUALITIES.len() as u64) % THINGS.len() as u64) as usize];

    format!("{quality}-{thing}")
}

/// A branch name to start a Conversation under, until the human names it
/// themselves.
///
/// Two words rather than a hash: it is a branch name, so it is going to be typed
/// and read aloud, and the whole point of prefilling one is that the human need
/// not stop and think of anything before they start writing the brief.
///
/// Nothing is asked of any repository here. A Draft's branch is a prefill that
/// is drawn nowhere and cut nowhere, so the moment the name has to be free is
/// the moment the branch is made, and that moment is [`free_branch_name`]'s.
fn branch_name() -> String {
    // A generator that could not answer is not a reason to refuse to start a
    // Conversation: the name is a prefill the human is free to replace, so an
    // unlucky machine gets the first pair rather than an error.
    pair(getrandom::u64().unwrap_or(0))
}

/// Every repository a Conversation's own branch name is about to be cut in: its
/// Repo, and each companion mirroring that name.
///
/// A companion with a name of its own is not one of them — that name is the
/// human's and does not move with this one, so a repository holding it is a
/// refusal about that companion rather than a reason to pick another name here.
pub(crate) fn cut_in(repo: &Path, companions: &[store::Companion]) -> Vec<PathBuf> {
    std::iter::once(repo.to_path_buf())
        .chain(
            companions
                .iter()
                .filter(|companion| companion.mirrors())
                .map(|companion| companion.repo.path.clone()),
        )
        .collect()
}

/// The name to cut a Conversation's branch under, where the name it is carrying
/// is one Verkstead invented.
///
/// `branch` itself wherever nothing in `repos` answers to it, which is almost
/// always, and another invented name where something does — see
/// [`free_branch_name`]. What each repository answers to is read once and whole:
/// its own branches and its remotes' both — see [`worktrees::cut_names`].
///
/// Both starts that cut a branch on an invented name come through here, and only
/// those: a name the human typed is refused on a repository that already has it
/// rather than picked around, that name being theirs and chosen.
pub(crate) fn name_to_cut(id: i64, branch: String, repos: &[PathBuf]) -> String {
    let taken: std::collections::HashSet<String> = repos
        .iter()
        .flat_map(|repo| worktrees::cut_names(repo))
        .collect();

    match taken.contains(&branch) {
        false => branch,
        true => free_branch_name(id, |name| !taken.contains(name)),
    }
}
/// A name for the branch a start is about to cut, where the one the Conversation
/// has been carrying is taken.
///
/// `free` is asked of every candidate, and it answers for every repository the
/// name is about to be cut in — the Conversation's own and each companion
/// mirroring it. A name free in one of them and taken in another is not a name
/// this start can use.
///
/// Every pair is tried, from a random one round to itself, so this reaches the
/// end empty-handed only where those repositories hold all thousand of them
/// between them. What stands behind that is the Conversation's id, which nothing
/// outside this Conversation collides with — and then a count, for the reason
/// [`worktrees::unclaimed_path`] carries one.
fn free_branch_name(id: i64, free: impl Fn(&str) -> bool) -> String {
    let picked = getrandom::u64().unwrap_or(0);

    let paired = (0..PAIRS)
        .map(|nth| pair(picked.wrapping_add(nth)))
        .find(|name| free(name));

    if let Some(name) = paired {
        return name;
    }

    let stem = pair(picked);

    std::iter::once(format!("{stem}-{id}"))
        .chain((2..).map(|nth| format!("{stem}-{id}-{nth}")))
        .find(|name| free(name))
        .expect("the count is unbounded, so some name is free")
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

    /// The property the whole of this turns on: the pairs are names, not
    /// samples of one. A word repeated in a list would quietly shrink the space
    /// a start picks from, and the smaller that space is the more often two
    /// Conversations of one Repo are handed the same branch.
    #[test]
    fn the_pairs_are_a_thousand_names_and_no_two_the_same() {
        let names: std::collections::HashSet<String> = (0..PAIRS).map(pair).collect();

        assert_eq!(names.len() as u64, PAIRS);
    }

    /// A name for a repository that already holds the one the Conversation was
    /// carrying: another pair, and one that repository does not have.
    #[test]
    fn a_taken_name_is_replaced_with_one_that_is_free() {
        let taken: std::collections::HashSet<String> = (0..PAIRS / 2).map(pair).collect();

        for _ in 0..16 {
            let name = free_branch_name(7, |name| !taken.contains(name));

            assert!(!taken.contains(&name), "{name:?} is taken");
            assert!(is_branch_name(&name), "git refused {name:?}");
        }
    }

    /// And it walks the whole list to find it, from wherever it starts: one free
    /// name among a thousand taken ones is still the one that comes back.
    #[test]
    fn the_one_free_name_is_the_one_that_comes_back() {
        let only = pair(11);

        assert_eq!(free_branch_name(7, |name| name == only), only);
    }

    /// A repository holding every pair there is falls back to the Conversation's
    /// id, which nothing outside that Conversation collides with.
    #[test]
    fn a_repository_holding_every_pair_falls_back_to_the_conversation() {
        let name = free_branch_name(7, |name| name.ends_with("-7"));

        assert!(name.ends_with("-7"), "{name:?} should carry the id");
        assert!(is_branch_name(&name), "git refused {name:?}");
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

    /// The whole of what an adopted stage stacks on: a base that is a local
    /// branch, is not the default branch, has not been merged into it, in a
    /// repository that records a mechanism.
    #[test]
    fn an_adopted_stage_stacks_on_the_unmerged_branch_it_was_based_on() {
        let repo = stacking();
        let tip = worktrees::resolve(&repo, "predecessor").expect("the branch resolves");

        assert_eq!(
            predecessor(&repo, &tip, "predecessor", "main"),
            Some("predecessor".to_owned()),
        );
    }

    /// Each of the four ways there is nothing to stack on, which are ways of
    /// this being an ordinary unstacked stage rather than ways of failing.
    #[test]
    fn a_base_with_nothing_left_to_stack_on_stacks_on_nothing() {
        let repo = stacking();
        let tip = worktrees::resolve(&repo, "predecessor").expect("the branch resolves");
        let main = worktrees::resolve(&repo, "main").expect("main resolves");

        assert_eq!(
            predecessor(&repo, &main, "main", "main"),
            None,
            "the default branch is what an unstacked stage comes off",
        );

        assert_eq!(
            predecessor(&repo, &tip, &tip, "main"),
            None,
            "a raw commit is not a branch a stack could be told about",
        );

        assert_eq!(
            predecessor(&repo, &tip, "nothing-by-that-name", "main"),
            None,
            "and neither is a name with no local branch behind it",
        );

        run(
            &repo,
            &["merge", "-q", "--no-ff", "-m", "merge it", "predecessor"],
        );

        assert_eq!(
            predecessor(&repo, &tip, "predecessor", "main"),
            None,
            "and a predecessor already in the default branch is finished work",
        );
    }

    /// The mechanism is the repository's, so a repository that records none has
    /// nothing to follow and the stage comes off its base unstacked.
    #[test]
    fn a_repository_recording_no_mechanism_stacks_nothing() {
        let repo = stacking();

        std::fs::write(
            repo.join(crate::stages::GIT_WORKFLOW),
            "# Git workflow\n\n## Review process\n\n### Finish sequence\n\nPush it.\n",
        )
        .unwrap();

        run(&repo, &["add", "-A"]);
        run(&repo, &["commit", "-m", "docs: no stacking here"]);
        run(&repo, &["checkout", "-q", "-B", "predecessor"]);
        run(&repo, &["checkout", "-q", "main"]);

        let tip = worktrees::resolve(&repo, "predecessor").expect("the branch resolves");

        assert_eq!(predecessor(&repo, &tip, "predecessor", "main"), None);
    }

    /// A repository that records a stacking mechanism, with one unmerged branch
    /// off its default branch to stand a stage on.
    fn stacking() -> PathBuf {
        // Leaked rather than returned beside the path: these tests are a handful
        // of git calls each, and a temporary directory that outlives the test
        // binary is the tidier of the two shapes to read.
        let dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
        let repo = dir.path().to_path_buf();

        run(&repo, &["init", "--initial-branch", "main"]);
        run(&repo, &["config", "user.email", "test@verkstead.invalid"]);
        run(&repo, &["config", "user.name", "Verkstead Test"]);

        let workflow = repo.join(crate::stages::GIT_WORKFLOW);
        std::fs::create_dir_all(workflow.parent().unwrap()).unwrap();
        std::fs::write(
            &workflow,
            "# Git workflow\n\n## Review process\n\n\
             ### Finish sequence\n\nPush it.\n\n\
             ### Stacking roadmap stages\n\n`gh stack init <predecessor> <new>`\n",
        )
        .unwrap();

        run(&repo, &["add", "-A"]);
        run(
            &repo,
            &["commit", "-m", "chore: how this repository reviews"],
        );

        run(&repo, &["checkout", "-q", "-b", "predecessor"]);
        std::fs::write(repo.join("predecessor.md"), "# the stage before\n").unwrap();
        run(&repo, &["add", "-A"]);
        run(&repo, &["commit", "-m", "feat: the stage before this one"]);
        run(&repo, &["checkout", "-q", "main"]);

        repo
    }

    fn run(dir: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .expect("git should be on the PATH for these tests");

        assert!(output.status.success(), "git {args:?} failed");
    }
}
