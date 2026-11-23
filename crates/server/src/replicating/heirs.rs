//! **Heirs as records** (ADR-0029, *A roadmap's home outlives its device*): which
//! device keeps what a device that left the cluster kept, a kind the store
//! declares and this server serves and pulls like any other — see [`super`] and
//! [`store::heirs`](crate::store::Heirs).
//!
//! Nothing is held of them in memory: every reading of where a stage's home is
//! reads them out of the database. What a merge hands on is a fresh round of
//! reports — every copy here born on a device that has an heir tells its home
//! again, the new home having been told nothing — and a look at every stage
//! held waiting to join, which may be home now.

use anyhow::Result;
use verkstead_schema::Nudge;

use super::Replicating;
use crate::AppState;
use crate::relaying::Read;
use crate::store;

impl Replicating for store::Heirs {
    const MOVED: Nudge = Nudge::Heirs;
    const READ: Read = Read::Heirs;

    async fn merged(state: &AppState) -> Result<()> {
        store::retell(&state.pool).await?;

        // The reports go out on their own clock; a member's stream being taken
        // up is what hurries them, and this is the same news.
        state.nudges.announce_here(Nudge::Heirs);

        Ok(())
    }
}
