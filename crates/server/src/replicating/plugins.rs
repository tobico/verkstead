//! **The installed Plugins as records** (ADR-0030, *Where plugins live*): one
//! record per Plugin under its **uuid**, which is its identity — nothing about
//! a plugin is keyed by its source — in the replicated table beside the
//! Authors and the MCP servers.
//!
//! **What a record holds is the cluster's**: where the plugin came from,
//! whether its checkbox is ticked, the capabilities granted at **Install**, and
//! who the manifest said it was when it was last read at a press. That last is
//! there for the device that cannot read the manifest itself — a local
//! plugin's row on every other device — which still draws its name.
//!
//! **What loading it came to is not**: a plugin's trouble is this device's
//! reading of its own copy, held in memory — see [`crate::plugins::installed`].
//! A merge here is what has it loaded again.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use verkstead_render::PluginCapability;
use verkstead_schema::Nudge;

use super::Replicating;
use crate::AppState;
use crate::relaying::Read;
use crate::store::replicated::Kind;

/// The installed Plugins, as a kind of record.
pub struct Plugins;

impl Kind for Plugins {
    const NAME: &'static str = "plugins";
    type Fields = Fields;
}

impl Replicating for Plugins {
    const MOVED: Nudge = Nudge::Plugins;
    const READ: Read = Read::Plugins;

    /// Every plugin this device can reach loaded again over what merged, and
    /// the pages told.
    async fn merged(state: &AppState) -> Result<()> {
        crate::plugins::installed::load(state).await
    }
}

/// What a Plugin's record holds besides its own stamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fields {
    pub source: Source,

    /// Its checkbox, which is the cluster's: unticked on one device is
    /// unticked on all of them.
    pub enabled: bool,

    /// The capabilities the human was shown and granted, at **Install** or at
    /// an **Update** that asked for more. A manifest asking beyond them is
    /// trouble until an Update shows the card again.
    pub granted: Vec<PluginCapability>,

    /// Who the manifest said it was when it was last read at a press: drawn on
    /// a row whose manifest this device cannot read.
    pub name: String,
    pub version: String,
    pub author: String,
    pub provides: Vec<String>,
}

/// Where a Plugin came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Source {
    /// A directory on one device, read in place and never copied (Set 1772,
    /// Q2): loaded there alone, and drawn greyed everywhere else.
    Local {
        path: String,
        /// The Device Id of the device it is on.
        device: String,
    },

    /// A git repository tracking its default branch, which every device
    /// clones for itself under the Data Directory's `plugins/<uuid>/` and
    /// checks out at `revision` (Set 1762, Q10). Nothing fetches on its own:
    /// an **Update** is the press that moves `revision`, and the other devices
    /// follow the record.
    Git {
        url: String,
        /// The commit pinned at **Install**, or at the last Update.
        revision: String,
    },
}
