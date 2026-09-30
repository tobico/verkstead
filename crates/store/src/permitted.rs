//! The devices of the cluster a Conversation's work may be transferred to by
//! the agent doing it (ADR-0020, *The agent's call*).
//!
//! **The ticks are the consent.** The human ticks, per Conversation, which
//! other machines the session may move the work onto, and nothing asks them
//! again when it does — so what is written here is the whole of the human's say
//! in a move the agent makes. A move the human presses ignores it entirely.
//!
//! **The drafting device is never a row.** The device in the Conversation's
//! **birth key** is always permitted, so a session that has moved can always
//! come home — which makes it implicit rather than a tick somebody could take
//! away. Nothing is ticked by default: a Conversation with no rows is one whose
//! agent may go nowhere but home.
//!
//! **A Device Id rather than anything renumbered.** Device Ids are the
//! cluster's own and mean the same machine in every database, so the rows
//! cross with the record as they stand — see [`super::slices`] — and a device
//! that leaves the cluster is taken off every list by the unlink itself: see
//! [`super::forget_member`].
//!
//! A table of its own beside `conversations` for the reason the MCP servers'
//! is: that table is STRICT, there is no migration machinery below it, and a
//! list is not a column.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The table the ticks live in: one row per device per Conversation, the pair
/// being the key, so a second tick of the same device is the insert refusing
/// rather than a read-then-write noticing in time.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS permitted_devices (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             device          TEXT NOT NULL,
             UNIQUE (conversation_id, device)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the permitted devices table")?;

    Ok(())
}

/// Tick a device: the agent doing this Conversation's work may move it there.
///
/// A device ticked already is the state the press asked for, and nothing is
/// said about it. Whether the device is one of the cluster's, and whether it is
/// the drafting device, are the server's to ask — this is the row.
pub async fn permit_device(pool: &SqlitePool, conversation_id: i64, device: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO permitted_devices (conversation_id, device) VALUES (?, ?)
         ON CONFLICT (conversation_id, device) DO NOTHING",
    )
    .bind(conversation_id)
    .bind(device)
    .execute(pool)
    .await
    .with_context(|| {
        format!("permitting Conversation {conversation_id} to be moved to device {device}")
    })?;

    Ok(())
}

/// And untick one. A device that was not ticked is the state the press asked
/// for, so there is nothing to refuse.
pub async fn forbid_device(pool: &SqlitePool, conversation_id: i64, device: &str) -> Result<()> {
    sqlx::query("DELETE FROM permitted_devices WHERE conversation_id = ? AND device = ?")
        .bind(conversation_id)
        .bind(device)
        .execute(pool)
        .await
        .with_context(|| {
            format!("taking device {device} off what Conversation {conversation_id} may move to")
        })?;

    Ok(())
}

/// Every device ticked on a Conversation, in the order they were ticked — the
/// drafting device not among them, it being permitted without a row.
pub async fn permitted_devices(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT device FROM permitted_devices WHERE conversation_id = ? ORDER BY id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading the devices Conversation {conversation_id} may be moved to")
    })?;

    Ok(rows.into_iter().map(|(device,)| device).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";
    const LAPTOP: &str = "11111111111111111111111111111111";
    const VM: &str = "22222222222222222222222222222222";

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
        let id = crate::start_conversation(&pool, repo.id, "permitted", THIS_DEVICE)
            .await
            .unwrap()
            .unwrap();

        (dir, pool, id)
    }

    #[tokio::test]
    async fn nothing_is_ticked_to_begin_with() {
        let (_dir, pool, id) = drafting().await;

        assert!(permitted_devices(&pool, id).await.unwrap().is_empty());
        assert!(
            crate::load_conversation(&pool, id)
                .await
                .unwrap()
                .unwrap()
                .permitted
                .is_empty()
        );
    }

    #[tokio::test]
    async fn ticks_come_back_in_the_order_they_were_made_and_once_each() {
        let (_dir, pool, id) = drafting().await;

        permit_device(&pool, id, VM).await.unwrap();
        permit_device(&pool, id, LAPTOP).await.unwrap();
        permit_device(&pool, id, VM).await.unwrap();

        assert_eq!(
            permitted_devices(&pool, id).await.unwrap(),
            vec![VM, LAPTOP]
        );
        assert_eq!(
            crate::load_conversation(&pool, id)
                .await
                .unwrap()
                .unwrap()
                .permitted,
            vec![VM, LAPTOP],
        );
    }

    #[tokio::test]
    async fn an_untick_takes_that_device_and_no_other() {
        let (_dir, pool, id) = drafting().await;

        permit_device(&pool, id, VM).await.unwrap();
        permit_device(&pool, id, LAPTOP).await.unwrap();

        forbid_device(&pool, id, VM).await.unwrap();
        forbid_device(&pool, id, VM).await.unwrap();

        assert_eq!(permitted_devices(&pool, id).await.unwrap(), vec![LAPTOP]);
    }

    /// The unlink's half: a device that is not a member any more is on no
    /// Conversation's list, whichever of them ticked it.
    #[tokio::test]
    async fn forgetting_a_member_takes_it_off_every_list() {
        let (_dir, pool, mine) = drafting().await;
        let repo = crate::registered_repos(&pool).await.unwrap()[0].clone();
        let other = crate::start_conversation(&pool, repo.id, "other", THIS_DEVICE)
            .await
            .unwrap()
            .unwrap();

        for id in [mine, other] {
            permit_device(&pool, id, VM).await.unwrap();
            permit_device(&pool, id, LAPTOP).await.unwrap();
        }

        crate::forget_member(&pool, VM).await.unwrap();

        for id in [mine, other] {
            assert_eq!(permitted_devices(&pool, id).await.unwrap(), vec![LAPTOP]);
        }

        crate::forget_every_member(&pool).await.unwrap();

        for id in [mine, other] {
            assert!(permitted_devices(&pool, id).await.unwrap().is_empty());
        }
    }
}
