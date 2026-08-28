//! Where the human put each Conversation in the sidebar.
//!
//! The list is one person's working set, so which piece of work sits at the top
//! is theirs to say rather than a sort's — dragging a row is how it is said, and
//! this is where what they said is kept. Nothing here decides anything: an order
//! arrives whole, and every Conversation named in it is placed at the position
//! it arrived in.
//!
//! Beside the Conversations rather than as a column on them, for the reason a
//! Set's lock is beside the Question Set: there is no migration machinery here,
//! `conversations` is STRICT, and a placement is a fact about the sidebar
//! rather than about the work.
//!
//! A Conversation with no row here has never been placed, and the sidebar puts
//! those at the top, newest first — see [`super::conversations`]. That is what
//! makes a hand-made order and a Conversation started a minute ago live
//! together: the order stands, and what is new arrives above it where it will
//! be seen.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS placements (
             conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
             place           INTEGER NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the placements table")?;

    Ok(())
}

/// Place the Conversations in the order given, and unplace everything else.
///
/// The whole order every time, because that is what a drag produces: the human
/// moved one row and the list they are looking at is the answer. Written in one
/// transaction, so a sidebar read halfway through gets the order before or the
/// order after and never half of each.
///
/// An id naming no Conversation is skipped rather than refused. A viewer sends
/// the list it drew, and a Conversation can be started or gone by the time it
/// arrives — neither is the human getting anything wrong, and refusing the whole
/// order over one stale id would throw away a drag that was about the others.
pub async fn place_conversations(pool: &SqlitePool, order: &[i64]) -> Result<()> {
    let mut tx = super::writing(pool, "placing the Conversations").await?;

    sqlx::query("DELETE FROM placements")
        .execute(&mut *tx)
        .await
        .context("clearing the placements")?;

    for (place, id) in order.iter().enumerate() {
        sqlx::query(
            // `OR IGNORE` for an id that arrives twice, which is a viewer with a
            // mistake in it rather than anything the human did: the first place
            // it was given stands, and the row it names is still placed.
            "INSERT OR IGNORE INTO placements (conversation_id, place)
             SELECT ?, ? WHERE EXISTS (SELECT 1 FROM conversations WHERE id = ?)",
        )
        .bind(id)
        .bind(i64::try_from(place).expect("a sidebar shorter than 2^63 rows"))
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("placing Conversation {id}"))?;
    }

    tx.commit().await.context("placing the Conversations")?;

    Ok(())
}
