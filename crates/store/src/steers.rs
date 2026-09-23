//! What the Steer form settled, kept beside the Steer Event it became.
//!
//! The Event's own body is where the human said the work goes and what they
//! wrote to send it there with — see [`super::Event::Steer`]. Everything else
//! the form asked is here: the digest tick, the interrupt tick, the Pairing
//! they picked and the companion rows they asked for. Together the two are the
//! form as it was filled, which is what the details pane draws it back as.
//!
//! **Beside the Event and keyed by it, the way a commit's repository is.**
//! `timeline_events` is STRICT and has two columns, so a fact that arrives
//! after it does arrives in a table of its own — there is no migration
//! machinery here to add a column with. One row per Steer Event, by the primary
//! key, written in the steer's own transaction: a record that said where the
//! work went without saying what was picked to run it would be half an account
//! of one press.
//!
//! **The companion rows are two tables beside it**, for the reason a pending
//! steer's are: a steer that added three repositories asked three rows' worth
//! of questions, and a column would be a list encoded into a string nothing
//! could read back a row at a time.
//!
//! **An Event with no row is a steer recorded before any of this was written
//! down**, and it is not an error anywhere: what it means is a steer whose
//! ticks and Pairing were never kept, and the pane draws it with the fields it
//! has. ADR-0006's rule — the record is kept and read as it was written.
//!
//! **The Profile is named by id and not copied**, unlike the one a finished
//! session wrote down — see [`super::RanUnder`], which keeps the name because
//! what it is for is saying what *ran*. This is a record of what was *picked*,
//! and what the pane draws is the picker's own reading of it, which wants the
//! account as it stands. So the id is held with no foreign key behind it: a
//! Profile the human has finished with is removable — see
//! [`super::delete_profile`] — and a steer that named one reads back as
//! [`RecordedPairing::Removed`] rather than as a steer that picked nothing.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::{Sqlite, Transaction};

use super::{CompanionMode, Pairing};

/// Everything one steer settled that its Event's own body does not carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerRecord {
    /// Whether the round a steer into Grilling opened was primed with
    /// everything already answered.
    ///
    /// `false` on every other target, the tick being drawn under Grilling
    /// alone.
    pub digest: bool,

    /// And whether the session running at the submit was ended where it stood.
    pub interrupt: bool,

    /// What the form's picker was on, which is what the steer settled as the
    /// Conversation's own.
    pub pairing: RecordedPairing,

    /// The companions the steer put into the sandbox, one entry per row the
    /// human ticked, by the Repo's name.
    pub added: Vec<SteerAddition>,

    /// And the ones already there it opened up, one per row ticked up.
    pub upgraded: Vec<SteerUpgrade>,
}

/// The Pairing a steer recorded, as it reads back now.
///
/// Three states rather than an `Option`, because the middle one is a fact about
/// the record rather than an absence: a steer into Done picked nothing and a
/// steer whose account has since been removed picked something that is gone,
/// and a pane that drew them the same would say *nothing picked* over a choice
/// the human made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedPairing {
    /// Nothing was picked: a steer into Done, where nothing runs and so nothing
    /// is picked to run it — or a steer recorded before any of this was written
    /// down.
    Nothing,

    /// One was picked and the Profile it named has been removed since.
    Removed,

    /// The Profile as it stands, and the model picked beside it.
    Under(Pairing),
}

/// One companion row of the form as the record keeps it: a Repo the steer put
/// into the sandbox, and what the row said about it.
///
/// The Repo by name rather than by id, as a commit's is: what every reader of
/// this wants is what the repository is called, and the row is drawn rather
/// than acted on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerAddition {
    pub repo: String,

    pub mode: CompanionMode,

    /// What its checkout came off, or `None` for the rule: that repository's
    /// default branch as origin held it.
    pub base_ref: Option<String>,

    /// What a read-write one's branch was called, empty being *mirroring* — the
    /// Conversation's own branch name.
    pub branch: String,
}

/// And one row of the set already there that the steer opened up.
///
/// One field beside the Repo, for [`super::Opening`]'s reason: there is one
/// direction, and what the new branch came off is the base already on the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerUpgrade {
    pub repo: String,

    pub branch: String,
}

/// What a submit settled beside the move, for [`record`] to write down.
///
/// The three things the Steer Event's own body cannot hold. The companion rows
/// are not here: they are the rows the same transaction is already joining —
/// see [`super::Steer::companions`] and [`super::Steer::opened`] — and a second
/// copy of them on the submit would be two shapes to keep true about one press.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Recorded<'a> {
    pub digest: bool,

    pub interrupt: bool,

    /// The Pairing the form's picker was on, where the target runs something.
    ///
    /// `None` on a steer into Done, and on one whose human left the picker
    /// exactly where the Conversation already had it — which is a pick all the
    /// same, and the submit sends it, so what is absent here is a form that
    /// sent none.
    pub pairing: Option<PickedPairing<'a>>,
}

/// Both halves of that pick, borrowed off the submit for the call's length —
/// [`super::Settling`]'s reason, this being read straight off what the form
/// sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickedPairing<'a> {
    pub profile_id: i64,

    /// One of that Profile's models. Never absent: there is no default model
    /// anywhere, so a Pairing is picked whole or not at all.
    pub model: &'a str,
}

/// The tables the record lives in: the row, and the two lists of companion rows
/// hanging off it.
///
/// The Conversation is on the row as well as on the Event above it, which is
/// the one thing here written twice — a commit's is, for the same reason turned
/// the other way about: this is what lets a whole Timeline's worth of them be
/// read in one query rather than one per Event.
///
/// The Profile is named with no `REFERENCES` behind it, and that is deliberate:
/// a record points at an account that may go, and a foreign key would make
/// removing a Profile the human has finished with refuse on the strength of a
/// steer they made a year ago. See the module docs.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS steers (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             digest          INTEGER NOT NULL,
             interrupt       INTEGER NOT NULL,
             profile_id      INTEGER,
             model           TEXT
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the steers table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS steer_additions (
             event_id INTEGER NOT NULL REFERENCES timeline_events(id),
             repo_id  INTEGER NOT NULL REFERENCES repos(id),
             mode     TEXT NOT NULL,
             base_ref TEXT,
             branch   TEXT NOT NULL,
             PRIMARY KEY (event_id, repo_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the steer additions table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS steer_upgrades (
             event_id INTEGER NOT NULL REFERENCES timeline_events(id),
             repo_id  INTEGER NOT NULL REFERENCES repos(id),
             branch   TEXT NOT NULL,
             PRIMARY KEY (event_id, repo_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the steer upgrades table")?;

    Ok(())
}

/// Write the rest of the form down beside the Steer Event it became.
///
/// Takes the transaction the move is being made in, for the reason the
/// companions do: a Conversation that steered without saying what was picked to
/// run the work would be a record half written, and the human reading it back
/// would have no way of knowing which half.
///
/// The companion rows are the ones the same transaction is joining and opening
/// — what was asked for and what came in are the same list, a steer being
/// refused whole where any row of it could not be made.
pub(crate) async fn record(
    tx: &mut Transaction<'_, Sqlite>,
    event_id: i64,
    conversation_id: i64,
    recorded: Recorded<'_>,
    added: &[super::Joining<'_>],
    upgraded: &[super::Opening<'_>],
) -> Result<()> {
    sqlx::query(
        "INSERT INTO steers (event_id, conversation_id, digest, interrupt, profile_id, model)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(recorded.digest)
    .bind(recorded.interrupt)
    .bind(recorded.pairing.map(|pairing| pairing.profile_id))
    .bind(recorded.pairing.map(|pairing| pairing.model))
    .execute(&mut **tx)
    .await
    .with_context(|| {
        format!("recording what the steer of Conversation {conversation_id} settled")
    })?;

    for companion in added {
        sqlx::query(
            "INSERT INTO steer_additions (event_id, repo_id, mode, base_ref, branch)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(event_id)
        .bind(companion.repo_id)
        .bind(companion.mode.stored())
        .bind(companion.base_ref)
        .bind(companion.branch)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!("recording what the steer of Conversation {conversation_id} added")
        })?;
    }

    for companion in upgraded {
        sqlx::query("INSERT INTO steer_upgrades (event_id, repo_id, branch) VALUES (?, ?, ?)")
            .bind(event_id)
            .bind(companion.repo_id)
            .bind(companion.branch)
            .execute(&mut **tx)
            .await
            .with_context(|| {
                format!("recording what the steer of Conversation {conversation_id} opened up")
            })?;
    }

    Ok(())
}

/// What each steer on a Conversation's Timeline settled, by the Event it
/// belongs to.
///
/// Three reads for the whole Timeline rather than three per steer, which is the
/// arrangement a commit's summary is read in and for the same arithmetic: the
/// Timeline's own query is at the number of columns a tuple can be read back
/// as, and a Conversation the human has steered a dozen times would otherwise
/// be a query per press of it.
///
/// A Conversation nobody has steered answers all three with nothing.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, SteerRecord>> {
    /// The row's own columns, in the order the query below selects them: the
    /// Event it hangs off, the two ticks, and the Pairing as the picker was on
    /// it.
    type Row = (i64, bool, bool, Option<i64>, Option<String>);

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT event_id, digest, interrupt, profile_id, model
         FROM steers WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what the steers of Conversation {conversation_id} settled")
    })?;

    let mut added = additions(pool, conversation_id).await?;
    let mut upgraded = upgrades(pool, conversation_id).await?;

    let mut records = HashMap::with_capacity(rows.len());

    for (event_id, digest, interrupt, profile_id, model) in rows {
        // The Profile read as a row rather than left as the id it is named by,
        // because what the pane draws is the picker's own reading of the
        // account — which wants the row as it stands. Read a steer at a time,
        // as a Repo's memory of what it was last grilled with is: a
        // Conversation is steered a handful of times, and a Timeline with no
        // steer on it asks nothing at all.
        let pairing = match profile_id {
            None => RecordedPairing::Nothing,
            Some(profile_id) => match super::load_profile(pool, profile_id).await? {
                Some(profile) => RecordedPairing::Under(Pairing { profile, model }),
                // Named an account the Profile list no longer holds, which is a
                // pick the human made and has since finished with rather than a
                // pick they never made.
                None => RecordedPairing::Removed,
            },
        };

        records.insert(
            event_id,
            SteerRecord {
                digest,
                interrupt,
                pairing,
                // Taken out rather than looked up, because each list belongs to
                // exactly one Event and the Events are walked once.
                added: added.remove(&event_id).unwrap_or_default(),
                upgraded: upgraded.remove(&event_id).unwrap_or_default(),
            },
        );
    }

    Ok(records)
}

/// The companions each steer on this Timeline put in, by the Event.
///
/// Driven off the steers rather than off the Events, which is what keeps this
/// one read: the row that says which Conversation a steer belongs to is the
/// steer's own, and these hang off it.
///
/// Ordered by the Repo's name, which is the order the form's own rows are drawn
/// in — a record read back in the order it is drawn is one the human can hold
/// against the form they filled.
async fn additions(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, Vec<SteerAddition>>> {
    let rows: Vec<(i64, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT a.event_id, r.name, a.mode, a.base_ref, a.branch
         FROM steer_additions a
         JOIN steers s ON s.event_id = a.event_id
         JOIN repos r ON r.id = a.repo_id
         WHERE s.conversation_id = ?
         ORDER BY r.name",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what the steers of Conversation {conversation_id} took in")
    })?;

    let mut added: HashMap<i64, Vec<SteerAddition>> = HashMap::new();

    for (event_id, repo, mode, base_ref, branch) in rows {
        added.entry(event_id).or_default().push(SteerAddition {
            repo,
            // Read rather than guessed past, as every other stored word is: a
            // mode this build does not know is a database written by a
            // Verkstead this one is not.
            mode: CompanionMode::read(&mode)?,
            base_ref,
            branch,
        });
    }

    Ok(added)
}

/// And the ones each opened up, read the same way and in the same order.
async fn upgrades(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, Vec<SteerUpgrade>>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT u.event_id, r.name, u.branch
         FROM steer_upgrades u
         JOIN steers s ON s.event_id = u.event_id
         JOIN repos r ON r.id = u.repo_id
         WHERE s.conversation_id = ?
         ORDER BY r.name",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what the steers of Conversation {conversation_id} opened up")
    })?;

    let mut upgraded: HashMap<i64, Vec<SteerUpgrade>> = HashMap::new();

    for (event_id, repo, branch) in rows {
        upgraded
            .entry(event_id)
            .or_default()
            .push(SteerUpgrade { repo, branch });
    }

    Ok(upgraded)
}
