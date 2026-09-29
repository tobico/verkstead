//! A Conversation arriving from another device: the row this device writes for
//! work that was being done somewhere else (ADR-0020, *Transfer*).
//!
//! **A create like any other, with two things given rather than invented.** The
//! id is this device's own, as every id in this database is — the machine the
//! work came from has its own and they collide by construction — and so is the
//! Repo, which the sending device matched against this registry before it sent
//! anything. What arrives fixed is the **Rank**, which carries the device that
//! issued it and is distinct cluster-wide by construction, so the row keeps its
//! place in the merged order; and the **birth key**, which is what every copy of
//! one piece of work says the same and is not this device's to invent. See
//! [`super::births`], and [`super::ranks`].
//!
//! **The Pairings arrive as local ids.** A Profile is named across a cluster by
//! the device it is at home on and its id there, and the device sending the work
//! resolves that pair against this device's own rows before it sends — a mirror
//! being a local id here like any other. So what lands here is three ids of this
//! database's own, exactly as a composer's presses would have left them.
//!
//! **And the row is the first of three legs rather than the whole of a move.** A
//! Conversation lands in the state it was in, under the repository the matching
//! settled, and what fills it follows against the id this device gave it: the
//! **slice**, which is the Timeline and everything hanging off it — see
//! [`super::slices`] — and the **checkout**, which is the branch and the working
//! changes. What the checkout leaves here is [`arrived_checkout`].
//!
//! **Nothing here is refused for the state it is in.** Which states may be moved
//! is the sending device's rule, asked before the press was allowed at all; what
//! this end is looking at is a message from a member, and a Conversation that
//! arrived in a state this device would not have sent is a member one version
//! apart rather than a row to throw away.

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::Birth;
use super::conversations::{Lifecycle, Role};

/// A Conversation as it arrives from the device that was doing the work.
///
/// Everything the row is written from, and nothing about the machine it came
/// off: a path, a worktree or a Profile id of that device's would mean nothing
/// here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    /// The key it was born under, which travels with the work and is written
    /// over the one this create would otherwise stamp.
    pub born: Birth,

    /// The Repo on **this** device the work is in, as the sending device's
    /// matching settled it.
    pub repo_id: i64,

    /// The branch the work is on — the name the checkout is actually on, which
    /// is what the sending device reads off the record it is handing over.
    pub branch: String,

    /// Whether that name is one somebody settled on rather than one Verkstead
    /// invented — see [`super::Conversation::branch_named`].
    pub branch_named: bool,

    /// And whether the first session is still to name it — see
    /// [`super::Conversation::naming`].
    pub naming: bool,

    /// The state the work is in, which is where it goes on from here.
    pub state: Lifecycle,

    /// Its **Rank**, verbatim: the key and the Device Id that issued it,
    /// unchanged, so the row sits where it sat in the merged list.
    pub rank: String,

    /// And what each role runs under, as ids of this device's own.
    pub grilling: ArrivingPicked,
    pub implementation: ArrivingPicked,
    pub review: ArrivingPicked,
}

/// What one role arrives having settled — [`super::Picked`] with a local
/// Profile id in place of the Profile itself.
///
/// The three states rather than an `Option`, because a role picked away is a
/// choice somebody made and an empty picker is one they have not: a copy that
/// could not tell them apart would arrive either refusing to start or starting
/// a session the human said there was to be none of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrivingPicked {
    /// Nothing has been chosen for this role.
    Nothing,

    /// It runs no session at all.
    ///
    /// Grilling and review only: the implementation is the one role that cannot
    /// be picked away, and a message saying otherwise is written down as it
    /// arrived rather than argued with.
    Skipped,

    /// The Profile and model its sessions run under, the Profile by this
    /// device's own id for it.
    Under {
        profile_id: i64,
        model: Option<String>,
    },
}

/// Write the arriving Conversation down, and answer with the id this device
/// gave it.
///
/// `None` is *there is no such Repo on this registry*, which is the one thing
/// this refuses: the id was matched against this device's registry a moment ago
/// and a Repo unregistered since is one no work may be put in. Read in the
/// insert's own `SELECT` rather than before it, exactly as a start reads it —
/// see [`super::conversations`]'s `started`.
///
/// **One transaction.** A Conversation that landed without its birth key would
/// be one the Merged List could not tell from a copy of somebody else's work,
/// and one that landed without its Pairings would be one nothing could be
/// launched in.
pub async fn arrive(pool: &SqlitePool, arriving: &Arrival) -> Result<Option<i64>> {
    let mut tx = super::writing(pool, "taking a Conversation in").await?;

    // When the row was made here, which is now: what the *work* started is the
    // Timeline's to say, and the Timeline travels with the record rather than
    // with the row.
    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO conversations
             (repo_id, created_at, branch, named_branch, naming, base_commit, state, rank)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?, ?, NULL, ?, ?
         FROM repos
         WHERE id = ? AND id NOT IN (SELECT repo_id FROM unregistered_repos)
         RETURNING id",
    )
    .bind(&arriving.branch)
    .bind(arriving.branch_named.then(|| arriving.branch.clone()))
    .bind(arriving.naming)
    .bind(arriving.state.stored())
    .bind(&arriving.rank)
    .bind(arriving.repo_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| {
        format!(
            "taking a Conversation in against Repo {repo}",
            repo = arriving.repo_id,
        )
    })?;

    let Some((id,)) = row else {
        return Ok(None);
    };

    // The key that came with the work, over the one this device would have
    // stamped: what cannot change about a piece of work is where it was born,
    // and this row is a copy of one that was born somewhere else. See
    // [`super::births::record_birth`], which is the same write outside a
    // transaction.
    super::births::stamp(&mut tx, id, &arriving.born).await?;

    for (role, picked) in [
        (Role::Grilling, &arriving.grilling),
        (Role::Implementation, &arriving.implementation),
        (Role::Review, &arriving.review),
    ] {
        match picked {
            ArrivingPicked::Nothing => {}

            ArrivingPicked::Skipped => {
                sqlx::query("INSERT INTO skipped_roles (conversation_id, role) VALUES (?, ?)")
                    .bind(id)
                    .bind(role.stored())
                    .execute(&mut *tx)
                    .await
                    .with_context(|| {
                        format!("writing down that Conversation {id} runs no {role:?} session")
                    })?;
            }

            ArrivingPicked::Under { profile_id, model } => {
                sqlx::query(&format!(
                    "UPDATE conversations SET {} = ? WHERE id = ?",
                    role.column(),
                ))
                .bind(profile_id)
                .bind(id)
                .execute(&mut *tx)
                .await
                .with_context(|| {
                    format!("writing down the Profile Conversation {id} runs {role:?} under")
                })?;

                // The model where one was paired. A Pairing made before there
                // were models to pair carries none, and it arrives carrying
                // none: what such a session runs on is the Profile's own first
                // model, which is that Profile's over here — see
                // [`super::Pairing::runs_on`].
                if let Some(model) = model {
                    sqlx::query(
                        "INSERT INTO pairing_models (conversation_id, role, model)
                         VALUES (?, ?, ?)",
                    )
                    .bind(id)
                    .bind(role.stored())
                    .bind(model)
                    .execute(&mut *tx)
                    .await
                    .with_context(|| {
                        format!("writing down the model Conversation {id} runs {role:?} on")
                    })?;
                }
            }
        }
    }

    tx.commit().await.context("taking a Conversation in")?;

    Ok(Some(id))
}

/// Where the work that arrived was checked out here, and what its branch was cut
/// from.
///
/// **Written after the checkout rather than with the row**, because the two are
/// two legs of one move: the row is what the far end numbers and the branch is
/// what it numbers *against* — a Worktree hangs off a Conversation, and there
/// has to be one here for it to hang from. See the server's
/// `peer::checkouts`, which is the leg that makes the directory.
///
/// **The path is this device's own** and nothing that arrived: a Worktree is a
/// directory on one machine, so it is never in a slice and never on the wire —
/// the device taking the work in names its own, under its own Data Directory.
///
/// **And the base travels** where the Worktree does not, being a fact about the
/// work rather than about a machine: what reads it is the commit sweep, which
/// leaves out everything the base already holds. A copy that landed without it
/// would report the history under the branch as this Conversation's own.
///
/// One transaction, the three being one statement about where the work now is.
pub async fn arrived_checkout(
    pool: &SqlitePool,
    id: i64,
    worktree: &std::path::Path,
    base_commit: Option<&str>,
    base_ref: Option<&str>,
) -> Result<()> {
    let mut tx = super::writing(pool, "recording an arriving checkout").await?;

    // Written over whatever is there rather than inserted, the way a start and a
    // steer write theirs: a record that somehow holds a Worktree already is
    // corrected to the one just cut.
    sqlx::query(
        "INSERT INTO worktrees (conversation_id, path) VALUES (?, ?)
         ON CONFLICT(conversation_id) DO UPDATE SET path = excluded.path",
    )
    .bind(id)
    .bind(super::repos::text(worktree)?)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("recording the worktree Conversation {id} arrived into"))?;

    sqlx::query("UPDATE conversations SET base_commit = ?, base_ref = ? WHERE id = ?")
        .bind(base_commit)
        .bind(base_ref)
        .bind(id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording what Conversation {id} branched from"))?;

    tx.commit()
        .await
        .context("recording an arriving checkout")?;

    Ok(())
}
