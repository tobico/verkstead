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
//! **What was already said belongs to the review.** A wrap-up starts with one
//! session that reads the branch and proposes what should change, and the
//! comments standing on the pull request when it starts are part of what it
//! reads — see [`for_the_review`]. They are recorded as addressed the moment that
//! session is dispatched, so nothing here dispatches about them afterwards, and
//! what they ask for reaches the human as a Question like every other finding
//! rather than as work somebody was quietly sent to do.
//!
//! Which is why nothing is dispatched from here until the review is over. A
//! comment sitting on the pull request while it runs is one the review has folded
//! in or is about to, and a batch session started over the top of that would be
//! the ungated half of this all over again. What lands afterwards is the batch's
//! again, once the review has settled and the Worktree is free.
//!
//! Comments said after that dispatch a **batch session**: a fresh session under
//! the Conversation's implementation Profile, inside the bundled responding
//! skill, given what was said and nothing about what to do with it. It proposes
//! what it would do as one small Set, waits, and lands what the human accepted —
//! the review's shape one turn later, and for the review's reason. What becomes
//! of it is [`crate::responding`]'s.
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
//! Written down as a session is dispatched rather than as it finishes, which
//! buys that at a price: a batch whose session is gone leaves a record saying
//! somebody dealt with what was said and nobody who did. So nothing is settled
//! here until that has been asked about — see
//! [`crate::responding::unattended`], which is where a proposal nobody is left
//! behind is picked up.
//!
//! Nor while the Worktree is busy. The same trade means a batch that has only
//! just started reads here exactly as one that is over — its comments are
//! addressed and it has not asked anything yet — and a wrap-up has more than one
//! of these watchers whenever a press starts its four over the top of the four
//! already running. So the Turn is asked for before anything is settled, and a
//! Worktree with a session in it settles nothing: without that, one watcher
//! could settle over the batch the other had just dispatched, and the wrap-up
//! reach Done with the session that was going to put the question still working.
//!
//! A `gh` that cannot answer changes nothing at all — it does not settle, it does
//! not unsettle, and it dispatches nothing. That is the only honest reading of
//! it: Verkstead does not know what has been said, and *nobody said anything* is
//! not a thing to conclude from not knowing.
//!
//! Nothing here ever asks the human itself. What asks is the session dispatched
//! about a batch, which puts what it would do to them rather than what they
//! said: their own words back at them would be the one question with nothing
//! behind it.

use std::path::Path;

use crate::AppState;
use crate::github::Comment;
use crate::store;

/// Watch what is said on `conversation_id`'s pull request until it stops
/// wrapping up.
///
/// Returns when there is nothing left to watch: the Conversation has moved on or
/// gone, or driving that stopped. Idle rather than looping, for the checks
/// watcher's reason — nothing advances past a stop, and a
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
    // leaves Wrapping for — Done, or closed from the menu — arrives here as the
    // same fact: this is not a wrap-up any more.
    if conversation.state != store::Lifecycle::Wrapping {
        return Watching::Done("the Conversation is not wrapping up any more");
    }

    // Asked before anything is dispatched, for the runner's reason: *the run does
    // not advance past a stop* means no session is launched while the human is
    // the only thing that can start one — an account out of window included, see
    // [`crate::stopping::stopped`].
    if crate::stopping::stopped(state, conversation_id).await {
        return Watching::Done("driving has stopped");
    }

    // Before anything is read of GitHub or settled here: a batch session that
    // asked and is no longer running has left the human a question with nobody
    // behind it, and the comments it was dispatched about written down as dealt
    // with. Settling over that would take the Conversation to Done with the Set
    // still open — see [`crate::responding::unattended`], which sees to whatever
    // is outstanding and says whether it found anything.
    //
    // Asked only once the review is over, because a batch is only dispatched
    // once it is: until then there is nothing here for anybody to have left
    // behind, and the review's own Set is [`crate::review`]'s to see to whatever
    // becomes of it.
    if reviewed(state, conversation_id).await
        && crate::responding::unattended(state, conversation_id).await
    {
        return Watching::Again;
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

    // Nothing is touched where GitHub could not be asked, and the next poll asks
    // again — of a `gh` that may by then have been logged in.
    let Some(fresh) = unaddressed(
        state,
        conversation_id,
        &conversation.repo.path,
        opened.number,
    )
    .await
    else {
        return Watching::Again;
    };

    if fresh.is_empty() {
        // Not while something is working in the Worktree. A batch is written down
        // as addressed the moment it is dispatched and before its session has said
        // a word, so *nothing is unaddressed* is also what a batch that has only
        // just started looks like from a second watcher — and a wrap-up has more
        // than one of these the moment a press starts its watchers over the top of
        // the ones already running. Settling over a batch that has not asked yet
        // is the same failure [`crate::responding::unattended`] refuses to settle
        // over once it has: a Conversation carried to Done with the session that
        // was going to put the question still working.
        //
        // The Turn is what tells the two apart, tried rather than waited for
        // because this is a poll. A Worktree that is busy settles nothing and the
        // next poll asks again, of a Worktree that by then is free — and the Turn
        // is held across the settling itself, so that a batch dispatched between
        // the asking and the writing cannot be settled over either.
        if let Some(_turn) = state.sessions.try_turn(conversation_id) {
            settle(state, conversation_id).await;
        } else {
            tracing::debug!(
                conversation_id,
                "something is working in the Worktree, so what was said is settled later",
            );
        }

        return Watching::Again;
    }

    // Said before anything is dispatched, because what wrap-up waits on is
    // nothing being left unaddressed, and these are.
    unsettle(state, conversation_id).await;

    // And nothing is dispatched at all until the review is over: everything
    // standing on the pull request while it runs is the review's to propose about,
    // and a batch session started over the top of that would be acting on a
    // comment nobody had agreed to act on.
    if !reviewed(state, conversation_id).await {
        tracing::debug!(
            conversation_id,
            comments = fresh.len(),
            "the review has not finished, so what has been said is left for it",
        );
        return Watching::Again;
    }

    dispatch(state, conversation_id, &fresh).await
}

/// What is on pull request `number` that nobody has been sent to deal with yet.
///
/// The two readers of the comments share this, because *what is new* is one
/// question however differently the two answer it: the watcher looks again after
/// the interval and the review goes on without them.
///
/// `None` is GitHub not having been asked, which is neither *nothing was said*
/// nor *something was*. An empty list is the answer that there is nothing new,
/// which is every pull request the moment it opens.
async fn unaddressed(
    state: &AppState,
    conversation_id: i64,
    repo: &Path,
    number: i64,
) -> Option<Vec<Comment>> {
    let asked = {
        let gh = state.github.clone();
        let repo = repo.to_path_buf();

        // Off the runtime's threads: this is a process, and one that goes to the
        // network.
        tokio::task::spawn_blocking(move || crate::github::comments(&gh, &repo, number)).await
    };

    let said = match asked {
        Ok(Ok(said)) => said,
        Ok(Err(trouble)) => {
            tracing::warn!(
                conversation_id,
                number,
                why = trouble.why(),
                "the comments could not be read, so nothing is decided about them",
            );
            return None;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking gh about the comments failed");
            return None;
        }
    };

    let already = match store::addressed_comments(&state.pool, conversation_id).await {
        Ok(already) => already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which comments had been dispatched for failed");
            return None;
        }
    };

    Some(
        said.into_iter()
            .filter(|comment| !already.contains(&comment.which))
            .collect(),
    )
}

/// Whether the wrap-up's review is over, which is what says a comment is a batch
/// session's rather than the review's.
///
/// A store that will not answer reads as *not yet*, which is the right way round
/// for the one thing this decides: on the other side of it is an agent being let
/// loose on feedback nobody has been asked about.
async fn reviewed(state: &AppState, conversation_id: i64) -> bool {
    match store::wrap_up_settled(&state.pool, conversation_id).await {
        Ok(settled) => settled.contains(&store::WaitingOn::Review),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether the review was over failed");
            false
        }
    }
}

/// Everything said on `conversation_id`'s pull request that nobody has been sent
/// to deal with, written out for the review session that is about to start — and
/// recorded as addressed, because that session is who deals with it.
///
/// `None` where there is nothing to fold in, which covers a pull request nobody
/// has written on and a `gh` that could not be asked. The second of those is the
/// module's rule again: *nobody said anything* is not a thing to conclude from
/// not knowing, so the review runs on the branch alone and the batch that comes
/// after it picks up what was there.
///
/// Written down as the session is dispatched rather than as it ends, for the
/// reason a batch's are — see [`store::record_addressed_comments`]. Recording
/// them and then failing to launch would lose them, which is the same trade every
/// dispatch here makes and the same one that keeps a restarted server from
/// dispatching twice.
///
/// Nothing races this for a comment. [`once`] dispatches nothing until the review
/// has settled, so every comment standing here is one the watcher has left alone
/// on purpose — and the caller is holding the Worktree's Turn besides, which is
/// what makes *present at review start* a moment rather than an approximation.
/// What lands after this reads is the batch's, once the review is over.
pub(crate) async fn for_the_review(state: &AppState, conversation_id: i64) -> Option<String> {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to read comments for failed");
            return None;
        }
    };

    let opened = match store::pull_request(&state.pool, conversation_id).await {
        Ok(Some(opened)) => opened,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the pull request to read comments on failed");
            return None;
        }
    };

    // A `gh` that could not be asked reads the same as a pull request nobody has
    // written on: the review goes ahead on the branch alone, and the batch after
    // it picks up whatever was there.
    let fresh = unaddressed(
        state,
        conversation_id,
        &conversation.repo.path,
        opened.number,
    )
    .await?;

    if fresh.is_empty() {
        return None;
    }

    let which: Vec<String> = fresh.iter().map(|comment| comment.which.clone()).collect();

    if let Err(error) = store::record_addressed_comments(&state.pool, conversation_id, &which).await
    {
        tracing::error!(error = ?error, conversation_id, "recording which comments the review was given failed");
        return None;
    }

    tracing::info!(
        conversation_id,
        comments = fresh.len(),
        "the pull request has been commented on, so the review is given what was said",
    );

    Some(said_by(&fresh))
}

/// Start one session about the whole batch, if the Worktree is free.
///
/// One agent in one Worktree. Tried rather than waited for, exactly as the checks
/// watcher tries: what else is in there is the review session or a fix the human
/// accepted, both of which take as long as they take. Nothing is lost by coming
/// back — the comments are still there, and a batch that grew while this waited
/// is one session about more of what was said, which is what a batch is for.
///
/// Once taken, the Turn is held for as long as the session lives, the wait on
/// the human included — the review's shape again, and the reason the watcher
/// takes no further look while one runs. A comment said in the meantime is the
/// next batch's, once this one is over.
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

    crate::responding::run(state, conversation_id, &said_by(fresh), &which).await;

    Watching::Again
}

/// What was said, in the order it was said in, and where each of it was said.
///
/// The comments whole rather than summarised, and in the markdown they were
/// written in. This is a human talking to whoever wrote the branch, and the
/// session that reads it is the nearest thing to that: a summary would be
/// Verkstead deciding which half of the feedback mattered.
///
/// The file and line travel with a comment left on the diff, because that is
/// half of what it means. "This is the wrong way round" is an instruction with
/// them and a riddle without.
///
/// Two sessions read this and both read it the same way: the review is given
/// what was standing when it started and a batch session is given what was said
/// after, and each of them proposes about it rather than acting on it. So this
/// is the whole of what either is told about the words — what to do with them is
/// the skill each is running inside.
fn said_by(fresh: &[Comment]) -> String {
    fresh
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
        .join("\n\n---\n\n")
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

    /// What either session is told about one comment: who said it and what they
    /// wrote, whole.
    #[test]
    fn a_session_is_told_who_said_it_and_what_they_wrote() {
        let said = said_by(&[comment("tobico", "Rename the `window` field.")]);

        assert!(
            said.contains("tobico") && said.contains("Rename the `window` field."),
            "who said it and what they wrote: {said}",
        );
    }

    /// A batch is one session's worth of feedback, so all of it reaches that one
    /// session — three replies in a minute are one point being made.
    #[test]
    fn every_comment_in_the_batch_reaches_the_one_session() {
        let said = said_by(&[
            comment("tobico", "Rename the `window` field."),
            comment("tobico", "And the test that pins it."),
        ]);

        assert!(
            said.contains("Rename the `window` field.") && said.contains("And the test that pins"),
            "both of them: {said}",
        );
        assert!(
            said.find("Rename the `window`") < said.find("And the test that pins"),
            "in the order they were said in: {said}",
        );
    }

    /// Where a comment on the diff was left travels with it, because that is
    /// half of what it means: *this is the wrong way round* is an instruction
    /// with the file and the line and a riddle without them.
    #[test]
    fn a_comment_left_on_the_diff_carries_where_it_was_left() {
        let said = said_by(&[on_a_line(
            "`src/window.rs` line 12",
            "This is the wrong way round.",
        )]);

        assert!(
            said.contains("on `src/window.rs` line 12"),
            "the file and the line: {said}",
        );
        assert!(
            said.contains("This is the wrong way round."),
            "and what they said about it: {said}",
        );
    }

    /// And one said about the pull request as a whole says nothing about where,
    /// rather than trailing an empty *on*.
    #[test]
    fn a_comment_about_the_whole_pull_request_names_no_place() {
        let said = said_by(&[comment("tobico", "Rename the `window` field.")]);

        assert!(said.contains("**tobico** said:"), "{said}");
    }

    /// A comment left by an account that has since gone is still a comment to
    /// answer, and it reads as somebody rather than as nobody.
    #[test]
    fn a_comment_with_no_author_left_is_still_something_to_answer() {
        let said = said_by(&[comment("", "Rename the `window` field.")]);

        assert!(
            said.contains("Somebody said") && said.contains("Rename the `window` field."),
            "{said}",
        );
    }

    /// Neither session is given any instruction along with the words: both of
    /// them propose about what was said rather than doing it, and a prompt
    /// telling either to push would be the ungated half arriving by the other
    /// door.
    #[test]
    fn what_a_session_is_given_is_what_was_said_and_not_what_to_do_about_it() {
        let said = said_by(&[
            comment("tobico", "Rename the `window` field."),
            on_a_line("`src/window.rs` line 12", "This is the wrong way round."),
        ]);

        assert!(
            said.contains("**tobico** said:") && said.contains("Rename the `window` field."),
            "who said it and what they wrote: {said}",
        );
        assert!(
            said.contains("on `src/window.rs` line 12"),
            "and where, for the one left on the diff: {said}",
        );
        assert!(
            !said.contains("push") && !said.contains("do it"),
            "with nothing telling it to act on them: {said}",
        );
    }
}
