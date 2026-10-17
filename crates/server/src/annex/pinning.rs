//! The box pinned to a release and rebuilt onto it: what the wizard's install
//! run does once, and what the Keeper's start does again whenever the box is
//! pinned to anything but the Keeper's own release (ADR-0024, *The box on
//! Windows*).
//!
//! **The Annex runs its Keeper's release.** At every start the flake in the box
//! is compared with the one this Keeper would write — see [`stale`] — and one
//! that differs is a box made by another release: a Keeper updated since, most
//! often. The two `.nix` files are then written whole again, the lock moves the
//! Verkstead inputs and nothing else, and the box is rebuilt — see
//! [`repinned`]. `nixpkgs`, `nixpkgs-unstable` and `nixos-wsl` keep what the
//! lock holds: the first two move on Update, and the third moves when a
//! release's template names another tag, which relocks it by itself.
//!
//! **Compared as the whole flake rather than as its version**, so that a
//! release which changes the template without moving the tag a development
//! build reports, or one that goes from a development build's binary back to
//! the release's, is a box re-pinned too. Two flakes this module wrote for one
//! release are the same text.
//!
//! **A development build's binary is compared by its bytes**, because its
//! flake names the release before it and says nothing of the build — see
//! [`Binary::Copied`].
//!
//! **A re-pin that fails puts the old files back**, so the box's flake goes on
//! saying the release the box runs, and the next start finds it stale again and
//! tries again.

use anyhow::bail;

use super::flake::{self, Binary, System};
use super::{Backend, Inside};

/// How many of a rebuild's last lines a failed one says — what `nix` says
/// about a build that failed is at the end, and it is what anybody would look
/// for.
pub const TAIL: usize = 12;

/// What every `nix` here is run with, because the configuration that turns
/// flakes on is the one being switched to: the image's own `nix` may not have
/// them on for the first rebuild.
const FLAKES: &str = "NIX_CONFIG=experimental-features = nix-command flakes";

/// Whether the box is pinned to something other than `system`, running
/// `binary`: its flake says otherwise, or a development build's binary in it is
/// not the one this Keeper names.
///
/// **A flake that will not be read is stale**: writing it again is what a box
/// with no flake needs, and a box that cannot be written to says why there.
pub fn stale(annex: &dyn Backend, system: &System, binary: &Binary) -> bool {
    match annex.read(flake::IN_THE_BOX) {
        Ok(pinned) if pinned == system.flake.as_bytes() => {}
        _ => return true,
    }

    let Binary::Copied(copied) = binary else {
        return false;
    };

    let Ok(wanted) = super::image::digest_of(copied) else {
        return true;
    };

    match annex.run(Inside::Root, &["sha256sum", "--", flake::COPIED_IN_THE_BOX]) {
        Ok(ran) if ran.status.success() => {
            let said = String::from_utf8_lossy(&ran.stdout);
            said.split_whitespace().next() != Some(wanted.as_str())
        }
        _ => true,
    }
}

/// The flake and configuration of `system` written into the box as its root,
/// and `binary` beside them where it is a development build's.
pub fn put(annex: &dyn Backend, system: &System, binary: &Binary) -> anyhow::Result<()> {
    annex.write(flake::IN_THE_BOX, system.flake.as_bytes(), 0o644)?;
    annex.write(
        flake::CONFIGURATION_IN_THE_BOX,
        system.configuration.as_bytes(),
        0o644,
    )?;

    if let Binary::Copied(binary) = binary {
        let bytes = std::fs::read(binary).map_err(|error| {
            anyhow::anyhow!(
                "Verkstead couldn't read {}, which {} names: {error}",
                binary.display(),
                flake::ANNEX_BINARY,
            )
        })?;

        annex.write(flake::COPIED_IN_THE_BOX, &bytes, 0o755)?;
    }

    Ok(())
}

/// The lock moved for `inputs` and for nothing else: the Verkstead inputs at a
/// re-pin — see [`flake::verksteads`] — and all but `nixos-wsl` at an Update —
/// see [`flake::updated`].
pub fn relocked(annex: &dyn Backend, inputs: &[&str]) -> anyhow::Result<()> {
    let mut argv = vec![FLAKES, "nix", "flake", "update", "--flake", flake::NIXOS];
    argv.extend_from_slice(inputs);

    let ran = annex.run(Inside::Root, &argv)?;

    if !ran.status.success() {
        let said = String::from_utf8_lossy(&ran.stderr);

        match said.lines().map(str::trim).rfind(|line| !line.is_empty()) {
            Some(last) => bail!("{last}"),
            None => bail!("nix flake update failed and said nothing."),
        }
    }

    Ok(())
}

/// The box rebuilt onto what is in `/etc/nixos`, each line it prints handed to
/// `line`, and the last [`TAIL`] of them the error where it failed.
pub fn rebuilt(annex: &dyn Backend, line: &mut dyn FnMut(&str)) -> anyhow::Result<()> {
    let flake = format!("{}#{}", flake::NIXOS, flake::CONFIGURATION);
    let mut tail = std::collections::VecDeque::with_capacity(TAIL);

    let status = annex.follow(
        Inside::Root,
        &[FLAKES, "nixos-rebuild", "switch", "--flake", &flake],
        &mut |said| {
            let said = said.trim();

            if said.is_empty() {
                return;
            }

            line(said);

            if tail.len() == TAIL {
                tail.pop_front();
            }
            tail.push_back(said.to_owned());
        },
    )?;

    if !status.success() {
        match tail.is_empty() {
            true => bail!("nixos-rebuild failed and said nothing."),
            false => bail!("{}", Vec::from(tail).join("\n")),
        }
    }

    Ok(())
}

/// The box pinned to `system`, running `binary`: written, relocked and
/// rebuilt, each line of the rebuild handed to `line`.
///
/// What failed is the error, and the flake, configuration and lock the box had
/// are put back first, so that what it says is what it runs. A development
/// build's binary is taken away rather than put back, which is what makes the
/// next start find the box stale.
pub fn repinned(
    annex: &dyn Backend,
    system: &System,
    binary: &Binary,
    line: &mut dyn FnMut(&str),
) -> anyhow::Result<()> {
    rebuilt_moving(annex, system, binary, flake::verksteads(binary), line)
}

/// The box **updated**: pinned to `system` as [`repinned`] pins it, with
/// every input of the lock moved but `nixos-wsl` rather than the Verkstead
/// inputs alone — see [`flake::updated`]. What failed is put back the same
/// way.
pub fn updated(
    annex: &dyn Backend,
    system: &System,
    binary: &Binary,
    line: &mut dyn FnMut(&str),
) -> anyhow::Result<()> {
    rebuilt_moving(annex, system, binary, flake::updated(binary), line)
}

/// The two above, which differ in the inputs they move and nothing else.
fn rebuilt_moving(
    annex: &dyn Backend,
    system: &System,
    binary: &Binary,
    inputs: &[&str],
    line: &mut dyn FnMut(&str),
) -> anyhow::Result<()> {
    let before: Vec<(&str, Vec<u8>)> = [
        flake::IN_THE_BOX,
        flake::CONFIGURATION_IN_THE_BOX,
        flake::LOCK_IN_THE_BOX,
    ]
    .into_iter()
    .filter_map(|path| Some((path, annex.read(path).ok()?)))
    .collect();

    let done = put(annex, system, binary)
        .and_then(|()| relocked(annex, inputs))
        .and_then(|()| rebuilt(annex, line));

    let Err(why) = done else {
        return Ok(());
    };

    for (path, bytes) in &before {
        if let Err(error) = annex.write(path, bytes, 0o644) {
            tracing::warn!(path, error = %format!("{error:#}"), "a file the re-pin wrote could not be put back");
        }
    }

    if let Binary::Copied(_) = binary {
        let _ = annex.run(Inside::Root, &["rm", "-f", "--", flake::COPIED_IN_THE_BOX]);
    }

    Err(why)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annex::wsl::tests::{Stub, inside, stubbed};

    /// What the box is pinned to here: a flake of another release, which is
    /// what a Keeper updated since finds.
    const OLD_FLAKE: &str = "verkstead.url = \"github:tobico/verkstead/v0.9.1\";\n";
    const OLD_CONFIGURATION: &str = "{ networking.hostName = \"winvm\"; }\n";
    const OLD_LOCK: &str = "{ \"nodes\": { \"nixpkgs\": { \"locked\": { \"rev\": \"abc\" } } } }\n";

    fn system(binary: &Binary) -> System {
        flake::system("winvm", binary)
    }

    fn write(mode: &str, path: &str) -> Vec<String> {
        inside("root", &["install", "-D", "-m", mode, "/dev/stdin", path])
    }

    fn cat(path: &str) -> Vec<String> {
        inside("root", &["cat", "--", path])
    }

    fn relock(inputs: &[&str]) -> Vec<String> {
        let mut argv = vec![FLAKES, "nix", "flake", "update", "--flake", "/etc/nixos"];
        argv.extend_from_slice(inputs);
        inside("root", &argv)
    }

    fn rebuild() -> Vec<String> {
        inside(
            "root",
            &[
                FLAKES,
                "nixos-rebuild",
                "switch",
                "--flake",
                "/etc/nixos#verkstead",
            ],
        )
    }

    /// A box whose flake is the one this Keeper writes is not stale, and one
    /// pinned to another release, or with no flake at all, is.
    #[test]
    fn a_box_is_stale_where_its_flake_is_not_this_keepers() {
        let system = system(&Binary::Released);

        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, &system.flake, 0)
            .answers(2, OLD_FLAKE, 0)
            .answers(3, "", 1);

        assert!(
            !stale(&backend, &system, &Binary::Released),
            "this release's"
        );
        assert!(
            stale(&backend, &system, &Binary::Released),
            "another release's"
        );
        assert!(stale(&backend, &system, &Binary::Released), "no flake");

        assert_eq!(wsl.calls(), vec![cat("/etc/nixos/flake.nix"); 3]);
    }

    /// A development build's box is stale where the binary in it is not the
    /// one named, and not where it is — the flake of the two being the same.
    #[test]
    fn a_development_builds_box_is_stale_where_its_binary_differs() {
        let named = Stub::new();
        let path = named.dir.path().join("verkstead");
        std::fs::write(&path, b"a Linux build").unwrap();
        let binary = Binary::Copied(path.clone());
        let system = system(&binary);

        let digest = crate::annex::image::digest_of(&path).unwrap();

        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, &system.flake, 0)
            .answers(2, format!("{digest}  /etc/nixos/verkstead\n"), 0)
            .answers(3, &system.flake, 0)
            .answers(4, format!("{}  /etc/nixos/verkstead\n", "0".repeat(64)), 0);

        assert!(!stale(&backend, &system, &binary), "the same bytes");
        assert!(stale(&backend, &system, &binary), "other bytes");

        let summed = inside("root", &["sha256sum", "--", "/etc/nixos/verkstead"]);
        assert_eq!(
            wsl.calls(),
            vec![
                cat("/etc/nixos/flake.nix"),
                summed.clone(),
                cat("/etc/nixos/flake.nix"),
                summed,
            ]
        );
    }

    /// A re-pin writes both `.nix` files again, moves the Verkstead inputs in
    /// the lock and no other, and rebuilds — the lines of the rebuild handed
    /// on as it prints them.
    #[test]
    fn a_repin_writes_relocks_only_verksteads_inputs_and_rebuilds() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, OLD_FLAKE, 0)
            .answers(2, OLD_CONFIGURATION, 0)
            .answers(3, OLD_LOCK, 0)
            .answers(7, "building the system configuration...\nactivating\n", 0);

        let mut lines = Vec::new();
        repinned(
            &backend,
            &system(&Binary::Released),
            &Binary::Released,
            &mut |line| lines.push(line.to_owned()),
        )
        .unwrap();

        assert_eq!(
            wsl.calls(),
            vec![
                cat("/etc/nixos/flake.nix"),
                cat("/etc/nixos/configuration.nix"),
                cat("/etc/nixos/flake.lock"),
                write("644", "/etc/nixos/flake.nix"),
                write("644", "/etc/nixos/configuration.nix"),
                relock(&["verkstead", "verkstead-binary"]),
                rebuild(),
            ]
        );
        assert_eq!(
            lines,
            ["building the system configuration...", "activating"]
        );
    }

    /// An Update writes the same files and rebuilds the same way, moving both
    /// nixpkgs with the Verkstead inputs, and `nixos-wsl` not at all.
    #[test]
    fn an_update_relocks_every_input_but_nixos_wsl() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, OLD_FLAKE, 0)
            .answers(2, OLD_CONFIGURATION, 0)
            .answers(3, OLD_LOCK, 0);

        updated(
            &backend,
            &system(&Binary::Released),
            &Binary::Released,
            &mut |_| {},
        )
        .unwrap();

        assert_eq!(
            wsl.calls()[3..],
            [
                write("644", "/etc/nixos/flake.nix"),
                write("644", "/etc/nixos/configuration.nix"),
                relock(&[
                    "nixpkgs",
                    "nixpkgs-unstable",
                    "verkstead",
                    "verkstead-binary"
                ]),
                rebuild(),
            ]
        );
    }

    /// A development build's re-pin copies its binary in beside the flake and
    /// moves the module's input alone, there being no binary input to move.
    #[test]
    fn a_development_builds_repin_copies_its_binary_in() {
        let named = Stub::new();
        let path = named.dir.path().join("verkstead");
        std::fs::write(&path, b"a Linux build").unwrap();
        let binary = Binary::Copied(path);

        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, OLD_FLAKE, 0)
            .answers(2, OLD_CONFIGURATION, 0)
            .answers(3, OLD_LOCK, 0);

        repinned(&backend, &system(&binary), &binary, &mut |_| {}).unwrap();

        assert_eq!(
            wsl.calls()[3..],
            [
                write("644", "/etc/nixos/flake.nix"),
                write("644", "/etc/nixos/configuration.nix"),
                write("755", "/etc/nixos/verkstead"),
                relock(&["verkstead"]),
                rebuild(),
            ]
        );
    }

    /// A rebuild that fails is the error in its last lines, and the three
    /// files the box had are put back, so its flake goes on saying the release
    /// it runs.
    #[test]
    fn a_failed_rebuild_puts_the_old_files_back() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, OLD_FLAKE, 0)
            .answers(2, OLD_CONFIGURATION, 0)
            .answers(3, OLD_LOCK, 0)
            .answers(
                7,
                "building the system configuration...\n\
                 error: builder for '/nix/store/abc-verkstead.drv' failed with exit code 1\n",
                1,
            );

        let failed = repinned(
            &backend,
            &system(&Binary::Released),
            &Binary::Released,
            &mut |_| {},
        )
        .unwrap_err();

        assert_eq!(
            format!("{failed:#}"),
            "building the system configuration...\n\
             error: builder for '/nix/store/abc-verkstead.drv' failed with exit code 1"
        );
        assert_eq!(
            wsl.calls()[7..],
            [
                write("644", "/etc/nixos/flake.nix"),
                write("644", "/etc/nixos/configuration.nix"),
                write("644", "/etc/nixos/flake.lock"),
            ]
        );
    }

    /// A relock that fails is its last line, and nothing is rebuilt.
    #[test]
    fn a_failed_relock_rebuilds_nothing() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, OLD_FLAKE, 0)
            .answers(2, OLD_CONFIGURATION, 0)
            .answers(3, OLD_LOCK, 0)
            .answers(6, "", 1);

        let failed = repinned(
            &backend,
            &system(&Binary::Released),
            &Binary::Released,
            &mut |_| {},
        )
        .unwrap_err();

        assert_eq!(
            format!("{failed:#}"),
            "nix flake update failed and said nothing."
        );
        assert!(!wsl.calls().contains(&rebuild()));
    }
}
