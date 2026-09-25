//! The renewal, as the member being told answers it: a device this one is
//! already linked to, saying that it has made its certificate again (ADR-0020,
//! *The certificate is renewed before it runs out*).
//!
//! **What this is for.** A membership is a set of fingerprints and the handshake
//! is the one place an expiry is a fact rather than a date in a file — see
//! [`super::current`]. So a device re-issues its certificate before the ninety
//! days are up, and unless every member is told, the day it starts presenting
//! the new one is the day every link it holds stops working. The re-issue was
//! built in `crates/server/src/device.rs`; this is the telling, and the answer
//! that takes one member off the list it is waiting on.
//!
//! **A member's own call, so it stands inside [`super::gate`]** with the
//! announcement and the unlink. The un-gated surface was closed at three in
//! [`super::exchange`] and this is not a fourth: a device whose certificate
//! nothing here has recorded has nothing to say about what certificate to hold
//! against which id, and saying it *is* that device is exactly what it cannot
//! prove. Which is why the announcement is made **presenting the outgoing
//! certificate** — the one every member still holds is the only one that gets
//! through their gates, and a renewing device that switched first would be a
//! device announcing its new certificate to nobody.
//!
//! **Two certificates arrive and both are kept.** The one the handshake carried
//! is what proves the caller and is what it is still presenting; the one named in
//! the payload is what it is changing to. The row is keyed on the new one from
//! here — a member *is* a fingerprint, and that is the one it will be — and the
//! old one is kept beside it and goes on being accepted, because the far end will
//! present it until the last of *its* members has acknowledged and it tells
//! nobody when that was. Settled that way rather than the other reading, which
//! was that a member accepts the old until it has acknowledged and the new
//! thereafter: in a cluster of three, the member that acknowledged first would
//! then be refusing every call from the device it had just acknowledged, for as
//! long as the third machine was switched off. See
//! [`verkstead_store::Member::renewing_from`], and
//! [`verkstead_store::changeover_over`], which is where the old one is let go of
//! — at the moment this device meets the new one, that being the only
//! unambiguous sign the far end has stopped presenting the other.
//!
//! **And the answer is the acknowledgement.** `204` says the fingerprint is
//! recorded against that id, which is what the caller writes down as one member
//! fewer to wait for. There is nothing in the body to read: what the caller
//! needed to know is that this call landed, and a device that answered anything
//! else has not recorded anything.
//!
//! **Nothing is confirmed here, and nothing is a second row.** No modal, no push
//! and no press — a certificate being renewed is not a device asking to join, and
//! the id it arrives under is one the human already allowed once. The same call
//! made twice writes the same two strings, which it has to: a member that was off
//! when the certificate was made again is told when it next answers, and an
//! answer that went missing is a call worth making again.

use axum::Json;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use verkstead_render::RenewedCertificate;
use verkstead_schema::Nudge;
use verkstead_store::Renewal;

use crate::device::Device;

use super::{Caller, Members};

/// Where a member says its certificate has been made again.
///
/// **Not under `/members`, where the announcement and the unlink are.** Those two
/// are a device being put on this one's list and taken off it, and they name the
/// device they are about; this one names nobody — the caller is the device it is
/// about, and the certificate the handshake carried is what says which. A path
/// with an id in it would be a path inviting a member to name somebody else.
pub const CERTIFICATE: &str = "/api/peer/v1/certificate";

/// The device being told, as this route answers out of it.
#[derive(Debug, Clone)]
pub(crate) struct Renewing {
    /// What this device is — held for the one question a payload about somebody
    /// else's certificate can ask of it: whether the id naming itself is this
    /// machine's own. A row recording this device as a member of its own cluster
    /// is the thing [`super::announcing`] refuses for the same reason.
    pub(crate) device: Device,

    /// Where the new fingerprint is written, against the same id.
    pub(crate) members: Members,

    /// And who to tell once it has landed, which is every workbench of this
    /// device that happens to be open: the fingerprint drawn against a row in
    /// the Devices section is not the one it was a moment ago.
    pub(crate) nudges: crate::nudge::Nudges,
}

/// `POST /api/peer/v1/certificate` — a member, naming the certificate it is
/// changing over to.
///
/// **Behind the gate, so the caller is a member before this is read at all.**
/// What is left to judge is that the payload is about the caller and nobody
/// else, and there are two halves to it. The identity has to name the
/// certificate this connection's handshake actually carried, which is the same
/// check a dial makes of an identity answer — a device that names another
/// certificate is not the device it says it is. And the row this device holds
/// under that certificate has to be the row the payload names, or a member would
/// be able to say what fingerprint to hold against somebody else's id, which is
/// the whole of a link undone by a payload.
pub(crate) async fn renewed(
    State(renewing): State<Renewing>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    Json(announced): Json<RenewedCertificate>,
) -> Response {
    // The gate let this caller through on a certificate, so there is one; a
    // refusal here rather than an unwrap because the two are separate readings
    // of one connection and nothing in this file enforces the order.
    let Some(presented) = caller.fingerprint() else {
        return refused(
            StatusCode::FORBIDDEN,
            "a device that presented no certificate has none to renew",
        );
    };

    if announced.incoming.trim().is_empty() {
        return refused(
            StatusCode::BAD_REQUEST,
            "a renewal with no incoming certificate is a renewal to nothing",
        );
    }

    if announced.identity.fingerprint != presented {
        return refused(
            StatusCode::BAD_REQUEST,
            "the certificate named as the one being presented is not the one this \
             connection carried",
        );
    }

    if announced.identity.device == renewing.device.id() {
        tracing::warn!(
            "a member announced a renewal under this device's own id, which is a row this \
             device would draw on its own list for ever, so it is left out",
        );

        return StatusCode::NO_CONTENT.into_response();
    }

    // A renewal to the certificate already recorded is a call that has already
    // landed — the answer to the first one went missing, or a debt was paid
    // twice. Acknowledged rather than recorded again, because recording it would
    // take the certificate the far end is still presenting for the one it was
    // changing from and leave this device refusing it.
    if announced.incoming == presented {
        return StatusCode::NO_CONTENT.into_response();
    }

    let held = match renewing.members.rows().await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(%why, "the membership a renewal is recorded against could not be read");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read the membership",
            );
        }
    };

    // Which member the caller *is*, read off the certificate rather than off the
    // payload: an id is a string anybody may write, and the whole of what is
    // being changed here is which certificate stands against one.
    let caller_is = held.iter().find(|member| {
        member.fingerprint == presented
            || member.renewing_from.as_deref() == Some(presented.as_str())
    });

    match caller_is {
        Some(member) if member.device == announced.identity.device => {}

        Some(member) => {
            tracing::warn!(
                caller = %member.device,
                named = %announced.identity.device,
                "a member announced a renewal of another device's certificate, which is a \
                 link this device holds being rewritten by somebody else, so it is refused",
            );

            return refused(
                StatusCode::FORBIDDEN,
                "a device may announce its own certificate and no other",
            );
        }

        // Through the gate and not on the list a moment later, which is an
        // unlink between the two readings. Nothing to record and nothing to
        // fail: that device is not a member, so there is no row for a
        // fingerprint to stand against.
        None => return StatusCode::NO_CONTENT.into_response(),
    }

    let renewal = Renewal {
        device: announced.identity.device.clone(),
        name: announced.identity.name,
        os: announced.identity.os,

        // Every address it advertised, in the order it advertised them, which is
        // the order a dial to it will work down — see [`crate::peer::dialling`].
        addresses: announced.identity.addresses,

        // The one it is still presenting, which is what every call from it will
        // carry until the last of its own members has acknowledged the other.
        presenting: presented,

        // And the one the row is keyed on from now on.
        incoming: announced.incoming,
    };

    match renewing.members.renewing(&renewal).await {
        Ok(true) => {}

        // Gone from the list between the reading above and this write, which is
        // the same unlink arriving a moment later.
        Ok(false) => return StatusCode::NO_CONTENT.into_response(),

        Err(why) => {
            tracing::error!(%why, "a member's renewed certificate could not be written down");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not write the certificate down",
            );
        }
    }

    tracing::info!(
        device = %renewal.device,
        fingerprint = %renewal.incoming,
        presenting = %renewal.presenting,
        "a member has made its certificate again, and both are accepted until this device \
         meets the new one",
    );

    // And every open workbench reads the section back: the fingerprint drawn
    // against that row is not the one it was, and nobody over here pressed
    // anything for it.
    renewing.nudges.announce(Nudge::Devices);

    StatusCode::NO_CONTENT.into_response()
}

/// A refusal on this listener, in the plain text the rest of it refuses in.
fn refused(status: StatusCode, why: &'static str) -> Response {
    (status, format!("{why}\n")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The renewal is its own path rather than any of the three the gate stands
    /// aside for: it is a member's call, and it goes where a member's calls go.
    #[test]
    fn the_renewal_is_not_one_of_the_un_gated_three() {
        assert_ne!(CERTIFICATE, super::super::IDENTITY);
        assert_ne!(CERTIFICATE, super::super::joining::JOIN);
        assert_ne!(CERTIFICATE, super::super::exchange::SETTLED);
    }

    /// And it names no device, because the caller is the device it is about and
    /// the handshake is what says which — a path with an id in it would invite a
    /// member to name somebody else's.
    #[test]
    fn the_renewal_names_no_device() {
        assert!(
            !CERTIFICATE.contains('{'),
            "the renewal's path carries no segment somebody fills in: {CERTIFICATE}",
        );
    }
}
