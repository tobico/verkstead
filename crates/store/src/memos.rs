//! The Memos a building session leaves for the sessions after it on the same
//! Conversation (ADR-0023).
//!
//! **A Memo is an Event**, and its body is the markdown the session wrote, in
//! the `body` column like the Brief and the Handoff — see
//! [`super::Event::Memo`]. What the body does not hold is here, keyed by the
//! Event the way a Steer's record is: who left it, out of which session, and
//! whether it has since been taken back.
//!
//! **Who left it is words**, computed at the moment it is left and kept as
//! text — *the session that worked task 03*, *the finish session*. Words rather
//! than a step, because the record says what ran and a step is a reading of the
//! runner's that may read differently a build from now. The Event id of the
//! session's own output sits beside them, so a card can say which session that
//! was.
//!
//! **A retirement is three columns on the same row**, empty until something
//! retires it: when, by whom — words on the same pattern, or *you* for the
//! human — and why, where a reason was given. A retired Memo stays on the
//! Timeline; it is the prompt it leaves.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// What a Memo's Event carries beside its markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memo {
    /// The words the card and the prompt stamp it with — *the session that
    /// worked task 03*.
    pub left_by: String,

    /// The Event of the session's own output, on the same Timeline.
    pub left_in: i64,

    /// `None` while the Memo still stands.
    pub retired: Option<Retirement>,
}

/// A Memo taken back: when, by whom, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retirement {
    /// RFC 3339.
    pub at: String,

    /// Words on the pattern of [`Memo::left_by`], or *you* for the human.
    pub by: String,

    /// Why, where whoever retired it said.
    pub reason: Option<String>,
}

/// The word the `kind` column holds for a Memo.
///
/// A constant beside the Event's own spelling, for the Pause's reason: a
/// Timeline read has to know the word without an Event in hand to ask.
pub(crate) const MEMO: &str = "memo";

/// The memos table, one row per Memo Event.
///
/// The Conversation is on the row as well as on the Event above it, for the
/// Steer's reason: it is what lets a whole Timeline's worth be read in one
/// query. The three retirement columns are declared from the first row, so the
/// table is the shape it will keep.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS memos (
             event_id        INTEGER PRIMARY KEY REFERENCES timeline_events(id),
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             left_by         TEXT NOT NULL,
             left_in         INTEGER NOT NULL REFERENCES timeline_events(id),
             retired_at      TEXT,
             retired_by      TEXT,
             reason          TEXT
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the memos table")?;

    Ok(())
}

/// Put a Memo on a Conversation's Timeline: the Event with the markdown, and
/// the row saying who left it, in one transaction.
///
/// `None` means there is no such Conversation, by the insert-from-select every
/// other Event is written with. `left_in` is the Event of the session leaving
/// it — which session that is, and whether it may leave one at all, is the
/// caller's to have settled.
pub async fn leave_memo(
    pool: &SqlitePool,
    conversation_id: i64,
    markdown: &str,
    left_by: &str,
    left_in: i64,
) -> Result<Option<i64>> {
    let mut tx = super::writing(pool, "leaving a Memo").await?;

    let event: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO timeline_events (conversation_id, at, kind, body)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?
         FROM conversations WHERE id = ?
         RETURNING id",
    )
    .bind(MEMO)
    .bind(markdown)
    .bind(conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("putting a Memo on the Timeline of {conversation_id}"))?;

    let Some((event_id,)) = event else {
        return Ok(None);
    };

    sqlx::query(
        "INSERT INTO memos (event_id, conversation_id, left_by, left_in) VALUES (?, ?, ?, ?)",
    )
    .bind(event_id)
    .bind(conversation_id)
    .bind(left_by)
    .bind(left_in)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording who left Memo {event_id}"))?;

    tx.commit().await.context("leaving a Memo")?;

    Ok(Some(event_id))
}

/// Every Memo on a Conversation's Timeline, by the Event each one is.
///
/// A read of its own rather than a join, for the arithmetic every side table
/// here is read by: the Timeline's query is at the columns a tuple can be read
/// back as.
pub(crate) async fn on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, Memo>> {
    type Row = (
        i64,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT event_id, left_by, left_in, retired_at, retired_by, reason
         FROM memos WHERE conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the Memos of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(
            |(event_id, left_by, left_in, retired_at, retired_by, reason)| {
                // Retired once both the moment and the who are there: the two are
                // written together, and the reason is optional beside them.
                let retired =
                    retired_at
                        .zip(retired_by)
                        .map(|(at, by)| Retirement { at, by, reason });

                (
                    event_id,
                    Memo {
                        left_by,
                        left_in,
                        retired,
                    },
                )
            },
        )
        .collect())
}

/// A Memo still standing, as the prompt of a later session is told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingMemo {
    /// Its Event, which is the number it is retired by.
    pub id: i64,

    /// See [`Memo::left_by`].
    pub left_by: String,

    /// What the session wrote, as it wrote it.
    pub markdown: String,
}

/// Every Memo on a Conversation that nothing has retired, oldest first: what
/// each session launched on it is told.
///
/// Read off the record at every launch and never marked delivered, unlike a
/// deferred Set's Answers — a Memo is not spent by being read, and stands until
/// something retires it.
pub async fn standing_memos(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<StandingMemo>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT memos.event_id, memos.left_by, timeline_events.body
         FROM memos JOIN timeline_events ON timeline_events.id = memos.event_id
         WHERE memos.conversation_id = ? AND memos.retired_at IS NULL
         ORDER BY memos.event_id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the standing Memos of Conversation {conversation_id}"))?;

    Ok(rows
        .into_iter()
        .map(|(id, left_by, markdown)| StandingMemo {
            id,
            left_by,
            markdown,
        })
        .collect())
}

/// What became of a retirement asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Retiring {
    /// The Memo is retired: it has left every later prompt, and its card says
    /// who took it back.
    Retired,

    /// It was retired already, by the words here. Said rather than taken as
    /// done, because a session retiring a Memo somebody else has is a session
    /// with the wrong one in mind.
    AlreadyRetired { by: String },

    /// No Memo of this Conversation's has that id — another Conversation's, an
    /// Event of some other kind, or nothing at all, which are one answer
    /// because a session can tell none of them apart from where it stands.
    NotThisConversations,
}

/// Take a Memo of `conversation_id`'s back, stamped `by` — the retiring
/// session's words, or *you* — with the reason where one was given.
///
/// Read and written in one transaction, so two retirements racing write one
/// stamp and the other is told it was already retired. The Event is never
/// touched: a retirement is the Memo's row changing, and the record keeps one
/// card per Memo.
pub async fn retire_memo(
    pool: &SqlitePool,
    conversation_id: i64,
    memo_id: i64,
    by: &str,
    reason: Option<&str>,
) -> Result<Retiring> {
    let mut tx = super::writing(pool, "retiring a Memo").await?;

    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT retired_by FROM memos WHERE event_id = ? AND conversation_id = ?")
            .bind(memo_id)
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| format!("reading Memo {memo_id} of Conversation {conversation_id}"))?;

    match row {
        None => return Ok(Retiring::NotThisConversations),
        Some((Some(by),)) => return Ok(Retiring::AlreadyRetired { by }),
        Some((None,)) => {}
    }

    sqlx::query(
        "UPDATE memos SET retired_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), retired_by = ?,
                          reason = ?
         WHERE event_id = ?",
    )
    .bind(by)
    .bind(reason)
    .bind(memo_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("retiring Memo {memo_id} of Conversation {conversation_id}"))?;

    tx.commit().await.context("retiring a Memo")?;

    Ok(Retiring::Retired)
}

/// Put a retired Memo of `conversation_id`'s back into every later prompt:
/// the three retirement columns cleared.
///
/// `false` where no Memo of this Conversation's has that id. One already
/// standing is restored by doing nothing, which is what was asked for.
pub async fn restore_memo(pool: &SqlitePool, conversation_id: i64, memo_id: i64) -> Result<bool> {
    let restored = sqlx::query(
        "UPDATE memos SET retired_at = NULL, retired_by = NULL, reason = NULL
         WHERE event_id = ? AND conversation_id = ?",
    )
    .bind(memo_id)
    .bind(conversation_id)
    .execute(pool)
    .await
    .with_context(|| format!("restoring Memo {memo_id} of Conversation {conversation_id}"))?;

    Ok(restored.rows_affected() > 0)
}

/// One Memo of a Conversation's as the Timeline reads it — when it was left,
/// its markdown, and its row — for a press that answers with the card it
/// changed. `None` where no Memo of this Conversation's has that id.
pub async fn memo_of(
    pool: &SqlitePool,
    conversation_id: i64,
    memo_id: i64,
) -> Result<Option<(String, String, Memo)>> {
    let at: Option<(String, String)> = sqlx::query_as(
        "SELECT timeline_events.at, timeline_events.body
         FROM memos JOIN timeline_events ON timeline_events.id = memos.event_id
         WHERE memos.event_id = ? AND memos.conversation_id = ?",
    )
    .bind(memo_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading Memo {memo_id} of Conversation {conversation_id}"))?;

    let Some((at, markdown)) = at else {
        return Ok(None);
    };

    Ok(on_timeline(pool, conversation_id)
        .await?
        .remove(&memo_id)
        .map(|memo| (at, markdown, memo)))
}
