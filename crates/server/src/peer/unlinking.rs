//! The unlink, as the device being told answers it: a member of this one's
//! cluster, saying that a device is not one any more (ADR-0020, *A cluster is a
//! membership*).
//!
//! **A membership rather than a set of pairs, which is the whole of why this
//! route exists.** Cutting only the pair between the device that pressed and
//! the device that left would be a cluster whose list read differently
//! depending on which machine you opened it on — so the press is broadcast, and
//! every member drops the same device off the same list. There is no second
//! confirmation anywhere: the human was asked once, on their own workbench, and
//! this call arrives over a link the receiving device has already verified.
//!
//! **A member's own call, so it stands inside [`super::gate`]** with the
//! announcement beside it. The un-gated surface was closed at three in
//! [`super::exchange`] and this is not a fourth — a device that this one holds
//! no membership for has nothing to say about who is in this cluster.
//!
//! **And the leaver is told over this same route.** What a device does when the
//! id named is its *own* is forget every member it holds, rather than forget
//! itself — which it is not on its own list to do. One route for the two
//! because they are one fact arriving: this device is out of that cluster, or
//! that device is out of this one, and which of them it is is read off the id
//! rather than said in a second word.
//!
//! **Which is what settles the order an unlink is done in.** The leaver is told
//! first, while the device pressing still holds a membership for it and it
//! still holds one for the device pressing — a call to a device already dropped
//! would be a dial with no row to dial from, and one *from* a device already
//! dropped would be refused at the leaver's own gate. See
//! [`crate::device::Devices::unlink`], where that order is kept.
//!
//! **Nothing here is refused for the device not being there.** An unlink twice
//! is an unlink, and a member told to drop a device it never heard of is a
//! member that has already got what the call is for — the stance
//! [`verkstead_store::forget_member`] takes, for its reason: a cluster is a
//! membership rather than a ledger of who dropped whom when.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use verkstead_schema::Nudge;

use crate::device::Device;

use super::Members;

/// Where a member says that a device is out of this cluster.
///
/// `DELETE` against the membership the announcement `POST`s to, which is the
/// same noun the other way round: a device taken off this one's list, rather
/// than a path with a verb in it.
pub const MEMBER: &str = "/api/peer/v1/members/{device}";

/// The device being told, as this route answers out of it.
#[derive(Debug, Clone)]
pub(crate) struct Dropping {
    /// What this device is — held for the one question that decides which of
    /// the two things this call is: a device named by its own id is this
    /// machine being told it has left, and any other is a device leaving this
    /// machine's list.
    pub(crate) device: Device,

    /// Where the row goes from.
    pub(crate) members: Members,

    /// And who to tell once it has gone, which is every workbench of this
    /// device that happens to be open: the Devices section has a row fewer than
    /// it had a moment ago, and nobody over here pressed anything for it.
    pub(crate) nudges: crate::nudge::Nudges,
}

/// `DELETE /api/peer/v1/members/{device}` — a member, naming a device this
/// cluster no longer holds.
///
/// **Behind the gate, so the caller is a member before this is read at all.**
/// What is left to judge is the id, and the one thing to judge about it is
/// whether it is this device's own: that is the leaver being told, and what it
/// does is let go of the whole membership rather than of one row.
///
/// `204` either way, and for an id naming nobody too. What this call asks for
/// is a state — that device is not on this list — and a device that was never
/// on it is already in that state.
pub(crate) async fn dropped(
    State(dropping): State<Dropping>,
    Path(device): Path<String>,
) -> Response {
    let leaving = device.trim();

    if leaving.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "a device with no id is not a device this cluster holds\n",
        )
            .into_response();
    }

    let done = if leaving == dropping.device.id() {
        tracing::info!(
            "a member says this device has been unlinked from its cluster, so every device \
             it held is let go of",
        );

        dropping.members.forget_everybody().await
    } else {
        tracing::info!(
            device = %leaving,
            "a member says a device is out of this cluster, and it is out of this one's list",
        );

        dropping.members.forget(leaving).await
    };

    if let Err(why) = done {
        tracing::error!(%why, "a device a member unlinked could not be let go of");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "this device could not let go of the membership\n",
        )
            .into_response();
    }

    // And every open workbench reads the section back: there is a row in the
    // Devices list that was there a moment ago, and nobody over here pressed
    // anything for it.
    dropping.nudges.announce(Nudge::Devices);

    StatusCode::NO_CONTENT.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The unlink is its own path rather than any of the three the gate stands
    /// aside for: it is a member's call, and it goes where a member's calls go.
    #[test]
    fn the_unlink_is_not_one_of_the_un_gated_three() {
        assert_ne!(MEMBER, super::super::IDENTITY);
        assert_ne!(MEMBER, super::super::joining::JOIN);
        assert_ne!(MEMBER, super::super::exchange::SETTLED);
    }

    /// And it is the announcement's own noun with an id on it, rather than a
    /// path of its own: one membership, said to and taken from.
    #[test]
    fn the_unlink_is_the_membership_with_a_device_named() {
        assert_eq!(
            MEMBER,
            format!("{}/{{device}}", super::super::announcing::MEMBERS),
        );
    }
}
