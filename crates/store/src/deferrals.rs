//! Stored asks: which Sets were stored rather than waited on, which of those a
//! session is idling on, and which of their Answers a session has already been
//! told about.
//!
//! A table of its own, for the reason locking has one: `question_sets` is
//! STRICT, there is no migration machinery here, and a fact learned about a Set
//! after that table was written hangs off it rather than becoming a column on
//! it.
//!
//! Beside the stored body rather than in it, which is the choice worth saying
//! out loud. The body is *what was asked* — the agent's own words, kept as they
//! were written — and how it was asked is neither the agent's wording nor
//! anything the human reads. Two things fall out of keeping it here: SQL can ask
//! the question, which is what lets a quiet session be reaped while an ask of
//! its own is still open; and a Set whose body this build can no longer read
//! still says which kind of ask it was.
//!
//! One row per stored ask and none for a blocking one, so the row being there is
//! the whole of *this was not waited on*. What the row holds is the other two
//! halves: whether a session is idling on it — see [`Ask`], where the third
//! state that column carries is set out — and when its Answers went into a
//! session's prompt, or nothing while they have not, which is [`unfolded`] and
//! is what makes the folding a record rather than something recomputed from what
//! happens to be answered.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::{SqliteConnection, SqlitePool};
use verkstead_schema::Response;

/// Which of the three ways a Set is being asked — see [`super::ask`], which is
/// where one is stored whichever it is.
///
/// A Blocking Ask idles the session that sent it until the Response arrives; a
/// Deferred Ask does not, and its Answers reach a later session of the same
/// Conversation instead. Both land on the Timeline, both leave the Conversation
/// *blocked on you*, and both notify: what differs is who is waiting.
///
/// Between them is the store-and-nudge ask, which is those two halves the other
/// way round (ADR-0011). It is *stored* as a Deferred Ask is, because the
/// backend that sent it cannot hold a shell command open for hours — and a
/// session is idling on it all the same, its turn ended, waiting for the line
/// Verkstead types into its terminal when the Response lands. Which channel a
/// Set was asked on is a fact about the backend rather than about the Set: the
/// CLI asks the same way everywhere, and the server reads the agent type of the
/// session that asked — see [`super::Channel`].
///
/// So the two questions asked of one of these are asked apart, because the
/// answers no longer move together: [`Ask::idled`], which is whether anybody is
/// waiting on the Response, and [`Ask::deferred_shaped`], which is whether it
/// was stored rather than waited on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// The session is idling on the Response, with `verkstead ask` still open.
    Blocking,

    /// The Set is stored and the session's turn is over, and it is idling on the
    /// nudge that says the Answers are there to fetch.
    StoreAndNudge,

    /// Nothing is waiting on it.
    Deferred,
}

impl Ask {
    /// Whether a session is idling on the Answer.
    ///
    /// The question every reader that decides a session's fate is really asking:
    /// the quiet grace, Rescue, a wrap-up's proposals and the locking of what a
    /// gone session left open all turn on whether there is somebody behind the
    /// question. A Deferred Ask alone has nobody.
    pub fn idled(self) -> bool {
        !matches!(self, Self::Deferred)
    }

    /// Whether the Set was stored rather than waited on, which is the whole of
    /// what the Timeline and the badges draw.
    ///
    /// Both stored kinds read alike here on purpose: nothing is holding a
    /// connection open on either, so ageing one against the clock would report
    /// an agent that had gone where none was ever there. What differs between
    /// them is entirely underneath.
    pub fn deferred_shaped(self) -> bool {
        !matches!(self, Self::Blocking)
    }
}

/// An answered stored ask no session has been told about yet: the Set, and what
/// the human said to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unfolded {
    pub set_id: i64,

    /// What was asked — or the stored body where this build can no longer read
    /// it, which is passed over rather than folded. See [`super::Asked`].
    pub set: super::Asked,

    pub response: Response,
}

pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS deferrals (
             set_id    INTEGER PRIMARY KEY REFERENCES question_sets(id),
             folded_at TEXT,
             idled     INTEGER NOT NULL DEFAULT 0
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the deferrals table")?;

    Ok(())
}

/// Record how a stored ask was asked, in the transaction that is storing it.
///
/// `idled` is the whole of what the row says beyond its own existence: a
/// store-and-nudge ask has a session idling on it, a Deferred Ask has nobody.
///
/// Taken as a connection rather than a pool because that is the only way it is
/// ever written: a record written outside the transaction that stored the Set
/// would be a moment where a stored ask read as a blocking one, and what reads
/// it in that moment is the driver deciding whether a quiet session is still
/// asking.
pub(crate) async fn defer(conn: &mut SqliteConnection, set_id: i64, idled: bool) -> Result<()> {
    sqlx::query("INSERT INTO deferrals (set_id, idled) VALUES (?, ?)")
        .bind(set_id)
        .bind(idled)
        .execute(conn)
        .await
        .with_context(|| format!("recording how Question Set {set_id} was asked"))?;

    Ok(())
}

/// How each of a Conversation's stored-ask Sets was asked, by Set.
///
/// A read of its own rather than a join in [`super::timeline`], and the reason is
/// arithmetic rather than judgement: that query is already at the sixteen columns
/// a tuple can be read back as. Cheap on its own — one indexed column, and most
/// Conversations have no stored ask at all.
///
/// Only the Sets with a row, so a Set this does not name was asked blocking —
/// which is what [`kind`] reads a missing entry as.
pub async fn stored_on_timeline(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<HashMap<i64, Ask>> {
    let rows: Vec<(i64, bool)> = sqlx::query_as(
        "SELECT d.set_id, d.idled
         FROM deferrals d
         JOIN set_events s ON s.set_id = d.set_id
         JOIN timeline_events e ON e.id = s.event_id
         WHERE e.conversation_id = ?",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading Conversation {conversation_id}'s stored asks"))?;

    Ok(rows
        .into_iter()
        .map(|(set_id, idled)| (set_id, kind(Some(idled))))
        .collect())
}

/// How this Set was asked, as the record beside it says.
///
/// A Set with no row here was asked blocking, which is the whole of what the
/// row's absence means; a row says which of the two stored kinds it was.
pub async fn asked_as(pool: &SqlitePool, set_id: i64) -> Result<Ask> {
    let found: Option<(bool,)> = sqlx::query_as("SELECT idled FROM deferrals WHERE set_id = ?")
        .bind(set_id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("reading how Question Set {set_id} was asked"))?;

    Ok(kind(found.map(|(idled,)| idled)))
}

/// What a `deferrals` row — or the absence of one — comes to.
///
/// One place decides it, because more than one query reads the pair back and a
/// second reading of *nobody is idling on this* would be the one that could
/// disagree.
pub(crate) fn kind(idled: Option<bool>) -> Ask {
    match idled {
        None => Ask::Blocking,
        Some(true) => Ask::StoreAndNudge,
        Some(false) => Ask::Deferred,
    }
}

/// The Conversation's answered stored asks that no session has been told about,
/// oldest first — which is the order they were asked in, and so the order the
/// human decided them in.
///
/// Both stored kinds, and the row being there is the whole of the reading. A
/// store-and-nudge ask whose session went before the nudge reached it is a Set
/// with Answers and nobody left to hand them to, which is exactly what the
/// folding is for; one whose session is still there is folded only if it dies
/// before it fetches, because folding is recorded rather than worked out.
///
/// Answered only. An unanswered one is still waiting on the human and one they
/// closed unanswered is a decision they declined to make, and neither has
/// anything to fold.
pub async fn unfolded(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Unfolded>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT q.id, q.body, r.body
         FROM deferrals d
         JOIN question_sets q ON q.id = d.set_id
         JOIN set_events s ON s.set_id = q.id
         JOIN timeline_events e ON e.id = s.event_id
         JOIN responses r ON r.set_id = q.id
         WHERE e.conversation_id = ? AND d.folded_at IS NULL
         ORDER BY q.id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading Conversation {conversation_id}'s unfolded Deferred Asks"))?;

    rows.into_iter()
        .map(|(set_id, body, answer)| {
            Ok(Unfolded {
                set_id,
                // Never a failure, whatever the body turns out to hold: an
                // unreadable Set has no exchange to write into a prompt, and it
                // is passed over where the digest is built rather than costing
                // the Sets around it — see [`super::Asked`].
                set: super::Asked::read(body),
                response: serde_json::from_str(&answer).with_context(|| {
                    format!("deserialising the stored Response to Question Set {set_id}")
                })?,
            })
        })
        .collect()
}

/// Record that these Sets' Answers have gone into a session's prompt.
///
/// Written once a session has actually been started on that prompt, so a launch
/// that came to nothing does not cost the human's Answers the one session they
/// were folded into. Folding is a record rather than something worked out from
/// what is answered: that is what makes *each is folded once and never again*
/// true of a Conversation whose sessions run for days.
pub async fn record_folded(pool: &SqlitePool, sets: &[i64]) -> Result<()> {
    for set_id in sets {
        sqlx::query(
            "UPDATE deferrals SET folded_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE set_id = ? AND folded_at IS NULL",
        )
        .bind(set_id)
        .execute(pool)
        .await
        .with_context(|| format!("recording Question Set {set_id} as folded into a prompt"))?;
    }

    Ok(())
}
