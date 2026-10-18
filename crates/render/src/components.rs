//! What the **Components** section is drawn from: what Verkstead has installed
//! onto this machine beyond what the wizard checks, read off the machine at the
//! moment the pane is opened (ADR-0024, *The box on Windows*).
//!
//! **One row today, the Annex's**, and the reading is shaped for more: a
//! harness added after setup is the next row, and it would be a field beside
//! this one rather than a second reading.
//!
//! **Nothing at all on a machine that cannot have the row**, which is what says
//! whether the section is drawn: an Annex is a Windows Keeper's, so everywhere
//! else the answer is `{"annex": null}` and the settings page draws no card.
//!
//! Nothing here is a setting, and nothing here starts anything to be read: the
//! state is the backend's probe, which reads WSL's own list rather than asking
//! the box, the version is the one the Keeper read off the box when it last
//! booted it, and the disk is a file's size on the Keeper's own disk.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Every row of the Components pane this machine can have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ComponentsView {
    /// The **Annex**'s row — *Linux development (WSL)* — or `None` on a machine
    /// that cannot have one, which is every machine but a Windows Keeper.
    pub annex: Option<AnnexComponent>,
}

/// The Annex's row: how its box stands, the release it runs and the disk it
/// takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AnnexComponent {
    /// How the box stands — see [`AnnexState`].
    pub state: AnnexState,

    /// The Verkstead release the box is pinned to, as the Keeper read it off
    /// the box's flake when it last booted it: held since that start, and so
    /// drawn on a stopped box too. `None` until the first boot of this start
    /// has read it, which the row draws as no version rather than a guess.
    pub version: Option<String>,

    /// What the box's disk image takes of the Keeper's disk, in bytes, or
    /// `None` where there is no image to measure.
    pub disk: Option<u64>,

    /// And why the box would not come up at this start, where it would not —
    /// the same line the Annex's row on the Devices list carries.
    pub trouble: Option<String>,
}

/// How the Annex's box stands.
///
/// The wizard's three ways of having no box yet are told apart, because two of
/// them are something the human does before Install can: a restart, and
/// virtualization turned on in the firmware. The words for both are the
/// viewer's, and the WSL row's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum AnnexState {
    /// There is no box made to the end: no WSL, or WSL with no distro of
    /// Verkstead's, or one an install run left part way.
    NotInstalled,

    /// WSL is turned on and the restart that finishes turning it on is owed.
    NeedsRestart,

    /// The firmware has virtualization off, so WSL can run nothing.
    VirtualizationOff,

    /// The box is up.
    Running,

    /// The box is there and is not up.
    Stopped,

    /// The box is being re-pinned or rebuilt, which is a stage of its own
    /// because it restarts the Annex's server under whatever it is running.
    Updating,
}
