//! Starting the next stage of a roadmap, with nobody asked.
//!
//! This is the piece that makes the whole pipeline unattended rather than merely
//! gateless. A wrap-up settles — see [`crate::settling`] — and the stage after
//! the one that settled starts: a Conversation of its own, on a branch of its
//! own, in a session inside the bundled fork of next-stage. That fork writes
//! `.tasks/`, at which point the runner takes over and works the backlog to
//! empty, which finishes and opens a pull request, which wraps up, which starts
//! the stage after it. Nothing in the loop asks for permission.
//!
//! **One thing stops it**, and it stops it naturally: the fork's breakdown quiz
//! is a blocking ask, so the stage waits there and its Conversation carries
//! *blocked on you* until the human answers from wherever they are.
//!
//! **A stage is a Conversation of its own** rather than the old one carrying on.
//! A Conversation is one Repo, one branch and one Worktree, and a stage is one
//! branch and one review unit — so it cannot be anything else. It is created
//! against the same Repo, under the same Profiles, primed with the stage brief as
//! its Brief, and it goes straight to Implementing: the grilling that would have
//! settled the work already happened, and the brief is what it settled.
//!
//! **What is read is the Worktree**, by the same rule the pinned stage list is
//! drawn by — see [`crate::stages`] — so the list the human is watching and the
//! stage that starts next cannot come to disagree.
//!
//! **What is decided is where the branch goes**, and only that. Stages stack on
//! the unmerged predecessor where the target repository records how, and
//! Verkstead reads whether that block is there rather than carrying a stacking
//! mechanism of its own: the session follows what the block says, because the
//! mechanism is the repository's. Where there is no block there is no convention
//! to invent, so the branch comes off the default branch and the Timeline says
//! so.
//!
//! Nothing here is refused for and nothing is returned. It runs at the end of an
//! unattended run with nobody watching, and what it has to say it says on the
//! Timeline as a notice — which is what a decision taken while nobody was looking
//! is owed.

use std::path::Path;

use crate::AppState;
use crate::stages::{self, Next, Stage};
use crate::store;
use crate::worktrees;

/// Start the stage after `conversation_id`'s, where there is one.
///
/// Called when a wrap-up settles, on every Conversation rather than on the ones
/// somebody thought were roadmap stages: whether this is a stage of anything is
/// read off the branch, and a Conversation whose branch has written to no roadmap
/// quietly is not one.
pub(crate) async fn carry_on(state: AppState, conversation_id: i64) {
    // Starting the next stage is the largest thing this pipeline does unasked,
    // so it asks the gate every driver asks: a Conversation whose keyboard the
    // human has is one Verkstead advances nothing behind — see [`crate::hold`].
    state.sessions.until_handed_back(conversation_id).await;

    let Some(conversation) = load(&state, conversation_id).await else {
        return;
    };

    let (Some(worktree), Some(base)) = (
        conversation.worktree.clone(),
        conversation.base_commit.clone(),
    ) else {
        // No Worktree is an aborted Conversation, and no base commit is one that
        // never started grilling. Neither can have written to a roadmap.
        return;
    };

    let branch = conversation.branch.clone();

    // Both readings together, off the runtime's threads: a git read and a
    // handful of file reads.
    let read = tokio::task::spawn_blocking({
        let worktree = worktree.clone();
        move || {
            (
                stages::next_stage(&worktree, &base, &branch),
                stages::stacks(&worktree),
            )
        }
    })
    .await;

    let (next, stacks) = match read {
        Ok(read) => read,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a roadmap had left failed");
            return;
        }
    };

    let stage = match next {
        Next::Stage(stage) => *stage,
        // The ordinary Conversation: an inline run or a feature's backlog, whose
        // branch touched no roadmap. Nothing to carry on and nothing to say —
        // there was never a roadmap for the human to wonder about.
        Next::NoRoadmap => return,
        Next::Complete { roadmap } => {
            tracing::info!(
                conversation_id,
                roadmap,
                "every stage of the roadmap is done, so nothing was started",
            );

            return say(
                &state,
                conversation_id,
                &format!(
                    "Every stage of the `{roadmap}` roadmap is done, so there is no stage to \
                     start. The roadmap is complete."
                ),
            )
            .await;
        }
        Next::Unstartable { why } => {
            tracing::warn!(conversation_id, why, "the next stage could not be started");

            return say(
                &state,
                conversation_id,
                &format!("The next stage of the roadmap could not be started: {why}."),
            )
            .await;
        }
    };

    start(&state, &conversation, conversation_id, stage, stacks).await;
}

/// Start `stage` as a Conversation of its own against the same Repo.
///
/// The order is the order [`crate::conversations::start_grilling`] does the same
/// job in, and for the same reason: git makes the branch and the worktree, and
/// only then does the store hear about it. A row saying a stage is under way with
/// nothing checked out would be a Conversation nothing could run and nothing
/// would clean up.
///
/// Everything that stops it stops it with a notice on the Timeline of the
/// Conversation that has just settled. That is where the human is looking — the
/// stage that would have carried it on does not exist, so there is no Timeline of
/// its own to say anything on.
async fn start(
    state: &AppState,
    conversation: &store::Conversation,
    settled: i64,
    stage: Stage,
    stacks: bool,
) {
    let branch = stage.branch();
    let repo = conversation.repo.path.clone();

    // Both Profiles are the predecessor's: a stage is the same work by the same
    // hands, one branch further on. The implementation one is what the session
    // runs under, and without it there is nothing to run.
    if conversation.implementation_profile.is_none() {
        return say(
            state,
            settled,
            &format!(
                "Stage {} of the `{}` roadmap is next, and this Conversation's implementation \
                 Profile has gone, so there is no account to run it under. Nothing was started.",
                stage.label, stage.roadmap,
            ),
        )
        .await;
    }

    // A branch by that name already is a stage somebody — or some earlier run —
    // has started already. Refused rather than worked around: the alternative is
    // a second Conversation quietly doing a stage that is already under way, on a
    // branch named after neither of them.
    if taken(&repo, &branch).await {
        return say(
            state,
            settled,
            &format!(
                "Stage {} of the `{}` roadmap is next, and `{branch}` is already a branch of \
                 this repository — so it looks to have been started already. Nothing was \
                 started.",
                stage.label, stage.roadmap,
            ),
        )
        .await;
    }

    // Where the branch goes, which is the whole of what Verkstead decides about
    // stacking. The predecessor's branch is the one this stage's work builds on,
    // and its tip is where the branch starts.
    let stacked_on = stacks.then(|| conversation.branch.clone());
    let from = stacked_on
        .clone()
        .unwrap_or_else(|| conversation.repo.default_branch.clone());

    let started = store::start_conversation(&state.pool, conversation.repo.id, &branch).await;

    let id = match started {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::error!(
                settled,
                "the Repo the stage would be against has gone, so nothing was started"
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, settled, "starting the next stage's Conversation failed");
            return;
        }
    };

    // Everything the human would have settled before pressing anything: who it
    // runs as, and what it is about. Both while it is still drafting, which is
    // the only state either can be recorded in.
    if let Err(error) = settle(state, id, conversation, &stage).await {
        tracing::error!(error = ?error, settled, stage = id, "preparing the next stage's Conversation failed");
        return gave_up(state, id).await;
    }

    let path = worktrees::worktree_path(&state.data_dir, id, &conversation.repo.name, &branch);

    let made = tokio::task::spawn_blocking({
        let path = path.clone();
        let branch = branch.clone();
        let from = from.clone();
        move || {
            let commit = worktrees::resolve(&repo, &from)?;

            worktrees::add(&repo, &path, &branch, &commit).then_some(commit)
        }
    })
    .await;

    let commit = match made {
        Ok(Some(commit)) => commit,
        Ok(None) => {
            say(
                state,
                settled,
                &format!(
                    "Stage {} of the `{}` roadmap could not be given a worktree on `{branch}` \
                     off `{from}`. Nothing was started.",
                    stage.label, stage.roadmap,
                ),
            )
            .await;

            return gave_up(state, id).await;
        }
        Err(error) => {
            tracing::error!(error = ?error, settled, stage = id, "making the next stage's worktree failed");
            return gave_up(state, id).await;
        }
    };

    match store::start_stage(&state.pool, id, &commit, &path, stacked_on.as_deref()).await {
        Ok(store::Staged::Started) => {}
        Ok(refused) => {
            tracing::error!(
                settled,
                stage = id,
                refused = ?refused,
                "the next stage's Conversation could not be set working",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, settled, stage = id, "recording the next stage failed");
            return;
        }
    }

    // What Verkstead decided, on both Timelines: on the stage's, because the
    // branch it is on was nobody's choice but this; and on the settled one,
    // because that is where the human was watching when it happened.
    say(
        state,
        id,
        &begun(&stage, &branch, stacked_on.as_deref(), &from),
    )
    .await;
    say(
        state,
        settled,
        &format!(
            "Stage {} of the `{}` roadmap — *{}* — has started as a Conversation of its own, \
             on `{branch}`.",
            stage.label, stage.roadmap, stage.title,
        ),
    )
    .await;

    tracing::info!(
        settled,
        stage = id,
        branch,
        label = stage.label,
        roadmap = stage.roadmap,
        "the next stage of the roadmap has started",
    );

    // Two Timelines moved and a Conversation appeared in the sidebar, and an open
    // page should say so without being reloaded.
    state.nudges.announce();

    tokio::spawn(crate::runner::plan_stage(state.clone(), id, stacked_on));
}

/// What the stage's own Timeline is told: which stage it is, and where its
/// branch came from.
///
/// Both halves said plainly, including the half that is an absence — a stage off
/// the default branch because the repository records no way to stack one is a
/// decision, and one the human may want to do something about.
fn begun(stage: &Stage, branch: &str, stacked_on: Option<&str>, from: &str) -> String {
    // The brief named rather than linked: it is a path in a Worktree the
    // workbench has no route to, and a link that went nowhere would be worse
    // than the path itself.
    let started = format!(
        "Stage {} of the `{}` roadmap — *{}* — started from `{}` with nobody asked.",
        stage.label, stage.roadmap, stage.title, stage.brief_path,
    );

    match stacked_on {
        Some(predecessor) => format!(
            "{started} Its branch `{branch}` stacks on `{predecessor}`, the branch of the stage \
             before it, the way this repository's `{}` records.",
            stages::GIT_WORKFLOW,
        ),
        None => format!(
            "{started} Its branch `{branch}` came off `{from}`: this repository's `{}` records \
             no way to stack a roadmap stage on the one before it, and there is no convention \
             to invent.",
            stages::GIT_WORKFLOW,
        ),
    }
}

/// Give the new Conversation everything a human would have settled before
/// pressing anything: the two Profiles, and the stage brief as its Brief.
///
/// The Profiles are the predecessor's, both of them. The implementation one is
/// what the work runs under; the grilling one is carried across because a stage
/// that is reopened later is grilled by whatever the roadmap's work has been
/// grilled by all along.
async fn settle(
    state: &AppState,
    id: i64,
    conversation: &store::Conversation,
    stage: &Stage,
) -> anyhow::Result<()> {
    if let Some(grilling) = &conversation.grilling_profile {
        store::set_grilling_profile(&state.pool, id, grilling.id).await?;
    }

    if let Some(implementation) = &conversation.implementation_profile {
        store::set_implementation_profile(&state.pool, id, implementation.id).await?;
    }

    store::save_brief(&state.pool, id, &stage.brief).await?;

    Ok(())
}

/// Stop the half-made Conversation, where a stage got as far as a record and no
/// further.
///
/// Aborted rather than left drafting, because drafting is a Conversation waiting
/// for a human to write a Brief and press a button — and this one is a stage
/// nobody is going to start by hand. Aborting is the work stopping wherever it
/// was, which is exactly what happened.
///
/// Nothing was checked out by the time this can run, so there is nothing to
/// clean up but the row.
async fn gave_up(state: &AppState, id: i64) {
    if let Err(error) = store::abort_conversation(&state.pool, id).await {
        tracing::error!(error = ?error, conversation_id = id, "stopping a half-made stage failed");
    }
}

/// Whether `repo` already has a branch by that name.
async fn taken(repo: &Path, branch: &str) -> bool {
    let repo = repo.to_owned();
    let branch = branch.to_owned();

    tokio::task::spawn_blocking(move || worktrees::branch_exists(&repo, &branch))
        .await
        .unwrap_or_else(|error| {
            tracing::error!(error = ?error, "asking whether a branch was taken failed");
            // Reads as taken, which is the right way round for the one thing it
            // decides: what is on the other side of it is making a branch and
            // letting an agent loose on it.
            true
        })
}

/// Put a notice on a Timeline.
///
/// Nothing is refused for: by the time anything here has something to say, what
/// it is saying has already happened. A notice that could not be written is a
/// line in the log and no more.
async fn say(state: &AppState, conversation_id: i64, markdown: &str) {
    match store::note(&state.pool, conversation_id, markdown).await {
        Ok(true) => state.nudges.announce(),
        Ok(false) => tracing::error!(
            conversation_id,
            "there is no Conversation left to say anything on"
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "putting a notice on a Timeline failed")
        }
    }
}

/// The Conversation whose wrap-up has settled, or `None` where there is nothing
/// left to read.
async fn load(state: &AppState, conversation_id: i64) -> Option<store::Conversation> {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(conversation) => conversation,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation that settled failed");
            None
        }
    }
}
