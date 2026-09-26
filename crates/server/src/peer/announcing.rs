//! The announcement, as the member being told answers it: a device this one is
//! already linked to, naming a newcomer (ADR-0020, *A cluster is a
//! membership*).
//!
//! **The first member-only route on this listener, and no exemption with it.**
//! The un-gated surface was closed at three in [`super::exchange`] — the
//! identity endpoint, the join post with the cancel beside it, and the dial
//! back answering one — and this is not a fourth. An announcement is a
//! *member's* own call: it comes from a device this one has already confirmed
//! and holds a certificate for, over a link that certificate proves, so it
//! stands behind [`super::gate`] like everything after it will.
//!
//! **Which is the whole of why the introducer makes it rather than the
//! newcomer.** A newcomer that announced itself would be a stranger asking a
//! member to record it, and a member has no way to tell that from anybody else
//! able to reach its peer port: the one human's press would be the cluster's
//! only gate, and every other member would be joinable without passing it. So
//! the claim is carried by something — the link the introducer already holds —
//! and a device naming *itself* to a member it has not joined is refused at the
//! gate without a line here being written to refuse it.
//!
//! **What arrives is what a roster carries**, which is [`DeviceIdentity`]: the
//! newcomer's id, name, OS, addresses and the fingerprint of the certificate it
//! presents. The same shape the handover in [`super::exchange`] hands over,
//! because it is the same thing said — a device described well enough to be
//! dialled and proved.
//!
//! **And recording it is keyed on the Device Id**, so an announcement made
//! twice is the row said again rather than a second row. Which is what makes
//! the call safe to repeat: a member that was off when a newcomer joined is
//! told when it next answers, and a member that was told already is told again
//! for nothing.
//!
//! **Nothing is confirmed here.** No modal, no push, no press — the human
//! pressed Allow once, on the device the newcomer asked, and a second
//! confirmation on every member is the arrangement the ADR turned down. What
//! the workbenches of this device get is a Nudge, so a Devices section somebody
//! has open draws the row that has just appeared.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use verkstead_render::DeviceIdentity;
use verkstead_schema::Nudge;
use verkstead_store::Linking;

use crate::device::Device;

use super::Members;

/// Where a member names a device as one of this cluster's.
///
/// `POST` to the membership rather than a path with a verb in it, because that
/// is what the call is: a device put on this one's list. The unlink broadcast
/// beside it is the same noun the other way round — see
/// [`super::unlinking::MEMBER`].
pub const MEMBERS: &str = "/api/peer/v1/members";

/// The device being told, as this route answers out of it.
#[derive(Debug, Clone)]
pub(crate) struct Told {
    /// What this device is — held for one question, which is whether the
    /// announcement names this machine. A well-behaved member never sends one
    /// that does; a row recording this device as a member of its own cluster is
    /// the kind of thing that would be drawn for ever, so it is dropped here
    /// rather than trusted not to arrive.
    pub(crate) device: Device,

    /// Where the newcomer is written down.
    pub(crate) members: Members,

    /// And who to tell once it has landed, which is every workbench of this
    /// device that happens to be open: the Devices section has a row it did not
    /// have a moment ago, and nothing else would say so.
    pub(crate) nudges: crate::nudge::Nudges,

    /// And this device's cluster as something to dial, held for the one thing a
    /// newcomer landing here sets off: where a changeover of this device's own
    /// is in flight, the device just recorded is one more member that has
    /// acknowledged nothing — and nobody over there knows that, the announcement
    /// being this end's to make. See
    /// [`crate::device::Devices::announcing_renewal`].
    pub(crate) devices: crate::device::Devices,
}

/// `POST /api/peer/v1/members` — a member, naming a device this cluster now
/// holds.
///
/// **Behind the gate, so the caller is a member before this is read at all.**
/// What is left to judge is the payload: a device with no id or no certificate
/// is not a member of anything, and one naming this machine is this machine.
/// Neither is recorded, and neither is a failure — an announcement is a thing a
/// peer makes about somebody else, and what this end does with a claim it
/// cannot use is leave its own list alone.
///
/// Recorded keyed on the Device Id, which is what makes the call safe to make
/// again: an announcement about a device already recorded updates what is known
/// about it rather than making a second row — see
/// [`verkstead_store::record_member`].
pub(crate) async fn announced(
    State(told): State<Told>,
    Json(newcomer): Json<DeviceIdentity>,
) -> Response {
    if newcomer.device.trim().is_empty() || newcomer.fingerprint.trim().is_empty() {
        return refused(
            StatusCode::BAD_REQUEST,
            "a device with no id or no certificate is not a member of anything",
        );
    }

    if newcomer.device == told.device.id() {
        tracing::warn!(
            "a member named this device as a newcomer, which is a row this device would \
             draw on its own list for ever, so it is left out",
        );

        return StatusCode::NO_CONTENT.into_response();
    }

    let linking = Linking {
        device: newcomer.device.clone(),
        name: newcomer.name,
        os: newcomer.os,

        // Every address it advertised, in the order it advertised them, which
        // is the order a dial to it will work down — see
        // [`crate::peer::dialling`].
        addresses: newcomer.addresses,

        // And the certificate it presents, which is the whole of what will
        // prove it at this device's gate from now on: a member *is* a
        // fingerprint. This end has not met it — the link it arrived over is
        // the introducer's, not the newcomer's — and that is what the vouching
        // is for.
        fingerprint: newcomer.fingerprint,
    };

    if let Err(why) = told.members.refreshed(&linking).await {
        tracing::error!(%why, "a device announced by a member could not be written down");

        return refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "this device could not write the membership down",
        );
    }

    tracing::info!(
        device = %linking.device,
        "a member named a device as one of this cluster's, and it is one",
    );

    // And every open workbench reads the section back: there is a row in the
    // Devices list that was not there a moment ago, and nobody over here
    // pressed anything for it.
    told.nudges.announce(Nudge::Devices);

    // And where this device is in the middle of a changeover of its own, the
    // newcomer is one more member that has yet to acknowledge the certificate
    // coming in — it has never heard of this device's changeover, and nothing
    // over at the introducer's end could tell it. Left undone, a changeover one
    // acknowledgement from finishing is held open by a device that is perfectly
    // reachable, until the next start.
    //
    // In a task, so the member that announced this is answered now rather than
    // after a dial down somebody else's addresses, and nothing at all where no
    // changeover is in flight.
    told.devices.announcing_renewal();

    StatusCode::NO_CONTENT.into_response()
}

/// A refusal on this listener, in the plain text the rest of it refuses in.
fn refused(status: StatusCode, why: &'static str) -> Response {
    (status, format!("{why}\n")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The announcement is its own path rather than any of the three the gate
    /// stands aside for: it is a member's call, and it goes where a member's
    /// calls go.
    #[test]
    fn the_announcement_is_not_one_of_the_un_gated_three() {
        assert_ne!(MEMBERS, super::super::IDENTITY);
        assert_ne!(MEMBERS, super::super::joining::JOIN);
        assert_ne!(MEMBERS, super::super::exchange::SETTLED);
    }
}
