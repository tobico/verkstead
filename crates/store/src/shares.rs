//! Where a Conversation's share was published, and when it was taken.
//!
//! A share is a file, and publishing one puts that file somewhere a link can
//! reach — a secret gist, made by the server through the configured token. What
//! is worth keeping about it is the gist and the moment, which are the two
//! facts a comment on a pull request and the workbench's Share row are both
//! written from.
//!
//! **The gist as GitHub gave it, rather than where a reader is sent.** Those
//! stopped being the same thing: a reader is sent through the share viewer,
//! which is a page that can move and a setting the human can change, so the
//! link is composed as it is drawn rather than written down here — see `link`
//! in `crates/server/src/sharing.rs`. Which is what lets a share published
//! before there was a viewer link through one today, and a viewer moved later
//! retarget every link there is without republishing anything. What this row
//! holds is where the file went, and that never changes.
//!
//! One row or none per Conversation, beside them rather than a column on them:
//! there is no migration machinery here and `conversations` is STRICT, which is
//! the unseen marks' reason and the archivings' said again.
//!
//! **The latest one, rather than every one ever made.** Publishing again is a
//! fresh snapshot of a Conversation that has moved, so the link it replaces is a
//! link to a file nobody should be sent to any more — the record here is *where
//! the share of this Conversation is*, and there is one answer to that at a
//! time. What was already sent is not unsent: a gist published yesterday goes on
//! standing at its own URL, and a comment left on a pull request goes on
//! pointing at it. This is only what the next comment is written from.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The published share of one Conversation: where it is, and when it was taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    /// The gist, as GitHub gave it — whole, so that nothing downstream has to
    /// build one out of an id and a guess about where gists live.
    ///
    /// Not the link a reader is handed. That is this URL composed through the
    /// share viewer, and it is composed on the way out rather than kept here —
    /// see the note above.
    pub url: String,

    /// When the share was published, RFC 3339. The moment matters because a
    /// share is a snapshot: a link with no date beside it says nothing about how
    /// far the Conversation has moved since.
    pub at: String,
}

/// The table a published share's row lives in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS shares (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             url             TEXT NOT NULL,
             at              TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the shares table")?;

    Ok(())
}

/// Write down where this Conversation's share was just published, replacing
/// wherever the one before it went.
///
/// Answers with the row as it now stands, the moment included: the caller shows
/// the human what was published and when, and reading the clock a second time to
/// say so would be reporting a different moment from the one recorded.
pub async fn record_share(pool: &SqlitePool, conversation_id: i64, url: &str) -> Result<Share> {
    let row: (String, String) = sqlx::query_as(
        "INSERT INTO shares (conversation_id, url, at)
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id) DO UPDATE
           SET url = excluded.url, at = excluded.at
         RETURNING url, at",
    )
    .bind(conversation_id)
    .bind(url)
    .fetch_one(pool)
    .await
    .with_context(|| format!("recording the share of Conversation {conversation_id}"))?;

    Ok(Share {
        url: row.0,
        at: row.1,
    })
}

/// And where it is, or `None` on a Conversation nobody has published one of.
pub async fn share(pool: &SqlitePool, conversation_id: i64) -> Result<Option<Share>> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT url, at FROM shares WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading the share of Conversation {conversation_id}"))?;

    Ok(row.map(|(url, at)| Share { url, at }))
}
