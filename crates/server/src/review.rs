//! The wrap-up self-review: the one session that reads the branch whole, and
//! what becomes of what it finds.
//!
//! There are no per-commit review states anywhere in Verkstead. Commits are
//! events to read, and this is where problems get raised instead — once, about a
//! branch, by a session with fresh context. The sessions that wrote the work each
//! saw one task and none of them saw the pull request; this one sees nothing else.
//!
//! **It proposes, and then it fixes what was agreed to.** What it produces first
//! is one Question Set on the Timeline, a Question per finding, with Options that
//! amount to *fix it* or *leave it* — which is what puts the human in the loop
//! without putting them at a terminal. Then it stays where it is: the ask blocks
//! until they answer, and when the answers come back the same session fixes each
//! finding they accepted, commits, pushes and ends. A finding they declined is
//! never raised again.
//!
//! One session for the lot of it, because a handful of fixes is not a handful of
//! pieces of work. The session that raised them is the one that read the branch,
//! and the one whose findings the human answered with words beside them; a fresh
//! session per finding would throw all of that away and re-read the diff to get
//! half of it back.
//!
//! A review that finds nothing asks nothing. It says so as the last thing it
//! prints — which is what the Timeline shows of a session — and ends. A Set with
//! no findings in it would be a row for the human to dismiss, and the point of
//! the phase is to spend their attention only where there is a decision.
//!
//! **The review settles when its session ends cleanly**, which is the one moment
//! everything it was sent to do is certainly over: the branch read, the findings
//! put, the accepted ones landed. Answering the Set settles nothing — the
//! Response is what the session acts on, and it is still acting when it arrives.
//!
//! A review session that ends badly is not a review that had nothing to do: it
//! is a review that did not finish. That stops the run at an Interruption like
//! every other, and retrying it is the review over again in a session as fresh as
//! the first.
//!
//! **One agent in one Worktree**, which is what the turns are for. The checks are
//! being watched at the same time as this runs, and a fix session dispatched
//! mid-review would end the review where it stood — starting a session for a
//! Conversation ends the one it already has. So the review waits for the
//! Worktree and holds it until its session is done, the wait on the human
//! included, and the checks watcher tries for it and comes back later.

use crate::AppState;
use crate::runner::Reviewed;
use crate::store;

/// Review `conversation_id`'s branch, where it has not been reviewed already.
///
/// Returns as soon as there is nothing to do — a review that has already asked or
/// already settled, a Conversation that has stopped wrapping up, or a run that is
/// blocked on the human. None of those is a failure: this is spawned by
/// everything that might have left a wrap-up without a review, and most of the
/// time one of them has already seen to it.
///
/// Nothing is refused for. This runs unattended with nobody watching, and what it
/// has to say it says on the Timeline or in the log.
pub(crate) async fn run(state: AppState, conversation_id: i64) {
    if !wanted(&state, conversation_id).await {
        return;
    }

    // Waited for rather than tried for: nothing else will start this review on
    // its behalf, so a Worktree busy with a fix session is a queue to join rather
    // than a reason to give up. It may be a long wait — and once taken, it is
    // held for as long as the review session lives, which is across the human's
    // answering too. That is the shape of one agent in one Worktree.
    let _turn = state.sessions.turn(conversation_id).await;

    // Asked again on the other side of the wait, because everything it asked
    // about moves while it waits: the fix session that held the Worktree may have
    // been the last of its attempts, and the Conversation may have been aborted
    // out from under this altogether.
    if !wanted(&state, conversation_id).await {
        return;
    }

    tracing::info!(
        conversation_id,
        "the work is on a pull request nobody has read, so a review session is starting"
    );

    match crate::runner::review(&state, conversation_id).await {
        // Everything it was sent to do is done: the branch read, whatever it
        // found put to the human, and whatever they accepted fixed and pushed.
        Reviewed::Done => {
            settle(&state, conversation_id).await;

            tracing::info!(
                conversation_id,
                "the review is over, so the wrap-up carries on"
            );
        }
        Reviewed::Stopped { how, writing } => stopped(&state, conversation_id, &how, writing).await,
        Reviewed::Nothing => {}
    }
}

/// Review it again because the human asked for it, and put the wrap-up's other
/// half back under watch while we are here.
///
/// The checks stopped being watched when this Interruption was raised — nothing
/// advances past an open one — so a retry that started only the review would
/// leave the pull request's checks unwatched for the rest of the wrap-up.
pub(crate) async fn retried(state: AppState, conversation_id: i64) {
    tracing::info!(conversation_id, "the review is being run again");

    crate::wrapping::watching(&state, conversation_id);
}

/// Whether there is a review to run at all.
///
/// Four ways there is not, and none of them is a failure: the Conversation has
/// stopped wrapping up, the review has already asked, the review has already
/// settled, or the run is blocked on the human — the same rule the runner and the
/// checks watcher keep, that nothing is launched while an Interruption is open.
///
/// A store that will not answer reads as *no*, which is the right way round for
/// the one thing this decides: on the other side of it is an agent being let
/// loose in a Worktree.
async fn wanted(state: &AppState, conversation_id: i64) -> bool {
    if !wrapping(state, conversation_id).await {
        return false;
    }

    match store::review_asked(&state.pool, conversation_id).await {
        Ok(None) => {}
        Ok(Some(set_id)) => {
            tracing::debug!(
                conversation_id,
                set_id,
                "the review has already put its findings to the human"
            );
            return false;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether the review had asked failed");
            return false;
        }
    }

    match store::wrap_up_settled(&state.pool, conversation_id).await {
        Ok(settled) if settled.contains(&store::WaitingOn::Review) => {
            tracing::debug!(
                conversation_id,
                "this Conversation has been reviewed already"
            );
            return false;
        }
        Ok(_) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a wrap-up had settled failed");
            return false;
        }
    }

    match store::open_interruption(&state.pool, conversation_id).await {
        Ok(None) => true,
        Ok(Some(event_id)) => {
            tracing::info!(
                conversation_id,
                event_id,
                "the run is blocked on the human, so no review was started"
            );
            false
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a wrap-up was blocked failed");
            false
        }
    }
}

/// Whether the Conversation is still wrapping up, which is the only state a
/// review belongs to.
async fn wrapping(state: &AppState, conversation_id: i64) -> bool {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state == store::Lifecycle::Wrapping,
        Ok(None) => {
            tracing::error!(conversation_id, "there is no Conversation left to review");
            false
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to review failed");
            false
        }
    }
}

/// Record that the review is over, so wrap-up has one less thing to wait on.
///
/// Once its session has ended cleanly and never before: what wrap-up is waiting
/// on is the whole of the review — the branch read, the findings put, the ones
/// the human accepted landed — and the session ending well is the one thing that
/// says all of it happened. Answering the Set says only that the decisions are
/// made.
async fn settle(state: &AppState, conversation_id: i64) {
    if let Err(error) =
        store::settle_wrap_up(&state.pool, conversation_id, store::WaitingOn::Review).await
    {
        tracing::error!(error = ?error, conversation_id, "recording that the review was over failed");
    }
}

/// Stop the run: the review did not finish, and what to do about it is the
/// human's.
///
/// The evidence is the tail of what the session said, which is where a review
/// that fell over says why — and the three remedies all mean something: run the
/// review again, read the branch yourself, or end the run.
async fn stopped(state: &AppState, conversation_id: i64, how: &str, writing: i64) {
    if let Err(error) = crate::interruptions::raise(
        state,
        conversation_id,
        store::Step::Review,
        "reviewing the branch the pull request is on",
        how,
        Some(writing),
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a review did not finish and the Interruption saying so could not be raised"
        );
    }
}
