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
//! **What loading it came to is not**: a plugin's trouble is this device's
//! reading of its own copy, held in memory — see [`crate::plugins::installed`].
//! A merge here is what has it loaded again.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use verkstead_render::{PluginCapability, PluginSecretValues};
use verkstead_schema::Nudge;

use super::Replicating;
use super::authors::{digest, writing_held};
use crate::AppState;
use crate::peer::plugins::SECRETS;
use crate::plugins::manifest::SettingKind;
use crate::relaying::{Read, asking, word_from};
use crate::store::replicated::{self, Kind, Record};

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
