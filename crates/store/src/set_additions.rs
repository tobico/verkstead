//! What the human adds to a Conversation from a Question Set's sheet: an MCP
//! server, a companion, a companion opened up to read-write, a device the work
//! may be moved to (ADR-0028).
//!
//! **Recorded as it is picked, applied at Submit.** A pick lands here the moment
//! it is made — so it survives a reload and shows on another device, and the ×
//! on its chip takes it off again — and nothing reaches the Conversation until
//! the Set is answered. Which is why each row carries a **state**: pending while
//! the Set waits, applied once a Submit has put it on the Conversation, struck
//! where a lock, a Close or a Transfer left it unapplied. The rows outlive the
//! Set settling, because they are the record of what came with the answer.
//!
//! **Under the label of the Question it was picked at**, the way a file on an
//! Answer is, or under none where it was picked at the set-level comment box.
//!
//! **One table for every kind**, with a column per thing a kind can name and the
//! kind's word saying which of them are filled. Several tables would be one
//! list read back in four pieces, and what the sheet draws is one row of chips
//! in the order the human picked.
//!
//! **The freeze is asked here**, in [`add_to_set`] and [`take_off_set`]: the
//! Set's Response or its lock, read under the lock the write is made under — a
//! Submit landing between the look and the write would otherwise have a pick
//! recorded on a Set whose additions had already been applied.

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

use super::companions::CompanionMode;

/// One thing picked to add to a Conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Addition {
    /// A declared MCP server, by name — the reference a Conversation holds,
    /// looked up in the settings whenever it is needed.
    Server { name: String },

    /// A registered Repo to become a Companion of the work, in the mode the
    /// human chose, off the base they named — `None` being the Repo's default
    /// branch — and, read-write, on the branch named for it.
    Companion {
        repo_id: i64,
        mode: CompanionMode,
        base: Option<String>,
        branch: Option<String>,
    },

    /// A read-only Companion of the work opened up to read-write, on a branch
    /// of its own cut from the base named.
    OpenedUp {
        repo_id: i64,
        base: Option<String>,
        branch: Option<String>,
    },

    /// Another device of the cluster, by its Device Id, the work may be moved
    /// to.
    Device { device: String },
}

impl Addition {
    /// The word the `kind` column holds, spelled out so the table reads as
    /// something.
    fn kind(&self) -> &'static str {
        match self {
            Self::Server { .. } => "server",
            Self::Companion { .. } => "companion",
            Self::OpenedUp { .. } => "opened-up",
            Self::Device { .. } => "device",
        }
    }
}

/// Where an addition has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditionState {
    /// Picked, while the Set waits.
    Pending,

    /// Put on the Conversation, by the Submit that answered the Set.
    Applied,

    /// Never put on: the Set settled some other way, or the addition could not
    /// cross with it.
    Struck,
}

impl AdditionState {
    fn stored(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Applied => "applied",
            Self::Struck => "struck",
        }
    }

    fn read(word: &str) -> Result<Self> {
        Ok(match word {
            "pending" => Self::Pending,
            "applied" => Self::Applied,
            "struck" => Self::Struck,
            other => bail!("an addition on a Question Set is in the unknown state {other:?}"),
        })
    }
}

/// One addition as the record holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetAddition {
    pub id: i64,

    /// The Set it was picked on.
    pub set_id: i64,

    /// The label of the Question it was picked at, or `None` for the set-level
    /// comment box.
    pub label: Option<String>,

    pub addition: Addition,

    pub state: AdditionState,

    /// When it was picked, RFC 3339.
    pub added_at: String,
}

/// What came of a pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddingToSet {
    /// Recorded, and this is the row — or the row already there, where the same
    /// server, Repo or device was picked on this Set before: one Set adds each
    /// once.
    Added(SetAddition),

    /// The Set has been answered.
    Answered,

    /// The Set was locked unanswered.
    Locked,
}

/// And of taking one off again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakingOff {
    /// Gone, whether or not there was one: the state the × asked for.
    TakenOff,
    Answered,
    Locked,
}

/// The table the rows live in.
///
/// `conversation_id` beside `set_id`, the way a file on an Answer carries both:
/// what these are additions *to* is the Conversation, and the walks that delete
/// and carry a Conversation find them by it.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS set_additions (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id),
             set_id          INTEGER NOT NULL REFERENCES question_sets(id),
             label           TEXT,
             kind            TEXT NOT NULL,
             server          TEXT,
             repo_id         INTEGER REFERENCES repos(id),
             mode            TEXT,
             base            TEXT,
             branch          TEXT,
             device          TEXT,
             state           TEXT NOT NULL,
             added_at        TEXT NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the set_additions table")?;

    Ok(())
}

/// The columns every read takes, in the order [`Row`] holds them.
const COLUMNS: &str =
    "id, set_id, label, kind, server, repo_id, mode, base, branch, device, state, added_at";

type Row = (
    i64,
    i64,
    Option<String>,
    String,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
);

/// A row read back. A kind this does not know, or one missing what its kind
/// names, is a database written by a Verkstead this one does not understand.
fn made(row: Row) -> Result<SetAddition> {
    let (id, set_id, label, kind, server, repo_id, mode, base, branch, device, state, added_at) =
        row;

    let addition = match (kind.as_str(), server, repo_id, device) {
        ("server", Some(name), _, _) => Addition::Server { name },
        ("companion", _, Some(repo_id), _) => Addition::Companion {
            repo_id,
            mode: CompanionMode::read(mode.as_deref().unwrap_or_default())?,
            base,
            branch,
        },
        ("opened-up", _, Some(repo_id), _) => Addition::OpenedUp {
            repo_id,
            base,
            branch,
        },
        ("device", _, _, Some(device)) => Addition::Device { device },
        (other, ..) => bail!("an addition on Question Set {set_id} is an unreadable {other:?}"),
    };

    Ok(SetAddition {
        id,
        set_id,
        label,
        addition,
        state: AdditionState::read(&state)?,
        added_at,
    })
}

/// What a pick is told apart from the others on its Set by.
enum Key<'a> {
    Text(&'a str),
    Id(i64),
}

/// Whether a Set has settled, asked inside the write it guards.
async fn settled(tx: &mut sqlx::SqliteConnection, set_id: i64) -> Result<Option<bool>> {
    let (answered, locked): (bool, bool) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM responses WHERE set_id = ?1),
                EXISTS (SELECT 1 FROM archivings WHERE set_id = ?1)",
    )
    .bind(set_id)
    .fetch_one(&mut *tx)
    .await
    .with_context(|| format!("reading whether Question Set {set_id} has settled"))?;

    Ok(match (answered, locked) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    })
}

/// Record a pick on a waiting Set, pending.
///
/// Which Conversation the Set was asked from, whether the label is one it asks
/// and whether the thing picked exists are the caller's to have asked: this
/// takes the row as it stands, and refuses only a Set that has settled.
pub async fn add_to_set(
    pool: &SqlitePool,
    conversation_id: i64,
    set_id: i64,
    label: Option<&str>,
    addition: &Addition,
) -> Result<AddingToSet> {
    let mut tx = super::writing(pool, "adding to a Question Set").await?;

    match settled(&mut tx, set_id).await? {
        Some(true) => return Ok(AddingToSet::Answered),
        Some(false) => return Ok(AddingToSet::Locked),
        None => {}
    }

    // One Set adds a thing once, whichever Question it was picked at: a server
    // by its name, a Repo — added or opened up — by its id, a device by its
    // Device Id. The second pick is the first one's row rather than a second
    // chip saying the same thing, or two saying different things about the one
    // Repo.
    let (same, key): (&str, Key) = match addition {
        Addition::Server { name } => ("kind = 'server' AND server = ?", Key::Text(name)),
        Addition::Companion { repo_id, .. } | Addition::OpenedUp { repo_id, .. } => (
            "kind IN ('companion', 'opened-up') AND repo_id = ?",
            Key::Id(*repo_id),
        ),
        Addition::Device { device } => ("kind = 'device' AND device = ?", Key::Text(device)),
    };

    let query = format!("SELECT {COLUMNS} FROM set_additions WHERE set_id = ? AND {same}");
    let query = sqlx::query_as::<_, Row>(&query).bind(set_id);
    let query = match key {
        Key::Text(text) => query.bind(text),
        Key::Id(id) => query.bind(id),
    };

    let there = query
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("looking for {addition:?} on Question Set {set_id}"))?;

    if let Some(there) = there {
        return Ok(AddingToSet::Added(made(there)?));
    }

    let (server, repo_id, mode, base, branch, device) = match addition {
        Addition::Server { name } => (Some(name.as_str()), None, None, None, None, None),
        Addition::Companion {
            repo_id,
            mode,
            base,
            branch,
        } => (
            None,
            Some(*repo_id),
            Some(mode.stored()),
            base.as_deref(),
            branch.as_deref(),
            None,
        ),
        Addition::OpenedUp {
            repo_id,
            base,
            branch,
        } => (
            None,
            Some(*repo_id),
            None,
            base.as_deref(),
            branch.as_deref(),
            None,
        ),
        Addition::Device { device } => (None, None, None, None, None, Some(device.as_str())),
    };

    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO set_additions
             (conversation_id, set_id, label, kind, server, repo_id, mode, base, branch,
              device, state, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         RETURNING {COLUMNS}"
    ))
    .bind(conversation_id)
    .bind(set_id)
    .bind(label.map(str::trim))
    .bind(addition.kind())
    .bind(server)
    .bind(repo_id)
    .bind(mode)
    .bind(base)
    .bind(branch)
    .bind(device)
    .bind(AdditionState::Pending.stored())
    .fetch_one(&mut *tx)
    .await
    .with_context(|| format!("adding to Question Set {set_id}"))?;

    let added = made(row)?;

    tx.commit().await.context("adding to a Question Set")?;

    Ok(AddingToSet::Added(added))
}

/// And take a pick off a waiting Set again, scoped to that Set: a row picked on
/// another is not this press's to take.
pub async fn take_off_set(pool: &SqlitePool, set_id: i64, id: i64) -> Result<TakingOff> {
    let mut tx = super::writing(pool, "taking an addition off a Question Set").await?;

    match settled(&mut tx, set_id).await? {
        Some(true) => return Ok(TakingOff::Answered),
        Some(false) => return Ok(TakingOff::Locked),
        None => {}
    }

    sqlx::query("DELETE FROM set_additions WHERE set_id = ? AND id = ?")
        .bind(set_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("taking addition {id} off Question Set {set_id}"))?;

    tx.commit()
        .await
        .context("taking an addition off a Question Set")?;

    Ok(TakingOff::TakenOff)
}

/// Put a Set's pending additions on its Conversation, inside the
/// transaction that stores its Response — so the Answers and what came with
/// them land together or not at all, and a Set is applied once because it is
/// answered once.
///
/// A server is attached by name although the round is no longer drafting,
/// which is the second moment ADR-0028 adds beside ADR-0021's; a device is
/// ticked exactly as *May be transferred to* ticks one. Either may be on the
/// Conversation already — the Steer, another Set — and that is the state the
/// pick asked for. Whether the server is still declared is the server's to
/// have asked before this, the declarations being in a file this crate has
/// never heard of.
///
/// A companion joins the work and an opened-up one turns read-write, past
/// drafting, as a steer's do — see [`super::companions::join`] — with the
/// checkouts the server has already made for them recorded beside: `checkouts`
/// is where each went, and is empty for a Conversation with nothing checked out
/// yet, whose grill start makes them. Each applied row of a read-write
/// companion keeps the branch it was cut, mirroring resolved, so the record
/// says what the Timeline and the agent are told.
pub(crate) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    set_id: i64,
    checkouts: &[super::CompanionWorktree],
) -> Result<()> {
    sqlx::query(
        "INSERT INTO conversation_mcp_servers (conversation_id, name, attached_at)
         SELECT conversation_id, server, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         FROM set_additions
         WHERE set_id = ? AND kind = 'server' AND state = 'pending'
         ORDER BY id
         ON CONFLICT (conversation_id, name) DO NOTHING",
    )
    .bind(set_id)
    .execute(&mut **tx)
    .await
    .with_context(|| format!("attaching the servers added on Question Set {set_id}"))?;

    sqlx::query(
        "INSERT INTO permitted_devices (conversation_id, device)
         SELECT conversation_id, device
         FROM set_additions
         WHERE set_id = ? AND kind = 'device' AND state = 'pending'
         ORDER BY id
         ON CONFLICT (conversation_id, device) DO NOTHING",
    )
    .bind(set_id)
    .execute(&mut **tx)
    .await
    .with_context(|| format!("permitting the devices added on Question Set {set_id}"))?;

    sqlx::query(
        "INSERT INTO companions (conversation_id, repo_id, mode, base_ref, branch)
         SELECT conversation_id, repo_id, mode, base, COALESCE(branch, '')
         FROM set_additions
         WHERE set_id = ? AND kind = 'companion' AND state = 'pending'
         ORDER BY id
         ON CONFLICT (conversation_id, repo_id) DO NOTHING",
    )
    .bind(set_id)
    .execute(&mut **tx)
    .await
    .with_context(|| format!("putting the Repos added on Question Set {set_id} alongside"))?;

    sqlx::query(
        "UPDATE companions
         SET mode = ?, branch = COALESCE((SELECT branch FROM set_additions
                                          WHERE set_additions.set_id = ?
                                            AND set_additions.kind = 'opened-up'
                                            AND set_additions.state = 'pending'
                                            AND set_additions.repo_id = companions.repo_id),
                                         '')
         WHERE (conversation_id, repo_id) IN
               (SELECT conversation_id, repo_id FROM set_additions
                WHERE set_id = ? AND kind = 'opened-up' AND state = 'pending')",
    )
    .bind(CompanionMode::ReadWrite.stored())
    .bind(set_id)
    .bind(set_id)
    .execute(&mut **tx)
    .await
    .with_context(|| format!("opening up the Repos opened on Question Set {set_id}"))?;

    // And mirroring resolved on the Set's own rows, the companions' keeping it
    // as mirroring: a read-write companion with no branch of its own is cut
    // the Conversation's.
    sqlx::query(
        "UPDATE set_additions
         SET branch = (SELECT branch FROM conversations
                       WHERE conversations.id = set_additions.conversation_id)
         WHERE set_id = ? AND state = 'pending' AND branch IS NULL
           AND (kind = 'opened-up' OR (kind = 'companion' AND mode = ?))",
    )
    .bind(set_id)
    .bind(CompanionMode::ReadWrite.stored())
    .execute(&mut **tx)
    .await
    .with_context(|| format!("naming the branches added on Question Set {set_id}"))?;

    if let Some(conversation_id) = sqlx::query_scalar::<_, i64>(
        "SELECT conversation_id FROM set_additions WHERE set_id = ? LIMIT 1",
    )
    .bind(set_id)
    .fetch_optional(&mut **tx)
    .await
    .with_context(|| format!("reading which Conversation Question Set {set_id} adds to"))?
    {
        super::companions::record_worktrees(tx, conversation_id, checkouts).await?;
    }

    sqlx::query(
        "UPDATE set_additions SET state = 'applied'
         WHERE set_id = ? AND state = 'pending'",
    )
    .bind(set_id)
    .execute(&mut **tx)
    .await
    .with_context(|| format!("recording what Question Set {set_id} added as applied"))?;

    Ok(())
}

/// Every addition picked on one Set, in the order they were picked.
pub async fn set_additions(pool: &SqlitePool, set_id: i64) -> Result<Vec<SetAddition>> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM set_additions WHERE set_id = ? ORDER BY id"
    ))
    .bind(set_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("reading the additions picked on Question Set {set_id}"))?;

    rows.into_iter().map(made).collect()
}

/// Every addition picked on any Set of one Conversation, in the order they were
/// picked: what the Timeline reads once for all of its Sets.
pub async fn conversation_additions(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Vec<SetAddition>> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM set_additions WHERE conversation_id = ? ORDER BY id"
    ))
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .with_context(|| {
        format!(
            "reading the additions picked on the Question Sets of Conversation {conversation_id}"
        )
    })?;

    rows.into_iter().map(made).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The device every Conversation started here is ranked by, named the way a
    /// cluster names one (ADR-0020, *Ranks*).
    const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

    /// A Conversation with one waiting Set on its Timeline, over a database
    /// with nothing else in it.
    async fn waiting() -> (tempfile::TempDir, SqlitePool, i64, i64) {
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
        let conversation = crate::start_conversation(&pool, repo.id, "adding", THIS_DEVICE, None)
            .await
            .unwrap()
            .unwrap();

        let set = verkstead_schema::QuestionSet::from_yaml(
            "title: the rate limiter\n\
             questions:\n\
             \x20 - label: Q1\n\
             \x20   text: where does the counter live?\n",
        )
        .unwrap();
        let set = crate::ask(&pool, conversation, &set, crate::Ask::Blocking)
            .await
            .unwrap()
            .unwrap()
            .id;

        (dir, pool, conversation, set)
    }

    fn server(name: &str) -> Addition {
        Addition::Server {
            name: name.to_owned(),
        }
    }

    fn added(adding: AddingToSet) -> SetAddition {
        match adding {
            AddingToSet::Added(added) => added,
            other => panic!("expected the pick recorded, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_pick_is_recorded_pending_under_its_label() {
        let (_dir, pool, conversation, set) = waiting().await;

        let put = added(
            add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
                .await
                .unwrap(),
        );

        assert_eq!(put.label.as_deref(), Some("Q1"));
        assert_eq!(put.addition, server("docs"));
        assert_eq!(put.state, AdditionState::Pending);
        assert_eq!(set_additions(&pool, set).await.unwrap(), vec![put]);
    }

    /// The comment box's picks name no Question.
    #[tokio::test]
    async fn a_pick_at_the_comment_box_names_no_question() {
        let (_dir, pool, conversation, set) = waiting().await;

        let put = added(
            add_to_set(&pool, conversation, set, None, &server("docs"))
                .await
                .unwrap(),
        );

        assert_eq!(put.label, None);
    }

    /// Every kind reads back as it was written, which is what the later picks
    /// stand on.
    #[tokio::test]
    async fn every_kind_reads_back_as_it_was_written() {
        let (_dir, pool, conversation, set) = waiting().await;

        // A Repo apiece, one Set adding each Repo once.
        let mut repos = Vec::new();
        for name in ["docs", "infra", "website"] {
            let path = std::path::PathBuf::from(format!("/srv/{name}"));
            repos.push(
                crate::register_repo(&pool, &path, name, "main")
                    .await
                    .unwrap()
                    .unwrap()
                    .id,
            );
        }

        let kinds = [
            server("docs"),
            Addition::Companion {
                repo_id: repos[0],
                mode: CompanionMode::ReadWrite,
                base: Some("develop".to_owned()),
                branch: Some("adding".to_owned()),
            },
            Addition::Companion {
                repo_id: repos[1],
                mode: CompanionMode::ReadOnly,
                base: None,
                branch: None,
            },
            Addition::OpenedUp {
                repo_id: repos[2],
                base: None,
                branch: Some("adding".to_owned()),
            },
            Addition::Device {
                device: "22222222222222222222222222222222".to_owned(),
            },
        ];

        for kind in &kinds {
            add_to_set(&pool, conversation, set, Some("Q1"), kind)
                .await
                .unwrap();
        }

        let read: Vec<Addition> = set_additions(&pool, set)
            .await
            .unwrap()
            .into_iter()
            .map(|one| one.addition)
            .collect();

        assert_eq!(read, kinds);
    }

    /// One Set puts a server on once, whichever Question it was picked at.
    #[tokio::test]
    async fn the_same_server_twice_is_one_row() {
        let (_dir, pool, conversation, set) = waiting().await;

        let first = added(
            add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
                .await
                .unwrap(),
        );
        let second = added(
            add_to_set(&pool, conversation, set, None, &server("docs"))
                .await
                .unwrap(),
        );

        assert_eq!(first, second);
        assert_eq!(set_additions(&pool, set).await.unwrap().len(), 1);
    }

    /// And a Repo once, whether it was picked to add or to open up, and a
    /// device once: the one Repo with two chips would be two things said about
    /// it.
    #[tokio::test]
    async fn the_same_repo_or_device_twice_is_one_row() {
        let (_dir, pool, conversation, set) = waiting().await;

        let first = added(
            add_to_set(
                &pool,
                conversation,
                set,
                Some("Q1"),
                &Addition::Companion {
                    repo_id: 1,
                    mode: CompanionMode::ReadOnly,
                    base: None,
                    branch: None,
                },
            )
            .await
            .unwrap(),
        );
        let second = added(
            add_to_set(
                &pool,
                conversation,
                set,
                None,
                &Addition::OpenedUp {
                    repo_id: 1,
                    base: None,
                    branch: None,
                },
            )
            .await
            .unwrap(),
        );
        assert_eq!(first, second);

        let device = Addition::Device {
            device: "22222222222222222222222222222222".to_owned(),
        };
        let ticked = added(
            add_to_set(&pool, conversation, set, Some("Q1"), &device)
                .await
                .unwrap(),
        );
        let again = added(
            add_to_set(&pool, conversation, set, None, &device)
                .await
                .unwrap(),
        );
        assert_eq!(ticked, again);

        assert_eq!(set_additions(&pool, set).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn taking_one_off_leaves_the_rest() {
        let (_dir, pool, conversation, set) = waiting().await;

        let docs = added(
            add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
                .await
                .unwrap(),
        );
        add_to_set(&pool, conversation, set, Some("Q1"), &server("tickets"))
            .await
            .unwrap();

        assert_eq!(
            take_off_set(&pool, set, docs.id).await.unwrap(),
            TakingOff::TakenOff
        );

        let left: Vec<Addition> = set_additions(&pool, set)
            .await
            .unwrap()
            .into_iter()
            .map(|one| one.addition)
            .collect();
        assert_eq!(left, vec![server("tickets")]);
    }

    /// An answered Set refuses both, and keeps what it had.
    #[tokio::test]
    async fn an_answered_set_refuses_adding_and_taking_off() {
        let (_dir, pool, conversation, set) = waiting().await;

        let docs = added(
            add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
                .await
                .unwrap(),
        );

        crate::insert_response(
            &pool,
            set,
            &verkstead_schema::Response {
                answers: vec![verkstead_schema::Answer {
                    label: "Q1".to_owned(),
                    selected: None,
                    free_text: Some("in Redis".to_owned()),
                    unanswered: false,
                    attachments: Vec::new(),
                    added: Vec::new(),
                }],
                comment: None,
                direction: None,
                nothing_else: false,
                added: Vec::new(),
            },
        )
        .await
        .unwrap();

        assert_eq!(
            add_to_set(&pool, conversation, set, Some("Q1"), &server("tickets"))
                .await
                .unwrap(),
            AddingToSet::Answered
        );
        assert_eq!(
            take_off_set(&pool, set, docs.id).await.unwrap(),
            TakingOff::Answered
        );
        // Kept, and applied by the answer that settled the Set.
        assert_eq!(
            set_additions(&pool, set).await.unwrap(),
            vec![SetAddition {
                state: AdditionState::Applied,
                ..docs
            }]
        );
    }

    /// And a locked one the same way, said as the lock.
    #[tokio::test]
    async fn a_locked_set_refuses_adding_and_taking_off() {
        let (_dir, pool, conversation, set) = waiting().await;

        let docs = added(
            add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
                .await
                .unwrap(),
        );

        crate::lock_set(&pool, &crate::Settlements::new(4), set)
            .await
            .unwrap();

        assert_eq!(
            add_to_set(&pool, conversation, set, Some("Q1"), &server("tickets"))
                .await
                .unwrap(),
            AddingToSet::Locked
        );
        assert_eq!(
            take_off_set(&pool, set, docs.id).await.unwrap(),
            TakingOff::Locked
        );
    }

    /// A pick is the Set's and nothing of the Conversation's: its servers are
    /// as they were.
    #[tokio::test]
    async fn a_pick_puts_nothing_on_the_conversation() {
        let (_dir, pool, conversation, set) = waiting().await;

        add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
            .await
            .unwrap();

        assert!(
            crate::mcp_servers(&pool, conversation)
                .await
                .unwrap()
                .is_empty()
        );
    }

    /// The Response a one-Question Set is answered with.
    fn answer() -> verkstead_schema::Response {
        verkstead_schema::Response {
            answers: vec![verkstead_schema::Answer {
                label: "Q1".to_owned(),
                selected: None,
                free_text: Some("in Redis".to_owned()),
                unanswered: false,
                attachments: Vec::new(),
                added: Vec::new(),
            }],
            ..verkstead_schema::Response::default()
        }
    }

    /// Answering puts the servers, the ticks and the companions on the
    /// Conversation, in the Response's own transaction.
    #[tokio::test]
    async fn answering_applies_every_kind() {
        let (_dir, pool, conversation, set) = waiting().await;
        let laptop = "22222222222222222222222222222222";
        let docs = crate::register_repo(&pool, std::path::Path::new("/srv/docs"), "docs", "main")
            .await
            .unwrap()
            .unwrap()
            .id;

        add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
            .await
            .unwrap();
        add_to_set(
            &pool,
            conversation,
            set,
            None,
            &Addition::Device {
                device: laptop.to_owned(),
            },
        )
        .await
        .unwrap();
        add_to_set(
            &pool,
            conversation,
            set,
            Some("Q1"),
            &Addition::Companion {
                repo_id: docs,
                mode: CompanionMode::ReadOnly,
                base: None,
                branch: None,
            },
        )
        .await
        .unwrap();

        let checkouts = [crate::CompanionWorktree {
            repo_id: docs,
            path: std::path::PathBuf::from("/srv/worktrees/docs-main"),
            base_commit: Some("abc123".to_owned()),
        }];
        crate::insert_response_with_checkouts(&pool, set, &answer(), &checkouts)
            .await
            .unwrap()
            .unwrap();

        let companions = crate::companions(&pool, conversation).await.unwrap();
        assert_eq!(companions.len(), 1);
        assert_eq!(companions[0].repo.id, docs);
        assert_eq!(companions[0].mode, CompanionMode::ReadOnly);
        assert_eq!(
            companions[0].worktree.as_deref(),
            Some(std::path::Path::new("/srv/worktrees/docs-main")),
            "and where it was checked out is on the record beside it",
        );

        assert_eq!(
            crate::mcp_servers(&pool, conversation).await.unwrap(),
            vec!["docs".to_owned()]
        );
        assert_eq!(
            crate::permitted_devices(&pool, conversation).await.unwrap(),
            vec![laptop.to_owned()]
        );

        let states: Vec<AdditionState> = set_additions(&pool, set)
            .await
            .unwrap()
            .into_iter()
            .map(|one| one.state)
            .collect();
        assert_eq!(
            states,
            vec![
                AdditionState::Applied,
                AdditionState::Applied,
                AdditionState::Applied
            ]
        );
    }

    /// A read-write companion left to mirror joins as mirroring, while the
    /// Set's record names the branch it was cut; an opened-up one turns
    /// read-write on the branch it was given.
    #[tokio::test]
    async fn answering_names_the_branches_and_opens_up() {
        let (_dir, pool, conversation, set) = waiting().await;
        let mut repos = Vec::new();
        for name in ["docs", "infra"] {
            let path = std::path::PathBuf::from(format!("/srv/{name}"));
            repos.push(
                crate::register_repo(&pool, &path, name, "main")
                    .await
                    .unwrap()
                    .unwrap()
                    .id,
            );
        }
        crate::add_companion(&pool, conversation, repos[1])
            .await
            .unwrap();

        add_to_set(
            &pool,
            conversation,
            set,
            Some("Q1"),
            &Addition::Companion {
                repo_id: repos[0],
                mode: CompanionMode::ReadWrite,
                base: None,
                branch: None,
            },
        )
        .await
        .unwrap();
        add_to_set(
            &pool,
            conversation,
            set,
            None,
            &Addition::OpenedUp {
                repo_id: repos[1],
                base: None,
                branch: Some("infra-adding".to_owned()),
            },
        )
        .await
        .unwrap();

        crate::insert_response(&pool, set, &answer())
            .await
            .unwrap()
            .unwrap();

        let companions = crate::companions(&pool, conversation).await.unwrap();
        let docs = companions
            .iter()
            .find(|one| one.repo.id == repos[0])
            .unwrap();
        assert_eq!(docs.mode, CompanionMode::ReadWrite);
        assert_eq!(
            docs.branch, "",
            "mirroring stays mirroring on the companion"
        );
        let infra = companions
            .iter()
            .find(|one| one.repo.id == repos[1])
            .unwrap();
        assert_eq!(infra.mode, CompanionMode::ReadWrite);
        assert_eq!(infra.branch, "infra-adding");

        let read = set_additions(&pool, set).await.unwrap();
        assert_eq!(
            read[0].addition,
            Addition::Companion {
                repo_id: repos[0],
                mode: CompanionMode::ReadWrite,
                base: None,
                branch: Some("adding".to_owned()),
            },
            "the Set's record names the branch mirroring cut",
        );
        assert_eq!(read[1].state, AdditionState::Applied);
    }

    /// A server the Conversation holds already is the state the pick asked for.
    #[tokio::test]
    async fn a_server_on_already_is_applied_all_the_same() {
        let (_dir, pool, conversation, set) = waiting().await;

        add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
            .await
            .unwrap();
        crate::attach_mcp_server(&pool, conversation, "docs")
            .await
            .unwrap();

        crate::insert_response(&pool, set, &answer())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            crate::mcp_servers(&pool, conversation).await.unwrap(),
            vec!["docs".to_owned()]
        );
        assert_eq!(
            set_additions(&pool, set).await.unwrap()[0].state,
            AdditionState::Applied
        );
    }

    /// A lock puts nothing on: the picks stay as they were, unapplied.
    #[tokio::test]
    async fn a_lock_applies_nothing() {
        let (_dir, pool, conversation, set) = waiting().await;

        add_to_set(&pool, conversation, set, Some("Q1"), &server("docs"))
            .await
            .unwrap();

        crate::lock_set(&pool, &crate::Settlements::new(4), set)
            .await
            .unwrap();

        assert!(
            crate::mcp_servers(&pool, conversation)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            set_additions(&pool, set).await.unwrap()[0].state,
            AdditionState::Pending
        );
    }
}
