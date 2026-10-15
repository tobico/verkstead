//! What a Keeper does to its **Annex**'s box, said once for every way of having
//! one (ADR-0024).
//!
//! **The Annex is a Verkstead like any other; the box it runs in is not.** A
//! NixOS-WSL distro beside a Windows Verkstead is the first, and a Docker or
//! Lima box beside a Mac, or a Windows VM beside a Linux desktop, are the same
//! thing with another way of making, starting and stopping it. So what the
//! Keeper asks of the box is a trait, [`Backend`], and the wizard's install
//! step, the Keeper's start and its stop are written against that rather than
//! against one way of doing it. [`wsl::Wsl`] is the one implementation, and nothing of
//! the others is written.
//!
//! **Every call blocks.** Each of them is a program run to its end — an
//! import is minutes of it — so a caller on the runtime puts them on a thread
//! that may be waited on, the way [`crate::remote::Elevate::raise`] is called.

use std::net::SocketAddr;
use std::path::Path;
use std::process::Output;

pub mod wsl;

/// The port an Annex's **Peer Listener** binds, never the Keeper's 8423.
///
/// Under WSL's mirrored networking the Annex has its Keeper's interfaces, so a
/// port of its own is what keeps the two listeners from colliding — see
/// ADR-0024, *The Keeper forwards for its Annex*.
pub const ANNEX_PEER_PORT: u16 = 8425;

/// The port an Annex's workbench binds on its own loopback, never the Keeper's
/// 8422, for the same reason.
pub const ANNEX_WORKBENCH_PORT: u16 = 8426;

/// What the Annex's box is called wherever its backend names one: the distro
/// under WSL. Verkstead's own, and never one the human has.
pub const BOX_NAME: &str = "Verkstead";

/// The machine's own state, as far as having an Annex goes — what the wizard's
/// row is drawn from and what the Keeper's start reads before it boots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probed {
    /// Nothing to run a box under: no WSL at all, or only the stub Windows
    /// ships saying it is not installed.
    NoWsl,

    /// It is turned on and the restart that finishes turning it on is still
    /// owed.
    NeedsRestart,

    /// The firmware has virtualization off, so nothing can run a box until it
    /// is turned on there.
    VirtualizationOff,

    /// It answers, and there is no box of Verkstead's own in it.
    NoDistro,

    /// The box is there.
    Present {
        /// Whether it is up at this moment.
        running: bool,

        /// Where its network is, which is what says where the pair dial each
        /// other.
        networking: Networking,
    },
}

/// How the box's network relates to its Keeper's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Networking {
    /// A network of the box's own behind a gateway on the Keeper's side —
    /// WSL's default.
    Nat,

    /// The Keeper's own interfaces and loopback, shared — WSL's
    /// `networkingMode=mirrored`.
    Mirrored,
}

/// Who a command inside the box runs as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inside {
    /// The box's root, which is who rebuilds it.
    Root,

    /// The NixOS module's service user, which is who the Annex's server runs as.
    Service,
}

/// Where the Keeper and its Annex dial each other, read off the running box —
/// the two addresses the Keeper's link takes (ADR-0024, *The Keeper and its
/// Annex reach each other directly*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Addresses {
    /// Where the Keeper reaches the Annex's peer listener, and where its forward
    /// points.
    pub annex: SocketAddr,

    /// Where the Annex reaches its Keeper's peer listener: `kept_by`'s `at`.
    pub keeper_at: SocketAddr,
}

/// What a Keeper asks of its Annex's box.
pub trait Backend: Send + Sync {
    /// The machine's state — see [`Probed`].
    fn probe(&self) -> Probed;

    /// Import `image` as the box, its disk in `disk`.
    fn make(&self, image: &Path, disk: &Path) -> anyhow::Result<()>;

    /// Start the box and keep it up until [`Backend::terminate`].
    fn boot(&self) -> anyhow::Result<()>;

    /// Stop the box.
    fn terminate(&self) -> anyhow::Result<()>;

    /// Run `argv` inside the box as `who`, to its end. A command that ran and
    /// failed is an `Ok` with its status; an `Err` is one that could not be run.
    fn run(&self, who: Inside, argv: &[&str]) -> anyhow::Result<Output>;

    /// The bytes of the file at `path` inside the box.
    fn read(&self, path: &str) -> anyhow::Result<Vec<u8>>;

    /// The two addresses, read off the running box under `networking`, with the
    /// Keeper's own peer listener on `keeper_port`.
    fn addresses(&self, networking: Networking, keeper_port: u16) -> anyhow::Result<Addresses>;
}
