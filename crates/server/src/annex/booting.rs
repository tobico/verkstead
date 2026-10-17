//! The Keeper's **Annex** booted with it, linked again, and stopped with it
//! (ADR-0024, *Boot with the Keeper, terminate with it*).
//!
//! **In the background, and never waited on.** A Keeper's start spawns
//! [`Annexed::started`] once its listeners are bound and goes on serving: a box
//! is seconds to minutes from up, and a workbench that would not answer until
//! it was would be the Annex costing the Keeper the thing it is for. Where the
//! probe finds no box made to the end, there is nothing to do.
//!
//! **And once more where the box is made**: the wizard's install run ends by
//! booting and linking the box it has just rebuilt through [`Annexed::up`] —
//! the same handle, held on [`Devices`], so the box it booted is the one the
//! Keeper's stop terminates — and a first link that fails is the WSL row's
//! reason there.
//!
//! **Four steps where there is one.** The box is booted and held up — see
//! [`Backend::boot`] — and then, over and over until it works or the boot's
//! time runs out, the two addresses are read off it and the link is run:
//! [`Devices::keep`], which is idempotent and takes the Annex's Data Directory
//! from inside the box (see [`InTheBox`]). Both addresses are read afresh at
//! every start, so a NAT gateway that moved across a reboot is right again at
//! the next link. Retrying the link is the wait: until the Annex's service is
//! up the dial finds nothing, and the first one that gets through is the
//! answer.
//!
//! **And pinned to the Keeper's own release before it is linked**, where its
//! flake says another: the files written again, the Verkstead inputs alone
//! relocked, and the box rebuilt, all before the link puts it to use — so the
//! rebuild, which restarts the Annex's server, never lands on live work. The
//! Components row reads *updating* meanwhile. A re-pin that fails leaves the
//! box on its old release with the line on its row, and the next start tries
//! again — see [`super::pinning`].
//!
//! **And the release its flake pins read back once it is up**, and held for
//! the Components row: what the box runs is said of a stopped one too, and
//! nothing is booted to say it — see [`Annexed::reading`].
//!
//! **A box that will not come up is a line on its row**, held on the
//! [`Devices`] handle and drawn under the Annex's row on the Devices list, and
//! in the log. The Keeper carries on either way, and the row reads
//! *unreachable* until the Annex answers.
//!
//! **And stopped with the Keeper.** Where this server hears its stop, it
//! terminates the box — see [`Annexed::stopped`]. Where it does not — the
//! desktop app ends its sidecar with `TerminateProcess`, and a crash is a crash
//! — what held the box up dies with this process, and WSL's idle timeout stops
//! it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, bail};
use verkstead_render::{AnnexComponent, AnnexState};

use super::flake::Binary;
use super::{Backend, DATA_DIR, Inside, Probed, flake, pinning};
use crate::device::{AnnexData, CERTIFICATE_FILE, Devices};
use crate::settings::{Config, KeptBy};

/// How long a box has to come up and be linked: the boot, the service's start,
/// and the first dial that gets through. A cold WSL VM and a NixOS boot under
/// systemd are most of a minute on a slow machine.
pub const BOOT_TIME: Duration = Duration::from_secs(180);

/// How long between one try at the link and the next, while the box comes up.
const RETRY: Duration = Duration::from_secs(2);

/// What the NixOS module's service is called, which is asked about when the
/// Annex never answered.
const SERVICE: &str = "verkstead";

/// A Keeper's Annex, booted at its start and terminated at its stop.
#[derive(Debug)]
pub struct Annexed {
    annex: Arc<dyn Backend>,

    /// How long the box has to come up — [`BOOT_TIME`], but for a suite.
    patience: Duration,

    /// The binary the box is to run, which is what it is pinned to — see
    /// [`pinning::stale`].
    binary: Binary,

    /// Whether this start booted the box, which is what says there is one to
    /// terminate at the stop.
    booted: AtomicBool,

    /// Whether the box is being re-pinned and rebuilt at this moment, which the
    /// Components row reads as *updating*.
    updating: AtomicBool,

    /// The release the box is pinned to, read off its flake when this start
    /// booted it and held from then on — so the Components row says it of a
    /// box that has stopped since, and nothing is booted to read it. `None`
    /// until the first boot has.
    pinned: Mutex<Option<String>>,
}

impl Annexed {
    /// The Annex whose box is `annex`.
    pub fn of(annex: Arc<dyn Backend>) -> Annexed {
        Annexed {
            annex,
            patience: BOOT_TIME,
            binary: Binary::Released,
            booted: AtomicBool::new(false),
            updating: AtomicBool::new(false),
            pinned: Mutex::new(None),
        }
    }

    /// The same, giving the box `patience` to come up rather than
    /// [`BOOT_TIME`] — a suite's box that never will.
    pub fn waiting(self, patience: Duration) -> Annexed {
        Annexed { patience, ..self }
    }

    /// The same, the box running `binary` rather than the release's — what
    /// [`flake::ANNEX_BINARY`] names in a development build.
    pub fn running(self, binary: Binary) -> Annexed {
        Annexed { binary, ..self }
    }

    /// Boot the box where there is one, and link it — see the module's docs.
    /// What went wrong is put on `devices` for the Annex's row, and an Annex
    /// that came up takes any earlier line away.
    pub async fn started(&self, devices: &Devices) {
        let _ = self.up(devices).await;
    }

    /// The same, saying how it went: `Ok(true)` for an Annex up and linked,
    /// `Ok(false)` where there is no box made to the end, and the line put on
    /// `devices` where it would not come up.
    ///
    /// **And the install run's last step**, the box it has just made booted and
    /// linked by the same Keeper that terminates it at its stop — see
    /// [`crate::onboarding::install`] — so the line is also the WSL row's
    /// reason there.
    pub async fn up(&self, devices: &Devices) -> Result<bool, String> {
        let annex = self.annex.clone();
        let probed = tokio::task::spawn_blocking(move || annex.probe()).await;

        let Ok(Probed::Present { .. }) = probed else {
            return Ok(false);
        };

        let annex = self.annex.clone();
        let booted = tokio::task::spawn_blocking(move || annex.boot()).await;

        if let Err(why) = booted
            .map_err(anyhow::Error::from)
            .and_then(|booted| booted)
        {
            tracing::warn!(why = %format!("{why:#}"), "this device's Annex could not be booted");
            let line = sentence("The annex couldn't be started", &format!("{why:#}"));
            devices.annex_troubled(Some(line.clone()));
            return Err(line);
        }

        self.booted.store(true, Ordering::SeqCst);

        // Up now, so what is asked of it from inside starts nothing: where its
        // network is, and the release its flake pins.
        let annex = self.annex.clone();
        let networking = tokio::task::spawn_blocking(move || annex.networking())
            .await
            .unwrap_or(super::Networking::Nat);

        // Pinned to this Keeper's release before anything is linked to it, and
        // what failed held to stay on its row once the link has come up.
        let repinning = self.repinned().await;

        self.read_pinned().await;

        let port = devices.keeping().port;
        let started = tokio::time::Instant::now();

        let why = loop {
            let tried = self.link(devices, networking, port).await;

            let why = match tried {
                Ok(addresses) => {
                    tracing::info!(
                        annex = %addresses.annex,
                        keeper_at = %addresses.keeper_at,
                        "this device's Annex is up and linked",
                    );
                    devices.annex_troubled(repinning);
                    return Ok(true);
                }
                Err(why) => why,
            };

            if started.elapsed() + RETRY >= self.patience {
                break why;
            }

            tokio::time::sleep(RETRY).await;
        };

        let annex = self.annex.clone();
        let said = tokio::task::spawn_blocking(move || service(annex.as_ref()))
            .await
            .ok()
            .flatten();

        tracing::warn!(
            why = %format!("{why:#}"),
            service = said.as_deref().unwrap_or("unknown"),
            "this device's Annex did not answer within the boot's time",
        );

        let line = match said {
            Some(service) => sentence("The annex didn't answer", &service),
            None => sentence("The annex didn't answer", &format!("{why:#}")),
        };

        devices.annex_troubled(Some(line.clone()));
        Err(line)
    }

    /// The box, booted, pinned to this Keeper's release where it is pinned to
    /// another — see [`pinning`] — with the Components row reading *updating*
    /// while it is. `None` where it is pinned to this release already or now,
    /// and otherwise the line for the Annex's row.
    ///
    /// The rebuild's last lines go to the log, and the last of them to the
    /// row: what `nix` says about a build that failed is at the end.
    async fn repinned(&self) -> Option<String> {
        let system = flake::system(&crate::platform::hostname(), &self.binary);

        let stale = {
            let annex = self.annex.clone();
            let system = system.clone();
            let binary = self.binary.clone();

            tokio::task::spawn_blocking(move || pinning::stale(annex.as_ref(), &system, &binary))
                .await
                .unwrap_or(false)
        };

        if !stale {
            return None;
        }

        tracing::info!(
            release = flake::VERSION,
            "this device's Annex is pinned to another release, so it is being pinned to this one and rebuilt",
        );

        self.updating.store(true, Ordering::SeqCst);

        let annex = self.annex.clone();
        let binary = self.binary.clone();
        let repinned = tokio::task::spawn_blocking(move || {
            pinning::repinned(annex.as_ref(), &system, &binary, &mut |line| {
                tracing::debug!(line, "the Annex's rebuild");
            })
        })
        .await;

        self.updating.store(false, Ordering::SeqCst);

        match repinned
            .map_err(anyhow::Error::from)
            .and_then(|repinned| repinned)
        {
            Ok(()) => {
                tracing::info!(
                    release = flake::VERSION,
                    "this device's Annex is pinned to this release"
                );
                None
            }
            Err(why) => {
                let why = format!("{why:#}");
                tracing::warn!(%why, "this device's Annex could not be pinned to this release, and runs the one it ran");

                let last = why.lines().rfind(|line| !line.trim().is_empty());
                Some(sentence(
                    "The annex couldn't be updated",
                    last.unwrap_or_default(),
                ))
            }
        }
    }

    /// Read the release the box's flake pins and hold it, for the Components
    /// row to draw from then on. A flake that will not be read, or names no
    /// release, leaves what was held: the version is the row's, and a box that
    /// came up is not one that failed for it.
    async fn read_pinned(&self) {
        let annex = self.annex.clone();
        let read = tokio::task::spawn_blocking(move || annex.read(flake::IN_THE_BOX)).await;

        match read.map_err(anyhow::Error::from).and_then(|read| read) {
            Ok(text) => match flake::pinned(&String::from_utf8_lossy(&text)) {
                Some(version) => *self.pinned.lock().expect("never poisoned") = Some(version),
                None => tracing::warn!("this device's Annex's flake names no release"),
            },
            Err(why) => {
                tracing::warn!(why = %format!("{why:#}"), "this device's Annex's flake could not be read");
            }
        }
    }

    /// The Annex's row on the Components pane, read now: the backend's probe,
    /// the release held since the box was booted, and the size of its disk
    /// image — none of which boots it — and `trouble`, the line its Devices row
    /// carries.
    ///
    /// **Blocks**, the probe being `wsl.exe` run twice — and not at all while
    /// the box is being re-pinned, which is what it reads as then.
    pub fn reading(&self, trouble: Option<String>) -> AnnexComponent {
        let state = match self.updating.load(Ordering::SeqCst) {
            true => AnnexState::Updating,
            false => self.probed(),
        };

        AnnexComponent {
            state,
            version: self.pinned.lock().expect("never poisoned").clone(),
            disk: self.annex.taken(),
            trouble,
        }
    }

    /// The row's state, as the backend's probe says it.
    fn probed(&self) -> AnnexState {
        match self.annex.probe() {
            Probed::NoWsl | Probed::NoDistro => AnnexState::NotInstalled,
            Probed::NeedsRestart => AnnexState::NeedsRestart,
            Probed::VirtualizationOff => AnnexState::VirtualizationOff,
            Probed::Present { running: true } => AnnexState::Running,
            Probed::Present { running: false } => AnnexState::Stopped,
        }
    }

    /// One try at the link: the two addresses read off the box, and the
    /// Keeper's Join run with them.
    ///
    /// Nothing of the Keeper's Authors is handed in: the Annex is a linked
    /// Member, so they reach it as mirrors over the one list, token copies and
    /// all (ADR-0025).
    async fn link(
        &self,
        devices: &Devices,
        networking: super::Networking,
        port: u16,
    ) -> anyhow::Result<super::Addresses> {
        let annex = self.annex.clone();
        let addresses =
            tokio::task::spawn_blocking(move || annex.addresses(networking, port)).await??;

        devices
            .keep(
                Arc::new(InTheBox(self.annex.clone())),
                &addresses.annex.to_string(),
                Some(&addresses.keeper_at.to_string()),
            )
            .await?;

        Ok(addresses)
    }

    /// Terminate the box, where this start booted it — the ordered stop's half
    /// of the module's docs.
    pub async fn stopped(&self) {
        if !self.booted.swap(false, Ordering::SeqCst) {
            return;
        }

        let annex = self.annex.clone();

        match tokio::task::spawn_blocking(move || annex.terminate()).await {
            Ok(Ok(())) => tracing::info!("this device's Annex has been stopped with it"),
            Ok(Err(why)) => {
                tracing::warn!(why = %format!("{why:#}"), "this device's Annex could not be stopped");
            }
            Err(why) => tracing::warn!(%why, "this device's Annex could not be stopped"),
        }
    }
}

/// The line the Annex's row carries: `what`, and the first line of `why` as a
/// sentence after it. The first line alone, because what a box says can be a
/// YAML parser's caret diagram under it, which is the log's and not a row's.
fn sentence(what: &str, why: &str) -> String {
    let why = why.lines().next().unwrap_or_default().trim();

    if why.is_empty() {
        return format!("{what}.");
    }

    let stop = if why.ends_with(['.', '!', '?']) {
        ""
    } else {
        "."
    };

    format!("{what}: {why}{stop}")
}

/// What the box's service says of itself where it is not up: its state and the
/// last line it logged, or `None` where it is running, or would not say.
fn service(annex: &dyn Backend) -> Option<String> {
    let state = annex
        .run(
            Inside::Root,
            &[
                "systemctl",
                "show",
                "--property=ActiveState",
                "--value",
                SERVICE,
            ],
        )
        .ok()?;
    let state = String::from_utf8_lossy(&state.stdout).trim().to_owned();

    if state.is_empty() || state == "active" {
        return None;
    }

    let logged = annex
        .run(
            Inside::Root,
            &[
                "journalctl",
                "--unit",
                SERVICE,
                "--lines=1",
                "--output=cat",
                "--no-pager",
            ],
        )
        .ok()
        .map(|logged| String::from_utf8_lossy(&logged.stdout).trim().to_owned())
        .filter(|logged| !logged.is_empty());

    Some(match logged {
        Some(logged) => format!("its service is {state}. {logged}"),
        None => format!("its service is {state}."),
    })
}

/// The Annex's Data Directory read and written from inside its box, as the
/// box's root — see [`AnnexData`] for why not through `\\wsl.localhost`.
///
/// **The config is handed back to the service user** after it is written, for
/// the reason the install run hands the whole directory over: the service reads
/// it at every call, and a file only root can read is an Annex that stops
/// answering its Keeper.
#[derive(Debug)]
pub struct InTheBox(pub Arc<dyn Backend>);

impl InTheBox {
    fn config(&self) -> String {
        format!("{DATA_DIR}/config.yaml")
    }

    /// `bytes` written at `path` and handed to the service user — see the
    /// type's docs for why.
    fn handed(&self, path: &str, bytes: &[u8], mode: u32) -> anyhow::Result<()> {
        self.0.write(path, bytes, mode)?;

        let handed = self.0.run(
            Inside::Root,
            &["chown", &format!("{SERVICE}:{SERVICE}"), path],
        )?;

        if !handed.status.success() {
            bail!(
                "Verkstead couldn't hand {path} to the service: {}",
                String::from_utf8_lossy(&handed.stderr).trim()
            );
        }

        Ok(())
    }
}

impl AnnexData for InTheBox {
    fn shown(&self) -> String {
        format!("{DATA_DIR} in the {} distro", super::BOX_NAME)
    }

    fn certificate(&self) -> anyhow::Result<String> {
        let pem = self.0.read(&format!("{DATA_DIR}/{CERTIFICATE_FILE}"))?;

        String::from_utf8(pem).context("the certificate is not text")
    }

    fn keep_by(&self, keeper: &KeptBy) -> anyhow::Result<()> {
        let path = self.config();
        let text = self.0.read(&path)?;

        let mut config = Config::read(&String::from_utf8_lossy(&text))
            .with_context(|| format!("{path} does not read as a config"))?;

        if !config.keep_by(keeper) {
            return Ok(());
        }

        self.handed(&path, config.text()?.as_bytes(), 0o640)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annex::wsl::Wsl;
    use crate::annex::wsl::tests::{
        ANSWERING_STATUS, FACTS_NO_WSL, Stub, inside, listed, made, strings, utf16,
    };

    /// An Annex over `backend`.
    fn annexed(backend: Wsl) -> Annexed {
        Annexed::of(Arc::new(backend))
    }

    /// A machine with no WSL reads *not installed*, with nothing to say of a
    /// version or a disk.
    #[test]
    fn the_components_row_of_a_machine_with_no_wsl_is_not_installed() {
        let powershell = Stub::new();
        powershell.answers(1, FACTS_NO_WSL, 0);

        let missing = powershell.dir.path().join("no-such-wsl.exe");
        let annexed = annexed(Wsl::at(missing, &powershell.program));

        assert_eq!(
            annexed.reading(None),
            AnnexComponent {
                state: AnnexState::NotInstalled,
                version: None,
                disk: None,
                trouble: None,
            }
        );
    }

    /// A running box reads *running*, the release its flake pinned when it was
    /// booted, and its disk image's size; once it has stopped it reads
    /// *stopped* with that same release, and nothing is run inside to say so.
    #[tokio::test]
    async fn the_components_row_says_running_or_stopped_with_the_release_held_since_the_boot() {
        let (backend, wsl, _powershell) = made();
        std::fs::write(backend.disk().join("ext4.vhdx"), vec![0; 4096]).unwrap();

        wsl.answers(
            1,
            "verkstead.url = \"github:tobico/verkstead/v0.9.1\";\n",
            0,
        )
        .answers(2, utf16(ANSWERING_STATUS), 0)
        .answers(3, utf16(&listed("Running")), 0)
        .answers(4, utf16(ANSWERING_STATUS), 0)
        .answers(5, utf16(&listed("Stopped")), 0);

        let annexed = annexed(backend);
        annexed.read_pinned().await;

        let trouble = Some("The annex didn't answer.".to_owned());
        assert_eq!(
            annexed.reading(trouble.clone()),
            AnnexComponent {
                state: AnnexState::Running,
                version: Some("0.9.1".to_owned()),
                disk: Some(4096),
                trouble,
            }
        );

        assert_eq!(
            annexed.reading(None),
            AnnexComponent {
                state: AnnexState::Stopped,
                version: Some("0.9.1".to_owned()),
                disk: Some(4096),
                trouble: None,
            }
        );

        let probing = vec![strings(&["--status"]), strings(&["--list", "--verbose"])];
        let calls = wsl.calls();
        assert_eq!(
            calls[0],
            inside("root", &["cat", "--", "/etc/nixos/flake.nix"])
        );
        assert_eq!(
            calls[1..],
            [probing.clone(), probing].concat(),
            "nothing run inside"
        );
    }

    /// A development build's binary, named the way [`flake::ANNEX_BINARY`]
    /// names one, and the flake a box running it is pinned with.
    fn development_build(dir: &std::path::Path) -> (Binary, String) {
        let path = dir.join("verkstead");
        std::fs::write(&path, b"a Linux build").unwrap();
        let binary = Binary::Copied(path);

        let flake = flake::system("winvm", &binary).flake;
        (binary, flake)
    }

    /// A development build whose binary has changed since the box was
    /// rebuilt copies it in and rebuilds, though the flake is the same.
    #[tokio::test]
    async fn a_development_builds_changed_binary_is_copied_in_and_rebuilt() {
        let (backend, wsl, _powershell) = made();
        let (binary, pinned) = development_build(wsl.dir.path());

        wsl.answers(1, &pinned, 0)
            .answers(2, format!("{}  /etc/nixos/verkstead\n", "0".repeat(64)), 0)
            .answers(3, &pinned, 0);

        let annexed = annexed(backend);
        let annexed = annexed.running(binary);

        assert_eq!(annexed.repinned().await, None);

        let calls = wsl.calls();
        let written = |mode: &str, path: &str| {
            inside("root", &["install", "-D", "-m", mode, "/dev/stdin", path])
        };

        assert!(
            calls.contains(&written("755", "/etc/nixos/verkstead")),
            "{calls:?}"
        );
        assert_eq!(
            calls.last(),
            Some(&inside(
                "root",
                &[
                    "NIX_CONFIG=experimental-features = nix-command flakes",
                    "nixos-rebuild",
                    "switch",
                    "--flake",
                    "/etc/nixos#verkstead",
                ],
            ))
        );
    }

    /// And one whose binary is the one in the box already rebuilds nothing.
    #[tokio::test]
    async fn a_development_builds_unchanged_binary_rebuilds_nothing() {
        let (backend, wsl, _powershell) = made();
        let (binary, pinned) = development_build(wsl.dir.path());
        let Binary::Copied(path) = &binary else {
            unreachable!()
        };
        let digest = crate::annex::image::digest_of(path).unwrap();

        wsl.answers(1, &pinned, 0)
            .answers(2, format!("{digest}  /etc/nixos/verkstead\n"), 0);

        let annexed = annexed(backend);
        let annexed = annexed.running(binary);

        assert_eq!(annexed.repinned().await, None);
        assert_eq!(
            wsl.calls(),
            [
                inside("root", &["cat", "--", "/etc/nixos/flake.nix"]),
                inside("root", &["sha256sum", "--", "/etc/nixos/verkstead"]),
            ]
        );
    }

    /// The row's line is one sentence, whatever the box said under its first
    /// line.
    #[test]
    fn a_rows_line_is_the_first_line_as_a_sentence() {
        assert_eq!(
            sentence(
                "The annex didn't answer",
                "config.yaml does not read as a config: line 1 column 10\n --> <input>:1:10\n  |"
            ),
            "The annex didn't answer: config.yaml does not read as a config: line 1 column 10.",
        );
        assert_eq!(
            sentence("The annex didn't answer", "its service is failed."),
            "The annex didn't answer: its service is failed.",
        );
        assert_eq!(
            sentence("The annex didn't answer", ""),
            "The annex didn't answer."
        );
    }
}
