//! Where driving stops, and what the human is told about it.
//!
//! A halt is Verkstead saying *nothing is driving this Conversation any more*.
//! It is two things written together: durable state on the Conversation — see
//! [`store::halt`] — and an ordinary **Notice** on its Timeline saying what
//! stopped, why, and what the evidence was. The state is what the *blocked on
//! you* badge is drawn from and what a restart reads; the Notice is what the
//! human reads.
//!
//! The evidence is gathered at the moment the run stopped, because all of it
//! moves on: a Worktree is a directory the human also has, and a session's
//! output belongs to a process that has gone. It goes into the Notice as
//! markdown rather than into columns of its own — a Notice is prose somebody
//! reads on a phone, and what a stop needs to say is a paragraph and two blocks
//! of terminal text.
//!
//! **A halt Verkstead decided on also reaches a phone.** The Notice is what
//! says it in full, and it says it to somebody who is looking; a run that
//! stopped is a run that stays stopped until Resume is pressed, so a stop
//! nobody is told about is one found days late. A stop nobody chose sends
//! nothing, a restart being free to pick that one up unasked — and a stop the
//! human pressed for sends nothing either, they being the one person a
//! notification about it would be telling their own news. See [`Decided`], which
//! is the whole of that rule, and [`crate::push::halted`].
//!
//! Nothing here reverts, resets or stashes anything, and nothing here starts
//! anything either. The repository is left exactly as the session left it, and
//! getting going again is Resume's — one press, recomputed from the state the
//! Conversation is in now rather than from what stopped.

use std::path::Path;

use anyhow::Result;
use verkstead_schema::Nudge;

use crate::AppState;
use crate::repos::git;
use crate::store;

/// Who stopped it, which decides the two things that follow from a halt: whether
/// a restart takes the Conversation up unasked, and whether a phone is told.
///
/// Both of those are really one question — *is anybody waiting on this?* — asked
/// of the record and of a pocket. Verkstead pulling the brake is waited on by
/// nobody until they are told, so it is pushed and it waits for a press. A stop
/// nobody chose is waited on by nothing at all: the next server up carries the
/// work on. And a stop the human pressed for waits for their press like the
/// first, but tells them nothing, because they are the one person who already
/// knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decided {
    /// Verkstead pulled the brake: a session that fell over, checks that would
    /// not go green, a finish step that left no pull request.
    Verkstead,

    /// The human pressed Stop or Force stop.
    Human,

    /// Nobody: a restart or a crash took the driver away.
    Nobody,
}

impl Decided {
    /// What the record keeps, which is only whether anybody chose — see
    /// [`store::Halt`]. Verkstead and the human are one thing to a restart: both
    /// are decisions, and neither is a server's to overturn.
    fn halt(self) -> store::Halt {
        match self {
            Self::Verkstead | Self::Human => store::Halt::Deliberate,
            Self::Nobody => store::Halt::Circumstance,
        }
    }

    /// And whether the human's devices are told.
    fn pushes(self) -> bool {
        matches!(self, Self::Verkstead)
    }
}

/// Whether driving has already stopped, which is what nothing may advance past.
///
/// Asked wherever a session is about to be launched — the runner between steps,
/// and each of the wrap-up's watchers before it dispatches — so that a
/// Conversation the human has to press Resume on does not quietly get another
/// agent spent on it. The one halt per Conversation makes a second stop
/// impossible; this makes a session behind the first one impossible too.
///
/// A Stop the human pressed while a session was running lands here: this is the
/// next launch it asked to come before, so the stop becomes a halt and the
/// launch does not happen — see [`crate::stops::asked`].
///
/// A store that will not answer reads as *stopped*, which is the right way round
/// for the one thing this decides: what is on the other side of it is launching
/// an agent, and something that could not tell whether the run had stopped
/// should wait rather than spend an account guessing.
pub(crate) async fn stopped(state: &AppState, conversation_id: i64) -> bool {
    if crate::stops::asked(state, conversation_id).await {
        return true;
    }

    match store::halted(&state.pool, conversation_id).await {
        Ok(Some(halted)) => {
            tracing::info!(
                conversation_id,
                event_id = halted.event_id,
                "driving has stopped, so nothing was launched"
            );
            true
        }
        Ok(None) => false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether driving had stopped failed");
            true
        }
    }
}

/// How much of what git made of the Worktree to keep.
///
/// A session that went wrong mid-refactor can leave hundreds of paths pending,
/// and this is read on a phone. Forty is more than enough to see *what kind of
/// mess* — which is the question the evidence is answering — and what is dropped
/// is said rather than silently cut.
pub(crate) const STATUS_LINES: usize = 40;

/// And how much of what the session last said.
///
/// The tail rather than the whole: what went wrong is at the end, and the whole
/// of it is on the Timeline already as the session's own Event, one row up from
/// this one.
pub(crate) const TAIL_LINES: usize = 40;

/// Stop driving: gather what happened and put a halt and its Notice on the
/// Timeline.
///
/// `what` is what ought to have been happening — "implementing the work" — and
/// `how` is why it is not, in the words the log uses. `writing` is the Timeline
/// Event the last session was printing into, which is where the tail comes
/// from; `None` where there was no session to read.
///
/// The Notice it became, or `None` where nothing was written. Neither way of
/// getting `None` is a failure: a Conversation already halted is one that has
/// already stopped, and the first Notice is the one that explains it — the
/// sweep looking again a minute later finds the same Conversation standing just
/// as still. A Conversation that has gone has nobody left to tell.
///
/// A halt that was written and that Verkstead decided on is also pushed to the
/// human's devices — see [`Decided`], which says which stops those are. The
/// `None` above is what keeps that to one push per stop: the sweep that finds
/// the same Conversation standing still writes nothing, so there is nothing here
/// to tell anybody about twice.
pub(crate) async fn halt(
    state: &AppState,
    conversation_id: i64,
    decided: Decided,
    what: &str,
    how: &str,
    writing: Option<i64>,
) -> Result<Option<i64>> {
    let said = said(
        what,
        how,
        &worktree_status(state, conversation_id).await,
        &session_tail(state, conversation_id, writing).await,
    );

    let halted = store::halt(&state.pool, conversation_id, decided.halt(), &said).await?;

    match halted {
        Some(event_id) => {
            tracing::warn!(
                conversation_id,
                event_id,
                decided = ?decided,
                how,
                "driving stopped, so the Conversation is blocked on the human"
            );

            // The Timeline has something on it that is waiting on the human, and
            // an open page should say so without being reloaded.
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });

            // And a phone, where Verkstead decided to stop. Behind the Nudge
            // rather than in front of it, both being handed to the runtime
            // either way: the page somebody is looking at should not wait on a
            // push service to redraw. See [`crate::push`] for why a stop nobody
            // chose sends nothing, and [`Decided`] for why the human's own press
            // sends nothing either.
            if decided.pushes() {
                crate::push::halted(state, conversation_id, &opening(what));
            }
        }
        None => tracing::info!(
            conversation_id,
            how,
            "driving stopped where it had stopped already, so the first halt stands"
        ),
    }

    Ok(halted)
}

/// What the Notice says: the stop, the reason, and the two pieces of evidence.
///
/// The evidence is indented rather than fenced, which is the one thing here
/// that is not the obvious way round. What goes in it is a terminal's output
/// and an agent's own prose, and an agent's prose is full of fences: a fenced
/// block would end wherever the tail happened to hold three backticks, and the
/// rest of the evidence would render as markdown of somebody else's.
///
/// Both blocks are always drawn, empty or not, and an empty one says why it is
/// empty. Evidence the human cannot tell is missing is worse than none: a stop
/// with no *Worktree* heading reads as a stop nobody looked into.
fn said(what: &str, how: &str, git_status: &str, tail: &str) -> String {
    format!(
        "**{}** stopped.\n\n{how}\n\n### The worktree\n\n{}\n\n### What the last session said\n\n{}\n",
        opening(what),
        indented(
            git_status,
            "Git had nothing pending, or the repository would not answer.",
        ),
        indented(tail, "It said nothing at all."),
    )
}

/// The stop with its first letter up, because it opens the sentence the Notice
/// is. Every caller names it the way the log does — "implementing the work" —
/// and a Notice opening in lower case would read as half a line.
fn opening(what: &str) -> String {
    let mut letters = what.chars();

    match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    }
}

/// A block of terminal output as markdown holds it: every line indented by
/// four, which is the one form nothing inside it can break out of.
///
/// `empty` is the sentence that stands in its place where there is nothing to
/// show, and it is prose rather than a block: there is nothing to preserve the
/// columns of.
fn indented(said: &str, empty: &str) -> String {
    let block: Vec<String> = said.lines().map(|line| format!("    {line}")).collect();

    if block.is_empty() {
        return empty.to_owned();
    }

    block.join("\n")
}

/// What git makes of the Conversation's Worktree, as `git status` says it.
///
/// Read now and kept, for the reason a commit's summary is kept: this is a
/// reading of a directory at the moment it went wrong, and the directory moves on
/// — not least because the human is being handed it to work in.
///
/// Empty where there is nothing to say: a Conversation with no Worktree left, a
/// repository that will not answer, or a Worktree with nothing pending in it —
/// which is itself worth seeing, since it means the session left no work behind.
pub(crate) async fn worktree_status(state: &AppState, conversation_id: i64) -> String {
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

/// The tail of what the last session said, for the human reading the Notice on
/// a phone.
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
pub(crate) async fn session_tail(
    state: &AppState,
    conversation_id: i64,
    writing: Option<i64>,
) -> String {
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
            tracing::error!(error = ?error, conversation_id, event_id, "reading a stopped session's Transcript failed");
        }
    }

    match store::capture(&state.pool, conversation_id, event_id).await {
        Ok(Some(capture)) => crate::capture::tail(&capture, TAIL_LINES),
        Ok(None) => String::new(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, event_id, "reading what a stopped session said failed");
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
pub(crate) fn shorten(said: &str, keep: usize, what: &str) -> String {
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

    /// What the human reads: the stop, why, and both pieces of evidence set
    /// apart from the prose around them.
    #[test]
    fn the_notice_says_what_stopped_why_and_what_the_evidence_was() {
        let said = said(
            "implementing the work",
            "nothing is driving it: no session is running",
            "## rate-limiting\n M limiter.md",
            "the task is beyond me",
        );

        assert_eq!(
            said,
            "**Implementing the work** stopped.\n\n\
             nothing is driving it: no session is running\n\n\
             ### The worktree\n\n\
             \x20   ## rate-limiting\n    \x20M limiter.md\n\n\
             ### What the last session said\n\n\
             \x20   the task is beyond me\n",
        );
    }

    /// A tail that holds a fence of its own is still the tail. An agent's prose
    /// is full of them, and a block that ended at the first one would let the
    /// rest of the evidence render as markdown of somebody else's.
    #[test]
    fn evidence_holding_a_fence_stays_inside_its_own_block() {
        let said = said(
            "wrapping the work up",
            "the checks are red",
            "",
            "```\nrm -rf\n```",
        );

        assert!(
            said.contains("    ```\n    rm -rf\n    ```\n"),
            "every line of it is indented, fences and all: {said:?}",
        );
    }

    /// And where there was nothing to gather, the block says so rather than
    /// being left out: a stop with no *Worktree* heading reads as a stop nobody
    /// looked into.
    #[test]
    fn evidence_nobody_could_gather_says_that_it_is_missing() {
        let said = said("grilling the work", "nothing is driving it", "", "");

        assert!(
            said.contains("Git had nothing pending, or the repository would not answer."),
            "{said:?}",
        );
        assert!(said.contains("It said nothing at all."), "{said:?}");
    }
}
