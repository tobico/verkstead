//! Asking to be let into a cluster: what Add sends, what the device being asked
//! sends back, and what the pending row it leaves behind says (ADR-0020, *The
//! join*).
//!
//! **Three types across two wires.** [`NewJoin`] goes from the browser to the
//! workbench — one typed address, which is the one thing on the Remote access
//! pane that is configured rather than read. What goes over the *peer* listener
//! from there is a [`DeviceIdentity`](crate::DeviceIdentity), the asking device
//! saying what it is to a device that has never met it, and [`JoinHeld`] is what
//! comes back: what the far end is calling this request, when it lets go of it,
//! and what the far end itself is. And [`PendingJoin`] is the row the asking
//! device draws while it waits.
//!
//! **The asking device says what it is with the same type it would answer a
//! stranger with**, rather than with a shape of its own. A join is a device
//! introducing itself, and there is one description of a device in this tree:
//! the id every record names it by, the fingerprint of the certificate it is
//! presenting, the name and the OS word it is shown under, and every address a
//! peer could reach it on. A second shape would be the same five fields with
//! somewhere new for them to disagree.
//!
//! **The pending row does not carry the fingerprint it draws.** What it shows is
//! the *asking* device's own — the same string the modal on the far end shows,
//! so that two people, one reading a phone and one reading a screen, are
//! comparing one certificate. That string is already on the reading, on this
//! device's own row, so a copy of it beside every pending request would be the
//! same fact written twice.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::device::DeviceIdentity;

/// What **Add** takes: an address to go and ask at.
///
/// **A port is optional and usually absent.** Every device answers on the peer
/// port unless its host has been told another, so what somebody types is a
/// machine's name or its address — and the port is something they should not
/// have to know. One that carries a port is dialled at it, because an install
/// that was told to listen elsewhere is reachable no other way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct NewJoin {
    pub address: String,
}

/// What a device answers a join post with: the request it is now holding, and
/// itself.
///
/// **Not a viewer type.** This one crosses the peer listener rather than the
/// workbench — it is one Verkstead speaking to another, and no browser ever
/// reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinHeld {
    /// What this device is calling the request, which is what a cancel names
    /// and what the answer to it will arrive under.
    ///
    /// Invented by the device that is holding it, because that is where the
    /// request lives: the asker's row is a note about a question somebody else
    /// has been handed.
    pub request: String,

    /// And when it lets go of it, RFC 3339 — ten minutes on from the moment it
    /// arrived.
    ///
    /// Told to the asker rather than left for it to reckon, because this end is
    /// what will refuse a call naming a request that has run out: an asker
    /// counting its own ten minutes would go on drawing *waiting* over a
    /// question there was no longer anything to answer.
    pub expires: String,

    /// And what this device is, so the asker's pending row can name the device
    /// it is waiting on rather than repeating the address back at the human.
    ///
    /// The fingerprint on it is checked against the certificate the handshake
    /// presented, the way every other answer in a cluster is: a device that
    /// names one certificate and presents another is not the device it says it
    /// is.
    pub identity: DeviceIdentity,
}

/// One join this device is waiting on, as the Devices section draws it.
///
/// **A row that is not a device yet.** It sits under the members with nothing of
/// a device's own on it — no OS mark and no addresses — because nothing has been
/// agreed: what is known is where this device knocked, what answered, and that
/// somebody at the far end has been asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PendingJoin {
    /// What the far end calls this request, which is what Cancel names.
    pub request: String,

    /// The address that was typed, which is the one place this device has been
    /// told to look for the other.
    pub address: String,

    /// And what the device at that address said it is shown under, which is what
    /// the row reads *waiting for confirmation on*.
    pub name: String,

    /// Whether the ten minutes have run out.
    ///
    /// **This device's own reading of the far end's word for the moment**, made
    /// as the pane is answered. Which is why it is a flag here rather than the
    /// moment itself: the page would otherwise be holding a clock, and the row
    /// says one of two things.
    ///
    /// An expired row is drawn as expired and dismissed rather than taken away
    /// on this device's say-so — somebody pressed Add and is owed the answer
    /// that nobody pressed anything back.
    pub expired: bool,
}
