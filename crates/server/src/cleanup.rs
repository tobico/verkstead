//! The sweep that lets go of what the human has finished looking at: the
//! Cleanup, which trims an archived Conversation some days after the archiving
//! and, where the human has asked for it, deletes it some days after that.
//!
//! Everything else in Verkstead keeps what it was given, and a Conversation the
//! human archived a fortnight ago is still holding every byte of every session
//! that ran in it. What a trim takes is the part of that nobody opens twice —
//! the full agent output, the Transcripts, the session names — and what it
//! leaves is the record: every card on the Timeline, and a Share exactly as it
//! was. See [`crate::store::trim_conversation`], where that boundary is drawn.
//!
//! **And a delete takes the whole of it**, which is the one thing in Verkstead
//! that forgets: every row the Conversation owns, and nothing outside the store
//! — see [`crate::store::delete_conversation`]. It is **off** until somebody
//! turns it on, and that is what makes it a feature rather than a hazard.
//!
//! **One clock each, and both start at the archiving.** Not at the close, and
//! not at the last thing that happened: archiving is the human saying they are
//! finished looking, which is the only moment in the record that means what a
//! cleanup needs it to mean. So an unarchiving stops both by taking the
//! archiving away, and a second archiving starts them again over whatever has
//! been printed since. Neither waits on the other — a delete set sooner than a
//! trim simply takes a Conversation that was never trimmed.
//!
//! **Hourly, over a threshold of days.** The pace is [`crate::Pace::cleanup`]
//! and the thresholds are the settings' — [`crate::settings::Cleanup`], whose
//! fallbacks where nobody has typed one are [`TRIMMED_AFTER`] and
//! [`DELETED_AFTER`]. An hour is plenty for a clock counted in days, and it is
//! what decides how soon after a threshold is crossed rather than whether it is
//! noticed at all.
//!
//! **And the settings are read on every pass**, like everything else out of
//! `config.yaml`: a switch turned off from a phone stops the next sweep, a
//! duration typed there is what the one after it goes by, and neither waits for
//! a restart.
//!
//! **The backlog goes on the first pass.** There is nothing here that only
//! looks at what was archived since it shipped: every Conversation already past
//! a threshold is cleaned the first time this runs, which is the deliberate
//! reading of *a cleanup, three days after the archiving* — and the reading of
//! the delete switch being turned on, which is a human saying what should
//! happen to what they have already put away.
//!
//! **And it says what it did in the log and nowhere else.** Nothing is refused
//! and nothing comes back. A cleanup writes no Timeline Event, sends no Nudge
//! and puts nothing in front of the human — a card that said *this was cleaned
//! up* would be the record growing where it was supposed to shrink, and the mark
//! the Conversation's own page draws is the record of a trim. A delete leaves no
//! mark at all, there being nothing left for one to be on.
//! **And a pass that took something gives the space back.** SQLite frees the
//! pages a delete emptied inside the file and leaves the file the size it was,
//! so the one thing here that is about disk would reclaim no disk at all — see
//! [`crate::store::reclaim`], run once at the end of a pass that took something
//! and never after one that found nothing.

use std::time::Duration;

use crate::AppState;
use crate::store;

/// How long a Conversation is archived before its bulk is taken, where nobody
/// has typed a number of their own — see
/// [`crate::settings::Cleanup::trim_after`], which is what the sweep reads.
///
/// Three days: long enough that a human who archived one by mistake, or wants
/// one more look at what a session printed, has a working day or two to say so;
/// short enough that the thing being kept for is a thing they would remember
/// wanting.
pub(crate) const TRIMMED_AFTER: u32 = 3;

/// And how long before the whole of it goes, where nobody has typed one either
/// — see [`crate::settings::Cleanup::delete_after`].
///
/// Thirty days, and only ever the fallback of a switch that is **off** until
/// somebody turns it on: a delete is the one thing in Verkstead that forgets,
/// so the number here is the one a human turning it on would have chosen
/// anyway rather than one that ever runs unasked.
pub(crate) const DELETED_AFTER: u32 = 30;

/// How often the archived Conversations are looked over, as [`crate::Pace`] has
/// it by default.
///
/// An hour. The clock this reads is counted in days, so a pass an hour is
/// already twenty-four times finer than the thing it is measuring — and what a
/// pass costs is one query that usually comes back with nothing.
pub(crate) const SWEPT_EVERY: Duration = Duration::from_secs(60 * 60);

/// Clean up after the archivings from now until the process stops: once at
/// startup, and every [`crate::Pace::cleanup`] after that.
///
/// At startup rather than after anything, for [`crate::merges::sweeping`]'s
/// reason: what this looks at is work the human has finished with, so there is
/// nothing a resume could be in the middle of putting right.
///
/// And never, on a server that runs no sessions — the other sweeps' reason,
/// which is that only the tests' routers are built that way: a fixture standing
/// a router up over a store held still would otherwise have rows deleted out
/// from under whatever it was written to assert.
pub(crate) fn sweeping(state: &AppState) {
    if !state.sessions.runs_sessions() {
        return;
    }

    let state = state.clone();

    tokio::spawn(async move {
        loop {
            sweep(&state).await;

            tokio::time::sleep(state.sessions.pace().cleanup).await;
        }
    });
}

/// One pass over everything there is a cleanup to do on.
///
/// Nothing is refused for and nothing is returned, [`crate::merges`]'s sweep
/// said again: this runs unattended with nobody watching, and what it has to
/// say it says in the log.
///
/// Both halves listed first and then done one at a time, rather than done in
/// the query that finds them: each is its own transaction, so a Conversation
/// unarchived from a phone while this is walking the list is one the store
/// itself refuses. Which is what the outcome is read for — the sweep asked for
/// something the record had moved on from, and the record is right.
async fn sweep(state: &AppState) {
    // Read off the file on every pass rather than held from startup, which is
    // what makes a switch flipped on a phone take effect on the next one. One
    // small file, read here rather than on a blocking thread for
    // [`crate::comments`]'s reason: a pass an hour is not a hot path.
    let config = state.settings.config();
    let cleanup = config.cleanup();

    // The delete first, and it is worth a sentence. The two clocks are
    // independent, so a Conversation can be past both at once — and a pass that
    // trimmed it and then deleted it would have taken the bulk out of something
    // it was about to take altogether, and said so twice in the log. Deleting
    // first leaves the trim nothing but what is staying.
    //
    // Both halves run whatever the other did, so neither is written with `||`:
    // what is collected here is whether there is space to give back, and a
    // delete that took something is no reason to skip the trim.
    let deleted = deleting(state, cleanup).await;
    let trimmed = trimming(state, cleanup).await;

    // And the rewrite that turns rows nobody can reach any more into disk the
    // human can use, once, after a pass that took something. A failure is a pass
    // that did not reclaim rather than a cleanup that went wrong — the rows are
    // gone either way, and the next pass to take something tries again.
    if deleted || trimmed {
        match store::reclaim(&state.pool).await {
            Ok(()) => tracing::info!("the space a cleanup freed has been given back"),
            Err(error) => {
                tracing::error!(error = ?error, "giving back the space a cleanup freed failed");
            }
        }
    }
}

/// The delete half of one pass: what has been archived longer than the delete's
/// days, gone.
///
/// **Off unless the human has turned it on**, which is the whole of why this is
/// safe to have written at all — see [`crate::settings::Cleanup::deletes`]. And
/// when they do turn it on, everything already past the threshold goes on the
/// next pass, exactly as the trim's backlog does: *a delete, thirty days after
/// the archiving* is a rule about the record rather than about what has happened
/// since somebody found the settings page.
///
/// Answers whether it took anything, which is what says there is space to give
/// back — see [`sweep`].
async fn deleting(state: &AppState, cleanup: &crate::settings::Cleanup) -> bool {
    if !cleanup.deletes() {
        tracing::debug!("the Cleanup's delete is switched off, so nothing is deleted");
        return false;
    }

    let delete_after = cleanup.delete_after();

    let waiting = match store::deletable(&state.pool, delete_after).await {
        Ok(waiting) => waiting,
        Err(error) => {
            tracing::error!(error = ?error, "listing the archived Conversations to delete failed");
            return false;
        }
    };

    let mut deleted = false;

    for conversation_id in waiting {
        match store::delete_conversation(&state.pool, conversation_id).await {
            Ok(store::Deletion::Deleted) => {
                deleted = true;

                tracing::info!(
                    conversation_id,
                    archived_for = delete_after,
                    "an archived Conversation has been deleted",
                );
            }
            Ok(outcome) => tracing::debug!(
                conversation_id,
                outcome = ?outcome,
                "an archived Conversation moved on before the sweep reached it",
            ),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "deleting an archived Conversation failed");
            }
        }
    }

    deleted
}

/// And the trim half, which is the same shape a step earlier.
async fn trimming(state: &AppState, cleanup: &crate::settings::Cleanup) -> bool {
    if !cleanup.trims() {
        tracing::debug!("the Cleanup's trim is switched off, so nothing is trimmed");
        return false;
    }

    let trim_after = cleanup.trim_after();

    let waiting = match store::trimmable(&state.pool, trim_after).await {
        Ok(waiting) => waiting,
        Err(error) => {
            tracing::error!(error = ?error, "listing the archived Conversations to clean up failed");
            return false;
        }
    };

    let mut trimmed = false;

    for conversation_id in waiting {
        match store::trim_conversation(&state.pool, conversation_id).await {
            Ok(store::Trimming::Trimmed) => {
                trimmed = true;

                tracing::info!(
                    conversation_id,
                    archived_for = trim_after,
                    "an archived Conversation has been trimmed",
                );
            }
            Ok(outcome) => tracing::debug!(
                conversation_id,
                outcome = ?outcome,
                "an archived Conversation moved on before the sweep reached it",
            ),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "trimming an archived Conversation failed");
            }
        }
    }

    trimmed
}
