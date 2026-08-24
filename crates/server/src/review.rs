//! The wrap-up self-review: the one session that reads the branch whole, and
//! what becomes of what it finds.
//!
//! There are no per-commit review states anywhere in Verkstead. Commits are
//! events to read, and this is where problems get raised instead — once, about a
//! branch, by a session with fresh context. The sessions that wrote the work each
//! saw one task and none of them saw the pull request; this one sees nothing else.
//!
//! **It reviews and it does not fix.** What it produces is one Question Set on
//! the Timeline, a Question per finding, with Options that amount to *fix this*
//! or *leave it* — which is what puts the human in the loop without putting them
//! at a terminal. Each finding they accept dispatches a session inside the
//! bundled addressing skill, given that finding as its feedback; a finding they
//! decline dispatches nothing and is never raised again. The review session is
//! not brought back to hear any of it: it had its say when it asked.
//!
//! A review that finds nothing asks nothing. It says so as the last thing it
//! prints — which is what the Timeline shows of a session — and the review
//! settles as one of the things wrap-up was waiting on. A Set with no findings in
//! it would be a row for the human to dismiss, and the point of the phase is to
//! spend their attention only where there is a decision.
//!
//! A review session that ends without asking and without ending well is not a
//! review that found nothing: it is a review that did not happen. That halts the
//! run like every other stop, and going again is the review over from the start,
//! in a session as fresh as the first.
//!
//! **One agent in one Worktree**, which is what the turns are for. The checks are
//! being watched at the same time as this runs, and a fix session dispatched
//! mid-review would end the review where it stood — starting a session for a
//! Conversation ends the one it already has. So the review waits for the
//! Worktree, the checks watcher tries for it and comes back later, and the
//! findings the human accepted are dispatched one at a time.

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
    // than a reason to give up. It may be a long wait, and that is the shape of
    // one agent in one Worktree.
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
        // The Set is on the Timeline and the human has it. What settles the
        // review is their Response — see [`answered`] — because a review nobody
        // has answered is exactly what wrap-up is waiting on.
        Reviewed::Asked => tracing::info!(
            conversation_id,
            "the review has put its findings to the human"
        ),
        Reviewed::FoundNothing => {
            settle(&state, conversation_id).await;

            tracing::info!(
                conversation_id,
                "the review found nothing worth raising, so the wrap-up carries on"
            );
        }
        Reviewed::Stopped { how, writing } => stopped(&state, conversation_id, &how, writing).await,
        Reviewed::Nothing => {}
    }
}

/// Review it again because the human asked for it, and put the wrap-up's other
/// half back under watch while we are here.
///
/// The checks stopped being watched when the run stopped — nothing advances past
/// that — so a retry that started only the review would leave the pull request's
/// checks unwatched for the rest of the wrap-up.
pub(crate) async fn retried(state: AppState, conversation_id: i64) {
    tracing::info!(conversation_id, "the review is being run again");

    crate::wrapping::watching(&state, conversation_id);
}

/// Dispatch a session for each finding the human accepted.
///
/// Spawned rather than awaited: this is called from the endpoint that takes their
/// Response, and what it starts is minutes of agent work — the human's browser
/// hears that their answer was taken, and the work of it happens behind that.
///
/// Nothing is dispatched for a review nobody accepted anything from, and nothing
/// is dispatched twice: a Set is answered once, so this runs once.
pub(crate) fn answered(state: &AppState, reviewed: store::Reviewed) {
    let store::Reviewed::Answered {
        conversation_id,
        fixing,
    } = reviewed
    else {
        tracing::error!("a review was answered on no Timeline, so nothing was dispatched");
        return;
    };

    if fixing.is_empty() {
        tracing::info!(
            conversation_id,
            "the review was answered with nothing to fix, so the wrap-up carries on"
        );
        return;
    }

    tokio::spawn(fix(state.clone(), conversation_id, fixing));
}

/// Work through them, one session at a time.
///
/// One at a time because they are one Worktree, and in the order the review
/// raised them because that is the order it thought about them in.
///
/// Each takes the Worktree for as long as its session runs, so a check going red
/// in the middle of this queues its fix session behind the rest rather than
/// ending one of them.
async fn fix(state: AppState, conversation_id: i64, fixing: Vec<store::Fixing>) {
    let all = fixing.len();

    for (n, finding) in fixing.into_iter().enumerate() {
        let _turn = state.sessions.turn(conversation_id).await;

        // Asked before each one, because they are dispatched over the course of an
        // hour: a Conversation aborted halfway through has nowhere left to work.
        if !wrapping(&state, conversation_id).await {
            tracing::info!(
                conversation_id,
                "the Conversation stopped wrapping up, so the rest of the review is not dispatched"
            );
            return;
        }

        tracing::info!(
            conversation_id,
            finding = n + 1,
            of = all,
            "a finding the human accepted is starting a session"
        );

        crate::runner::address(&state, conversation_id, &feedback(&finding)).await;
    }
}

/// What the fix session is told: the finding as the review wrote it for whoever
/// would fix it, and whatever the human said when they agreed.
///
/// Their words last, under the finding rather than over it, for the reason a
/// retry note goes under the documents: the finding says what is wrong, and this
/// says what they thought about it. "Yes, but leave the public signature alone"
/// is only worth writing if it reaches the session that can act on it.
fn feedback(finding: &store::Fixing) -> String {
    let mut told = format!(
        "The review of this branch raised this, and the human has said to fix it.\n\n{}\n",
        finding.what.trim(),
    );

    if !finding.said.trim().is_empty() {
        told.push_str(&format!(
            "\nWhat they said when they agreed:\n\n{}\n",
            finding.said.trim(),
        ));
    }

    told
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

    !crate::halts::stopped(state, conversation_id).await
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
/// Only ever for a review that found nothing: every other review is settled by
/// the human answering it, which the store does as it takes the Response.
async fn settle(state: &AppState, conversation_id: i64) {
    if let Err(error) =
        store::settle_wrap_up(&state.pool, conversation_id, store::WaitingOn::Review).await
    {
        tracing::error!(error = ?error, conversation_id, "recording that the review found nothing failed");
    }
}

/// Stop the run: the review did not happen, and what to do about it is the
/// human's.
///
/// The evidence is the tail of what the session said, which is where a review
/// that fell over says why — and what to do about it is Resume, read the branch
/// themselves, or abort.
///
/// [`store::Halt::Deliberate`], because a wrap-up that goes on without its
/// review is a branch nobody read: Verkstead stops rather than pass a session
/// that crashed off as a clean bill of health, and going again is the human's
/// press.
async fn stopped(state: &AppState, conversation_id: i64, how: &str, writing: i64) {
    if let Err(error) = crate::halts::halt(
        state,
        conversation_id,
        store::Halt::Deliberate,
        "reviewing the branch the pull request is on",
        how,
        Some(writing),
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a review did not happen and the halt saying so could not be recorded"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(what: &str, said: &str) -> store::Fixing {
        store::Fixing {
            what: what.to_owned(),
            said: said.to_owned(),
        }
    }

    /// What a fix session is told about a finding: the review's own words for
    /// whoever would fix it, and that the human agreed it is worth fixing.
    #[test]
    fn a_fix_session_is_told_the_finding_and_that_it_was_accepted() {
        let told = feedback(&finding(
            "`window.rs` never resets the counter between windows.",
            "",
        ));

        assert!(
            told.contains("`window.rs` never resets the counter"),
            "the finding as the review wrote it: {told}",
        );
        assert!(
            told.contains("said to fix it"),
            "and that this is not a suggestion: {told}",
        );
        assert!(
            !told.contains("What they said"),
            "with nothing said about words nobody wrote: {told}",
        );
    }

    /// And their qualification, where they wrote one — which is the whole reason
    /// the Answer's free text travels with the finding.
    #[test]
    fn what_the_human_wrote_alongside_reaches_the_session_that_can_act_on_it() {
        let told = feedback(&finding(
            "`window.rs` never resets the counter between windows.",
            "Yes, but leave the public signature alone.",
        ));

        assert!(
            told.contains("leave the public signature alone"),
            "their words reach the session: {told}",
        );
        assert!(
            told.find("never resets the counter") < told.find("leave the public signature"),
            "under the finding rather than over it: {told}",
        );
    }
}
