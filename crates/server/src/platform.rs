//! Where a directory of Verkstead's own goes when nobody has said: the
//! platform's own place for it, resolved by hand out of the environment values
//! that platform keeps the answer in.
//!
//! One directory is resolved here so far — the **Data Directory**, which is
//! `~/.local/share/verkstead` on Linux, `~/Library/Application
//! Support/Verkstead` on macOS and `%APPDATA%\Verkstead` on Windows unless
//! `--data-dir` says otherwise. Everything it holds moves with it, because it
//! is one directory and the only thing that varies is where it is when nobody
//! has said.
//!
//! **By hand rather than by crate.** `dirs` would answer Linux and macOS out of
//! the same environment variables this does, and then answer Windows through a
//! Win32 known-folder call — which this CI cannot compile, let alone run, until
//! Windows is a target it builds for. So the resolution is written as a
//! function of the values rather than of the process: the platform and the
//! environment go in and a path comes back, and all three arms are exercised by
//! ordinary unit tests on the Linux runner.
//!
//! **The process environment is read once, at the edge** — in
//! [`Environment::of_the_process`], which nothing below the entry point calls.
//! That is what leaves the tests reading values they were handed rather than
//! mutating the ones the process has: `std::env::set_var` is `unsafe` under
//! this edition and races every other test in its binary, which is why the
//! Build Cache's own resolution has no unit test at all.

use std::path::{Path, PathBuf};

/// The name Verkstead's own directory takes where the platform's convention is
/// the binary's name in lowercase — Linux, where it stands among the
/// `~/.local/share` neighbours that are named for their programs.
const LOWERCASE: &str = "verkstead";

/// And where the convention is the product's name as a human writes it — macOS
/// and Windows, whose `Application Support` and `%APPDATA%` hold the names
/// people read in a file dialog.
const CAPITALISED: &str = "Verkstead";

/// Whose conventions a directory is resolved by.
///
/// A value rather than a `cfg`, so that the arm a machine will never run is
/// still an arm a test can call. [`Platform::HERE`] is the one this binary was
/// built for, and it is the only one anything outside a test passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// XDG: the directory is `$XDG_DATA_HOME` where that is set to an absolute
    /// path, and under `~/.local/share` otherwise.
    Linux,
    /// The directories Apple names, all of them under the home directory.
    MacOs,
    /// The application-data directories Windows names in the environment, so
    /// that a roaming profile is the operating system's business rather than
    /// Verkstead's.
    Windows,
}

impl Platform {
    /// The platform this binary was compiled for. Everything but a test
    /// resolves against this one.
    pub const HERE: Platform = if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    };
}

/// The environment values the platform directories are resolved out of, as they
/// were read: nothing here has been judged yet, because which of them is worth
/// anything is the resolving arm's business.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    /// `$XDG_DATA_HOME`, which only the Linux arm looks at.
    pub xdg_data_home: Option<PathBuf>,

    /// `$HOME`, which both Unix arms fall back to and Windows never reads.
    pub home: Option<PathBuf>,

    /// `%APPDATA%`, the roaming application data directory, which only the
    /// Windows arm looks at.
    pub appdata: Option<PathBuf>,
}

impl Environment {
    /// What this process was started with — the one read of the real
    /// environment, made at the entry point and passed down from there.
    pub fn of_the_process() -> Environment {
        Environment {
            xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            home: std::env::var_os("HOME").map(PathBuf::from),
            appdata: std::env::var_os("APPDATA").map(PathBuf::from),
        }
    }
}

/// The Data Directory this run keeps everything in: `said`, where the flag or
/// `VERKSTEAD_DATA_DIR` said one, and the platform's own place otherwise.
///
/// **Nowhere to resolve to refuses startup**, naming the flag, exactly as the
/// Build Cache refuses naming `--build-cache-dir` and for the same reason: a
/// service unit that says nothing about a home would otherwise be handed a Data
/// Directory nobody chose and nobody will find. That makes it an error at
/// startup rather than anything the flag parser can express — which is why the
/// flag holds only what was *said*, and why the resolving happens here, where a
/// failure has somewhere to be worded.
pub fn data_dir(said: Option<&Path>) -> anyhow::Result<PathBuf> {
    match said {
        Some(dir) => Ok(dir.to_owned()),
        None => default_data_dir(Platform::HERE, &Environment::of_the_process())
            .ok_or_else(|| nowhere(Platform::HERE)),
    }
}

/// Where the Data Directory goes on `platform` when nobody has said, out of
/// `env`, or `None` where that environment names nowhere to put one.
pub fn default_data_dir(platform: Platform, env: &Environment) -> Option<PathBuf> {
    let dir = match platform {
        Platform::Linux => xdg(
            env.xdg_data_home.as_deref(),
            env.home.as_deref(),
            ".local/share",
        )?
        .join(LOWERCASE),
        Platform::MacOs => absolute(env.home.as_deref())?
            .join("Library/Application Support")
            .join(CAPITALISED),
        Platform::Windows => set(env.appdata.as_deref())?.join(CAPITALISED),
    };

    Some(dir)
}

/// The refusal a machine with nowhere to put a Data Directory gets, naming both
/// what it left unset and the flag that would settle it.
fn nowhere(platform: Platform) -> anyhow::Error {
    let unset = match platform {
        Platform::Linux => "neither XDG_DATA_HOME nor HOME is set to an absolute path",
        Platform::MacOs => "HOME is not set to an absolute path",
        Platform::Windows => "APPDATA is not set",
    };

    anyhow::anyhow!(
        "there is nowhere to keep the Data Directory: {unset}, so say where it goes \
         with --data-dir"
    )
}

/// The XDG reading a Unix directory gets: the variable where it is set to an
/// absolute path, and `$HOME/<under>` where it is not — the specification's own
/// fallback, and what most machines have.
fn xdg(variable: Option<&Path>, home: Option<&Path>, under: &str) -> Option<PathBuf> {
    match absolute(variable) {
        Some(dir) => Some(dir.to_owned()),
        None => Some(absolute(home)?.join(under)),
    }
}

/// `value` where it is an absolute path, and nothing where it is relative or
/// empty — as the XDG specification says of its own variables, and for the
/// reason this whole module exists: a directory resolved against wherever the
/// server happened to be started is the thing the platform default replaces.
fn absolute(value: Option<&Path>) -> Option<&Path> {
    value.filter(|dir| dir.is_absolute())
}

/// `value` where the machine set it to anything at all.
///
/// What the Windows arm has instead of [`absolute`], deliberately.
/// `Path::is_absolute` answers by the rules of the platform the code was
/// compiled for, so `C:\Users\you\AppData\Roaming` put through it on the Linux
/// runner comes back relative — and a check that holds on one platform and
/// misfires on the other is worse than the one Windows does not need, its
/// application-data variables being absolute or absent.
fn set(value: Option<&Path>) -> Option<&Path> {
    value.filter(|dir| !dir.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An environment holding `home` and nothing else, which is what a Unix
    /// machine that has never heard of XDG has.
    fn with_home(home: &str) -> Environment {
        Environment {
            home: Some(PathBuf::from(home)),
            ..Environment::default()
        }
    }

    #[test]
    fn linux_puts_it_under_the_xdg_data_directory() {
        assert_eq!(
            default_data_dir(Platform::Linux, &with_home("/home/you")),
            Some(PathBuf::from("/home/you/.local/share/verkstead")),
        );
    }

    #[test]
    fn linux_honours_an_absolute_xdg_data_home() {
        let env = Environment {
            xdg_data_home: Some(PathBuf::from("/var/lib/data")),
            ..with_home("/home/you")
        };

        assert_eq!(
            default_data_dir(Platform::Linux, &env),
            Some(PathBuf::from("/var/lib/data/verkstead")),
            "the variable is the specification's own answer where it says one",
        );
    }

    #[test]
    fn linux_ignores_a_relative_xdg_data_home() {
        let env = Environment {
            xdg_data_home: Some(PathBuf::from("data")),
            ..with_home("/home/you")
        };

        assert_eq!(
            default_data_dir(Platform::Linux, &env),
            Some(PathBuf::from("/home/you/.local/share/verkstead")),
            "a relative value would resolve against whatever directory the unit \
             started the server in, which is what the default is here to stop",
        );
    }

    #[test]
    fn macos_puts_it_in_application_support() {
        assert_eq!(
            default_data_dir(Platform::MacOs, &with_home("/Users/you")),
            Some(PathBuf::from(
                "/Users/you/Library/Application Support/Verkstead"
            )),
        );
    }

    #[test]
    fn macos_reads_no_xdg_variable() {
        let env = Environment {
            xdg_data_home: Some(PathBuf::from("/var/lib/data")),
            ..with_home("/Users/you")
        };

        assert_eq!(
            default_data_dir(Platform::MacOs, &env),
            Some(PathBuf::from(
                "/Users/you/Library/Application Support/Verkstead"
            )),
            "the variable is not part of the picture on a Mac, whoever exported it",
        );
    }

    #[test]
    fn windows_puts_it_in_the_roaming_application_data() {
        let env = Environment {
            appdata: Some(PathBuf::from(r"C:\Users\you\AppData\Roaming")),
            ..Environment::default()
        };

        assert_eq!(
            default_data_dir(Platform::Windows, &env),
            Some(PathBuf::from(r"C:\Users\you\AppData\Roaming").join("Verkstead")),
        );
    }

    #[test]
    fn windows_reads_no_home() {
        assert_eq!(
            default_data_dir(Platform::Windows, &with_home("/home/you")),
            None,
            "a Unix home on a Windows machine says nothing about where its \
             application data goes",
        );
    }

    #[test]
    fn nowhere_to_put_it_is_nowhere_on_every_platform() {
        for platform in [Platform::Linux, Platform::MacOs, Platform::Windows] {
            assert_eq!(
                default_data_dir(platform, &Environment::default()),
                None,
                "{platform:?} with an empty environment has nowhere to resolve to",
            );
        }
    }

    #[test]
    fn an_empty_or_relative_value_is_no_value_at_all() {
        for (platform, env) in [
            (Platform::Linux, with_home("")),
            (Platform::Linux, with_home("home/you")),
            (Platform::MacOs, with_home("")),
            (
                Platform::Windows,
                Environment {
                    appdata: Some(PathBuf::new()),
                    ..Environment::default()
                },
            ),
        ] {
            assert_eq!(
                default_data_dir(platform, &env),
                None,
                "{platform:?} should ignore {env:?} rather than resolve against the \
                 working directory",
            );
        }
    }

    #[test]
    fn nowhere_to_resolve_to_says_so_and_names_the_flag() {
        for (platform, variable) in [
            (Platform::Linux, "XDG_DATA_HOME"),
            (Platform::MacOs, "HOME"),
            (Platform::Windows, "APPDATA"),
        ] {
            let refusal = nowhere(platform).to_string();

            assert!(
                refusal.contains("--data-dir") && refusal.contains(variable),
                "the refusal should name the flag that settles it and what \
                 {platform:?} left unset, got:\n{refusal}",
            );
        }
    }

    #[test]
    fn what_was_said_wins_over_every_platform() {
        assert_eq!(
            data_dir(Some(Path::new("."))).unwrap(),
            PathBuf::from("."),
            "a developer running out of a checkout asks for the working \
             directory, and gets it",
        );
    }
}
