//! What is said on a wrapping Conversation's pull request, and the sessions it
//! dispatches.
//!
//! The other half of what a human can do to an open pull request. They read the
//! branch and they write on it, and Verkstead reads what they wrote through the
//! host's `gh` — for as long as the Conversation is Wrapping and no longer, on
//! the same interval the checks are asked about.
//!
//! All three places they write count: the pull request's conversation, the words
//! at the top of a review, and the comments left on the lines of the diff. The
//! last is where a review of code mostly happens, so a watcher that read only
//! the conversation would miss the feedback it most needs to act on — see
//! [`crate::github::comments`], which is where the three are read as one. Two
//! `gh` calls a poll, so four a minute per wrapping Conversation beside the
//! checks watcher's two, and there are rarely more than a handful of those.
//!
//! Comments it has not seen before dispatch an **addressing session**: a fresh
//! session under the Conversation's implementation Profile, inside the bundled
//! addressing skill, given what was said as its feedback. It commits and pushes
//! as that skill says, with no gate in front of either, and the branch watcher
//! puts what it committed on the Timeline.
//!
//! **One session per batch rather than one per comment.** A human writing three
//! replies in a minute is making one point, and three sessions racing each other
//! in one Worktree is the thing a batch prevents.
//!
//! Which comments have been dispatched for is written down rather than held in
//! memory — see [`store::record_addressed_comments`]. A server that came back up
//! and read every comment as new would dispatch a session about feedback that
//! was addressed yesterday.
//!
//! A `gh` that cannot answer changes nothing at all — it does not settle, it does
//! not unsettle, and it dispatches nothing. That is the only honest reading of
//! it: Verkstead does not know what has been said, and *nobody said anything* is
//! not a thing to conclude from not knowing.
//!
//! Nothing here ever stops the run. A comment is the human already talking, and
//! halting over their own comment would be the one stop with nothing behind it —
//! where a check that will not go green is the machine running out of things to
//! try, this is not.

use crate::AppState;
use crate::github::Comment;
use crate::store;

/// Watch what is said on `conversation_id`'s pull request until it stops
/// wrapping up.
///
/// Returns when there is nothing left to watch: the Conversation has moved on or
/// gone, or a run stopped at an Interruption. Idle rather than looping, for the
/// checks watcher's reason — nothing advances past an open Interruption, and a
/// watcher that dispatched sessions behind one would be working on a run the
/// human has stopped.
///
/// Nothing here is refused for. This runs unattended with nobody watching, and
/// what it has to say it says on the Timeline or in the log.
pub(crate) async fn watch(state: AppState, conversation_id: i64) {
    loop {
        if let Watching::Done(why) = once(&state, conversation_id).await {
            tracing::info!(
                conversation_id,
                why,
                "the pull request's comments are no longer being read"
            );
            return;
        }

        tokio::time::sleep(state.sessions.pace().checks).await;
    }
}

/// What one look at the comments decided.
enum Watching {
    /// Look again after the interval.
    Again,

    /// Stop watching, for this reason.
    Done(&'static str),
}

/// Take one look: ask GitHub what has been said, and dispatch a session for
/// whatever is new.
async fn once(state: &AppState, conversation_id: i64) -> Watching {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return Watching::Done("there is no Conversation left to read comments for"),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to read comments for failed");
            return Watching::Again;
        }
    };

    // The one thing that ends the watching by itself. Everything a Conversation
    // leaves Wrapping for — Done, or aborted from the menu — arrives here as the
    // same fact: this is not a wrap-up any more.
    if conversation.state != store::Lifecycle::Wrapping {
        return Watching::Done("the Conversation is not wrapping up any more");
    }

    // Asked before anything is dispatched, for the runner's reason: *the run does
    // not advance past a halt* means no session is launched while the human is
    // the only thing that can start one.
    if crate::halts::stopped(state, conversation_id).await {
        return Watching::Done("driving has stopped");
    }

    let opened = match store::pull_request(&state.pool, conversation_id).await {
        Ok(Some(opened)) => opened,
        // A Conversation wrapping up has a pull request — recording one *is* the
        // move — so this is a record that has been got at rather than a wrap-up
        // to carry on with.
        Ok(None) => return Watching::Done("the Conversation has no pull request to read"),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the pull request to read comments on failed");
            return Watching::Again;
        }
    };

    let asked = {
        let gh = state.github.clone();
        let repo = conversation.repo.path.clone();
        let number = opened.number;

        // Off the runtime's threads: this is a process, and one that goes to the
        // network.
        tokio::task::spawn_blocking(move || crate::github::comments(&gh, &repo, number)).await
    };

    let said = match asked {
        Ok(Ok(said)) => said,
        // GitHub could not be asked, which is neither *nothing was said* nor
        // *something was*. Nothing is touched and the next poll asks again, of a
        // `gh` that may by then have been logged in.
        Ok(Err(trouble)) => {
            tracing::warn!(
                conversation_id,
                number = opened.number,
                why = trouble.why(),
                "the comments could not be read, so the wrap-up goes on waiting",
            );

            return Watching::Again;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking gh about the comments failed");
            return Watching::Again;
        }
    };

    let already = match store::addressed_comments(&state.pool, conversation_id).await {
        Ok(already) => already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which comments had been dispatched for failed");
            return Watching::Again;
        }
    };

    let fresh: Vec<Comment> = said
        .into_iter()
        .filter(|comment| !already.contains(&comment.which))
        .collect();

    if fresh.is_empty() {
        settle(state, conversation_id).await;
        return Watching::Again;
    }

    // Said before anything is dispatched, because what wrap-up waits on is
    // nothing being left unaddressed, and these are.
    unsettle(state, conversation_id).await;

    dispatch(state, conversation_id, &fresh).await
}

/// Start one session about the whole batch, if the Worktree is free.
///
/// One agent in one Worktree. Tried rather than waited for, exactly as the checks
/// watcher tries: what else is in there is the review session or a fix the human
/// accepted, both of which take as long as they take. Nothing is lost by coming
/// back — the comments are still there, and a batch that grew while this waited
/// is one session about more of what was said, which is what a batch is for.
async fn dispatch(state: &AppState, conversation_id: i64, fresh: &[Comment]) -> Watching {
    let Some(_turn) = state.sessions.try_turn(conversation_id) else {
        tracing::debug!(
            conversation_id,
            "something else is working in the Worktree, so the comments are read again later",
        );
        return Watching::Again;
    };

    let which: Vec<String> = fresh.iter().map(|comment| comment.which.clone()).collect();

    // Written down as the session is dispatched rather than as it ends, so that a
    // batch a server dispatched for and then restarted over is not dispatched for
    // twice.
    if let Err(error) = store::record_addressed_comments(&state.pool, conversation_id, &which).await
    {
        tracing::error!(error = ?error, conversation_id, "recording which comments were being dispatched for failed");
        return Watching::Again;
    }

    tracing::info!(
        conversation_id,
        comments = fresh.len(),
        "the pull request has been commented on, so a session is starting on it",
    );

    crate::runner::address(state, conversation_id, &feedback(fresh)).await;

    Watching::Again
}

/// What the session is told: everything new that has been said, in the order it
/// was said in, and where each of it was said.
///
/// The comments whole rather than summarised, and in the markdown they were
/// written in. This is a human talking to whoever wrote the branch, and the
/// session that reads it is the nearest thing to that: a summary would be
/// Verkstead deciding which half of the feedback mattered.
///
/// The file and line travel with a comment left on the diff, because that is
/// half of what it means. "This is the wrong way round" is an instruction with
/// them and a riddle without.
fn feedback(fresh: &[Comment]) -> String {
    let said = fresh
        .iter()
        .map(|comment| {
            let who = match comment.author.is_empty() {
                true => "Somebody".to_owned(),
                false => format!("**{}**", comment.author),
            };

            let where_said = match comment.about.is_empty() {
                true => String::new(),
                false => format!(" on {}", comment.about),
            };

            format!("{who} said{where_said}:\n\n{}", comment.markdown.trim())
        })
        .collect::<Vec<String>>()
        .join("\n\n---\n\n");

    format!(
        "{} on the pull request this branch is on. Work out what {} asking for, do it, and \
         push it so the pull request has it.\n\n{said}\n",
        match fresh.len() {
            1 => "This has been said",
            _ => "These have been said",
        },
        match fresh.len() {
            1 => "it is",
            _ => "they are",
        },
    )
}

/// Record that nothing is left unaddressed, so wrap-up has one less thing to
/// wait on.
async fn settle(state: &AppState, conversation_id: i64) {
    if let Err(error) =
        store::settle_wrap_up(&state.pool, conversation_id, store::WaitingOn::Comments).await
    {
        tracing::error!(error = ?error, conversation_id, "recording that the comments are all addressed failed");
    }
}

/// And that something is, which is a comment nobody has been sent to deal with
/// yet.
async fn unsettle(state: &AppState, conversation_id: i64) {
    if let Err(error) =
        store::unsettle_wrap_up(&state.pool, conversation_id, store::WaitingOn::Comments).await
    {
        tracing::error!(error = ?error, conversation_id, "putting the comments back to waiting failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(author: &str, markdown: &str) -> Comment {
        Comment {
            which: format!("IC_{author}_{}", markdown.len()),
            author: author.to_owned(),
            at: "2026-08-21T09:00:00Z".to_owned(),
            about: String::new(),
            markdown: markdown.to_owned(),
        }
    }

    /// The same, left on a line of the diff rather than in the conversation.
    fn on_a_line(about: &str, markdown: &str) -> Comment {
        Comment {
            about: about.to_owned(),
            ..comment("tobico", markdown)
        }
    }

    /// What the session is told about one comment: who said it and what they
    /// wrote, whole, plus that the fix has to reach the pull request.
    #[test]
    fn a_session_is_told_what_was_said_and_that_it_has_to_reach_the_pull_request() {
        let told = feedback(&[comment("tobico", "Rename the `window` field.")]);

        assert!(
            told.contains("tobico") && told.contains("Rename the `window` field."),
            "who said it and what they wrote: {told}",
        );
        assert!(
            told.contains("push"),
            "and that it has to reach the pull request: {told}",
        );
    }

    /// A batch is one session's worth of feedback, so all of it reaches that one
    /// session — three replies in a minute are one point being made.
    #[test]
    fn every_comment_in_the_batch_reaches_the_one_session() {
        let told = feedback(&[
            comment("tobico", "Rename the `window` field."),
            comment("tobico", "And the test that pins it."),
        ]);

        assert!(
            told.contains("Rename the `window` field.") && told.contains("And the test that pins"),
            "both of them: {told}",
        );
        assert!(
            told.find("Rename the `window`") < told.find("And the test that pins"),
            "in the order they were said in: {told}",
        );
    }

    /// Where a comment on the diff was left travels with it, because that is
    /// half of what it means: *this is the wrong way round* is an instruction
    /// with the file and the line and a riddle without them.
    #[test]
    fn a_comment_left_on_the_diff_carries_where_it_was_left() {
        let told = feedback(&[on_a_line(
            "`src/window.rs` line 12",
            "This is the wrong way round.",
        )]);

        assert!(
            told.contains("on `src/window.rs` line 12"),
            "the file and the line: {told}",
        );
        assert!(
            told.contains("This is the wrong way round."),
            "and what they said about it: {told}",
        );
    }

    /// And one said about the pull request as a whole says nothing about where,
    /// rather than trailing an empty *on*.
    #[test]
    fn a_comment_about_the_whole_pull_request_names_no_place() {
        let told = feedback(&[comment("tobico", "Rename the `window` field.")]);

        assert!(told.contains("**tobico** said:"), "{told}");
    }

    /// A comment left by an account that has since gone is still a comment to
    /// act on, and it reads as somebody rather than as nobody.
    #[test]
    fn a_comment_with_no_author_left_is_still_something_to_do() {
        let told = feedback(&[comment("", "Rename the `window` field.")]);

        assert!(
            told.contains("Somebody said") && told.contains("Rename the `window` field."),
            "{told}",
        );
    }
}
