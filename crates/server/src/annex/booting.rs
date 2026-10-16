//! The Keeper's **Annex** booted with it, linked again, and stopped with it
//! (ADR-0024, *Boot with the Keeper, terminate with it*).
//!
//! **In the background, and never waited on.** A Keeper's start spawns
//! [`Annexed::started`] once its listeners are bound and goes on serving: a box
//! is seconds to minutes from up, and a workbench that would not answer until
//! it was would be the Annex costing the Keeper the thing it is for. Where the
//! probe finds no box made to the end, there is nothing to do.
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
//! **And the Keeper's git author and GitHub token handed in first**, at every
//! start rather than at make alone: the Annex's sessions commit as the Keeper
//! and authenticate as it, so a change on the Keeper reaches them the next
//! time the two start together — see [`InTheBox::commit_as`].
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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, bail};

use super::{Backend, DATA_DIR, Inside, Probed};
use crate::device::{AnnexData, CERTIFICATE_FILE, Devices};
use crate::settings::{Config, GitAuthor, KeptBy, Secrets, Settings};

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

    /// The Keeper's own settings, whose git author and GitHub token are handed
    /// to the Annex at every start — see [`InTheBox::commit_as`].
    keeper: Settings,

    /// How long the box has to come up — [`BOOT_TIME`], but for a suite.
    patience: Duration,

    /// Whether this start booted the box, which is what says there is one to
    /// terminate at the stop.
    booted: AtomicBool,
}

impl Annexed {
    /// The Annex whose box is `annex`, kept by the device whose settings are
    /// `keeper`.
    pub fn of(annex: Arc<dyn Backend>, keeper: Settings) -> Annexed {
        Annexed {
            annex,
            keeper,
            patience: BOOT_TIME,
            booted: AtomicBool::new(false),
        }
    }

    /// The same, giving the box `patience` to come up rather than
    /// [`BOOT_TIME`] — a suite's box that never will.
    pub fn waiting(self, patience: Duration) -> Annexed {
        Annexed { patience, ..self }
    }

    /// Boot the box where there is one, and link it — see the module's docs.
    /// What went wrong is put on `devices` for the Annex's row, and an Annex
    /// that came up takes any earlier line away.
    pub async fn started(&self, devices: &Devices) {
        let annex = self.annex.clone();
        let probed = tokio::task::spawn_blocking(move || annex.probe()).await;

        let Ok(Probed::Present { networking, .. }) = probed else {
            return;
        };

        let annex = self.annex.clone();
        let booted = tokio::task::spawn_blocking(move || annex.boot()).await;

        if let Err(why) = booted
            .map_err(anyhow::Error::from)
            .and_then(|booted| booted)
        {
            tracing::warn!(why = %format!("{why:#}"), "this device's Annex could not be booted");
            devices.annex_troubled(Some(sentence(
                "The annex couldn't be started",
                &format!("{why:#}"),
            )));
            return;
        }

        self.booted.store(true, Ordering::SeqCst);

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
                    devices.annex_troubled(None);
                    return;
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

        devices.annex_troubled(Some(match said {
            Some(service) => sentence("The annex didn't answer", &service),
            None => sentence("The annex didn't answer", &format!("{why:#}")),
        }));
    }

    /// One try at the link: the Keeper's git author and GitHub token handed
    /// in, the two addresses read off the box, and the Keeper's Join run with
    /// them.
    async fn link(
        &self,
        devices: &Devices,
        networking: super::Networking,
        port: u16,
    ) -> anyhow::Result<super::Addresses> {
        let in_the_box = InTheBox(self.annex.clone());
        let keeper = self.keeper.clone();
        tokio::task::spawn_blocking(move || {
            in_the_box.commit_as(
                keeper.config().git_author(),
                keeper.secrets().github_token(),
            )
        })
        .await??;

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

    fn secrets(&self) -> String {
        format!("{DATA_DIR}/secrets.yaml")
    }

    /// The Annex's sessions committing as `author` and authenticating with
    /// `token`, which are the Keeper's (ADR-0024): written into its
    /// `config.yaml` and `secrets.yaml` where they say otherwise, every other
    /// key as it stands.
    ///
    /// **At every start rather than at make alone**, so a git author or token
    /// changed on the Keeper reaches the Annex the next time the two start
    /// together. A file that says so already is not written at all.
    pub fn commit_as(&self, author: &GitAuthor, token: Option<&str>) -> anyhow::Result<()> {
        let path = self.config();
        let text = self.0.read(&path)?;

        let mut config = Config::read(&String::from_utf8_lossy(&text))
            .with_context(|| format!("{path} does not read as a config"))?;

        if config.commit_as(author) {
            self.handed(&path, config.text()?.as_bytes(), 0o640)?;
        }

        let path = self.secrets();
        let text = self.0.read(&path)?;

        let secrets = Secrets::read(&String::from_utf8_lossy(&text))
            .with_context(|| format!("{path} does not read as secrets"))?;

        if secrets.github_token() != token {
            let secrets = secrets.with_token(token.map(str::to_owned));
            self.handed(&path, secrets.text()?.as_bytes(), 0o600)?;
        }

        Ok(())
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
    use super::sentence;

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
