//! The files the human put on a Conversation for its sessions to read.
//!
//! One row per file, in a table of its own beside the Conversations rather than
//! anything on them: the `conversations` table is STRICT and there is no
//! migration machinery here, which is the unseen marks' reason and the
//! archivings' said again. Several per Conversation, where every other sidecar
//! is one or none — a Conversation takes as many files as the human has to
//! hand.
//!
//! **What a file is attached to is its origin.** There are two of them: the
//! Brief, and an Answer to a Question Set. A second value in this column rather
//! than a second table, because the upload is the Conversation's rather than the
//! Brief's and what changes between the two is only what the human was looking
//! at when they made it.
//!
//! An Answer's file names the Set it was put on and the label of the Question it
//! answers, in two columns the Brief's rows leave empty — which is the whole of
//! what the second origin costs the table, and is what lets the Set's own page
//! read back the files put on it. Both arrived after the table did, so they
//! arrive on a database written before them as well: see
//! [`super::migrations`], where the one-time rewrites live.
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
/// Two values, and the second names which Answer it is: the same upload made
/// from a different page — see this module's own documentation for why that is
/// a value here rather than a table beside it.
///
/// The Set and the label ride inside the value rather than beside it, because
/// there is no such thing as an Answer's file that names neither and no such
/// thing as a Brief's that names either. What the table holds is the word and
/// two columns; what this holds is the pair that word is only ever read with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Put on the composer, beside the Brief being written.
    Brief,

    /// Put on the answer sheet, under one Question of one Question Set.
    Answer {
        /// The Set it was put on.
        set: i64,

        /// And the label of the Question it answers — `Q7` for a Question,
        /// `Q7a` for a Sub-question, which is what an Answer names one by.
        label: String,
    },
}

impl Origin {
    /// The word the column holds. Lowercase and spelled out, so the table reads
    /// as something rather than as a number nobody can look up.
    pub(crate) fn stored(&self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::Answer { .. } => "answer",
        }
    }

    /// The Set an Answer's file was put on, and nothing for the Brief's.
    pub(crate) fn set(&self) -> Option<i64> {
        match self {
            Self::Brief => None,
            Self::Answer { set, .. } => Some(*set),
        }
    }

    /// And the Question it was put under, the same way round.
    pub(crate) fn label(&self) -> Option<&str> {
        match self {
            Self::Brief => None,
            Self::Answer { label, .. } => Some(label),
        }
    }

    /// The origin a stored row names: the word, and the two columns it is read
    /// with.
    ///
    /// A word this does not know is a database written by a Verkstead this one
    /// does not understand, which is worth saying rather than guessing past —
    /// the same reading `Lifecycle` is given. An `answer` row that names no Set
    /// or no Question is the same kind of thing said the other way round: the
    /// word says what the two columns are for, so a row where they disagree is
    /// one nothing here can make an Answer's file out of.
    pub(crate) fn read(word: &str, set: Option<i64>, label: Option<String>) -> Result<Self> {
        Ok(match word {
            "brief" => Self::Brief,
            "answer" => match (set, label) {
                (Some(set), Some(label)) => Self::Answer { set, label },
                _ => bail!("a file is attached to an Answer that names no Set or no Question"),
            },
            other => bail!("a file is attached to the unknown origin {other:?}"),
        })
    }
}

/// The table the rows live in.
///
/// `set_id` and `label` are an Answer's file's own two columns, empty on every
/// Brief's — which is what a database written before there was a second origin
/// has them added as, and why they are nullable here as well. See
/// [`super::migrations`].
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS attachments (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             origin          TEXT NOT NULL,
             name            TEXT NOT NULL,
             bytes           INTEGER NOT NULL,
             added_at        TEXT NOT NULL,
             set_id          INTEGER REFERENCES question_sets(id),
             label           TEXT
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
        "INSERT INTO attachments (conversation_id, origin, name, bytes, added_at, set_id, label)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?)
         RETURNING id, added_at",
    )
    .bind(conversation_id)
    .bind(origin.stored())
    .bind(name)
    .bind(bytes)
    .bind(origin.set())
    .bind(origin.label())
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

/// One row as every reading below selects it: the columns in one order, so that
/// what a row is made of is said once — see [`made`].
type Row = (
    i64,
    String,
    String,
    i64,
    String,
    Option<i64>,
    Option<String>,
);

/// A row as an [`Attachment`], which is where the origin's three columns become
/// the one value they are only ever read as.
fn made((id, origin, name, bytes, added_at, set, label): Row) -> Result<Attachment> {
    Ok(Attachment {
        id,
        origin: Origin::read(&origin, set, label)?,
        name,
        bytes,
        added_at,
    })
}

/// Every file attached to one Conversation, oldest first.
///
/// Which is the order they were attached in, and the order the pills are drawn
/// in: a row of them is a record of what the human handed over, and re-sorting
/// it by name would put a file somewhere other than where they last saw it.
///
/// Both origins, because what this is a listing of is the Conversation's own
/// directory: every file in it was put there by the human, whichever page they
/// were on at the time. A caller after one origin's alone reads the origin.
pub async fn attachments(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<Attachment>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, origin, name, bytes, added_at, set_id, label
         FROM attachments
         WHERE conversation_id = ?
         ORDER BY id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the files attached to Conversation {conversation_id}"))?;

    rows.into_iter().map(made).collect()
}

/// And every file put on one Question Set, oldest first for that reason again.
///
/// Which Question each is under is its own origin's, so this is one reading
/// whatever the Set asks: the sheet, the record after it and a Share all draw
/// their pills by grouping these under the labels they name.
pub async fn set_attachments(pool: &SqlitePool, set_id: i64) -> Result<Vec<Attachment>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, origin, name, bytes, added_at, set_id, label
         FROM attachments
         WHERE set_id = ?
         ORDER BY id",
    )
    .bind(set_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the files put on Question Set {set_id}"))?;

    rows.into_iter().map(made).collect()
}

/// The title of every Question Set this Conversation has files on the Answers
/// to, oldest Set first.
///
/// What a later session's prompt heads an Answer's files with — see
/// `attached_to` in `crates/server/src/skills.rs`. A label alone says which of a
/// Set's Questions and nothing about which Set, and a Conversation that has been
/// asked all week has plenty of both; the title is on the Set rather than on the
/// file, so it is read beside the files rather than carried on them.
///
/// Only the Sets these files name, rather than every Set the Conversation ever
/// asked: the join is what drops the Brief's rows, which name none.
pub async fn attached_sets(pool: &SqlitePool, conversation_id: i64) -> Result<Vec<(i64, String)>> {
    sqlx::query_as(
        "SELECT DISTINCT a.set_id, q.title
         FROM attachments a
         JOIN question_sets q ON q.id = a.set_id
         WHERE a.conversation_id = ?
         ORDER BY a.set_id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!("reading the Sets Conversation {conversation_id} has files on the Answers to")
    })
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
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, origin, name, bytes, added_at, set_id, label
         FROM attachments
         WHERE conversation_id = ? AND id = ?",
    )
    .bind(conversation_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading attachment {id} of Conversation {conversation_id}"))?;

    row.map(made).transpose()
}

/// And one of the files put on a Set, scoped to that Set for the same reason:
/// the id is in a path under it, and another Set's row is not this request's to
/// touch — not even another Set of the same Conversation.
pub async fn set_attachment(pool: &SqlitePool, set_id: i64, id: i64) -> Result<Option<Attachment>> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, origin, name, bytes, added_at, set_id, label
         FROM attachments
         WHERE set_id = ? AND id = ?",
    )
    .bind(set_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading attachment {id} of Question Set {set_id}"))?;

    row.map(made).transpose()
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

/// And the same for a file put on a Set, scoped the way [`set_attachment`] is.
pub async fn detach_from_set(pool: &SqlitePool, set_id: i64, id: i64) -> Result<bool> {
    let taken = sqlx::query("DELETE FROM attachments WHERE set_id = ? AND id = ?")
        .bind(set_id)
        .bind(id)
        .execute(pool)
        .await
        .with_context(|| format!("detaching attachment {id} from Question Set {set_id}"))?
        .rows_affected();

    Ok(taken > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The device every Conversation started here is ranked by, named the way a
    /// cluster names one (ADR-0020, *Ranks*).
    const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

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
        let id = crate::start_conversation(&pool, repo.id, "attachments", THIS_DEVICE)
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
        let theirs = crate::start_conversation(&pool, 1, "elsewhere", THIS_DEVICE)
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

    /// A Question Set on that Conversation's Timeline, to put files on.
    async fn asked(pool: &SqlitePool, conversation: i64) -> i64 {
        asked_about(pool, conversation, "the rate limiter").await
    }

    /// And one called something in particular, for the reading that is about
    /// what a Set is called rather than about what was put on it.
    async fn asked_about(pool: &SqlitePool, conversation: i64, title: &str) -> i64 {
        let set = verkstead_schema::QuestionSet::from_yaml(&format!(
            "title: {title}\n\
             questions:\n\
             \x20 - label: Q1\n\
             \x20   text: where does the counter live?\n",
        ))
        .unwrap();

        crate::ask(pool, conversation, &set, crate::Ask::Blocking)
            .await
            .unwrap()
            .unwrap()
            .id
    }

    /// A file put on an Answer names the Set and the Question, and comes back
    /// under both: on the Set it was put on, and among the Conversation's own
    /// files, which are every file in its directory whatever page they were put
    /// from.
    #[tokio::test]
    async fn a_file_on_an_answer_names_the_set_and_the_question() {
        let (_dir, pool, conversation) = conversation().await;
        let set = asked(&pool, conversation).await;

        let put = attach(
            &pool,
            conversation,
            Origin::Answer {
                set,
                label: "Q1".to_owned(),
            },
            "counter.png",
            2_048,
        )
        .await
        .unwrap();

        assert_eq!(
            put.origin,
            Origin::Answer {
                set,
                label: "Q1".to_owned(),
            },
        );

        assert_eq!(
            set_attachments(&pool, set).await.unwrap(),
            vec![put.clone()]
        );
        assert_eq!(attachments(&pool, conversation).await.unwrap(), vec![put]);
    }

    /// And the Brief's files are not on any Set, which is what makes the Set's
    /// own reading the sheet's rather than the Conversation's.
    #[tokio::test]
    async fn the_briefs_files_are_on_no_set() {
        let (_dir, pool, conversation) = conversation().await;
        let set = asked(&pool, conversation).await;

        attach(&pool, conversation, Origin::Brief, "wireframe.png", 12)
            .await
            .unwrap();

        assert!(set_attachments(&pool, set).await.unwrap().is_empty());
    }

    /// One Set's file is not another's, the way one Conversation's is not
    /// another's — and not even where the two Sets are the same Conversation's.
    #[tokio::test]
    async fn another_sets_attachment_is_not_this_ones() {
        let (_dir, pool, conversation) = conversation().await;
        let mine = asked(&pool, conversation).await;
        let theirs = asked(&pool, conversation).await;

        let put = attach(
            &pool,
            conversation,
            Origin::Answer {
                set: theirs,
                label: "Q1".to_owned(),
            },
            "notes.md",
            9,
        )
        .await
        .unwrap();

        assert_eq!(set_attachment(&pool, mine, put.id).await.unwrap(), None);
        assert!(!detach_from_set(&pool, mine, put.id).await.unwrap());
        assert_eq!(set_attachments(&pool, theirs).await.unwrap().len(), 1);

        assert!(detach_from_set(&pool, theirs, put.id).await.unwrap());
        assert!(set_attachments(&pool, theirs).await.unwrap().is_empty());
    }

    /// What the Sets an Answer's files were put on are called, which is what a
    /// later session's prompt heads them with. The Brief's own files name no
    /// Set, so a Conversation with only those has none of these.
    #[tokio::test]
    async fn the_sets_a_conversations_answer_files_were_put_on_say_what_they_are_called() {
        let (_dir, pool, conversation) = conversation().await;
        let counting = asked_about(&pool, conversation, "How the limiter counts").await;
        let wording = asked_about(&pool, conversation, "The wording of the error").await;

        attach(&pool, conversation, Origin::Brief, "rates.csv", 12)
            .await
            .unwrap();

        assert!(attached_sets(&pool, conversation).await.unwrap().is_empty());

        for set in [wording, counting, counting] {
            attach(
                &pool,
                conversation,
                Origin::Answer {
                    set,
                    label: "Q1".to_owned(),
                },
                "trace.txt",
                9,
            )
            .await
            .unwrap();
        }

        assert_eq!(
            attached_sets(&pool, conversation).await.unwrap(),
            [
                (counting, "How the limiter counts".to_owned()),
                (wording, "The wording of the error".to_owned()),
            ],
            "each Set named once, whatever it is holding",
        );
    }

    /// An `answer` row that names no Set is a record nothing here can read as
    /// either origin, and it is said rather than guessed past.
    #[tokio::test]
    async fn an_answer_that_names_no_set_is_refused() {
        let (_dir, pool, conversation) = conversation().await;
        let set = asked(&pool, conversation).await;

        attach(
            &pool,
            conversation,
            Origin::Answer {
                set,
                label: "Q1".to_owned(),
            },
            "notes.md",
            9,
        )
        .await
        .unwrap();

        sqlx::query("UPDATE attachments SET set_id = NULL")
            .execute(&pool)
            .await
            .unwrap();

        assert!(attachments(&pool, conversation).await.is_err());
    }
}
