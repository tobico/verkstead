//! Which device keeps what a device that has left the cluster kept: its
//! **Heir** (ADR-0029, *A roadmap's home outlives its device*).
//!
//! **Home is the stage's birth device**, and a birth key never changes — see
//! [`super::reports`]. So a device that leaves the cluster for good would leave
//! every stage born on it waiting on a home that will never answer again: its
//! reports owed for ever, its finish held for ever. The heir is what ends that.
//! A record that says *what this device kept is kept by that one now*, and every
//! reading that asks where a stage's home is asks through it — see [`Homes`].
//! The birth key stays exactly as it was, which is what every copy of the work
//! and every tombstone is still keyed by.
//!
//! **A Replicated record** (ADR-0028), keyed by the Device Id of the device that
//! left, its one field the Device Id of its heir. The cluster's rather than any
//! one device's, because every device holding a stage born on the leaver has to
//! read the same heir for it: a stage away from home on a third machine reports
//! to whoever the record names. Newest write wins, so a second naming replaces
//! the first, and a record is never removed — a home that came back would be a
//! second home beside the heir.
//!
//! **What the leaver knew went with it.** Home's tombstones, its queue places
//! and the order its chain joined in were rows in its own database. The heir
//! starts from what the live copies still hold: each one reports to it as it
//! would have to home, and the heir **adopts** a stage it has never heard of —
//! see [`adopt`] — making the tombstone the leaver would have held. A stage that
//! finished on the leaver itself and never moved has no live copy anywhere, and
//! the heir never hears of it.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::births::{Birth, Transferred};
use super::conversations::State;
use super::replicated::{self, Kind, Written};

/// The heirs, as a kind of record.
pub struct Heirs;

impl Kind for Heirs {
    const NAME: &'static str = "heirs";
    type Fields = Heir;
}

/// What a heir record holds besides its own stamps, under the uuid that is the
/// Device Id of the device that left.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heir {
    /// The **Device Id** of the device that keeps what it kept.
    pub heir: String,
}

/// How many heirs a home is followed through: more than any cluster loses
/// devices in a row, and a bound on two devices each named the other's heir.
const MOST_HEIRS: usize = 16;

/// Every heir this device has heard of, read once and asked of many times.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Homes {
    heirs: HashMap<String, String>,
}

impl Homes {
    /// Homes from `(left, heir)` pairs, which is what a test hands it.
    pub fn from_heirs<I, S>(heirs: I) -> Homes
    where
        I: IntoIterator<Item = (S, S)>,
        S: Into<String>,
    {
        Homes {
            heirs: heirs
                .into_iter()
                .map(|(left, heir)| (left.into(), heir.into()))
                .collect(),
        }
    }

    /// Where the home of work born on `born_on` is now: `born_on` itself where
    /// it has no heir, and otherwise its heir's home — an heir may leave too.
    ///
    /// A chain that comes back round on itself ends at the last device it had
    /// not already passed through, rather than going round for ever.
    pub fn of(&self, born_on: &str) -> String {
        let mut at = born_on.to_owned();
        let mut passed = HashSet::from([at.clone()]);

        for _ in 0..MOST_HEIRS {
            let Some(heir) = self.heirs.get(&at) else {
                break;
            };

            if !passed.insert(heir.clone()) {
                break;
            }

            at = heir.clone();
        }

        at
    }

    /// Whether `device` has left and has an heir named for it.
    pub fn left(&self, device: &str) -> bool {
        self.heirs.contains_key(device)
    }
}

/// Every heir this device has heard of.
pub async fn homes(pool: &SqlitePool) -> Result<Homes> {
    Ok(Homes {
        heirs: replicated::live_records::<Heirs>(pool)
            .await
            .context("reading the heirs")?
            .into_iter()
            .map(|record| (record.uuid, record.fields.heir))
            .collect(),
    })
}

/// Name `heir` as the heir of `left`, written here by `here`. `true` where that
/// was news: a naming that changed nothing writes nothing.
///
/// **And every copy here born on `left` tells its home again**, the new home
/// having been told nothing — see [`retell`].
pub async fn name_heir(pool: &SqlitePool, here: &str, left: &str, heir: &str) -> Result<bool> {
    let written = replicated::write::<Heirs>(
        pool,
        here,
        left,
        &Heir {
            heir: heir.to_owned(),
        },
    )
    .await
    .with_context(|| format!("naming {heir} the heir of {left}"))?;

    retell(pool).await?;

    Ok(written == Written::Written)
}

/// Forget what every stage here born on a device with an heir last told its
/// home, so each tells its home again.
///
/// What a stage last told is kept so it is not told twice — see
/// [`super::reports::told`]. A home that has changed was told nothing, and a
/// report that matched what the old home took would never be sent to the new
/// one. Run where an heir is named here and where a member's naming is merged,
/// and blunt on purpose: a report sent again writes the same row again.
pub async fn retell(pool: &SqlitePool) -> Result<()> {
    let left: Vec<String> = replicated::live_records::<Heirs>(pool)
        .await
        .context("reading the heirs")?
        .into_iter()
        .map(|record| record.uuid)
        .collect();

    for device in left {
        sqlx::query(
            "DELETE FROM stage_reports
             WHERE conversation_id IN (SELECT conversation_id FROM births WHERE device = ?)",
        )
        .bind(&device)
        .execute(pool)
        .await
        .with_context(|| format!("forgetting what stages born on {device} told their home"))?;
    }

    Ok(())
}

/// What an heir needs to know about a stage it has never heard of to keep a
/// tombstone of it: what the leaver would have held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adopting {
    /// The Repo **on this device** the stage is in, as the reporting device's
    /// match settled it.
    pub repo_id: i64,

    /// Which roadmap, and which stage of it.
    pub roadmap: String,
    pub stage: String,

    /// The branch the stage is on, and the one it was cut from where that was
    /// not the default branch.
    pub branch: String,
    pub stacks_on: Option<String>,

    /// The state the live copy is in, which the tombstone starts in.
    pub state: State,
}

/// Make the tombstone of stage `born`, whose live copy is `live`, on the device
/// that is its home's heir: the rows a roadmap reads of a stage, and the mark
/// that says the work is elsewhere. Answers the tombstone's id, or `None` where
/// the Repo is not registered here.
///
/// **One where one is held already**, found by the birth key the way every copy
/// is: a second report racing the first adopts nothing twice. What the standing
/// says beyond the state — the moves, the stop, the join — is [`super::follow`]'s
/// to write, which the caller runs next as for any other tombstone.
pub async fn adopt(
    pool: &SqlitePool,
    born: &Birth,
    live: &Transferred,
    adopting: &Adopting,
) -> Result<Option<i64>> {
    let mut tx = super::writing(pool, "adopting a stage whose home has left").await?;

    let held: Option<(i64,)> =
        sqlx::query_as("SELECT conversation_id FROM births WHERE device = ? AND born_as = ?")
            .bind(&born.device)
            .bind(born.id)
            .fetch_optional(&mut *tx)
            .await
            .context("looking for a copy of the stage being adopted")?;

    if let Some((id,)) = held {
        return Ok(Some(id));
    }

    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO conversations (repo_id, created_at, branch, named_branch, state)
         SELECT id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?, ?, ?
         FROM repos
         WHERE id = ? AND id NOT IN (SELECT repo_id FROM unregistered_repos)
         RETURNING id",
    )
    .bind(&adopting.branch)
    .bind(&adopting.branch)
    .bind(adopting.state.stored())
    .bind(adopting.repo_id)
    .fetch_optional(&mut *tx)
    .await
    .with_context(|| format!("adopting stage {} of {}", adopting.stage, adopting.roadmap))?;

    let Some((id,)) = row else {
        return Ok(None);
    };

    super::births::stamp(&mut tx, id, born).await?;

    sqlx::query("INSERT INTO transferred (conversation_id, device, live_as) VALUES (?, ?, ?)")
        .bind(id)
        .bind(&live.device)
        .bind(live.id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("marking adopted Conversation {id} as elsewhere"))?;

    sqlx::query("INSERT INTO stage_roadmaps (conversation_id, roadmap, stage) VALUES (?, ?, ?)")
        .bind(id)
        .bind(&adopting.roadmap)
        .bind(&adopting.stage)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording which stage adopted Conversation {id} is"))?;

    sqlx::query("INSERT INTO stage_branches (conversation_id, stacks_on) VALUES (?, ?)")
        .bind(id)
        .bind(&adopting.stacks_on)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("recording what adopted Conversation {id} stands on"))?;

    tx.commit()
        .await
        .with_context(|| format!("adopting stage {} of {}", adopting.stage, adopting.roadmap))?;

    Ok(Some(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_with_no_heir_is_its_own_home() {
        let homes = Homes::from_heirs([("wolf", "fox")]);

        assert_eq!(homes.of("fox"), "fox");
        assert_eq!(homes.of("bear"), "bear");
        assert!(!homes.left("fox"));
    }

    #[test]
    fn a_home_is_followed_through_every_heir() {
        let homes = Homes::from_heirs([("wolf", "fox"), ("fox", "bear")]);

        assert_eq!(homes.of("wolf"), "bear");
        assert_eq!(homes.of("fox"), "bear");
        assert!(homes.left("wolf"));
    }

    #[test]
    fn two_devices_named_each_other_s_heir_end_somewhere() {
        let homes = Homes::from_heirs([("wolf", "fox"), ("fox", "wolf")]);

        assert_eq!(homes.of("wolf"), "fox");
        assert_eq!(homes.of("fox"), "wolf");
    }
}
