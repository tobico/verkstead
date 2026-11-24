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

use crate::{CompileCaching, DiskUse, Running, StoreView};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// The Plugins page: the installed above the built-in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginsView {
    pub installed: Vec<PluginRow>,

    /// The ones this binary ships and lists, the Skill Pack `verkstead` first;
    /// the fixture that proves the path is not listed, and the group is drawn
    /// only once it has a row. A built-in has no **Remove** and no **Update**,
    /// and is always on but for a Language plugin, which is on until it is
    /// unticked.
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
    /// The languages it provides, by label — `Rust`, `C/C++` — in the order
    /// its manifest lists them. Empty for one providing none.
    pub languages: Vec<String>,
    /// The drawn name of each Activity it provides, in the order written.
    pub activities: Vec<String>,
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
    /// How many settings fields its manifest declares: where any, the row
    /// opens a pane drawing them.
    pub settings: u32,
    /// How many Agent Profiles name the harness it provides, whatever the
    /// checkbox says: a disabled one's stay, offered on no picker, and while
    /// any stand **Remove** is refused.
    pub profiles: u32,
    /// The harness it provides, where it provides one and has loaded here:
    /// whether its binary is on this machine, and the press that installs
    /// it.
    pub harness: Option<PluginHarness>,
}

/// The harness an Agent plugin's row provides, as this device has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginHarness {
    /// Its id, which the install run is pressed with.
    pub id: crate::AgentType,
    /// The program a session of it runs, looked for on a session's `PATH`.
    pub binary: String,
    /// Whether a session here would find it, which is its onboarding row's
    /// answer.
    pub present: bool,
    /// Whether its manifest declares commands that install it here, which is
    /// what the row's **Install** runs.
    pub installable: bool,
    /// And what the install run has made of it, where one has been started.
    pub install: crate::InstallState,
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
    /// Inside this binary: a built-in.
    BuiltIn,
    /// An installer's own language: an entry of this device's `config.yaml`
    /// naming a language no plugin provides. Drawn only on a device whose file
    /// has the entry, and with no Remove or Update — the entry is the file's.
    Config,
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
    /// The drawn name of each Activity it provides.
    pub activities: Vec<String>,
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
    /// Starting conversations of its own.
    Conversations,
    /// A secret the host sets for it, in the environment variable named.
    Secrets { name: String },
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

/// A Plugin's settings pane: the fields its manifest declares, each with the
/// cluster's value, drawn whether or not the plugin loaded here, so that a
/// setting it needs can be put right from the pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginSettingsView {
    pub uuid: String,
    pub name: String,
    /// Why it didn't load here, where it didn't: the row's own trouble.
    pub trouble: Option<String>,
    pub fields: Vec<PluginField>,
    /// What a Language plugin's pane draws under its fields: each language it
    /// provides, with what its store holds and its Clear. Empty for a plugin
    /// providing none. Drawn whether or not the plugin is ticked, because an
    /// off language still holds what it fetched.
    pub languages: Vec<PluginLanguageCache>,
    /// What is running as this was drawn, which a Clear is refused over.
    pub running: Running,
}

/// One language of a Language plugin, as its pane draws its cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginLanguageCache {
    /// The name `config.yaml` keys it by, which a Clear names.
    pub name: String,
    pub label: String,
    /// The size its store is held to now: the plugin's `size` where it is
    /// one, and the default otherwise.
    pub size: String,
    /// What its store holds, as last measured.
    pub disk_use: DiskUse,
    /// Each directory of its store, and how it is kept under the size.
    pub stores: Vec<StoreView>,
    /// When its swept stores were last swept, RFC 3339, or null where they
    /// have not been since the server started.
    pub swept: Option<String>,
    /// Whether it compiles through the Compile Server, and whether that is
    /// cached — null where it names no such capability.
    pub compiling: Option<CompileCaching>,
    /// The label of the language whose store its compiles are kept in, where
    /// that is another's: C/C++, compiling through Rust's.
    pub through: Option<String>,
}

/// One settings field, as its manifest declares it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginField {
    /// What the value is kept under, unique among the plugin's fields.
    pub key: String,
    pub label: String,
    pub description: Option<String>,
    /// What an empty box shows: the value in force where none is set, where
    /// there is one — a Language plugin's default size.
    pub placeholder: Option<String>,
    pub value: PluginFieldValue,
}

/// A field's kind and what the cluster holds for it. A secret's value never
/// comes back: only whether one is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind")]
pub enum PluginFieldValue {
    /// A line of text, empty where none is set.
    Text {
        value: String,
    },
    /// A size such as `30G`, empty where none is set.
    Size {
        value: String,
    },
    /// A list of lines.
    List {
        values: Vec<String>,
    },
    Secret {
        set: bool,
    },
}

/// A save of a Plugin's pane: the fields it changed, each by key. A field
/// left out is left as it is, which is how a secret nobody typed is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginSettingsSave {
    pub fields: Vec<PluginFieldEdit>,
}

/// One field's new value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginFieldEdit {
    pub key: String,
    pub value: PluginFieldWrite,
}

/// What a field is to hold. Empty text, an empty size and an empty list each
/// clear it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind")]
pub enum PluginFieldWrite {
    Text {
        value: String,
    },
    Size {
        value: String,
    },
    List {
        values: Vec<String>,
    },
    /// A secret typed in, which replaces whatever was set.
    Secret {
        value: String,
    },
    /// A secret taken away.
    SecretCleared,
}

/// What a save of a Plugin's pane came to: the pane read again, and the
/// fields it was refused over, where any — in which case nothing was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginSettingsSaved {
    pub settings: PluginSettingsView,
    pub refused: Vec<PluginFieldRefused>,
}

/// One field a save was refused over, and why, to be drawn at the field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PluginFieldRefused {
    pub key: String,
    pub why: String,
}

/// The `secret` settings a device holds for one Plugin, by key: a member's own
/// call on the Peer Listener, for the copy it keeps in its own `secrets.yaml`,
/// and never the viewer's. Empty where it holds none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginSecretValues {
    pub secrets: std::collections::BTreeMap<String, String>,
}
