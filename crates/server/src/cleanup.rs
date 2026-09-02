//! The sweep that lets go of what the human has finished looking at: the
//! Cleanup, which trims an archived Conversation some days after the archiving.
//!
//! Everything else in Verkstead keeps what it was given, and a Conversation the
//! human archived a fortnight ago is still holding every byte of every session
//! that ran in it. What a trim takes is the part of that nobody opens twice —
//! the full agent output, the Transcripts, the session names — and what it
//! leaves is the record: every card on the Timeline, and a Share exactly as it
//! was. See [`crate::store::trim_conversation`], where that boundary is drawn.
//!
//! **One clock, and it starts at the archiving.** Not at the close, and not at
//! the last thing that happened: archiving is the human saying they are finished
//! looking, which is the only moment in the record that means what a cleanup
//! needs it to mean. So an unarchiving stops the clock by taking the archiving
//! away, and a second archiving starts a new one over whatever has been printed
//! since.
//!
//! **Hourly, over a threshold of days.** The pace is [`crate::Pace::cleanup`]
//! and the threshold is the settings' — [`crate::settings::Cleanup`], whose
//! fallback where nobody has typed one is [`TRIMMED_AFTER`]. An hour is plenty
//! for a clock counted in days, and it is what decides how soon after a
//! threshold is crossed rather than whether it is noticed at all.
//!
//! **And the settings are read on every pass**, like everything else out of
//! `config.yaml`: a switch turned off from a phone stops the next sweep, a
//! duration typed there is what the one after it goes by, and neither waits for
//! a restart.
//!
//! **The backlog goes on the first pass.** There is nothing here that only
//! looks at what was archived since it shipped: every Conversation already past
//! the threshold is trimmed the first time this runs, which is the deliberate
//! reading of *a cleanup, three days after the archiving*.
//!
//! **And it says what it did in the log and nowhere else.** Nothing is refused
//! and nothing comes back. A trim writes no Timeline Event, sends no Nudge and
//! puts nothing in front of the human — a card that said *this was cleaned up*
//! would be the record growing where it was supposed to shrink, and the mark
//! the Conversation's own page draws is the record of it.

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
/// Listed first and then trimmed one at a time, rather than done in the query
/// that finds them: each trim is its own transaction, so a Conversation
/// unarchived from a phone while this is walking the list is one the trim
/// itself refuses. Which is what the outcome is read for — the sweep asked for
/// something the record had moved on from, and the record is right.
async fn sweep(state: &AppState) {
    // Read off the file on every pass rather than held from startup, which is
    // what makes a switch flipped on a phone take effect on the next one. One
    // small file, read here rather than on a blocking thread for
    // [`crate::comments`]'s reason: a pass an hour is not a hot path.
    let config = state.settings.config();
    let cleanup = config.cleanup();

    if !cleanup.trims() {
        tracing::debug!("the Cleanup's trim is switched off, so nothing is trimmed");
        return;
    }

    let trim_after = cleanup.trim_after();

    let waiting = match store::trimmable(&state.pool, trim_after).await {
        Ok(waiting) => waiting,
        Err(error) => {
            tracing::error!(error = ?error, "listing the archived Conversations to clean up failed");
            return;
        }
    };

    for conversation_id in waiting {
        match store::trim_conversation(&state.pool, conversation_id).await {
            Ok(store::Trimming::Trimmed) => tracing::info!(
                conversation_id,
                archived_for = trim_after,
                "an archived Conversation has been trimmed",
            ),
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
}
