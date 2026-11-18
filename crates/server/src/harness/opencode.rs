//! OpenCode.

use std::path::{Path, PathBuf};

use super::{Harness, Line, Resume, SignedOut};
use crate::platform::Platform;
use crate::store;

/// The program an OpenCode session is run as.
pub(crate) const BINARY: &str = "opencode";

/// What opencode has on its Screen while it is working, and nothing of what it
/// has there when it is waiting for a human — see [`crate::sessions::Signature::AtWork`].
///
/// Read off opencode 1.18.25 driven on a hundred-column terminal rather than
/// guessed at, and it comes out where the two before it came out: the frame
/// opencode leaves when its turn is over and the frame it draws mid-turn are
/// the same screen but for the status bar at its foot. Mid-turn that bar is a
/// progress dial and this label — `⬝⬝⬝⬝⬝■■■  esc interrupt` — and at rest it is
/// the project's path instead. The composer above it, the `Build auto ·
/// <model>` label on its border and the `tab agents` and `ctrl+p commands`
/// hints beside this one stand in both, so none of them says whether the
/// session has stopped. Across two turns of one session sampled once a second —
/// a tool call and then a streamed reply, twice — the label was in every
/// working frame and in none of the resting ones.
///
/// The label rather than the dial in front of it, which goes and comes with it:
/// the dial's cells fill and empty every frame where this does not move, and a
/// keybinding label is a harder thing to find by accident in what the session
/// printed than a run of block characters is.
///
/// Two words where codex's is three — opencode writes `esc interrupt` where
/// codex writes `esc to interrupt` — so neither backend's constant reads the
/// other's frame, which is what the tests on this reading turn on.
///
/// **And it is the same label in either interface opencode offers.** The
/// minimal one — `--mini`, which is what draws inline rather than taking the
/// alternate screen — puts it in a status bar of its own, and it goes there
/// when the turn is over exactly as it goes here. So the reading does not turn
/// on which of the two a session was started in, whatever a later stage
/// decides about the Capture (ADR-0011).
///
/// Named for the same reason [`super::codex::AT_WORK`] is, and it is the same bargain
/// the usage-limit phrase makes: the wording is opencode's and it will move,
/// and moving it costs one edit here.
const AT_WORK: &str = "esc interrupt";

/// OpenCode.
pub(crate) struct OpenCode;

impl Harness for OpenCode {
    fn binary(&self) -> &str {
        BINARY
    }

    fn named(&self) -> &str {
        "OpenCode"
    }

    /// **opencode has no such flag and takes the alternate screen**, which is a
    /// finding rather than an oversight (ADR-0011): its Screen reads either way,
    /// since the screen model tracks which buffer is in front, and what its Capture
    /// then replays to is the farewell banner opencode leaves on the ordinary
    /// buffer as it exits rather than anything the session did. The record this
    /// backend is read back from is its session store, which is what the Timeline
    /// draws from. `--mini` — the minimal interface, which draws inline and carries
    /// the same at-work label — is what to reach for the day the Capture has to be
    /// that record instead.
    ///
    /// **OpenCode's is the shortest line here.** The model as `-m provider/model`
    /// — the whole string is what the human typed on the Profile — the Brief under
    /// `--prompt`, and `--auto` for the approvals, which is where the other three
    /// backends' bypasses sit. `--prompt` submits rather than only prefilling
    /// (checked against opencode 1.18.25: the home screen fills the prompt in and
    /// sends it as soon as the model store is ready), so nothing is typed into the
    /// terminal to start the session working. It has no sandbox of its own to
    /// switch off, and it takes no session id at launch: `--session` means
    /// *continue this one* and is validated against the store before the TUI
    /// starts, so a fresh name would be a session that never starts rather than a
    /// session named — which is why its log is found rather than named, as codex's
    /// is. Everything else about the account is in the directories the Profile
    /// named — see [`crate::sandbox`].
    ///
    /// **And that same flag is its resume**, the shortest line here staying the
    /// shortest: `--session <session-id>` where the name a session that took one
    /// would have gone, the prompt under its own flag exactly where it already is,
    /// and the model and the approvals unchanged. Nothing is added and nothing comes
    /// off — opencode has no equivalent of codex's question about which directory to
    /// resume in, because it does not ask: it resumes in the directory the session's
    /// own row records, which is why that row is what has to be put right. `--fork`
    /// beside the flag is what would make it a conversation branched rather than
    /// carried on, and it is never passed (opencode 1.18.31's own help, and driven to
    /// prove the rest of this).
    ///
    /// **Its id is the store's own, as Codex's is.** opencode names no session on the
    /// line, so what goes after the flag is the id the row in its store turned out to
    /// be under — read as the records reader finds that row and written onto the
    /// record there, which is what leaves a device that never ran the session able to
    /// resume it. See [`crate::transcript::found_in_the_store`].
    ///
    /// **And its store is the one that has to be written into rather than moved
    /// about.** opencode keeps a row per session in one database and the memory sync
    /// carries the file whole, so the row of a carried session lands still recording
    /// the *sending* device's Worktree — and a resume against it would open in a
    /// directory this machine has not got, the row's own directory being where
    /// opencode resumes. So that one column is brought onto this device's Worktree
    /// before the line is built at all, and a store this build cannot write leaves
    /// the row alone and falls through to Verkstead's own Resume. See
    /// [`crate::records::carried`].
    fn line(&self, _worktree: Option<&Path>, _companions: &[PathBuf]) -> Line {
        Line {
            model: "-m".to_owned(),
            prompt: Some("--prompt".to_owned()),
            names_the_session: false,
            resume: Some(Resume::Flag("--session".to_owned())),
            tail: vec!["--auto".to_owned()],
        }
    }

    fn at_work(&self) -> Option<&str> {
        Some(AT_WORK)
    }

    /// None. It retries a provider's limit internally before anything reaches
    /// the screen (ADR-0011), so nobody has watched one surface and there is no
    /// wording to be read against — which leaves such a stop landing as the
    /// ordinary stall it would have been anyway.
    fn exhausted(&self) -> Option<&str> {
        None
    }

    /// None: Verkstead cannot run an OpenCode login.
    fn signed_out(&self) -> Option<SignedOut<'_>> {
        None
    }

    fn channel(&self) -> store::Channel {
        store::Channel::Blocking
    }

    /// `<server>_<tool>`, the two joined by one underscore, read off OpenCode
    /// 1.18.31.
    fn tools_named(&self) -> &str {
        "<server>_<tool>"
    }

    /// Where its store goes, because opencode names the file after the release
    /// channel the install came from and the reader that follows a Transcript
    /// opens the path this chose; and how long its shell tool holds a command
    /// the model gave no timeout of its own, which is what a blocking ask under
    /// this harness stands on — see [`crate::sandbox`].
    fn environment(&self, _platform: Platform) -> Vec<(&str, &str)> {
        vec![
            (
                crate::sandbox::OPENCODE_DB,
                crate::sandbox::OPENCODE_DB_FILE,
            ),
            (
                crate::sandbox::OPENCODE_BASH_DEFAULT_TIMEOUT,
                crate::sandbox::OPENCODE_BASH_DEFAULT_TIMEOUT_MS,
            ),
        ]
    }

    fn written_in_worktree(&self) -> Option<&str> {
        None
    }
}
