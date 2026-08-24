//! The other proposal a wrap-up makes: what a batch of comments left on the
//! pull request comes to.
//!
//! Everything standing on the pull request when the review starts belongs to
//! the review — see [`crate::comments::for_the_review`]. This is about what is
//! said *after* it: a batch of comments the human wrote while the branch sat
//! there, which nothing has been asked about yet.
//!
//! **It proposes, and then it fixes what was agreed to**, exactly as the review
//! does and for the same reason. A comment is the human talking, but the words
//! on a pull request are not an instruction to a session: "this is the wrong way
//! round" says what is wrong and not what to do about it, and a session sent to
//! do it would be deciding in their place. So the batch session reads what was
//! said and the code it is about, puts one small Set saying what it would do,
//! waits, and lands what they accepted. One session for the batch and one Set
//! for it, because three replies in a minute are one point being made.
//!
//! A batch asking for nothing asks nothing. A question the commits since have
//! answered, a note saying this reads well — the session says so as the last
//! thing it prints, which is what the Timeline shows of one, and the batch
//! stays settled as addressed. Spending the human's attention only where there
//! is a decision, which is the review's rule one turn later.
//!
//! **Nothing the human accepted is allowed to go quietly**, which is the review's
//! net too and the reason this module exists rather than the dispatch simply
//! being a session launch. A batch session that asked, was answered and then went
//! — cleanly or otherwise — with nothing committed since is a wrap-up owing work
//! nobody is left to do. So the record is asked afterwards rather than the
//! session trusted, and what is owed stops the run at an Interruption. Retrying
//! it is the doing over again: one fix session handed every accepted proposal at
//! once, because the decisions were made and only the carrying out failed.
//! Nothing is asked again.
//!
//! A batch session that ended badly having been owed nothing never got as far as
//! asking. That stops the run at an Interruption too — and because the batch was
//! written down as addressed the moment it was dispatched, the comments are
//! forgotten again as it is raised, so a retry is the batch over again in a
//! session as fresh as the first.
//!
//! **One agent in one Worktree.** The caller holds the Conversation's Turn
//! across the whole of a batch session, the wait on the human included, exactly
//! as the review's caller does — see [`crate::comments::dispatch`].

use crate::AppState;
use crate::runner::Reviewed;
use crate::store;

/// Run one batch session about `said`, and see out whatever it leaves.
///
/// The caller is holding the Conversation's Turn and has recorded `which` — the
/// comments this batch is made of — as addressed. Both are what make this the
/// one session that will act on them.
///
/// Nothing is refused for. This runs unattended with nobody watching, and what
/// it has to say it says on the Timeline or in the log.
pub(crate) async fn run(state: &AppState, conversation_id: i64, said: &str, which: &[String]) {
    match crate::runner::respond(state, conversation_id, said).await {
        Reviewed::Done => over(state, conversation_id, which, None).await,
        Reviewed::Stopped { how, writing } => {
            over(state, conversation_id, which, Some((how, writing))).await
        }
        Reviewed::Nothing => {}
    }
}

/// The batch session is over: leave the wrap-up to carry on, or stop the run.
///
/// One question first, and it is asked of the record rather than of the session:
/// is there anything the human accepted that never landed? That is the failure
/// this half exists for, and it reads the same whether the session saw itself out
/// or fell over — the decisions are made either way, and what is owed is owed.
///
/// `ended_badly` is how it ended where it did not end well, and the Timeline
/// Event it was printing into. A session owed nothing that ended badly is one
/// that never got as far as asking, which is the other Interruption here.
async fn over(
    state: &AppState,
    conversation_id: i64,
    which: &[String],
    ended_badly: Option<(String, i64)>,
) {
    let owed = unlanded(state, conversation_id).await;

    if !owed.is_empty() {
        let (how, writing) = match &ended_badly {
            Some((how, writing)) => (Some(how.as_str()), Some(*writing)),
            None => (None, None),
        };

        return dropped(state, conversation_id, &owed, how, writing).await;
    }

    if let Some((how, writing)) = ended_badly {
        return stopped(state, conversation_id, which, &how, writing).await;
    }

    // Everything it was sent to do is done: what was said read, whatever it would
    // do put to the human, and whatever they accepted fixed and pushed — or the
    // batch asked for nothing and it said so. The comments are addressed either
    // way, and the watcher settles them on its next poll.
    tracing::info!(
        conversation_id,
        comments = which.len(),
        "what was said on the pull request has been answered, so the wrap-up carries on"
    );
}

/// Answer the batch again because the human asked for it — or land what it was
/// answered and never landed, which is the other thing a retry here can mean.
///
/// Which of the two is a fact about the record rather than something to choose:
/// proposals the human accepted with nothing committed since are a run that
/// stopped between the deciding and the doing, and what a retry owes there is the
/// doing alone. Everything else is the batch over again — the comments were
/// forgotten as the Interruption was raised, so the watcher this puts back on
/// dispatches a fresh session about the same words.
///
/// The wrap-up's other halves go back under watch either way. Nothing advances
/// past an open Interruption, so the checks stopped being watched when this one
/// was raised, and a retry that started only the fixes would leave the pull
/// request's checks unwatched for the rest of the wrap-up.
pub(crate) async fn retried(state: AppState, conversation_id: i64) {
    crate::wrapping::watching(&state, conversation_id);

    let owed = unlanded(&state, conversation_id).await;

    if owed.is_empty() {
        tracing::info!(
            conversation_id,
            "what was said on the pull request is being answered again"
        );
        return;
    }

    tracing::info!(
        conversation_id,
        fixes = owed.len(),
        "what was said was answered and never acted on, so a session is starting on the \
         fixes alone"
    );

    land(state, conversation_id, owed).await
}

/// Land the fixes the human accepted, in one session that does nothing else.
///
/// One session handed all of them together, which is what the batch's own would
/// have done: the decisions were made, so there is nothing to propose and nothing
/// to read the comments for a second time.
///
/// The Worktree is taken for it like any other session's, so a red check going
/// red mid-fix queues behind this rather than ending it.
///
/// Asked of the record again afterwards, exactly as it was the first time: a fix
/// session that landed nothing has left the same work owed, and letting that one
/// through would be the failure this whole path exists to close.
async fn land(state: AppState, conversation_id: i64, owed: Vec<store::Fixing>) {
    let _turn = state.sessions.turn(conversation_id).await;

    if !crate::wrapping::still_going(&state, conversation_id).await {
        tracing::info!(
            conversation_id,
            "the Conversation stopped wrapping up, so the fixes it owed were not dispatched"
        );
        return;
    }

    let writing = crate::runner::address(&state, conversation_id, &feedback(&owed)).await;

    let owed = unlanded(&state, conversation_id).await;

    if !owed.is_empty() {
        return dropped(&state, conversation_id, &owed, None, writing).await;
    }

    tracing::info!(
        conversation_id,
        "the fixes the batch was owed have landed, so the wrap-up carries on"
    );
}

/// What the fix session is told: every proposal the human accepted, in the words
/// the batch session wrote for whoever would fix them, and whatever they said
/// beside each answer.
///
/// The comments themselves are not here, and that is the point of the whole
/// path: what the human agreed to is this session's reading of them, written for
/// an agent to act on. Their own words already had their say when the Set was
/// answered.
///
/// Nothing here is put as a question. The Set was answered and the answers are
/// what this is made of, so a session that came back with a proposal would be
/// asking the human to decide something they already have.
fn feedback(owed: &[store::Fixing]) -> String {
    let findings = owed
        .iter()
        .map(|finding| match finding.said.trim().is_empty() {
            true => finding.what.trim().to_owned(),
            false => format!(
                "{}\n\nWhat they said when they agreed:\n\n{}",
                finding.what.trim(),
                finding.said.trim(),
            ),
        })
        .collect::<Vec<String>>()
        .join("\n\n---\n\n");

    format!(
        "What was said on this branch's pull request was read, and the human has said to do \
         {this}. The session that was to do it ended without landing anything, so what is \
         left is the doing rather than the deciding: none of this is still a question. Fix \
         {each}, commit, and push so the pull request has {it}.\n\n{findings}\n",
        this = match owed.len() {
            1 => "this about it",
            _ => "these about it",
        },
        each = match owed.len() {
            1 => "it",
            _ => "each of them",
        },
        it = match owed.len() {
            1 => "it",
            _ => "them",
        },
    )
}

/// What a batch session was told to fix and nothing has landed.
///
/// Empty is the ordinary answer and covers every way there is nothing owed: the
/// batch asked nothing, the Set is still waiting on the human, they declined
/// every proposal, or the session that was going to fix them did so.
///
/// A store that will not answer reads as *nothing owed*, which is the right way
/// round for what is on the other side of this: stopping the run and letting an
/// agent loose in a Worktree. The error is in the log, where a broken database
/// says everything else it has to say.
async fn unlanded(state: &AppState, conversation_id: i64) -> Vec<store::Fixing> {
    match store::unlanded_batch_fixes(&state.pool, conversation_id).await {
        Ok(owed) => owed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a batch session was owed failed");
            Vec::new()
        }
    }
}

/// Stop the run: the batch session did not finish, and what to do about it is
/// the human's.
///
/// The comments are forgotten first, because a retry is the batch over again and
/// they were written down as addressed the moment it was dispatched. Forgotten
/// rather than left, so that the session the retry's watcher starts is one about
/// the same words rather than one about nothing — and forgotten before the
/// Interruption is raised, so that a forgetting that fails leaves the run stopped
/// rather than quietly going round again.
///
/// The evidence is the tail of what the session said, which is where one that
/// fell over says why — and the three remedies all mean something: answer what
/// was said again, read the comments yourself, or end the run.
async fn stopped(
    state: &AppState,
    conversation_id: i64,
    which: &[String],
    how: &str,
    writing: i64,
) {
    if let Err(error) = store::forget_addressed_comments(&state.pool, conversation_id, which).await
    {
        tracing::error!(error = ?error, conversation_id, "forgetting a batch nobody answered failed");
    }

    if let Err(error) = crate::interruptions::raise(
        state,
        conversation_id,
        store::Step::Comments,
        "answering what was said on the pull request",
        how,
        Some(writing),
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a batch session did not finish and the Interruption saying so could not be raised"
        );
    }
}

/// Stop the run: the human accepted fixes that nothing landed, and only they can
/// say what happens now.
///
/// The Interruption names the doing rather than the reading, because that is the
/// half that failed — and it says what is owed in the session's own words, so the
/// choice is answerable without opening the Set again. Retrying it is the fixes
/// in one session; taking over is the human making them; aborting ends the run
/// with the branch exactly as the session left it.
///
/// The comments are not forgotten here, unlike the other Interruption's: they
/// were answered, and what is owed is the answer rather than the reading.
///
/// `how` is how the session ended where it ended badly, and `writing` the Event
/// it was printing into — both absent for a session that saw itself out and
/// simply never pushed.
async fn dropped(
    state: &AppState,
    conversation_id: i64,
    owed: &[store::Fixing],
    how: Option<&str>,
    writing: Option<i64>,
) {
    if let Err(error) = crate::interruptions::raise(
        state,
        conversation_id,
        store::Step::Comments,
        "landing the fixes what was said on the pull request was answered with",
        &owing(owed, how),
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a batch session's accepted fixes were never landed and the Interruption saying \
             so could not be raised"
        );
    }
}

/// What is owed, as the Interruption says it: how many fixes, and the session's
/// own words for each.
///
/// Its words rather than Verkstead's, because the human decided against those
/// words an hour ago and these are the ones they will recognise. Clamped to a
/// line the way the review's are, and for the same reason — this is one line on
/// a card read on a phone. See [`crate::review::in_a_line`].
fn owing(owed: &[store::Fixing], how: Option<&str>) -> String {
    let fixes = owed
        .iter()
        .map(|finding| format!("“{}”", crate::review::in_a_line(&finding.what)))
        .collect::<Vec<String>>()
        .join("; ");

    let what = format!(
        "{} the human accepted about what was said {} landed: {fixes}",
        match owed.len() {
            1 => "one fix".to_owned(),
            n => format!("{n} fixes"),
        },
        match owed.len() {
            1 => "was never",
            _ => "were never",
        },
    );

    match how {
        Some(how) => format!("{how}, and {what}"),
        None => format!("it ended without pushing, and {what}"),
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

    /// What a fix session is told about the batch's accepted proposals: the
    /// session's own words for whoever would fix them, and that these are
    /// decisions rather than proposals.
    #[test]
    fn a_fix_session_is_told_every_accepted_proposal_at_once() {
        let told = feedback(&[
            finding("Move the reset above the comparison in `window.rs`.", ""),
            finding("Collapse the two clocks onto one.", ""),
        ]);

        assert!(
            told.contains("Move the reset above the comparison")
                && told.contains("Collapse the two clocks onto one"),
            "both of them, in the words the batch session wrote: {told}",
        );
        assert!(
            told.contains("said to do these about it")
                && told.contains("none of this is still a question"),
            "and that the deciding is over: {told}",
        );
        assert!(
            told.contains("push"),
            "with the push that puts them on the pull request: {told}",
        );
        assert!(
            !told.contains("What they said"),
            "and nothing said about words nobody wrote: {told}",
        );
    }

    /// And their qualification, where they wrote one — which is the whole reason
    /// the Answer's free text is kept on the Set at all.
    #[test]
    fn what_the_human_wrote_alongside_reaches_the_session_that_can_act_on_it() {
        let told = feedback(&[finding(
            "Move the reset above the comparison in `window.rs`.",
            "Yes, but leave the public signature alone.",
        )]);

        assert!(
            told.contains("leave the public signature alone"),
            "their words reach the session: {told}",
        );
        assert!(
            told.find("Move the reset") < told.find("leave the public signature"),
            "under the proposal rather than over it: {told}",
        );
    }

    /// What the Interruption says: that the fixes never landed, and which ones.
    #[test]
    fn the_interruption_says_what_is_unlanded_in_the_sessions_own_words() {
        let says = owing(
            &[
                finding("Move the reset above the comparison.", ""),
                finding("Collapse the two clocks onto one.", ""),
            ],
            None,
        );

        assert!(
            says.contains("2 fixes") && says.contains("never landed"),
            "how much is owed: {says}",
        );
        assert!(
            says.contains("Move the reset") && says.contains("Collapse the two clocks"),
            "and what, as the session wrote it: {says}",
        );
        assert!(
            says.contains("what was said"),
            "and that it was about the pull request's comments: {says}",
        );
    }

    /// A session that fell over says both: how it ended, and what it left owed.
    #[test]
    fn a_session_that_ended_badly_says_so_beside_what_it_left() {
        let says = owing(
            &[finding("Move the reset above the comparison.", "")],
            Some("exited with status 1"),
        );

        assert!(
            says.contains("exited with status 1") && says.contains("one fix"),
            "how it ended and what is owed: {says}",
        );
    }
}
