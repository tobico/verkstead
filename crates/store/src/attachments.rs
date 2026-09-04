//! The files the human put on a Conversation for its sessions to read.
//!
//! One row per file, in a table of its own beside the Conversations rather than
//! anything on them: the `conversations` table is STRICT and there is no
//! migration machinery here, which is the unseen marks' reason and the
//! archivings' said again. Several per Conversation, where every other sidecar
//! is one or none — a Conversation takes as many files as the human has to
//! hand.
//!
//! **What a file is attached to is its origin.** The Brief is the one origin
//! there is, and an Answer to a Question Set is the one planned next: a second
//! value in this column rather than a second table, because the upload is the
//! Conversation's rather than the Brief's and what changes between the two is
//! only what the human was looking at when they made it.
//!
//! The bytes are not here. They are one flat directory per Conversation under
//! the Data Directory — see `crates/server/src/attachments.rs`, which owns that
//! side of it — and what this holds is the name the file ended up under, its
//! size and the moment. So a row is a record of a file rather than the file,
//! and the two are written and taken away together.
//!
//! **The name as it stands on disk**, which is not always the name the browser
//! sent: a second file of the same name is renamed rather than replacing the
//! first, and this is what it was renamed to. Every later reference goes by the
//! row's own id all the same — the name is what a human reads and an agent
//! opens, and neither is a key.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

/// One attached file, as the record holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// Its own id, which is what a removal and every later reference name it
    /// by. Two files on one Conversation may share a name — the renaming makes
    /// that hard rather than impossible, a rename being over a directory that
    /// may be written to by hand — and neither of them is a key.
    pub id: i64,

    /// What it was attached to.
    pub origin: Origin,

    /// The file's name as it stands in the Conversation's directory, extension
    /// and all — the name the browser sent, or what the renaming made of it.
    pub name: String,

    /// How large it is, in bytes. Kept rather than read off the file every time
    /// the Conversation is: what is drawn beside a pill and listed in a prompt
    /// is a fact about the file that was attached, and a share carries it
    /// somewhere there is no file at all.
    pub bytes: i64,

    /// When it was attached, RFC 3339.
    pub added_at: String,
}

/// What a file was attached to.
///
/// One value today. The second is an Answer to a Question Set, which is the
/// same upload made from a different page — see this module's own
/// documentation for why that is a value here rather than a table beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Put on the composer, beside the Brief being written.
    Brief,
}

impl Origin {
    /// The word the column holds. Lowercase and spelled out, so the table reads
    /// as something rather than as a number nobody can look up.
    pub(crate) fn stored(self) -> &'static str {
        match self {
            Self::Brief => "brief",
        }
    }

    /// The origin a stored word names. A word this does not know is a database
    /// written by a Verkstead this one does not understand, which is worth
    /// saying rather than guessing past — the same reading `Lifecycle` is given.
    pub(crate) fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "brief" => Self::Brief,
            other => bail!("a file is attached to the unknown origin {other:?}"),
        })
    }
}

/// The table the rows live in.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS attachments (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             origin          TEXT NOT NULL,
             name            TEXT NOT NULL,
             bytes           INTEGER NOT NULL,
             added_at        TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the attachments table")?;

    Ok(())
}

/// Write down a file that has landed in a Conversation's directory.
///
/// `name` is the name it ended up under rather than the one that was sent: the
/// renaming happens where the file is written, and what is recorded is what an
/// agent will find there.
///
/// Answers with the row as it now stands, the id and the moment included — the
/// caller hands both straight back to the composer, and reading the clock a
/// second time to say when would be reporting a different moment from the one
/// recorded.
pub async fn attach(
    pool: &SqlitePool,
    conversation_id: i64,
    origin: Origin,
    name: &str,
    bytes: i64,
) -> Result<Attachment> {
    let row: (i64, String) = sqlx::query_as(
        "INSERT INTO attachments (conversation_id, origin, name, bytes, added_at)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         RETURNING id, added_at",
    )
    .bind(conversation_id)
    .bind(origin.stored())
    .bind(name)
    .bind(bytes)
    .fetch_one(pool)
    .await
    .with_context(|| format!("attaching {name:?} to Conversation {conversation_id}"))?;

    Ok(Attachment {
        id: row.0,
        origin,
        name: name.to_owned(),
        bytes,
        added_at: row.1,
    })
}

/// Every file attached to one Conversation, oldest first.
///
/// Which is the order they were attached in, and the order the pills are drawn
/// in: a row of them is a record of what the human handed over, and re-sorting
/// it by name would put a file somewhere other than where they last saw it.
pub async fn attachments(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Attachment>> {
    let rows: Vec<(i64, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, origin, name, bytes, added_at
         FROM attachments
         WHERE conversation_id = ?
         ORDER BY id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the files attached to Conversation {conversation_id}"))?;

    rows.into_iter()
        .map(|(id, origin, name, bytes, added_at)| {
            Ok(Attachment {
                id,
                origin: Origin::read(&origin)?,
                name,
                bytes,
                added_at,
            })
        })
        .collect()
}

/// One of them, or `None` where this Conversation has no attachment with that
/// id.
///
/// Scoped to the Conversation rather than read by id alone, the way a companion
/// is: the id is in the path under a Conversation, and a row that belongs to
/// another one is not a row this request may touch.
pub async fn attachment(
    pool: &SqlitePool,
    conversation_id: i64,
    id: i64,
) -> Result<Option<Attachment>> {
    let row: Option<(String, String, i64, String)> = sqlx::query_as(
        "SELECT origin, name, bytes, added_at
         FROM attachments
         WHERE conversation_id = ? AND id = ?",
    )
    .bind(conversation_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading attachment {id} of Conversation {conversation_id}"))?;

    let Some((origin, name, bytes, added_at)) = row else {
        return Ok(None);
    };

    Ok(Some(Attachment {
        id,
        origin: Origin::read(&origin)?,
        name,
        bytes,
        added_at,
    }))
}

/// Take one off the record, answering whether there was one to take.
///
/// The file itself is removed where the file is, by the caller that has just
/// read this row for its name. Two writes rather than one act, and the row goes
/// last: a file left behind with no row is a stray in a directory the Cleanup
/// will take anyway, and a row left behind with no file is a pill that opens
/// nothing.
pub async fn detach(pool: &SqlitePool, conversation_id: i64, id: i64) -> Result<bool> {
    let taken = sqlx::query("DELETE FROM attachments WHERE conversation_id = ? AND id = ?")
        .bind(conversation_id)
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("detaching attachment {id} from Conversation {conversation_id}"))?
        .rows_affected();

    Ok(taken > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Conversation to attach to, over a database with nothing else in it.
    async fn conversation() -> (tempfile::TempDir, SqlitePool, i64) {
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
        let id = crate::start_conversation(&pool, repo.id, "attachments")
            .await
            .unwrap()
            .unwrap();

        (dir, pool, id)
    }

    #[tokio::test]
    async fn a_file_is_recorded_and_read_back() {
        let (_dir, pool, conversation) = conversation().await;

        let written = attach(&pool, conversation, Origin::Brief, "wireframe.png", 48_112)
            .await
            .unwrap();

        assert_eq!(written.name, "wireframe.png");
        assert_eq!(written.bytes, 48_112);
        assert_eq!(written.origin, Origin::Brief);

        assert_eq!(
            attachments(&pool, conversation).await.unwrap(),
            vec![written]
        );
    }

    /// Oldest first, which is the order they were handed over in.
    #[tokio::test]
    async fn they_come_back_in_the_order_they_were_attached() {
        let (_dir, pool, conversation) = conversation().await;

        for name in ["zebra.csv", "apple.png", "middle.md"] {
            attach(&pool, conversation, Origin::Brief, name, 12)
                .await
                .unwrap();
        }

        let names: Vec<String> = attachments(&pool, conversation)
            .await
            .unwrap()
            .into_iter()
            .map(|attachment| attachment.name)
            .collect();

        assert_eq!(names, ["zebra.csv", "apple.png", "middle.md"]);
    }

    /// One Conversation's files are not another's, which is what scopes both
    /// the reading and the removal.
    #[tokio::test]
    async fn another_conversations_attachment_is_not_this_ones() {
        let (_dir, pool, mine) = conversation().await;
        let theirs = crate::start_conversation(&pool, 1, "elsewhere")
            .await
            .unwrap()
            .unwrap();

        let put = attach(&pool, theirs, Origin::Brief, "notes.md", 9)
            .await
            .unwrap();

        assert_eq!(attachment(&pool, mine, put.id).await.unwrap(), None);
        assert!(!detach(&pool, mine, put.id).await.unwrap());
        assert_eq!(attachments(&pool, theirs).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn taking_one_off_says_whether_there_was_one() {
        let (_dir, pool, conversation) = conversation().await;

        let put = attach(&pool, conversation, Origin::Brief, "notes.md", 9)
            .await
            .unwrap();

        assert!(detach(&pool, conversation, put.id).await.unwrap());
        assert!(!detach(&pool, conversation, put.id).await.unwrap());
        assert!(attachments(&pool, conversation).await.unwrap().is_empty());
    }

    /// An origin this build does not know is a database written by a Verkstead
    /// this one does not understand, and worth saying rather than guessing past.
    #[tokio::test]
    async fn an_origin_nobody_here_knows_is_refused() {
        let (_dir, pool, conversation) = conversation().await;

        attach(&pool, conversation, Origin::Brief, "notes.md", 9)
            .await
            .unwrap();
        sqlx::query("UPDATE attachments SET origin = 'seance'")
            .execute(&pool)
            .await
            .unwrap();

        assert!(attachments(&pool, conversation).await.is_err());
    }
}
