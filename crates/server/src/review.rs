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
//! **What is already on the pull request is part of what it reads.** A human who
//! commented before the review started has said something about this branch, and
//! this is the session that reads the branch — so those comments go into its
//! prompt whole and what they ask for is proposed in the same Set, beside the
//! findings it made itself. Which is what stops them being acted on ungated: they
//! are recorded as addressed as this session is dispatched, so no batch session is
//! later sent to do what nobody agreed to. See [`crate::comments::for_the_review`].
//!
//! A review that finds nothing and was given nothing asks nothing. It says so as
//! the last thing it prints — which is what the Timeline shows of a session — and
//! ends. A Set with no findings in it would be a row for the human to dismiss,
//! and the point of the phase is to spend their attention only where there is a
//! decision. Comments asking for work are that decision's other source: a review
//! with nothing of its own to raise still proposes about them.
//!
//! **The review settles when its session ends cleanly and its fixes have
//! landed**, which is the one moment everything it was sent to do is certainly
//! over: the branch read, the findings put, the accepted ones committed.
//! Answering the Set settles nothing — the Response is what the session acts on,
//! and it is still acting when it arrives.
//!
//! **Nothing the human accepted is allowed to go quietly.** A session that asked,
//! was answered and then went — cleanly or otherwise — with nothing committed
//! since is a wrap-up owing work nobody is left to do, and a review that settled
//! there would reach Done with approved fixes lost. So the record is asked
//! afterwards rather than the session trusted: the findings they accepted are on
//! the Set, their words are on the Response, and a branch with no commit since
//! the answers is the doing never having happened. That stops the run at an
//! Interruption saying what is owed, and a retry is the doing over again — one
//! fix session handed every accepted finding at once, because the decisions were
//! made and only the carrying out failed. Nothing is asked again.
//!
//! A review session that ends badly having been owed nothing is not a review that
//! had nothing to do: it is a review that did not finish. That stops the run at
//! an Interruption like every other, and retrying it is the review over again in
//! a session as fresh as the first.
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

    // Read inside the Turn, which is what makes *what was said before the review
    // started* a fact rather than a race: nothing can dispatch about a comment
    // while this holds the Worktree, and one that lands from here on is the next
    // batch session's. Recorded as addressed as this session is dispatched, so
    // nothing is later sent to do ungated what the Set is about to propose.
    let said = crate::comments::for_the_review(&state, conversation_id).await;

    match crate::runner::review(&state, conversation_id, said).await {
        Reviewed::Done => over(&state, conversation_id, None).await,
        Reviewed::Stopped { how, writing } => {
            over(&state, conversation_id, Some((how, writing))).await
        }
        Reviewed::Nothing => {}
    }
}

/// The review session is over: settle the review, or stop the run.
///
/// One question first, and it is asked of the record rather than of the session:
/// is there anything the human accepted that never landed? That is the failure
/// this half exists for, and it reads the same whether the session saw itself out
/// or fell over — the decisions are made either way, and what is owed is owed.
///
/// `ended_badly` is how it ended where it did not end well, and the Timeline
/// Event it was printing into. A session owed nothing that ended badly is a
/// review that did not finish, which is the other Interruption here.
async fn over(state: &AppState, conversation_id: i64, ended_badly: Option<(String, i64)>) {
    let owed = unlanded(state, conversation_id).await;

    if !owed.is_empty() {
        let (how, writing) = match &ended_badly {
            Some((how, writing)) => (Some(how.as_str()), Some(*writing)),
            None => (None, None),
        };

        return dropped(state, conversation_id, &owed, how, writing).await;
    }

    if let Some((how, writing)) = ended_badly {
        return stopped(state, conversation_id, &how, writing).await;
    }

    // Everything it was sent to do is done: the branch read, whatever it found
    // put to the human, and whatever they accepted fixed and pushed.
    settle(state, conversation_id).await;

    tracing::info!(
        conversation_id,
        "the review is over, so the wrap-up carries on"
    );
}

/// Review it again because the human asked for it — or land what it was answered
/// and never landed, which is the other thing a retry here can mean.
///
/// Which of the two is a fact about the record rather than something to choose:
/// findings the human accepted with nothing committed since are a run that
/// stopped between the deciding and the doing, and what a retry owes there is the
/// doing alone. Everything else is the review over again, in a session as fresh
/// as the first.
///
/// The wrap-up's other half goes back under watch either way. The checks stopped
/// being watched when this Interruption was raised — nothing advances past an
/// open one — so a retry that started only the review would leave the pull
/// request's checks unwatched for the rest of the wrap-up.
pub(crate) async fn retried(state: AppState, conversation_id: i64) {
    crate::wrapping::watching(&state, conversation_id);

    let owed = unlanded(&state, conversation_id).await;

    if owed.is_empty() {
        tracing::info!(conversation_id, "the review is being run again");
        return;
    }

    tracing::info!(
        conversation_id,
        fixes = owed.len(),
        "the review was answered and never acted on, so a session is starting on the \
         fixes alone"
    );

    land(state, conversation_id, owed).await
}

/// Land the fixes the human accepted, in one session that does nothing else.
///
/// One session handed all of them together, which is what the review's own would
/// have done: the decisions were made, so there is nothing to propose and nothing
/// to read the branch for a second time. A session per finding would be a fresh
/// context per fix, each re-reading the diff to work out what the review already
/// wrote down.
///
/// The Worktree is taken for it like any other session's, so a red check going
/// red mid-fix queues behind this rather than ending it.
///
/// Asked of the record again afterwards, exactly as it was the first time: a fix
/// session that landed nothing has left the same work owed, and letting that one
/// through would be the failure this whole path exists to close.
async fn land(state: AppState, conversation_id: i64, owed: Vec<store::Fixing>) {
    let _turn = state.sessions.turn(conversation_id).await;

    // Asked on the other side of the wait, for the reason the review asks twice:
    // a Conversation aborted while this queued has nowhere left to work.
    if !wrapping(&state, conversation_id).await {
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

    settle(&state, conversation_id).await;

    tracing::info!(
        conversation_id,
        "the fixes the review was owed have landed, so the wrap-up carries on"
    );
}

/// What the fix session is told: every finding the human accepted, in the words
/// the review wrote for whoever would fix them, and whatever they said beside
/// each answer.
///
/// Their words go under each finding rather than over it, for the reason a retry
/// note goes under the documents: the finding says what is wrong, and this says
/// what they thought about it. "Yes, but leave the public signature alone" is
/// only worth writing if it reaches the session that can act on it.
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
        "The review of this branch raised {this}, and the human has said to fix {it}. The \
         session that was to do it ended without landing anything, so what is left is the \
         doing rather than the deciding: none of this is still a question. Fix {each}, commit, \
         and push so the pull request has {it}.\n\n{findings}\n",
        this = match owed.len() {
            1 => "this",
            _ => "these",
        },
        it = match owed.len() {
            1 => "it",
            _ => "them",
        },
        each = match owed.len() {
            1 => "it",
            _ => "each of them",
        },
    )
}

/// The findings this Conversation's review was told to fix and nothing has
/// landed.
///
/// Empty is the ordinary answer and covers every way there is nothing owed: no
/// review has asked, the Set is still waiting on the human, they declined every
/// finding, or the session that was going to fix them did so.
///
/// A store that will not answer reads as *nothing owed*, which is the right way
/// round for what is on the other side of this: stopping the run and letting an
/// agent loose in a Worktree. The error is in the log, where a broken database
/// says everything else it has to say.
async fn unlanded(state: &AppState, conversation_id: i64) -> Vec<store::Fixing> {
    match store::unlanded_fixes(&state.pool, conversation_id).await {
        Ok(owed) => owed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a review was owed failed");
            Vec::new()
        }
    }
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

/// Stop the run: the human accepted fixes that nothing landed, and only they can
/// say what happens now.
///
/// The Interruption names the doing rather than the reading, because that is the
/// half that failed — and it says what is owed in the review's own words, so the
/// choice is answerable without opening the Set again. Retrying it is the fixes
/// in one session; taking over is the human making them; aborting ends the run
/// with the branch exactly as the session left it.
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
        store::Step::Review,
        "landing the fixes the review's findings were accepted for",
        &owing(owed, how),
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a review's accepted fixes were never landed and the Interruption saying so \
             could not be raised"
        );
    }
}

/// How much of a finding the Interruption carries.
///
/// This is one line on a card read on a phone, under the step it belongs to, so
/// what it is for is recognising which fix rather than reading it. The whole of
/// each is on the Set the human answered, one row up the same Timeline.
const OWED_WIDTH: usize = 100;

/// What is owed, as the Interruption says it: how many fixes, and the review's
/// own words for each.
///
/// The review's words rather than Verkstead's, because the human decided against
/// those words an hour ago and these are the ones they will recognise.
fn owing(owed: &[store::Fixing], how: Option<&str>) -> String {
    let fixes = owed
        .iter()
        .map(|finding| format!("“{}”", in_a_line(&finding.what)))
        .collect::<Vec<String>>()
        .join("; ");

    let what = format!(
        "{} the human accepted {} landed: {fixes}",
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

/// One finding on one line: whitespace collapsed, and clamped to what a card
/// holds.
fn in_a_line(what: &str) -> String {
    let said = what.split_whitespace().collect::<Vec<&str>>().join(" ");

    match said.char_indices().nth(OWED_WIDTH) {
        Some((cut, _)) => format!("{}…", said[..cut].trim_end()),
        None => said,
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

    /// What a fix session is told about the findings: the review's own words for
    /// whoever would fix them, and that these are decisions rather than
    /// proposals.
    #[test]
    fn a_fix_session_is_told_every_accepted_finding_at_once() {
        let told = feedback(&[
            finding("`window.rs` never resets the counter between windows.", ""),
            finding("`limits.rs` and `window.rs` each grew their own clock.", ""),
        ]);

        assert!(
            told.contains("`window.rs` never resets the counter")
                && told.contains("each grew their own clock"),
            "both findings, in the words the review wrote: {told}",
        );
        assert!(
            told.contains("said to fix them") && told.contains("none of this is still a question"),
            "and that the deciding is over: {told}",
        );
        assert!(
            !told.contains("What they said"),
            "with nothing said about words nobody wrote: {told}",
        );
    }

    /// And their qualification, where they wrote one — which is the whole reason
    /// the Answer's free text is kept on the Set at all.
    #[test]
    fn what_the_human_wrote_alongside_reaches_the_session_that_can_act_on_it() {
        let told = feedback(&[finding(
            "`window.rs` never resets the counter between windows.",
            "Yes, but leave the public signature alone.",
        )]);

        assert!(
            told.contains("leave the public signature alone"),
            "their words reach the session: {told}",
        );
        assert!(
            told.find("never resets the counter") < told.find("leave the public signature"),
            "under the finding rather than over it: {told}",
        );
    }

    /// What the Interruption says: that the fixes never landed, and which ones.
    #[test]
    fn the_interruption_says_what_is_unlanded_in_the_review_s_own_words() {
        let says = owing(
            &[
                finding("Reset the counter as the window rolls.", ""),
                finding("Collapse the two clocks onto one.", ""),
            ],
            None,
        );

        assert!(
            says.contains("2 fixes") && says.contains("never landed"),
            "how much is owed: {says}",
        );
        assert!(
            says.contains("Reset the counter") && says.contains("Collapse the two clocks"),
            "and what, as the review wrote it: {says}",
        );
    }

    /// A session that fell over says both: how it ended, and what it left owed.
    #[test]
    fn a_session_that_ended_badly_says_so_beside_what_it_left() {
        let says = owing(
            &[finding("Reset the counter as the window rolls.", "")],
            Some("exited with status 1"),
        );

        assert!(
            says.contains("exited with status 1") && says.contains("one fix"),
            "how it ended and what is owed: {says}",
        );
    }

    /// A finding written for an agent is paragraphs long, and this is a line on a
    /// card: it is collapsed and clamped, and says that it was.
    #[test]
    fn a_finding_too_long_for_a_card_is_clamped_rather_than_wrapped() {
        let what = format!("first line\n  second line\n\n{}", "x".repeat(200));
        let line = in_a_line(&what);

        assert!(
            line.starts_with("first line second line"),
            "collapsed onto one line: {line}",
        );
        assert!(line.ends_with('…'), "and clamped, visibly: {line}");
        assert!(
            line.chars().count() <= OWED_WIDTH + 1,
            "to what a card holds: {line}",
        );
    }
}
