//! **Agent Profiles as records** (ADR-0028), replicating: the Profile record —
//! its name, harness, models and memory switch — and each device's **binding**
//! of it, two kinds the store declares and this server serves and pulls like
//! any other — see [`super`].
//!
//! Nothing is held of them in memory: every list, picker and launch reads them
//! out of the database. What a merge hands on is what the records *name*
//! outside their own table, settled by the store — see [`store::settle`]:
//!
//! - **A mirror held from before the uuids is bridged** onto the record its
//!   home published, by where that record was minted and the id it had there,
//!   and the copy of its account under the Data Directory follows the uuid.
//! - **A removed record nulls every Pairing** on this device that named it, and
//!   the copy of its account under the Data Directory goes with it — the login
//!   of a Profile nobody here can pick any more being nothing to keep.
//!
//! And the pages are told, so every Profiles section and picker redraws off
//! the merged table.

use anyhow::Result;
use verkstead_schema::Nudge;

use super::Replicating;
use crate::AppState;
use crate::relaying::Read;
use crate::store;

impl Replicating for store::Profiles {
    const MOVED: Nudge = Nudge::Profiles;
    const READ: Read = Read::Profiles;

    /// Whatever the records arriving name settled, whether or not this pull
    /// moved one: a bridge that could not be made last time — the home too old
    /// to publish, or the record not yet here — is tried again.
    async fn pulled(state: &AppState) -> Result<()> {
        settled(state).await
    }
}

impl Replicating for store::Bindings {
    const MOVED: Nudge = Nudge::Profiles;

    /// The same word as the records': both are served by one route, and a
    /// member that cannot answer one cannot answer the other.
    const READ: Read = Read::Profiles;
}

/// Settle what the Profile records name — see [`store::settle`] — and move what
/// outside the database follows a uuid: the copy of an account under the Data
/// Directory, renamed with its bridged mirror, and taken away with its removed
/// record.
async fn settled(state: &AppState) -> Result<()> {
    let settled = store::settle(&state.pool).await?;

    for (provisional, uuid) in &settled.bridged {
        tracing::info!(
            provisional = %provisional,
            profile = %uuid,
            "a mirror held from before the uuids is bridged onto the record its home published",
        );

        crate::mirroring::account::bridged(&state.data_dir, provisional, uuid).await;
    }

    crate::mirroring::account::swept(&state.data_dir, &settled.removed).await;

    if !settled.bridged.is_empty() {
        state.nudges.announce_here(Nudge::Profiles);
    }

    Ok(())
}
