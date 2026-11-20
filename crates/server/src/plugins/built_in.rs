//! The Plugins Verkstead ships, inside the binary: each directory under the
//! repository's `plugins/`, its manifest, its wasm component and every other
//! file in it embedded by `build.rs` (ADR-0030, *The shape of a plugin*).
//!
//! **A built-in goes through the loader an installed plugin does** — the same
//! manifest reader, the same [`component::load`] — reading its files out of
//! the binary rather than a directory. That is what the built-ins being the
//! same artefact a user writes comes to.
//!
//! **And one that fails to load is trouble, never a server that stops** (Set
//! 1762, Q15): a manifest refused, a wasm component that traps while it
//! starts. The start says so in the log and carries on without it.

use std::path::Path;
use std::sync::LazyLock;

use super::component::{self, Trouble};
use super::manifest::{Files, Manifest, Refusal};
use crate::platform::Platform;

/// One built-in as `build.rs` embedded it.
#[derive(Debug, Clone, Copy)]
pub struct BuiltIn {
    /// Its directory under `plugins/`, which names it in the log until its
    /// manifest has been read.
    pub directory: &'static str,
    /// Its `plugin.yaml`.
    pub manifest: &'static str,
    /// Every file in its directory but the manifest, by its path there — its
    /// wasm component where it has one, and a Skill Pack's skills.
    pub files: &'static [(&'static str, &'static [u8])],
    /// Its directory in the checkout, which a debug build reads instead of
    /// `files`, exactly as the viewer is read: editing a skill is then visible
    /// to a running `cargo run -p verkstead-cli -- serve` without a recompile.
    /// None in a release build, and for a built-in whose component is built
    /// rather than written, since that is not in its directory.
    pub checkout: Option<&'static str>,
}

impl BuiltIn {
    /// Where the files its manifest names are read from.
    pub fn files(&self) -> Files<'static> {
        match self.checkout {
            Some(dir) => Files::Dir(Path::new(dir)),
            None => Files::Embedded(self.files),
        }
    }

    /// Its manifest as `platform` reads it.
    pub fn manifest(&self, platform: Platform) -> Result<Manifest, Refusal> {
        Manifest::parse(self.manifest, self.files(), platform)
    }

    /// Whether its manifest's `uuid` is `uuid`, read off that one key.
    ///
    /// What finding a built-in by a dependency's uuid asks before reading the
    /// whole manifest: reading one checks its own dependencies' skills against
    /// the built-ins, so a built-in that depends on another would otherwise
    /// read itself while it was being read.
    pub fn is(&self, uuid: &str) -> bool {
        serde_saphyr::from_str::<serde_json::Value>(self.manifest)
            .ok()
            .and_then(|tree| tree.get("uuid")?.as_str().map(|own| own == uuid))
            .unwrap_or(false)
    }
}

/// Every built-in this binary carries.
pub const BUILT_INS: &[BuiltIn] = include!(concat!(env!("OUT_DIR"), "/built_ins.rs"));

/// The fixture's uuid: a built-in that provides nothing, whose wasm component
/// exports nothing, and which is not listed anywhere. It is the proof that
/// the path above works on every platform a release is built for.
pub const FIXTURE: &str = "613e2a43-80ee-4307-8623-28969e05330f";

/// Codex's uuid, which is its built-in harness's in [`crate::store::BUILT_IN`]
/// so that a Profile stored as `codex` reads as its plugin.
pub const CODEX: &str = "49ea40c6-298c-412a-95b5-c89b9f539616";

/// Whether the Plugins page leaves the built-in `uuid` out: the fixture,
/// which is listed nowhere.
pub fn unlisted(uuid: &str) -> bool {
    uuid == FIXTURE
}

/// The uuid of the built-in that is the Skill Pack `verkstead`: the skills
/// every session Verkstead runs is told to read, and the one plugin that pack
/// name is reserved for. The first row of the Plugins page's Built-in group.
pub const VERKSTEAD: &str = "15447a8d-f28d-4573-8b8b-a7226d477755";

/// The built-in that is Codex.
#[cfg(test)]
pub(crate) fn codex() -> BuiltIn {
    *BUILT_INS
        .iter()
        .find(|built_in| built_in.directory == "codex")
        .expect("build.rs embeds every directory under plugins/, and plugins/codex is one")
}

/// The harness each built-in providing an agent declares, stood up out of its
/// manifest as `platform` reads it and its component — see
/// [`crate::harness::of`], which is what asks. One that doesn't load is left
/// out, said in the log.
pub(crate) async fn agents(platform: Platform) -> Vec<crate::harness::plugged::Plugged> {
    let mut agents = Vec::new();
    for built_in in BUILT_INS {
        match agent(*built_in, platform).await {
            Ok(Some(plugged)) => agents.push(plugged),
            Ok(None) => {}
            Err(trouble) => tracing::warn!(
                plugin = built_in.directory,
                %trouble,
                "a built-in plugin's harness didn't load, and is left out",
            ),
        }
    }
    agents
}

/// The harness `built_in` declares, or none where it provides no agent.
async fn agent(
    built_in: BuiltIn,
    platform: Platform,
) -> Result<Option<crate::harness::plugged::Plugged>, Trouble> {
    let manifest = built_in
        .manifest(platform)
        .map_err(|refusal| Trouble { why: refusal.why })?;
    let Some(agent) = manifest
        .provides
        .iter()
        .find_map(|provision| provision.agent())
    else {
        return Ok(None);
    };

    let mark = match agent.mark.as_ref().or(manifest.mark.as_ref()) {
        Some(super::manifest::Mark::Inline(svg)) => svg.clone(),
        Some(super::manifest::Mark::File(path)) => built_in
            .files()
            .read(path)
            .and_then(|svg| String::from_utf8(svg).ok())
            .ok_or_else(|| Trouble {
                why: format!("Its mark `{}` couldn't be read.", path.display()),
            })?,
        None => String::new(),
    };
    let Some(loaded) = component::load(built_in.files(), &manifest, None).await? else {
        return Err(Trouble {
            why: "It provides an agent and has no component.".to_owned(),
        });
    };

    Ok(Some(crate::harness::plugged::Plugged::new(
        crate::store::AgentType::read(&manifest.uuid),
        agent.clone(),
        mark,
        loaded,
    )))
}

/// The built-in the Skill Pack `verkstead` is.
pub fn verkstead() -> BuiltIn {
    *BUILT_INS
        .iter()
        .find(|built_in| built_in.directory == "verkstead")
        .expect("build.rs embeds every directory under plugins/, and plugins/verkstead is one")
}

/// The built-ins the Plugins page lists, their manifests as this platform
/// reads them, in the order `build.rs` embedded them: every one but those
/// [`unlisted`].
///
/// A manifest that will not read is left out rather than listed, which only a
/// build that should never have shipped can make so — the start says why, and
/// the suite reads every one. Read once, the binary being the same all along.
pub fn listed() -> &'static [Manifest] {
    static LISTED: LazyLock<Vec<Manifest>> = LazyLock::new(|| {
        BUILT_INS
            .iter()
            .filter_map(|built_in| built_in.manifest(Platform::HERE).ok())
            .filter(|manifest| !unlisted(&manifest.uuid))
            .collect()
    });

    &LISTED
}

/// What the built-in plugin `uuid`'s harness declares to install it, as
/// `platform` reads its manifest — None where no built-in is `uuid`, or it
/// provides no agent.
///
/// **Read for `platform` rather than off its harness**, which this process
/// stood up as [`Platform::HERE`] reads it: the wizard's install plan is a
/// function of the machine it is planned for, and a suite plans every
/// platform's from one. Each platform's manifests are read once.
pub(crate) fn agent_install(
    uuid: &str,
    platform: Platform,
) -> Option<&'static super::manifest::agent::Install> {
    static READ: LazyLock<[(Platform, Vec<Manifest>); 3]> = LazyLock::new(|| {
        [Platform::Linux, Platform::MacOs, Platform::Windows].map(|platform| {
            let read = BUILT_INS
                .iter()
                .filter_map(|built_in| built_in.manifest(platform).ok())
                .collect();
            (platform, read)
        })
    });

    READ.iter()
        .find(|(read_for, _)| *read_for == platform)?
        .1
        .iter()
        .find(|manifest| manifest.uuid == uuid)?
        .provides
        .iter()
        .find_map(|provision| provision.agent())
        .map(|agent| &agent.install)
}

/// The listed built-in that provides the language `name`, where one does:
/// whose tick on the Plugins page is that language's switch.
pub fn providing(name: &str) -> Option<&'static Manifest> {
    listed()
        .iter()
        .find(|manifest| languages(manifest).any(|provided| provided.name == name))
}

/// Whether `manifest`'s built-in has a tick: a Language plugin's is whether
/// sessions get its languages, an Agent plugin's whether its harness is
/// offered, and every other built-in is always on.
pub fn switched(manifest: &Manifest) -> bool {
    languages(manifest).next().is_some()
        || manifest
            .provides
            .iter()
            .any(|provision| provision.agent().is_some())
}

/// The languages `manifest` provides, by the name `config.yaml` keys each by,
/// in the order it lists them.
pub fn languages(manifest: &Manifest) -> impl Iterator<Item = &crate::languages::Provided> {
    manifest
        .provides
        .iter()
        .filter_map(|provision| provision.language.as_ref())
}

/// The uuid of the built-in whose blocks are the Activities Verkstead ships,
/// one component each: *Follow-up rounds* first, then the *Backlog runner*,
/// the *Stage runner*, *One session* and the *Interview* (Set 1803, Q3).
pub const ACTIVITIES: &str = "d7a5e719-8491-46a7-b7d7-8f9f85e57bd6";

/// The built-in the shipped Activities are.
pub fn activities() -> BuiltIn {
    *BUILT_INS
        .iter()
        .find(|built_in| built_in.directory == "activities")
        .expect("build.rs embeds every directory under plugins/, and plugins/activities is one")
}

/// A built-in, once the start has tried to load it.
#[derive(Debug)]
pub struct Started {
    pub built_in: BuiltIn,
    /// Its manifest as this platform reads it, where it could be read.
    pub manifest: Option<Manifest>,
    /// Why it did not load, where it did not.
    pub trouble: Option<Trouble>,
}

/// Load every built-in this binary carries, as `platform` reads them.
pub async fn started(platform: Platform, home: Option<&Path>) -> Vec<Started> {
    let mut started = Vec::new();
    for built_in in BUILT_INS {
        started.push(start(*built_in, platform, home).await);
    }
    started
}

/// Load one built-in: its manifest read, and its wasm component instantiated
/// under the capabilities that manifest asks for.
pub async fn start(built_in: BuiltIn, platform: Platform, home: Option<&Path>) -> Started {
    let files = built_in.files();
    let manifest = match built_in.manifest(platform) {
        Ok(manifest) => manifest,
        Err(refusal) => {
            return Started {
                built_in,
                manifest: None,
                trouble: Some(Trouble { why: refusal.why }),
            };
        }
    };

    let trouble = component::load(files, &manifest, home).await.err();
    Started {
        built_in,
        manifest: Some(manifest),
        trouble,
    }
}

impl Started {
    /// What the log calls it: its display name, or its directory where the
    /// manifest could not be read.
    pub fn name(&self) -> &str {
        self.manifest
            .as_ref()
            .map_or(self.built_in.directory, |manifest| &manifest.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::Capability;

    /// The fixture's bytes are in the binary, whichever way `build.rs` came by
    /// them, and it loads.
    #[tokio::test]
    async fn the_embedded_fixture_loads() {
        let started = started(Platform::HERE, None).await;

        let fixture = started
            .iter()
            .find(|started| {
                started
                    .manifest
                    .as_ref()
                    .is_some_and(|manifest| manifest.uuid == FIXTURE)
            })
            .expect("the fixture is embedded");
        assert_eq!(fixture.trouble, None);
        assert!(
            fixture
                .built_in
                .files
                .iter()
                .any(|(name, bytes)| { *name == "fixture.wasm" && bytes.starts_with(b"\0asm") }),
            "the fixture carries its wasm component",
        );
    }

    /// The skills Verkstead ships are a built-in providing the one pack
    /// `verkstead`, read through the manifest reader — out of the binary as a
    /// release reads them, and off the checkout as a debug build does.
    #[tokio::test]
    async fn the_verkstead_pack_is_a_built_in_with_its_thirteen_skills() {
        let shipped = verkstead();

        let started = start(shipped, Platform::HERE, None).await;
        assert_eq!(started.trouble, None);

        for files in [Files::Embedded(shipped.files), shipped.files()] {
            let manifest = Manifest::parse(shipped.manifest, files, Platform::HERE).unwrap();
            assert_eq!(manifest.uuid, VERKSTEAD);
            assert_eq!(manifest.packs.len(), 1);
            assert_eq!(manifest.packs[0].name, "verkstead");
            assert_eq!(manifest.packs[0].skills.len(), 13, "{manifest:?}");
            assert!(
                manifest.packs[0]
                    .skills
                    .iter()
                    .any(|skill| skill == "grilling")
            );
        }
    }

    /// The shipped Activities are a built-in depending on the pack
    /// `verkstead`, whose *Follow-up rounds*, *Backlog runner*, *Stage runner*,
    /// *One session*, *Interview* and *Wrap-up* blocks each name a component of
    /// their own written against the `activity` world.
    #[tokio::test]
    async fn the_activities_are_a_built_in_with_their_components() {
        let shipped = activities();

        let started = start(shipped, Platform::HERE, None).await;
        assert_eq!(started.trouble, None);

        let manifest = started.manifest.expect("the manifest reads");
        assert_eq!(manifest.uuid, ACTIVITIES);
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].uuid, VERKSTEAD);

        let [rounds, backlog, stages, one, interview, wrap_up] = manifest.activities.as_slice()
        else {
            panic!("six Activities: {:?}", manifest.activities);
        };
        assert_eq!(rounds.id, "follow-up-rounds");
        assert_eq!(rounds.name, "Follow-up rounds");
        assert_eq!(rounds.outcomes, ["committed", "nothing"]);
        assert_eq!(
            rounds.skills,
            ["verkstead/following-up", "verkstead/investigating"]
        );

        assert_eq!(backlog.id, "backlog-runner");
        assert_eq!(backlog.name, "Backlog runner");
        assert_eq!(backlog.outcomes, ["finished"]);
        assert_eq!(
            backlog.skills,
            ["verkstead/next-task", "verkstead/next-stage"]
        );

        assert_eq!(stages.id, "stage-runner");
        assert_eq!(stages.name, "Stage runner");
        assert_eq!(stages.outcomes, ["finished", "continued"]);
        assert!(stages.skills.is_empty(), "it launches no session");
        assert_eq!(one.id, "one-session");
        assert_eq!(one.name, "One session");
        assert_eq!(one.outcomes, ["done", "refused"]);
        assert_eq!(
            one.skills,
            [
                "verkstead/implementing",
                "verkstead/submitting",
                "verkstead/instruction"
            ]
        );
        assert_eq!(
            manifest.capabilities,
            [Capability::Conversations],
            "the stage runner starts conversations, and the plugin says so at install",
        );

        assert_eq!(interview.id, "interview");
        assert_eq!(interview.name, "Interview");
        assert_eq!(
            interview.outcomes,
            [
                "direction:inline",
                "direction:task-list",
                "direction:roadmap",
                "gone"
            ]
        );
        assert_eq!(interview.skills, ["verkstead/grilling"]);
        assert_eq!(wrap_up.id, "wrap-up");
        assert_eq!(wrap_up.name, "Wrap-up");
        assert_eq!(
            wrap_up.outcomes,
            ["settled", "merged", "closed", "given-up"]
        );
        assert_eq!(
            wrap_up.skills,
            [
                "verkstead/submitting",
                "verkstead/addressing",
                "verkstead/reviewing",
                "verkstead/responding"
            ]
        );

        for component in [
            "follow-up-rounds.wasm",
            "backlog-runner.wasm",
            "stage-runner.wasm",
            "one-session.wasm",
            "interview.wasm",
            "wrap-up.wasm",
        ] {
            assert!(
                shipped
                    .files
                    .iter()
                    .any(|(name, bytes)| *name == component && bytes.starts_with(b"\0asm")),
                "each block's component is embedded beside the manifest: {component}",
            );
        }
    }

    /// Codex's plugin is what `verkstead plugin check` would accept, on every
    /// platform — its manifest read for each, and its component started and
    /// exporting the three things an agent is — under Codex's built-in uuid,
    /// which the store reads as the word `codex`.
    #[tokio::test]
    async fn codex_is_a_built_in_every_platform_loads() {
        let codex = codex();

        for platform in [Platform::Linux, Platform::MacOs, Platform::Windows] {
            let started = start(codex, platform, None).await;
            assert_eq!(started.trouble, None, "{platform:?}");
            let manifest = started.manifest.expect("the manifest reads");
            assert_eq!(manifest.name, "Codex");
            assert_eq!(manifest.uuid, CODEX);
            assert_eq!(
                crate::store::AgentType::read(&manifest.uuid),
                crate::store::AgentType::CODEX
            );
        }
        assert!(
            codex
                .files
                .iter()
                .any(|(name, bytes)| *name == "codex.wasm" && bytes.starts_with(b"\0asm")),
            "Codex carries its wasm component",
        );
    }

    /// Bytes that are not a component are trouble on that built-in, and the
    /// load comes back rather than failing.
    #[tokio::test]
    async fn a_built_in_that_will_not_instantiate_is_trouble() {
        let fixture = BUILT_INS
            .iter()
            .find(|built_in| built_in.directory == "fixture")
            .expect("the fixture is embedded");
        let broken = BuiltIn {
            files: &[("fixture.wasm", b"\0asm not a component")],
            ..*fixture
        };

        let started = start(broken, Platform::HERE, None).await;

        assert_eq!(started.name(), "Fixture");
        let trouble = started.trouble.expect("trouble");
        assert!(
            trouble
                .why
                .starts_with("The component isn't a WebAssembly component"),
            "{trouble}",
        );
    }

    /// A component that traps as it starts is trouble too.
    #[tokio::test]
    async fn a_built_in_that_traps_is_trouble() {
        let fixture = BUILT_INS
            .iter()
            .find(|built_in| built_in.directory == "fixture")
            .expect("the fixture is embedded");
        let traps = wat::parse_str(
            r#"(component
                 (core module $m (func $start unreachable) (start $start))
                 (core instance (instantiate $m)))"#,
        )
        .unwrap();
        let traps: &'static [u8] = traps.leak();
        let files: &'static [(&'static str, &'static [u8])] =
            Vec::from([("fixture.wasm", traps)]).leak();

        let started = start(BuiltIn { files, ..*fixture }, Platform::HERE, None).await;

        let trouble = started.trouble.expect("trouble");
        assert!(
            trouble
                .why
                .starts_with("The component stopped while it was starting"),
            "{trouble}",
        );
    }

    /// A manifest the reader refuses is trouble on the built-in, named by its
    /// directory since it has no name to go by.
    #[tokio::test]
    async fn a_built_in_whose_manifest_is_refused_is_trouble() {
        let broken = BuiltIn {
            directory: "broken",
            manifest: "uuid: not-a-uuid\n",
            files: &[],
            checkout: None,
        };

        let started = start(broken, Platform::HERE, None).await;

        assert_eq!(started.name(), "broken");
        assert!(started.trouble.is_some());
    }
}
