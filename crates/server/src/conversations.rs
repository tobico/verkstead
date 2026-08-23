//! Starting a Conversation and changing what the human may change about one:
//! everything between a workbench form and a row in the store.
//!
//! Two of the three edits are decided against the repository rather than taken
//! on trust, and git is the one asked both times — whether a name is one it
//! would take for a branch, and whether anything in the repository answers to
//! what was typed as a base commit. Refused here rather than at grill start,
//! where a bad name or a commit that is not there would be a failure with nobody
//! watching.
//!
//! Starting the grilling is where a Conversation stops being a record and gets
//! somewhere to work — see [`start_grilling`] — and where the session that does
//! the work is launched in it. Adopting is the same moment by the other door:
//! a roadmap Verkstead did not write has its next stage started here, with the
//! human's press standing in for the predecessor that would otherwise have
//! started it — see [`adopt`]. Aborting is where both are given back: the
//! session ends, and then the worktree goes.

use std::path::{Path, PathBuf};

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{
    Adopted, BaseRecorded, BranchRenamed, BriefSaved, ConversationAborted, DirectionChosen,
    GrillingStarted, ProfileEntry, Started, Worktree,
};
use verkstead_schema::Direction;

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
pub(crate) async fn start(pool: &SqlitePool, repo_id: i64) -> Result<Started> {
    Ok(
        match store::start_conversation(pool, repo_id, &branch_name()).await? {
            Some(id) => Started::Started { id },
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
    pool: &SqlitePool,
    repo_id: i64,
    roadmap: &str,
) -> Result<Started> {
    Ok(
        match store::start_adoption(pool, repo_id, &branch_name(), roadmap).await? {
            Some(id) => Started::Started { id },
            None => Started::NoSuchRepo,
        },
    )
}

/// Finish what answering a Question Set started, and say what it did to the
/// Conversation it was asked from.
///
/// The store hands back what happened and says nothing about it — see
/// [`store::Taken`] — so this is where a proposal settled becomes a line in the
/// log. Three of the four outcomes are unremarkable and one cannot happen, which
/// is exactly why it is worth saying when it does.
///
/// An accepted proposal is also the moment the grilling is over, and three
/// things follow from that. The session that proposed is ended: it has its
/// Response, there is nothing left for it to do, and what comes next runs in the
/// same worktree under a different account. Its handoff is taken onto the
/// Timeline, because the directory it was written in is Verkstead's own scratch
/// space and the Timeline is where a Conversation's documents live. And the
/// direction the human picked is acted on there and then — the pick rides the
/// Set, so accepting and choosing are one answer and there is nothing left to
/// press.
///
/// None of the three is refused for: by the time this runs the Response is
/// stored and the Conversation has moved. What a session that would not end or a
/// handoff that could not be read leaves behind is something to see in the log,
/// and no more than that. An Interruption is raised about a session that ran and
/// went wrong — see [`crate::interruptions`] — and neither of these is one: the
/// grilling is over either way, and what follows it starts from the Brief.
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
            directing: Directing::Moved,
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
                "a wrap-up proposal names a Conversation that is not there, so nothing moved"
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

    state.sessions.end(conversation_id).await;

    if let Err(error) = take_handoff(state, conversation_id).await {
        tracing::error!(error = ?error, conversation_id, "the handoff could not be put on the Timeline");
    }

    // And the pick, through the one path a direction is ever acted on. Logged
    // rather than raised for the reason the two above are: the Response is
    // stored and the grilling is over, so what a start that did not happen
    // leaves is a Conversation to look at rather than an answer to refuse.
    match choose_direction(state, conversation_id, picked).await {
        Ok(DirectionChosen::Chosen) => {}
        Ok(refused) => tracing::error!(
            conversation_id,
            ?refused,
            "the direction picked on the closing Set was not acted on"
        ),
        Err(error) => tracing::error!(
            error = ?error,
            conversation_id,
            "acting on the direction picked on the closing Set failed"
        ),
    }
}

/// Take the handoff document the grilling wrote and put it on the Timeline.
///
/// Taken after the session has ended, so that what is read is a finished
/// document rather than one still being written. A grilling that never wrote one
/// leaves nothing to take, which is a thing to note and not a failure — see
/// [`crate::handoffs::Handoffs::take`].
async fn take_handoff(state: &AppState, conversation_id: i64) -> Result<()> {
    let handoffs = Handoffs::under(&state.state_dir);

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

/// Record how the human chose to have the work built, and set it building.
///
/// The choice is recorded first and on its own, because it is a different thing
/// from the work starting: the Timeline gets the choice as an Event and then the
/// move that follows from it, and a Conversation whose direction is settled but
/// whose session would not start still says which way it was headed.
///
/// All three start something. Inline starts a session that builds the work; a
/// task list starts one that breaks it into `.tasks/` first, and the sessions
/// that work through that backlog follow from what it commits; a staged roadmap
/// starts one that writes `docs/roadmaps/` and carries the branch to a pull
/// request, and the stages it plans become Conversations of their own.
///
/// Reached from the pick on an accepted proposal — see [`settle_a_proposal`] —
/// and from the chooser press, which are the same move made a moment apart.
///
/// Choosing on a Conversation that is not in Direction is refused by the store,
/// which is what makes *implement inline* impossible from anywhere else: the
/// choice and the start are one answer, so neither half of it happens without
/// the other.
pub(crate) async fn choose_direction(
    state: &AppState,
    id: i64,
    direction: Direction,
) -> Result<DirectionChosen> {
    match store::choose_direction(&state.pool, id, direction).await? {
        store::Directed::NoSuchConversation => return Ok(DirectionChosen::NoSuchConversation),
        store::Directed::NotChoosing => return Ok(DirectionChosen::NotChoosing),
        store::Directed::Chosen => {}
    }

    build(state, id, direction).await?;

    Ok(DirectionChosen::Chosen)
}

/// Set the work being built: the Conversation moves to Implementing, and a fresh
/// session under its implementation Profile starts in its worktree, inside
/// whichever skill the chosen `direction` primes it for.
///
/// A fresh session rather than the grilling one carrying on, because the two run
/// under Profiles the Conversation fixed separately and a session cannot change
/// the account it is running as. What the grilling knew reaches it as the
/// handoff — see [`crate::skills::implementing`].
///
/// Implementing either way. Writing the backlog is the work starting rather than
/// a step before it: an agent is loose in the worktree and about to commit to the
/// branch, which is the thing the state is there to say.
///
/// The move is recorded before the session is started, exactly as starting a
/// grilling records the worktree before launching one, and it is read the same
/// way: a Conversation that is implementing with nothing implementing it is a
/// thing to look at and start again, where one that had launched an agent nothing
/// recorded would be an agent nobody could see or stop. So a session that will
/// not start is logged rather than raised — the choice stands, and the chooser
/// says where it was chosen that nothing started off it. Not an Interruption:
/// those are raised about a session that ran and went wrong, and this is one
/// that never ran, with nothing to gather as evidence and nothing the human has
/// to decide between.
///
/// A task list is where this stops being one session's business. The session
/// just started writes the backlog and then idles, as an interactive session
/// does, and everything after it — a fresh session per task, ended as each one
/// lands — is [`crate::runner`]'s. It is handed the session rather than told to
/// go and find one: seeing that session out is the first step of the run.
async fn build(state: &AppState, id: i64, direction: Direction) -> Result<()> {
    let pool = &state.pool;

    match store::start_implementing(pool, id).await? {
        store::Implementing::Started => {}
        // Something moved the Conversation between the choice and this — a
        // second press from another device, or an abort mid-decision.
        store::Implementing::NotChoosing => {
            tracing::info!(
                conversation_id = id,
                "the Conversation left Direction before the implementation could start"
            );
            return Ok(());
        }
        store::Implementing::NoSuchConversation => {
            tracing::error!(
                conversation_id = id,
                "there is no Conversation to implement"
            );
            return Ok(());
        }
    }

    // Read back rather than assembled from what was just recorded, for the reason
    // starting a grilling reads it back: where an agent is about to be let loose
    // is the one thing that must not be guessed at.
    let Some(conversation) = store::load_conversation(pool, id).await? else {
        tracing::error!(
            conversation_id = id,
            "there is no Conversation to implement"
        );
        return Ok(());
    };

    // Both Profiles are settled before grilling starts, so a missing one here is
    // one deleted since: the Conversation has moved and there is no account to
    // run the work as.
    let Some(profile) = conversation.implementation_profile.clone() else {
        tracing::error!(
            conversation_id = id,
            "the implementation Profile is gone, so no session was started"
        );
        return Ok(());
    };

    // The grilling session ended when its proposal was accepted. Ended again
    // here because this is where it matters rather than where it happened: one
    // worktree holds one agent, and two would be two agents editing each other's
    // files.
    state.sessions.end(id).await;

    let (brief, handoff) = documents(pool, id).await?;

    // Which skill the session runs under is the whole of the difference between
    // the two: same Profile, same worktree, same two documents.
    let prompt = match direction {
        Direction::Inline => skills::implementing(&brief, handoff.as_deref()),
        Direction::TaskList => skills::breaking_down(&brief, handoff.as_deref()),
        Direction::Roadmap => skills::staging(&brief, handoff.as_deref()),
    };

    let started = match state
        .sessions
        .start(pool, &state.nudges, &conversation, &profile, &prompt)
        .await
    {
        Ok(started) => started,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "an implementation session could not be started");
            None
        }
    };

    // All three are followed, and the drivers differ in what they are watching
    // for rather than in whether anybody is watching. A backlog's run is a
    // session per step, each one seen out and the next launched; an inline run is
    // one session, and what says it did anything is what it committed; a roadmap
    // is one session too, watched for the roadmap it writes and then carried on
    // to the pull request it opened. Every way round, a session that ends without
    // landing its work stops the run at an Interruption rather than leaving a
    // Conversation that says it is implementing with nothing implementing it.
    match (direction, started) {
        (Direction::TaskList, Some(session)) => {
            tokio::spawn(crate::runner::follow(state.clone(), id, session));
        }
        (Direction::Inline, Some(session)) => {
            tokio::spawn(crate::runner::follow_inline(state.clone(), id, session));
        }
        (Direction::Roadmap, Some(session)) => {
            // The commit the branch came off, which is what says a roadmap in the
            // Worktree is one this branch wrote rather than one the repository
            // already had. Recorded at grill start, so a Conversation with a
            // Worktree has one; without it there is nothing to watch for, and the
            // session is left running rather than followed.
            match conversation.base_commit.clone() {
                Some(base) => {
                    tokio::spawn(crate::runner::follow_roadmap(
                        state.clone(),
                        id,
                        base,
                        session,
                    ));
                }
                None => tracing::error!(
                    conversation_id = id,
                    "the Conversation has no base commit, so the roadmap session is not followed"
                ),
            }
        }
        // A session that would not start is logged above. There is nothing to
        // follow, and the chooser says so where it was chosen — see the viewer's
        // `chosen-note`.
        (_, None) => {}
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

/// Record the commit the work branches from, or put the Conversation back on the
/// default-branch rule.
///
/// What is stored is the commit the repository resolved, not what was typed: a
/// tag or a branch name is a moving target, and the point of overriding the rule
/// is to pin the work to one commit. Blank counts as clearing it — a field
/// emptied is the human taking the override away, not naming a commit called
/// nothing.
pub(crate) async fn set_base_commit(
    pool: &SqlitePool,
    id: i64,
    asked: Option<&str>,
) -> Result<BaseRecorded> {
    let asked = asked.map(str::trim).filter(|asked| !asked.is_empty());

    let commit = match asked {
        None => None,
        Some(asked) => {
            // The repository to ask is the Conversation's own, so the
            // Conversation has to be there before there is anywhere to ask.
            let Some(conversation) = store::load_conversation(pool, id).await? else {
                return Ok(BaseRecorded::NoSuchConversation);
            };

            let asked = asked.to_owned();
            let resolved =
                tokio::task::spawn_blocking(move || resolve(&conversation.repo.path, &asked))
                    .await?;

            match resolved {
                Some(commit) => Some(commit),
                None => return Ok(BaseRecorded::NoSuchCommit),
            }
        }
    };

    Ok(
        match store::set_base_commit(pool, id, commit.as_deref()).await? {
            store::Edited::Saved => BaseRecorded::Recorded,
            store::Edited::NoSuchConversation => BaseRecorded::NoSuchConversation,
            store::Edited::NotDrafting => BaseRecorded::NotDrafting,
        },
    )
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
/// grilling with a Timeline that says so and no session on it. Not an
/// Interruption either: a grilling is attended, and those are for the unattended
/// runs a human is not watching.
///
/// The whole state rather than the four pieces of it this needs: what starting a
/// grilling reaches is most of what the server holds — the store, the boundary,
/// the state directory, the sessions and whoever is watching them — and a
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
    let grilling = crate::profiles::entry(watched, conversation.grilling_profile.clone()).await?;
    let implementation =
        crate::profiles::entry(watched, conversation.implementation_profile.clone()).await?;

    if let Some(refusal) = unready(grilling.as_ref(), implementation.as_ref()) {
        return Ok(refusal.grilling());
    }

    // Kept rather than only judged: it is what the session about to start is
    // primed with, and it is frozen from the moment the Conversation moves.
    let brief = brief(pool, id).await?;

    if brief.trim().is_empty() {
        return Ok(GrillingStarted::EmptyBrief);
    }

    // What the work branches from. An override is re-resolved rather than
    // trusted: it resolved when the human typed it, and a commit can be gone by
    // now. Without one it is the default branch's tip, which is a rule that has
    // never resolved to anything until this moment.
    let named = conversation
        .base_commit
        .clone()
        .unwrap_or_else(|| conversation.repo.default_branch.clone());

    let repo = conversation.repo.path.clone();
    let branch = conversation.branch.clone();
    let path = worktrees::worktree_path(&state.state_dir, id, &conversation.repo.name, &branch);

    // The filesystem and git halves together, off the runtime: a worktree of a
    // large repository is not a quick call, and every part of this blocks.
    let made = tokio::task::spawn_blocking({
        let path = path.clone();
        move || {
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
    if let Some(profile) = conversation.grilling_profile.clone()
        && let Err(error) = state
            .sessions
            .start(
                pool,
                &state.nudges,
                &conversation,
                &profile,
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
    let grilling = crate::profiles::entry(watched, conversation.grilling_profile.clone()).await?;
    let implementation =
        crate::profiles::entry(watched, conversation.implementation_profile.clone()).await?;

    if let Some(refusal) = unready(grilling.as_ref(), implementation.as_ref()) {
        return Ok(refusal.adopting());
    }

    // Where the stage branches from. The override where the human fixed one —
    // which is how an unmerged predecessor is stacked on, that being their move
    // rather than Verkstead's — and the default branch's tip where they did not.
    let named = conversation
        .base_commit
        .clone()
        .unwrap_or_else(|| conversation.repo.default_branch.clone());

    let repo = conversation.repo.path.clone();

    // The reading, off the runtime's threads: resolving a commit and reading a
    // roadmap out of a git directory are both blocking calls.
    let read = tokio::task::spawn_blocking({
        let repo = repo.clone();
        let named = named.clone();

        move || {
            let Some(commit) = worktrees::resolve(&repo, &named) else {
                return Err(Adopted::NoBaseCommit);
            };

            // The same rule the notice was drawn by and the page was drawn by,
            // asked here at the base commit — and asked again, rather than
            // taken from either, because a roadmap is a document anybody may
            // have moved since. Which clause refused it is the answer to the
            // button: each of them is a different thing to go and do about it.
            match crate::stages::startable(&repo, &commit, &roadmap) {
                Startable::Stage(abandoned) => Ok((commit, abandoned.stage)),
                Startable::NoRoadmap => Err(Adopted::NoRoadmap),
                Startable::Complete => Err(Adopted::RoadmapComplete),
                Startable::InFlight => Err(Adopted::StageInFlight),
                Startable::NoBrief => Err(Adopted::NoBrief),
                Startable::BranchTaken => Err(Adopted::BranchExists),
            }
        }
    })
    .await?;

    let (commit, stage) = match read {
        Ok(read) => read,
        Err(refusal) => return Ok(refusal),
    };

    // The stage's own slug, as the unattended start names one — the brief's
    // filename without its number. The name the Conversation was started under
    // was the server's invention for a row in the sidebar, and it is discarded
    // here: a stage is worked on the branch its roadmap will annotate it with.
    let branch = stage.branch();
    let path = worktrees::worktree_path(&state.state_dir, id, &conversation.repo.name, &branch);

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
    // and an open page should say so without being reloaded.
    state.nudges.announce();

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
pub(crate) async fn abort(state: &AppState, id: i64) -> Result<ConversationAborted> {
    let pool = &state.pool;

    let Some(conversation) = store::load_conversation(pool, id).await? else {
        return Ok(ConversationAborted::NoSuchConversation);
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
            return Ok(ConversationAborted::WorktreeStuck);
        }
    }

    // And the directory beside it, with whatever the sessions put there. It is
    // given back for the reason the worktree is: it is somewhere a Conversation
    // was given to work, and the Conversation has stopped. Whatever it held that
    // was worth keeping is on the Timeline already.
    let handoffs = Handoffs::under(&state.state_dir);
    tokio::task::spawn_blocking(move || handoffs.remove(id)).await?;

    Ok(match store::abort_conversation(pool, id).await? {
        store::Aborting::Aborted => ConversationAborted::Aborted,
        store::Aborting::AlreadyAborted => ConversationAborted::AlreadyAborted,
        store::Aborting::NoSuchConversation => ConversationAborted::NoSuchConversation,
    })
}

/// Why these two Profiles are not a pair of accounts to run under, or `None`
/// where they are.
///
/// The same rule [`crate::profiles::ready_to_grill`] answers for the pane, said
/// the other way round: that one says whether to offer the button, this one says
/// what was wrong when it was pressed. Each is named separately, because
/// choosing a Profile and mending a broken one are different jobs.
///
/// The rule rather than either button's answer, because both buttons ask it:
/// starting a grilling and adopting a stage each want a pair of Profiles fixed
/// before they will do anything, and each says so in its own words — see
/// [`Unready::grilling`] and [`Unready::adopting`].
fn unready(
    grilling: Option<&ProfileEntry>,
    implementation: Option<&ProfileEntry>,
) -> Option<Unready> {
    let Some(grilling) = grilling else {
        return Some(Unready::NoGrillingProfile);
    };

    let Some(implementation) = implementation else {
        return Some(Unready::NoImplementationProfile);
    };

    [grilling, implementation]
        .into_iter()
        .any(|profile| profile.broken.is_some())
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

/// The Brief off a Conversation's Timeline.
///
/// Found rather than taken from the front: the Brief is the first Event, but by
/// the time anything asks this there are moves on the Timeline after it.
async fn brief(pool: &SqlitePool, id: i64) -> Result<String> {
    Ok(store::timeline(pool, id)
        .await?
        .into_iter()
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
/// The Brief is found rather than taken from the front: it is the first Event,
/// but by the time anything asks this there are moves on the Timeline after it.
/// The handoff is the *last* of its kind rather than the first, which is the
/// other way round for a reason: a Conversation gets one handoff per grilling
/// round, and the one that hands over is the one the grilling that just ended
/// wrote.
pub(crate) async fn documents(pool: &SqlitePool, id: i64) -> Result<(String, Option<String>)> {
    let timeline = store::timeline(pool, id).await?;

    let brief = timeline
        .iter()
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
/// reads it: the two Profiles, and a Brief with something in it.
///
/// Answered against what the endpoint has already read rather than by loading
/// the Conversation again — and it deliberately says nothing about the branch or
/// the base commit, which are decided against git when the button is pressed.
pub(crate) fn ready_to_grill(
    state: store::Lifecycle,
    grilling: Option<&ProfileEntry>,
    implementation: Option<&ProfileEntry>,
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

/// The commit `asked` names in the repository at `path`, in full, or `None` if
/// nothing there answers to it.
///
/// `^{commit}` is what makes a tag or a branch resolve to the commit it points
/// at rather than to itself, and what refuses a tree or a blob that happens to
/// share a prefix.
fn resolve(path: &Path, asked: &str) -> Option<String> {
    let commit = git(
        path,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            // Whatever was typed is the human's, so it must not be able to
            // arrive as an option.
            "--end-of-options",
            &format!("{asked}^{{commit}}"),
        ],
    )?;

    let commit = commit.trim();

    (!commit.is_empty()).then(|| commit.to_owned())
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
