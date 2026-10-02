//! What this device is forwarding: the ports of terminals on other devices of
//! the cluster, each held on this device's own `localhost` at the same number
//! while the terminal's tab is open here.
//!
//! **This device's own reading and nobody else's.** A **Forward** is a listener
//! on this machine's loopback, so what it says is a fact about this machine:
//! the reading is answered to this device's own browser and held back from the
//! link, the way the Remote access, Devices and push namespaces are. A member
//! reading it would be reading which of *its own* ports some other machine had
//! taken.
//!
//! **Forward** is the term, kept apart from the Relay: the Relay carries a
//! browser's call to a member, and a Forward carries any program's connection
//! to a port a member's terminal is listening on.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// `GET /api/ui/forwards` — every Forward this device holds, forwarding or
/// skipped, lowest port first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ForwardsView {
    pub forwards: Vec<ForwardView>,
}

/// One of them: the port, the device and terminal it reaches, the Conversation
/// that terminal is on, and whether it is forwarding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ForwardView {
    /// The number, which is the same on both machines: `localhost` here at this
    /// port is the server listening on it there.
    pub port: u16,

    /// The device the terminal is on, by its Device Id.
    pub device: String,

    /// And what that device is called and the word for its operating system,
    /// as its row in the membership holds them — what the popup draws its mark
    /// and its name off.
    pub name: String,
    pub os: String,

    /// The Conversation the terminal belongs to, by its id on that device.
    pub conversation: i64,

    /// And what it is called, as the sidebar calls it — `None` where this device
    /// holds no row of that member's for it yet.
    pub title: Option<String>,

    /// Which of the Conversation's terminals is listening.
    pub terminal: i64,

    pub standing: ForwardStanding,
}

/// Whether a Forward is carrying connections, or could not be held here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ForwardStanding {
    /// A listener on this device's loopback, carrying every connection made to
    /// it across the link.
    Forwarding,

    /// Nothing listening here for it, and why. Tried again while it stands, so a
    /// port that frees is taken without anybody pressing anything.
    Skipped { reason: ForwardSkip },
}

/// Why a Forward could not be held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(rename_all = "snake_case")]
pub enum ForwardSkip {
    /// The number is already bound on this device — its own dev server, or
    /// another member's port of the same number already forwarded. Neither is
    /// taken over and nothing is remapped: *port busy here*.
    PortBusy,

    /// The machine would not let this device listen on it at all, which is a
    /// privileged port on a server that holds no privilege.
    NotPermitted,
}
