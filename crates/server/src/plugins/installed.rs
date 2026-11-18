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
//! **A git plugin is every device's** (Set 1762, Q10): the record holds its
//! URL and the revision pinned at install, and each device clones it for
//! itself under `plugins/<uuid>/` — see [`super::git`] — loading it once the
//! clone is at that revision, and in trouble until it can be. **Update** on its
//! row fetches the default branch and re-pins the record, which the other
//! devices follow. A fetch that fails, or a manifest there that doesn't read,
//! leaves the old checkout loaded and the row saying why.
//!
//! **Dependencies are resolved after loading** (Set 1771, Q1): a plugin that
//! loaded is in trouble still where a dependency it declares is missing,
//! disabled, outside the version it asks for, or in trouble itself, against
//! the installed plugins that loaded here and the built-ins. Its trouble names
//! the dependency by the name its own manifest gives it, and the row offers
//! the press that puts it right where there is one — **Install** from the
//! repository the dependency names, or **Update** on the dependency's row.
//! Every load resolves again, so a press on the dependency clears it.
//!
//! **Install is the grant, and the whole of it** (ADR-0030, *Capabilities and
//! trust*). The consent card is read off the manifest with nothing written,
//! and the press carries back the capabilities the card showed: a manifest
//! that asks for anything more by the time it lands is refused rather than
//! installed over what the human saw. A manifest edited since to ask for more
//! is trouble at load until an **Update** shows the card again and it is
//! confirmed.

use std::collections::BTreeMap;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use verkstead_render::{
    PluginAt, PluginCapability, PluginConsent, PluginEnabled, PluginFix, PluginInstall, PluginNeed,
    PluginRow, PluginSource, PluginUpdate, PluginUpdated, PluginsView,
};
use verkstead_schema::{ApiError, Nudge};

use super::built_in::BUILT_INS;
use super::component;
use super::git::{self, Staged};
use super::manifest::{Access, Capability, Dependency, Files, Manifest};
use crate::AppState;
use crate::github::Credential;
use crate::platform::Platform;
use crate::replicating::plugins::{Fields, Plugins, Source};
use crate::store::replicated::{self, Record, Written};
use crate::ui::refused;

/// What loading the installed plugins on this device came to: each one's
/// trouble, by uuid, where it did not load. Shared by every clone of the
/// state.
#[derive(Debug, Clone, Default)]
pub(crate) struct Installed {
    troubles: Arc<Mutex<BTreeMap<String, Trouble>>>,

    /// Why the last **Update** of a git plugin didn't land, by uuid: the old
    /// checkout stays loaded, and the row says so until an Update does.
    pulls: Arc<Mutex<BTreeMap<String, String>>>,

    /// Held across one load, so two never interleave their writes of the
    /// troubles.
    loading: Arc<tokio::sync::Mutex<()>>,
}

impl Installed {
    /// Why `uuid` did not load here, where it did not — or why its last
    /// Update didn't land, where it loaded at the revision before.
    pub(crate) fn trouble(&self, uuid: &str) -> Option<String> {
        let loading = self
            .troubles
            .lock()
            .expect("nothing panics holding the troubles")
            .get(uuid)
            .map(|trouble| trouble.why.clone());

        loading.or_else(|| {
            self.pulls
                .lock()
                .expect("nothing panics holding the pulls")
                .get(uuid)
                .cloned()
        })
    }

    /// The presses that would put right the dependencies `uuid` is in
    /// trouble over.
    fn needs(&self, uuid: &str) -> Vec<PluginNeed> {
        self.troubles
            .lock()
            .expect("nothing panics holding the troubles")
            .get(uuid)
            .map(|trouble| trouble.needs.clone())
            .unwrap_or_default()
    }

    /// What `uuid`'s Update came to: why it didn't land, or nothing.
    fn pulled(&self, uuid: &str, why: Option<String>) {
        let mut pulls = self.pulls.lock().expect("nothing panics holding the pulls");
        match why {
            Some(why) => pulls.insert(uuid.to_owned(), why),
            None => pulls.remove(uuid),
        };
    }
}

/// Why a plugin counts as disabled here, and what would put its
/// dependencies right.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Trouble {
    why: String,
    needs: Vec<PluginNeed>,
}

/// Load every enabled plugin this device can reach, resolve what each depends
/// on, and hold what each came to — the pages told where that moved.
pub(crate) async fn load(state: &AppState) -> Result<()> {
    let _one = state.plugins.loading.lock().await;

    let home = crate::sandbox::Homes::of_the_server(&state.data_dir)
        .map(|homes| homes.servers().to_owned());
    let credential = credential(state);

    let records = replicated::live_records::<Plugins>(&state.pool).await?;
    let mut troubles = BTreeMap::new();
    let mut manifests = BTreeMap::new();
    for record in &records {
        if !reachable(state, &record.fields) {
            continue;
        }

        match loaded(state, record, home.as_deref(), &credential).await {
            Ok(manifest) => {
                manifests.insert(record.uuid.clone(), manifest);
            }
            Err(why) => {
                tracing::warn!(
                    plugin = record.fields.name,
                    uuid = record.uuid,
                    trouble = why,
                    "an installed plugin didn't load, and counts as disabled",
                );
                troubles.insert(
                    record.uuid.clone(),
                    Trouble {
                        why,
                        needs: Vec::new(),
                    },
                );
            }
        }
    }

    for (uuid, trouble) in resolved(state, &records, &mut manifests) {
        tracing::warn!(
            uuid,
            trouble = trouble.why,
            "an installed plugin's dependency isn't there, and it counts as disabled",
        );
        troubles.insert(uuid, trouble);
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
        Source::Git { .. } => true,
    }
}

/// What a plugin's clone and pull authenticate as: the one Author where there
/// is one, the host's login otherwise — a Repo's clone with no Author picked.
fn credential(state: &AppState) -> Credential {
    state.github.as_author(None).credential()
}

/// Every plugin in `loaded` whose dependencies aren't all there taken out of
/// it, answering each one's trouble. Taken out, it counts as disabled for the
/// plugins depending on it in turn, so this goes round until nothing moves.
fn resolved(
    state: &AppState,
    records: &[Record<Fields>],
    loaded: &mut BTreeMap<String, Manifest>,
) -> BTreeMap<String, Trouble> {
    let built_ins: BTreeMap<String, semver::Version> = BUILT_INS
        .iter()
        .filter_map(|built_in| {
            Manifest::parse(
                built_in.manifest,
                Files::Embedded(built_in.files),
                Platform::HERE,
            )
            .ok()
        })
        .map(|manifest| (manifest.uuid, manifest.version))
        .collect();

    let mut troubles = BTreeMap::new();
    loop {
        let short: Vec<(String, Trouble)> = loaded
            .iter()
            .filter_map(|(uuid, manifest)| {
                let unmet: Vec<(&Dependency, Unmet)> = manifest
                    .dependencies
                    .iter()
                    .filter_map(|dependency| {
                        unmet(state, dependency, records, loaded, &built_ins)
                            .map(|unmet| (dependency, unmet))
                    })
                    .collect();
                (!unmet.is_empty()).then(|| (uuid.clone(), trouble(&unmet)))
            })
            .collect();

        if short.is_empty() {
            return troubles;
        }
        for (uuid, trouble) in short {
            loaded.remove(&uuid);
            troubles.insert(uuid, trouble);
        }
    }
}

/// How a dependency isn't there.
enum Unmet {
    Missing,
    Disabled,
    /// Installed at a version outside the specifier, and whether its row has
    /// an **Update** — a built-in or a directory on another device has none.
    Outdated {
        version: String,
        updatable: bool,
    },
    /// A directory on another device, so nothing of it is loaded here.
    Elsewhere,
    /// Installed and ticked, but in trouble itself.
    Troubled,
}

/// How `dependency` isn't there, against the installed plugins and those of
/// them `loaded` here, and the built-ins; nothing where it is.
fn unmet(
    state: &AppState,
    dependency: &Dependency,
    records: &[Record<Fields>],
    loaded: &BTreeMap<String, Manifest>,
    built_ins: &BTreeMap<String, semver::Version>,
) -> Option<Unmet> {
    if let Some(version) = built_ins.get(&dependency.uuid) {
        return (!dependency.version.matches(version)).then(|| Unmet::Outdated {
            version: version.to_string(),
            updatable: false,
        });
    }

    let Some(record) = records.iter().find(|record| record.uuid == dependency.uuid) else {
        return Some(Unmet::Missing);
    };
    if !record.fields.enabled {
        return Some(Unmet::Disabled);
    }
    if !on_this_device(state, &record.fields) {
        return Some(Unmet::Elsewhere);
    }

    match loaded.get(&dependency.uuid) {
        Some(manifest) if dependency.version.matches(&manifest.version) => None,
        Some(manifest) => Some(Unmet::Outdated {
            version: manifest.version.to_string(),
            updatable: true,
        }),
        None => Some(Unmet::Troubled),
    }
}

/// The trouble a plugin is in over its `unmet` dependencies: a sentence for
/// each, and the press that puts each right where there is one.
fn trouble(unmet: &[(&Dependency, Unmet)]) -> Trouble {
    let mut why = Vec::new();
    let mut needs = Vec::new();
    for (dependency, unmet) in unmet {
        let name = &dependency.name;
        let mut need = |fix: PluginFix| {
            needs.push(PluginNeed {
                uuid: dependency.uuid.clone(),
                name: name.clone(),
                fix,
            })
        };

        why.push(match unmet {
            Unmet::Missing => {
                if let Some(repository) = &dependency.repository {
                    need(PluginFix::Install {
                        repository: repository.clone(),
                    });
                }
                format!("It needs {name}, which isn't installed.")
            }
            Unmet::Disabled => {
                format!("It needs {name}, which is disabled. Tick it to load this one.")
            }
            Unmet::Outdated { version, updatable } => {
                if *updatable {
                    need(PluginFix::Update);
                }
                format!(
                    "It needs {name} {}, and {version} is installed.",
                    dependency.version
                )
            }
            Unmet::Elsewhere => {
                format!("It needs {name}, which is a directory on another device.")
            }
            Unmet::Troubled => format!("It needs {name}, which didn't load."),
        });
    }

    Trouble {
        why: why.join(" "),
        needs,
    }
}

/// One plugin read and its wasm component started, answering its manifest, or
/// why not. A git plugin's clone is put at its pinned revision first, made
/// where this device has none.
async fn loaded(
    state: &AppState,
    record: &Record<Fields>,
    home: Option<&FsPath>,
    credential: &Credential,
) -> Result<Manifest, String> {
    let dir = match &record.fields.source {
        Source::Local { path, .. } => PathBuf::from(path),
        Source::Git { url, revision } => {
            let (data_dir, uuid, url, revision, credential) = (
                state.data_dir.clone(),
                record.uuid.clone(),
                url.clone(),
                revision.clone(),
                credential.clone(),
            );
            tokio::task::spawn_blocking(move || {
                git::pinned(&data_dir, &uuid, &url, &revision, &credential)
            })
            .await
            .map_err(|error| format!("Its clone couldn't be checked: {error}."))??
        }
    };

    let manifest = Manifest::read(&dir, Platform::HERE).map_err(|refusal| refusal.why)?;
    same_plugin(&manifest, &record.uuid)?;

    if !within(&manifest, &record.fields.granted) {
        return Err(
            "It asks for more than was granted at install. Press Update to review it.".to_owned(),
        );
    }

    component::load(Files::Dir(&dir), &manifest, home)
        .await
        .map(|_| manifest)
        .map_err(|trouble| trouble.why)
}

/// Refused where `manifest` has another uuid than the record `uuid` it was
/// read for.
fn same_plugin(manifest: &Manifest, uuid: &str) -> Result<(), String> {
    if manifest.uuid == uuid {
        return Ok(());
    }

    Err(format!(
        "Its `plugin.yaml` now has the uuid `{}`, so it's another plugin. Put back `{uuid}`.",
        manifest.uuid,
    ))
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
            let (source, elsewhere) = match &record.fields.source {
                Source::Local { path, device } => (
                    PluginSource::Local { path: path.clone() },
                    (!here).then(|| names.get(device).cloned().unwrap_or_else(|| device.clone())),
                ),
                Source::Git { url, revision } => (
                    PluginSource::Git {
                        url: url.clone(),
                        revision: revision.clone(),
                    },
                    None,
                ),
            };

            let loads = here && record.fields.enabled;
            PluginRow {
                trouble: loads.then(|| state.plugins.trouble(&record.uuid)).flatten(),
                needs: if loads {
                    state.plugins.needs(&record.uuid)
                } else {
                    Vec::new()
                },
                elsewhere,
                source,
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

/// Where a plugin being added was read: a directory, or a clone of a URL
/// staged under `plugins/`, taken back unless **Install** lands it.
enum Reading {
    Local(PathBuf),
    Git { url: String, staged: Staged },
}

impl Reading {
    fn dir(&self) -> &FsPath {
        match self {
            Reading::Local(dir) => dir,
            Reading::Git { staged, .. } => staged.path(),
        }
    }
}

/// The manifest at a path the human typed or browsed to, or in a clone of the
/// git URL they gave, refused where it isn't a plugin this device could
/// install. Refused before anything is listed or written, a clone that didn't
/// land included.
async fn readable(state: &AppState, asked: &str) -> Result<(Manifest, Reading), Response> {
    let refusal = |why: String| refused(StatusCode::BAD_REQUEST, ApiError::new(why));
    let asked = asked.trim();

    let reading = if git::is_url(asked) {
        let (data_dir, url, credential) =
            (state.data_dir.clone(), asked.to_owned(), credential(state));
        let staged =
            tokio::task::spawn_blocking(move || git::staged(&data_dir, &url, &credential)).await;

        match staged {
            Ok(Ok(staged)) => Reading::Git {
                url: asked.to_owned(),
                staged,
            },
            Ok(Err(why)) => {
                return Err(refusal(format!(
                    "That repository couldn't be cloned. Git said: {why}"
                )));
            }
            Err(error) => return Err(failed(error.into())),
        }
    } else {
        let dir = FsPath::new(asked);
        if !dir.is_absolute() {
            return Err(refusal(
                "That isn't an absolute path or a git URL. Name the plugin's directory from the \
                 root, or give its repository's URL."
                    .to_owned(),
            ));
        }
        if !dir.is_dir() {
            return Err(refusal("There's no directory there.".to_owned()));
        }
        Reading::Local(dir.to_owned())
    };

    let manifest =
        Manifest::read(reading.dir(), Platform::HERE).map_err(|refusal_| refusal(refusal_.why))?;

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

    Ok((manifest, reading))
}

/// `POST /api/ui/plugins/read` — the consent card for the plugin at a path or
/// a git URL, a clone of which is taken back once it is read.
async fn read(State(state): State<AppState>, Json(at): Json<PluginAt>) -> Response {
    match readable(&state, &at.source).await {
        Ok((manifest, _)) => Json(consent(&manifest)).into_response(),
        Err(refusal) => refusal,
    }
}

/// `POST /api/ui/plugins` — **Install**: the plugin at the path or URL written
/// down with the capabilities the card showed granted, and loaded. A git one is
/// cloned again, and that clone is what lands under `plugins/<uuid>/`, its
/// revision pinned on the record.
async fn install(State(state): State<AppState>, Json(press): Json<PluginInstall>) -> Response {
    let (manifest, reading) = match readable(&state, &press.source).await {
        Ok(read) => read,
        Err(refusal) => return refusal,
    };

    if !within(&manifest, &press.granted) {
        return refused(
            StatusCode::CONFLICT,
            ApiError::new("Its manifest asks for more than the card showed. Read it again."),
        );
    }

    let checkout = git::checkout(&state.data_dir, &manifest.uuid);
    let source = match reading {
        Reading::Local(dir) => Source::Local {
            path: dir.to_string_lossy().into_owned(),
            device: state.device.clone(),
        },
        Reading::Git { url, staged } => {
            let revision = staged.revision().to_owned();
            let to = checkout.clone();
            match tokio::task::spawn_blocking(move || staged.lands(&to)).await {
                Ok(Ok(())) => {}
                Ok(Err(why)) => {
                    return refused(StatusCode::INTERNAL_SERVER_ERROR, ApiError::new(why));
                }
                Err(error) => return failed(error.into()),
            }
            Source::Git { url, revision }
        }
    };
    let git = matches!(source, Source::Git { .. });
    let fields = fields(&manifest, source, true);

    let written =
        replicated::write::<Plugins>(&state.pool, &state.device, &manifest.uuid, &fields).await;
    if git && !matches!(written, Ok(Written::Written | Written::Unchanged)) {
        taken_back(checkout).await;
    }

    match written {
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

/// Take back a clone whose record was never written.
async fn taken_back(checkout: PathBuf) {
    let gone = tokio::task::spawn_blocking(move || std::fs::remove_dir_all(checkout)).await;
    if let Ok(Err(error)) = gone {
        tracing::error!(error = ?error, "a plugin's clone that never installed couldn't be taken back");
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

/// What a manifest read at an **Update** asks of the grant.
enum Granting {
    /// Nothing beyond what was granted: that grant stands, rather than
    /// narrowing to what this read of the manifest happens to name.
    Stands,
    /// More, and the press carried the grant the card showed for it.
    Grants,
    /// More, and the card is to be shown first.
    Shows(PluginConsent),
    /// More than even the card the press carried showed.
    Short,
}

fn granting(manifest: &Manifest, granted: &[PluginCapability], press: &PluginUpdate) -> Granting {
    match &press.granted {
        _ if within(manifest, granted) => Granting::Stands,
        None => Granting::Shows(consent(manifest)),
        Some(granted) if within(manifest, granted) => Granting::Grants,
        Some(_) => Granting::Short,
    }
}

/// The record's fields after an Update read `manifest` from `source`.
fn updated(
    record: &Record<Fields>,
    manifest: &Manifest,
    source: Source,
    granting: Granting,
) -> Fields {
    let mut fields = fields(manifest, source, record.fields.enabled);
    if let Granting::Stands = granting {
        fields.granted = record.fields.granted.clone();
    }
    fields
}

fn short_of_the_card() -> Response {
    refused(
        StatusCode::CONFLICT,
        ApiError::new("Its manifest asks for more than the card showed. Press Update again."),
    )
}

/// `POST /api/ui/plugins/{uuid}/update` — **Update**. A local plugin's
/// manifest is read again; a git plugin's default branch is fetched and read
/// at its new revision. Where the manifest now asks for more than was granted
/// the consent card comes back first, and the press made again with the grant
/// it showed lands it.
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

    let answered = match &record.fields.source {
        Source::Local { path, .. } => update_local(&state, &record, path, &press).await,
        Source::Git { url, revision } => {
            // Held so that no load puts the clone back at the pinned revision
            // while the new one is being read.
            let _one = state.plugins.loading.lock().await;
            update_git(&state, &record, url, revision, &press).await
        }
    };
    if let Some(refusal) = answered {
        return refusal;
    }

    match moved(&state).await {
        Ok(plugins) => Json(PluginUpdated::Updated { plugins }).into_response(),
        Err(refusal) => refusal,
    }
}

/// A local plugin's manifest read again, and the record written over what it
/// says — or the answer that comes first. A manifest that doesn't read leaves
/// the record as it was, and loading it again is what puts trouble on the row.
async fn update_local(
    state: &AppState,
    record: &Record<Fields>,
    path: &str,
    press: &PluginUpdate,
) -> Option<Response> {
    let manifest = Manifest::read(FsPath::new(path), Platform::HERE).ok()?;
    if manifest.uuid != record.uuid {
        return None;
    }

    let granting = match granting(&manifest, &record.fields.granted, press) {
        Granting::Shows(consent) => {
            return Some(Json(PluginUpdated::Consent { consent }).into_response());
        }
        Granting::Short => return Some(short_of_the_card()),
        granting => granting,
    };

    let fields = updated(record, &manifest, record.fields.source.clone(), granting);
    if let Err(error) =
        replicated::rewrite::<Plugins>(&state.pool, &state.device, &record.uuid, &fields).await
    {
        return Some(failed(error));
    }

    None
}

/// A git plugin's default branch fetched into this device's clone and read at
/// its new revision, which the record is re-pinned to — or the answer that
/// comes first. Anything short of landing puts the clone back at `pinned`, and
/// a fetch that failed or a manifest that doesn't read is the row's trouble
/// with the old checkout still loaded.
async fn update_git(
    state: &AppState,
    record: &Record<Fields>,
    url: &str,
    pinned: &str,
    press: &PluginUpdate,
) -> Option<Response> {
    let (data_dir, uuid, url_, pinned_, credential) = (
        state.data_dir.clone(),
        record.uuid.clone(),
        url.to_owned(),
        pinned.to_owned(),
        credential(state),
    );
    let pulled = tokio::task::spawn_blocking(move || {
        let dir = git::pinned(&data_dir, &uuid, &url_, &pinned_, &credential)?;
        let revision = git::fetched(&dir, &credential).map_err(|why| {
            format!(
                "The pull failed, so it's still at {}. Git said: {why}",
                git::short(&pinned_)
            )
        })?;
        git::check_out(&dir, &revision)?;

        let manifest = Manifest::read(&dir, Platform::HERE)
            .map_err(|refusal| refusal.why)
            .and_then(|manifest| same_plugin(&manifest, &uuid).map(|()| manifest));
        match manifest {
            Ok(manifest) => Ok((dir, revision, manifest)),
            Err(why) => {
                git::check_out(&dir, &pinned_)?;
                Err(format!(
                    "The pull to {} wasn't taken, so it's still at {}. {why}",
                    git::short(&revision),
                    git::short(&pinned_),
                ))
            }
        }
    })
    .await;

    let (dir, revision, manifest) = match pulled {
        Ok(Ok(pulled)) => pulled,
        Ok(Err(why)) => {
            state.plugins.pulled(&record.uuid, Some(why));
            return None;
        }
        Err(error) => return Some(failed(error.into())),
    };

    let put_back = |answer: Response| async {
        let pinned = pinned.to_owned();
        if let Ok(Err(why)) =
            tokio::task::spawn_blocking(move || git::check_out(&dir, &pinned)).await
        {
            tracing::error!(%why, "a plugin's clone couldn't be put back at its pinned revision");
        }
        Some(answer)
    };

    let granting = match granting(&manifest, &record.fields.granted, press) {
        Granting::Shows(consent) => {
            return put_back(Json(PluginUpdated::Consent { consent }).into_response()).await;
        }
        Granting::Short => return put_back(short_of_the_card()).await,
        granting => granting,
    };

    let source = Source::Git {
        url: url.to_owned(),
        revision,
    };
    let fields = updated(record, &manifest, source, granting);
    if let Err(error) =
        replicated::rewrite::<Plugins>(&state.pool, &state.device, &record.uuid, &fields).await
    {
        return put_back(failed(error)).await;
    }

    state.plugins.pulled(&record.uuid, None);
    None
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
