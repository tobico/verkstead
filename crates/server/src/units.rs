//! The units of a store: what the sweep takes out of it whole, found the way
//! its descriptor says — see [`crate::languages::Unit`].
//!
//! **Found by the one walk the disk use is measured by** — see
//! [`crate::disk_use::walk`] — so a unit is reached without following a
//! symlink, and the walk stops going down at one: nothing inside a unit is
//! another, and nothing inside it is read beyond the one listing a `holding`
//! pattern asks for.
//!
//! **A symlink is never a unit.** pnpm's `projects/` links into the Worktrees
//! and bun keeps a link per version beside the directory that is the version;
//! taking a link for a package would be counting something else's files, or
//! nothing at all.

use std::path::{Path, PathBuf};

use crate::disk_use::{Walking, walk};
use crate::languages::Unit;

/// Every unit in the store at `store`, by `units`, each once and none inside
/// another, in path order.
///
/// A store, or a unit's `under`, that is not there yet holds none. What cannot
/// be listed further down is passed over the way the disk use passes it over —
/// a unit missed this time is one found next time.
pub fn listed(store: &Path, units: &[Unit]) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for unit in units {
        let walked = walk(&unit.root(store), &mut |entry| {
            if entry.metadata.is_symlink() {
                return Walking::Past;
            }

            let name = entry
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();

            let children = || match entry.metadata.is_dir() {
                true => std::fs::read_dir(entry.path)
                    .map(|entries| {
                        entries
                            .filter_map(|entry| entry.ok())
                            .map(|entry| entry.file_name().to_string_lossy().into_owned())
                            .collect()
                    })
                    .unwrap_or_default(),
                false => Vec::new(),
            };

            if unit.is(&name, entry.depth, children) {
                found.push(entry.path.to_owned());

                return Walking::Past;
            }

            match unit.deepest(entry.depth) {
                true => Walking::Past,
                false => Walking::Into,
            }
        });

        if let Err(error) = walked {
            tracing::warn!(error = ?error, store = %store.display(), "a store's units could not be listed, so none of them was");
        }
    }

    // Two units' `under`s may overlap, one inside the other: what is inside a
    // unit found already is part of it rather than one of its own.
    found.sort();
    found.dedup();

    let mut whole: Vec<PathBuf> = Vec::with_capacity(found.len());

    for path in found {
        if !whole.last().is_some_and(|outer| path.starts_with(outer)) {
            whole.push(path);
        }
    }

    whole
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The units a store's descriptor says, read the way `config.yaml` is.
    fn units(yaml: &str) -> Vec<Unit> {
        serde_saphyr::from_str(yaml).unwrap()
    }

    fn write(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    /// What was found, as paths under `store` with `/` between their segments.
    fn found(store: &Path, yaml: &str) -> Vec<String> {
        listed(store, &units(yaml))
            .iter()
            .map(|unit| {
                unit.strip_prefix(store)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    /// Every entry at a depth under `under`, files as well as directories, and
    /// nothing shallower or beside it.
    #[test]
    fn a_unit_by_depth_is_every_entry_that_deep_under_its_root() {
        let dir = tempfile::tempdir().unwrap();

        write(
            &dir.path()
                .join("registry/src/index-1/greet-1.0.0/src/lib.rs"),
        );
        write(
            &dir.path()
                .join("registry/src/index-1/wave-0.1.0/Cargo.toml"),
        );
        write(&dir.path().join("registry/cache/index-1/greet-1.0.0.crate"));
        write(&dir.path().join("registry/src/stray"));
        write(&dir.path().join("elsewhere/a/b"));

        assert_eq!(
            found(
                dir.path(),
                "- under: registry/src\n  depth: 2\n- under: registry/cache\n  depth: 2\n",
            ),
            [
                "registry/cache/index-1/greet-1.0.0.crate",
                "registry/src/index-1/greet-1.0.0",
                "registry/src/index-1/wave-0.1.0",
            ],
        );
    }

    /// The first entry down whose name matches, at whatever depth, and the walk
    /// stops there: nothing inside a unit is another.
    #[test]
    fn a_unit_by_name_is_the_first_entry_down_that_matches() {
        let dir = tempfile::tempdir().unwrap();

        write(&dir.path().join("example.test/greet@v1.0.0/inner@v2/go.mod"));
        write(&dir.path().join("golang.org/x/text@v0.3.0/go.mod"));
        write(
            &dir.path()
                .join("cache/download/example.test/greet/@v/v1.0.0.zip"),
        );
        write(&dir.path().join("cache/lock"));

        assert_eq!(
            found(dir.path(), "- named: [\"*@*\"]\n"),
            [
                "cache/download/example.test/greet/@v",
                "example.test/greet@v1.0.0",
                "golang.org/x/text@v0.3.0",
            ],
        );
    }

    /// The first directory down holding an entry that matches — Maven's
    /// version directory, at whatever depth its group puts it.
    #[test]
    fn a_unit_by_what_it_holds_is_the_first_directory_down_holding_it() {
        let dir = tempfile::tempdir().unwrap();

        write(&dir.path().join("org/example/greet/1.0.0/greet-1.0.0.pom"));
        write(&dir.path().join("org/example/greet/1.0.0/greet-1.0.0.jar"));
        write(
            &dir.path()
                .join("org/example/greet/maven-metadata-central.xml"),
        );
        write(&dir.path().join("com/a/b/c/d/lib/2.0/_remote.repositories"));
        write(&dir.path().join(".locks/org.example~greet~1.0.0.lock"));

        assert_eq!(
            found(
                dir.path(),
                "- holding: [\"*.pom\", \"_remote.repositories\"]\n",
            ),
            ["com/a/b/c/d/lib/2.0", "org/example/greet/1.0.0"],
        );
    }

    /// Where more than one is said, an entry has to be all of them.
    #[test]
    fn a_unit_saying_two_things_is_both() {
        let dir = tempfile::tempdir().unwrap();

        write(&dir.path().join("cache/greet-npm-1.0.0.zip"));
        write(&dir.path().join("cache/.gitignore"));
        write(&dir.path().join("cache/deeper/other-npm-1.0.0.zip"));

        assert_eq!(
            found(
                dir.path(),
                "- under: cache\n  depth: 1\n  named: [\"*.zip\"]\n"
            ),
            ["cache/greet-npm-1.0.0.zip"],
        );
    }

    /// A link is never a unit and never gone into, however well it matches —
    /// bun's per-version link, or a link out into a project.
    #[cfg(unix)]
    #[test]
    fn a_symlink_is_never_a_unit() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");

        write(&store.join("greet@1.0.0@@@1/index.js"));
        write(&dir.path().join("project/other@2.0.0@@@1/index.js"));
        std::fs::create_dir_all(store.join("greet")).unwrap();
        std::os::unix::fs::symlink(store.join("greet@1.0.0@@@1"), store.join("greet/1.0.0@@@1"))
            .unwrap();
        std::os::unix::fs::symlink(dir.path().join("project"), store.join("linked")).unwrap();

        assert_eq!(
            found(&store, "- named: [\"*@*@@@*\"]\n"),
            ["greet@1.0.0@@@1"]
        );
    }

    /// A store nothing has filled, or a root not there yet, holds none.
    #[test]
    fn a_store_not_there_holds_no_units() {
        let dir = tempfile::tempdir().unwrap();

        assert!(found(&dir.path().join("never"), "- depth: 1\n").is_empty());
        assert!(found(dir.path(), "- under: not/yet\n  depth: 1\n").is_empty());
    }

    /// Two rules whose roots overlap find each unit once, and never one inside
    /// another.
    #[test]
    fn overlapping_rules_find_each_unit_once() {
        let dir = tempfile::tempdir().unwrap();

        write(&dir.path().join("a/b/c"));

        assert_eq!(
            found(
                dir.path(),
                "- depth: 1\n- under: a\n  depth: 1\n- depth: 1\n"
            ),
            ["a"],
        );
    }
}
