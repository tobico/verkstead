//! What a Repo was last grilled with: its Pairings, remembered against the
//! Repo so the next Conversation started on it arrives with every picker
//! already filled.
//!
//! Server-side rather than in the browser, because the workbench is answered
//! from a phone as readily as from a desk: a memory kept in one browser's
//! storage would be a memory the other device does not have.
//!
//! One row per Repo per role, written at grill start and replacing whatever was
//! there — the last Pairing a Repo was actually grilled with is the whole of
//! what this remembers, and a history of the ones before it would be a list
//! nothing reads.
//!
//! Nothing here is a promise that a remembered Pairing still runs. A Profile's
//! pair can be moved out from under it and its model list can be retyped, and
//! neither touches this table: what the read hands back is what was written,
//! and whether it is still something to launch a session under is judged above
//! the store, where the boundary and the Profile's list are both read.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::Picked;
use super::conversations::Role;

/// What a Repo was last grilled with, as far as the store can still stand
/// behind it.
///
/// Any of them is [`Picked::Nothing`] where nothing was remembered for that
/// role, or where what was remembered names a Profile that has since gone.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepoPairings {
    pub grilling: Picked,
    pub implementation: Picked,
    pub review: Picked,
}

/// The two tables the memory lives in.
///
/// Both keyed by Repo and role, so remembering again replaces rather than
/// accumulates. `model` is NOT NULL because a half-remembered Pairing is
/// nothing to prefill a picker with: both halves are chosen in one press, and
/// both are remembered together or not at all.
///
/// A role picked away is remembered in the second of them, for the reason a
/// Conversation's own skip is kept apart from its Profile column: a skip is not
/// a Profile, and a row that had to invent one to say *no session* would be a
/// memory nothing could prefill honestly. Writing either one takes the other
/// away, in the one transaction — see [`remember`] — so the two cannot disagree
/// about what a Repo was last grilled with.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS repo_pairings (
             repo_id    INTEGER NOT NULL REFERENCES repos(id),
             role       TEXT NOT NULL,
             profile_id INTEGER NOT NULL REFERENCES profiles(id),
             model      TEXT NOT NULL,
             PRIMARY KEY (repo_id, role)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the repo pairings table")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS repo_skips (
             repo_id INTEGER NOT NULL REFERENCES repos(id),
             role    TEXT NOT NULL,
             PRIMARY KEY (repo_id, role)
         ) STRICT",
    )
    .execute(pool)
    .await
    .context("creating the repo skips table")?;

    Ok(())
}

/// Remember what a Conversation is being grilled with, against its Repo.
///
/// Copied off the Conversation inside the transaction that moves it rather than
/// passed in, so the memory is the Pairings the grilling actually started under
/// and there is no second reading of them to disagree with the first.
///
/// A role whose Pairing has no model — a Profile chosen before pairings existed
/// — is not remembered: the join drops it, and the row that was there stays as
/// it was. Prefilling a picker with half a choice would be worse than leaving
/// it empty.
///
/// A role the human picked away is remembered as that, and the two writes are a
/// pair: whichever of them lands takes the other's row away, so what comes back
/// is one answer rather than two to choose between.
pub(crate) async fn remember(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    conversation_id: i64,
) -> Result<()> {
    for role in Role::ALL {
        sqlx::query(&format!(
            "INSERT INTO repo_pairings (repo_id, role, profile_id, model)
             SELECT c.repo_id, ?, c.{}, m.model
             FROM conversations c
             JOIN pairing_models m
               ON m.conversation_id = c.id AND m.role = ?
             WHERE c.id = ? AND c.{} IS NOT NULL
             ON CONFLICT (repo_id, role)
             DO UPDATE SET profile_id = excluded.profile_id, model = excluded.model",
            role.column(),
            role.column(),
        ))
        .bind(role.stored())
        .bind(role.stored())
        .bind(conversation_id)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!("remembering what Conversation {conversation_id} is grilled with")
        })?;

        // Written from the same Conversation and in the same breath: a role it
        // picked away is remembered as picked away, and one it paired is
        // remembered as a pair with whatever skip was there taken off.
        sqlx::query(
            "INSERT INTO repo_skips (repo_id, role)
             SELECT c.repo_id, ?
             FROM conversations c
             JOIN skipped_roles s ON s.conversation_id = c.id AND s.role = ?
             WHERE c.id = ?
             ON CONFLICT (repo_id, role) DO NOTHING",
        )
        .bind(role.stored())
        .bind(role.stored())
        .bind(conversation_id)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!("remembering which roles Conversation {conversation_id} picked away")
        })?;

        sqlx::query(
            "DELETE FROM repo_skips
             WHERE role = ?
               AND repo_id IN (SELECT repo_id FROM conversations WHERE id = ?)
               AND NOT EXISTS (
                     SELECT 1 FROM skipped_roles
                     WHERE conversation_id = ? AND role = ?
                   )",
        )
        .bind(role.stored())
        .bind(conversation_id)
        .bind(conversation_id)
        .bind(role.stored())
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!("forgetting which roles Conversation {conversation_id}'s Repo picked away")
        })?;
    }

    Ok(())
}

/// What the Repo was last grilled with, for the pickers of a Conversation
/// just started on it.
///
/// A row naming a Profile that is not there comes back as [`Picked::Nothing`]
/// rather than as an error. Nothing reachable leaves one — a Profile the
/// Conversation grilled under it still names cannot be removed — so what this
/// guards is a database somebody edited by hand, and it guards it the way the
/// memory fails everywhere: by handing back the unchosen picker a Repo with no
/// memory gets.
pub async fn remembered_pairings(pool: &SqlitePool, repo_id: i64) -> Result<RepoPairings> {
    Ok(RepoPairings {
        grilling: remembered(pool, repo_id, Role::Grilling).await?,
        implementation: remembered(pool, repo_id, Role::Implementation).await?,
        review: remembered(pool, repo_id, Role::Review).await?,
    })
}

/// One role's memory, read as what it says was picked.
///
/// The skip first, for the reason a Conversation's own is read first: the two
/// writes are a pair and a skip is the newer of them wherever both are there.
async fn remembered(pool: &SqlitePool, repo_id: i64, role: Role) -> Result<Picked> {
    let skipped: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM repo_skips WHERE repo_id = ? AND role = ?")
            .bind(repo_id)
            .bind(role.stored())
            .fetch_optional(pool)
            .await
            .with_context(|| format!("reading which roles Repo {repo_id} last picked away"))?;

    if skipped.is_some() {
        return Ok(Picked::Skipped);
    }

    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT profile_id, model FROM repo_pairings WHERE repo_id = ? AND role = ?",
    )
    .bind(repo_id)
    .bind(role.stored())
    .fetch_optional(pool)
    .await
    .with_context(|| format!("reading what Repo {repo_id} was last grilled with"))?;

    let Some((profile_id, model)) = row else {
        return Ok(Picked::Nothing);
    };

    Ok(match super::load_profile(pool, profile_id).await? {
        Some(profile) => Picked::Under(super::Pairing {
            profile,
            model: Some(model),
        }),
        None => Picked::Nothing,
    })
}
