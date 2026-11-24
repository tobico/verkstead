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

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use verkstead_render::{
    PluginField, PluginFieldEdit, PluginFieldRefused, PluginFieldValue, PluginFieldWrite,
    PluginSettingsSave, PluginSettingsSaved, PluginSettingsView,
};
use verkstead_schema::Nudge;

use super::installed::{failed, held, on_this_device};
use super::manifest::SettingKind;
use crate::AppState;
use crate::replicating::authors::writing_held;
use crate::replicating::plugins::{Declared, Fields, Plugins, SettingValue, digests};
use crate::store::replicated::{self, Record, Written};

/// The pane as this device reads `record`.
fn view(state: &AppState, record: &Record<Fields>) -> PluginSettingsView {
    let loads = record.fields.enabled && on_this_device(state, &record.fields);

    PluginSettingsView {
        uuid: record.uuid.clone(),
        name: record.fields.name.clone(),
        trouble: loads.then(|| state.plugins.trouble(&record.uuid)).flatten(),
        fields: record
            .fields
            .declared
            .iter()
            .map(|declared| field(&record.fields, declared))
            .collect(),
    }
}

fn field(fields: &Fields, declared: &Declared) -> PluginField {
    let line = || match fields.values.get(&declared.key) {
        Some(SettingValue::Line(line)) => line.clone(),
        _ => String::new(),
    };

    PluginField {
        key: declared.key.clone(),
        label: declared.label.clone(),
        description: declared.description.clone(),
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
    match held(&state, &uuid).await {
        Ok(record) => Json(view(&state, &record)).into_response(),
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
    let record = match held(&state, &uuid).await {
        Ok(record) => record,
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
    let record = match held(&state, &uuid).await {
        Ok(record) => record,
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
    match replicated::rewrite::<Plugins>(&state.pool, &state.device, &uuid, &fields).await {
        Ok(Written::Written) => state.nudges.announce(Nudge::Plugins),
        Ok(_) => {}
        Err(error) => return failed(error),
    }

    match held(&state, &uuid).await {
        Ok(record) => Json(PluginSettingsSaved {
            settings: view(&state, &record),
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
