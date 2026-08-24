//! Where an unattended run stops, and what the human does about it.
//!
//! An Interruption is something Verkstead noticed and cannot resolve itself: a
//! session that exited badly, or one that ended having landed nothing. Roadrunner
//! asked this over askance because nobody was at its terminal; here the Timeline
//! *is* where the human looks, so the same question is GUI-native — the Event
//! carries the evidence and offers the three remedies, and the Conversation
//! carries *blocked on you* until one is chosen.
//!
//! Two halves, and they are deliberately far apart in time. [`raise`] gathers the
//! evidence at the moment the run stopped, because all of it moves on: a Worktree
//! is a directory the human also has, and a session's output belongs to a process
//! that has gone. [`settle`] runs whenever the human gets to it, which may be the
//! next morning, and acts on what they chose.
//!
//! Nothing here reverts, resets or stashes anything. In every case the repository
//! is left exactly as the session left it — that is what makes *take over
//! manually* a remedy at all, and it is why aborting from here does not remove
//! the Worktree the way aborting a Conversation from its menu does: the human is
//! being handed the wreckage on purpose.
//!
//! Usage limits — an account exhausting its window mid-run — are not raised
//! here. They stop a run all the same, and [`held_up`] is where the two are
//! asked about together; what recognises one and what ends the wait is
//! [`crate::limits`].

use std::path::Path;

use anyhow::Result;
use verkstead_render::{Remedy, RemedySettled};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::repos::git;
use crate::store;

/// How much of what git made of the Worktree to keep.
///
/// A session that went wrong mid-refactor can leave hundreds of paths pending,
/// and this is read on a phone. Forty is more than enough to see *what kind of
/// mess* — which is the question the evidence is answering — and what is dropped
/// is said rather than silently cut.
const STATUS_LINES: usize = 40;

/// And how much of what the session last said.
///
/// The tail rather than the whole: what went wrong is at the end, and the whole
/// of it is on the Timeline already as the session's own Event, one row up from
/// this one.
const TAIL_LINES: usize = 40;

/// Stop the run: gather what went wrong and put it on the Timeline.
///
/// `writing` is the Timeline Event the failed session was printing into, which is
/// where the tail comes from. `None` where there was no session to read — a step
/// nothing could be launched for — and the evidence then carries the other three
/// facts alone.
///
/// The Event it became, or `None` where nothing was raised. Neither way of
/// getting `None` is a failure: a Conversation that already has one open is a run
/// that has already stopped, and the first Interruption is the one the human is
/// being asked about. A Conversation that has gone has nobody left to ask.
pub(crate) async fn raise(
    state: &AppState,
    conversation_id: i64,
    step: store::Step,
    what: &str,
    how: &str,
    writing: Option<i64>,
) -> Result<Option<i64>> {
    let evidence = store::Evidence {
        step,
        what: what.to_owned(),
        how: how.to_owned(),
        git_status: worktree_status(state, conversation_id).await,
        tail: session_tail(state, conversation_id, writing).await,
    };

    let raised = store::record_interruption(&state.pool, conversation_id, &evidence).await?;

    match raised {
        Some(event_id) => {
            tracing::warn!(
                conversation_id,
                event_id,
                step = ?step,
                how,
                "a run stopped, so the Conversation is blocked on the human"
            );

            // The Timeline has something on it that is waiting on the human, and
            // an open page should say so without being reloaded.
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });
        }
        None => tracing::info!(
            conversation_id,
            how,
            "a run stopped where one had stopped already, so the first Interruption stands"
        ),
    }

    Ok(raised)
}

/// Which Event a run has stopped on, or `None` where nothing is stopping it.
///
/// The one question every launcher asks before it spends an account, and there
/// are two ways of answering it yes. An open **Interruption** is something
/// Verkstead noticed and cannot resolve, waiting on a Remedy. An open **Pause**
/// is the account itself being out of window, waiting on the human or on the
/// clock — see [`crate::limits`]. Neither is a run to launch anything for, and
/// both leave the Conversation *blocked on you*, so the callers want one answer
/// rather than two questions they could get out of step.
///
/// The Interruption first where somehow both are open, for the reason a Hold
/// comes before an Interruption in the badge: it is the one that needs an answer
/// rather than a wait.
pub(crate) async fn held_up(pool: &sqlx::SqlitePool, conversation_id: i64) -> Result<Option<i64>> {
    if let Some(event_id) = store::open_interruption(pool, conversation_id).await? {
        return Ok(Some(event_id));
    }

    store::open_pause(pool, conversation_id).await
}

/// What git makes of the Conversation's Worktree, as `git status` says it.
///
/// Read now and kept, for the reason a commit's summary is kept: this is a
/// reading of a directory at the moment it went wrong, and the directory moves on
/// — not least because *take over manually* hands it to the human to work in.
///
/// Empty where there is nothing to say: a Conversation with no Worktree left, a
/// repository that will not answer, or a Worktree with nothing pending in it —
/// which is itself worth seeing, since it means the session left no work behind.
async fn worktree_status(state: &AppState, conversation_id: i64) -> String {
    let worktree = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.worktree,
        Ok(None) => None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a Conversation to ask git about failed");
            None
        }
    };

    let Some(worktree) = worktree else {
        return String::new();
    };

    let said = tokio::task::spawn_blocking(move || status(&worktree)).await;

    match said {
        Ok(said) => said,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking git about a Worktree failed");
            String::new()
        }
    }
}

/// The same, blocking: `git status` short, capped, and honest about the cap.
///
/// `--short --branch` rather than `--porcelain`, because this is read by a human
/// rather than parsed — the branch line is the first thing worth knowing about a
/// Worktree a run stopped in. Through [`git`], so it passes `--no-optional-locks`
/// like every other read here: the session may have died holding `index.lock`,
/// and a reader that waited on one would gather no evidence at all.
fn status(worktree: &Path) -> String {
    let Some(said) = git(worktree, &["status", "--short", "--branch"]) else {
        return String::new();
    };

    shorten(&said, STATUS_LINES, "path")
}

/// The tail of what the failed session said, for the human reading the
/// Interruption on a phone.
///
/// The agent's own prose off its Transcript where it kept one. That is the
/// evidence somebody wants: an agent that gave up says why in a sentence, and
/// the terminal underneath that sentence is a display of it — boxes, colours and
/// a status bar — that says the same thing at ten times the length.
///
/// The Capture where there is no Transcript, tidied of the terminal's own
/// sequences. That is every session on a backend keeping no log, and for those
/// it is the whole record rather than a lesser one.
///
/// Empty where there is nothing to read at all: a step nothing could be launched
/// for, or a session that went without a word.
async fn session_tail(state: &AppState, conversation_id: i64, writing: Option<i64>) -> String {
    let Some(event_id) = writing else {
        return String::new();
    };

    match store::transcript(&state.pool, conversation_id, event_id).await {
        Ok(Some(lines)) => {
            let said = verkstead_render::statements(&lines);

            if !said.is_empty() {
                // A blank line between statements, because that is what they
                // are: an agent's turns are paragraphs of markdown and running
                // two of them together would read as one.
                return shorten(&said.join("\n\n"), TAIL_LINES, "line");
            }
        }
        Ok(None) => return String::new(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, event_id, "reading a failed session's Transcript failed");
        }
    }

    match store::capture(&state.pool, conversation_id, event_id).await {
        Ok(Some(capture)) => crate::capture::tail(&capture, TAIL_LINES),
        Ok(None) => String::new(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, event_id, "reading what a failed session said failed");
            String::new()
        }
    }
}

/// The last `keep` lines of `said`, with a line above them saying what was left
/// out.
///
/// Said rather than silently cut. Evidence the human cannot tell is partial is
/// worse than less of it: a status showing forty paths reads as *forty paths
/// changed* unless it says otherwise.
fn shorten(said: &str, keep: usize, what: &str) -> String {
    let lines: Vec<&str> = said.lines().collect();

    if lines.len() <= keep {
        return lines.join("\n");
    }

    let dropped = lines.len() - keep;
    let plural = if dropped == 1 { "" } else { "s" };

    std::iter::once(format!("… and {dropped} earlier {what}{plural}"))
        .chain(lines[dropped..].iter().map(|line| (*line).to_owned()))
        .collect::<Vec<String>>()
        .join("\n")
}

/// Take the human's remedy: record it, then do it.
///
/// Recorded before anything acts on it, for the reason a worktree is recorded
/// before a session is launched in it — an Interruption acted on without being
/// closed is one the human could answer twice, and a retry pressed twice is two
/// agents in one Worktree. [`RemedySettled::AlreadySettled`] is therefore an
/// ordinary answer rather than an error: the second press of a button is the
/// first choice arriving again.
///
/// Only the closing is refused for. What the remedy *does* — a session that would
/// not start, a Conversation that would not move — is logged and left on the
/// record, because by the time it runs the Interruption is closed and answering
/// the button with a failure would say that none of it happened.
pub(crate) async fn settle(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    remedy: Remedy,
    note: &str,
) -> Result<RemedySettled> {
    // Read before it is closed, because the step is what a retry launches again
    // and a settled row is not the one to read it off.
    let step = match store::interruption(&state.pool, conversation_id, event_id).await? {
        Some(interruption) => interruption.evidence.step,
        None => return Ok(RemedySettled::NoSuchInterruption),
    };

    let settling =
        store::settle_interruption(&state.pool, conversation_id, event_id, stored(remedy), note)
            .await?;

    match settling {
        store::Settling::NoSuchInterruption => return Ok(RemedySettled::NoSuchInterruption),
        store::Settling::AlreadySettled => return Ok(RemedySettled::AlreadySettled),
        store::Settling::Settled => {}
    }

    tracing::info!(
        conversation_id,
        event_id,
        remedy = ?remedy,
        step = ?step,
        "an Interruption was settled"
    );

    match remedy {
        // Handed straight back to the runner, which is where a run is driven from
        // whether it is starting or picking up: what step to launch has to be
        // settled before the session is, and the runner is what settles it.
        Remedy::Retry => {
            tokio::spawn(crate::runner::retry(
                state.clone(),
                conversation_id,
                step,
                note.to_owned(),
            ));
        }
        // Nothing to do but stop, which is what settling already did: the run was
        // not advancing, and now nothing will restart it. The Worktree stays
        // exactly as the session left it — which is the whole of what taking over
        // manually needs.
        Remedy::TakeOver => tracing::info!(
            conversation_id,
            "Verkstead has stopped driving; the Worktree is the human's"
        ),
        Remedy::Abort => abort(state, conversation_id).await?,
    }

    Ok(RemedySettled::Settled)
}

/// The store's word for a remedy. One word either side, and this is where the two
/// vocabularies are held to each other.
fn stored(remedy: Remedy) -> store::Remedy {
    match remedy {
        Remedy::Retry => store::Remedy::Retry,
        Remedy::TakeOver => store::Remedy::TakeOver,
        Remedy::Abort => store::Remedy::Abort,
    }
}

/// End the run here: the Conversation stops wherever it had got to.
///
/// Unlike aborting from the Conversation's own menu, this leaves the Worktree
/// exactly where it is. Everything about an Interruption is built on the
/// repository being left as the session left it, and a remedy that swept the
/// evidence away as it ended the run would be the one thing the human could not
/// undo.
///
/// No session is ended, because there is none to end: an Interruption is raised
/// by a session that has already gone.
async fn abort(state: &AppState, conversation_id: i64) -> Result<()> {
    match store::abort_conversation(&state.pool, conversation_id).await? {
        store::Aborting::Aborted => tracing::info!(
            conversation_id,
            "the run was ended at an Interruption; the Worktree is left as the session left it"
        ),
        // Somebody aborted it from the menu between the Interruption being raised
        // and this being pressed, which is the same thing arriving twice.
        store::Aborting::AlreadyAborted => tracing::info!(
            conversation_id,
            "the run had already been ended when the Interruption was aborted"
        ),
        store::Aborting::NoSuchConversation => tracing::error!(
            conversation_id,
            "there is no Conversation left to end the run of"
        ),
    }

    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evidence the human cannot tell is partial is worse than less of it, so
    /// what was left out is said rather than silently cut.
    #[test]
    fn a_long_status_keeps_its_end_and_says_what_it_dropped() {
        let said: String = (1..=45).map(|n| format!(" M file{n}.rs\n")).collect();

        let shortened = shorten(&said, STATUS_LINES, "path");
        let lines: Vec<&str> = shortened.lines().collect();

        assert_eq!(
            lines.len(),
            STATUS_LINES + 1,
            "the cap, and one line saying what is missing",
        );
        assert_eq!(lines[0], "… and 5 earlier paths");
        assert_eq!(
            lines[1], " M file6.rs",
            "the end of it is what is kept: the last thing that happened",
        );
        assert_eq!(lines[STATUS_LINES], " M file45.rs");
    }

    /// The ordinary case, which is nearly every one: a session that stopped
    /// having touched a handful of files.
    #[test]
    fn a_short_status_is_left_exactly_as_git_said_it() {
        let said = "## rate-limiting\n M crates/limiter/src/lib.rs\n?? notes.md\n";

        assert_eq!(
            shorten(said, STATUS_LINES, "path"),
            "## rate-limiting\n M crates/limiter/src/lib.rs\n?? notes.md",
        );
    }

    /// One dropped line is one path, not one paths.
    #[test]
    fn what_was_dropped_is_counted_in_words_that_agree() {
        let said: String = (1..=STATUS_LINES + 1).map(|n| format!("{n}\n")).collect();

        assert!(
            shorten(&said, STATUS_LINES, "path").starts_with("… and 1 earlier path\n"),
            "{}",
            shorten(&said, STATUS_LINES, "path"),
        );
    }
}
