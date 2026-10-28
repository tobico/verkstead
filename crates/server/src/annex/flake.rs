//! What the Annex's box is told: the flake and configuration its Keeper writes
//! into `/etc/nixos`, and the `config.yaml` and `secrets.yaml` it writes into
//! the Annex's Data Directory (ADR-0024, *The box on Windows*).
//!
//! **Written whole, every time.** Nothing in these files is the human's, so
//! there is nothing to merge: the two `.nix` files are templates beside this
//! one with every `@name@` filled in, and the two YAML files are the server's
//! own settings types written as they write themselves, so the Annex reads them
//! exactly as a Verkstead reads its own.
//!
//! **The Annex runs the release its Keeper runs.** The flake's `verkstead`
//! input is the Keeper's own release tag, for the NixOS module, and the binary
//! is that same release's `verkstead-linux-x64` as a locked non-flake input —
//! never `nix/release.json`, which a tag's own tree names the *previous*
//! release in, because the manifest is committed after the tag.
//!
//! **A development build reports the release's version and is not it**, so
//! [`ANNEX_BINARY`] names a Linux binary to put in instead: the file is copied
//! beside the flake, the configuration runs it, and no binary input is written
//! at all. The release tag still supplies the module.
//!
//! Nothing here touches the box: what puts these files in it is the wizard's
//! install run, through [`super::Backend::run`], and the re-pin at the
//! Keeper's start — see [`super::pinning`].

use std::path::PathBuf;

use super::{ANNEX_PEER_PORT, ANNEX_WORKBENCH_PORT};
use crate::settings::{Config, KeptBy, Secrets};

/// The NixOS-WSL release the box's image is downloaded from and its module is
/// pinned at, which move together: an image from one release rebuilt onto the
/// module of another is a box nobody has tried.
pub const NIXOS_WSL: &str = "2605.7.2";

/// What the configuration is called in the flake, which is what
/// `nixos-rebuild switch --flake /etc/nixos#<this>` names.
pub const CONFIGURATION: &str = "verkstead";

/// What a development build's binary is called beside the flake, in
/// `/etc/nixos` — see [`Binary::Copied`].
pub const COPIED_BINARY: &str = "verkstead";

/// The environment variable that names a Linux binary to put in the box in
/// place of the release's — see [`Binary`].
pub const ANNEX_BINARY: &str = "VERKSTEAD_ANNEX_BINARY";

/// Where the Annex's Repos live: bound in through the module's `paths`, and
/// the parent a Repo made or cloned on the Annex goes under.
pub const REPOS: &str = "/srv/repos";

/// The Keeper's own version, which the box is pinned to.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The directory of the box the flake, its lock, its configuration and a
/// development build's binary are in.
pub const NIXOS: &str = "/etc/nixos";

/// Where the flake is in the box, which is where the release it is pinned to
/// is read back from — see [`pinned`].
pub const IN_THE_BOX: &str = "/etc/nixos/flake.nix";

/// And its configuration, and the lock its first rebuild wrote beside it.
pub const CONFIGURATION_IN_THE_BOX: &str = "/etc/nixos/configuration.nix";
pub const LOCK_IN_THE_BOX: &str = "/etc/nixos/flake.lock";

/// And where a development build's binary is copied to — see
/// [`Binary::Copied`].
pub const COPIED_IN_THE_BOX: &str = "/etc/nixos/verkstead";

/// What the `verkstead` input's url is up to the tag — see [`pinned`].
const RELEASE_URL: &str = "github:tobico/verkstead/v";

/// The system the box is, and the release asset built for it.
const SYSTEM: &str = "x86_64-linux";
const RELEASE_ASSET: &str = "verkstead-linux-x64";

/// The templates — see this module's documentation.
const FLAKE: &str = include_str!("box/flake.nix.in");
const CONFIGURATION_NIX: &str = include_str!("box/configuration.nix.in");

/// The repository's own flake, which is where the stable channel is read from
/// so that the box and the module's own checks cannot drift apart.
const REPOSITORY_FLAKE: &str = include_str!("../../../../flake.nix");

/// Which binary the box runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binary {
    /// The release's, fetched by the flake at the Keeper's version.
    Released,

    /// A file of the Keeper's machine, copied into `/etc/nixos` as
    /// [`COPIED_BINARY`] — a development build's.
    Copied(PathBuf),
}

impl Binary {
    /// What [`ANNEX_BINARY`] says, where it says anything.
    pub fn from_env() -> Binary {
        std::env::var_os(ANNEX_BINARY)
            .filter(|path| !path.is_empty())
            .map_or(Binary::Released, |path| Binary::Copied(path.into()))
    }
}

/// What the Keeper says of itself to the box it is making.
#[derive(Debug, Clone)]
pub struct Keeper<'a> {
    /// Its hostname, which the box takes as its own.
    pub hostname: &'a str,

    /// Its Device Id and fingerprint, which the Annex is kept by.
    ///
    /// And nothing of its Authors: the Annex is a linked Member, so those reach
    /// it as mirrors over the one list, each with its token's copy (ADR-0025) —
    /// see [`Config::of_an_annex`].
    pub kept_by: &'a KeptBy,
}

/// The four files, as text.
#[derive(Debug, Clone)]
pub struct Written {
    /// `/etc/nixos/flake.nix`. Its lock is the first rebuild's to write.
    pub flake: String,

    /// `/etc/nixos/configuration.nix`.
    pub configuration: String,

    /// `config.yaml` in the Annex's Data Directory.
    pub config: String,

    /// `secrets.yaml` beside it.
    pub secrets: String,
}

/// The two `.nix` files alone, which are what a re-pin writes again — see
/// [`super::pinning`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System {
    /// `/etc/nixos/flake.nix`.
    pub flake: String,

    /// `/etc/nixos/configuration.nix`.
    pub configuration: String,
}

/// The files that make `keeper`'s Annex, running `binary`.
pub fn written(keeper: &Keeper, binary: &Binary) -> std::io::Result<Written> {
    let System {
        flake,
        configuration,
    } = system(keeper.hostname, binary);

    let config = Config::of_an_annex(keeper.kept_by.clone()).text()?;
    let secrets = Secrets::default().text()?;

    Ok(Written {
        flake,
        configuration,
        config,
        secrets,
    })
}

/// The flake that pins the box to this Keeper's release running `binary`, and
/// the configuration that names it after `hostname`.
pub fn system(hostname: &str, binary: &Binary) -> System {
    let (binary_input, binary) = match binary {
        Binary::Released => (
            format!(
                "\n    # The release binary of that same version, its hash locked.\n    \
                 verkstead-binary = {{\n      \
                 url = \"file+https://github.com/tobico/verkstead/releases/download/v{VERSION}/{RELEASE_ASSET}\";\n      \
                 flake = false;\n    \
                 }};\n"
            ),
            "inputs.verkstead-binary".to_owned(),
        ),
        Binary::Copied(_) => (String::new(), format!("./{COPIED_BINARY}")),
    };

    let flake = filled(
        FLAKE,
        &[
            ("channel", channel()),
            ("nixosWsl", NIXOS_WSL),
            ("version", VERSION),
            ("binaryInput", &binary_input),
            ("configuration", CONFIGURATION),
            ("system", SYSTEM),
            ("binary", &binary),
        ],
    );

    let configuration = filled(
        CONFIGURATION_NIX,
        &[
            ("hostName", &host_name(hostname)),
            ("listen", &format!("127.0.0.1:{ANNEX_WORKBENCH_PORT}")),
            ("peerListen", &format!("0.0.0.0:{ANNEX_PEER_PORT}")),
            ("repos", REPOS),
            ("stateVersion", state_version()),
        ],
    );

    System {
        flake,
        configuration,
    }
}

/// The inputs that are Verkstead's in a flake [`system`] wrote for `binary`:
/// the release's module, and its binary where the flake fetches one. These are
/// what a re-pin moves in the lock, and nothing else.
pub fn verksteads(binary: &Binary) -> &'static [&'static str] {
    match binary {
        Binary::Released => &["verkstead", "verkstead-binary"],
        Binary::Copied(_) => &["verkstead"],
    }
}

/// The inputs **Update** moves in a flake [`system`] wrote for `binary`: both
/// nixpkgs, and the Verkstead inputs with them — every input but `nixos-wsl`,
/// which is pinned at the tag [`NIXOS_WSL`] names and moves with a release.
pub fn updated(binary: &Binary) -> &'static [&'static str] {
    match binary {
        Binary::Released => &[
            "nixpkgs",
            "nixpkgs-unstable",
            "verkstead",
            "verkstead-binary",
        ],
        Binary::Copied(_) => &["nixpkgs", "nixpkgs-unstable", "verkstead"],
    }
}

/// The release a flake this module wrote pins the box to: its `verkstead`
/// input's tag, without the `v` — `0.42.0`, say — or `None` in a flake that
/// names no release.
///
/// What the box was last rebuilt onto rather than what this Keeper is: a
/// Keeper updated since reads its old release here until the box is pinned
/// again.
pub fn pinned(flake: &str) -> Option<String> {
    let (_, after) = flake.split_once(RELEASE_URL)?;
    let tag = &after[..after.find('"')?];

    (!tag.is_empty()).then(|| tag.to_owned())
}

/// The stable channel the repository's own flake pins nixpkgs to:
/// `nixos-26.05`, say.
pub fn channel() -> &'static str {
    const PINNED: &str = "github:NixOS/nixpkgs/";

    let after = REPOSITORY_FLAKE
        .split_once(PINNED)
        .map(|(_, after)| after)
        .expect("the repository's flake.nix pins nixpkgs");

    &after[..after.find('"').expect("the pin is a quoted url")]
}

/// The NixOS release the channel is, which is the box's `stateVersion`: what it
/// was first made at, and so what its stateful defaults stay at.
fn state_version() -> &'static str {
    channel().trim_start_matches("nixos-")
}

/// `hostname` as NixOS takes one: letters, digits, `-` and `_`, starting and
/// ending with a letter or digit, at most 63 long. A Windows hostname is that
/// already, nearly always; one that is not loses what NixOS would refuse rather
/// than failing the rebuild.
fn host_name(hostname: &str) -> String {
    let label = hostname.split('.').next().unwrap_or_default();

    let kept: String = label
        .chars()
        .map(
            |c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                true => c,
                false => '-',
            },
        )
        .take(63)
        .collect();

    let kept = kept.trim_matches(|c: char| !c.is_ascii_alphanumeric());

    match kept.is_empty() {
        true => "verkstead".to_owned(),
        false => kept.to_owned(),
    }
}

/// `template` with every `@name@` replaced by its value.
fn filled(template: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(template.to_owned(), |text, (name, value)| {
            text.replace(&format!("@{name}@"), value)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn kept_by() -> KeptBy {
        KeptBy::of("86f1933fecb070cbee865fbb84819d14", "3F:0A:9C")
    }

    /// Whether `text` still holds an `@name@` nothing filled in.
    fn unfilled(text: &str) -> bool {
        regex::Regex::new("@[A-Za-z]+@").unwrap().is_match(text)
    }

    fn written_for(hostname: &str, binary: &Binary) -> Written {
        let kept_by = kept_by();

        written(
            &Keeper {
                hostname,
                kept_by: &kept_by,
            },
            binary,
        )
        .unwrap()
    }

    #[test]
    fn the_channel_is_the_repositorys_own() {
        assert!(channel().starts_with("nixos-"), "{}", channel());
        assert!(
            REPOSITORY_FLAKE.contains(&format!("\"github:NixOS/nixpkgs/{}\"", channel())),
            "{}",
            channel()
        );
        assert_eq!(format!("nixos-{}", state_version()), channel());
    }

    #[test]
    fn the_flake_pins_the_keepers_own_version_and_the_repositorys_channel() {
        let flake = written_for("winvm", &Binary::Released).flake;

        assert!(
            flake.contains(&format!("\"github:NixOS/nixpkgs/{}\"", channel())),
            "{flake}"
        );
        assert!(
            flake.contains(&format!("\"github:tobico/verkstead/v{VERSION}\"")),
            "{flake}"
        );
        assert!(
            flake.contains(&format!(
                "\"file+https://github.com/tobico/verkstead/releases/download/v{VERSION}/verkstead-linux-x64\""
            )),
            "{flake}"
        );
        assert!(
            flake.contains(&format!("\"github:nix-community/NixOS-WSL/{NIXOS_WSL}\"")),
            "{flake}"
        );
        assert!(
            flake.contains("binary = inputs.verkstead-binary;"),
            "{flake}"
        );
        assert!(!unfilled(&flake), "{flake}");
    }

    /// What a written flake pins is read back off it, which is the version
    /// the Components row draws.
    #[test]
    fn the_release_pinned_is_read_back_off_the_written_flake() {
        for binary in [Binary::Released, Binary::Copied("/tmp/verkstead".into())] {
            let flake = written_for("winvm", &binary).flake;

            assert_eq!(pinned(&flake).as_deref(), Some(VERSION), "{binary:?}");
        }

        assert_eq!(pinned("{ inputs = { }; }"), None);
        assert_eq!(
            pinned("verkstead.url = \"github:tobico/verkstead/v0.9.1\";"),
            Some("0.9.1".to_owned())
        );
    }

    #[test]
    fn a_copied_binary_writes_no_binary_input() {
        let flake = written_for("winvm", &Binary::Copied("/tmp/verkstead".into())).flake;

        assert!(!flake.contains("verkstead-binary"), "{flake}");
        assert!(!flake.contains("releases/download"), "{flake}");
        assert!(flake.contains("binary = ./verkstead;"), "{flake}");
        assert!(
            flake.contains(&format!("\"github:tobico/verkstead/v{VERSION}\"")),
            "the release tag still supplies the module: {flake}"
        );
    }

    /// Update moves every input the written flake declares but `nixos-wsl`,
    /// for either binary — so an input a later template adds is one it moves
    /// too, or this says so.
    #[test]
    fn update_moves_every_input_but_nixos_wsl() {
        for binary in [Binary::Released, Binary::Copied("/tmp/verkstead".into())] {
            let flake = written_for("winvm", &binary).flake;

            let (_, inputs) = flake.split_once("inputs = {").unwrap();
            let (inputs, _) = inputs.split_once("\n  };").unwrap();

            // An input is a line at the block's own depth that names one.
            let mut declared: Vec<&str> = inputs
                .lines()
                .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
                .filter_map(|line| {
                    let name = line.trim().split(['.', ' ']).next()?;
                    name.starts_with(|c: char| c.is_ascii_alphabetic())
                        .then_some(name)
                })
                .filter(|name| *name != "nixos-wsl")
                .collect();
            declared.sort_unstable();

            let mut moved = updated(&binary).to_vec();
            moved.sort_unstable();

            assert_eq!(moved, declared, "{binary:?}: {flake}");
        }
    }

    #[test]
    fn the_configuration_says_the_annexs_own_ports_repos_and_the_keepers_hostname() {
        let configuration = written_for("WINVM", &Binary::Released).configuration;

        for line in [
            "networking.hostName = \"WINVM\";",
            "listen = \"127.0.0.1:8426\";",
            "peerListen = \"0.0.0.0:8425\";",
            "paths = [ \"/srv/repos\" ];",
            "advertising = false;",
            &format!("system.stateVersion = \"{}\";", state_version()),
        ] {
            assert!(configuration.contains(line), "no {line} in {configuration}");
        }
        assert!(!unfilled(&configuration), "{configuration}");
    }

    #[test]
    fn a_hostname_nixos_would_refuse_is_made_one_it_takes() {
        for (hostname, taken) in [
            ("winvm", "winvm"),
            ("DESKTOP-4QK2J7", "DESKTOP-4QK2J7"),
            ("laptop.corp.example", "laptop"),
            ("tobi's pc", "tobi-s-pc"),
            ("-edge-", "edge"),
            ("", "verkstead"),
            ("…", "verkstead"),
        ] {
            assert_eq!(host_name(hostname), taken, "for {hostname:?}");
        }

        assert_eq!(host_name(&"a".repeat(80)).len(), 63);
    }

    #[test]
    fn the_settings_files_read_back_through_the_servers_own_types() {
        let files = written_for("winvm", &Binary::Released);

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.yaml"), &files.config).unwrap();
        std::fs::write(dir.path().join("secrets.yaml"), &files.secrets).unwrap();

        let settings = Settings::in_data_dir(dir.path());
        let config = settings.config();

        assert_eq!(config.kept_by(), Some(&kept_by()), "{}", files.config);
        assert!(
            config.authors().is_empty(),
            "the Keeper's Authors reach the Annex as mirrors, not as its own: {}",
            files.config,
        );
        assert_eq!(files.secrets, "", "and no token of its own either");
    }

    /// The written flake, evaluated by a real `nix`: the module's options, the
    /// hostname and the packages read back out of the configuration, and the
    /// whole system's derivation evaluates. Fetches both nixpkgs and NixOS-WSL,
    /// so it is minutes on a cold store — and skipped, saying so, where there
    /// is no `nix` to run.
    ///
    /// The Keeper's release is the one input it does not fetch. The flake
    /// names the release of this crate's version, and that tag is made only
    /// once the release's own tests have passed — so between a version bump
    /// and its tag, and for good after a release that failed part way, it is
    /// not there to fetch. The module comes from this checkout instead, which
    /// is the module that release will carry, and the binary is a stub file:
    /// an evaluation reads its path and never runs it. What the two inputs
    /// say, release and asset, is the written flake's own tests' to check.
    #[cfg(target_os = "linux")]
    mod evaluated {
        use super::*;
        use std::process::Command;

        /// What the package installs from a copied binary: a store path ending
        /// in its name, where the release input's is one named for the asset or
        /// `source`.
        const COPIED_INSTALL: &str = "verkstead $out/bin/verkstead";

        /// What the configuration reads back as, one JSON object.
        const READ_BACK: &str = "c: {
            listen = c.services.verkstead.listen;
            peerListen = c.services.verkstead.peerListen;
            paths = c.services.verkstead.paths;
            advertising = c.services.verkstead.advertising;
            hostName = c.networking.hostName;
            wslHostname = c.wsl.wslConf.network.hostname;
            packages = map (p: p.pname or p.name) c.environment.systemPackages;
            package = c.services.verkstead.package.buildCommand;
            toplevel = c.system.build.toplevel.drvPath;
        }";

        /// `files` written into a directory, evaluated, or `None` where there
        /// is no `nix`.
        fn evaluate(files: &Written, copied: bool) -> Option<(serde_json::Value, String)> {
            if Command::new("nix").arg("--version").output().is_err() {
                println!("skipping the Annex flake's evaluation: no nix on PATH");
                return None;
            }

            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("flake.nix"), &files.flake).unwrap();
            std::fs::write(dir.path().join("configuration.nix"), &files.configuration).unwrap();
            if copied {
                std::fs::write(dir.path().join(COPIED_BINARY), "#!/bin/sh\n").unwrap();
            }

            // Locked first, because only `flake lock` writes a lock that an
            // override has moved. This checkout as a git flake, so that only
            // what it tracks is copied and not `target`; `shallow` because
            // CI's checkout is; and dirty locks allowed because a working tree
            // usually is.
            let checkout = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap();
            let stub = tempfile::tempdir().unwrap();
            std::fs::write(stub.path().join(RELEASE_ASSET), "#!/bin/sh\n").unwrap();

            let mut lock = Command::new("nix");
            lock.args(["--extra-experimental-features", "nix-command flakes"])
                .args(["flake", "lock", "--allow-dirty-locks"])
                .args(["--override-input", "verkstead"])
                .arg(format!("git+file://{}?shallow=1", checkout.display()));
            if !copied {
                lock.args(["--override-input", "verkstead-binary"])
                    .arg(format!(
                        "file+file://{}",
                        stub.path().join(RELEASE_ASSET).display()
                    ));
            }
            let locked = lock
                .arg(format!("path:{}", dir.path().display()))
                .output()
                .unwrap();

            assert!(
                locked.status.success(),
                "{}",
                String::from_utf8_lossy(&locked.stderr)
            );

            let output = Command::new("nix")
                .args(["--extra-experimental-features", "nix-command flakes"])
                .args(["eval", "--allow-dirty-locks"])
                .arg("--json")
                .arg(format!(
                    "path:{}#nixosConfigurations.{CONFIGURATION}.config",
                    dir.path().display()
                ))
                .args(["--apply", READ_BACK])
                .output()
                .unwrap();

            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );

            let lock = std::fs::read_to_string(dir.path().join("flake.lock")).unwrap();

            Some((serde_json::from_slice(&output.stdout).unwrap(), lock))
        }

        #[test]
        fn the_written_flake_evaluates_to_what_it_says() {
            let Some((read, lock)) = evaluate(&written_for("winvm", &Binary::Released), false)
            else {
                return;
            };

            assert_eq!(read["listen"], "127.0.0.1:8426");
            assert_eq!(read["peerListen"], "0.0.0.0:8425");
            assert_eq!(read["paths"], serde_json::json!(["/srv/repos"]));
            assert_eq!(read["advertising"], false);
            assert_eq!(read["hostName"], "winvm");
            assert_eq!(read["wslHostname"], "winvm");

            let packages: Vec<&str> = read["packages"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|p| p.as_str())
                .collect();
            for wanted in [
                "bubblewrap",
                "git",
                "gh",
                "nix",
                "sccache",
                "claude-code",
                "codex",
                "opencode",
            ] {
                assert!(packages.contains(&wanted), "no {wanted} in {packages:?}");
            }

            let package = read["package"].as_str().unwrap();

            assert!(!package.contains(COPIED_INSTALL), "{package}");
            assert!(read["toplevel"].as_str().unwrap().ends_with(".drv"));
            assert!(lock.contains("\"verkstead-binary\""), "{lock}");
        }

        #[test]
        fn a_copied_binary_is_the_package_and_nothing_is_fetched_for_it() {
            let Some((read, lock)) = evaluate(
                &written_for("winvm", &Binary::Copied("/tmp/verkstead".into())),
                true,
            ) else {
                return;
            };

            let package = read["package"].as_str().unwrap();

            assert!(package.contains(COPIED_INSTALL), "{package}");
            assert!(!lock.contains("verkstead-binary"), "{lock}");
        }
    }
}
