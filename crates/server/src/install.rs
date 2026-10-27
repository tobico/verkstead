//! What this Verkstead was installed as, read off its own executable at startup.
//!
//! **What says whether it can update itself** (ADR-0027, *A device updates
//! itself*): the desktop app's three artifacts are what electron-updater can
//! take onto a newer release, and what nix, a checkout or a downloaded binary
//! put on disk is replaced the way it was put there, by the human. So this
//! device's row on **Devices** is offered an Update on the first three and a
//! line saying how to update by hand on the rest, and nothing is ever attempted
//! on those.
//!
//! **Read rather than told.** Nothing configures it and the app passes nothing
//! for it: the executable is where it is, the `--desktop` flag says who started
//! it, and an AppImage's runtime leaves `APPDIR` in the environment the app
//! hands its sidecar (`desktop/src/unmounted.ts`). Those three are the whole of
//! the reading, which is why [`InstallKind::of`] is a function of them and
//! nothing else — every arm is a unit test on every platform's paths.

use std::path::Path;

use verkstead_render::InstallKind;

use crate::StartedBy;
use crate::platform::Platform;

/// What a start is read as an install from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Started<'a> {
    /// Which platform's paths these are.
    pub platform: Platform,
    /// The executable serving, with the nix `-wrapped` name unwrapped — see
    /// [`crate::sandbox::unwrapped`].
    pub executable: &'a Path,
    /// Whether the desktop app started it, which is what `--desktop` says.
    pub desktop: bool,
    /// Where an AppImage's runtime mounted the image, where it did.
    pub appdir: Option<&'a Path>,
    /// `%LOCALAPPDATA%`, under which the Windows installer puts the app.
    pub local_app_data: Option<&'a Path>,
}

/// The directory the Windows installer puts the app in, under
/// `%LOCALAPPDATA%` — `INSTALL_DIRECTORY_NAME` in `desktop/build/installer.nsh`.
const WINDOWS_INSTALL: &str = "Programs/Verkstead";

/// The directory every packed app keeps its sidecar in, under its resources —
/// `extraResources` in `desktop/electron-builder.yml`.
const BUNDLED_CLI: &str = "cli";

/// What a start was installed as.
pub(crate) fn of(started: Started<'_>) -> InstallKind {
    let executable = spelled(started.platform, started.executable);
    let parts: Vec<&str> = executable
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();

    // Before the desktop kinds, because neither is one whoever started it: a
    // nix-built app is still nix's to replace, and `pnpm start` runs the
    // checkout's own build with `--desktop` on it.
    if started.platform != Platform::Windows && executable.starts_with("/nix/store/") {
        return InstallKind::Nix;
    }

    if built_in_a_checkout(&parts) {
        return InstallKind::Checkout;
    }

    if !started.desktop {
        return InstallKind::Binary;
    }

    let bundled = parts.len() >= 3 && parts[parts.len() - 2] == BUNDLED_CLI;

    match started.platform {
        Platform::Linux => {
            let Some(appdir) = started.appdir else {
                return InstallKind::Binary;
            };
            let appdir = spelled(started.platform, appdir);
            let inside = executable.starts_with(&format!("{}/", appdir.trim_end_matches('/')));

            if inside && bundled {
                InstallKind::AppImage
            } else {
                InstallKind::Binary
            }
        }
        Platform::MacOs => {
            let bundle = parts.len() >= 5
                && parts[parts.len() - 5].ends_with(".app")
                && parts[parts.len() - 4] == "Contents"
                && parts[parts.len() - 3] == "Resources";

            if bundle && bundled {
                InstallKind::MacApp
            } else {
                InstallKind::Binary
            }
        }
        Platform::Windows => {
            let Some(local) = started.local_app_data else {
                return InstallKind::Binary;
            };
            let under = format!(
                "{}/{}/resources/",
                spelled(started.platform, local).trim_end_matches('/'),
                WINDOWS_INSTALL.to_lowercase(),
            );

            if executable.starts_with(&under) && bundled {
                InstallKind::WindowsInstall
            } else {
                InstallKind::Binary
            }
        }
    }
}

/// This process's own install, as [`of`] reads it — and `Binary`, said in the
/// log, where the process cannot say what it is running.
pub(crate) fn of_this_process(started_by: StartedBy) -> InstallKind {
    let executable = match std::env::current_exe() {
        Ok(executable) => crate::sandbox::unwrapped(&executable),
        Err(error) => {
            tracing::warn!(
                error = ?error,
                "Verkstead cannot say what image it is running, so it reads as a bare binary \
                 and its row offers no Update"
            );
            return InstallKind::Binary;
        }
    };

    let appdir = std::env::var_os("APPDIR").filter(|dir| !dir.is_empty());
    let local_app_data = std::env::var_os("LOCALAPPDATA").filter(|dir| !dir.is_empty());

    of(Started {
        platform: Platform::HERE,
        executable: &executable,
        desktop: started_by == StartedBy::TheDesktopApp,
        appdir: appdir.as_deref().map(Path::new),
        local_app_data: local_app_data.as_deref().map(Path::new),
    })
}

/// The word a member row keeps an install kind under — the wire's own, so a
/// row read by a human says what the member said.
pub(crate) fn word(install: InstallKind) -> &'static str {
    match install {
        InstallKind::AppImage => "AppImage",
        InstallKind::MacApp => "MacApp",
        InstallKind::WindowsInstall => "WindowsInstall",
        InstallKind::Nix => "Nix",
        InstallKind::Checkout => "Checkout",
        InstallKind::Binary => "Binary",
    }
}

/// And read back: `None` for a word this release has none for, which is a
/// row a newer Verkstead wrote, and reads as a member that said none.
pub(crate) fn from_word(said: &str) -> Option<InstallKind> {
    [
        InstallKind::AppImage,
        InstallKind::MacApp,
        InstallKind::WindowsInstall,
        InstallKind::Nix,
        InstallKind::Checkout,
        InstallKind::Binary,
    ]
    .into_iter()
    .find(|install| word(*install) == said)
}

/// What a member row keeps for the kind a device said, `None` kept as none —
/// see [`word`].
pub(crate) fn kept(install: Option<InstallKind>) -> Option<String> {
    install.map(|install| word(install).to_owned())
}

/// And what a member row's kept word says, the other way — see [`from_word`].
pub(crate) fn said(kept: Option<&str>) -> Option<InstallKind> {
    kept.and_then(from_word)
}

/// A path as one spelling: forward slashes, and on Windows lower-cased, the
/// filesystem there not reading case.
///
/// A string rather than a [`Path`], because the paths read here are every
/// platform's and a [`Path`] parses its own host's alone: `C:\Users` on Linux is
/// one component.
fn spelled(platform: Platform, path: &Path) -> String {
    let path = path.to_string_lossy();

    match platform {
        Platform::Windows => path.replace('\\', "/").to_lowercase(),
        Platform::Linux | Platform::MacOs => path.into_owned(),
    }
}

/// Whether the executable is a cargo build: under a `target/` whose next
/// directory, or the one after a target triple, is a profile's.
fn built_in_a_checkout(parts: &[&str]) -> bool {
    let profile = |part: Option<&&str>| matches!(part, Some(&"debug") | Some(&"release"));

    parts
        .iter()
        .enumerate()
        .filter(|(_, part)| **part == "target")
        .any(|(at, _)| profile(parts.get(at + 1)) || profile(parts.get(at + 2)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Every kind is kept in the word the wire spells it with, and read back
    /// as itself; a word from a newer Verkstead reads as none.
    #[test]
    fn a_kind_is_kept_in_the_wires_word() {
        for install in [
            InstallKind::AppImage,
            InstallKind::MacApp,
            InstallKind::WindowsInstall,
            InstallKind::Nix,
            InstallKind::Checkout,
            InstallKind::Binary,
        ] {
            assert_eq!(
                serde_json::to_value(install).unwrap(),
                serde_json::json!(word(install))
            );
            assert_eq!(from_word(word(install)), Some(install));
        }

        assert_eq!(from_word("Flatpak"), None);
    }

    /// A start on `platform` from `executable`, with nothing else said.
    fn started(platform: Platform, executable: &str, desktop: bool) -> InstallKind {
        read(platform, executable, desktop, None, None)
    }

    fn read(
        platform: Platform,
        executable: &str,
        desktop: bool,
        appdir: Option<&str>,
        local_app_data: Option<&str>,
    ) -> InstallKind {
        let appdir = appdir.map(PathBuf::from);
        let local_app_data = local_app_data.map(PathBuf::from);

        of(Started {
            platform,
            executable: Path::new(executable),
            desktop,
            appdir: appdir.as_deref(),
            local_app_data: local_app_data.as_deref(),
        })
    }

    const MOUNT: &str = "/tmp/.mount_VerksteaB3kQ9z";

    #[test]
    fn the_appimage_sidecar_reads_as_an_appimage() {
        let kind = read(
            Platform::Linux,
            "/tmp/.mount_VerksteaB3kQ9z/resources/cli/verkstead",
            true,
            Some(MOUNT),
            None,
        );

        assert_eq!(kind, InstallKind::AppImage);
        assert!(kind.updates_itself());
    }

    #[test]
    fn an_appdir_the_executable_is_not_inside_is_no_appimage() {
        let kind = read(
            Platform::Linux,
            "/usr/local/bin/verkstead",
            true,
            Some(MOUNT),
            None,
        );

        assert_eq!(kind, InstallKind::Binary);
    }

    #[test]
    fn the_appimage_sidecar_started_by_hand_is_no_desktop_install() {
        let kind = read(
            Platform::Linux,
            "/tmp/.mount_VerksteaB3kQ9z/resources/cli/verkstead",
            false,
            Some(MOUNT),
            None,
        );

        assert_eq!(kind, InstallKind::Binary);
    }

    #[test]
    fn a_desktop_run_on_linux_without_an_appdir_is_no_appimage() {
        let kind = started(
            Platform::Linux,
            "/opt/verkstead/resources/cli/verkstead",
            true,
        );

        assert_eq!(kind, InstallKind::Binary);
    }

    #[test]
    fn the_mac_bundles_sidecar_reads_as_the_mac_app() {
        let kind = started(
            Platform::MacOs,
            "/Applications/Verkstead.app/Contents/Resources/cli/verkstead",
            true,
        );

        assert_eq!(kind, InstallKind::MacApp);
        assert!(kind.updates_itself());
    }

    #[test]
    fn a_bundle_anywhere_is_the_mac_app() {
        let kind = started(
            Platform::MacOs,
            "/Users/ada/Downloads/Verkstead.app/Contents/Resources/cli/verkstead",
            true,
        );

        assert_eq!(kind, InstallKind::MacApp);
    }

    #[test]
    fn a_mac_binary_outside_a_bundle_is_a_bare_binary() {
        assert_eq!(
            started(Platform::MacOs, "/usr/local/bin/verkstead", true),
            InstallKind::Binary,
        );
        assert_eq!(
            started(
                Platform::MacOs,
                "/Applications/Verkstead.app/Contents/Resources/cli/verkstead",
                false,
            ),
            InstallKind::Binary,
        );
    }

    const LOCAL: &str = r"C:\Users\ada\AppData\Local";

    #[test]
    fn the_windows_installs_sidecar_reads_as_the_windows_install() {
        let kind = read(
            Platform::Windows,
            r"C:\Users\ada\AppData\Local\Programs\Verkstead\resources\cli\verkstead.exe",
            true,
            None,
            Some(LOCAL),
        );

        assert_eq!(kind, InstallKind::WindowsInstall);
        assert!(kind.updates_itself());
    }

    #[test]
    fn the_windows_install_is_read_without_regard_to_case() {
        let kind = read(
            Platform::Windows,
            r"c:\users\ada\appdata\local\programs\verkstead\Resources\CLI\Verkstead.exe",
            true,
            None,
            Some(LOCAL),
        );

        assert_eq!(kind, InstallKind::WindowsInstall);
    }

    #[test]
    fn a_windows_binary_elsewhere_is_a_bare_binary() {
        assert_eq!(
            read(
                Platform::Windows,
                r"C:\Users\ada\Downloads\verkstead-windows-x64.exe",
                true,
                None,
                Some(LOCAL),
            ),
            InstallKind::Binary,
        );
        assert_eq!(
            read(
                Platform::Windows,
                r"C:\Users\ada\AppData\Local\Programs\Verkstead\resources\cli\verkstead.exe",
                false,
                None,
                Some(LOCAL),
            ),
            InstallKind::Binary,
        );
        assert_eq!(
            read(
                Platform::Windows,
                r"C:\Users\ada\AppData\Local\Programs\Verkstead\resources\cli\verkstead.exe",
                true,
                None,
                None,
            ),
            InstallKind::Binary,
        );
    }

    #[test]
    fn the_nixos_module_reads_as_nix() {
        let kind = started(
            Platform::Linux,
            "/nix/store/0c9m4l1x8yb2-verkstead-0.2.2/bin/verkstead",
            false,
        );

        assert_eq!(kind, InstallKind::Nix);
        assert!(!kind.updates_itself());
    }

    #[test]
    fn the_nix_wrapper_reads_as_nix_either_way() {
        assert_eq!(
            started(
                Platform::Linux,
                "/nix/store/0c9m4l1x8yb2-verkstead-0.2.2/bin/.verkstead-wrapped",
                false,
            ),
            InstallKind::Nix,
        );
        assert_eq!(
            started(
                Platform::MacOs,
                "/nix/store/0c9m4l1x8yb2-verkstead-0.2.2/bin/verkstead",
                true,
            ),
            InstallKind::Nix,
        );
    }

    #[test]
    fn a_checkouts_build_reads_as_a_checkout_whoever_started_it() {
        for desktop in [false, true] {
            assert_eq!(
                started(
                    Platform::Linux,
                    "/home/ada/src/verkstead/target/debug/verkstead",
                    desktop
                ),
                InstallKind::Checkout,
            );
            assert_eq!(
                started(
                    Platform::MacOs,
                    "/Users/ada/src/verkstead/target/release/verkstead",
                    desktop,
                ),
                InstallKind::Checkout,
            );
            assert_eq!(
                started(
                    Platform::Windows,
                    r"C:\src\verkstead\target\debug\verkstead.exe",
                    desktop,
                ),
                InstallKind::Checkout,
            );
        }
    }

    #[test]
    fn a_cross_build_reads_as_a_checkout() {
        let kind = started(
            Platform::Linux,
            "/home/ada/src/verkstead/target/x86_64-unknown-linux-musl/release/verkstead",
            false,
        );

        assert_eq!(kind, InstallKind::Checkout);
    }

    #[test]
    fn a_checkouts_build_run_from_an_appimage_mount_is_still_a_checkout() {
        // `VERKSTEAD_CLI` pointed at a checkout's build from inside a mount.
        let kind = read(
            Platform::Linux,
            "/home/ada/src/verkstead/target/debug/verkstead",
            true,
            Some(MOUNT),
            None,
        );

        assert_eq!(kind, InstallKind::Checkout);
    }

    #[test]
    fn a_directory_merely_called_target_is_no_checkout() {
        let kind = started(Platform::Linux, "/srv/target/bin/verkstead", false);

        assert_eq!(kind, InstallKind::Binary);
    }

    #[test]
    fn a_bare_serve_is_a_bare_binary() {
        for platform in [Platform::Linux, Platform::MacOs] {
            assert_eq!(
                started(platform, "/usr/local/bin/verkstead", false),
                InstallKind::Binary,
            );
        }
        assert_eq!(
            started(Platform::Windows, r"C:\tools\verkstead.exe", false),
            InstallKind::Binary,
        );
    }
}
