//! Claude Code: the first harness, and the one every rule here was written
//! for before there was a second.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::{Harness, Line, Resume, SignedOut};
use crate::platform::Platform;
use crate::store;

/// The program a Claude session is run as.
pub(crate) const BINARY: &str = "claude";

/// What claude 2.1.234 opens its banner with, the rest of the line being the
/// reset words and the key to press.
const EXHAUSTED: &str = "usage limit reached";

/// What claude 2.1.283 ends a line with when its account's login is gone:
/// `Not logged in · Please run /login`, `Login expired · Please run /login`,
/// `OAuth token revoked · Please run /login`, and
/// `API Error: 401 Invalid API key · Please run /login`. Read off the binary.
///
/// Matched without regard to case. The reasons in front of it are claude's and
/// will move; the instruction is the part they all share.
const SIGNED_OUT: &str = "please run /login";

/// Claude Code.
pub(crate) struct Claude;

impl Harness for Claude {
    fn binary(&self) -> &str {
        BINARY
    }

    fn named(&self) -> &str {
        "Claude Code"
    }

    /// **Its resume is `--resume <session-id>`**, written where the session id it
    /// is mutually exclusive with would have gone — after the prompt, which stays
    /// exactly where it already is, and which claude takes alongside the flag as the
    /// first message of the conversation it picks up (checked against claude 2.1.278:
    /// the log it resumed carries the prompt as its own last one, under the same session
    /// id and in the same file). A session id it holds no log for is refused by name
    /// rather than opened, which is why the log is proved to be there before the line is
    /// built at all — see [`crate::transcript::carried`].
    fn line(&self, _worktree: Option<&Path>, _companions: &[PathBuf]) -> Line {
        Line {
            model: "--model".to_owned(),
            prompt: None,
            names_the_session: true,
            resume: Some(Resume::Flag("--resume".to_owned())),
            tail: vec!["--dangerously-skip-permissions".to_owned()],
        }
    }

    /// None, and none rather than an unknown one: Claude draws inline rather
    /// than repainting a screen, so there is no frame to read a prompt off,
    /// and three seconds of silence is an answer that works.
    fn at_work(&self) -> Option<&str> {
        None
    }

    fn exhausted(&self) -> Option<&str> {
        Some(EXHAUSTED)
    }

    fn signed_out(&self) -> Option<SignedOut<'_>> {
        Some(SignedOut {
            ends: SIGNED_OUT,
            then: None,
        })
    }

    fn channel(&self) -> store::Channel {
        store::Channel::Blocking
    }

    /// `mcp__<server>__<tool>`, read off Claude Code 2.1.268.
    fn tools_named(&self) -> &str {
        "mcp__<server>__<tool>"
    }

    /// That it is not to update the install it is running. The native binary
    /// writes a new version into `~/.local/share/claude/versions/`, which is
    /// read-only inside — see [`crate::sandbox`], which is what puts it inside
    /// — so an updater in there is a session spending its first minutes
    /// failing at something nobody asked it to do, and the install is the
    /// human's own besides. Every Claude session wherever `claude` was found,
    /// rather than only the ones that found the human's: a session is never
    /// what moves an install on, whosever it is.
    fn environment(&self, _platform: Platform) -> Vec<(&str, &str)> {
        vec![(crate::sandbox::DISABLE_AUTOUPDATER, "1")]
    }

    fn written_in_worktree(&self) -> Option<&str> {
        None
    }

    /// Whether the file `at`, resolved for `claude`, is Claude Code's **desktop
    /// app** rather than the command-line tool a session is launched as.
    ///
    /// **Decided by shape, and nothing is run to find out.** An Electron binary
    /// asked for `--version` may open a window on somebody's screen, and
    /// [ADR 0016](../../../../docs/adr/0016-onboarding.md) kept the wizard to reads —
    /// so this is `stat`s and path reading, and every arm of it is a unit test on a
    /// machine that is not Windows.
    ///
    /// Two shapes, and the platform is a value rather than a `cfg`:
    ///
    /// - an ancestor directory named [`ANTHROPIC_CLAUDE`], which is where the app a
    ///   person downloads installs itself under `%LOCALAPPDATA%` — the versioned
    ///   `app-x.y.z` it really runs out of is under the same directory;
    /// - a sibling [`SQUIRREL_UPDATE`], which is Squirrel's updater standing beside
    ///   the app it updates.
    ///
    /// **Two, and both of them seen.** A third was written and taken out again: a
    /// parent directory called `WindowsApps`, where an app-execution alias stands
    /// and where a Microsoft Store package would put one. No machine here has ever had that
    /// alias, and the cost of guessing wrong is not a row's wrong sentence — this
    /// answer refuses a session outright, with nothing on the Profile to overrule
    /// it, so a `claude.exe` alias standing in the directory every Windows user
    /// already has on their `PATH` would stop the product on that machine and tell
    /// the human to install what they have. A shape goes in here when somebody has
    /// seen it.
    fn desktop_app(&self, platform: Platform, at: &Path) -> bool {
        if platform != Platform::Windows {
            return false;
        }

        let Some(directory) = at.parent() else {
            return false;
        };

        // The ancestors rather than the parent: what the downloaded app resolves to
        // sits a versioned directory below the one it installed into.
        at.ancestors()
            .skip(1)
            .filter_map(Path::file_name)
            .any(|name| called(name, ANTHROPIC_CLAUDE))
            || directory.join(SQUIRREL_UPDATE).is_file()
    }
}

/// Whether a path component is `against`, the way Windows reads one: without
/// regard to case.
fn called(component: &OsStr, against: &str) -> bool {
    component
        .as_encoded_bytes()
        .eq_ignore_ascii_case(against.as_bytes())
}

/// Where the Claude Code desktop app installs itself under `%LOCALAPPDATA%`.
const ANTHROPIC_CLAUDE: &str = "AnthropicClaude";

/// Squirrel's updater, which stands beside the app it updates.
const SQUIRREL_UPDATE: &str = "Update.exe";
