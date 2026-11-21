//! The Plugins Verkstead ships, inside the binary: each directory under the
//! repository's `plugins/`, its manifest and its wasm component embedded by
//! `build.rs` (ADR-0030, *The shape of a plugin*).
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

use super::component::{self, Trouble};
use super::manifest::{Files, Manifest};
use crate::platform::Platform;

/// One built-in as `build.rs` embedded it.
#[derive(Debug, Clone, Copy)]
pub struct BuiltIn {
    /// Its directory under `plugins/`, which names it in the log until its
    /// manifest has been read.
    pub directory: &'static str,
    /// Its `plugin.yaml`.
    pub manifest: &'static str,
    /// The files its manifest names, by their paths in its directory: its
    /// wasm component, where it has one.
    pub files: &'static [(&'static str, &'static [u8])],
}

/// Every built-in this binary carries.
pub const BUILT_INS: &[BuiltIn] = include!(concat!(env!("OUT_DIR"), "/built_ins.rs"));

/// The fixture's uuid: a built-in that provides nothing, whose wasm component
/// exports nothing, and which is not listed anywhere. It is the proof that
/// the path above works on every platform a release is built for.
pub const FIXTURE: &str = "613e2a43-80ee-4307-8623-28969e05330f";

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
    let files = Files::Embedded(built_in.files);
    let manifest = match Manifest::parse(built_in.manifest, files, platform) {
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
        };

        let started = start(broken, Platform::HERE, None).await;

        assert_eq!(started.name(), "broken");
        assert!(started.trouble.is_some());
    }
}
