//! Where a directory of Verkstead's own goes when nobody has said: the
//! platform's own place for it, resolved by hand out of the environment values
//! that platform keeps the answer in.
//!
//! Three directories are resolved here. The **Data Directory** is
//! `~/.local/share/verkstead` on Linux, `~/Library/Application
//! Support/Verkstead` on macOS and `%APPDATA%\Verkstead` on Windows unless
//! `--data-dir` says otherwise. Everything it holds moves with it, because it
//! is one directory and the only thing that varies is where it is when nobody
//! has said.
//!
//! The **Log Directory** is the other one, and nothing says where it goes:
//! `~/.local/state/verkstead` on Linux, `~/Library/Logs/Verkstead` on macOS,
//! `%LOCALAPPDATA%\Verkstead` on Windows. The three platforms disagree about
//! what such a directory even *is* — state on Linux, logs on macOS, the local
//! rather than the roaming application data on Windows — so it is one helper
//! with three arms rather than one notion spelled three ways.
//!
//! The **Build Cache** is the third, and the odd one: `$XDG_CACHE_HOME` or
//! `~/.cache/verkstead`, read the XDG way on every platform rather than three
//! ways. It is here because the reading is here — [`crate::build_cache`] owns
//! everything else about that directory, including being the one directory
//! Verkstead makes outside its own.
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
//! Build Cache's own resolution went untested for as long as it read the
//! process itself.

use std::path::{Path, PathBuf};

/// The name Verkstead's own directory takes where the platform's convention is
/// the binary's name in lowercase — Linux, where it stands among the
/// `~/.local/share` neighbours that are named for their programs, and the XDG
/// cache directory wherever that is read.
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
    /// Windows arm looks at, and only for the Data Directory.
    pub appdata: Option<PathBuf>,

    /// `$XDG_STATE_HOME`, which only the Linux arm looks at, and only for the
    /// Log Directory.
    pub xdg_state_home: Option<PathBuf>,

    /// `%LOCALAPPDATA%`, the application data that stays on the machine rather
    /// than following a roaming profile around — where a log file belongs, and
    /// the only thing the Windows arm reads for the Log Directory.
    pub local_appdata: Option<PathBuf>,

    /// `$XDG_CACHE_HOME`, which the Build Cache is resolved out of on every
    /// platform rather than on Linux alone — see [`default_cache_dir`].
    pub xdg_cache_home: Option<PathBuf>,
}

impl Environment {
    /// What this process was started with — the one read of the real
    /// environment, made at the entry point and passed down from there.
    pub fn of_the_process() -> Environment {
        Environment {
            xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            home: std::env::var_os("HOME").map(PathBuf::from),
            appdata: std::env::var_os("APPDATA").map(PathBuf::from),
            xdg_state_home: std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
            local_appdata: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
            xdg_cache_home: std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
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

/// The Log Directory this machine gives Verkstead, or `None` where it names
/// nowhere to put one.
///
/// The read of the process environment for the second of the two directories,
/// and the whole of what a binary outside this crate calls. **Nothing here
/// writes to it and nothing creates it**: the answer is where a log file would
/// go, and the binary that opens one is the binary that makes the directory,
/// exactly as the Build Cache makes its own where it uses it. The server keeps
/// logging to stdout whatever this says, so nowhere to resolve to answers
/// nothing rather than refusing anything — what a desktop binary with no home
/// should do is a decision for the stage that has a dialog to put it in.
pub fn log_dir() -> Option<PathBuf> {
    default_log_dir(Platform::HERE, &Environment::of_the_process())
}

/// Where the Log Directory goes on `platform`, out of `env`, or `None` where
/// that environment names nowhere to put one.
///
/// The three platforms disagree about what this directory *is* — a state
/// directory on Linux, a logs directory on macOS, the local rather than the
/// roaming application data on Windows — so what they share is the use a log
/// file makes of it rather than a notion each of them spells its own way.
pub fn default_log_dir(platform: Platform, env: &Environment) -> Option<PathBuf> {
    let dir = match platform {
        Platform::Linux => xdg(
            env.xdg_state_home.as_deref(),
            env.home.as_deref(),
            ".local/state",
        )?
        .join(LOWERCASE),
        Platform::MacOs => absolute(env.home.as_deref())?
            .join("Library/Logs")
            .join(CAPITALISED),
        Platform::Windows => set(env.local_appdata.as_deref())?.join(CAPITALISED),
    };

    Some(dir)
}

/// The Build Cache this machine gives Verkstead when nobody has said, or `None`
/// where it names nowhere to put one.
///
/// The read of the process environment for the third of them, and what
/// [`crate::build_cache`] resolves against — that module owns everything else
/// about the cache, including making it and refusing startup where it cannot
/// be made.
pub fn cache_dir() -> Option<PathBuf> {
    default_cache_dir(&Environment::of_the_process())
}

/// Where the Build Cache goes when nobody has said, out of `env`:
/// `$XDG_CACHE_HOME/verkstead` where that is set to an absolute path, and
/// `~/.cache/verkstead` otherwise.
///
/// **The one resolution here with no platform to take**, and deliberately: the
/// cache is read the XDG way wherever the server is compiled for, which is what
/// it has always done. Where a build cache belongs on a Mac is a question for
/// the stage that compiles on one, and moving it is not this module's to decide
/// on the way past.
pub fn default_cache_dir(env: &Environment) -> Option<PathBuf> {
    let dir = xdg(env.xdg_cache_home.as_deref(), env.home.as_deref(), ".cache")?.join(LOWERCASE);

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

    #[test]
    fn linux_logs_under_the_xdg_state_directory() {
        assert_eq!(
            default_log_dir(Platform::Linux, &with_home("/home/you")),
            Some(PathBuf::from("/home/you/.local/state/verkstead")),
        );
    }

    #[test]
    fn linux_honours_an_absolute_xdg_state_home() {
        let env = Environment {
            xdg_state_home: Some(PathBuf::from("/var/lib/state")),
            ..with_home("/home/you")
        };

        assert_eq!(
            default_log_dir(Platform::Linux, &env),
            Some(PathBuf::from("/var/lib/state/verkstead")),
            "the state variable gets the reading the data one gets, because it \
             is the same specification saying it",
        );
    }

    #[test]
    fn linux_ignores_a_relative_xdg_state_home() {
        let env = Environment {
            xdg_state_home: Some(PathBuf::from("state")),
            ..with_home("/home/you")
        };

        assert_eq!(
            default_log_dir(Platform::Linux, &env),
            Some(PathBuf::from("/home/you/.local/state/verkstead")),
            "a relative value would put the log file wherever the app was \
             launched from, which is the thing being fixed",
        );
    }

    #[test]
    fn linux_does_not_log_in_the_data_directory() {
        let env = Environment {
            xdg_data_home: Some(PathBuf::from("/var/lib/data")),
            ..with_home("/home/you")
        };

        assert_eq!(
            default_log_dir(Platform::Linux, &env),
            Some(PathBuf::from("/home/you/.local/state/verkstead")),
            "the data variable says nothing about where the state directory is",
        );
    }

    #[test]
    fn macos_logs_in_the_library_logs_directory() {
        assert_eq!(
            default_log_dir(Platform::MacOs, &with_home("/Users/you")),
            Some(PathBuf::from("/Users/you/Library/Logs/Verkstead")),
            "a Mac keeps logs of its own somewhere Console.app already looks",
        );
    }

    #[test]
    fn macos_reads_no_xdg_variable_for_it_either() {
        let env = Environment {
            xdg_state_home: Some(PathBuf::from("/var/lib/state")),
            ..with_home("/Users/you")
        };

        assert_eq!(
            default_log_dir(Platform::MacOs, &env),
            Some(PathBuf::from("/Users/you/Library/Logs/Verkstead")),
        );
    }

    #[test]
    fn windows_logs_in_the_local_application_data() {
        let env = Environment {
            local_appdata: Some(PathBuf::from(r"C:\Users\you\AppData\Local")),
            ..Environment::default()
        };

        assert_eq!(
            default_log_dir(Platform::Windows, &env),
            Some(PathBuf::from(r"C:\Users\you\AppData\Local").join("Verkstead")),
        );
    }

    #[test]
    fn windows_does_not_log_in_the_roaming_application_data() {
        let env = Environment {
            appdata: Some(PathBuf::from(r"C:\Users\you\AppData\Roaming")),
            ..Environment::default()
        };

        assert_eq!(
            default_log_dir(Platform::Windows, &env),
            None,
            "a log file follows nobody between machines, so the roaming \
             directory is not an answer to this question",
        );
    }

    #[test]
    fn nowhere_to_log_is_nowhere_on_every_platform() {
        for platform in [Platform::Linux, Platform::MacOs, Platform::Windows] {
            assert_eq!(
                default_log_dir(platform, &Environment::default()),
                None,
                "{platform:?} with an empty environment has nowhere to log to, \
                 and answering nothing is the whole of what that costs",
            );
        }
    }

    #[test]
    fn an_empty_or_relative_value_is_no_log_directory_at_all() {
        for (platform, env) in [
            (Platform::Linux, with_home("")),
            (Platform::Linux, with_home("home/you")),
            (Platform::MacOs, with_home("")),
            (
                Platform::Windows,
                Environment {
                    local_appdata: Some(PathBuf::new()),
                    ..Environment::default()
                },
            ),
        ] {
            assert_eq!(
                default_log_dir(platform, &env),
                None,
                "{platform:?} should ignore {env:?} rather than resolve against \
                 the working directory",
            );
        }
    }

    /// The Build Cache's own resolution, which read the process directly until
    /// this module was here to read it for them — and so had none of these.
    #[test]
    fn the_cache_falls_back_to_the_xdg_cache_directory() {
        assert_eq!(
            default_cache_dir(&with_home("/home/you")),
            Some(PathBuf::from("/home/you/.cache/verkstead")),
        );
    }

    #[test]
    fn the_cache_honours_an_absolute_xdg_cache_home() {
        let env = Environment {
            xdg_cache_home: Some(PathBuf::from("/var/cache")),
            ..with_home("/home/you")
        };

        assert_eq!(
            default_cache_dir(&env),
            Some(PathBuf::from("/var/cache/verkstead")),
            "the packaged unit says /var/cache/verkstead with the flag, and a \
             machine that exports the variable means the same thing",
        );
    }

    /// Both halves of the reading, because until it was this module's the cache
    /// filtered the variable and not the home — so a relative `HOME` put the
    /// cache wherever the unit had started the server.
    #[test]
    fn the_cache_ignores_a_relative_value() {
        let relative_variable = Environment {
            xdg_cache_home: Some(PathBuf::from("cache")),
            ..with_home("/home/you")
        };

        assert_eq!(
            default_cache_dir(&relative_variable),
            Some(PathBuf::from("/home/you/.cache/verkstead")),
            "a relative variable is no answer, and the home behind it is",
        );

        assert_eq!(
            default_cache_dir(&with_home("home/you")),
            None,
            "and a relative home is no answer either: a cache resolved against \
             wherever the unit started the server is one nobody will find to \
             clear",
        );
    }

    #[test]
    fn the_cache_reads_no_data_or_state_variable() {
        let env = Environment {
            xdg_data_home: Some(PathBuf::from("/var/lib/data")),
            xdg_state_home: Some(PathBuf::from("/var/lib/state")),
            ..with_home("/home/you")
        };

        assert_eq!(
            default_cache_dir(&env),
            Some(PathBuf::from("/home/you/.cache/verkstead")),
            "one reading of the environment is not one directory: the cache is \
             the cache variable's and nothing else's",
        );
    }

    #[test]
    fn nowhere_to_cache_is_nowhere() {
        assert_eq!(
            default_cache_dir(&Environment::default()),
            None,
            "an empty environment names nowhere to put a cache, which is what \
             the Build Cache refuses startup on",
        );
    }

    #[test]
    fn resolving_a_log_directory_makes_none() {
        let home = std::env::temp_dir().join("verkstead-log-dir-resolution");
        let dir = default_log_dir(Platform::Linux, &with_home(&home.to_string_lossy()))
            .expect("a home is enough to resolve against");

        assert!(
            !dir.exists(),
            "resolving says where the directory would go and nothing else — the \
             binary that opens a log file is the one that makes {}",
            dir.display(),
        );
    }
}
