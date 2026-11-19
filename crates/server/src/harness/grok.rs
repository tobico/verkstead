//! Grok Build.

use std::path::{Path, PathBuf};

use std::time::SystemTime;

use verkstead_render::Broken;

use super::{
    Carrying, Declared, HOME_REFUSED, Harness, Line, MODEL, PROMPT, Resume, SESSION, SignedOut,
    said,
};
use crate::platform::Platform;
use crate::sandbox::root::{self, Part, Place, Whose};
use crate::sandbox::{self, Home};
use crate::settings::AttachedServer;
use crate::store;
use crate::transcript::{self, Search};

/// The program a Grok Build session is run as.
pub(crate) const BINARY: &str = "grok";

/// The title grok 1.0.13 puts on the card it draws a free account that has
/// run out, the rest of the card being three tiers to upgrade to.
///
/// **The card rather than the sentence, because the card is what a session
/// sees.** Run out of window, grok retries the refused turn, gives up, and
/// draws a bordered panel headed by this — on the frame, which is why the frame
/// is read: see [`crate::limits`]. The sentence `You’ve reached your free Grok
/// Build usage limit for now.` is grok's too and is *not* this: that is what its
/// headless mode prints, and Verkstead launches the interactive one. Both read
/// off grok 1.0.13 driven until it drew each of them.
///
/// A free account's card is the one that has been seen. A paid plan's is headed
/// `You hit your weekly limit.` in the same binary and has not been watched
/// drawn, and one phrase per backend cannot open both, so a paid stop lands as
/// the ordinary stall until this build holds more than one wording per backend.
const EXHAUSTED: &str = "you hit your free usage limit";

/// What grok has on its Screen while it is working, and nothing of what it has
/// there when it is waiting for a human — see [`crate::sessions::Signature::AtWork`].
///
/// Read off grok 1.0.13 driven on a hundred-column terminal rather than guessed
/// at, and it comes out where codex came out: the frame grok leaves when its
/// turn is over and the frame it draws mid-turn are the same screen but for the
/// live status line — `⠧ Responding… 5.7s … [stop]` — and this hint on the row
/// under the composer. The composer itself, its `❯`, the `grok-4.6 ·
/// always-approve` label on its border and the `Shift+Tab:mode` and
/// `Ctrl+x:shortcuts` hints beside this one stand in both, so none of them says
/// whether the session has stopped.
///
/// The hint rather than the `[stop]` chip, which goes and comes with it: the
/// hints are the row grok draws at the foot of every frame, where the status
/// line is there only while a turn runs, and a keybinding label is a harder
/// thing to find by accident in what the session printed than a bracketed word
/// is.
///
/// The fragment rather than the whole row, because the rest of it moves while
/// this does not: a turn that has backgrounded something adds `Ctrl+b:send to
/// bg` beside it, and the hints at rest change with what the composer is
/// offering.
///
/// Named for the same reason [`super::codex::AT_WORK`] is, and it is the same bargain
/// the usage-limit phrase makes: the wording is grok's and it will move, and
/// moving it costs one edit here.
const AT_WORK: &str = "Esc:cancel";

/// Grok Build.
pub(crate) struct Grok;

impl Harness for Grok {
    fn binary(&self) -> &str {
        BINARY
    }

    fn named(&self) -> &str {
        "Grok Build"
    }

    fn mark(&self) -> &str {
        include_str!("marks/grok.svg")
    }

    fn paths(&self) -> Vec<Declared<'_>> {
        vec![Declared {
            key: store::HOME,
            label: "Home directory",
            purpose: "Mounted at `~/.grok`.",
            placeholder: "/home/you/accounts/work/.grok",
            file: false,
            refused: HOME_REFUSED,
            within: &[],
        }]
    }

    fn found_at(&self) -> Vec<&str> {
        vec!["~/.grok"]
    }

    /// **Its resume is Claude's shape under Claude's spelling**, and the exclusion is
    /// the backend's own rather than Verkstead's reading of it: grok 1.0.34 says of
    /// `--session-id` that it names a **new** conversation, that it must not already
    /// exist, and that with `--resume` it is only valid alongside `--fork-session` —
    /// which a conversation carried on is the opposite of. So `--resume <session-id>`
    /// stands where the name would have been, the prompt stays positional, and a
    /// UUID-shaped value is always read as an id rather than matched against session
    /// titles. Its log is proved to be there for Claude's reason and one more: grok
    /// resolves a resume against the directory it was started in, so the log has to have
    /// been moved under this device's own name for this Worktree first — see
    /// [`crate::transcript::relocated`], and the one left on [`Line::resume`]'s `None`.
    ///
    /// **Grok Build is the one backend after Claude that takes the session id.** It
    /// takes it under the spelling [`crate::sessions::Agents`] writes, it insists on a valid
    /// UUID and it refuses one it already has a session for — all three of which
    /// Verkstead's own names satisfy, being version-4 UUIDs drawn fresh per session.
    /// So its log is named at launch rather than found afterwards, which is
    /// Claude's shape and the opposite of Codex's. Its own sandbox is off for the
    /// reason codex's is, and nothing about its account is said on the line: grok
    /// reads its whole configuration out of the home the Profile named.
    fn line(&self, _worktree: Option<&Path>, _companions: &[PathBuf]) -> Line {
        Line {
            model: said(&["-m", MODEL]),
            prompt: said(&[PROMPT]),
            session: Some(said(&["--session-id", SESSION])),
            resume: Some(Resume::Flag(said(&["--resume", SESSION]))),
            tail: said(&["--always-approve", "--sandbox", "off", "--no-alt-screen"]),
            worktree: Vec::new(),
        }
    }

    fn at_work(&self) -> Option<&str> {
        Some(AT_WORK)
    }

    fn exhausted(&self) -> Option<&str> {
        Some(EXHAUSTED)
    }

    /// None: Verkstead cannot run a Grok Build login.
    fn signed_out(&self) -> Option<SignedOut<'_>> {
        None
    }

    fn channel(&self) -> store::Channel {
        store::Channel::StoreAndNudge
    }

    /// `<server>__<tool>`, read off Grok Build 1.0.34: the prefix dropped, which
    /// is the name its `use_tool` takes.
    fn tools_named(&self) -> &str {
        "<server>__<tool>"
    }

    fn environment(&self, _platform: Platform) -> Vec<(&str, &str)> {
        Vec::new()
    }

    fn kept_in(&self, home: &Path) -> store::Account {
        store::Account::home(store::AgentType::GROK, home.join(sandbox::GROK_INSIDE_HOME))
    }

    fn account_inside(&self, account: &store::Account, home: &Path) -> Vec<(PathBuf, PathBuf)> {
        vec![(
            account.directory().to_owned(),
            home.join(sandbox::GROK_INSIDE_HOME),
        )]
    }

    fn judged(&self, account: &store::Account) -> Vec<(PathBuf, Broken)> {
        vec![(account.directory().to_owned(), Broken::HomeMissing)]
    }

    fn root(&self) -> &str {
        sandbox::GROK_INSIDE_HOME
    }

    fn login(&self) -> &str {
        root::AUTH
    }

    fn configuration(&self) -> Vec<PathBuf> {
        vec![PathBuf::from(root::GROK_CONFIG)]
    }

    /// A `config.toml`, carrying the account's own keys that reach a model
    /// and the Conversation's servers — see [`root::toml_carrying`].
    fn configured(
        &self,
        found: Vec<Option<Vec<u8>>>,
        servers: &[AttachedServer],
    ) -> Vec<(PathBuf, Vec<u8>)> {
        let [config] = <[_; 1]>::try_from(found).expect("one file asked for");

        vec![(
            PathBuf::from(root::GROK_CONFIG),
            root::toml_carrying(
                config
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .as_deref(),
                &root::GROK_CARRIED,
                servers,
                root::HEADERS,
            ),
        )]
    }

    fn instructions(&self) -> PathBuf {
        PathBuf::from(root::AGENTS)
    }

    fn memory(&self, _place: Option<&Place>) -> Vec<PathBuf> {
        root::GROK_MEMORY.iter().map(PathBuf::from).collect()
    }

    fn unshared(&self) -> Vec<&str> {
        root::GROK_MEMORY.to_vec()
    }

    fn synced(
        &self,
        _place: Option<&Place>,
        _worktree: Option<&str>,
        sessions: &[String],
    ) -> Vec<Part> {
        vec![
            Part {
                label: root::MEMORY.into(),
                inside: PathBuf::from(root::GROK_MEMORY[1]),
                whose: Whose::Everything,
            },
            Part {
                label: root::SESSIONS_PART.into(),
                inside: PathBuf::from(root::SESSIONS),
                whose: Whose::Called(sessions.to_vec()),
            },
        ]
    }

    fn mirrored(&self) -> Vec<PathBuf> {
        vec![Path::new(sandbox::GROK_INSIDE_HOME).join(root::GROK_CONFIG)]
    }

    fn mirroring(&self, account: &store::Account) -> Vec<Option<Vec<u8>>> {
        let config = std::fs::read_to_string(account.directory().join(root::GROK_CONFIG)).ok();

        vec![Some(root::toml_carrying(
            config.as_deref(),
            &root::GROK_CARRIED,
            &[],
            root::HEADERS,
        ))]
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
        transcript::grok_search(account, memory, home, session, worktree, launched)
    }

    fn carried<'a>(
        &'a self,
        account: &'a Path,
        memory: bool,
        session: &'a str,
        worktree: &'a Path,
        home: &'a Home,
    ) -> Carrying<'a> {
        Box::pin(transcript::grok_carried(
            account, memory, session, worktree, home,
        ))
    }

    /// On Linux: grok saves its login by renaming a file over it, which a
    /// bind refuses — see [`root::Root::login_copied`].
    fn login_copied(&self, platform: Platform) -> bool {
        platform == Platform::Linux
    }

    fn model(&self, _role: store::Role) -> &str {
        "grok-4.6"
    }

    fn models(&self) -> Vec<(&str, &str)> {
        vec![("grok-4.6", "Grok 4.6"), ("grok-4.5", "Grok 4.5")]
    }

    /// Codex's, the other that stores and nudges.
    fn running(&self) -> &str {
        include_str!("running/store-and-nudge.md")
    }
}
