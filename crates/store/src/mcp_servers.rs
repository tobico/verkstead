//! The MCP servers a Conversation asks its sessions to be launched with.
//!
//! One row per server per Conversation, in a table of its own beside the
//! Conversations rather than anything on them: the `conversations` table is
//! STRICT and there is no migration machinery here, which is the attachments'
//! reason and the companions' said again. Several per Conversation, as the
//! attachments are — the human attaches whichever of the declarations this
//! piece of work needs.
//!
//! **What is stored is the name and nothing else.** A declaration is a name, a
//! URL and headers on the settings page, kept in `config.yaml`; what a
//! Conversation holds is a *reference* to one by name, looked up afresh
//! wherever it is needed. So a URL corrected in the settings fixes every
//! Conversation that attached that server, and a declaration deleted there
//! leaves a row pointing at nothing — which is a chip that says the server is
//! gone rather than a row that quietly disappeared. See **MCP server** in
//! `CONTEXT.md`, and [`super::companions`], which keeps a reference the other
//! way round: a Repo by its id, because a Repo is a row here.
//!
//! **The name is the key**, which is what makes *attached twice* something the
//! insert refuses rather than something a read-then-write has to notice in
//! time — the companions' primary key, for its reason.
//!
//! Nothing here asks whether the Conversation may still be changed, or whether
//! anything of that name is declared. Both are the server's: the freeze is the
//! Brief's own and is read where the file uploads read it, and the declarations
//! are in a file this crate has never heard of.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The table the references live in.
///
/// One row per name per Conversation, which the unique index is what refuses a
/// second of — the companions' primary key said as a constraint beside a
/// counting key, because what the rows are read back *in* is the order they
/// arrived in. That is the attachments' table's shape and for its reason: the
/// chips are drawn in the order the human attached them, and a clock is not an
/// order — two attached inside one millisecond would come back in whichever
/// order the tie-break happened to give.
///
/// `attached_at` is kept all the same, as the attachments' `added_at` is: it is
/// what the row is a record *of*, and a table nothing could be read out of by
/// hand is a table nobody can answer a question about.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS conversation_mcp_servers (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             name            TEXT NOT NULL,
             attached_at     TEXT NOT NULL,
             UNIQUE (conversation_id, name)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the conversation MCP servers table")?;

    Ok(())
}

/// Put a declared server's name on a Conversation, answering whether it was not
/// there already.
///
/// `false` is the name being on it twice over, which is what the primary key
/// refuses — two tabs, or a menu drawn before the chip landed. The caller says
/// nothing about it: the state the press asked for is the state there is.
pub async fn attach_mcp_server(
    pool: &SqlitePool,
    conversation_id: i64,
    name: &str,
) -> Result<bool> {
    let put: Option<(String,)> = sqlx::query_as(
        "INSERT INTO conversation_mcp_servers (conversation_id, name, attached_at)
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (conversation_id, name) DO NOTHING
         RETURNING name",
    )
    .bind(conversation_id)
    .bind(name)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("attaching MCP server {name:?} to Conversation {conversation_id}"))?;

    Ok(put.is_some())
}

/// And take one off again, answering whether there was one to take.
///
/// A name that is not on it is no refusal, the way a companion's removal and an
/// attachment's are none: what the × asked for is that the chip be gone.
pub async fn detach_mcp_server(
    pool: &SqlitePool,
    conversation_id: i64,
    name: &str,
) -> Result<bool> {
    let taken =
        sqlx::query("DELETE FROM conversation_mcp_servers WHERE conversation_id = ? AND name = ?")
            .bind(conversation_id)
            .bind(name)
            .execute(pool)
            .await
            .with_context(|| {
                format!("taking MCP server {name:?} off Conversation {conversation_id}")
            })?
            .rows_affected();

    Ok(taken > 0)
}

/// Every MCP server one Conversation has attached, in the order they were
/// attached in.
///
/// The names alone, because the names are the whole of what is stored: what
/// each of them refers to is read out of the settings by whoever needs the URL.
pub async fn mcp_servers(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT name
         FROM conversation_mcp_servers
         WHERE conversation_id = ?
         ORDER BY id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading the MCP servers attached to Conversation {conversation_id}")
    })?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Conversation to attach to, over a database with nothing else in it.
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
        let id = crate::start_conversation(&pool, repo.id, "mcp-servers")
            .await
            .unwrap()
            .unwrap();

        (dir, pool, id)
    }

    #[tokio::test]
    async fn a_conversation_starts_with_no_servers_on_it() {
        let (_dir, pool, id) = drafting().await;

        assert!(mcp_servers(&pool, id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_names_come_back_in_the_order_they_were_attached() {
        let (_dir, pool, id) = drafting().await;

        for name in ["tickets", "docs", "alerts"] {
            assert!(attach_mcp_server(&pool, id, name).await.unwrap());
        }

        assert_eq!(
            mcp_servers(&pool, id).await.unwrap(),
            vec!["tickets", "docs", "alerts"],
        );
    }

    /// The primary key rather than a look that two tabs could both get past —
    /// and the second attach is answered rather than raised.
    #[tokio::test]
    async fn the_same_name_twice_is_one_row() {
        let (_dir, pool, id) = drafting().await;

        assert!(attach_mcp_server(&pool, id, "docs").await.unwrap());
        assert!(!attach_mcp_server(&pool, id, "docs").await.unwrap());

        assert_eq!(mcp_servers(&pool, id).await.unwrap(), vec!["docs"]);
    }

    #[tokio::test]
    async fn taking_one_off_leaves_the_rest() {
        let (_dir, pool, id) = drafting().await;

        attach_mcp_server(&pool, id, "docs").await.unwrap();
        attach_mcp_server(&pool, id, "tickets").await.unwrap();

        assert!(detach_mcp_server(&pool, id, "docs").await.unwrap());

        assert_eq!(mcp_servers(&pool, id).await.unwrap(), vec!["tickets"]);
    }

    /// A name that is not on it is the state the × asked for rather than a
    /// refusal, so the answer says there was nothing to take and nothing else
    /// happens.
    #[tokio::test]
    async fn taking_off_one_that_is_not_there_says_so() {
        let (_dir, pool, id) = drafting().await;

        assert!(!detach_mcp_server(&pool, id, "docs").await.unwrap());
    }

    /// And the names are on the Conversation itself, which is what every
    /// launch reads them off — the companions' rule, for the companions'
    /// reason.
    #[tokio::test]
    async fn a_conversation_carries_the_names_it_has_attached() {
        let (_dir, pool, id) = drafting().await;

        assert!(
            crate::load_conversation(&pool, id)
                .await
                .unwrap()
                .unwrap()
                .mcp_servers
                .is_empty(),
            "a Conversation with nothing attached carries nothing"
        );

        attach_mcp_server(&pool, id, "tickets").await.unwrap();
        attach_mcp_server(&pool, id, "docs").await.unwrap();

        assert_eq!(
            crate::load_conversation(&pool, id)
                .await
                .unwrap()
                .unwrap()
                .mcp_servers,
            vec!["tickets", "docs"],
        );
    }

    /// One Conversation's chips are its own: the name is a key inside a
    /// Conversation rather than across the record.
    #[tokio::test]
    async fn one_conversations_servers_are_not_anothers() {
        let (_dir, pool, mine) = drafting().await;
        let repo = crate::registered_repos(&pool).await.unwrap()[0].clone();
        let theirs = crate::start_conversation(&pool, repo.id, "other")
            .await
            .unwrap()
            .unwrap();

        attach_mcp_server(&pool, mine, "docs").await.unwrap();
        assert!(attach_mcp_server(&pool, theirs, "docs").await.unwrap());

        assert_eq!(mcp_servers(&pool, mine).await.unwrap(), vec!["docs"]);

        detach_mcp_server(&pool, mine, "docs").await.unwrap();

        assert_eq!(mcp_servers(&pool, theirs).await.unwrap(), vec!["docs"]);
    }
}
