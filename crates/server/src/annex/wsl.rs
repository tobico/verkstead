//! The Annex's box as a NixOS-WSL distro, driven through `wsl.exe` — the one
//! [`Backend`] there is.
//!
//! **Everything here is a program run and what it printed.** The pieces that
//! read an answer are functions over text, and the rest is a thin layer that
//! runs `wsl.exe` with `.unseen()` (see [`crate::unseen`]) and hands them what
//! came back. Both programs it runs are named on the value rather than looked
//! up on `PATH`, so a suite points it at a stub the way the install suite
//! points the wizard at a stub package manager.
//!
//! **`wsl.exe` prints UTF-16LE**, with no byte-order mark, and `WSL_UTF8=1` in
//! its environment turns that off on the WSL that reads it — but not on the
//! inbox `wsl.exe` of a machine with no WSL, which is the one saying *not
//! installed* and says it in UTF-16 whatever it is told (`fixtures/` holds what
//! one really printed). So the variable is set and the output is decoded by
//! looking at it. What a command *inside* the distro prints is that command's
//! own bytes and is never decoded here.
//!
//! **Commands inside run under `env` with NixOS's own `PATH`.** `wsl --exec`
//! looks a program up on WSL's default `PATH`, which is a Debian's and holds
//! nothing on NixOS; going through a shell would put the box's profile in
//! between instead. `/run/current-system/sw/bin/env` is on every NixOS, and the
//! `PATH` it is handed is the system's — with `/bin` and `/usr/bin` after it,
//! where WSL's own `wslinfo` is put.
//!
//! **Telling a restart owed from virtualization that is off** is the delicate
//! part, because `wsl.exe` says the same thing for both. So where it does not
//! answer, three facts are read off Windows through PowerShell: whether a
//! hypervisor is running, whether the firmware has virtualization on, and the
//! Virtual Machine Platform feature's state. A running hypervisor reports the
//! firmware's virtualization as *off* — it has taken it — so a machine with
//! Hyper-V on is never read as one with virtualization off; see [`Facts`].

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::Mutex;

use anyhow::{Context, bail};

use super::{
    ANNEX_PEER_PORT, Addresses, BOX_NAME, Backend, Image, Inside, Networking, Probed, Step,
};
use crate::unseen::Unseen;

/// The file in the disk's directory that says the distro was made to the end —
/// see [`Backend::made`]. Beside the disk rather than inside the distro, so the
/// probe that reads it on every poll of the wizard boots nothing.
const MADE: &str = "made";

/// The NixOS-WSL image, at the release the flake pins its module at — see
/// [`super::flake::NIXOS_WSL`].
const RELEASES: &str = "https://github.com/nix-community/NixOS-WSL/releases/download";
const IMAGE: &str = "nixos.wsl";

/// What turns WSL on, each raised in a dialog of its own: the optional
/// features and WSL itself with no distro of anybody's, and then the newest
/// WSL over whatever that put there. The restart after them is the human's.
///
/// `wsl.exe` by name rather than the one a value was pointed at, because these
/// run in an elevated PowerShell on the far side of the dialog, where the
/// machine's own is the only one there is.
const TURNING_ON: &[Step] = &[
    Step {
        line: "wsl.exe --install --no-distribution",
        doing: "Turning on WSL…",
    },
    Step {
        line: "wsl.exe --update",
        doing: "Updating WSL…",
    },
];

/// The `PATH` a command inside the distro is run with — see the module's docs.
const INSIDE_PATH: &str = "PATH=/run/wrappers/bin:/run/current-system/sw/bin:/bin:/usr/bin";

/// The program that puts it there, which every NixOS has at this path.
const INSIDE_ENV: &str = "/run/current-system/sw/bin/env";

/// The NixOS module's service user.
const SERVICE_USER: &str = "verkstead";

/// What `wsl.exe` exits with when there is no WSL behind it — Windows'
/// `ERROR_NOT_SUPPORTED`, which is what the inbox `wsl.exe` really returned.
const NOT_INSTALLED: i32 = 50;

/// The page `wsl.exe` points at when it cannot start a WSL 2 machine for want
/// of the Virtual Machine Platform or of virtualization: the one part of that
/// message that is the same in every language.
const ENABLE_VIRTUALIZATION: &str = "aka.ms/enablevirtualization";

/// What PowerShell is asked for the three facts, one `key=value` to a line.
const FACTS: &str = "$c = Get-CimInstance Win32_ComputerSystem; \
     $p = Get-CimInstance Win32_Processor | Select-Object -First 1; \
     $f = Get-CimInstance Win32_OptionalFeature -Filter \"Name='VirtualMachinePlatform'\"; \
     \"hypervisor=$($c.HypervisorPresent)\"; \
     \"firmware=$($p.VirtualizationFirmwareEnabled)\"; \
     \"platform=$($f.InstallState)\"";

/// The distro, through `wsl.exe`.
#[derive(Debug)]
pub struct Wsl {
    /// The `wsl.exe` it runs.
    wsl: PathBuf,

    /// The PowerShell the three facts are read through.
    powershell: PathBuf,

    /// Where the distro's disk is kept: `%LOCALAPPDATA%\Verkstead\wsl`.
    disk: PathBuf,

    /// Where NixOS-WSL's releases are downloaded from: GitHub, but for a suite.
    releases: String,

    /// What keeps the distro up after [`Backend::boot`]: WSL stops a distro a
    /// few seconds after the last process started through `wsl.exe` ends, so a
    /// booted one is one with a process of the Keeper's held in it.
    held: Mutex<Option<Child>>,
}

impl Wsl {
    /// The machine's own, found on `PATH` the way a shell would.
    pub fn here() -> Wsl {
        Wsl::at("wsl.exe", "powershell.exe")
    }

    /// One running the `wsl` and `powershell` it is told to — a suite's stubs.
    pub fn at(wsl: impl Into<PathBuf>, powershell: impl Into<PathBuf>) -> Wsl {
        let local = std::env::var_os("LOCALAPPDATA").map_or_else(std::env::temp_dir, PathBuf::from);

        Wsl {
            wsl: wsl.into(),
            powershell: powershell.into(),
            disk: local.join("Verkstead").join("wsl"),
            releases: RELEASES.to_owned(),
            held: Mutex::new(None),
        }
    }

    /// The same, keeping the distro's disk in `disk` — a suite's own directory.
    pub fn on(self, disk: impl Into<PathBuf>) -> Wsl {
        Wsl {
            disk: disk.into(),
            ..self
        }
    }

    /// The same, downloading the image's releases from `releases` — a suite's
    /// own server.
    pub fn downloading_from(self, releases: impl Into<String>) -> Wsl {
        Wsl {
            releases: releases.into(),
            ..self
        }
    }

    /// The name of the firewall rule that lets the distro reach `port`.
    fn rule(port: u16) -> String {
        format!("Verkstead-Annex-{port}")
    }

    /// `wsl.exe` with `arguments`, run to its end.
    fn wsl(&self, arguments: &[&str]) -> std::io::Result<Said> {
        let output = self.command(arguments).stdin(Stdio::null()).output()?;

        Ok(Said::of(&output))
    }

    /// `wsl.exe` with `arguments`, as a command not yet started.
    fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(&self.wsl);
        command.args(arguments).env("WSL_UTF8", "1").unseen();
        command
    }

    /// `wsl.exe` with `arguments`, which has to succeed.
    fn must(&self, arguments: &[&str]) -> anyhow::Result<()> {
        let said = self
            .wsl(arguments)
            .with_context(|| format!("Verkstead couldn't run {}", self.wsl.display()))?;

        if !said.success {
            bail!("{}", said.why());
        }

        Ok(())
    }

    /// The arguments that run `argv` inside the distro as `who`.
    fn inside<'a>(who: Inside, argv: &[&'a str]) -> Vec<&'a str> {
        let user = match who {
            Inside::Root => "root",
            Inside::Service => SERVICE_USER,
        };

        let mut arguments = vec![
            "-d",
            BOX_NAME,
            "-u",
            user,
            "--exec",
            INSIDE_ENV,
            INSIDE_PATH,
        ];
        arguments.extend_from_slice(argv);
        arguments
    }

    /// The three facts, or none of them where PowerShell would not say.
    fn facts(&self) -> Facts {
        let told = Command::new(&self.powershell)
            .args(["-NoProfile", "-NonInteractive", "-Command", FACTS])
            .unseen()
            .stdin(Stdio::null())
            .output();

        match told {
            Ok(told) => Facts::read(&decoded(&told.stdout)),
            Err(_) => Facts::default(),
        }
    }

    /// The networking mode, from `wslinfo` inside the distro.
    ///
    /// A `wslinfo` that does not know the question is a WSL older than mirrored
    /// networking, which is NAT.
    fn networking(&self) -> Networking {
        match self.run(Inside::Root, &["wslinfo", "--networking-mode"]) {
            Ok(ran) if ran.status.success() => networking(&String::from_utf8_lossy(&ran.stdout)),
            _ => Networking::Nat,
        }
    }

    /// What one command inside printed, which has to have succeeded.
    fn inside_text(&self, argv: &[&str]) -> anyhow::Result<String> {
        let ran = self.run(Inside::Root, argv)?;

        if !ran.status.success() {
            bail!(
                "{} said: {}",
                argv.join(" "),
                String::from_utf8_lossy(&ran.stderr).trim()
            );
        }

        Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
    }
}

impl Backend for Wsl {
    fn probe(&self) -> Probed {
        let status = self.wsl(&["--status"]);

        let answering = matches!(
            &status,
            Ok(said) if said.success && !said.text.contains(ENABLE_VIRTUALIZATION)
        );

        if !answering {
            let installed = match &status {
                Ok(said) => {
                    said.code != Some(NOT_INSTALLED) && !said.text.contains("is not installed")
                }
                Err(_) => false,
            };

            return unanswered(installed, self.facts());
        }

        let listed = match self.wsl(&["--list", "--verbose"]) {
            Ok(said) if said.success => said.text,
            _ => return Probed::NoDistro,
        };

        match listed_state(&listed) {
            Some(running) if self.disk.join(MADE).is_file() => Probed::Present {
                running,
                networking: self.networking(),
            },
            Some(_) | None => Probed::NoDistro,
        }
    }

    fn turning_on(&self) -> &'static [Step] {
        TURNING_ON
    }

    fn image(&self) -> Image {
        let release = format!("{}/{}", self.releases, super::flake::NIXOS_WSL);

        Image {
            url: format!("{release}/{IMAGE}"),
            checksum: format!("{release}/{IMAGE}.sha256"),
            file: IMAGE,
            name: "NixOS-WSL",
        }
    }

    fn disk(&self) -> &Path {
        &self.disk
    }

    fn make(&self, image: &Path) -> anyhow::Result<()> {
        let listed = self.wsl(&["--list", "--verbose"]);

        if matches!(&listed, Ok(said) if listed_state(&said.text).is_some()) {
            return Ok(());
        }

        std::fs::create_dir_all(&self.disk)
            .with_context(|| format!("Verkstead couldn't make {}", self.disk.display()))?;

        let disk = self.disk.to_string_lossy();
        let image = image.to_string_lossy();

        self.must(&["--import", BOX_NAME, &disk, &image, "--version", "2"])
    }

    fn made(&self) -> anyhow::Result<()> {
        let made = self.disk.join(MADE);

        std::fs::write(&made, super::flake::VERSION)
            .with_context(|| format!("Verkstead couldn't write {}", made.display()))
    }

    fn opening(&self, port: u16) -> Option<String> {
        if self.networking() == Networking::Mirrored {
            return None;
        }

        let rule = Self::rule(port);

        let asked = Command::new(&self.powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "if (Get-NetFirewallRule -Name '{rule}' -ErrorAction SilentlyContinue) \
                     {{ 'present' }}"
                ),
            ])
            .unseen()
            .stdin(Stdio::null())
            .output();

        if matches!(&asked, Ok(asked) if decoded(&asked.stdout).trim() == "present") {
            return None;
        }

        // The adapter by its prefix: it is `vEthernet (WSL)` on one WSL and
        // `vEthernet (WSL (Hyper-V firewall))` on a newer one.
        Some(format!(
            "New-NetFirewallRule -Name '{rule}' -DisplayName 'Verkstead Annex' \
             -Direction Inbound -Action Allow -Protocol TCP -LocalPort {port} \
             -InterfaceAlias 'vEthernet (WSL*' | Out-Null"
        ))
    }

    fn boot(&self) -> anyhow::Result<()> {
        let mut held = self
            .held
            .lock()
            .expect("the held process is never poisoned");

        if let Some(child) = held.as_mut()
            && matches!(child.try_wait(), Ok(None))
        {
            return Ok(());
        }

        let child = self
            .command(&Self::inside(Inside::Root, &["sleep", "infinity"]))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Verkstead couldn't run {}", self.wsl.display()))?;

        *held = Some(child);
        Ok(())
    }

    fn terminate(&self) -> anyhow::Result<()> {
        if let Some(mut child) = self
            .held
            .lock()
            .expect("the held process is never poisoned")
            .take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }

        self.must(&["--terminate", BOX_NAME])
    }

    fn run(&self, who: Inside, argv: &[&str]) -> anyhow::Result<Output> {
        self.command(&Self::inside(who, argv))
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("Verkstead couldn't run {}", self.wsl.display()))
    }

    fn follow(
        &self,
        who: Inside,
        argv: &[&str],
        line: &mut dyn FnMut(&str),
    ) -> anyhow::Result<ExitStatus> {
        let mut child = self
            .command(&Self::inside(who, argv))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Verkstead couldn't run {}", self.wsl.display()))?;

        // Both streams onto one channel, a thread reading each, so a line is
        // handed on in the order the two were printed in near enough.
        let (lines, said) = std::sync::mpsc::channel::<String>();

        let readers: Vec<_> = [
            child
                .stdout
                .take()
                .map(|out| Box::new(out) as Box<dyn Read + Send>),
            child
                .stderr
                .take()
                .map(|err| Box::new(err) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        .map(|stream| {
            let lines = lines.clone();

            std::thread::spawn(move || {
                for read in BufReader::new(stream).split(b'\n') {
                    let Ok(read) = read else { break };
                    let _ = lines.send(String::from_utf8_lossy(&read).trim_end().to_owned());
                }
            })
        })
        .collect();

        drop(lines);

        for said in said {
            line(&said);
        }

        for reader in readers {
            let _ = reader.join();
        }

        child
            .wait()
            .with_context(|| format!("Verkstead couldn't run {}", self.wsl.display()))
    }

    fn write(&self, path: &str, bytes: &[u8], mode: u32) -> anyhow::Result<()> {
        let mode = format!("{mode:o}");

        let mut child = self
            .command(&Self::inside(
                Inside::Root,
                &["install", "-D", "-m", &mode, "/dev/stdin", path],
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Verkstead couldn't run {}", self.wsl.display()))?;

        // From a thread, and whatever it comes to: a program that stopped
        // reading has said why on its standard error, and its status is the
        // verdict rather than the broken pipe this would see.
        let mut stdin = child.stdin.take().expect("stdin is piped");
        let bytes = bytes.to_vec();
        let feeding = std::thread::spawn(move || {
            let _ = stdin.write_all(&bytes);
        });

        let ran = child
            .wait_with_output()
            .with_context(|| format!("Verkstead couldn't run {}", self.wsl.display()))?;
        let _ = feeding.join();

        if !ran.status.success() {
            bail!(
                "Verkstead couldn't write {path} in the {BOX_NAME} distro: {}",
                Said::of(&ran).why()
            );
        }

        Ok(())
    }

    fn read(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let ran = self.run(Inside::Root, &["cat", "--", path])?;

        if !ran.status.success() {
            bail!(
                "Verkstead couldn't read {path} in the {BOX_NAME} distro: {}",
                String::from_utf8_lossy(&ran.stderr).trim()
            );
        }

        Ok(ran.stdout)
    }

    fn addresses(&self, networking: Networking, keeper_port: u16) -> anyhow::Result<Addresses> {
        let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);

        let (annex, keeper) = match networking {
            Networking::Mirrored => (loopback, loopback),
            Networking::Nat => {
                let shown = self.inside_text(&["ip", "-4", "-o", "addr", "show", "dev", "eth0"])?;
                let annex = interface_address(&shown)
                    .with_context(|| format!("eth0 has no IPv4 address: {}", shown.trim()))?;

                let routes = self.inside_text(&["ip", "route", "show", "default"])?;
                let gateway = default_gateway(&routes)
                    .with_context(|| format!("there is no default route: {}", routes.trim()))?;

                (annex, gateway)
            }
        };

        Ok(Addresses {
            annex: SocketAddr::new(annex, ANNEX_PEER_PORT),
            keeper_at: SocketAddr::new(keeper, keeper_port),
        })
    }
}

/// What `wsl.exe` said about one call: its two streams decoded into one text,
/// and how it ended.
#[derive(Debug)]
struct Said {
    success: bool,
    code: Option<i32>,
    text: String,
}

impl Said {
    fn of(output: &Output) -> Said {
        let mut text = decoded(&output.stdout);
        text.push_str(&decoded(&output.stderr));

        Said {
            success: output.status.success(),
            code: output.status.code(),
            text,
        }
    }

    /// The first thing it said, as the reason a call failed.
    fn why(&self) -> String {
        self.text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("wsl.exe said nothing, and ended with {:?}.", self.code))
    }
}

/// What a machine whose WSL is not answering is: `installed` says whether a
/// `wsl.exe` is there that knows it is WSL rather than the inbox one saying it
/// is not installed.
fn unanswered(installed: bool, facts: Facts) -> Probed {
    if facts.virtualization_off() {
        return Probed::VirtualizationOff;
    }

    // The feature turned on with no hypervisor running yet is a restart owed;
    // with one running, the restart has happened and WSL is still not there.
    if !installed && !(facts.platform == Some(Feature::Enabled) && !facts.hypervisor) {
        return Probed::NoWsl;
    }

    Probed::NeedsRestart
}

/// The three facts about Windows the probe reads where WSL is not answering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Facts {
    /// A hypervisor is running — Hyper-V, or WSL 2's own once it is up.
    hypervisor: bool,

    /// What the processor says of the firmware's virtualization, which is
    /// `false` under a running hypervisor whatever the firmware says.
    firmware: Option<bool>,

    /// The Virtual Machine Platform optional feature.
    platform: Option<Feature>,
}

/// An optional feature's `InstallState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Feature {
    Enabled,
    Disabled,
    Absent,
}

impl Facts {
    /// Off in the firmware, which only a machine with no hypervisor running can
    /// say truthfully.
    fn virtualization_off(&self) -> bool {
        !self.hypervisor && self.firmware == Some(false)
    }

    fn read(text: &str) -> Facts {
        let mut facts = Facts::default();

        for line in text.lines() {
            let Some((key, value)) = line.trim().split_once('=') else {
                continue;
            };

            let truth = match value.trim() {
                "True" => Some(true),
                "False" => Some(false),
                _ => None,
            };

            match key.trim() {
                "hypervisor" => facts.hypervisor = truth == Some(true),
                "firmware" => facts.firmware = truth,
                "platform" => {
                    facts.platform = match value.trim() {
                        "1" => Some(Feature::Enabled),
                        "2" => Some(Feature::Disabled),
                        "3" => Some(Feature::Absent),
                        _ => None,
                    }
                }
                _ => {}
            }
        }

        facts
    }
}

/// Whether `wsl --list --verbose` lists the distro, and if so whether it is
/// running.
fn listed_state(listed: &str) -> Option<bool> {
    listed.lines().skip(1).find_map(|line| {
        let mut columns = line.trim().trim_start_matches('*').split_whitespace();
        let name = columns.next()?;
        let state = columns.next()?;

        name.eq_ignore_ascii_case(BOX_NAME)
            .then(|| state.eq_ignore_ascii_case("Running"))
    })
}

/// The mode `wslinfo --networking-mode` names, NAT for anything but mirrored.
fn networking(said: &str) -> Networking {
    if said.trim().eq_ignore_ascii_case("mirrored") {
        Networking::Mirrored
    } else {
        Networking::Nat
    }
}

/// The address `ip -4 -o addr show` gives an interface.
fn interface_address(shown: &str) -> Option<IpAddr> {
    let mut words = shown.split_whitespace();
    words.find(|word| *word == "inet")?;

    words.next()?.split('/').next()?.parse().ok()
}

/// The gateway `ip route show default` names.
fn default_gateway(routes: &str) -> Option<IpAddr> {
    let mut words = routes.split_whitespace();
    words.find(|word| *word == "via")?;

    words.next()?.parse().ok()
}

/// What `wsl.exe` printed, as text — UTF-16LE where it looks like it, UTF-8
/// otherwise.
///
/// UTF-16 is told by its zeros: text in it that is mostly ASCII, which all of
/// `wsl.exe`'s is, has a zero for every second byte, where UTF-8 has none.
fn decoded(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);

    let zeros = bytes
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|byte| **byte == 0)
        .count();
    let text = if bytes.len() >= 2 && zeros * 2 >= bytes.len() / 2 {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();

        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };

    text.replace(['\r', '\0'], "")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// What the inbox `wsl.exe` of a Windows 11 with no WSL really printed for
    /// `--status`, on its standard error, ending with 50: UTF-16LE with no
    /// byte-order mark, `WSL_UTF8=1` in its environment or not.
    const NOT_INSTALLED_STATUS: &[u8] = include_bytes!("fixtures/not-installed.utf16");

    /// What the facts are on that same machine, which is a VM with a
    /// hypervisor under it: virtualization on, and the platform not.
    const FACTS_NO_WSL: &str = "hypervisor=True\r\nfirmware=True\r\nplatform=2\r\n";

    /// What a WSL that is installed and cannot start a machine says for
    /// `--status`, after `wsl --install --no-distribution` and before the
    /// restart, or with virtualization off. Written from WSL's own message
    /// rather than captured: no machine here has been in either state yet.
    const CANNOT_START_STATUS: &str = "Default Version: 2\r\n\r\n\
        WSL2 is not supported with your current machine configuration.\r\n\
        Please enable the \"Virtual Machine Platform\" optional component and ensure \
        virtualization is enabled in the BIOS.\r\n\
        Enable \"Virtual Machine Platform\" by running: wsl.exe --install --no-distribution\r\n\
        For information please visit https://aka.ms/enablevirtualization\r\n";

    /// What a WSL that answers says for `--status`, as WSL 3.0.3 on that same
    /// machine really printed it once it was on: the WSL 1 lines are about the
    /// optional component `--install --no-distribution` leaves off.
    const ANSWERING_STATUS: &str = "Default Version: 2\r\n\
        WSL1 is not supported with your current machine configuration.\r\n\
        Please enable the \"Windows Subsystem for Linux\" optional component to use WSL1.\r\n";

    /// What it lists with no distro at all, ending with -1.
    pub(crate) const NO_DISTROS: &str = "Windows Subsystem for Linux has no installed distributions.\r\n\
        You can resolve this by installing a distribution with the instructions below:\r\n";

    /// What it lists with the human's own distro beside Verkstead's, in the
    /// layout WSL 3.0.3 really lists in.
    fn listed(state: &str) -> String {
        format!(
            "  NAME         STATE           VERSION\r\n\
             * Ubuntu       Stopped         2\r\n  \
             Verkstead    {state:<15} 2\r\n"
        )
    }

    /// A program the backend is pointed at in place of `wsl.exe` or
    /// PowerShell, answering its calls in the order they come: the `n`th call
    /// prints `out<n>` and exits with `code<n>`, and every call's arguments are
    /// written down.
    ///
    /// **A shell script on Unix and a batch file on Windows**, so the suite runs
    /// on both: it is a program run by name either way, which is what the
    /// backend does with the real one.
    pub(crate) struct Stub {
        pub(crate) dir: tempfile::TempDir,
        pub(crate) program: PathBuf,
    }

    impl Stub {
        pub(crate) fn new() -> Stub {
            let dir = tempfile::tempdir().unwrap();

            #[cfg(unix)]
            let program = {
                use std::os::unix::fs::PermissionsExt;

                let program = dir.path().join("stub");
                std::fs::write(
                    &program,
                    "#!/bin/sh\n\
                     here=$(dirname \"$0\")\n\
                     n=$(( $(cat \"$here/count\" 2>/dev/null || echo 0) + 1 ))\n\
                     echo \"$n\" > \"$here/count\"\n\
                     for a in \"$@\"; do printf '%s\\037' \"$a\"; done >> \"$here/calls\"\n\
                     printf '\\n' >> \"$here/calls\"\n\
                     cat \"$here/out$n\" 2>/dev/null\n\
                     exit \"$(cat \"$here/code$n\" 2>/dev/null || echo 0)\"\n",
                )
                .unwrap();
                std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
                program
            };

            #[cfg(windows)]
            let program = {
                let program = dir.path().join("stub.cmd");
                std::fs::write(
                    &program,
                    "@echo off\r\n\
                     setlocal\r\n\
                     set n=0\r\n\
                     if exist \"%~dp0count\" set /p n=<\"%~dp0count\"\r\n\
                     set /a n+=1\r\n\
                     >\"%~dp0count\" echo %n%\r\n\
                     >>\"%~dp0calls\" echo(%*\r\n\
                     if exist \"%~dp0out%n%\" type \"%~dp0out%n%\"\r\n\
                     set code=0\r\n\
                     if exist \"%~dp0code%n%\" set /p code=<\"%~dp0code%n%\"\r\n\
                     exit /b %code%\r\n",
                )
                .unwrap();
                program
            };

            Stub { dir, program }
        }

        /// The `n`th call prints `out` and ends with `code`.
        pub(crate) fn answers(&self, n: usize, out: impl AsRef<[u8]>, code: i32) -> &Stub {
            std::fs::write(self.dir.path().join(format!("out{n}")), out).unwrap();
            std::fs::write(self.dir.path().join(format!("code{n}")), code.to_string()).unwrap();
            self
        }

        /// Every call's arguments, in order.
        pub(crate) fn calls(&self) -> Vec<Vec<String>> {
            let written =
                std::fs::read_to_string(self.dir.path().join("calls")).unwrap_or_default();

            written
                .lines()
                .map(|line| {
                    if cfg!(windows) {
                        split_command_line(line)
                    } else {
                        line.split('\u{1f}')
                            .filter(|argument| !argument.is_empty())
                            .map(str::to_owned)
                            .collect()
                    }
                })
                .collect()
        }
    }

    /// A batch file's `%*` back into its arguments: split on spaces outside
    /// quotes, a quote doubled inside them being a quote.
    fn split_command_line(line: &str) -> Vec<String> {
        let mut arguments = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        let mut any = false;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    chars.next();
                    current.push('"');
                }
                '"' => {
                    quoted = !quoted;
                    any = true;
                }
                ' ' if !quoted => {
                    if any || !current.is_empty() {
                        arguments.push(std::mem::take(&mut current));
                    }
                    any = false;
                }
                c => current.push(c),
            }
        }

        if any || !current.is_empty() {
            arguments.push(current);
        }

        arguments
    }

    /// `text` as `wsl.exe` prints it by default.
    pub(crate) fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    /// A backend over a stub `wsl` and a stub PowerShell, its disk in the
    /// `wsl` stub's directory.
    fn stubbed() -> (Wsl, Stub, Stub) {
        let wsl = Stub::new();
        let powershell = Stub::new();

        let backend = Wsl::at(&wsl.program, &powershell.program).on(wsl.dir.path().join("disk"));

        (backend, wsl, powershell)
    }

    /// The same, with a distro that was made to the end.
    fn made() -> (Wsl, Stub, Stub) {
        let (backend, wsl, powershell) = stubbed();

        std::fs::create_dir_all(backend.disk()).unwrap();
        backend.made().unwrap();

        (backend, wsl, powershell)
    }

    /// The arguments a command inside is run with, after the user.
    pub(crate) fn inside(user: &str, argv: &[&str]) -> Vec<String> {
        let mut arguments = vec![
            "-d",
            "Verkstead",
            "-u",
            user,
            "--exec",
            INSIDE_ENV,
            INSIDE_PATH,
        ];
        arguments.extend_from_slice(argv);
        arguments.into_iter().map(str::to_owned).collect()
    }

    pub(crate) fn strings(arguments: &[&str]) -> Vec<String> {
        arguments
            .iter()
            .map(|argument| argument.to_string())
            .collect()
    }

    #[test]
    fn a_machine_with_no_wsl_exe_has_no_wsl() {
        let powershell = Stub::new();
        powershell.answers(1, FACTS_NO_WSL, 0);

        let missing = powershell.dir.path().join("no-such-wsl.exe");
        let backend = Wsl::at(missing, &powershell.program);

        assert_eq!(backend.probe(), Probed::NoWsl);
    }

    #[test]
    fn the_inbox_wsl_saying_not_installed_is_no_wsl() {
        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, NOT_INSTALLED_STATUS, 50);
        powershell.answers(1, FACTS_NO_WSL, 0);

        assert_eq!(backend.probe(), Probed::NoWsl);
        assert_eq!(wsl.calls(), vec![strings(&["--status"])]);
    }

    #[test]
    fn no_wsl_with_virtualization_off_in_the_firmware_says_so() {
        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, NOT_INSTALLED_STATUS, 50);
        powershell.answers(1, "hypervisor=False\r\nfirmware=False\r\nplatform=2\r\n", 0);

        assert_eq!(backend.probe(), Probed::VirtualizationOff);
    }

    #[test]
    fn the_platform_turned_on_with_no_hypervisor_yet_is_a_restart_owed() {
        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, NOT_INSTALLED_STATUS, 50);
        powershell.answers(1, "hypervisor=False\r\nfirmware=True\r\nplatform=1\r\n", 0);

        assert_eq!(backend.probe(), Probed::NeedsRestart);
    }

    #[test]
    fn a_wsl_that_cannot_start_a_machine_yet_needs_a_restart() {
        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, utf16(CANNOT_START_STATUS), 0);
        powershell.answers(1, "hypervisor=False\r\nfirmware=True\r\nplatform=2\r\n", 0);

        assert_eq!(backend.probe(), Probed::NeedsRestart);
    }

    #[test]
    fn a_wsl_that_cannot_start_a_machine_with_virtualization_off_says_so() {
        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, utf16(CANNOT_START_STATUS), 0);
        powershell.answers(1, "hypervisor=False\r\nfirmware=False\r\nplatform=1\r\n", 0);

        assert_eq!(backend.probe(), Probed::VirtualizationOff);
    }

    #[test]
    fn a_running_hypervisor_is_never_read_as_virtualization_off() {
        // Hyper-V on: the processor reports the firmware's virtualization as off
        // because the hypervisor has it.
        let hyper_v = "hypervisor=True\r\nfirmware=False\r\nplatform=1\r\n";

        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, utf16(ANSWERING_STATUS), 0)
            .answers(2, utf16(NO_DISTROS), -1);
        powershell.answers(1, hyper_v, 0);

        assert_eq!(backend.probe(), Probed::NoDistro);
        assert!(
            powershell.calls().is_empty(),
            "a WSL that answers is not asked about"
        );

        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, utf16(CANNOT_START_STATUS), 0);
        powershell.answers(1, hyper_v, 0);

        assert_eq!(backend.probe(), Probed::NeedsRestart);
    }

    #[test]
    fn a_wsl_that_answers_with_no_distro_of_verksteads_has_none() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, utf16(ANSWERING_STATUS), 0).answers(
            2,
            utf16("  NAME      STATE           VERSION\r\n* Ubuntu    Running         2\r\n"),
            0,
        );

        assert_eq!(backend.probe(), Probed::NoDistro);
        assert_eq!(
            wsl.calls(),
            vec![strings(&["--status"]), strings(&["--list", "--verbose"])]
        );
    }

    /// The distro listed in `state`, with `wslinfo` inside saying `mode`.
    fn present(state: &str, mode: &str) -> (Probed, Vec<Vec<String>>) {
        let (backend, wsl, _powershell) = made();
        wsl.answers(1, utf16(ANSWERING_STATUS), 0)
            .answers(2, utf16(&listed(state)), 0)
            .answers(3, format!("{mode}\n"), 0);

        (backend.probe(), wsl.calls())
    }

    #[test]
    fn the_distro_is_present_stopped_or_running_under_nat_and_mirrored() {
        for (state, running) in [("Stopped", false), ("Running", true)] {
            for (mode, networking) in [("nat", Networking::Nat), ("mirrored", Networking::Mirrored)]
            {
                let (probed, calls) = present(state, mode);

                assert_eq!(
                    probed,
                    Probed::Present {
                        running,
                        networking
                    },
                    "{state} under {mode}"
                );
                assert_eq!(calls[2], inside("root", &["wslinfo", "--networking-mode"]));
            }
        }
    }

    #[test]
    fn a_wslinfo_that_does_not_know_the_question_is_nat() {
        let (backend, wsl, _powershell) = made();
        wsl.answers(1, utf16(ANSWERING_STATUS), 0)
            .answers(2, utf16(&listed("Running")), 0)
            .answers(3, "wslinfo: unknown option\n", 1);

        assert_eq!(
            backend.probe(),
            Probed::Present {
                running: true,
                networking: Networking::Nat
            }
        );
    }

    #[test]
    fn a_list_in_utf8_reads_as_one_in_utf16_does() {
        let (backend, wsl, _powershell) = made();
        wsl.answers(1, ANSWERING_STATUS, 0)
            .answers(2, listed("Running"), 0)
            .answers(3, "mirrored\n", 0);

        assert_eq!(
            backend.probe(),
            Probed::Present {
                running: true,
                networking: Networking::Mirrored
            }
        );
    }

    #[test]
    fn under_nat_the_addresses_are_eth0s_and_the_default_gateway() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(
            1,
            "2: eth0    inet 172.24.80.2/20 brd 172.24.95.255 scope global eth0\\       \
             valid_lft forever preferred_lft forever\n",
            0,
        )
        .answers(2, "default via 172.24.80.1 dev eth0 proto kernel \n", 0);

        let addresses = backend.addresses(Networking::Nat, 8423).unwrap();

        assert_eq!(addresses.annex, "172.24.80.2:8425".parse().unwrap());
        assert_eq!(addresses.keeper_at, "172.24.80.1:8423".parse().unwrap());
        assert_eq!(
            wsl.calls(),
            vec![
                inside("root", &["ip", "-4", "-o", "addr", "show", "dev", "eth0"]),
                inside("root", &["ip", "route", "show", "default"]),
            ]
        );
    }

    #[test]
    fn under_mirrored_both_addresses_are_the_loopback() {
        let (backend, wsl, _powershell) = stubbed();

        let addresses = backend.addresses(Networking::Mirrored, 9423).unwrap();

        assert_eq!(addresses.annex, "127.0.0.1:8425".parse().unwrap());
        assert_eq!(addresses.keeper_at, "127.0.0.1:9423".parse().unwrap());
        assert!(wsl.calls().is_empty(), "nothing inside is asked");
    }

    #[test]
    fn an_eth0_with_no_address_is_refused_with_what_it_said() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, "", 0);

        let refused = backend.addresses(Networking::Nat, 8423).unwrap_err();

        assert!(refused.to_string().contains("eth0"), "{refused}");
    }

    #[test]
    fn make_imports_the_image_with_its_disk_in_the_directory() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, utf16(NO_DISTROS), -1).answers(
            2,
            utf16("The operation completed successfully.\r\n"),
            0,
        );

        let disk = backend.disk().to_owned();
        let image = wsl.dir.path().join("nixos.wsl");
        backend.make(&image).unwrap();

        assert!(disk.is_dir(), "the disk's directory is made");
        assert_eq!(
            wsl.calls(),
            vec![
                strings(&["--list", "--verbose"]),
                strings(&[
                    "--import",
                    "Verkstead",
                    &disk.to_string_lossy(),
                    &image.to_string_lossy(),
                    "--version",
                    "2",
                ])
            ]
        );
    }

    /// A distro already there is one a run stopped part way through imported,
    /// and making it again is leaving it as it is.
    #[test]
    fn make_leaves_a_distro_that_is_already_there() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, utf16(&listed("Stopped")), 0);

        backend.make(&wsl.dir.path().join("nixos.wsl")).unwrap();

        assert_eq!(wsl.calls(), vec![strings(&["--list", "--verbose"])]);
    }

    /// And one that is there but was never made to the end is no Annex yet,
    /// read without booting it.
    #[test]
    fn a_distro_never_made_to_the_end_is_none() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, utf16(ANSWERING_STATUS), 0)
            .answers(2, utf16(&listed("Stopped")), 0);

        assert_eq!(backend.probe(), Probed::NoDistro);
        assert_eq!(wsl.calls().len(), 2, "nothing is run inside");

        std::fs::create_dir_all(backend.disk()).unwrap();
        backend.made().unwrap();

        assert_eq!(
            std::fs::read_to_string(backend.disk().join(MADE)).unwrap(),
            super::super::flake::VERSION
        );
    }

    #[test]
    fn the_image_is_nixos_wsl_at_the_pinned_release() {
        let image = Wsl::here().image();

        assert_eq!(
            image.url,
            format!(
                "https://github.com/nix-community/NixOS-WSL/releases/download/{}/nixos.wsl",
                super::super::flake::NIXOS_WSL
            )
        );
        assert_eq!(image.checksum, format!("{}.sha256", image.url));
        assert_eq!(image.file, "nixos.wsl");
    }

    /// The firewall is opened under NAT where the rule is not there yet, and
    /// left alone under mirrored networking or where it is.
    #[test]
    fn the_firewall_is_opened_once_and_only_under_nat() {
        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, "nat\n", 0);

        let opening = backend.opening(8423).expect("a rule to add");

        assert!(
            opening.starts_with("New-NetFirewallRule -Name 'Verkstead-Annex-8423' "),
            "{opening}"
        );
        assert!(opening.contains("-LocalPort 8423"), "{opening}");
        assert!(
            opening.contains("-InterfaceAlias 'vEthernet (WSL*'"),
            "{opening}"
        );
        assert_eq!(
            wsl.calls(),
            vec![inside("root", &["wslinfo", "--networking-mode"])]
        );
        assert!(
            powershell.calls()[0]
                .iter()
                .any(|argument| argument
                    .contains("Get-NetFirewallRule -Name 'Verkstead-Annex-8423'")),
            "{:?}",
            powershell.calls()
        );

        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, "nat\n", 0);
        powershell.answers(1, "present\r\n", 0);

        assert_eq!(backend.opening(8423), None, "the rule is there");

        let (backend, wsl, powershell) = stubbed();
        wsl.answers(1, "mirrored\n", 0);

        assert_eq!(backend.opening(8423), None, "mirrored has no adapter");
        assert!(powershell.calls().is_empty(), "the firewall is not asked");
    }

    #[test]
    fn write_installs_what_it_is_handed_as_root() {
        let (backend, wsl, _powershell) = stubbed();

        backend
            .write(
                "/var/lib/verkstead/secrets.yaml",
                b"github_token: x\n",
                0o600,
            )
            .unwrap();

        assert_eq!(
            wsl.calls(),
            vec![inside(
                "root",
                &[
                    "install",
                    "-D",
                    "-m",
                    "600",
                    "/dev/stdin",
                    "/var/lib/verkstead/secrets.yaml"
                ]
            )]
        );
    }

    #[test]
    fn a_refused_write_says_what_it_was_told() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(
            1,
            "install: cannot create directory: Read-only file system\n",
            1,
        );

        let refused = backend
            .write("/etc/nixos/flake.nix", b"{ }", 0o644)
            .unwrap_err();

        assert!(
            refused.to_string().contains("Read-only file system"),
            "{refused}"
        );
    }

    #[test]
    fn follow_hands_on_every_line_and_ends_with_the_status() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, "building the system configuration...\nactivating\n", 4);

        let mut lines = Vec::new();
        let status = backend
            .follow(Inside::Root, &["nixos-rebuild", "switch"], &mut |line| {
                lines.push(line.to_owned())
            })
            .unwrap();

        assert_eq!(status.code(), Some(4));
        assert_eq!(
            lines,
            ["building the system configuration...", "activating"]
        );
        assert_eq!(
            wsl.calls(),
            vec![inside("root", &["nixos-rebuild", "switch"])]
        );
    }

    #[test]
    fn a_refused_import_says_what_wsl_said() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, utf16(NO_DISTROS), -1).answers(
            2,
            utf16("A distribution with the supplied name already exists.\r\n"),
            -1,
        );

        let refused = backend.make(&wsl.dir.path().join("image")).unwrap_err();

        assert_eq!(
            refused.to_string(),
            "A distribution with the supplied name already exists."
        );
    }

    #[test]
    fn boot_holds_a_process_inside_and_terminate_stops_the_distro() {
        let (backend, wsl, _powershell) = stubbed();

        backend.boot().unwrap();

        // The held process is the stub, which ends at once: wait for its call
        // to be written before the terminate's.
        let deadline = Instant::now() + Duration::from_secs(10);
        while wsl.calls().is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }

        backend.terminate().unwrap();

        assert_eq!(
            wsl.calls(),
            vec![
                inside("root", &["sleep", "infinity"]),
                strings(&["--terminate", "Verkstead"]),
            ]
        );
    }

    #[test]
    fn run_inside_as_root_and_as_the_service_user() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, "uid=0(root)\n", 0)
            .answers(2, "uid=999(verkstead)\n", 3);

        let root = backend.run(Inside::Root, &["id"]).unwrap();
        let service = backend.run(Inside::Service, &["id"]).unwrap();

        assert_eq!(root.stdout, b"uid=0(root)\n");
        assert!(root.status.success());
        assert_eq!(service.stdout, b"uid=999(verkstead)\n");
        assert_eq!(service.status.code(), Some(3));
        assert_eq!(
            wsl.calls(),
            vec![inside("root", &["id"]), inside("verkstead", &["id"])]
        );
    }

    #[test]
    fn read_is_the_files_bytes_untouched() {
        let (backend, wsl, _powershell) = stubbed();
        wsl.answers(1, "ada-box\n", 0);

        let read = backend.read("/etc/hostname").unwrap();

        assert_eq!(read, b"ada-box\n");
        assert_eq!(
            wsl.calls(),
            vec![inside("root", &["cat", "--", "/etc/hostname"])]
        );
    }

    #[test]
    fn utf16_and_utf8_both_decode() {
        assert_eq!(
            decoded(NOT_INSTALLED_STATUS).lines().next(),
            Some(
                "The Windows Subsystem for Linux is not installed. You can install by running \
                 'wsl.exe --install'."
            )
        );
        assert_eq!(
            decoded(&utf16("Default Version: 2\r\n")),
            "Default Version: 2\n"
        );
        assert_eq!(decoded(b"Default Version: 2\r\n"), "Default Version: 2\n");

        let mut marked = vec![0xFF, 0xFE];
        marked.extend(utf16("Verkstead"));
        assert_eq!(decoded(&marked), "Verkstead");

        assert_eq!(decoded("Grüße\n".as_bytes()), "Grüße\n");
    }
}
