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
//! **And its settings' values**, by the key each field is declared under: a
//! `text`, `size` or `list` value on the record itself, and a `secret` in
//! `secrets.yaml` under the plugin's uuid, the record carrying a digest of it
//! and every device fetching its own copy from a member that holds one — an
//! MCP server's header values' way (ADR-0021, ADR-0028).
//!
//! **A built-in has a record too, once anything is written to it** — a tick,
//! a settings save, the take-up below — and none until then, **absent meaning
//! enabled**, so a cluster that never touched one has it on. Its source is
//! [`Source::BuiltIn`]: nothing to clone, and no Remove.
//!
//! **A Language plugin's switch is taken up out of `config.yaml` once** — see
//! [`taken_up`] — where each language's `enabled` was each device's own.
//! **On wins**: a plugin is on where any of its languages was on, on any
//! device, which [`Plugins`]' [`Kind::taken_up_together`] keeps across two
//! devices' take-ups. A press made since wins over every take-up, so a device
//! upgrading late never ticks again what a human unticked.
//!
//! **An installer's own language is taken up the same way**, the first time
//! a device reads its entry: its `enabled` and `size` into the record of the
//! plugin it is drawn as — see [`Source::Config`] — on winning and the larger
//! size winning, once per language and device.
//!
//! **What loading it came to is not**: a plugin's trouble is this device's
//! reading of its own copy, held in memory — see [`crate::plugins::installed`].
//! A merge here is what has it loaded again.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use verkstead_render::{PluginCapability, PluginSecretValues};
use verkstead_schema::Nudge;

use super::Replicating;
use super::authors::{digest, writing_held};
use crate::AppState;
use crate::languages::SIZE_SETTING;
use crate::peer::plugins::SECRETS;
use crate::plugins::built_in;
use crate::plugins::manifest::SettingKind;
use crate::relaying::{Read, asking, word_from};
use crate::settings::Settings;
use crate::store::replicated::{self, Kind, Record};

/// The installed Plugins, as a kind of record.
pub struct Plugins;

impl Kind for Plugins {
    const NAME: &'static str = "plugins";
    type Fields = Fields;

    /// A plugin's uuid is its author's, so a plugin removed is the same uuid
    /// when it is installed again. That Install makes the record again, on
    /// every device, rather than being refused for good.
    const REVIVES: bool = true;

    /// Two devices' take-ups of a built-in's switch and size: on where either
    /// was on (Set 1784, Q4), and the larger size where either had one, so a
    /// take-up only ever moves a record toward on and never shrinks a cache
    /// somebody was using.
    fn taken_up_together(winner: &Fields, loser: &Fields) -> Option<Fields> {
        let mut values = winner.values.clone();
        if let Some(size) = larger(size_of(winner), size_of(loser)) {
            values.insert(SIZE_SETTING.to_owned(), SettingValue::Line(size.to_owned()));
        }

        Some(Fields {
            enabled: winner.enabled || loser.enabled,
            values,
            ..winner.clone()
        })
    }
}

impl Replicating for Plugins {
    const MOVED: Nudge = Nudge::Plugins;
    const READ: Read = Read::Plugins;

    /// Every plugin this device can reach loaded again over what merged, and
    /// the pages told.
    async fn merged(state: &AppState) -> Result<()> {
        crate::plugins::installed::load(state).await
    }

    /// Every plugin's secrets the records name and this device holds no
    /// current copy of fetched, and the copy of a removed one's taken away.
    async fn pulled(state: &AppState) -> Result<()> {
        copied(state).await
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

    /// The drawn names of the Activities it provides, as its manifest said
    /// when it was last read at a press.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activities: Vec<String>,

    /// The uuids of the plugins its manifest depended on when it was last read
    /// at a press: what keeps those from being removed, and what goes with
    /// one when it is unticked, read the same on every device.
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// The settings fields its manifest declared when it was last read at a
    /// press: what its pane draws on every device, whether or not the
    /// plugin loads there.
    #[serde(default)]
    pub declared: Vec<Declared>,

    /// The cluster's value of each `text`, `size` and `list` field, by key. A
    /// field nothing is set for is not here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, SettingValue>,

    /// Which value is held for each `secret` field, by key: a digest of it —
    /// see [`digest`] — never the value, which is in `secrets.yaml` under the
    /// plugin's uuid. A device whose copy is not the one named here fetches it
    /// from a member that holds it — see [`copied`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,
}

/// A settings field a plugin's manifest declares, as its record keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declared {
    pub key: String,
    pub label: String,
    pub kind: SettingKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// What a `text`, `size` or `list` field holds: a list is lines, and the
/// other two a line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingValue {
    Line(String),
    Lines(Vec<String>),
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

    /// One this binary ships, read out of the binary on every device: its
    /// record holds the cluster's switch and settings, and nothing about where
    /// it came from.
    BuiltIn,

    /// An installer's own language: a `languages:` entry in `config.yaml`
    /// naming a language no plugin provides, drawn as a Language plugin of its
    /// own on every device whose file has the entry, and on no other. Its
    /// record is under a uuid derived from the language's name — see
    /// [`crate::plugins::installed::installer_uuid`] — so every such device
    /// shares one tick and one size (Set 1784, Q5).
    Config {
        /// What `languages:` keys its entry by.
        language: String,
    },
}

/// The size `fields` holds for its languages' stores, where it holds one.
fn size_of(fields: &Fields) -> Option<&str> {
    match fields.values.get(SIZE_SETTING) {
        Some(SettingValue::Line(size)) => Some(size),
        _ => None,
    }
}

/// The larger of two sizes, a size that will not read counting as none.
fn larger<'a>(one: Option<&'a str>, other: Option<&'a str>) -> Option<&'a str> {
    [one, other]
        .into_iter()
        .flatten()
        .filter_map(|size| Some((crate::languages::bytes(size).ok()?, size)))
        .max_by_key(|(bytes, _)| *bytes)
        .map(|(_, size)| size)
}

/// What a built-in's switch is taken up as, apart from the installed
/// Plugins, which were records from the start.
const LANGUAGES_TAKEN_UP: &str = "plugins.languages";

/// Take each built-in Language plugin's switch and size up out of
/// `config.yaml` — its languages' `enabled` and `size`, and
/// `rust_build_cache`'s for Rust, the map winning where both are written —
/// where they have not been already, and hold what the records say for the
/// settings either way.
///
/// **Once**, and stamped older than any press: a plugin is on where any of its
/// languages was on here, and sized at the largest size any of them was given
/// — a size that will not read being none, so the default stands — written as
/// this device's take-up, and put together with every other device's by on
/// winning and the larger size winning — see [`Plugins`]. The file's `enabled`
/// and `size` are not read again from here on; a save of the settings page
/// leaves them in the file as written, it writing no key of a `languages:`
/// entry — see [`crate::settings::Config::keeping_what_the_page_never_drew`].
pub async fn taken_up(pool: &SqlitePool, settings: &Settings, device: &str) -> Result<()> {
    if !replicated::taken_up_as(pool, LANGUAGES_TAKEN_UP).await? {
        let settings_ = settings.clone();
        let config = tokio::task::spawn_blocking(move || settings_.config()).await?;
        let languages = crate::languages::configured(&config);
        // The file's own sizes, `rust_build_cache`'s under the map: what
        // `configured` holds for a built-in once the records are held is the
        // record's, and these are what goes into it.
        let written = config.languages();

        let kept: Vec<(String, Fields)> = built_in::listed()
            .iter()
            .filter(|manifest| built_in::languages(manifest).next().is_some())
            .map(|manifest| {
                let on = built_in::languages(manifest).any(|provided| {
                    languages
                        .get(&provided.name)
                        .is_none_or(crate::languages::Descriptor::enabled)
                });
                let size = built_in::languages(manifest)
                    .filter_map(|provided| written.get(&provided.name)?.size_configured())
                    .fold(None, |largest, size| larger(largest, Some(size)));

                let mut fields = crate::plugins::installed::built_in_fields(manifest, on, None);
                if let Some(size) = size {
                    fields
                        .values
                        .insert(SIZE_SETTING.to_owned(), SettingValue::Line(size.to_owned()));
                }
                (manifest.uuid.clone(), fields)
            })
            .collect();

        replicated::take_up_as_the_oldest_as::<Plugins>(pool, device, LANGUAGES_TAKEN_UP, &kept)
            .await
            .context("taking the languages' switches up out of config.yaml")?;
    }

    installers_taken_up(pool, settings, device).await?;

    held(pool, settings).await
}

/// Take each installer's own language's switch and size up out of
/// `config.yaml` into the record of the plugin it is drawn as, where this
/// device has not already: the first time it reads the entry, which may be
/// long after its first start, and once per language — the built-ins' rule,
/// marked by language rather than once for all of them.
///
/// The entry's `enabled` is the plugin's tick, and its `size` the plugin's,
/// one that will not read being none; put together with every other device's
/// take-up of the same language by on winning and the larger size winning.
async fn installers_taken_up(pool: &SqlitePool, settings: &Settings, device: &str) -> Result<()> {
    let settings = settings.clone();
    let config = tokio::task::spawn_blocking(move || settings.config()).await?;
    let written = config.languages();

    for (name, descriptor) in &crate::plugins::installed::installers(&config) {
        let what = format!("{LANGUAGES_TAKEN_UP}.{name}");
        if replicated::taken_up_as(pool, &what).await? {
            continue;
        }

        let entry = written.get(name);
        let mut fields = crate::plugins::installed::installer_fields(
            name,
            descriptor,
            entry.is_none_or(crate::languages::Descriptor::enabled),
            None,
        );
        if let Some(size) = larger(entry.and_then(|entry| entry.size_configured()), None) {
            fields
                .values
                .insert(SIZE_SETTING.to_owned(), SettingValue::Line(size.to_owned()));
        }

        let kept = [(crate::plugins::installed::installer_uuid(name), fields)];
        replicated::take_up_as_the_oldest_as::<Plugins>(pool, device, &what, &kept)
            .await
            .with_context(|| {
                format!("taking the {name} language's switch up out of config.yaml")
            })?;
    }

    Ok(())
}

/// Hand what the Language plugins' records say — which languages are
/// unticked, and each one's size — on to the settings, which is what every
/// session's languages are read out of: the built-ins', the installed
/// Language plugins' the last load read, and those an installer's own
/// languages are drawn as.
///
/// One at a time: the records are read and handed on under one lock, so a
/// load that read them before a save wrote can never hand its older reading
/// on after the save's.
pub async fn held(pool: &SqlitePool, settings: &Settings) -> Result<()> {
    static HOLDING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _one = HOLDING.lock().await;

    let records = replicated::live_records::<Plugins>(pool).await?;

    let mut held = crate::languages::Held {
        installed: settings.installed_languages(),
        ..crate::languages::Held::default()
    };

    // An installed Language plugin's, by its record's uuid. Only a ticked
    // one is loaded, so only its size is left to say.
    for (uuid, provided) in &held.installed {
        if let Some(size) = records
            .iter()
            .find(|record| record.uuid == *uuid)
            .and_then(|record| size_of(&record.fields))
        {
            held.sizes.insert(provided.name.clone(), size.to_owned());
        }
    }

    // An installer's own language, by the language its record names: held
    // whatever this device's file says, and laid over the entry only where the
    // file still has it.
    for record in &records {
        let Source::Config { language } = &record.fields.source else {
            continue;
        };
        held.recorded.insert(language.clone());
        if !record.fields.enabled {
            held.off.insert(language.clone());
        }
        if let Some(size) = size_of(&record.fields) {
            held.sizes.insert(language.clone(), size.to_owned());
        }
    }

    for manifest in built_in::listed() {
        let Some(record) = records.iter().find(|record| {
            record.uuid == manifest.uuid && matches!(record.fields.source, Source::BuiltIn)
        }) else {
            continue;
        };

        for provided in built_in::languages(manifest) {
            if !record.fields.enabled {
                held.off.insert(provided.name.clone());
            }
            if let Some(size) = size_of(&record.fields) {
                held.sizes.insert(provided.name.clone(), size.to_owned());
            }
        }
    }

    settings.holding_plugin_languages(held);

    Ok(())
}

/// The most a member's answer about one plugin's secrets may be: the token's
/// bound, **sixty-four kilobytes**.
const MOST_THE_SECRETS_ARE: usize = 64 * 1024;

/// Whether `copy`, secrets this device holds for a plugin, are the ones
/// `fields` names.
fn names(fields: &Fields, copy: &BTreeMap<String, String>) -> bool {
    digests(copy) == fields.secrets
}

/// Which secrets `values` are, key by key — see [`Fields::secrets`].
pub(crate) fn digests(values: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), digest(value)))
        .collect()
}

/// Bring this device's `secrets.yaml` into line with the plugins' records: a
/// copy fetched of every live plugin's secrets the record names and this
/// device holds no copy of, or another copy than the one it names; and the
/// copy taken away where the record's were cleared or the record removed. The
/// MCP servers' header values' way, said at
/// [`super::mcp_servers::copied`]: asked of the members in turn, the record's
/// writer first; a member not answering leaves the copy as it stands; asked
/// outside the lock a save writes under, and written inside it, each copy
/// decided again against the record and the file as they stand then.
pub(crate) async fn copied(state: &AppState) -> Result<()> {
    let wanted = wanted(state).await?;

    let mut copies = Vec::new();
    for (record, held) in wanted {
        let copy = match record.removed() || record.fields.secrets.is_empty() {
            true => None,
            false => match fetched(state, &record, held).await {
                Some(values) => Some(values),
                None => continue,
            },
        };
        copies.push((record.uuid, copy));
    }

    if copies.is_empty() {
        return Ok(());
    }

    let _writing = writing_held().await;

    let records = replicated::records::<Plugins>(&state.pool).await?;
    let settings = state.settings.clone();

    tokio::task::spawn_blocking(move || {
        let secrets = settings.secrets();

        for (uuid, copy) in copies {
            let Some(record) = records.iter().find(|record| record.uuid == uuid) else {
                continue;
            };
            let held = secrets.plugin_secrets_of(&uuid);
            let named = !record.removed() && !record.fields.secrets.is_empty();

            let values = match (copy, named) {
                (None, false) if held.is_some() => None,
                (Some(values), true) if names(&record.fields, &values) && held != Some(&values) => {
                    Some(values)
                }
                _ => continue,
            };

            settings.setting_plugin_secrets(&uuid, values)?;
        }

        Ok::<_, std::io::Error>(())
    })
    .await?
    .context("writing the copies of the plugins' secrets into secrets.yaml")
}

/// The records whose secrets this device's copy is out of line with, each
/// with the copy it holds — removed ones included, whose copy goes.
async fn wanted(
    state: &AppState,
) -> Result<Vec<(Record<Fields>, Option<BTreeMap<String, String>>)>> {
    let records = replicated::records::<Plugins>(&state.pool).await?;
    let settings = state.settings.clone();

    let wanted = tokio::task::spawn_blocking(move || {
        let secrets = settings.secrets();

        records
            .into_iter()
            .filter_map(|record| {
                let held = secrets.plugin_secrets_of(&record.uuid).cloned();
                let out_of_line = match record.removed() || record.fields.secrets.is_empty() {
                    true => held.is_some(),
                    false => held
                        .as_ref()
                        .is_none_or(|copy| !names(&record.fields, copy)),
                };
                out_of_line.then_some((record, held))
            })
            .collect()
    })
    .await?;

    Ok(wanted)
}

/// The secrets `record` names, put together from `held` — this device's own
/// copy — and the members, its writer first, or `None` where the members
/// answering do not between them hold every one.
async fn fetched(
    state: &AppState,
    record: &Record<Fields>,
    held: Option<BTreeMap<String, String>>,
) -> Option<BTreeMap<String, String>> {
    let named = &record.fields.secrets;
    let mut values: BTreeMap<String, String> = held
        .unwrap_or_default()
        .into_iter()
        .filter(|(key, value)| named.get(key) == Some(&digest(value)))
        .collect();

    let complete = |values: &BTreeMap<String, String>| values.len() == named.len();
    if complete(&values) {
        return Some(values);
    }

    let devices = state.devices.as_ref()?;
    let mut members: Vec<String> = match devices.membership().rows().await {
        Ok(members) => members.into_iter().map(|member| member.device).collect(),
        Err(why) => {
            tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so no plugin's secrets \
                 were copied",
            );
            return None;
        }
    };
    members.sort_by_key(|member| *member != record.written_by);

    for member in members {
        match secrets_at(Some(devices), &member, &record.uuid).await {
            Ok(answered) => {
                for (key, value) in answered {
                    if !values.contains_key(&key) && named.get(&key) == Some(&digest(&value)) {
                        values.insert(key, value);
                    }
                }
                if complete(&values) {
                    return Some(values);
                }
            }
            Err(why) => tracing::debug!(
                device = member,
                plugin = record.uuid,
                "a member's copy of a plugin's secrets could not be read: {}",
                why.saying,
            ),
        }
    }

    None
}

/// The secrets the member at `device` holds for the plugin `uuid`, by key and
/// empty where it holds none — or the refusal naming the machine.
async fn secrets_at(
    devices: Option<&crate::device::Devices>,
    device: &str,
    uuid: &str,
) -> Result<BTreeMap<String, String>, crate::Refusal> {
    let said: PluginSecretValues = word_from(
        devices,
        device,
        asking(SECRETS.replace("{plugin}", uuid)),
        MOST_THE_SECRETS_ARE,
    )
    .await?;

    Ok(said.secrets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::{CPP, GO, RUST};

    const RUST_CPP: &str = "795515ae-96cd-4d7c-8071-e6753b3f4624";
    const GO_PLUGIN: &str = "01b5328a-3524-457d-ab88-1af1c65079ea";

    /// A database and settings files in one fresh Data Directory, its
    /// `config.yaml` saying `config`.
    async fn fresh(config: &str) -> (tempfile::TempDir, SqlitePool, Settings) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::store::open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();
        let settings = Settings::in_data_dir(dir.path());
        std::fs::write(settings.config_path(), config).unwrap();

        (dir, pool, settings)
    }

    async fn enabled(pool: &SqlitePool, uuid: &str) -> bool {
        replicated::live_records::<Plugins>(pool)
            .await
            .unwrap()
            .into_iter()
            .find(|record| record.uuid == uuid)
            .unwrap_or_else(|| panic!("{uuid} was taken up"))
            .fields
            .enabled
    }

    async fn size(pool: &SqlitePool, uuid: &str) -> Option<String> {
        replicated::live_records::<Plugins>(pool)
            .await
            .unwrap()
            .into_iter()
            .find(|record| record.uuid == uuid)
            .and_then(|record| size_of(&record.fields).map(str::to_owned))
    }

    fn on(config: &crate::settings::Config, language: &str) -> bool {
        crate::languages::configured(config)
            .get(language)
            .unwrap()
            .enabled()
    }

    /// A plugin is taken up on where any of its languages was on — Rust off
    /// and C/C++ on comes up on — and off only where every one was off; a
    /// language with no entry is on. Once: a second start takes nothing up.
    #[tokio::test]
    async fn each_plugin_is_taken_up_on_where_any_of_its_languages_was() {
        let (_dir, pool, settings) = fresh(
            "rust_build_cache:\n  enabled: false\nlanguages:\n  cpp:\n    enabled: true\n  \
             go:\n    enabled: false\n",
        )
        .await;

        taken_up(&pool, &settings, "a").await.unwrap();

        assert!(enabled(&pool, RUST_CPP).await);
        assert!(!enabled(&pool, GO_PLUGIN).await);
        for listed in built_in::listed()
            .iter()
            .filter(|listed| built_in::languages(listed).next().is_some())
        {
            let off = listed.uuid == GO_PLUGIN;
            assert_eq!(enabled(&pool, &listed.uuid).await, !off, "{}", listed.name);
        }

        let config = settings.config();
        assert!(on(&config, RUST) && on(&config, CPP));
        assert!(
            !on(&config, GO),
            "and the settings hold Go off from here on"
        );

        // The file's `enabled` is not read again: Go on in it now changes
        // nothing, and neither does a second start.
        std::fs::write(
            settings.config_path(),
            "languages:\n  go:\n    enabled: true\n",
        )
        .unwrap();
        taken_up(&pool, &settings, "a").await.unwrap();
        assert!(!enabled(&pool, GO_PLUGIN).await);
        assert!(!on(&settings.config(), GO));
    }

    /// Two devices' take-ups come to on where either had it on, whichever
    /// arrives where; and a press made since stands over a device upgrading
    /// later.
    #[tokio::test]
    async fn on_wins_across_devices_and_a_press_since_stands() {
        let (_one, off_here, settings) = fresh("languages:\n  go:\n    enabled: false\n").await;
        taken_up(&off_here, &settings, "a").await.unwrap();

        let (_two, on_there, settings) = fresh("").await;
        taken_up(&on_there, &settings, "b").await.unwrap();

        let theirs = replicated::records::<Plugins>(&on_there).await.unwrap();
        let ours = replicated::records::<Plugins>(&off_here).await.unwrap();
        replicated::merge::<Plugins>(&off_here, &theirs)
            .await
            .unwrap();
        replicated::merge::<Plugins>(&on_there, &ours)
            .await
            .unwrap();
        assert!(
            enabled(&off_here, GO_PLUGIN).await,
            "on wins where it arrives"
        );
        assert!(enabled(&on_there, GO_PLUGIN).await, "and where it was");

        // Both off stays off.
        let (_three, off_too, settings) = fresh("languages:\n  go:\n    enabled: false\n").await;
        taken_up(&off_too, &settings, "c").await.unwrap();
        let (_four, off_also, settings) = fresh("languages:\n  go:\n    enabled: false\n").await;
        taken_up(&off_also, &settings, "d").await.unwrap();
        let theirs = replicated::records::<Plugins>(&off_also).await.unwrap();
        replicated::merge::<Plugins>(&off_too, &theirs)
            .await
            .unwrap();
        assert!(!enabled(&off_too, GO_PLUGIN).await);

        // A human unticks Go on `a`; `e`, upgrading later with Go on, is
        // merged with that and stays unticked.
        let unticked = Fields {
            enabled: false,
            ..replicated::live_records::<Plugins>(&off_here)
                .await
                .unwrap()
                .into_iter()
                .find(|record| record.uuid == GO_PLUGIN)
                .unwrap()
                .fields
        };
        replicated::write::<Plugins>(&off_here, "a", GO_PLUGIN, &unticked)
            .await
            .unwrap();
        let (_five, late, settings) = fresh("").await;
        let pressed = replicated::records::<Plugins>(&off_here).await.unwrap();
        replicated::merge::<Plugins>(&late, &pressed).await.unwrap();
        taken_up(&late, &settings, "e").await.unwrap();
        assert!(!enabled(&late, GO_PLUGIN).await);

        // And the other way about: taken up first, the press merged after.
        let (_six, later, settings) = fresh("").await;
        taken_up(&later, &settings, "f").await.unwrap();
        replicated::merge::<Plugins>(&later, &pressed)
            .await
            .unwrap();
        assert!(!enabled(&later, GO_PLUGIN).await);
    }

    /// A plugin's size is taken up as the largest any of its languages was
    /// given here — `rust_build_cache`'s read as Rust's, the map winning — and
    /// one that will not read as none; from then on the record's size is what
    /// every session reads, and the file's is not.
    #[tokio::test]
    async fn each_plugins_size_is_taken_up_and_read_from_the_record() {
        let (_dir, pool, settings) = fresh(
            "rust_build_cache:\n  size: 50G\nlanguages:\n  rust:\n    size: 40G\n  \
             go:\n    size: 20G\n  node:\n    size: 1.5g\n",
        )
        .await;

        taken_up(&pool, &settings, "a").await.unwrap();

        assert_eq!(size(&pool, RUST_CPP).await.as_deref(), Some("40G"));
        assert_eq!(size(&pool, GO_PLUGIN).await.as_deref(), Some("20G"));
        let node = built_in::providing(crate::languages::NODE).unwrap();
        assert_eq!(size(&pool, &node.uuid).await, None, "1.5g is no size");

        // What a session reads is the record's: the file changed since
        // changes nothing.
        std::fs::write(settings.config_path(), "languages:\n  go:\n    size: 99G\n").unwrap();
        let languages = crate::languages::configured(&settings.config());
        assert_eq!(languages.get(RUST).unwrap().size(RUST), "40G");
        assert_eq!(languages.get(CPP).unwrap().size(CPP), "40G");
        assert_eq!(languages.get(GO).unwrap().size(GO), "20G");
        assert_eq!(
            languages
                .get(crate::languages::NODE)
                .unwrap()
                .size(crate::languages::NODE),
            "10G",
        );

        // And `rust_build_cache` alone is Rust's.
        let (_old, old, settings) = fresh("rust_build_cache:\n  size: 50G\n").await;
        taken_up(&old, &settings, "b").await.unwrap();
        assert_eq!(size(&old, RUST_CPP).await.as_deref(), Some("50G"));
    }

    /// An installer's own language is taken up the first time a device reads
    /// its entry, under the uuid derived from its name: two devices' take-ups
    /// come to on where either had it on, at the larger size; both off stays
    /// off; and from then on the record, not the entry, is what sessions read.
    #[tokio::test]
    async fn an_installers_language_is_taken_up_by_name_on_and_larger_winning() {
        let zig = crate::plugins::installed::installer_uuid("zig");
        let entry = |more: &str| {
            format!("languages:\n  zig:\n    env:\n      Z: \"{{cache}}/zig\"\n{more}")
        };

        let (_one, off_here, settings) = fresh(&entry("    enabled: false\n    size: 5G\n")).await;
        taken_up(&off_here, &settings, "a").await.unwrap();
        assert!(!enabled(&off_here, &zig).await);
        assert!(!on(&settings.config(), "zig"));

        // A device whose file had no entry takes nothing up of it.
        let (_none, without, settings) = fresh("").await;
        taken_up(&without, &settings, "c").await.unwrap();
        assert!(
            replicated::live_records::<Plugins>(&without)
                .await
                .unwrap()
                .iter()
                .all(|record| record.uuid != zig)
        );

        // Nor, once taken up, a second time: the entry written on since is
        // not read.
        let (_late, late, settings) = fresh("").await;
        taken_up(&late, &settings, "d").await.unwrap();
        std::fs::write(settings.config_path(), entry("    enabled: false\n")).unwrap();
        taken_up(&late, &settings, "d").await.unwrap();
        assert!(!enabled(&late, &zig).await, "taken up when first read");
        std::fs::write(settings.config_path(), entry("")).unwrap();
        taken_up(&late, &settings, "d").await.unwrap();
        assert!(!enabled(&late, &zig).await, "and not again");
        assert!(!on(&settings.config(), "zig"), "the record is what is read");

        let (_two, on_there, settings) = fresh(&entry("    size: 8G\n")).await;
        taken_up(&on_there, &settings, "b").await.unwrap();

        let theirs = replicated::records::<Plugins>(&on_there).await.unwrap();
        replicated::merge::<Plugins>(&off_here, &theirs)
            .await
            .unwrap();
        assert!(enabled(&off_here, &zig).await, "on wins");
        assert_eq!(size(&off_here, &zig).await.as_deref(), Some("8G"));

        let (_three, off_too, settings) = fresh(&entry("    enabled: false\n")).await;
        taken_up(&off_too, &settings, "e").await.unwrap();
        let ours = replicated::records::<Plugins>(&late).await.unwrap();
        replicated::merge::<Plugins>(&off_too, &ours).await.unwrap();
        assert!(!enabled(&off_too, &zig).await, "both off stays off");
    }

    /// Two devices' sizes take up as the larger, whichever arrives where; and
    /// a size saved since stands over a device upgrading later.
    #[tokio::test]
    async fn the_larger_size_wins_across_devices_and_a_save_since_stands() {
        let (_one, smaller, settings) = fresh("languages:\n  go:\n    size: 20G\n").await;
        taken_up(&smaller, &settings, "a").await.unwrap();
        let (_two, larger_here, settings) = fresh("languages:\n  go:\n    size: 40G\n").await;
        taken_up(&larger_here, &settings, "b").await.unwrap();
        let (_three, none_said, settings) = fresh("").await;
        taken_up(&none_said, &settings, "z").await.unwrap();

        let theirs = replicated::records::<Plugins>(&larger_here).await.unwrap();
        let ours = replicated::records::<Plugins>(&smaller).await.unwrap();
        let nobodys = replicated::records::<Plugins>(&none_said).await.unwrap();
        replicated::merge::<Plugins>(&smaller, &theirs)
            .await
            .unwrap();
        replicated::merge::<Plugins>(&larger_here, &ours)
            .await
            .unwrap();
        replicated::merge::<Plugins>(&none_said, &ours)
            .await
            .unwrap();
        replicated::merge::<Plugins>(&smaller, &nobodys)
            .await
            .unwrap();
        assert_eq!(size(&smaller, GO_PLUGIN).await.as_deref(), Some("40G"));
        assert_eq!(size(&larger_here, GO_PLUGIN).await.as_deref(), Some("40G"));
        assert_eq!(
            size(&none_said, GO_PLUGIN).await.as_deref(),
            Some("20G"),
            "a size beats none",
        );

        // A human saves 5G on `a`; `c`, upgrading later at 80G, stays at 5G.
        let mut saved = replicated::live_records::<Plugins>(&smaller)
            .await
            .unwrap()
            .into_iter()
            .find(|record| record.uuid == GO_PLUGIN)
            .unwrap()
            .fields;
        saved
            .values
            .insert(SIZE_SETTING.to_owned(), SettingValue::Line("5G".to_owned()));
        replicated::write::<Plugins>(&smaller, "a", GO_PLUGIN, &saved)
            .await
            .unwrap();
        let pressed = replicated::records::<Plugins>(&smaller).await.unwrap();
        let (_four, late, settings) = fresh("languages:\n  go:\n    size: 80G\n").await;
        taken_up(&late, &settings, "c").await.unwrap();
        replicated::merge::<Plugins>(&late, &pressed).await.unwrap();
        assert_eq!(size(&late, GO_PLUGIN).await.as_deref(), Some("5G"));
    }
}
