//! The Antigravity CLI.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use verkstead_render::Broken;

use super::{
    Carrying, Declared, HOME_REFUSED, Harness, Line, MODEL, PROMPT, ReadIn, Resume, SESSION,
    SignedOut, said,
};
use crate::logins::Speaking;
use crate::platform::Platform;
use crate::sandbox::root::{self, Part, Place, Whose};
use crate::sandbox::{self, Home};
use crate::settings::AttachedServer;
use crate::store;
use crate::transcript::{self, Search};

/// The program an Antigravity CLI session is run as.
pub(crate) const BINARY: &str = "agy";

/// What agy 1.4.12 ends a line with when it has no login to run on:
/// `Welcome to the Antigravity CLI. You are currently not signed in.`, over the
/// sign-in picker it then waits at.
///
/// Drawn the same for an account that never had a login and for one whose
/// token was refused when agy went to refresh it — both read off the real
/// binary, the second with Google's token endpoint answered by a stand-in.
/// **And drawn by a session signed in too**, on its way up: measured on a Mac,
/// where the Keychain agy is refused first holds it long enough to be seen,
/// the sentence is followed by `Signing in...` and drawn over once the login
/// file is read. What tells the two apart is [`PICKER`].
const SIGNED_OUT: &str = "you are currently not signed in.";

/// The line agy 1.4.12 draws next when it really has no login: the head of
/// its sign-in picker, `Select login method:`, over the methods to pick from.
const PICKER: &str = "select login method:";

/// What the Antigravity CLI has on its Screen while it is working, and nothing
/// of what it has there when it is waiting for a human — see
/// [`crate::sessions::Signature::AtWork`].
///
/// Read off agy 1.4.12, signed in and launched on the line a session is, with
/// the running light off, sampled once a second across three sessions: a reply
/// of one word, a call to an MCP server, and a shell command held for four
/// minutes as a background task. agy draws inline, but it repaints the foot of
/// what it drew — the composer between two rules and a hint row under it — and
/// that row is where the two states differ. Mid-turn its left end is this
/// label; at rest it is `? for shortcuts`. The model's name on the right of the
/// row, and the `1 task(s) · /tasks` beside it while a background task runs,
/// stand in both. The label was in every working frame and in none of the
/// resting ones, a turn that had handed its command to the background included.
///
/// **Silence would have read it too**: a resting agy sends not a byte, and a
/// working one never went a second without repainting its spinner. The label
/// rather than the silence because it is the reading every backend that
/// repaints a screen takes, and because a turn waiting on a slow model would
/// otherwise rest on a spinner nobody has promised to keep turning.
///
/// Three words, as codex's are, and a different three — `esc to cancel` against
/// `esc to interrupt` — so neither backend's constant reads the other's frame.
///
/// Named for the same reason [`super::codex::AT_WORK`] is, and it is the same bargain
/// the usage-limit phrase makes: the wording is agy's and it will move, and
/// moving it costs one edit here.
const AT_WORK: &str = "esc to cancel";

/// The Antigravity CLI.
pub(crate) struct Agy;

impl Harness for Agy {
    fn binary(&self) -> &str {
        BINARY
    }

    fn named(&self) -> &str {
        "Antigravity CLI"
    }

    fn mark(&self) -> &str {
        include_str!("marks/antigravity.svg")
    }

    fn paths(&self) -> Vec<Declared<'_>> {
        vec![Declared {
            key: store::HOME,
            label: "Home directory",
            purpose: "Mounted at `~/.gemini`. It holds `antigravity-cli` once agy has run in it.",
            placeholder: "/home/you/accounts/work/.gemini",
            file: false,
            made: false,
            refused: HOME_REFUSED,
            within: &[],
        }]
    }

    fn found_at(&self) -> Vec<&str> {
        vec!["~/.gemini"]
    }

    /// **The Antigravity CLI's says what a session needs of agy's settings on the
    /// line**, as a session-only `--settings` JSON that agy never saves — see
    /// [`settings`] — so nothing of it is written into the account (ADR-0011).
    /// The model under `--model`, the Brief under `--prompt-interactive`, which
    /// runs it as the first turn and stays interactive, `--dangerously-skip-
    /// permissions` for the approvals, and an `--add-dir` per Companion Repo, which
    /// is how agy is told a directory beyond the one it was started in is part of
    /// the workspace. It takes no session id at launch, so its conversation is
    /// found rather than named. Read off agy 1.4.12's own help.
    ///
    /// **Its resume is `--conversation <id>`**, written where the session id would
    /// have gone, after the Brief — which agy takes alongside it as the next turn
    /// of the conversation it picks up (checked against agy 1.4.12, which appended
    /// that turn to the same conversation's transcript). The id is the one its
    /// summaries index gave the conversation, put on the record as the session was
    /// followed — see [`crate::transcript`] — and the record is proved to be here
    /// before the line is built, see [`crate::transcript::carried`].
    fn line(&self, worktree: Option<&Path>, companions: &[PathBuf]) -> Line {
        let mut tail = vec!["--dangerously-skip-permissions".to_owned()];

        for companion in companions {
            tail.push("--add-dir".to_owned());
            tail.push(companion.display().to_string());
        }

        tail.push("--settings".to_owned());
        tail.push(settings(
            worktree
                .into_iter()
                .chain(companions.iter().map(PathBuf::as_path)),
        ));

        Line {
            model: said(&["--model", MODEL]),
            prompt: said(&["--prompt-interactive", PROMPT]),
            session: None,
            resume: Some(Resume::Flag(said(&["--conversation", SESSION]))),
            tail,
        }
    }

    fn at_work(&self) -> Option<&str> {
        Some(AT_WORK)
    }

    /// None: nobody has watched agy run out yet.
    fn exhausted(&self) -> Option<&str> {
        None
    }

    fn signed_out(&self) -> Option<SignedOut<'_>> {
        Some(SignedOut {
            ends: SIGNED_OUT,
            then: Some(PICKER),
            read_in: ReadIn::Every,
        })
    }

    /// Blocking, and for free: agy 1.4.12's shell tool turns a long command
    /// into a background task, ends the turn, and wakes the session when it
    /// returns — an hour's hold measured with no model turn spent on it.
    fn channel(&self) -> store::Channel {
        store::Channel::Blocking
    }

    /// No tool named after its server at all, read off agy 1.4.12: every
    /// server's tools are reached through one tool of agy's own,
    /// `call_mcp_tool`, which takes the two names as its `ServerName` and
    /// `ToolName` arguments. agy files each tool's schema under
    /// `antigravity-cli/mcp/<server>/<tool>.json` for the model to read first.
    fn tools_named(&self) -> &str {
        r#"call_mcp_tool(ServerName: "<server>", ToolName: "<tool>")"#
    }

    /// On Windows, that its login is the account's file rather than the session
    /// account's vault — see [`crate::sandbox`]'s `AGY_FILE_STORE`.
    fn environment(&self, platform: Platform) -> Vec<(&str, &str)> {
        match platform {
            Platform::Windows => vec![crate::sandbox::AGY_FILE_STORE],
            Platform::Linux | Platform::MacOs => Vec::new(),
        }
    }

    /// `.gemini/`, where agy 1.4.12 keeps its transcript of the session and
    /// whatever else it files under the workspace.
    fn written_in_worktree(&self) -> Vec<&str> {
        vec![".gemini/"]
    }

    fn kept_in(&self, home: &Path) -> store::Account {
        store::Account::home(store::AgentType::AGY, home.join(sandbox::AGY_INSIDE_HOME))
    }

    /// agy's own directory inside `~/.gemini` rather than the whole of it:
    /// that is what a session is given the memory of, and what tells an
    /// account from a bare `~/.gemini` Gemini CLI left — see
    /// [`sandbox::AGY_ACCOUNT_MARK`].
    fn account_inside(&self, account: &store::Account, home: &Path) -> Vec<(PathBuf, PathBuf)> {
        vec![(
            account.directory().join(sandbox::AGY_ACCOUNT_MARK),
            home.join(sandbox::AGY_INSIDE_HOME)
                .join(sandbox::AGY_ACCOUNT_MARK),
        )]
    }

    fn judged(&self, account: &store::Account) -> Vec<(PathBuf, Broken)> {
        vec![(account.directory().to_owned(), Broken::HomeMissing)]
    }

    fn root(&self) -> &str {
        sandbox::AGY_INSIDE_HOME
    }

    /// Its two directories — see [`root::AGY_CLI`] — with the `cache/` inside
    /// the first that holds [`root::AGY_ONBOARDED`].
    fn furnished(&self) -> Vec<PathBuf> {
        vec![
            PathBuf::from(root::AGY_CLI),
            PathBuf::from(root::AGY_CUSTOMISATION),
            Path::new(root::AGY_ONBOARDED).parent().unwrap().to_owned(),
        ]
    }

    fn login(&self) -> &str {
        root::AGY_LOGIN
    }

    fn configuration(&self) -> Vec<PathBuf> {
        vec![PathBuf::from(root::AGY_SETTINGS)]
    }

    /// A `settings.json` — see [`root::agy_settings`] — and, where the
    /// Conversation attached any, an `mcp_config.json` beside it holding the
    /// servers — see [`root::agy_servers`] — and the record that the account
    /// has been through its first run.
    fn configured(
        &self,
        found: Vec<Option<Vec<u8>>>,
        servers: &[AttachedServer],
    ) -> Vec<(PathBuf, Vec<u8>)> {
        let [settings] = <[_; 1]>::try_from(found).expect("one file asked for");

        let settings = (
            PathBuf::from(root::AGY_SETTINGS),
            root::agy_settings(settings.as_deref()),
        );

        let onboarded = (
            PathBuf::from(root::AGY_ONBOARDED),
            root::AGY_ONBOARDED_AS.into(),
        );

        match servers.is_empty() {
            true => vec![settings, onboarded],
            false => vec![
                settings,
                (PathBuf::from(root::AGY_MCP), root::agy_servers(servers)),
                onboarded,
            ],
        }
    }

    fn instructions(&self) -> PathBuf {
        PathBuf::from(root::AGY_INSTRUCTIONS)
    }

    fn memory(&self, _place: Option<&Place>) -> Vec<PathBuf> {
        root::AGY_MEMORY
            .iter()
            .chain(root::AGY_SUMMARIES_JOINED)
            .map(PathBuf::from)
            .collect()
    }

    /// The summaries database and its two siblings — see
    /// [`root::AGY_SUMMARIES_JOINED`].
    fn memory_files(&self) -> Vec<PathBuf> {
        root::AGY_SUMMARIES_JOINED
            .iter()
            .map(PathBuf::from)
            .collect()
    }

    /// The two directories, and not the database: agy makes its own where
    /// there is none, and a database of nothing is no file.
    fn unshared(&self) -> Vec<&str> {
        root::AGY_MEMORY.to_vec()
    }

    /// The record `--conversation` resumes from and the brain directory the
    /// Transcript is followed in, each picked out by the ids agy gave this
    /// Conversation's sessions. Not the summaries database, which is the
    /// human's index of every conversation they have had: agy 1.4.12 resumes a
    /// conversation that has no row in it and writes the row itself.
    fn synced(
        &self,
        _place: Option<&Place>,
        _worktree: Option<&str>,
        sessions: &[String],
    ) -> Vec<Part> {
        vec![
            Part {
                label: root::CONVERSATIONS,
                inside: PathBuf::from(root::AGY_MEMORY[0]),
                whose: Whose::Databases(sessions.to_vec()),
            },
            Part {
                label: root::BRAIN,
                inside: PathBuf::from(root::AGY_MEMORY[1]),
                whose: Whose::Called(sessions.to_vec()),
            },
        ]
    }

    fn mirrored(&self) -> Vec<PathBuf> {
        vec![Path::new(sandbox::AGY_INSIDE_HOME).join(root::AGY_SETTINGS)]
    }

    fn mirroring(&self, account: &store::Account) -> Vec<Option<Vec<u8>>> {
        let settings = std::fs::read(account.directory().join(root::AGY_SETTINGS)).ok();

        vec![Some(root::agy_settings(settings.as_deref()))]
    }

    fn speaking(&self) -> Option<&Speaking> {
        Some(&crate::logins::AGY)
    }

    fn search(
        &self,
        account: &Path,
        memory: bool,
        home: &Home,
        session: &str,
        worktree: Option<&Path>,
        launched: SystemTime,
    ) -> Search {
        transcript::agy_search(account, memory, home, session, worktree, launched)
    }

    fn carried<'a>(
        &'a self,
        account: &'a Path,
        memory: bool,
        session: &'a str,
        worktree: &'a Path,
        home: &'a Home,
    ) -> Carrying<'a> {
        Box::pin(transcript::agy_carried(
            account, memory, session, worktree, home,
        ))
    }

    /// Gemini 3.8 Flash at two of its three thinking levels, high for the
    /// grilling and medium for the rest — see [`crate::pairing_defaults`].
    fn model(&self, role: store::Role) -> &str {
        match role {
            store::Role::Grilling => "gemini-3.8-flash-high",
            store::Role::Implementation | store::Role::Review => "gemini-3.8-flash-medium",
        }
    }

    /// What `agy models` lists to a signed-in account on agy 1.4.12, each named
    /// as agy names it: the Flash tiers first, the two [`Harness::model`]
    /// names among them, and the other vendors' models agy serves after.
    fn models(&self) -> Vec<(&str, &str)> {
        vec![
            ("gemini-3.8-flash-high", "Gemini 3.8 Flash (High)"),
            ("gemini-3.8-flash-medium", "Gemini 3.8 Flash (Medium)"),
            ("gemini-3.8-flash-low", "Gemini 3.8 Flash (Low)"),
            ("claude-opus-4-6-thinking", "Claude Opus 4.6 (Thinking)"),
            ("claude-sonnet-4-6", "Claude Sonnet 4.6 (Thinking)"),
            ("gpt-oss-120b-medium", "GPT-OSS 120B (Medium)"),
        ]
    }

    /// `run_command` turning the ask into a background task of its own after
    /// a wait, and waking the session when it ends.
    fn running(&self) -> &str {
        include_str!("running/agy.md")
    }
}

/// The session-only settings an Antigravity CLI session is launched with, as
/// the JSON `--settings` takes: what running unattended in a Worktree needs of
/// agy, said for this session and never saved into the account.
///
/// - `toolPermission: always-proceed`, so no tool call waits on an approval
///   nobody is there to give — the setting beside the flag that says the same.
/// - `allowNonWorkspaceAccess`, because the skills a session reads and the
///   handoff it writes to are outside the Worktree.
/// - `trustedWorkspaces`, the Worktree and every companion, so none of them
///   stops the session on a question about trusting it.
/// - `altScreenMode: never`, so agy draws inline and the Capture is the record
///   of what the session did, as `--no-alt-screen` is for codex and grok.
/// - `runningLightSpeed: off`, the light agy runs along its at-work line, which
///   would repaint the Screen for nothing every few frames.
/// - `enableTerminalSandbox: false`: agy's own sandbox for its shell tool,
///   nested inside bwrap, which is already the boundary.
///
/// The mode is left as agy's default — `auto` turns that same sandbox on — and
/// effort and telemetry are left as the account has them. Key names read off
/// agy 1.4.12.
fn settings<'a>(trusted: impl Iterator<Item = &'a Path>) -> String {
    serde_json::json!({
        "toolPermission": "always-proceed",
        "allowNonWorkspaceAccess": true,
        "trustedWorkspaces": trusted
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>(),
        "altScreenMode": "never",
        "runningLightSpeed": "off",
        "enableTerminalSandbox": false,
    })
    .to_string()
}
