//! The **Annex Forward**: a listener of the Keeper's own, on
//! [`ANNEX_FORWARD_PORT`] unless told otherwise, that joins every connection it
//! accepts to a fresh one to its Annex's peer listener and copies bytes both
//! ways until either end closes (ADR-0024, *The Keeper forwards for its
//! Annex*).
//!
//! **Raw bytes, and never terminated.** An Annex inside WSL has an address only
//! its Keeper can dial, so a third device reaches it through here — and what
//! that device is checking, as on every dial in a cluster, is the far end's
//! certificate against the fingerprint its member row holds. So the TLS is the
//! Annex's own, end to end: nothing here holds a certificate, reads a byte or
//! knows that what it carries is TLS at all. That is also why this is not
//! [`crate::forwarding`], which is the terminal **Forward** carried over HTTP.
//!
//! **Pointed by the linking operation, and at one Annex.** Where it forwards to
//! is the address the Keeper's Join last went through — see
//! [`crate::device::Devices::keep`] — held in a [`Pointing`] both ends share.
//! Linking again re-points the one forward rather than adding another; a Keeper
//! with two Annexes is a port each, and a later version's. Until anything has
//! been linked a connection is closed as soon as it is accepted, there being
//! nobody to put it through to.
//!
//! [`ANNEX_FORWARD_PORT`]: crate::peer::ANNEX_FORWARD_PORT

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::State;
use axum::extract::connect_info::ConnectInfo;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio::net::TcpStream;

use crate::device::Device;
use crate::device::reading::Reading;

use super::{Caller, Members};

/// How long the dial to the Annex is given before the connection it was for is
/// let go of.
///
/// The peer listener's own handshake deadline, for the same reason: an Annex
/// that has not answered in ten seconds is one the call behind it would not
/// have worked against either, and the caller is free to dial again.
const REACHING: Duration = Duration::from_secs(10);

/// How long an accept that failed waits before going round again — the peer
/// listener's second, for the peer listener's reason.
const AGAIN: Duration = Duration::from_secs(1);

/// Where the forward puts a connection through to: the Annex's peer address,
/// or nothing before an Annex has been linked.
///
/// One handle shared between the listener, which reads it at each connection,
/// and the linking operation, which writes it — so a link made while the
/// forward is up applies to the next connection with nothing restarted.
#[derive(Clone, Debug, Default)]
pub struct Pointing {
    at: Arc<RwLock<Option<String>>>,
}

impl Pointing {
    /// A forward pointed nowhere, which is every Keeper's before it links its
    /// Annex and every other device's for good.
    pub fn at_nothing() -> Pointing {
        Pointing::default()
    }

    /// Point the forward at `address`, the Annex's peer listener — in place of
    /// wherever it pointed before, there being one forward.
    ///
    /// Read the way a dial reads an address: a bare one is the peer port's,
    /// and one with a port on it is dialled at that port.
    pub fn point_at(&self, address: &str) {
        let address = crate::peer::dialling::authority(address);

        *self
            .at
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(address);
    }

    /// Where it forwards to now, if anywhere.
    pub fn pointed_at(&self) -> Option<String> {
        self.at
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// The forward's socket, taken.
///
/// Bound before it is served, as the peer listener is: an address somebody
/// else is already on is a start that should fail saying so.
pub struct Listener {
    /// The standard library's rather than the runtime's, binding being what a
    /// start does before there is a runtime to do it on.
    socket: std::net::TcpListener,

    /// Where it really landed, which is what the startup line says and what a
    /// suite that asked for `:0` dials.
    address: SocketAddr,
}

impl Listener {
    /// Take `address`.
    pub fn bound(address: SocketAddr) -> Result<Listener> {
        let socket = std::net::TcpListener::bind(address)
            .with_context(|| format!("binding the Annex Forward to {address}"))?;

        let address = socket
            .local_addr()
            .context("asking the Annex Forward's socket where it landed")?;

        Ok(Listener { socket, address })
    }

    /// Where it is listening.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Forward every connection to wherever `pointing` says, until the process
    /// is stopped.
    ///
    /// Each connection is a task of its own, so a slow Annex holds up nobody
    /// but the caller waiting on it. Returning at all is the forward having
    /// stopped, which the start treats as it treats either listener stopping.
    pub async fn serving(self, pointing: Pointing) -> Result<()> {
        self.socket
            .set_nonblocking(true)
            .context("putting the Annex Forward's socket into the mode the runtime reads it in")?;

        let socket = tokio::net::TcpListener::from_std(self.socket)
            .context("handing the Annex Forward's socket to the runtime")?;

        loop {
            let (caller, _) = match socket.accept().await {
                Ok(accepted) => accepted,
                Err(why) => {
                    tracing::warn!(%why, "the Annex Forward could not accept a connection");
                    tokio::time::sleep(AGAIN).await;
                    continue;
                }
            };

            // Nobody to put it through to: closed as it stands, which a caller
            // reads as nothing answering.
            let Some(annex) = pointing.pointed_at() else {
                drop(caller);
                continue;
            };

            tokio::spawn(forward(caller, annex));
        }
    }
}

/// Where an Annex asks its Keeper what to advertise: the Keeper's own addresses,
/// each at this forward's port (ADR-0024, *advertise as told*).
///
/// A member's route, behind the gate like every route but the un-gated three —
/// and answered to one member alone, the device this one keeps.
pub const FORWARDED: &str = "/api/peer/v1/forwarded";

/// What [`forwarded`] answers out of: this device, the machine it is on — its
/// forward's port among what that reading knows — and who it is linked to.
#[derive(Debug, Clone)]
pub(crate) struct Forwarding {
    pub(crate) device: Device,
    pub(crate) reading: Reading,
    pub(crate) members: Members,
}

/// `GET /api/peer/v1/forwarded` — this device's Annex, asking where it is to
/// say it can be reached.
///
/// **Only a device keeping an Annex answers it, and only to that Annex.** The
/// caller is through the gate, so it is a member; what is left to ask is
/// whether its row is marked as this device's Annex. Any other member is
/// refused — this device forwards for nobody else, and its addresses at the
/// forward would be a place to reach somebody it is not.
pub(crate) async fn forwarded(
    State(forwarding): State<Forwarding>,
    ConnectInfo(caller): ConnectInfo<Caller>,
) -> Response {
    let calling = match caller.fingerprint() {
        Some(fingerprint) => forwarding.members.presenting(&fingerprint).await,
        None => Ok(None),
    };

    let annex = match calling {
        Ok(member) => {
            member.filter(|member| member.kept_by.as_deref() == Some(forwarding.device.id()))
        }

        Err(why) => {
            tracing::error!(%why, "who asked where this device forwards could not be read");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read who it is linked to\n",
            )
                .into_response();
        }
    };

    if annex.is_none() {
        return (
            StatusCode::FORBIDDEN,
            "this device forwards for its own Annex alone, and the caller is not it\n",
        )
            .into_response();
    }

    match forwarding.reading.forwarded().await {
        Some(addresses) => Json(addresses).into_response(),

        None => (
            StatusCode::NOT_FOUND,
            "this device has no Annex Forward to be reached through\n",
        )
            .into_response(),
    }
}

/// Put `caller` through to `annex` and copy both ways until either end closes.
async fn forward(mut caller: TcpStream, annex: String) {
    let mut reached = match tokio::time::timeout(REACHING, TcpStream::connect(&annex)).await {
        Ok(Ok(reached)) => reached,
        Ok(Err(why)) => {
            tracing::debug!(%annex, %why, "the Annex Forward could not reach the Annex");
            return;
        }
        Err(_) => {
            tracing::debug!(%annex, "the Annex Forward gave up reaching the Annex");
            return;
        }
    };

    // Either end closing, or failing, is the end of the connection; what the
    // two sides said to each other is theirs, so nothing is logged about it.
    let _ = tokio::io::copy_bidirectional(&mut caller, &mut reached).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pointing again replaces where the one forward goes rather than adding a
    /// second, and a bare address is read at the peer port as a dial reads it.
    #[test]
    fn pointing_again_re_points_the_one_forward() {
        let pointing = Pointing::at_nothing();
        assert_eq!(pointing.pointed_at(), None);

        pointing.point_at("127.0.0.1:9423");
        pointing.clone().point_at("192.168.1.24");

        assert_eq!(pointing.pointed_at().as_deref(), Some("192.168.1.24:8423"),);
    }
}
