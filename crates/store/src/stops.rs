//! Where driving stopped: when it stopped, whether anybody decided it, the
//! Notice that says what happened, and the words a usage-window stop shows
//! about when the account comes back.
//!
//! A Conversation is driven or it is **Stopped**, and there is one record of
//! which. However the run stopped — a session that fell over, a driver a crash
//! took away, the human's press, an account out of window — what is written is
//! the same four facts, so everything that reads it asks one question about one
//! thing.
//!
//! The state lives in columns on the Conversation itself rather than in a table
//! beside it, which is what makes *at most one stop per Conversation* a fact
//! about the record rather than a rule something has to keep. A Conversation
//! that is stopped is stopped once: a second stop raised against one already
//! stopped is the same stop noticed twice — a sweep looking again, or two
//! watchers finding the same dead session — and the first Notice is the one that
//! explains it.
//!
//! What *happened* is the Notice, which is an ordinary Timeline Event and stays
//! on the record for ever. What is kept here is the one fact the Notice cannot
//! carry: that the Conversation is stopped *now*, which is what the *blocked on
//! you* badge is drawn from and what says whether anything ought to be driving
//! it.
//!
//! Cleared when driving starts again — see [`clear_stop`], which is what Resume
//! presses. Nothing here starts anything: what a stop *means* is the server's,
//! and what this holds is only that there is one.
//!
//! Beside it, the stop that has not landed yet: the human pressed **Stop** while
//! a session was still running, so the run stops once that session has reached
//! its own end rather than now — see [`ask_to_stop`]. On the Conversation for
//! the reason the stop itself is, and durable for the same one. A Conversation
//! the human asked to stop is one that stays stopped, and a server restarted in
//! the gap that read nothing here would take it up again as though nobody had
//! asked.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

use super::conversations::Event;

/// Who stopped it.
///
/// Two things follow from the word, and they are not the same question. A
/// restart asks *did anybody decide this?* — what Verkstead pulled the brake on
/// and what the human pressed both stay stopped until somebody says otherwise,
/// and what a crash took away is a Conversation nobody decided anything about,
/// so starting it again is putting things back rather than overriding a
/// decision. The waiting marks ask something narrower: *is this stop the
/// human's to look into?* A stop they made themselves is not — they were there
/// — so the sidebar disc and the *blocked on you* badge are drawn only for the
/// stops that came from outside them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Verkstead pulled the brake: a session that fell over, checks that would
    /// not go green, a finish step with no pull request, an account out of
    /// window.
    Verkstead,

    /// The human pressed Stop or Force stop.
    Human,

    /// A stop written before the two above were told apart, when both were
    /// stored as one word.
    ///
    /// Nothing can say which of them it was, so it is read as the human's: the
    /// marks are worth something only while they are rare, and a badge nobody
    /// can explain is worse than a stop somebody has to go and find.
    Deliberate,

    /// Nothing chose anything: a restart or a crash took the driver away.
    Circumstance,
}

impl Decision {
    /// The word the column holds. Lowercase and spelled out, so a database
    /// opened by hand says something.
    pub(crate) fn stored(self) -> &'static str {
        match self {
            Self::Verkstead => "verkstead",
            Self::Human => "human",
            Self::Deliberate => "deliberate",
            Self::Circumstance => "circumstance",
        }
    }

    /// The one a stored word names. An unknown word is a database written by a
    /// Verkstead this one does not understand, exactly as an unknown lifecycle
    /// state is.
    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "verkstead" => Self::Verkstead,
            "human" => Self::Human,
            "deliberate" => Self::Deliberate,
            "circumstance" => Self::Circumstance,
            other => bail!("a Conversation is stopped for the unknown reason {other:?}"),
        })
    }

    /// Whether anybody decided it, which is the one thing a restart asks: what
    /// somebody chose waits for a press, and what nobody chose is taken up
    /// unasked.
    #[must_use]
    pub fn decided(self) -> bool {
        !matches!(self, Self::Circumstance)
    }

    /// Whether the stop is one to draw the waiting marks for — the sidebar disc
    /// and the *blocked on you* badge.
    ///
    /// A stop the human made themselves is not. It still waits for their press
    /// like every other, but they pressed it: a mark saying *look here* would
    /// be Verkstead telling them their own news, and what makes the marks worth
    /// looking at is that they appear only where something happened without
    /// them.
    #[must_use]
    pub fn waits_on_the_human(self) -> bool {
        matches!(self, Self::Verkstead | Self::Circumstance)
    }
}

/// The same rule as [`Decision::waits_on_the_human`], said as SQL about a
/// Conversation row aliased `c`.
///
/// Here rather than in the sidebar's own query — see [`super::conversations`],
/// its one reader — because the words are this module's. Built from
/// [`Decision::stored`], so a query cannot go on asking for a word the writing
/// half has stopped using.
pub(crate) fn waited_on() -> String {
    format!(
        "c.stopped_at IS NOT NULL AND c.stopped_by IN ('{}', '{}')",
        Decision::Verkstead.stored(),
        Decision::Circumstance.stored(),
    )
}

/// A stop, whole: what kind of stop it is, when it happened, which Event
/// explains it, and what it has to show about a window coming back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stopped {
    pub decision: Decision,

    /// The Notice saying what stopped, why, and what the evidence was. Where
    /// the *blocked on you* badge points, a badge with nowhere to go being no
    /// use to anybody.
    pub notice: i64,

    /// When it stopped, RFC 3339.
    pub at: String,

    /// When the account that ran out comes back, where the stop carries
    /// anything at all — and `None` on every stop that is not a usage window's,
    /// which is nearly all of them.
    ///
    /// Words beside the Resume button rather than a moment anything acts on:
    /// what a stopped run waits for is a press. Kept as text for the reason the
    /// line that said it is kept — the wording is the backend's and will move,
    /// and somebody reading a stop a week later is reading what the session
    /// printed rather than this build's opinion of it.
    pub resets: Option<String>,
}

/// The columns a stop lives in, added to `conversations` rather than declared
/// with it.
///
/// Always through `ALTER TABLE`, so that a database made this morning and one
/// written a year ago take the same path and end with the same shape — the
/// table is created by [`super::conversations`] and this is the one place that
/// knows what a stop is.
///
/// **The columns arrive holding what they replace.** A stopped Conversation used
/// to be a row in one table beside it or an unresumed row in another, and the
/// moment the columns are added is the one moment it is known that nothing has
/// been copied yet — so the copying happens here rather than in
/// [`super::migrations`], which has no such moment to hang it on. Neither table
/// is touched: they are the record of what happened, and an old Pause Event
/// stays on its Timeline exactly as it was written.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    let mut added = false;

    for (column, kind) in [
        ("stopped_at", "TEXT"),
        ("stopped_by", "TEXT"),
        ("stopped_notice", "INTEGER REFERENCES timeline_events(id)"),
        ("stopped_resets", "TEXT"),
        ("stop_asked_at", "TEXT"),
    ] {
        let there: Option<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('conversations') WHERE name = ?")
                .bind(column)
                .fetch_optional(pool)
                .await
                .with_context(|| format!("looking for the {column} column of a Conversation"))?;

        if there.is_some() {
            continue;
        }

        sqlx::query(&format!(
            "ALTER TABLE conversations ADD COLUMN {column} {kind}"
        ))
        .execute(pool)
        .await
        .with_context(|| format!("adding the {column} column to the Conversations"))?;

        added = true;
    }

    if added {
        carried_over(pool).await?;
    }

    Ok(())
}

/// The two tables a Verkstead of before kept a stopped Conversation in, named
/// here and nowhere else: both are read once and never written, and nothing
/// else in this build knows either word.
const HALTS: &str = "halts";
const ASKED: &str = "stops_asked";

/// Read the stops a Verkstead of before kept beside the Conversations onto the
/// Conversations themselves.
///
/// Every open row of [`HALTS`] and every unresumed Pause, and the Stop asked for
/// that never landed. Nothing is rewritten and nothing is dropped: what those
/// tables hold is what happened, and this is the same thing said where the one
/// stop is now read from.
///
/// [`HALTS`] first and a Pause only where that left nothing, because that is the
/// order the two stopped a run in: a Conversation that had both was one nothing
/// could launch for either reason, and the first of them is the one that named
/// its own Notice.
///
/// A halt carries its own word across untouched — it was the same column then,
/// and a word nothing here can improve on is one to leave alone. A Pause is
/// written as [`Decision::Verkstead`], because there is nothing to guess about
/// it: an open Pause is an account out of window, which is Verkstead pulling the
/// brake, and it is exactly the kind of stop the human has to be told about.
///
/// Any of the tables may be missing altogether — a database made after this
/// stage has none of them — which is nothing to do rather than a failure.
async fn carried_over(pool: &SqlitePool) -> Result<()> {
    if there(pool, HALTS).await? {
        sqlx::query(&format!(
            "UPDATE conversations
                SET stopped_at     = (SELECT h.at FROM {HALTS} h
                                       WHERE h.conversation_id = conversations.id),
                    stopped_by     = (SELECT h.halt FROM {HALTS} h
                                       WHERE h.conversation_id = conversations.id),
                    stopped_notice = (SELECT h.event_id FROM {HALTS} h
                                       WHERE h.conversation_id = conversations.id)
              WHERE EXISTS (SELECT 1 FROM {HALTS} h
                             WHERE h.conversation_id = conversations.id)"
        ))
        .execute(pool)
        .await
        .context("reading the halts of before as stops")?;
    }

    if there(pool, "pauses").await? {
        // The Event's own time rather than this morning's: the run stopped
        // whenever it stopped, and a stop stamped with the upgrade would say the
        // work stopped when the server was restarted.
        sqlx::query(&format!(
            "UPDATE conversations
                SET stopped_at     = (SELECT e.at FROM pauses p
                                        JOIN timeline_events e ON e.id = p.event_id
                                       WHERE p.conversation_id = conversations.id AND p.resumed_at IS NULL),
                    stopped_by     = '{verkstead}',
                    stopped_notice = (SELECT p.event_id FROM pauses p
                                       WHERE p.conversation_id = conversations.id AND p.resumed_at IS NULL),
                    stopped_resets = (SELECT p.resets_at FROM pauses p
                                       WHERE p.conversation_id = conversations.id AND p.resumed_at IS NULL)
              WHERE stopped_at IS NULL
                AND EXISTS (SELECT 1 FROM pauses p
                             WHERE p.conversation_id = conversations.id AND p.resumed_at IS NULL)",
            verkstead = Decision::Verkstead.stored(),
        ))
        .execute(pool)
        .await
        .context("reading the open Pauses of before as stops")?;
    }

    if there(pool, ASKED).await? {
        sqlx::query(&format!(
            "UPDATE conversations
                SET stop_asked_at = (SELECT s.at FROM {ASKED} s
                                      WHERE s.conversation_id = conversations.id)
              WHERE EXISTS (SELECT 1 FROM {ASKED} s
                             WHERE s.conversation_id = conversations.id)"
        ))
        .execute(pool)
        .await
        .context("reading the stops asked for before as stops asked for")?;
    }

    Ok(())
}

/// Whether this database has a table by that name at all.
async fn there(pool: &SqlitePool, table: &str) -> Result<bool> {
    let found: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("looking for the {table} table"))?;

    Ok(found.is_some())
}

/// Stop driving a Conversation, and put the Notice saying why on its Timeline.
///
/// `resets` is what the stop has to show about an account coming back, which is
/// a usage window's stop and nothing else's.
///
/// The Event the Notice became, or `None` where nothing was written: a
/// Conversation already stopped, or one that is not there. Neither is a failure.
/// A run stops once, so a second stop against a stopped Conversation is the
/// same stop arriving twice — the sweep looking again a minute later is exactly
/// that, and so is a display redrawing its out-of-window banner twice a second —
/// and the first Notice is the one the human reads.
///
/// One transaction, because a stop without its Notice is a badge pointing at
/// nothing, and a Notice without its stop is a Conversation that says it
/// stopped and does not know it.
pub async fn stop(
    pool: &SqlitePool,
    conversation_id: i64,
    decision: Decision,
    markdown: &str,
    resets: Option<&str>,
) -> Result<Option<i64>> {
    match write_stop(
        pool,
        conversation_id,
        decision,
        markdown,
        resets,
        Standing::Whatever,
    )
    .await?
    {
        Stopping::Stopped(notice) => Ok(Some(notice)),
        Stopping::Already | Stopping::Withdrawn => Ok(None),
    }
}

/// The same, for the stop the human asked for while a session was still
/// running — and only for as long as they are still asking for it.
///
/// The request is read, and forgotten, inside the transaction that writes the
/// stop. What makes that the difference between a fix and a comment is the act
/// on the other side of the race: a Steer and a Resume each take the request
/// back on their way past — see [`forget_stop`] — and what they are taking back
/// is a stop nothing has written yet. Read outside the write, a run landing one
/// writes it on the far side of the steer that withdrew it: the Conversation
/// has moved, a fresh run is starting, and a stop nobody is asking for any more
/// stops that run before it has launched anything.
///
/// So the request is the condition rather than the cue. Withdrawn first and
/// nothing is written; written first and the steer clears the stop it finds,
/// which is what it was always going to do. Either order leaves the human with
/// the one they pressed.
pub async fn stop_as_asked(
    pool: &SqlitePool,
    conversation_id: i64,
    decision: Decision,
    markdown: &str,
    resets: Option<&str>,
) -> Result<Stopping> {
    write_stop(
        pool,
        conversation_id,
        decision,
        markdown,
        resets,
        Standing::Asked,
    )
    .await
}

/// What landing a stop came to — see [`stop_as_asked`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopping {
    /// Written, and this is the Notice it put on the Timeline.
    Stopped(i64),

    /// Nothing written, and nothing wrong: the Conversation had stopped
    /// already, or it is not there at all.
    Already,

    /// Nothing written because nobody is asking any more: the request this
    /// stop stands on was taken back before the write got to it.
    Withdrawn,
}

/// What a stop being written stands on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// Whatever the run had to say. The ordinary stop: a session that fell
    /// over, an account out of window, the human's own press.
    Whatever,

    /// The request the human left behind them — see [`stop_as_asked`].
    Asked,
}

/// Both of the above, and the transaction they share.
async fn write_stop(
    pool: &SqlitePool,
    conversation_id: i64,
    decision: Decision,
    markdown: &str,
    resets: Option<&str>,
    standing: Standing,
) -> Result<Stopping> {
    let mut tx = super::writing(pool, "stopping a Conversation").await?;

    // Asked inside the transaction, so the answer still holds when the write
    // below acts on it. Both columns for the one reason: a stop standing on the
    // human's request is a stop the request has to still be there for.
    let already: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT stopped_at, stop_asked_at FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!("asking whether Conversation {conversation_id} had stopped")
            })?;

    // A Conversation that is not there has nobody left to tell, and one that has
    // stopped has been told already.
    let asked_for = match already {
        None => return Ok(Stopping::Already),
        Some((Some(_), _)) => return Ok(Stopping::Already),
        Some((None, asked_for)) => asked_for,
    };

    if standing == Standing::Asked && asked_for.is_none() {
        return Ok(Stopping::Withdrawn);
    }

    let event = Event::Notice(markdown.to_owned());

    // Selected from `conversations` rather than trusting the id, as every other
    // Event is written: SQLite enforces a foreign key only when asked to, and a
    // stop attributed to a Conversation that is not there would be one nobody
    // could ever start again.
    let written: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?
         FROM conversations WHERE id = ?
         RETURNING id",
    )
    .bind(event.kind())
    .bind(markdown)
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!("putting the Notice of a stop on the Timeline of Conversation {conversation_id}")
    })?;

    let Some((notice,)) = written else {
        return Ok(Stopping::Already);
    };

    sqlx::query(
        "UPDATE conversations
            SET stopped_at     = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                stopped_by     = ?,
                stopped_notice = ?,
                stopped_resets = ?
          WHERE id = ?",
    )
    .bind(decision.stored())
    .bind(notice)
    .bind(resets)
    .bind(conversation_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording that Conversation {conversation_id} has stopped"))?;

    // And the request goes with the stop it became, in the transaction that
    // wrote it: one left behind would be read as a stop still to come and land
    // all over again at the next launch.
    if standing == Standing::Asked {
        sqlx::query("UPDATE conversations SET stop_asked_at = NULL WHERE id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("forgetting the stop Conversation {conversation_id} was asked to make")
            })?;
    }

    tx.commit().await.context("stopping a Conversation")?;

    Ok(Stopping::Stopped(notice))
}

/// Whether a Conversation is stopped, and what the stop is.
///
/// `None` is a Conversation nothing has stopped — which is every one that is
/// being driven, and every one nothing is supposed to be driving.
pub async fn stopped(pool: &SqlitePool, conversation_id: i64) -> Result<Option<Stopped>> {
    type Row = (Option<String>, Option<String>, Option<i64>, Option<String>);

    let row: Option<Row> = sqlx::query_as(
        "SELECT stopped_at, stopped_by, stopped_notice, stopped_resets
         FROM conversations WHERE id = ?",
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading whether Conversation {conversation_id} is stopped"))?;

    let Some((Some(at), by, notice, resets)) = row else {
        return Ok(None);
    };

    // The four are written in one statement, so a row with a time on it and no
    // Notice beside it is a database somebody has been in by hand.
    let (Some(by), Some(notice)) = (by, notice) else {
        bail!("Conversation {conversation_id} says it stopped and does not say what stopped it");
    };

    Ok(Some(Stopped {
        decision: Decision::read(&by)?,
        notice,
        at,
        resets,
    }))
}

/// Ask for the run to stop once whatever is running now has reached its end.
///
/// What **Stop** records where a session is still going. Nothing is ended and
/// nothing is put on the Timeline: the stop and its Notice come later, as the
/// run is about to launch the next thing — see the server's `stops` module.
///
/// Nothing happens twice. A second press is the first one arriving again, and
/// the Conversation is stopping either way — which is what the `IS NULL` keeps:
/// the first press is the one the record remembers.
pub async fn ask_to_stop(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE conversations
            SET stop_asked_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE id = ? AND stop_asked_at IS NULL",
    )
    .bind(conversation_id)
    .execute(pool)
    .await
    .with_context(|| format!("asking Conversation {conversation_id} to stop"))?;

    Ok(())
}

/// Whether the human has asked this Conversation to stop and it has not stopped
/// yet.
///
/// Asked in front of every launch a run makes, which is where a stop asked for
/// becomes a stop.
pub async fn asked_to_stop(pool: &SqlitePool, conversation_id: i64) -> Result<bool> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT stop_asked_at FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| {
                format!("reading whether Conversation {conversation_id} was asked to stop")
            })?;

    Ok(matches!(row, Some((Some(_),))))
}

/// Take an asked-for stop away: it has landed, or Resume has overtaken it.
///
/// Nothing to do where none was asked for, which is every Conversation nobody
/// has pressed Stop on.
pub async fn forget_stop(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query("UPDATE conversations SET stop_asked_at = NULL WHERE id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await
        .with_context(|| {
            format!("forgetting the stop asked for on Conversation {conversation_id}")
        })?;

    Ok(())
}

/// Take the stop away, which is what starting to drive again does.
///
/// The Notice stays where it is: it is a record of a stop that really happened,
/// and a Timeline that took yesterday's back would be one nobody could read.
/// What goes is only the state — the badge, the reset words beside the button,
/// and the reason a restart would leave the Conversation alone.
///
/// Nothing to do where there was no stop, which is the ordinary case for a
/// Conversation being driven perfectly well.
pub async fn clear_stop(pool: &SqlitePool, conversation_id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE conversations
            SET stopped_at = NULL, stopped_by = NULL,
                stopped_notice = NULL, stopped_resets = NULL
          WHERE id = ?",
    )
    .bind(conversation_id)
    .execute(pool)
    .await
    .with_context(|| format!("starting to drive Conversation {conversation_id} again"))?;

    Ok(())
}
