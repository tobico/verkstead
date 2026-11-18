//! The Plugins a human installed: what loading them on this device came to,
//! and the **Plugins** section's reading and presses (ADR-0030).
//!
//! **The installed list is a Replicated record** — see
//! [`crate::replicating::plugins`] — and **loading is each device's own**:
//! every enabled plugin this device can reach is read through the manifest
//! reader and its wasm component started through the loader, at the start and
//! whenever the record moves, a merge from a member included. What that comes
//! to is held here in memory, a plugin's **trouble** where it did not load,
//! which counts it as disabled and stops nothing else (Set 1762, Q15).
//!
//! **A local plugin is read in place** (Set 1772, Q2): the record holds its
//! path and the device it is on, and nothing is copied. Every other device
//! lists it greyed, naming that device, and loads nothing of it. **Update** on
//! its row reads the manifest again, which is the development loop: edit, then
//! press.
//!
//! **Install is the grant, and the whole of it** (ADR-0030, *Capabilities and
//! trust*). The consent card is read off the manifest with nothing written,
//! and the press carries back the capabilities the card showed: a manifest
//! that asks for anything more by the time it lands is refused rather than
//! installed over what the human saw. A manifest edited since to ask for more
//! is trouble at load until an **Update** shows the card again and it is
//! confirmed.

use std::collections::BTreeMap;
use std::path::Path as FsPath;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use verkstead_render::{
    PluginAt, PluginCapability, PluginConsent, PluginEnabled, PluginInstall, PluginRow,
    PluginSource, PluginUpdate, PluginUpdated, PluginsView,
};
use verkstead_schema::{ApiError, Nudge};

use super::built_in::BUILT_INS;
use super::component;
use super::manifest::{Access, Capability, Files, Manifest};
use crate::AppState;
use crate::platform::Platform;
use crate::replicating::plugins::{Fields, Plugins, Source};
use crate::store::replicated::{self, Record, Written};
use crate::ui::refused;

/// What loading the installed plugins on this device came to: each one's
/// trouble, by uuid, where it did not load. Shared by every clone of the
/// state.
#[derive(Debug, Clone, Default)]
pub(crate) struct Installed {
    troubles: Arc<Mutex<BTreeMap<String, String>>>,

    /// Held across one load, so two never interleave their writes of the
    /// troubles.
    loading: Arc<tokio::sync::Mutex<()>>,
}

impl Installed {
    /// Why `uuid` did not load here, where it did not.
    pub(crate) fn trouble(&self, uuid: &str) -> Option<String> {
        self.troubles
            .lock()
            .expect("nothing panics holding the troubles")
            .get(uuid)
            .cloned()
    }
}

/// Load every enabled plugin this device can reach, and hold what each came
/// to — the pages told where that moved.
pub(crate) async fn load(state: &AppState) -> Result<()> {
    let _one = state.plugins.loading.lock().await;

    let home = crate::sandbox::Homes::of_the_server(&state.data_dir)
        .map(|homes| homes.servers().to_owned());

    let mut troubles = BTreeMap::new();
    for record in replicated::live_records::<Plugins>(&state.pool).await? {
        if !reachable(state, &record.fields) {
            continue;
        }

        if let Err(trouble) = loaded(&record, home.as_deref()).await {
            tracing::warn!(
                plugin = record.fields.name,
                uuid = record.uuid,
                %trouble,
                "an installed plugin didn't load, and counts as disabled",
            );
            troubles.insert(record.uuid.clone(), trouble);
        }
    }

    let moved = {
        let mut held = state
            .plugins
            .troubles
            .lock()
            .expect("nothing panics holding the troubles");
        let moved = *held != troubles;
        *held = troubles;
        moved
    };

    if moved {
        state.nudges.announce_here(Nudge::Plugins);
    }

    Ok(())
}

/// Load at the start, beside everything else a start sets going: a plugin's
/// wasm component can take a moment to compile, and nothing waits on it.
pub(crate) fn loading(state: &AppState) {
    let state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = load(&state).await {
            tracing::error!(error = ?error, "the installed plugins couldn't be loaded");
        }
    });
}

/// Whether a plugin is this device's to load: ticked, and on this device.
fn reachable(state: &AppState, fields: &Fields) -> bool {
    fields.enabled && on_this_device(state, fields)
}

fn on_this_device(state: &AppState, fields: &Fields) -> bool {
    match &fields.source {
        Source::Local { device, .. } => *device == state.device,
    }
}

/// One plugin read and its wasm component started, or why not.
async fn loaded(record: &Record<Fields>, home: Option<&FsPath>) -> Result<(), String> {
    let Source::Local { path, .. } = &record.fields.source;
    let dir = FsPath::new(path);

    let manifest = Manifest::read(dir, Platform::HERE).map_err(|refusal| refusal.why)?;

    if manifest.uuid != record.uuid {
        return Err(format!(
            "Its `plugin.yaml` now has the uuid `{}`, so it's another plugin. Put back `{}`.",
            manifest.uuid, record.uuid,
        ));
    }

    if !within(&manifest, &record.fields.granted) {
        return Err(
            "It asks for more than was granted at install. Press Update to review it.".to_owned(),
        );
    }

    component::load(Files::Dir(dir), &manifest, home)
        .await
        .map(drop)
        .map_err(|trouble| trouble.why)
}

/// Whether every capability `manifest` asks for is among `granted`.
fn within(manifest: &Manifest, granted: &[PluginCapability]) -> bool {
    capabilities(manifest)
        .iter()
        .all(|asked| granted.contains(asked))
}

/// What a manifest asks for, as the card shows it.
fn capabilities(manifest: &Manifest) -> Vec<PluginCapability> {
    manifest
        .capabilities
        .iter()
        .map(|capability| match capability {
            Capability::Filesystem { path, access } => PluginCapability::Filesystem {
                path: path.clone(),
                writable: *access == Access::ReadWrite,
            },
            Capability::Outbound { host } => PluginCapability::Outbound { host: host.clone() },
            Capability::Environment { name } => {
                PluginCapability::Environment { name: name.clone() }
            }
        })
        .collect()
}

/// The Category of each thing a manifest provides, once each, in the order
/// first written.
fn provides(manifest: &Manifest) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for provision in &manifest.provides {
        let word = provision.category.word();
        if !words.iter().any(|seen| seen == word) {
            words.push(word.to_owned());
        }
    }
    words
}

fn consent(manifest: &Manifest) -> PluginConsent {
    PluginConsent {
        uuid: manifest.uuid.clone(),
        name: manifest.name.clone(),
        version: manifest.version.to_string(),
        author: manifest.author.name.clone(),
        email: manifest.author.email.clone(),
        description: manifest.description.clone(),
        provides: provides(manifest),
        capabilities: capabilities(manifest),
    }
}

/// The record's fields for `manifest`, read at a press.
fn fields(manifest: &Manifest, source: Source, enabled: bool) -> Fields {
    Fields {
        source,
        enabled,
        granted: capabilities(manifest),
        name: manifest.name.clone(),
        version: manifest.version.to_string(),
        author: manifest.author.name.clone(),
        provides: provides(manifest),
    }
}

/// The Plugins page as this device reads it.
pub(crate) async fn view(state: &AppState) -> Result<PluginsView> {
    let names: BTreeMap<String, String> = match state.devices.as_ref() {
        Some(devices) => devices
            .membership()
            .rows()
            .await?
            .into_iter()
            .map(|member| (member.device, member.name))
            .collect(),
        None => BTreeMap::new(),
    };

    let installed = replicated::live_records::<Plugins>(&state.pool)
        .await?
        .into_iter()
        .map(|record| {
            let here = on_this_device(state, &record.fields);
            let Source::Local { path, device } = &record.fields.source;

            PluginRow {
                trouble: if here && record.fields.enabled {
                    state.plugins.trouble(&record.uuid)
                } else {
                    None
                },
                elsewhere: (!here)
                    .then(|| names.get(device).cloned().unwrap_or_else(|| device.clone())),
                source: PluginSource::Local { path: path.clone() },
                uuid: record.uuid,
                name: record.fields.name,
                version: record.fields.version,
                author: record.fields.author,
                provides: record.fields.provides,
                enabled: record.fields.enabled,
            }
        })
        .collect();

    Ok(PluginsView {
        installed,
        built_in: Vec::new(),
    })
}

/// The routes the section reads and presses — mounted beside the rest of the
/// viewer's in [`crate::ui`].
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/ui/plugins", get(plugins).post(install))
        // The consent card, read off a path with nothing written: what
        // **Install** then grants is what this showed.
        .route("/api/ui/plugins/read", post(read))
        .route("/api/ui/plugins/{uuid}/update", post(update))
        .route("/api/ui/plugins/{uuid}/enabled", post(enable))
}

/// `GET /api/ui/plugins` — the Plugins page.
async fn plugins(State(state): State<AppState>) -> Response {
    answered(&state).await
}

async fn answered(state: &AppState) -> Response {
    match view(state).await {
        Ok(view) => Json(view).into_response(),
        Err(error) => failed(error),
    }
}

fn failed(error: anyhow::Error) -> Response {
    tracing::error!(error = ?error, "the installed plugins couldn't be read");
    refused(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiError::new("The installed plugins couldn't be read."),
    )
}

/// The manifest at a path the human typed or browsed to, refused where it
/// isn't a plugin this device could install.
async fn readable(state: &AppState, path: &str) -> Result<Manifest, Response> {
    let refusal = |why: String| refused(StatusCode::BAD_REQUEST, ApiError::new(why));

    let dir = FsPath::new(path.trim());
    if !dir.is_absolute() {
        return Err(refusal(
            "That isn't an absolute path. Name the plugin's directory from the root.".to_owned(),
        ));
    }
    if !dir.is_dir() {
        return Err(refusal("There's no directory there.".to_owned()));
    }

    let manifest = Manifest::read(dir, Platform::HERE).map_err(|refusal_| refusal(refusal_.why))?;

    let built_in = BUILT_INS.iter().any(|built_in| {
        Manifest::parse(
            built_in.manifest,
            Files::Embedded(built_in.files),
            Platform::HERE,
        )
        .is_ok_and(|shipped| shipped.uuid == manifest.uuid)
    });
    if built_in {
        return Err(refused(
            StatusCode::CONFLICT,
            ApiError::new(format!(
                "Its uuid `{}` is a built-in plugin's. Give it a uuid of its own.",
                manifest.uuid
            )),
        ));
    }

    let installed = match replicated::live_records::<Plugins>(&state.pool).await {
        Ok(records) => records,
        Err(error) => return Err(failed(error)),
    };
    if let Some(record) = installed.iter().find(|record| record.uuid == manifest.uuid) {
        return Err(refused(
            StatusCode::CONFLICT,
            ApiError::new(format!(
                "{} is already installed, with the same uuid.",
                record.fields.name
            )),
        ));
    }

    Ok(manifest)
}

/// `POST /api/ui/plugins/read` — the consent card for the plugin at a path.
async fn read(State(state): State<AppState>, Json(at): Json<PluginAt>) -> Response {
    match readable(&state, &at.path).await {
        Ok(manifest) => Json(consent(&manifest)).into_response(),
        Err(refusal) => refusal,
    }
}

/// `POST /api/ui/plugins` — **Install**: the plugin at the path written down
/// with the capabilities the card showed granted, and loaded.
async fn install(State(state): State<AppState>, Json(press): Json<PluginInstall>) -> Response {
    let manifest = match readable(&state, &press.path).await {
        Ok(manifest) => manifest,
        Err(refusal) => return refusal,
    };

    if !within(&manifest, &press.granted) {
        return refused(
            StatusCode::CONFLICT,
            ApiError::new("Its manifest asks for more than the card showed. Read it again."),
        );
    }

    let source = Source::Local {
        path: press.path.trim().to_owned(),
        device: state.device.clone(),
    };
    let fields = fields(&manifest, source, true);

    match replicated::write::<Plugins>(&state.pool, &state.device, &manifest.uuid, &fields).await {
        Ok(Written::Removed) => {
            return refused(
                StatusCode::CONFLICT,
                ApiError::new(format!(
                    "A plugin with the uuid `{}` was removed, and its uuid can't be installed \
                     again.",
                    manifest.uuid
                )),
            );
        }
        Ok(_) => {}
        Err(error) => return failed(error),
    }

    match moved(&state).await {
        Ok(plugins) => Json(plugins).into_response(),
        Err(refusal) => refusal,
    }
}

/// The record moved here: the cluster told, this device's copies loaded again,
/// and the page read again for the answer.
async fn moved(state: &AppState) -> Result<PluginsView, Response> {
    state.nudges.announce(Nudge::Plugins);

    load(state).await.map_err(failed)?;
    view(state).await.map_err(failed)
}

/// The record under `uuid`, or the refusal naming it as not installed.
async fn held(state: &AppState, uuid: &str) -> Result<Record<Fields>, Response> {
    match replicated::live_records::<Plugins>(&state.pool).await {
        Ok(records) => records
            .into_iter()
            .find(|record| record.uuid == uuid)
            .ok_or_else(|| {
                refused(
                    StatusCode::NOT_FOUND,
                    ApiError::new("That plugin isn't installed."),
                )
            }),
        Err(error) => Err(failed(error)),
    }
}

/// `POST /api/ui/plugins/{uuid}/update` — **Update**: a local plugin's
/// manifest read again. Where it now asks for more than was granted the
/// consent card comes back first, and the press made again with the grant it
/// showed lands it. A manifest that doesn't read leaves the record as it was
/// and the plugin in trouble.
async fn update(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    Json(press): Json<PluginUpdate>,
) -> Response {
    let record = match held(&state, &uuid).await {
        Ok(record) => record,
        Err(refusal) => return refusal,
    };

    if !on_this_device(&state, &record.fields) {
        return refused(
            StatusCode::CONFLICT,
            ApiError::new("This plugin is a directory on another device. Update it there."),
        );
    }

    let Source::Local { path, .. } = &record.fields.source;

    if let Ok(manifest) = Manifest::read(FsPath::new(path), Platform::HERE)
        && manifest.uuid == record.uuid
    {
        let granted = match &press.granted {
            _ if within(&manifest, &record.fields.granted) => None,
            None => {
                return Json(PluginUpdated::Consent {
                    consent: consent(&manifest),
                })
                .into_response();
            }
            Some(granted) if within(&manifest, granted) => Some(granted),
            Some(_) => {
                return refused(
                    StatusCode::CONFLICT,
                    ApiError::new(
                        "Its manifest asks for more than the card showed. Press Update again.",
                    ),
                );
            }
        };

        let mut fields = fields(
            &manifest,
            record.fields.source.clone(),
            record.fields.enabled,
        );
        if granted.is_none() {
            // Nothing new asked for: what was granted stands, rather than
            // narrowing to what this read of the manifest happens to name.
            fields.granted = record.fields.granted.clone();
        }

        if let Err(error) =
            replicated::rewrite::<Plugins>(&state.pool, &state.device, &uuid, &fields).await
        {
            return failed(error);
        }
    }

    match moved(&state).await {
        Ok(plugins) => Json(PluginUpdated::Updated { plugins }).into_response(),
        Err(refusal) => refusal,
    }
}

/// `POST /api/ui/plugins/{uuid}/enabled` — the checkbox, which is the
/// cluster's.
async fn enable(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    Json(press): Json<PluginEnabled>,
) -> Response {
    let record = match held(&state, &uuid).await {
        Ok(record) => record,
        Err(refusal) => return refusal,
    };

    let fields = Fields {
        enabled: press.enabled,
        ..record.fields
    };

    if let Err(error) =
        replicated::rewrite::<Plugins>(&state.pool, &state.device, &uuid, &fields).await
    {
        return failed(error);
    }

    match moved(&state).await {
        Ok(plugins) => Json(plugins).into_response(),
        Err(refusal) => refusal,
    }
}
