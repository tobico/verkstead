//! The Skill Packs a Conversation asks its sessions to be given.
//!
//! [`super::mcp_servers`]' shape said again, for the same reasons: one row per
//! pack per Conversation in a table of its own, the name and nothing else, and
//! the name the key. A pack is a thing an installed plugin provides, so what a
//! Conversation holds is a reference to one by name, read against the plugins
//! that loaded on this device at the moment it is wanted — a pack disabled or
//! removed since leaves a row pointing at nothing, which is a chip that says
//! the pack is gone rather than one that quietly disappeared. See **Skill
//! Pack** in `CONTEXT.md`.
//!
//! **The freeze is asked here**, in [`attach_skill_pack_to_draft`] and
//! [`detach_skill_pack_from_draft`], as the servers' is. Whether a pack of that
//! name is installed is the server's, the plugins being something this crate
//! has never heard of.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::conversations::{Edited, not_drafting};

/// The table the references live in — [`super::mcp_servers`]' table, for its
/// reasons: the unique index refuses a second of one name, and the counting
/// key is the order the chips are drawn in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS conversation_skill_packs (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             name            TEXT NOT NULL,
             attached_at     TEXT NOT NULL,
             UNIQUE (conversation_id, name)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the conversation skill packs table")?;

    Ok(())
}

/// Put a pack's name on a Conversation while its round drafts, refused once
/// it is past drafting.
///
/// The look and the write under one lock, through [`super::writing`], as
/// [`super::attach_mcp_server_to_draft`] is. A name on it already is
/// [`Edited::Saved`] all the same: the state the press asked for is the state
/// there is.
pub async fn attach_skill_pack_to_draft(
    pool: &SqlitePool,
    conversation_id: i64,
    name: &str,
) -> Result<Edited> {
    let mut tx = super::writing(pool, "attaching a skill pack to a Brief").await?;

    if let Some(refusal) = not_drafting(&mut tx, conversation_id).await? {
        return Ok(refusal);
    }

    sqlx::query(
        "INSERT INTO conversation_skill_packs (conversation_id, name, attached_at)
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, name) DO NOTHING",
    )
    .bind(conversation_id)
    .bind(name)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("attaching skill pack {name:?} to Conversation {conversation_id}"))?;

    tx.commit()
        .await
        .context("attaching a skill pack to a Brief")?;

    Ok(Edited::Saved)
}

/// And take one off again, under the same lock. A name that is not on it is no
/// refusal: what the × asked for is that the chip be gone.
pub async fn detach_skill_pack_from_draft(
    pool: &SqlitePool,
    conversation_id: i64,
    name: &str,
) -> Result<Edited> {
    let mut tx = super::writing(pool, "taking a skill pack off a Brief").await?;

    if let Some(refusal) = not_drafting(&mut tx, conversation_id).await? {
        return Ok(refusal);
    }

    sqlx::query("DELETE FROM conversation_skill_packs WHERE conversation_id = ? AND name = ?")
        .bind(conversation_id)
        .bind(name)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!("taking skill pack {name:?} off Conversation {conversation_id}")
        })?;

    tx.commit()
        .await
        .context("taking a skill pack off a Brief")?;

    Ok(Edited::Saved)
}

/// Every pack one Conversation has attached, in the order they were attached
/// in — the names alone, which are the whole of what is stored.
pub async fn skill_packs(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT name
         FROM conversation_skill_packs
         WHERE conversation_id = ?
         ORDER BY id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading the skill packs attached to Conversation {conversation_id}")
    })?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

    async fn drafting() -> (tempfile::TempDir, SqlitePool, i64) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let repo = crate::register_repo(
            &pool,
            std::path::Path::new("/srv/verkstead"),
            "verkstead",
            "main",
        )
        .await
        .unwrap()
        .unwrap();
        let id = crate::start_conversation(&pool, repo.id, "skill-packs", THIS_DEVICE, None)
            .await
            .unwrap()
            .unwrap();

        (dir, pool, id)
    }

    #[tokio::test]
    async fn the_names_come_back_in_the_order_they_were_attached_and_once_each() {
        let (_dir, pool, id) = drafting().await;

        for name in ["review-kit", "docs", "review-kit"] {
            assert_eq!(
                attach_skill_pack_to_draft(&pool, id, name).await.unwrap(),
                Edited::Saved
            );
        }

        assert_eq!(
            skill_packs(&pool, id).await.unwrap(),
            vec!["review-kit", "docs"]
        );
        assert_eq!(
            crate::load_conversation(&pool, id)
                .await
                .unwrap()
                .unwrap()
                .skill_packs,
            vec!["review-kit", "docs"],
            "a Conversation carries the packs it has attached, which every launch reads",
        );
    }

    #[tokio::test]
    async fn taking_one_off_leaves_the_rest() {
        let (_dir, pool, id) = drafting().await;

        attach_skill_pack_to_draft(&pool, id, "docs").await.unwrap();
        attach_skill_pack_to_draft(&pool, id, "review-kit")
            .await
            .unwrap();

        assert_eq!(
            detach_skill_pack_from_draft(&pool, id, "docs")
                .await
                .unwrap(),
            Edited::Saved
        );
        assert_eq!(
            detach_skill_pack_from_draft(&pool, id, "docs")
                .await
                .unwrap(),
            Edited::Saved,
            "a chip that is not there is the state the × asked for"
        );

        assert_eq!(skill_packs(&pool, id).await.unwrap(), vec!["review-kit"]);
    }

    /// Fixed once the round starts, both ways, as the servers are.
    #[tokio::test]
    async fn the_packs_freeze_with_the_brief() {
        let (_dir, pool, id) = drafting().await;
        attach_skill_pack_to_draft(&pool, id, "docs").await.unwrap();

        sqlx::query("UPDATE conversations SET state = 'grilling' WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            attach_skill_pack_to_draft(&pool, id, "review-kit")
                .await
                .unwrap(),
            Edited::NotDrafting
        );
        assert_eq!(
            detach_skill_pack_from_draft(&pool, id, "docs")
                .await
                .unwrap(),
            Edited::NotDrafting
        );
        assert_eq!(skill_packs(&pool, id).await.unwrap(), vec!["docs"]);
    }
}
