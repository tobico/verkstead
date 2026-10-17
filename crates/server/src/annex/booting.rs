//! The Keeper's **Annex** booted with it, linked again, and stopped with it
//! (ADR-0024, *Boot with the Keeper, terminate with it*).
//!
//! **In the background, and never waited on.** A Keeper's start spawns
//! [`Annexed::started`] once its listeners are bound and goes on serving: a box
//! is seconds to minutes from up, and a workbench that would not answer until
//! it was would be the Annex costing the Keeper the thing it is for. Where the
//! probe finds no box made to the end, there is nothing to boot.
//!
//! **And an Annex whose box has gone is forgotten**, where the probe finds no
//! WSL, or no distro and no disk image of one, and this Keeper holds an Annex: one
//! unregistered by hand is noticed rather than offered, its row dropped here
//! and on every member — see [`Devices::forget_annex`]. A probe that could not
//! be read is not "no box", and forgets nothing.
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
//! **And the Keeper's settings about the work handed in first**, at every
//! start rather than at make alone: the Annex has no settings of its own, so
//! its sessions are given the Keeper's Instructions, MCP servers, at-once limit
//! and Cleanup, and a change on the Keeper reaches them the next time the two
//! start together — see [`InTheBox::working_as`]. Not its Authors: those reach
//! the Annex as mirrors over the one list, token copies and all (ADR-0025).
//!
//! **And pinned to the Keeper's own release before it is linked**, where its
//! flake says another: the files written again, the Verkstead inputs alone
//! relocked, and the box rebuilt, all before the link puts it to use — so the
//! rebuild, which restarts the Annex's server, never lands on live work. The
//! Components row reads *updating* meanwhile. A re-pin that fails leaves the
//! box on its old release with the line on its row, and the next start tries
//! again — see [`super::pinning`].
//!
//! **And updated on the human's press**, which is the same rebuild moving
//! both nixpkgs with the Verkstead inputs, then the same link again, because
//! the rebuild restarted the Annex's server — refused while a session runs
//! there, naming the Conversation. See [`Annexed::update`].
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
use crate::settings::{Config, KeptBy, Secrets, Settings};

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

    /// The Keeper's own settings, whose keys about the work are handed to the
    /// Annex at every start — see [`InTheBox::working_as`].
    keeper: Settings,

    /// How long the box has to come up — [`BOOT_TIME`], but for a suite.
    patience: Duration,

    /// The binary the box is to run, which is what it is pinned to — see
    /// [`pinning::stale`].
    binary: Binary,

    /// Whether this start booted the box, which is what says there is one to
    /// terminate at the stop.
    booted: AtomicBool,

    /// Whether the box is being re-pinned and rebuilt at this moment, or
    /// updated and linked again, which the Components row reads as *updating*.
    updating: AtomicBool,

    /// Held by a press of Update while it is refused or started, so that two
    /// presses are one after the other — see [`Annexed::update`].
    pressing: tokio::sync::Mutex<()>,

    /// The release the box is pinned to, read off its flake when this start
    /// booted it and held from then on — so the Components row says it of a
    /// box that has stopped since, and nothing is booted to read it. `None`
    /// until the first boot has.
    pinned: Mutex<Option<String>>,
}

impl Annexed {
    /// The Annex whose box is `annex`, kept by the device whose settings are
    /// `keeper`.
    pub fn of(annex: Arc<dyn Backend>, keeper: Settings) -> Annexed {
        Annexed {
            annex,
            keeper,
            patience: BOOT_TIME,
            binary: Binary::Released,
            booted: AtomicBool::new(false),
            updating: AtomicBool::new(false),
            pressing: tokio::sync::Mutex::new(()),
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
    /// `Ok(false)` where there is no box made to the end — the Annex forgotten,
    /// where there was one and the box is gone — and the line put on `devices`
    /// where it would not come up.
    ///
    /// **And the install run's last step**, the box it has just made booted and
    /// linked by the same Keeper that terminates it at its stop — see
    /// [`crate::onboarding::install`] — so the line is also the WSL row's
    /// reason there.
    pub async fn up(&self, devices: &Devices) -> Result<bool, String> {
        let annex = self.annex.clone();
        let probed = tokio::task::spawn_blocking(move || annex.probe()).await;

        match probed {
            Ok(Probed::Present { .. }) => {}

            // No box, where this Keeper held an Annex: it was taken away by
            // hand, so it is forgotten rather than offered — see
            // [`Annexed::forgotten`]. A probe that could not be read, or a box
            // that cannot run until a restart or the firmware, forgets nothing;
            // nor does a box still listed and never said to be made, whose disk
            // image is still there.
            Ok(Probed::NoWsl) => {
                self.forgotten(devices).await;
                return Ok(false);
            }
            Ok(Probed::NoDistro) if self.annex.taken().is_none() => {
                self.forgotten(devices).await;
                return Ok(false);
            }

            _ => return Ok(false),
        }

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

        self.linked(devices, networking, repinning).await
    }

    /// The Annex forgotten, its box having gone from under it (ADR-0024):
    /// its row dropped here and on every member, and the release held for
    /// the Components row dropped with it — see [`Devices::forget_annex`]. A
    /// Keeper that held no Annex has nothing to forget.
    async fn forgotten(&self, devices: &Devices) {
        *self.pinned.lock().expect("never poisoned") = None;

        match devices.forget_annex().await {
            Ok(true) => tracing::info!(
                "this device's Annex's box is gone, so the Annex has been forgotten on every member",
            ),
            Ok(false) => {}
            Err(why) => tracing::error!(
                why = %format!("{why:#}"),
                "this device's Annex's box is gone, and the Annex could not be forgotten",
            ),
        }
    }

    /// The box, up, linked over and over until it answers or the boot's time
    /// runs out — `Ok(true)` once it has, with `held` put on its row, and
    /// otherwise the line said there. The tail of [`Annexed::up`], and of an
    /// Update, whose rebuild restarted the Annex's server.
    async fn linked(
        &self,
        devices: &Devices,
        networking: super::Networking,
        held: Option<String>,
    ) -> Result<bool, String> {
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
                    devices.annex_troubled(held);
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
        let repinned = self.rebuilt(system, pinning::repinned).await;
        self.updating.store(false, Ordering::SeqCst);

        repinned
    }

    /// The box rebuilt onto `system` by `how` — [`pinning::repinned`] or
    /// [`pinning::updated`] — and `None` where it was, or the line for the
    /// Annex's row where it was not. Whoever calls it says *updating* around
    /// it, for as long as it is theirs to say.
    ///
    /// The rebuild's last lines go to the log, and the last of them to the
    /// row: what `nix` says about a build that failed is at the end.
    async fn rebuilt(&self, system: flake::System, how: Rebuild) -> Option<String> {
        let annex = self.annex.clone();
        let binary = self.binary.clone();
        let rebuilt = tokio::task::spawn_blocking(move || {
            how(annex.as_ref(), &system, &binary, &mut |line| {
                tracing::debug!(line, "the Annex's rebuild");
            })
        })
        .await;

        match rebuilt
            .map_err(anyhow::Error::from)
            .and_then(|rebuilt| rebuilt)
        {
            Ok(()) => {
                tracing::info!(
                    release = flake::VERSION,
                    "this device's Annex is pinned to this release and rebuilt"
                );
                None
            }
            Err(why) => {
                let why = format!("{why:#}");
                tracing::warn!(%why, "this device's Annex could not be rebuilt, and runs what it ran");

                let last = why.lines().rfind(|line| !line.trim().is_empty());
                Some(sentence(
                    "The annex couldn't be updated",
                    last.unwrap_or_default(),
                ))
            }
        }
    }

    /// **Update** on the Components row: the box's inputs moved — every one
    /// but `nixos-wsl` — and it rebuilt and linked again, in the background,
    /// with the row reading *updating* until it is linked (ADR-0024).
    ///
    /// **Refused while a session runs in the Annex**, the line naming the
    /// Conversation, because the rebuild restarts the Annex's server and every
    /// session with it — see [`running_there`]. What is running is asked of the
    /// Annex at the press rather than taken off the merged list, which could be
    /// a refresh behind, and an Annex that cannot be asked refuses rather than
    /// guesses.
    ///
    /// **One at a time**, and none beside the re-pin at the start: a press
    /// while either is going is taken, and the row goes on reading the one
    /// there is. Presses are put through one by one, so a second one waits for
    /// the first to have been refused or started.
    pub async fn update(self: Arc<Self>, devices: Devices) -> Result<(), String> {
        let _pressed = self.pressing.lock().await;

        if self.updating.load(Ordering::SeqCst) {
            return Ok(());
        }

        let annex = self.annex.clone();
        let probed = tokio::task::spawn_blocking(move || annex.probe()).await;

        if !matches!(probed, Ok(Probed::Present { .. })) {
            return Err("There's no annex on this machine to update.".to_owned());
        }

        running_there(&devices).await?;

        self.updating.store(true, Ordering::SeqCst);

        let this = self.clone();
        tokio::spawn(async move {
            this.updated(&devices).await;
            this.updating.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    /// The Update itself, once it has been taken — see [`Annexed::update`].
    /// The release read again afterwards, a rebuilt box being pinned to this
    /// Keeper's, and the line of a failed rebuild kept on the row through the
    /// link that follows: the Annex answers on what it ran before.
    async fn updated(&self, devices: &Devices) {
        tracing::info!("this device's Annex is being updated");

        let system = flake::system(&crate::platform::hostname(), &self.binary);
        let trouble = self.rebuilt(system, pinning::updated).await;

        self.read_pinned().await;

        let annex = self.annex.clone();
        let networking = tokio::task::spawn_blocking(move || annex.networking())
            .await
            .unwrap_or(super::Networking::Nat);

        let _ = self.linked(devices, networking, trouble).await;
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
            Probed::NoWsl | Probed::NoDistro | Probed::Unread => AnnexState::NotInstalled,
            Probed::NeedsRestart => AnnexState::NeedsRestart,
            Probed::VirtualizationOff => AnnexState::VirtualizationOff,
            Probed::Present { running: true } => AnnexState::Running,
            Probed::Present { running: false } => AnnexState::Stopped,
        }
    }

    /// One try at the link: the Keeper's settings about the work handed in,
    /// read off its files at this moment, the two addresses read off the box,
    /// and the Keeper's Join run with them.
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
        let in_the_box = InTheBox(self.annex.clone());
        let keeper = self.keeper.clone();
        tokio::task::spawn_blocking(move || {
            in_the_box.working_as(&keeper.config(), &keeper.secrets())
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

/// A rebuild of the box onto a system, running a binary — the re-pin's or the
/// Update's, which differ in the inputs they move.
type Rebuild =
    fn(&dyn Backend, &flake::System, &Binary, &mut dyn FnMut(&str)) -> anyhow::Result<()>;

/// Where the Annex's own Conversations are read, over the Relay: the sidebar's
/// list, which over the Peer Listener is that device's rows alone, with the
/// archived ones in it too — see [`crate::ui::conversations`].
const ITS_CONVERSATIONS: &str = "/api/ui/conversations?archived=true";

/// What [`running_there`] reads of each row of the Annex's list: the fields of
/// a [`verkstead_render::ConversationEntry`] that say whether a session runs
/// on it and what it is called, and nothing else of it.
#[derive(Debug, serde::Deserialize)]
struct Running {
    branch: String,
    branch_named: bool,
    state: verkstead_render::Lifecycle,
    naming: bool,
    working: bool,
}

/// `Ok` where no session is running in this Keeper's Annex, asked of the Annex
/// now; and otherwise the refusal, naming the Conversation one is running on,
/// or the first of them and how many more. An Annex that is not linked, cannot
/// be reached or answers what cannot be read is a refusal of its own: nothing
/// is known about what it is running.
async fn running_there(devices: &Devices) -> Result<(), String> {
    const UNASKED: &str = "The annex couldn't be asked what it's running, so it wasn't updated.";

    let held = devices
        .membership()
        .rows()
        .await
        .map_err(|_| UNASKED.to_owned())?;

    let Some(annex) = held
        .iter()
        .find(|member| member.kept_by.as_deref() == Some(devices.id()))
    else {
        return Err(UNASKED.to_owned());
    };

    let said = crate::relaying::read_of(
        devices,
        &annex.device,
        ITS_CONVERSATIONS.to_owned(),
        "Conversations",
        crate::merging::MOST_A_LIST_IS,
    )
    .await
    .ok_or_else(|| UNASKED.to_owned())?;

    let rows: Vec<Running> = serde_json::from_slice(&said).map_err(|_| UNASKED.to_owned())?;

    let mut working = rows.iter().filter(|row| row.working);

    let Some(first) = working.next() else {
        return Ok(());
    };

    // Called what the sidebar calls it — see [`crate::merging::settled`].
    let named = match crate::merging::settled(first.branch_named, first.state, first.naming) {
        true => first.branch.clone(),
        false => "a draft".to_owned(),
    };

    Err(match working.count() {
        0 => format!("The annex can't be updated while a session runs on {named}. Stop it first."),
        1 => format!(
            "The annex can't be updated while sessions run on {named} and 1 more conversation. \
             Stop them first."
        ),
        more => format!(
            "The annex can't be updated while sessions run on {named} and {more} more \
             conversations. Stop them first."
        ),
    })
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

    /// The Annex's sessions working as the Keeper's would (ADR-0024, *The
    /// Annex has no settings of its own*): everything in `keeper`'s
    /// `config.yaml` and `secrets.yaml` that is about the work rather than the
    /// machine written into the Annex's — see [`Config::working_as`] and
    /// [`Secrets::working_as`] for which keys those are — every other key as it
    /// stands.
    ///
    /// **At every start rather than at make alone**, so a change on the Keeper
    /// reaches the Annex the next time the two start together. A file that
    /// says so already is not written at all, so a start with nothing changed
    /// touches neither.
    pub fn working_as(&self, keeper: &Config, secret: &Secrets) -> anyhow::Result<()> {
        let path = self.config();
        let text = self.0.read(&path)?;

        let config = Config::read(&String::from_utf8_lossy(&text))
            .with_context(|| format!("{path} does not read as a config"))?;

        let copied = config.working_as(keeper).text()?;

        if copied != config.text()? {
            self.handed(&path, copied.as_bytes(), 0o640)?;
        }

        let path = self.secrets();
        let text = self.0.read(&path)?;

        let secrets = Secrets::read(&String::from_utf8_lossy(&text))
            .with_context(|| format!("{path} does not read as secrets"))?;

        let copied = secrets.working_as(secret).text()?;

        if copied != secrets.text()? {
            self.handed(&path, copied.as_bytes(), 0o600)?;
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
    use super::*;
    use crate::annex::wsl::Wsl;
    use crate::annex::wsl::tests::{
        ANSWERING_STATUS, FACTS_NO_WSL, NO_DISTROS, Stub, inside, listed, made, strings, utf16,
    };

    /// An Annex over `backend`, kept by a Keeper with no settings of note.
    fn annexed(backend: Wsl) -> (Annexed, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let annexed = Annexed::of(Arc::new(backend), Settings::in_data_dir(dir.path()));

        (annexed, dir)
    }

    /// A machine with no WSL reads *not installed*, with nothing to say of a
    /// version or a disk.
    #[test]
    fn the_components_row_of_a_machine_with_no_wsl_is_not_installed() {
        let powershell = Stub::new();
        powershell.answers(1, FACTS_NO_WSL, 0);

        let missing = powershell.dir.path().join("no-such-wsl.exe");
        let (annexed, _dir) = annexed(Wsl::at(missing, &powershell.program));

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

        let (annexed, _dir) = annexed(backend);
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

        let (annexed, _dir) = annexed(backend);
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

        let (annexed, _dir) = annexed(backend);
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

    /// A Keeper with Instructions and an MCP server with an auth header, and
    /// the Annex's files as a start that copied them leaves them: a bind and
    /// its Keeper of its own beside what was copied.
    fn keeper_and_copied() -> (tempfile::TempDir, Settings, String, String) {
        let dir = tempfile::tempdir().unwrap();
        let keeper = Settings::in_data_dir(dir.path());

        std::fs::write(
            keeper.config_path(),
            "instructions: Say it plainly.\n\
             mcp_servers:\n- name: docs\n  url: https://docs.example.com/mcp\n  headers: [Authorization]\n",
        )
        .unwrap();
        std::fs::write(
            keeper.secrets_path(),
            "author_tokens:\n  work: ghp_keeper\nmcp_headers:\n  docs:\n    Authorization: Bearer abc\n",
        )
        .unwrap();

        let annex =
            Config::read("sandbox_binds:\n- /srv/data\nkept_by:\n  device: a\n  fingerprint: f\n")
                .unwrap();
        let config = annex.working_as(&keeper.config()).text().unwrap();
        let secrets = Secrets::default()
            .working_as(&keeper.secrets())
            .text()
            .unwrap();

        (dir, keeper, config, secrets)
    }

    /// A start whose Annex already says what the Keeper says reads both files
    /// and writes neither.
    #[test]
    fn a_start_with_nothing_changed_writes_neither_file() {
        let (backend, wsl, _powershell) = made();
        let (_dir, keeper, config, secrets) = keeper_and_copied();

        wsl.answers(1, &config, 0).answers(2, &secrets, 0);

        InTheBox(Arc::new(backend))
            .working_as(&keeper.config(), &keeper.secrets())
            .unwrap();

        assert_eq!(
            wsl.calls(),
            [
                inside("root", &["cat", "--", "/var/lib/verkstead/config.yaml"]),
                inside("root", &["cat", "--", "/var/lib/verkstead/secrets.yaml"]),
            ]
        );
    }

    /// And one whose Keeper has changed since writes both, and hands each to
    /// the service.
    #[test]
    fn a_start_after_a_change_on_the_keeper_writes_both_files() {
        let (backend, wsl, _powershell) = made();
        let (_dir, keeper, _config, _secrets) = keeper_and_copied();

        wsl.answers(1, "instructions: Say it at length.\n", 0)
            .answers(4, "author_tokens:\n  work: ghp_mirrored\n", 0);

        InTheBox(Arc::new(backend))
            .working_as(&keeper.config(), &keeper.secrets())
            .unwrap();

        let written = |mode: &str, path: &str| {
            inside("root", &["install", "-D", "-m", mode, "/dev/stdin", path])
        };
        let handed = |path: &str| inside("root", &["chown", "verkstead:verkstead", path]);

        assert_eq!(
            wsl.calls(),
            [
                inside("root", &["cat", "--", "/var/lib/verkstead/config.yaml"]),
                written("640", "/var/lib/verkstead/config.yaml"),
                handed("/var/lib/verkstead/config.yaml"),
                inside("root", &["cat", "--", "/var/lib/verkstead/secrets.yaml"]),
                written("600", "/var/lib/verkstead/secrets.yaml"),
                handed("/var/lib/verkstead/secrets.yaml"),
            ]
        );
    }

    const KEEPER: &str = "keeper";
    const ANNEX: &str = "annex";
    const MEMBER: &str = "laptop";

    /// A Keeper's devices, with its Annex's row and one other member's
    /// seeded — the member at an address nobody answers at — its Annex
    /// Forward pointed at the Annex, the Annex's row carrying a line, and the
    /// pages it tells listened to.
    struct Keeping {
        devices: Devices,
        pool: sqlx::SqlitePool,
        pointing: crate::peer::annex_forward::Pointing,
        told: tokio::sync::broadcast::Receiver<verkstead_schema::Nudged>,
        _dir: tempfile::TempDir,
    }

    async fn keeping() -> Keeping {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        for (device, kept_by) in [(ANNEX, Some(KEEPER)), (MEMBER, None)] {
            verkstead_store::record_member(
                &pool,
                &verkstead_store::Linking {
                    device: device.to_owned(),
                    name: device.to_owned(),
                    os: "Linux".to_owned(),
                    addresses: vec!["127.0.0.1:1".to_owned()],
                    fingerprint: format!("{device}-fingerprint"),
                    kept_by: kept_by.map(str::to_owned),
                },
            )
            .await
            .unwrap();
        }

        let reading = crate::device::reading::Reading::stated(
            crate::remote::Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
            crate::platform::Platform::Windows,
            None,
            Vec::new(),
        );

        let pointing = crate::peer::annex_forward::Pointing::at_nothing();
        pointing.point_at("127.0.0.1:8425");

        let nudges = crate::nudge::Nudges::new();
        let told = nudges.subscribe();

        let devices = Devices::of(
            crate::device::Device::stated(dir.path(), KEEPER).unwrap(),
            reading,
            crate::peer::Members::recorded(pool.clone()),
            crate::peer::joining::Joins::recorded(pool.clone()),
        )
        .waiting(Duration::from_millis(500))
        .forwarding_for_the_annex(pointing.clone())
        .nudging(nudges);

        devices.annex_troubled(Some("The annex didn't answer.".to_owned()));

        Keeping {
            devices,
            pool,
            pointing,
            told,
            _dir: dir,
        }
    }

    /// Whose rows `keeping` holds now.
    async fn held(keeping: &Keeping) -> Vec<String> {
        verkstead_store::members(&keeping.pool)
            .await
            .unwrap()
            .into_iter()
            .map(|member| member.device)
            .collect()
    }

    /// The Keeper's start over `backend`, the release its Annex was pinned to
    /// held from an earlier boot.
    async fn started_over(backend: Wsl, keeping: &Keeping) -> Annexed {
        let (annexed, _dir) = annexed(backend);
        *annexed.pinned.lock().unwrap() = Some("0.1.0".to_owned());

        assert_eq!(annexed.up(&keeping.devices).await, Ok(false));
        annexed
    }

    /// A distro unregistered by hand is forgotten at the Keeper's next start:
    /// its row dropped here, the other member told to drop it — and owed the
    /// telling, being away — and everything held for it let go of.
    #[tokio::test]
    async fn a_keepers_start_with_its_distro_gone_forgets_its_annex() {
        let only_the_humans =
            utf16("  NAME      STATE           VERSION\r\n* Ubuntu    Stopped         2\r\n");

        for (listed, code) in [(only_the_humans, 0), (utf16(NO_DISTROS), -1)] {
            let mut keeping = keeping().await;

            // The marker outlives an unregister: it is the Keeper's own file,
            // beside the disk image `wsl --unregister` deletes.
            let (backend, wsl, _powershell) = made();
            wsl.answers(1, utf16(ANSWERING_STATUS), 0)
                .answers(2, listed, code);

            let annexed = started_over(backend, &keeping).await;

            assert_eq!(held(&keeping).await, [MEMBER], "the Annex's row is gone");
            assert_eq!(
                verkstead_store::announcements_owed(&keeping.pool, ANNEX)
                    .await
                    .unwrap(),
                [MEMBER],
                "and the member that did not answer is owed the telling",
            );

            assert_eq!(keeping.pointing.pointed_at(), None);
            assert_eq!(keeping.devices.annex_trouble(), None);
            assert_eq!(*annexed.pinned.lock().unwrap(), None);
            assert!(
                matches!(
                    keeping.told.try_recv(),
                    Ok(verkstead_schema::Nudged {
                        device: None,
                        moved: verkstead_schema::Nudge::Devices,
                        ..
                    })
                ),
                "and the pages were told the list moved",
            );

            assert_eq!(
                wsl.calls(),
                [strings(&["--status"]), strings(&["--list", "--verbose"])],
                "nothing is booted or run inside",
            );
        }
    }

    /// With WSL itself gone, the same.
    #[tokio::test]
    async fn a_keepers_start_with_wsl_gone_forgets_its_annex() {
        let keeping = keeping().await;

        let powershell = Stub::new();
        powershell.answers(1, FACTS_NO_WSL, 0);
        let missing = powershell.dir.path().join("no-such-wsl.exe");

        started_over(Wsl::at(missing, &powershell.program), &keeping).await;

        assert_eq!(held(&keeping).await, [MEMBER]);
        assert_eq!(keeping.pointing.pointed_at(), None);
    }

    /// A probe that could not be read is not "no box": nothing is forgotten,
    /// and nobody is told anything.
    #[tokio::test]
    async fn a_probe_that_could_not_be_read_forgets_nothing() {
        let mut keeping = keeping().await;

        let powershell = Stub::new();
        let unrunnable = powershell.dir.path().join("wsl.exe");
        std::fs::write(&unrunnable, "not a program").unwrap();

        let annexed = started_over(Wsl::at(unrunnable, &powershell.program), &keeping).await;

        assert_eq!(held(&keeping).await, [ANNEX, MEMBER]);
        assert!(
            verkstead_store::announcements_owed(&keeping.pool, ANNEX)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            keeping.pointing.pointed_at().as_deref(),
            Some("127.0.0.1:8425")
        );
        assert!(keeping.devices.annex_trouble().is_some());
        assert!(annexed.pinned.lock().unwrap().is_some());
        assert!(keeping.told.try_recv().is_err());
    }

    /// A list that failed, or lists the distro without the Keeper's marker,
    /// beside a disk image that is still there is not a box gone: nothing is
    /// forgotten, and nobody is told anything.
    #[tokio::test]
    async fn a_keepers_start_beside_the_boxs_disk_forgets_nothing() {
        let failed = utf16("Error code: Wsl/Service/E_UNEXPECTED\r\n");

        for (listed, code, marked) in [(failed, -1, true), (utf16(&listed("Stopped")), 0, false)] {
            let mut keeping = keeping().await;

            let (backend, wsl, _powershell) = made();
            std::fs::write(backend.disk().join("ext4.vhdx"), vec![0; 4096]).unwrap();
            if !marked {
                std::fs::remove_file(backend.disk().join("made")).unwrap();
            }
            wsl.answers(1, utf16(ANSWERING_STATUS), 0)
                .answers(2, listed, code);

            started_over(backend, &keeping).await;

            assert_eq!(held(&keeping).await, [ANNEX, MEMBER], "marked: {marked}");
            assert!(keeping.devices.annex_trouble().is_some());
            assert!(keeping.told.try_recv().is_err());
        }
    }

    /// A Keeper that never had an Annex has nothing to forget, and tells
    /// nobody so.
    #[tokio::test]
    async fn a_keeper_with_no_annex_and_no_box_forgets_nothing() {
        let mut keeping = keeping().await;
        verkstead_store::forget_member(&keeping.pool, ANNEX)
            .await
            .unwrap();

        let (backend, wsl, _powershell) = made();
        wsl.answers(1, utf16(ANSWERING_STATUS), 0)
            .answers(2, utf16(NO_DISTROS), -1);

        started_over(backend, &keeping).await;

        assert_eq!(held(&keeping).await, [MEMBER]);
        assert!(keeping.told.try_recv().is_err());
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
