//! **A Plugin's settings pane** (ADR-0030, Set 1769, Q43): the fields its
//! manifest declares, each drawn by kind — `text`, `size`, `list` and `secret`
//! — and a save of what was typed into them.
//!
//! **The values are the cluster's.** A `text`, `size` or `list` value is
//! written on the plugin's record under the field's key, and reaches every
//! device as the record does. A `secret` goes where an MCP server's header
//! values go: into `secrets.yaml` under the plugin's uuid, never back to the
//! viewer — which learns only whether one is set — and the record carries a
//! digest of it, which is what every other device fetches its own copy by —
//! see [`crate::replicating::plugins::copied`].
//!
//! **The fields are the record's**, as the manifest declared them when last
//! read at a press, so the pane is drawn on every device, and for a plugin in
//! trouble too: a setting a later stage's Category needs in order to load can
//! be put right here.
//!
//! **Only the server decides what a value may be.** A size it cannot read is
//! refused with the reason at the field, and a save refused over any field
//! writes none of them.
//!
//! **A built-in's pane is its embedded manifest's**, record or none: the
//! fields are the ones this binary declares, and the values the record's where
//! one has been written. A save is what makes the record of a built-in nobody
//! has touched, the way its first tick does. A Language plugin's `size` is
//! what `{size}` reads for each language it provides — see
//! [`crate::languages::configured`] — and its placeholder is that language's
//! default.
//!
//! **And so is an installer's own language's**, drawn as a plugin of its own:
//! its fields are the size field where its entry names a store, read off the
//! entry rather than any record, and its first save makes its record — see
//! [`super::installed::installer_fields`].

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use time::format_description::well_known::Rfc3339;
use verkstead_render::{
    PluginField, PluginFieldEdit, PluginFieldRefused, PluginFieldValue, PluginFieldWrite,
    PluginLanguageCache, PluginSettingsSave, PluginSettingsSaved, PluginSettingsView,
};
use verkstead_schema::Nudge;

use super::built_in;
use super::installed::{
    built_in_fields, failed, held, installer_fields, installer_uuid, installers, on_this_device,
};
use super::manifest::SettingKind;
use crate::AppState;
use crate::languages::SIZE_SETTING;
use crate::replicating::authors::writing_held;
use crate::replicating::plugins::{Declared, Fields, Plugins, SettingValue, digests};
use crate::store::replicated::{self, Written};

/// The plugin a pane is of: its uuid and its record's fields, a built-in's
/// read off its embedded manifest — see [`held_or_built_in`].
struct Held {
    uuid: String,
    fields: Fields,
    /// Whether there is a record yet: a built-in nobody has written to has
    /// none.
    recorded: bool,
}

/// The pane's plugin under `uuid`: a built-in's fields as its embedded
/// manifest declares them, with its record's switch and values where it has
/// one and on with none set where it has not; any other plugin's record, or
/// the refusal naming it as not installed.
async fn held_or_built_in(state: &AppState, uuid: &str) -> Result<Held, Response> {
    let manifest = built_in::listed()
        .iter()
        .find(|manifest| manifest.uuid == uuid);
    let config = state.settings.config();
    let installer = installers(&config)
        .into_iter()
        .find(|(language, _)| installer_uuid(language) == uuid);

    if manifest.is_none() && installer.is_none() {
        return held(state, uuid).await.map(|record| Held {
            uuid: record.uuid,
            fields: record.fields,
            recorded: true,
        });
    }

    let kept = match replicated::live_records::<Plugins>(&state.pool).await {
        Ok(records) => records.into_iter().find(|record| record.uuid == uuid),
        Err(error) => return Err(failed(error)),
    };
    let kept = kept.as_ref().map(|record| &record.fields);

    let fields = match (manifest, installer) {
        (Some(manifest), _) => {
            built_in_fields(manifest, kept.is_none_or(|kept| kept.enabled), kept)
        }
        (None, Some((language, descriptor))) => {
            let enabled = match kept {
                Some(kept) => kept.enabled,
                None => config
                    .languages()
                    .get(&language)
                    .is_none_or(crate::languages::Descriptor::enabled),
            };
            installer_fields(&language, &descriptor, enabled, kept)
        }
        (None, None) => unreachable!("one or the other, checked above"),
    };

    Ok(Held {
        uuid: uuid.to_owned(),
        fields,
        recorded: kept.is_some(),
    })
}

/// The languages the Language plugin `uuid` provides on this device, by the
/// name `config.yaml` keys each by, in the order its manifest lists them: a
/// built-in's, or the one an installer's own is drawn for where this
/// device's file has it. Nothing for any other plugin.
fn provided(state: &AppState, uuid: &str) -> Vec<String> {
    if let Some(manifest) = built_in::listed()
        .iter()
        .find(|manifest| manifest.uuid == uuid)
    {
        return built_in::languages(manifest)
            .map(|provided| provided.name.clone())
            .collect();
    }

    installers(&state.settings.config())
        .into_iter()
        .map(|(language, _)| language)
        .filter(|language| installer_uuid(language) == uuid)
        .collect()
}

/// The pane as this device reads `held`.
fn view(state: &AppState, held: &Held) -> PluginSettingsView {
    let loads = held.fields.enabled && on_this_device(state, &held.fields);

    PluginSettingsView {
        uuid: held.uuid.clone(),
        name: held.fields.name.clone(),
        trouble: loads.then(|| state.plugins.trouble(&held.uuid)).flatten(),
        fields: held
            .fields
            .declared
            .iter()
            .map(|declared| field(state, &held.uuid, &held.fields, declared))
            .collect(),
        languages: caches(state, &held.uuid),
        running: crate::ui::running(state),
    }
}

/// What a Language plugin's pane draws under its fields: each language it
/// provides as this device's stores hold it, ticked or not, in the order its
/// manifest lists them — see [`provided`]. Empty for any other plugin.
fn caches(state: &AppState, uuid: &str) -> Vec<PluginLanguageCache> {
    let provided = provided(state, uuid);
    if provided.is_empty() {
        return Vec::new();
    }

    let loaded = crate::languages::configured(&state.settings.config());
    let caches_compiles = state.sessions.caches_compiles();
    // The language whose store the one Compile Server keeps its compiles in:
    // the first naming it, as `Languages::wanting` sizes it.
    let sizer = loaded
        .iter()
        .find(|(_, descriptor)| descriptor.names(crate::languages::SCCACHE));

    provided
        .iter()
        .filter_map(|name| {
            let name = name.as_str();
            let descriptor = loaded.get(name)?;
            let label = |name: &str, descriptor: &crate::languages::Descriptor| {
                descriptor.label().unwrap_or(name).to_owned()
            };
            let compiling = descriptor.names(crate::languages::SCCACHE);

            Some(PluginLanguageCache {
                name: name.to_owned(),
                label: label(name, descriptor),
                size: descriptor.size(name).to_owned(),
                disk_use: crate::ui::disk_use_of(name, descriptor, &state.disk_use),
                stores: crate::ui::stores_of(descriptor),
                swept: state
                    .sweeps
                    .of(name)
                    .and_then(|at| at.format(&Rfc3339).ok()),
                compiling: compiling.then(|| crate::ui::compile_caching(caches_compiles)),
                through: sizer
                    .filter(|(sizer, _)| compiling && *sizer != name)
                    .map(|(sizer, descriptor)| label(sizer, descriptor)),
            })
        })
        .collect()
}

/// What a field of the plugin `uuid` shows while nothing is set: a Language
/// plugin's size is its first language's default — `30G` for Rust & C/C++,
/// whose sccache has always been started at it, and `10G` for the rest.
fn placeholder(state: &AppState, uuid: &str, declared: &Declared) -> Option<String> {
    if declared.kind != SettingKind::Size || declared.key != SIZE_SETTING {
        return None;
    }

    let first = provided(state, uuid).into_iter().next()?;
    Some(crate::languages::default_size(&first).to_owned())
}

fn field(state: &AppState, uuid: &str, fields: &Fields, declared: &Declared) -> PluginField {
    let line = || match fields.values.get(&declared.key) {
        Some(SettingValue::Line(line)) => line.clone(),
        _ => String::new(),
    };

    PluginField {
        key: declared.key.clone(),
        label: declared.label.clone(),
        description: declared.description.clone(),
        placeholder: placeholder(state, uuid, declared),
        value: match declared.kind {
            SettingKind::Text => PluginFieldValue::Text { value: line() },
            SettingKind::Size => PluginFieldValue::Size { value: line() },
            SettingKind::List => PluginFieldValue::List {
                values: match fields.values.get(&declared.key) {
                    Some(SettingValue::Lines(lines)) => lines.clone(),
                    _ => Vec::new(),
                },
            },
            SettingKind::Secret => PluginFieldValue::Secret {
                set: fields.secrets.contains_key(&declared.key),
            },
        },
    }
}

/// `GET /api/ui/plugins/{uuid}/settings` — the pane.
pub(crate) async fn pane(State(state): State<AppState>, Path(uuid): Path<String>) -> Response {
    match held_or_built_in(&state, &uuid).await {
        Ok(held) => Json(view(&state, &held)).into_response(),
        Err(refusal) => refusal,
    }
}

/// What a save asks of one field, once it is found to be one the plugin
/// declares and of its kind.
enum Change {
    Value(Option<SettingValue>),
    Secret(Option<String>),
}

/// What `edit` asks of the field `declared`, or why it is refused.
fn change(declared: Option<&Declared>, edit: &PluginFieldEdit) -> Result<Change, String> {
    let Some(declared) = declared else {
        return Err("This plugin has no such setting.".to_owned());
    };

    let line = |value: &str| {
        let value = value.trim();
        Change::Value((!value.is_empty()).then(|| SettingValue::Line(value.to_owned())))
    };

    match (declared.kind, &edit.value) {
        (SettingKind::Text, PluginFieldWrite::Text { value }) => Ok(line(value)),
        (SettingKind::Size, PluginFieldWrite::Size { value }) => {
            let trimmed = value.trim();
            if !trimmed.is_empty() && crate::languages::bytes(trimmed).is_err() {
                return Err(format!(
                    "{trimmed} isn't a size. Write a whole number with K, M, G or T after it, \
                     such as 30G."
                ));
            }
            Ok(line(trimmed))
        }
        (SettingKind::List, PluginFieldWrite::List { values }) => {
            let lines: Vec<String> = values
                .iter()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect();
            Ok(Change::Value(
                (!lines.is_empty()).then_some(SettingValue::Lines(lines)),
            ))
        }
        (SettingKind::Secret, PluginFieldWrite::Secret { value }) => {
            let value = value.trim();
            Ok(Change::Secret(
                (!value.is_empty()).then(|| value.to_owned()),
            ))
        }
        (SettingKind::Secret, PluginFieldWrite::SecretCleared) => Ok(Change::Secret(None)),
        _ => Err("That isn't a value of this setting's kind. Read the page again.".to_owned()),
    }
}

/// `POST /api/ui/plugins/{uuid}/settings` — a save of the pane: every field it
/// sent checked first, and written only where none is refused. A secret goes
/// to `secrets.yaml` and its digest onto the record, under the lock the
/// copying writes under, so the copying never reads one written and the other
/// not.
pub(crate) async fn save(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    Json(press): Json<PluginSettingsSave>,
) -> Response {
    let record = match held_or_built_in(&state, &uuid).await {
        Ok(held) => held,
        Err(refusal) => return refusal,
    };

    let mut changes = Vec::new();
    let mut refused = Vec::new();
    for edit in &press.fields {
        let declared = record
            .fields
            .declared
            .iter()
            .find(|declared| declared.key == edit.key);
        match change(declared, edit) {
            Ok(change) => changes.push((edit.key.clone(), change)),
            Err(why) => refused.push(PluginFieldRefused {
                key: edit.key.clone(),
                why,
            }),
        }
    }

    if !refused.is_empty() {
        return Json(PluginSettingsSaved {
            settings: view(&state, &record),
            refused,
        })
        .into_response();
    }

    let _writing = writing_held().await;

    // Read again under the lock, so a merge since the pane was read isn't
    // written over.
    let record = match held_or_built_in(&state, &uuid).await {
        Ok(held) => held,
        Err(refusal) => return refusal,
    };

    let mut values = record.fields.values.clone();
    let mut secrets: Vec<(String, Option<String>)> = Vec::new();
    for (key, change) in changes {
        match change {
            Change::Value(Some(value)) => {
                values.insert(key, value);
            }
            Change::Value(None) => {
                values.remove(&key);
            }
            Change::Secret(value) => secrets.push((key, value)),
        }
    }

    let digested = match secrets.is_empty() {
        true => record.fields.secrets.clone(),
        false => {
            let (settings, uuid) = (state.settings.clone(), uuid.clone());
            let written = tokio::task::spawn_blocking(move || {
                let mut held = settings
                    .secrets()
                    .plugin_secrets_of(&uuid)
                    .cloned()
                    .unwrap_or_default();
                for (key, value) in secrets {
                    match value {
                        Some(value) => held.insert(key, value),
                        None => held.remove(&key),
                    };
                }
                let digested = digests(&held);
                settings
                    .setting_plugin_secrets(&uuid, Some(held))
                    .map(|()| digested)
            })
            .await;
            match written {
                Ok(Ok(digested)) => digested,
                Ok(Err(error)) => return failed(error.into()),
                Err(error) => return failed(error.into()),
            }
        }
    };

    let fields = Fields {
        values,
        secrets: digested,
        ..record.fields.clone()
    };
    // A built-in nobody has written to is made by this save, as by its first
    // tick; any other plugin's record is only ever edited here.
    let written = match record.recorded {
        true => replicated::rewrite::<Plugins>(&state.pool, &state.device, &uuid, &fields).await,
        false => replicated::write::<Plugins>(&state.pool, &state.device, &uuid, &fields).await,
    };
    match written {
        Ok(Written::Written) => {
            // And a Language plugin's size is in the next session's
            // variables, held again here rather than on the Nudge's return.
            if let Err(error) =
                crate::replicating::plugins::held(&state.pool, &state.settings).await
            {
                return failed(error);
            }
            state.nudges.announce(Nudge::Plugins);
        }
        Ok(_) => {}
        Err(error) => return failed(error),
    }

    match held_or_built_in(&state, &uuid).await {
        Ok(held) => Json(PluginSettingsSaved {
            settings: view(&state, &held),
            refused: Vec::new(),
        })
        .into_response(),
        Err(refusal) => refusal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(kind: SettingKind) -> Declared {
        Declared {
            key: "it".to_owned(),
            label: "It".to_owned(),
            kind,
            description: None,
        }
    }

    fn edit(value: PluginFieldWrite) -> PluginFieldEdit {
        PluginFieldEdit {
            key: "it".to_owned(),
            value,
        }
    }

    #[test]
    fn a_size_that_is_not_a_size_is_refused_with_the_reason() {
        let size = declared(SettingKind::Size);
        let sized = |value: &str| {
            change(
                Some(&size),
                &edit(PluginFieldWrite::Size {
                    value: value.to_owned(),
                }),
            )
        };

        assert!(matches!(
            sized("30G"),
            Ok(Change::Value(Some(SettingValue::Line(line)))) if line == "30G"
        ));
        assert!(matches!(sized(" "), Ok(Change::Value(None))));
        assert_eq!(
            sized("1.5g").err().as_deref(),
            Some(
                "1.5g isn't a size. Write a whole number with K, M, G or T after it, such as 30G."
            ),
        );
    }

    #[test]
    fn a_value_of_another_kind_or_an_undeclared_key_is_refused() {
        let text = declared(SettingKind::Text);
        assert!(change(Some(&text), &edit(PluginFieldWrite::SecretCleared)).is_err());
        assert!(
            change(
                None,
                &edit(PluginFieldWrite::Text {
                    value: "x".to_owned()
                })
            )
            .is_err()
        );
    }
}
