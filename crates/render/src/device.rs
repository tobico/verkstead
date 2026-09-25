//! What a device says it is when another one asks: the answer the identity
//! endpoint on the peer listener gives (ADR-0020).
//!
//! **The one thing on that listener nobody has to be a member to read.** A
//! device that has never been heard of dials the peer port, completes a
//! handshake without showing a certificate at all, and reads this — which is
//! what makes linking possible in the first place: the human types an address,
//! and what comes back is the device they are about to link, named and
//! fingerprinted, before anything has been agreed between the two machines.
//!
//! So it carries nothing that is anybody's work. The id is what a record and a
//! URL name this device by, and the fingerprint is the certificate that same
//! handshake just handed over, spelled to be read aloud — those two are the
//! whole of an identity, and neither of them is a secret.
//!
//! **And three things read off the machine rather than kept anywhere**: the
//! name it is shown under, the word for its operating system, and every address
//! a peer could reach it on. They are read at the moment they are answered —
//! a laptop moves between the LAN and the tailnet, and DHCP moves everybody —
//! and none of them is configured: the name is the hostname, the OS is the
//! platform's own word with *Linux (WSL)* for the one case a hostname cannot
//! tell apart, and the addresses are read off the machine's interfaces and its
//! Tailscale.
//!
//! A wire type in this crate like every other, and exported to TypeScript with
//! them: the Devices section of the Remote access pane draws this device's own
//! answer, so the browser reads the same shape a peer does.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// One device, as it answers for itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct DeviceIdentity {
    /// The Device Id: sixteen random bytes as lower-case hex, invented at this
    /// device's first start and never changing.
    ///
    /// Spelled `device` rather than `id` because that is what the startup line
    /// calls it and what a URL segment will be — an `id` on a payload that is
    /// about a device could be a Conversation's or a Repo's.
    pub device: String,

    /// And the fingerprint of the certificate the handshake that carried this
    /// answer presented: the SHA-256 of its own bytes, upper-case hex in
    /// colon-separated pairs.
    ///
    /// Answered rather than left to be worked out from the connection, because
    /// the two are checked against each other: a caller compares what it read
    /// here with what the handshake handed it, and a device that named a
    /// certificate other than the one it presented is not the device it says it
    /// is. And it is what the human compares by eye — one person reading off a
    /// phone while another reads off a screen — which is why it is spelled the
    /// way every other tool spells one.
    pub fingerprint: String,

    /// What this device is *shown* under: the hostname of the machine it is on,
    /// read at the moment this is answered.
    ///
    /// Nothing is configured and nothing is typed. A name somebody could set
    /// would be a name two devices could be given, and what a device is *named
    /// by* is the id above — which was invented precisely because neither the
    /// hostname nor the tailnet node name can be relied on to be anybody's
    /// alone.
    ///
    /// A machine that will not say what it is called reads *this machine*,
    /// which is what the one other sentence naming this box already falls back
    /// to.
    pub name: String,

    /// And the word for the operating system it is running, which is what
    /// draws the icon beside the name.
    ///
    /// **A WSL reads *Linux (WSL)*, and that is the whole reason this is
    /// here.** A Windows machine and the WSL on it share a hostname, so the
    /// name cannot tell the two apart and this is the only thing that can —
    /// which is the setup cluster mode was written for. Everywhere else it is
    /// the platform's own spelling of itself: *Linux*, *macOS*, *Windows*.
    pub os: String,

    /// And every address a peer could reach this device on, in the order one
    /// should try them: the tailnet name and its addresses first where
    /// Tailscale is up, then the LAN addresses.
    ///
    /// **All of them, read at each answer rather than configured.** A laptop
    /// moves between the LAN and the tailnet and DHCP moves everybody, so the
    /// address somebody typed to link two devices is only the first one ever
    /// known — this list is what keeps a device that moved reachable.
    ///
    /// Empty is a device on neither a tailnet nor a network, which is an answer
    /// rather than a failure: it still has an id and a fingerprint, and those
    /// are what somebody looking at this is comparing.
    pub addresses: Vec<String>,
}
