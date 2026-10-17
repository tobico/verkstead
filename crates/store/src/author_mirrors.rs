//! The **mirror rows** of the Authors: one row per Author a member of this
//! cluster holds at home, marked with that device (ADR-0025, the brief's
//! *Decisions in force*).
//!
//! **A row in the database rather than an entry in `config.yaml`**, because a
//! mirror is found rather than told: `config.yaml` is what Verkstead was told,
//! and what a member last answered is nothing anybody typed here. So this
//! device's own Authors stay in its settings files and everybody else's are
//! here, and the two are drawn as one list.
//!
//! **Keyed by the uuid**, which is the same on every device: a Repo records an
//! Author by it whichever machine the Author is at home on, so there is no local
//! id here and nothing that reads one. What the row holds besides is what is
//! drawn — the display name, the git name, the email — and whether the home
//! device holds a token for it. Never the token: that crosses on a call of its
//! own, into `secrets.yaml`.
//!
//! **Written whole per member**: what a member answered is every Author it holds
//! at home, so a refresh is its rows replaced — the ones it still names
//! rewritten, the ones it no longer names gone. See [`record_author_mirrors`].

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// The table the mirrors live in, one row per Author a member holds at home.
///
/// Hanging off nothing, for the reason a Profile mirror's home device is a
/// plain column: the membership a row's device is pruned against comes and goes
/// on its own terms — see [`forget_author_mirrors_of_departed`].
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS author_mirrors (
             id           TEXT PRIMARY KEY,
             home_device  TEXT NOT NULL,
             display_name TEXT NOT NULL,
             name         TEXT NOT NULL,
             email        TEXT NOT NULL,
             holds_token  INTEGER NOT NULL
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the author_mirrors table")?;

    Ok(())
}

/// One member's Author, as this device writes it down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorMirror {
    /// Its uuid, which is its uuid on every device.
    pub id: String,

    /// The Device Id of the member it is at home on.
    pub device: String,

    /// What it is called in rows and pickers, empty where nobody named it and
    /// the git name stands in.
    pub display_name: String,

    pub name: String,
    pub email: String,

    /// Whether the home device holds a GitHub token for it.
    pub holds_token: bool,
}

type Row = (String, String, String, String, String, bool);

fn mirror((id, device, display_name, name, email, holds_token): Row) -> AuthorMirror {
    AuthorMirror {
        id,
        device,
        display_name,
        name,
        email,
        holds_token,
    }
}

/// Every Author mirror there is, by device and then by what each is called.
pub async fn author_mirrors(pool: &SqlitePool) -> Result<Vec<AuthorMirror>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, home_device, display_name, name, email, holds_token
         FROM author_mirrors
         ORDER BY home_device,
                  CASE WHEN display_name = '' THEN name ELSE display_name END COLLATE NOCASE,
                  id",
    )
    .fetch_all(pool)
    .await
    .context("reading the Authors of this device's members")?;

    Ok(rows.into_iter().map(mirror).collect())
}

/// Write down everything `device` answered as its own Authors: each row
/// upserted by uuid, and every mirror of that device it no longer names taken
/// away.
///
/// **What an Author removed at home comes to here**: the next refresh finds it
/// gone from what that device answers, and its row goes.
///
/// Whether anything moved comes back, which is whether the pages have anything
/// new to draw — every refresh but the one after an edit somewhere is the same
/// rows written over themselves.
pub async fn record_author_mirrors(
    pool: &SqlitePool,
    device: &str,
    at_home: &[AuthorMirror],
) -> Result<bool> {
    let mut tx = super::writing(pool, "writing down a member's Authors").await?;

    let before: Vec<Row> = sqlx::query_as(
        "SELECT id, home_device, display_name, name, email, holds_token
         FROM author_mirrors
         WHERE home_device = ?
         ORDER BY id",
    )
    .bind(device)
    .fetch_all(&mut *tx)
    .await
    .with_context(|| format!("reading the Authors of {device} as they were"))?;

    sqlx::query("DELETE FROM author_mirrors WHERE home_device = ?")
        .bind(device)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("clearing the Authors of {device}"))?;

    for author in at_home {
        // Keyed by the uuid alone: an Author that has moved home is the one row
        // with another device on it, never two rows for one Author.
        sqlx::query(
            "INSERT INTO author_mirrors (id, home_device, display_name, name, email, holds_token)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 home_device = excluded.home_device,
                 display_name = excluded.display_name,
                 name = excluded.name,
                 email = excluded.email,
                 holds_token = excluded.holds_token",
        )
        .bind(&author.id)
        .bind(device)
        .bind(&author.display_name)
        .bind(&author.name)
        .bind(&author.email)
        .bind(author.holds_token)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("writing down Author {} of {device}", author.id))?;
    }

    let mut after: Vec<AuthorMirror> = at_home
        .iter()
        .map(|author| AuthorMirror {
            device: device.to_owned(),
            ..author.clone()
        })
        .collect();
    after.sort_by(|one, other| one.id.cmp(&other.id));

    tx.commit()
        .await
        .with_context(|| format!("writing down the Authors of {device}"))?;

    Ok(before.into_iter().map(mirror).collect::<Vec<_>>() != after)
}

/// And every Author mirror of a device that is no longer a member of this
/// cluster, taken away — the membership read inside the delete, for the reason
/// the Profile mirrors' own prune reads it there. A member that is merely not
/// answering keeps its rows.
///
/// The uuids that went come back, for whatever else was held under them.
pub async fn forget_author_mirrors_of_departed(pool: &SqlitePool) -> Result<Vec<String>> {
    let gone: Vec<(String,)> = sqlx::query_as(
        "DELETE FROM author_mirrors
         WHERE home_device NOT IN (SELECT device FROM members)
         RETURNING id",
    )
    .fetch_all(pool)
    .await
    .context("forgetting the Authors of devices this one is no longer linked to")?;

    Ok(gone.into_iter().map(|(id,)| id).collect())
}
