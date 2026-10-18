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
use std::process::{ExitStatus, Output};

pub mod booting;
pub mod flake;
pub mod image;
pub mod pinning;
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

/// Where the Annex's Data Directory is inside its box: the NixOS module's
/// `StateDirectory`, which its `config.yaml`, `secrets.yaml` and `device.pem`
/// are in.
pub const DATA_DIR: &str = "/var/lib/verkstead";

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

    /// It answers, and there is no box of Verkstead's own in it — or one that
    /// was imported and never finished being made, which a Keeper stopped part
    /// way through the install run leaves: a box for the next run to pick up
    /// rather than one to boot. See [`Backend::made`].
    NoDistro,

    /// What runs boxes is there and could not be asked: `wsl.exe` is on the
    /// machine and would not run, or listed no box while the box's disk image
    /// is still on the Keeper's disk. Nothing is known about the box, which is
    /// what keeps it apart from [`Probed::NoWsl`] and [`Probed::NoDistro`]: a
    /// Keeper's start that finds either of those forgets its Annex, and one
    /// that finds this forgets nothing — see [`booting::Annexed::up`].
    ///
    /// Drawn as [`Probed::NoDistro`] is everywhere else, which is what it read
    /// as before it was told apart.
    Unread,

    /// The box is there, and was made to the end.
    ///
    /// **Read without starting it**, which is why where its network is is not
    /// here: that is asked of the box from inside, and asking a stopped one
    /// anything boots it. See [`Backend::networking`], asked once it is up.
    Present {
        /// Whether it is up at this moment.
        running: bool,
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

/// One command that puts what the box runs under onto this machine, run as an
/// administrator, and what the install screen says while it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// The command, as one line of the platform's shell.
    pub line: &'static str,

    /// And the status line while it runs.
    pub doing: &'static str,
}

/// What the Keeper is, as far as making its Annex goes: who the Annex is kept
/// by, and the port of the Keeper's own peer listener, which the box dials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keeping {
    pub kept_by: crate::settings::KeptBy,
    pub port: u16,
}

/// Where the image a box is made from is downloaded, and what it is checked
/// against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// The image itself.
    pub url: String,

    /// A file holding its SHA-256, the way `sha256sum` prints one.
    pub checksum: String,

    /// What it is called once it is in [`Backend::disk`].
    pub file: &'static str,

    /// And what the status line calls it while it downloads.
    pub name: &'static str,
}

/// What a Keeper asks of its Annex's box.
pub trait Backend: std::fmt::Debug + Send + Sync {
    /// The machine's state — see [`Probed`].
    fn probe(&self) -> Probed;

    /// What turns on what the box runs under, on a machine whose probe said
    /// [`Probed::NoWsl`]: each command raised in its own dialog, in order.
    ///
    /// Lines rather than a call, because what runs them is the wizard's
    /// install run, behind the app's own dialog — see
    /// [`crate::remote::Elevate`]. The restart after them is the human's.
    fn turning_on(&self) -> &'static [Step];

    /// The image the box is made from.
    fn image(&self) -> Image;

    /// The directory of the Keeper's machine the box's disk is kept in, and
    /// its image downloaded into.
    fn disk(&self) -> &Path;

    /// How much of the Keeper's disk the box takes: the size of its disk
    /// image, read off the Keeper's own filesystem and never from inside — or
    /// nothing where there is no image there to read.
    fn taken(&self) -> Option<u64>;

    /// Import `image` as the box, its disk in [`Backend::disk`]. A box that is
    /// already there is left as it is, which is what a run picked up after a
    /// Keeper stopped part way through it finds.
    fn make(&self, image: &Path) -> anyhow::Result<()>;

    /// Say the box was made to the end: written and rebuilt. Until this, a box
    /// that is there probes as [`Probed::NoDistro`].
    fn made(&self) -> anyhow::Result<()>;

    /// The command that lets the box reach its Keeper's peer listener on
    /// `port` through the Keeper's firewall, to be raised as an administrator
    /// — or nothing, where nothing stands between them or the rule is already
    /// there.
    fn opening(&self, port: u16) -> Option<String>;

    /// Start the box and keep it up until [`Backend::terminate`].
    fn boot(&self) -> anyhow::Result<()>;

    /// Stop the box.
    fn terminate(&self) -> anyhow::Result<()>;

    /// Where its network is, read off the running box, which is what says
    /// where the pair dial each other — asked once it is booted, never of a
    /// box that is stopped, which asking would start.
    fn networking(&self) -> Networking;

    /// Run `argv` inside the box as `who`, to its end. A command that ran and
    /// failed is an `Ok` with its status; an `Err` is one that could not be run.
    fn run(&self, who: Inside, argv: &[&str]) -> anyhow::Result<Output>;

    /// The same, handing every line it prints on either stream to `line` as it
    /// prints it — which is how a rebuild minutes long is watched.
    fn follow(
        &self,
        who: Inside,
        argv: &[&str],
        line: &mut dyn FnMut(&str),
    ) -> anyhow::Result<ExitStatus>;

    /// Put `bytes` at `path` inside the box as its root, with `mode`, making
    /// the directories above it.
    fn write(&self, path: &str, bytes: &[u8], mode: u32) -> anyhow::Result<()>;

    /// The bytes of the file at `path` inside the box.
    fn read(&self, path: &str) -> anyhow::Result<Vec<u8>>;

    /// The two addresses, read off the running box under `networking`, with the
    /// Keeper's own peer listener on `keeper_port`.
    fn addresses(&self, networking: Networking, keeper_port: u16) -> anyhow::Result<Addresses>;
}

/// A box that is only ever probed, answering what it was told to — what the
/// wizard's suites stand a machine up with.
///
/// The probe is all the wizard asks of a box, and [`wsl`]'s own suite is
/// where reading one off a real `wsl.exe` is proven. Everything else here
/// refuses.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct Probing(pub Probed);

#[cfg(test)]
impl Backend for Probing {
    fn probe(&self) -> Probed {
        self.0
    }

    fn turning_on(&self) -> &'static [Step] {
        wsl::Wsl::here().turning_on()
    }

    fn image(&self) -> Image {
        wsl::Wsl::here().image()
    }

    fn disk(&self) -> &Path {
        Path::new("nowhere")
    }

    fn taken(&self) -> Option<u64> {
        None
    }

    fn make(&self, _: &Path) -> anyhow::Result<()> {
        anyhow::bail!("a probed box is never made")
    }

    fn made(&self) -> anyhow::Result<()> {
        anyhow::bail!("a probed box is never made")
    }

    fn opening(&self, _: u16) -> Option<String> {
        None
    }

    fn boot(&self) -> anyhow::Result<()> {
        anyhow::bail!("a probed box is never booted")
    }

    fn terminate(&self) -> anyhow::Result<()> {
        anyhow::bail!("a probed box is never terminated")
    }

    fn networking(&self) -> Networking {
        Networking::Nat
    }

    fn run(&self, _: Inside, _: &[&str]) -> anyhow::Result<Output> {
        anyhow::bail!("nothing runs inside a probed box")
    }

    fn follow(&self, _: Inside, _: &[&str], _: &mut dyn FnMut(&str)) -> anyhow::Result<ExitStatus> {
        anyhow::bail!("nothing runs inside a probed box")
    }

    fn write(&self, _: &str, _: &[u8], _: u32) -> anyhow::Result<()> {
        anyhow::bail!("nothing is written inside a probed box")
    }

    fn read(&self, _: &str) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("nothing is read inside a probed box")
    }

    fn addresses(&self, _: Networking, _: u16) -> anyhow::Result<Addresses> {
        anyhow::bail!("a probed box has no addresses")
    }
}
