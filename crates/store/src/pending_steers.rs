//! The pending steer: a steer somebody has started and not yet decided.
//!
//! Pressing **Steer** stops the drive and writes one of these beside the
//! Conversation. The workbench draws it as the last item on the Timeline and
//! the form is its details pane rather than a window over the page, so a steer
//! that takes an afternoon to write is written the way anything long is
//! written — coming and going, on whichever device is to hand, with the rest of
//! the workbench still there to be read. Submitting freezes it into the Steer
//! Event and takes the row away; cancelling takes the row away and leaves the
//! Conversation stopped.
//!
//! **Beside the Conversation rather than on its Timeline.** A Timeline is
//! ordered by row id, so a row written at the press could not be moved to the
//! end at the submit without changing the id the address selects it by — and a
//! seen-out session goes on landing commits under the press, which would leave
//! the Steer and the Moved line it came of pages apart. So the record is
//! written at the submit, as it always was, and this is what stands in the
//! meantime: one per Conversation, by the primary key, and no part of the
//! record at all. Nothing here boards a Share.
//!
//! **One per Conversation, and the second press finds the first.** The press is
//! what opens it and what selects it, and which of the two happened is
//! [`Pending`]'s to say: a Conversation already holding one is a Conversation
//! with a form half written in it, and a second row would be the workbench
//! throwing that away for somebody who pressed twice.
//!
//! **A slot for every field the form has** — where it is going, the brief, the
//! digest tick, the instruction, the follow-up brief, the Pairing, the
//! interrupt tick, and the companion rows the steer would add and open up. All
//! of them empty when the row is written: what fills them is the form saving
//! itself as it is typed. Empty is what the form opens on, and it is what every
//! one of them means — no target picked yet, nothing written, nothing ticked.

use super::{CompanionMode, Lifecycle};
use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::{Sqlite, Transaction};

/// A steer somebody is in the middle of: when they started it, and the form as
/// it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSteer {
    /// When the press was made, RFC 3339 as SQLite stamped it.
    ///
    /// Beside the form rather than in it, because it is the one thing about a
    /// pending steer the human does not write: the press stamps it and a save
    /// leaves it exactly where it was.
    pub at: String,

    /// And the form, as the last save left it.
    pub form: PendingForm,
}

/// The Steer form, in the shape the row keeps it: every field the pane draws,
/// with empty meaning what empty means on the pane.
///
/// One value rather than a parameter list, because a save carries the whole
/// form — a row holding the target of one keystroke and the instruction of
/// another would be a form that was never on anybody's screen.
///
/// [`Default`] is the form the press opens: no target picked, nothing written,
/// nothing ticked and no companion row touched.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingForm {
    /// Where the human has said the work goes, or `None` while they have not
    /// said.
    pub target: Option<Lifecycle>,

    /// The new round's Brief, for a steer into Grilling.
    pub brief: Option<String>,

    /// And whether the session that round starts is primed with everything
    /// already answered.
    pub digest: bool,

    /// The hand-written work, for a steer into Implementing.
    pub instruction: Option<String>,

    /// And the brief, for a steer into Follow-up.
    pub follow_up: Option<String>,

    /// What the work would run under from here, where the human has picked
    /// something.
    pub pairing: Option<PendingPairing>,

    /// And whether the session running now is to be ended where it stands.
    pub interrupt: bool,

    /// The companions the steer would put into the sandbox, one entry per row
    /// ticked.
    pub added: Vec<PendingAddition>,

    /// And the ones already there it would open up, one entry per row ticked
    /// up.
    pub upgraded: Vec<PendingUpgrade>,
}

/// The Pairing a pending steer is carrying: both halves of the pick.
///
/// One value rather than two slots that could disagree, because a Pairing is
/// picked whole — a Profile with no model is not something the picker can be
/// left on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPairing {
    pub profile_id: i64,
    pub model: String,
}

/// One companion row of the form: a registered Repo the steer would put into
/// the sandbox, with what the row says about it.
///
/// The four facts a setup row settles, in the shape [`super::Joining`] carries
/// them to the submit — this is the same row, held while it is being filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingAddition {
    pub repo_id: i64,
    pub mode: CompanionMode,
    /// What its checkout comes off, or `None` for the rule: that repository's
    /// default branch as origin holds it.
    pub base_ref: Option<String>,
    /// What a read-write one's branch is called, empty being *mirroring* — the
    /// Conversation's own branch name.
    pub branch: String,
}

/// And one row of the set already there that the steer would open up.
///
/// One field beside the Repo, for [`super::Opening`]'s reason: there is one
/// direction, and what the new branch comes off is the base already on the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUpgrade {
    pub repo_id: i64,
    pub branch: String,
}

/// What a press on Steer found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    /// There was none, and there is one now.
    Opened,

    /// There was one already, and it is left exactly as it stands. A second
    /// press selects the form somebody is part-way through rather than starting
    /// it again.
    Standing,

    /// No Conversation of that id, so there is nothing to steer.
    NoSuchConversation,
}

/// The tables a pending steer lives in: the row, and the two lists of companion
/// rows hanging off it.
///
/// Two tables beside the row rather than columns on it, for the reason the
/// companions of a Conversation are a table rather than a column: a form
/// holding three added repos holds three rows' worth of answers, and a column
/// would be a list encoded into a string nothing could read back a row at a
/// time.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pending_steers (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             opened_at       TEXT NOT NULL,
             target          TEXT,
             brief           TEXT,
             digest          INTEGER NOT NULL DEFAULT 0,
             instruction     TEXT,
             follow_up       TEXT,
             profile_id      INTEGER REFERENCES profiles(id),
             model           TEXT,
             interrupt       INTEGER NOT NULL DEFAULT 0
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pending steers table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pending_steer_additions (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             mode            TEXT NOT NULL,
             base_ref        TEXT,
             branch          TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pending steer additions table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pending_steer_upgrades (
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             repo_id         INTEGER NOT NULL REFERENCES repos(id),
             branch          TEXT NOT NULL,
             PRIMARY KEY (conversation_id, repo_id)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the pending steer upgrades table")?;

    Ok(())
}

/// Open a pending steer beside a Conversation, where none is there yet.
///
/// The press, said to the record: `INSERT … ON CONFLICT DO NOTHING` selecting
/// from `conversations`, so one statement answers both of the questions a press
/// asks — whether there is a Conversation to steer at all, and whether somebody
/// has already started steering it. Two tabs pressing at once make one row
/// between them, which is the same answer as one human pressing twice.
///
/// Every slot opens empty. What fills them is the form saving itself as it is
/// typed, and empty is what it opens on.
pub async fn open_pending_steer(pool: &SqlitePool, conversation_id: i64) -> Result<Pending> {
    let landed = sqlx::query(
        "INSERT INTO pending_steers (conversation_id, opened_at)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM conversations WHERE id = ?
         ON CONFLICT (conversation_id) DO NOTHING",
    )
    .bind(conversation_id)
    .execute(pool)
    .await
    .with_context(|| format!("opening a pending steer on Conversation {conversation_id}"))?
    .rows_affected();

    if landed > 0 {
        return Ok(Pending::Opened);
    }

    // Nothing was written, which is two different things: a Conversation
    // already holding one, and no Conversation at all. The row itself is what
    // tells them apart.
    let standing: Option<(i64,)> =
        sqlx::query_as("SELECT conversation_id FROM pending_steers WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("looking for the pending steer of Conversation {conversation_id}")
            })?;

    Ok(if standing.is_some() {
        Pending::Standing
    } else {
        Pending::NoSuchConversation
    })
}
/// The pending steer beside a Conversation, where there is one.
///
/// `None` is the ordinary Conversation: nobody is steering it, and there is no
/// item at the end of its Timeline.
pub async fn pending_steer(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Option<PendingSteer>> {
    type Row = (
        String,
        Option<String>,
        Option<String>,
        i64,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        i64,
    );

    let Some(row): Option<Row> = sqlx::query_as(
        "SELECT opened_at, target, brief, digest, instruction, follow_up, profile_id, model,
                interrupt
         FROM pending_steers WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading the pending steer of Conversation {conversation_id}"))?
    else {
        return Ok(None);
    };

    let (at, target, brief, digest, instruction, follow_up, profile_id, model, interrupt) = row;

    // Read rather than guessed past, as every other stored state word is: a
    // target this build does not understand is a database written by a
    // Verkstead this one is not.
    let target = target.as_deref().map(Lifecycle::read).transpose()?;

    let added: Vec<(i64, String, Option<String>, String)> = sqlx::query_as(
        "SELECT repo_id, mode, base_ref, branch
         FROM pending_steer_additions WHERE conversation_id = ? ORDER BY repo_id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what a pending steer of Conversation {conversation_id} would add")
    })?;

    let upgraded: Vec<(i64, String)> = sqlx::query_as(
        "SELECT repo_id, branch
         FROM pending_steer_upgrades WHERE conversation_id = ? ORDER BY repo_id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading what a pending steer of Conversation {conversation_id} would open up")
    })?;

    Ok(Some(PendingSteer {
        at,
        form: PendingForm {
            target,
            brief,
            digest: digest != 0,
            instruction,
            follow_up,
            // Both halves or neither: a Pairing is picked whole, so a row
            // holding one of them is a row nothing picked.
            pairing: profile_id
                .zip(model)
                .map(|(profile_id, model)| PendingPairing { profile_id, model }),
            interrupt: interrupt != 0,
            added: added
                .into_iter()
                .map(|(repo_id, mode, base_ref, branch)| {
                    Ok(PendingAddition {
                        repo_id,
                        mode: CompanionMode::read(&mode)?,
                        base_ref,
                        branch,
                    })
                })
                .collect::<Result<_>>()?,
            upgraded: upgraded
                .into_iter()
                .map(|(repo_id, branch)| PendingUpgrade { repo_id, branch })
                .collect(),
        },
    }))
}

/// Write the form onto the pending steer, whole.
///
/// The whole form in one act, which is what keeps the row a thing somebody
/// could have been looking at: the page saves what is on the screen rather than
/// the field that moved, so the row is never the target of one keystroke beside
/// the instruction of another. The companion rows go the same way — cleared and
/// written again — because a row unticked is a row that has to leave, and there
/// is no second call to notice it went.
///
/// Answers whether there was a pending steer to write onto. `false` is a save
/// landing behind a submit or a cancel from another device, which is the page
/// arguing with a record that has moved on rather than a failure.
///
/// `opened_at` is not touched: when the press was made is the one thing on the
/// row that is not the form.
pub async fn save_pending_steer(
    pool: &SqlitePool,
    conversation_id: i64,
    form: &PendingForm,
) -> Result<bool> {
    let mut tx = super::writing(pool, "saving a pending steer").await?;

    let written = sqlx::query(
        "UPDATE pending_steers
         SET target = ?, brief = ?, digest = ?, instruction = ?, follow_up = ?,
             profile_id = ?, model = ?, interrupt = ?
         WHERE conversation_id = ?",
    )
    .bind(form.target.map(Lifecycle::stored))
    .bind(form.brief.as_deref())
    .bind(i64::from(form.digest))
    .bind(form.instruction.as_deref())
    .bind(form.follow_up.as_deref())
    .bind(form.pairing.as_ref().map(|pairing| pairing.profile_id))
    .bind(form.pairing.as_ref().map(|pairing| pairing.model.as_str()))
    .bind(i64::from(form.interrupt))
    .bind(conversation_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("saving the pending steer of Conversation {conversation_id}"))?
    .rows_affected();

    if written == 0 {
        return Ok(false);
    }

    for table in ["pending_steer_additions", "pending_steer_upgrades"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE conversation_id = ?"))
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "clearing what the pending steer of Conversation {conversation_id} asked for"
                )
            })?;
    }

    for addition in &form.added {
        sqlx::query(
            "INSERT INTO pending_steer_additions
                 (conversation_id, repo_id, mode, base_ref, branch)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(conversation_id)
        .bind(addition.repo_id)
        .bind(addition.mode.stored())
        .bind(addition.base_ref.as_deref())
        .bind(addition.branch.as_str())
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("saving what a pending steer of Conversation {conversation_id} would add")
        })?;
    }

    for upgrade in &form.upgraded {
        sqlx::query(
            "INSERT INTO pending_steer_upgrades (conversation_id, repo_id, branch)
             VALUES (?, ?, ?)",
        )
        .bind(conversation_id)
        .bind(upgrade.repo_id)
        .bind(upgrade.branch.as_str())
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("saving what a pending steer of Conversation {conversation_id} would open up")
        })?;
    }

    tx.commit().await.context("saving a pending steer")?;

    Ok(true)
}

/// Take the pending steer away: what Cancel presses, and what a submit does in
/// the transaction that writes the record.
///
/// Answers whether there was one. Nothing to discard is an ordinary outcome
/// rather than a failure — a cancel from one device landing behind a submit
/// from another is exactly that.
pub async fn discard_pending_steer(pool: &SqlitePool, conversation_id: i64) -> Result<bool> {
    let mut tx = super::writing(pool, "discarding a pending steer").await?;
    let discarded = discard(&mut tx, conversation_id).await?;
    tx.commit().await.context("discarding a pending steer")?;

    Ok(discarded)
}

/// The same, inside somebody else's transaction — the submit's, which takes the
/// row away in the act that writes the record it became.
///
/// The two lists first and the row itself last, because the row is what says
/// there was a pending steer at all: the lists hang off it and stand empty on
/// most forms.
pub(crate) async fn discard(
    tx: &mut Transaction<'static, Sqlite>,
    conversation_id: i64,
) -> Result<bool> {
    for table in ["pending_steer_additions", "pending_steer_upgrades"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE conversation_id = ?"))
            .bind(conversation_id)
            .execute(&mut **tx)
            .await
            .with_context(|| {
                format!(
                    "discarding what the pending steer of Conversation {conversation_id} asked for"
                )
            })?;
    }

    let gone = sqlx::query("DELETE FROM pending_steers WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("discarding the pending steer of Conversation {conversation_id}"))?
        .rows_affected();

    Ok(gone > 0)
}

/// A pending steer said as SQL about a Conversation row aliased `c`, for the
/// fold of what waits on the human — see [`super::conversations`], its one
/// reader.
///
/// A form somebody opened and has not decided is a Conversation waiting on
/// them, and it is the plainest of the sources: the press stopped the drive, so
/// nothing is going to move until they submit or cancel. The stop it made is
/// their own and says nothing in the marks — see [`super::Decision::waits_on_the_human`]
/// — so without this a Conversation with a half-written steer on it would read
/// as quiet from the sidebar, which is the one place somebody who left the form
/// yesterday is going to look for it.
///
/// The row's existence and nothing about its contents. A form with nothing
/// written in it is as much somebody's to finish as one with an afternoon in
/// it: what waits on them is the decision rather than the typing.
pub(crate) fn waited_on() -> &'static str {
    "EXISTS (SELECT 1 FROM pending_steers p WHERE p.conversation_id = c.id)"
}
