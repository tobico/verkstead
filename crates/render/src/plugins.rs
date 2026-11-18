//! What the **Plugins** section is drawn from, and what its presses send
//! (ADR-0030).
//!
//! **One row per Plugin**, whatever it provides, in two groups: the ones
//! installed above the ones Verkstead ships. A row is read off the installed
//! list, which is a Replicated record, and off what this device made of loading
//! it — its trouble — which is this device's alone.
//!
//! **The consent card is a reading of its own**: the manifest at a path, or in
//! a clone of a git URL taken back once it is read, with nothing written, so
//! that what the human grants at **Install** is what they were shown. The grant travels back with the press, and a
//! manifest asking for more by then is refused rather than installed.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// The Plugins page: the installed above the built-in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginsView {
    pub installed: Vec<PluginRow>,

    /// The ones this binary ships and lists. None yet: the fixture that proves
    /// the path is not listed, and the group is drawn only once it has a row.
    /// A built-in has no **Remove**.
    pub built_in: Vec<PluginRow>,
}

/// One Plugin's row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginRow {
    /// Its identity, which every press names it by.
    pub uuid: String,
    pub name: String,
    pub version: String,
    /// The author's name.
    pub author: String,
    /// The Category of each thing it provides, as the manifest writes it, once
    /// each.
    pub provides: Vec<String>,
    pub source: PluginSource,
    /// Its checkbox: whether the cluster has it on.
    pub enabled: bool,
    /// Why it didn't load here, where it didn't. A plugin with trouble counts
    /// as disabled.
    pub trouble: Option<String>,
    /// The press that would put right a dependency its trouble names, where
    /// there is one: one per dependency, in the order the manifest lists them.
    pub needs: Vec<PluginNeed>,
    /// The device it is on, by name, where it is a local plugin on another
    /// device: drawn greyed, and not loaded here.
    pub elsewhere: Option<String>,
    /// The names of the ticked plugins that go with it when it is unticked,
    /// depending on it or on one of those: what the checkbox says first.
    pub dependents: Vec<String>,
}

/// A dependency a row's plugin is in trouble over, with the press that would
/// put it right from that row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginNeed {
    /// The dependency's uuid, which an **Update** names.
    pub uuid: String,
    /// Its display name, as the dependent's manifest writes it.
    pub name: String,
    pub fix: PluginFix,
}

/// What puts a dependency right. A disabled one is put right by ticking its
/// own row, which the trouble says, and needs no press of its own here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind")]
pub enum PluginFix {
    /// It isn't installed: **Install** from the repository the dependency
    /// names, through the consent card **Add a plugin…** shows.
    Install { repository: String },
    /// It is installed outside the version the dependent asks for: **Update**
    /// on its row, which shows its card first where it asks for more.
    Update,
}

/// Where a Plugin came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind")]
pub enum PluginSource {
    /// A directory on one device, read where it is.
    Local { path: String },
    /// A git repository every device clones for itself, checked out at the
    /// revision pinned at install or at the last **Update**.
    Git { url: String, revision: String },
}

/// Where to read a manifest for the consent card: a directory's absolute path,
/// or a git URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginAt {
    pub source: String,
}

/// The consent card: who the plugin is, what it provides, and every
/// capability it asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginConsent {
    pub uuid: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub email: String,
    pub description: Option<String>,
    pub provides: Vec<String>,
    pub capabilities: Vec<PluginCapability>,
}

/// One capability a plugin's wasm component asks for.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind")]
pub enum PluginCapability {
    /// A path on the host.
    Filesystem { path: String, writable: bool },
    /// A host it may open connections to.
    Outbound { host: String },
    /// An environment variable it may read.
    Environment { name: String },
}

/// **Install**: the directory or git URL, and the capabilities the card
/// showed, which is the grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginInstall {
    pub source: String,
    pub granted: Vec<PluginCapability>,
}

/// **Update**: nothing the first time, and the capabilities the card showed
/// where the manifest asked for more and the card was confirmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginUpdate {
    pub granted: Option<Vec<PluginCapability>>,
}

/// What an **Update** came to: the page read again, or the consent card where
/// the manifest now asks for more than was granted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind")]
pub enum PluginUpdated {
    Updated { plugins: PluginsView },
    Consent { consent: PluginConsent },
}

/// The checkbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginEnabled {
    pub enabled: bool,
}
