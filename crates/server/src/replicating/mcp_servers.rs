//! **The declared MCP servers as records** (ADR-0028, amending ADR-0021): every
//! declaration is a record of the database's replicated table, the cluster's
//! rather than a key of each device's `config.yaml`.
//!
//! **One record per declaration, under a uuid.** Its fields are the name, the
//! URL and the header *names*; the values are secrets, kept in `secrets.yaml`
//! under the same uuid. The name stays what the agent and the chips see, and a
//! server is still never renamed — but removing one is a deletion stamp, and
//! **declaring a removed name again is a new record** under a fresh uuid, so a
//! removal is final here as it is for every kind.
//!
//! **Taken up out of `config.yaml` once**, at the serve — see [`taken_up`] —
//! and never read there again:
//!
//! - **A declaration taken up gets a uuid derived from its name** — see
//!   [`derived_uuid`] — so `docs` declared by hand on two devices becomes one
//!   record rather than two of a name. Only at the move: every declaration
//!   made after it gets a fresh random one.
//! - **It is stamped older than any edit**, as the Instructions are, the
//!   Device Id deciding between two devices' taken-up copies. A name removed
//!   since stays removed when a late device takes up its own copy, the merge
//!   keeping the deletion stamp.
//! - **Its header values are rekeyed** from the name to the uuid in the same
//!   move — see [`Settings::rekeying_the_mcp_headers`].
//!
//! **An Annex takes nothing up**: its `mcp_servers:` key is its Keeper's old
//! copy, dropped rather than taken up, and the Annex gets the declarations as
//! a member, like any other.
//!
//! **Names are unique at the press**, against every live record this device
//! holds, whoever wrote it — see [`refusals`]. Two of a name can only arrive
//! by replication, from two devices that each declared it apart; both are kept
//! and drawn, and a launch hands neither until one is removed — see
//! [`crate::settings::Config::declaration_of`].
//!
//! **Held in memory for the settings**, as the Authors and the Instructions
//! are: every write here and every merge hands the live declarations on to
//! [`Settings::holding_mcp_servers`].
//!
//! **The header values travel from whoever holds them**, as an Author's token
//! does — see [`copied`]. The record carries a digest of each value, never the
//! value, so setting, rotating or clearing one is a write to the record, which
//! is what tells the other devices to fetch. Keyed by the uuid on every device,
//! so deleting `docs` and declaring it again never has the old removal take
//! the new values away, whichever order a device pulls the two in.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sqlx::SqlitePool;

use verkstead_render::McpHeaderValues;
use verkstead_schema::Nudge;

use super::Replicating;
use super::authors::{digest, writing_held};
use crate::AppState;
use crate::peer::mcp_servers::HEADERS;
use crate::relaying::{Read, asking, word_from};
use crate::settings::{HeaderValue, McpServer, ServerTrouble, Settings, trouble};
use crate::store::replicated::{self, Kind, Written};

/// The declared MCP servers, as a kind of record.
pub struct McpServers;

impl Kind for McpServers {
    const NAME: &'static str = "mcp_servers";
    type Fields = Fields;
}

impl Replicating for McpServers {
    const MOVED: Nudge = Nudge::McpServers;
    const READ: Read = Read::McpServers;

    /// The merged declarations held for the settings — taken up first, where a
    /// server stood up without a serve has not — and the pages told.
    async fn merged(state: &AppState) -> Result<()> {
        taken_up(&state.pool, &state.settings, &state.device).await?;
        held(&state.pool, &state.settings).await?;

        state.nudges.announce_here(Nudge::McpServers);

        Ok(())
    }

    /// Every declaration's values the records say are held and this device
    /// holds no current copy of fetched, and every copy of values cleared or
    /// of a declaration removed taken away.
    async fn pulled(state: &AppState) -> Result<()> {
        copied(state).await
    }
}

/// The most a member's answer about one declaration's values may be: the
/// token's bound, **sixty-four kilobytes**.
const MOST_THE_VALUES_ARE: usize = 64 * 1024;

/// What a declaration's record holds besides its own stamps: the name, the URL
/// and the names of the headers it is spoken to with, in the order they are
/// sent. Never a header's value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fields {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<String>,

    /// Which value is held for each header, by its name: a digest of the value
    /// — see [`digest`] — so a device can tell whether its copy is the one the
    /// record names without the value crossing on the record. A header with no
    /// value held anywhere is not here, and neither is any header of a
    /// declaration with none.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
}

impl Fields {
    fn of(server: &McpServer) -> Option<Fields> {
        Some(Fields {
            name: server.name()?.to_owned(),
            url: server.url()?.to_owned(),
            headers: server.headers().to_vec(),
            values: BTreeMap::new(),
        })
    }

    /// And with the values `held` says are held, for the headers these fields
    /// still declare: an edit that said nothing about a value says what the
    /// record said, which a device holding no copy of it can make.
    fn keeping_the_values_of(self, held: &Fields) -> Fields {
        let values = held
            .values
            .iter()
            .filter(|(header, _)| self.headers.contains(header))
            .map(|(header, digest)| (header.clone(), digest.clone()))
            .collect();

        Fields { values, ..self }
    }

    /// Whether `copy`, values this device holds for it, are the ones it names.
    fn names(&self, copy: &BTreeMap<String, String>) -> bool {
        digests(copy) == self.values
    }
}

/// Which values `values` are, header by header — see [`Fields::values`].
fn digests(values: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(header, value)| (header.clone(), digest(value)))
        .collect()
}

/// The namespace the uuids taken up at the move are derived in, the same
/// constant on every device.
const NAMESPACE: [u8; 16] = [
    0x6a, 0x2f, 0x0c, 0x91, 0x5e, 0x3d, 0x4b, 0x7a, 0x9f, 0x18, 0xc4, 0x2e, 0x70, 0xd5, 0x8b, 0x13,
];

/// The uuid a declaration named `name` is taken up under: a name-based,
/// version-5 uuid in [`NAMESPACE`], so that two devices taking up `docs` take
/// up the one record.
pub fn derived_uuid(name: &str) -> String {
    let mut hashing = Sha1::new();
    hashing.update(NAMESPACE);
    hashing.update(name.as_bytes());
    let hash = hashing.finalize();

    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);

    // Version 5 in the high nibble of the seventh byte and the variant in the
    // top bits of the ninth.
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let mut uuid = String::with_capacity(36);

    for (at, byte) in bytes.iter().enumerate() {
        if matches!(at, 4 | 6 | 8 | 10) {
            uuid.push('-');
        }

        uuid.push_str(&format!("{byte:02x}"));
    }

    uuid
}

/// Every live declaration this device holds, each carrying its record's uuid,
/// in the order they were first written down here.
pub async fn declared(pool: &SqlitePool) -> Result<Vec<McpServer>> {
    Ok(replicated::live_records::<McpServers>(pool)
        .await?
        .into_iter()
        .map(|record| {
            McpServer::of(
                Some(record.fields.name),
                Some(record.fields.url),
                record.fields.headers,
            )
            .recorded_as(record.uuid)
        })
        .collect())
}

/// Take the declarations up out of `config.yaml` into the table, where they
/// have not been already, and hold the live records for the settings either
/// way.
///
/// **Once**, the database first and the file second, as the Instructions are
/// — see [`super::instructions::taken_up`]. And the header values before
/// either, so that a start stopped part way through finds them under the uuid
/// the move is about to give each declaration, or under the name still. A file
/// that will not parse is left alone and nothing taken up until the human
/// fixes it.
pub async fn taken_up(pool: &SqlitePool, settings: &Settings, device: &str) -> Result<()> {
    if settings.holds_mcp_servers() {
        return Ok(());
    }

    if !replicated::taken_up::<McpServers>(pool).await? {
        let settings_ = settings.clone();
        let read = tokio::task::spawn_blocking(move || settings_.mcp_servers_as_written()).await?;

        if let Some((written, annex)) = read {
            // Nothing taken up of an Annex's, which is its Keeper's old copy.
            let kept: Vec<(String, Fields)> = match annex {
                true => Vec::new(),
                false => written
                    .iter()
                    .filter_map(|server| {
                        let fields = Fields::of(server)?;
                        Some((derived_uuid(&fields.name), fields))
                    })
                    .collect(),
            };

            let moved: Vec<(String, String)> = kept
                .iter()
                .map(|(uuid, fields)| (fields.name.clone(), uuid.clone()))
                .collect();

            let settings_ = settings.clone();
            tokio::task::spawn_blocking(move || settings_.rekeying_the_mcp_headers(&moved))
                .await?
                .context("keeping the MCP servers' header values under their uuids")?;

            // Each record saying which values the file holds for it, now they
            // are under its uuid.
            let settings_ = settings.clone();
            let kept = tokio::task::spawn_blocking(move || {
                let secrets = settings_.secrets();

                kept.into_iter()
                    .map(|(uuid, fields)| {
                        let values = secrets.mcp_headers_of(&uuid).map(digests);
                        let fields = Fields {
                            values: values.unwrap_or_default(),
                            ..fields
                        };

                        (uuid, fields)
                    })
                    .collect::<Vec<_>>()
            })
            .await?;

            replicated::take_up_as_the_oldest::<McpServers>(pool, device, &kept)
                .await
                .context("taking the MCP servers up out of config.yaml")?;

            if !kept.is_empty() {
                tracing::info!(
                    "the MCP servers are records in the database now, out of config.yaml"
                );
            }

            let settings = settings.clone();
            tokio::task::spawn_blocking(move || settings.dropping_the_mcp_servers())
                .await?
                .context("taking the `mcp_servers:` key out of config.yaml")?;
        }
    }

    held(pool, settings).await
}

/// Hand the live declarations on to the settings, which is what the Built
/// Root, the settings page and the Attach menu read them out of.
pub async fn held(pool: &SqlitePool, settings: &Settings) -> Result<()> {
    settings.holding_mcp_servers(declared(pool).await?);

    Ok(())
}

/// One row of the settings section as a save sends it: the uuid of the record
/// it was drawn from, or `None` for a row added on the page, and the
/// declaration as it is to stand.
#[derive(Debug, Clone)]
pub struct Row {
    pub id: Option<String>,
    pub server: McpServer,
}

/// What would stop each of `rows` being written down, by where it stands among
/// them — and empty where there is nothing wrong with any of them.
///
/// **A new row's name is checked against every live record** that stands after
/// the save, whoever wrote it — the ones the page drew and the ones it did not,
/// a record merged after the page was read among them — and against the rows
/// above it. A row drawn from a record keeps that record's name, which is never
/// changed, so only its URL can be wrong.
pub fn refusals(
    live: &[McpServer],
    rows: &[Row],
    removed: &[String],
) -> Vec<(usize, ServerTrouble)> {
    let sent = |id: &str| rows.iter().any(|row| row.id.as_deref() == Some(id));

    let mut above: Vec<McpServer> = live
        .iter()
        .filter(|server| {
            server
                .id()
                .is_none_or(|id| !sent(id) && !removed.iter().any(|gone| gone == id))
        })
        .cloned()
        .collect();

    let mut refused = Vec::new();

    for (at, row) in rows.iter().enumerate() {
        let record = row
            .id
            .as_deref()
            .and_then(|id| live.iter().find(|server| server.id() == Some(id)));

        let trouble = match record {
            Some(record) => {
                let standing = McpServer::of(
                    record.name().map(str::to_owned),
                    row.server.url().map(str::to_owned),
                    row.server.headers().to_vec(),
                );
                let trouble = trouble(&standing, &[])
                    .filter(|trouble| matches!(trouble, ServerTrouble::Url(_)));

                above.push(standing);
                trouble
            }

            // A row whose record has since been removed elsewhere is written
            // nowhere — a removal is final — so nothing about it is refused.
            None if row.id.is_some() => None,

            None => {
                let trouble = trouble(&row.server, &above);
                above.push(row.server.clone());
                trouble
            }
        };

        if let Some(trouble) = trouble {
            refused.push((at, trouble));
        }
    }

    refused
}

/// Write what a save of the section sent, made here by `device`, once
/// [`refusals`] has found nothing wrong with it: each row drawn from a record
/// an edit of that record, each new row a new record under a fresh uuid, and
/// each of `removed` a deletion stamp. Answers the uuid each row is written
/// under — `None` for one whose record was removed meanwhile, which is final —
/// and whether anything was written.
pub async fn saved(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    rows: &[Row],
    removed: &[String],
) -> Result<(Vec<Option<String>>, bool)> {
    taken_up(pool, settings, device).await?;

    let live = replicated::live_records::<McpServers>(pool).await?;
    let mut moved = false;
    let mut written = Vec::with_capacity(rows.len());

    for row in rows {
        let record = row
            .id
            .as_deref()
            .and_then(|id| live.iter().find(|record| record.uuid == id));

        let uuid = match (&row.id, record) {
            // Its URL and headers as sent, and its name as the record has it:
            // a server is never renamed. And its values as the record says
            // until [`valued`] says otherwise.
            (Some(id), Some(record)) => {
                let standing = McpServer::of(
                    Some(record.fields.name.clone()),
                    row.server.url().map(str::to_owned),
                    row.server.headers().to_vec(),
                );

                if let Some(fields) =
                    Fields::of(&standing).map(|fields| fields.keeping_the_values_of(&record.fields))
                {
                    moved |= replicated::rewrite::<McpServers>(pool, device, id, &fields).await?
                        == Written::Written;
                }

                Some(id.clone())
            }

            (Some(_), None) => None,

            (None, _) => {
                let Some(fields) = Fields::of(&row.server) else {
                    written.push(None);
                    continue;
                };
                let uuid = crate::settings::minted_uuid()?;

                replicated::write::<McpServers>(pool, device, &uuid, &fields).await?;
                moved = true;

                Some(uuid)
            }
        };

        written.push(uuid);
    }

    for id in removed {
        moved |= replicated::remove::<McpServers>(pool, device, id).await?;
    }

    held(pool, settings).await?;

    Ok((written, moved))
}

/// Write onto each declaration's record which values `secrets.yaml` now holds
/// for it, once a save has written the values `touched` says — by uuid, each
/// header's action as [`crate::settings::Secrets::with_mcp_headers`] applied
/// it. Whether anything was written.
///
/// **A header set or cleared says what the file holds now**, and one the save
/// left alone keeps what the record said: a device holding no copy of a value
/// is no word about it. So a declaration nothing was set or cleared on is not
/// written at all.
///
/// The caller holds the lock the copying writes under across the file and
/// this — see [`super::authors::writing_held`] — so the copying never reads
/// one written and the other not.
pub async fn valued(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    touched: &[(String, Vec<(String, HeaderValue)>)],
) -> Result<bool> {
    let live = replicated::live_records::<McpServers>(pool).await?;
    let secrets = {
        let settings = settings.clone();
        tokio::task::spawn_blocking(move || settings.secrets()).await?
    };

    let mut moved = false;

    for (uuid, actions) in touched {
        if actions
            .iter()
            .all(|(_, action)| matches!(action, HeaderValue::Keep))
        {
            continue;
        }

        let Some(record) = live.iter().find(|record| &record.uuid == uuid) else {
            continue;
        };

        let values = record
            .fields
            .headers
            .iter()
            .filter_map(|header| {
                let action = actions
                    .iter()
                    .find(|(named, _)| named == header)
                    .map(|(_, action)| action);

                let digest = match action {
                    None | Some(HeaderValue::Keep) => record.fields.values.get(header).cloned(),
                    Some(HeaderValue::Set(_) | HeaderValue::Clear) => {
                        secrets.mcp_header(uuid, header).map(digest)
                    }
                };

                Some((header.clone(), digest?))
            })
            .collect();

        let fields = Fields {
            values,
            ..record.fields.clone()
        };

        moved |= replicated::rewrite::<McpServers>(pool, device, uuid, &fields).await?
            == Written::Written;
    }

    if moved {
        held(pool, settings).await?;
    }

    Ok(moved)
}

/// Bring this device's `secrets.yaml` into line with the records: a copy
/// fetched of the values of every live declaration whose record names values
/// this device holds no copy of, or another copy than the one it names; and
/// the copy taken away where the record's values were cleared or the record
/// removed. Run after every pull, whatever it merged, so values that could not
/// be fetched last time are asked for again — an Author's token's way, said
/// at [`super::authors::copied`].
///
/// **Asked of the members in turn**, the record's writer first, and put
/// together header by header: each value whose digest is the record's is
/// taken, from this device's own copy first, and the copy is written once
/// every header the record names has one. **A member not answering leaves the
/// copy as it stands**, and it is used.
///
/// **The asking outside the lock the save writes under and the writing inside
/// it** — see [`super::authors::writing_held`] — each copy decided again against the record and the file as they
/// stand then: values set here between the two are this device's own, and
/// nothing fetched or cleared before it lands over them.
pub(crate) async fn copied(state: &AppState) -> Result<()> {
    let wanted = wanted(state).await?;

    if wanted.is_empty() {
        return Ok(());
    }

    let mut copies = Vec::new();

    for (record, held) in wanted {
        let copy = match record.removed() || record.fields.values.is_empty() {
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

    let records = replicated::records::<McpServers>(&state.pool).await?;
    let settings = state.settings.clone();

    let moved = tokio::task::spawn_blocking(move || {
        let secrets = settings.secrets();
        let mut moved = false;

        for (uuid, copy) in copies {
            let Some(record) = records.iter().find(|record| record.uuid == uuid) else {
                continue;
            };
            let held = secrets.mcp_headers_of(&uuid);
            let named = !record.removed() && !record.fields.values.is_empty();

            let values = match (copy, named) {
                (None, false) if held.is_some() => None,
                (Some(values), true) if record.fields.names(&values) && held != Some(&values) => {
                    Some(values)
                }
                _ => continue,
            };

            settings.setting_mcp_headers(&uuid, values)?;
            moved = true;
        }

        Ok::<_, std::io::Error>(moved)
    })
    .await?
    .context("writing the copies of the MCP servers' header values into secrets.yaml")?;

    if moved {
        state.nudges.announce_here(Nudge::McpServers);
    }

    Ok(())
}

/// The records whose values this device's copy is out of line with, each with
/// the copy it holds — removed ones included, whose copy goes.
async fn wanted(
    state: &AppState,
) -> Result<Vec<(replicated::Record<Fields>, Option<BTreeMap<String, String>>)>> {
    let records = replicated::records::<McpServers>(&state.pool).await?;
    let settings = state.settings.clone();

    let wanted = tokio::task::spawn_blocking(move || {
        let secrets = settings.secrets();

        records
            .into_iter()
            .filter_map(|record| {
                let held = secrets.mcp_headers_of(&record.uuid).cloned();

                let out_of_line = match record.removed() || record.fields.values.is_empty() {
                    true => held.is_some(),
                    false => held.as_ref().is_none_or(|copy| !record.fields.names(copy)),
                };

                out_of_line.then_some((record, held))
            })
            .collect()
    })
    .await?;

    Ok(wanted)
}

/// The values `record` names, put together from `held` — this device's own
/// copy — and the members, its writer first, or `None` where the members
/// answering do not between them hold every one.
async fn fetched(
    state: &AppState,
    record: &replicated::Record<Fields>,
    held: Option<BTreeMap<String, String>>,
) -> Option<BTreeMap<String, String>> {
    let named = &record.fields.values;

    let mut values: BTreeMap<String, String> = held
        .unwrap_or_default()
        .into_iter()
        .filter(|(header, value)| named.get(header) == Some(&digest(value)))
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
                "the devices this one is linked to could not be read, so no MCP server's \
                 header values were copied",
            );
            return None;
        }
    };

    members.sort_by_key(|member| *member != record.written_by);

    for member in members {
        match values_at(Some(devices), &member, &record.uuid).await {
            Ok(answered) => {
                for (header, value) in answered {
                    if !values.contains_key(&header) && named.get(&header) == Some(&digest(&value))
                    {
                        values.insert(header, value);
                    }
                }

                if complete(&values) {
                    return Some(values);
                }
            }

            Err(why) => tracing::debug!(
                device = member,
                server = record.uuid,
                "a member's copy of an MCP server's header values could not be read: {}",
                why.saying,
            ),
        }
    }

    None
}

/// The header values the member at `device` holds for the declaration `id`, by
/// header name and empty where it holds none — or the refusal naming the
/// machine, which is a member not answering, a caller it holds no membership
/// for, or a uuid it holds no live record of.
///
/// Public for the suite that stands devices up and asks it.
pub async fn values_at(
    devices: Option<&crate::device::Devices>,
    device: &str,
    id: &str,
) -> Result<BTreeMap<String, String>, crate::Refusal> {
    let said: McpHeaderValues = word_from(
        devices,
        device,
        asking(HEADERS.replace("{server}", id)),
        MOST_THE_VALUES_ARE,
    )
    .await?;

    Ok(said.headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A database and settings files in one fresh Data Directory.
    async fn fresh() -> (tempfile::TempDir, SqlitePool, Settings) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::store::open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();
        let settings = Settings::in_data_dir(dir.path());

        (dir, pool, settings)
    }

    fn row(id: Option<&str>, name: &str, url: &str) -> Row {
        Row {
            id: id.map(str::to_owned),
            server: McpServer::of(Some(name.to_owned()), Some(url.to_owned()), Vec::new()),
        }
    }

    #[test]
    fn a_derived_uuid_is_a_version_5_uuid_and_the_same_on_every_device() {
        let docs = derived_uuid("docs");

        assert_eq!(docs, derived_uuid("docs"));
        assert_ne!(docs, derived_uuid("tracker"));
        assert_eq!(docs.len(), 36);
        assert_eq!(&docs[14..15], "5");
        assert!(matches!(&docs[19..20], "8" | "9" | "a" | "b"), "{docs}");
    }

    /// An installation with `mcp_servers:` and `mcp_headers` starts with the
    /// same servers out of the database, under name-derived uuids stamped
    /// older than any edit, its header values rekeyed under them, and the key
    /// gone from the file; a hand-edited key afterwards is ignored.
    #[tokio::test]
    async fn the_declarations_in_config_yaml_are_taken_up_and_their_values_rekeyed() {
        let (dir, pool, settings) = fresh().await;

        std::fs::write(
            settings.config_path(),
            "mcp_servers:\n- name: docs\n  url: https://docs.example.com/mcp\n  headers: \
             [Authorization]\nshare_on_done: true\n",
        )
        .unwrap();
        std::fs::write(
            settings.secrets_path(),
            "author_tokens:\n  work: ghp_x\nmcp_headers:\n  docs:\n    Authorization: Bearer abc\n",
        )
        .unwrap();

        taken_up(&pool, &settings, "this-device").await.unwrap();

        let docs = derived_uuid("docs");
        let records = replicated::records::<McpServers>(&pool).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].uuid, docs);
        assert_eq!(
            (records[0].updated_at, records[0].written_by.as_str()),
            (0, "this-device")
        );
        assert_eq!(
            records[0].fields.values.get("Authorization"),
            Some(&digest("Bearer abc")),
            "the record says which value the file holds, and not the value"
        );

        let config = settings.config();
        assert_eq!(config.mcp_servers().len(), 1);
        assert_eq!(config.mcp_servers()[0].id(), Some(docs.as_str()));

        let secrets = settings.secrets();
        assert_eq!(
            secrets.mcp_header(&docs, "Authorization"),
            Some("Bearer abc")
        );
        assert_eq!(secrets.mcp_header("docs", "Authorization"), None);
        assert_eq!(secrets.author_token("work"), Some("ghp_x"));
        assert_eq!(
            config.attached_among(&["docs".to_owned()], &secrets)[0].headers(),
            [("Authorization".to_owned(), "Bearer abc".to_owned())]
        );

        let written = std::fs::read_to_string(settings.config_path()).unwrap();
        assert!(!written.contains("mcp_servers"), "{written}");
        assert!(written.contains("share_on_done: true"), "{written}");

        std::fs::write(
            settings.config_path(),
            "mcp_servers:\n- name: byhand\n  url: https://example.com\n",
        )
        .unwrap();
        assert_eq!(settings.config().mcp_servers().len(), 1);

        let restarted = Settings::in_data_dir(dir.path());
        restarted.forgetting_the_mcp_servers();
        taken_up(&pool, &restarted, "this-device").await.unwrap();
        assert_eq!(
            restarted.config().mcp_servers()[0].name(),
            Some("docs"),
            "taken up once"
        );
    }

    /// An Annex's `mcp_servers:` key is its Keeper's old copy: dropped from
    /// the file and not taken up, and its secrets left as the Keeper wrote
    /// them.
    #[tokio::test]
    async fn an_annex_takes_nothing_up() {
        let (_dir, pool, settings) = fresh().await;

        std::fs::write(
            settings.config_path(),
            "mcp_servers:\n- name: docs\n  url: https://docs.example.com/mcp\n\
             kept_by:\n  device: a\n  fingerprint: f\n",
        )
        .unwrap();

        taken_up(&pool, &settings, "this-device").await.unwrap();

        assert!(
            replicated::records::<McpServers>(&pool)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(settings.config().mcp_servers().is_empty());

        let written = std::fs::read_to_string(settings.config_path()).unwrap();
        assert!(!written.contains("mcp_servers"), "{written}");
        assert!(written.contains("kept_by"), "{written}");
    }

    /// Removing a declaration and declaring its name again makes a new record
    /// under a new uuid, the old one kept with its deletion stamp; an edit
    /// keeps the record's name whatever the row says.
    #[tokio::test]
    async fn a_name_declared_again_is_a_new_record() {
        let (_dir, pool, settings) = fresh().await;

        let (first, moved) = saved(
            &pool,
            &settings,
            "this-device",
            &[row(None, "docs", "https://one.example.com")],
            &[],
        )
        .await
        .unwrap();
        assert!(moved);
        let first = first[0].clone().unwrap();

        let (_, moved) = saved(
            &pool,
            &settings,
            "this-device",
            &[row(Some(&first), "renamed", "https://two.example.com")],
            &[],
        )
        .await
        .unwrap();
        assert!(moved);
        assert_eq!(settings.config().mcp_servers()[0].name(), Some("docs"));
        assert_eq!(
            settings.config().mcp_servers()[0].url(),
            Some("https://two.example.com")
        );

        let (_, moved) = saved(
            &pool,
            &settings,
            "this-device",
            &[row(Some(&first), "docs", "https://two.example.com")],
            &[],
        )
        .await
        .unwrap();
        assert!(!moved, "a save that changes nothing writes nothing");

        let (second, _) = saved(
            &pool,
            &settings,
            "this-device",
            &[row(None, "docs", "https://three.example.com")],
            std::slice::from_ref(&first),
        )
        .await
        .unwrap();
        let second = second[0].clone().unwrap();

        assert_ne!(first, second);

        let records = replicated::records::<McpServers>(&pool).await.unwrap();
        assert!(
            records
                .iter()
                .find(|record| record.uuid == first)
                .unwrap()
                .removed()
        );
        assert_eq!(settings.config().mcp_servers().len(), 1);
        assert_eq!(
            settings.config().mcp_servers()[0].id(),
            Some(second.as_str())
        );
    }

    /// A new row named as a live record the page never drew — one merged from
    /// another device — is refused, and so is one named as a row above it;
    /// the record's own row and one removed in the same save are not.
    #[test]
    fn a_new_name_is_refused_against_every_live_record() {
        let live = [
            McpServer::of(Some("docs".into()), Some("https://a".into()), Vec::new())
                .recorded_as("u-docs".into()),
            McpServer::of(Some("tracker".into()), Some("https://b".into()), Vec::new())
                .recorded_as("u-tracker".into()),
        ];

        let refused = refusals(&live, &[row(None, "docs", "https://c")], &[]);
        assert_eq!(refused.len(), 1);
        assert!(matches!(refused[0], (0, ServerTrouble::Name(_))));

        assert!(refusals(&live, &[row(None, "docs", "https://c")], &["u-docs".into()]).is_empty());

        let refused = refusals(
            &live,
            &[
                row(Some("u-docs"), "docs", "https://a"),
                row(None, "notes", "https://n"),
                row(None, "notes", "https://m"),
            ],
            &[],
        );
        assert_eq!(refused.len(), 1);
        assert!(matches!(refused[0], (2, ServerTrouble::Name(_))));

        let refused = refusals(&live, &[row(Some("u-docs"), "docs", "")], &[]);
        assert!(matches!(refused[..], [(0, ServerTrouble::Url(_))]));
    }
}
