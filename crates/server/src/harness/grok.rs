//! Grok Build.

use std::path::{Path, PathBuf};

use super::{Harness, Line, Resume, SignedOut};
use crate::platform::Platform;
use crate::store;

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
            model: "-m".to_owned(),
            prompt: None,
            names_the_session: true,
            resume: Some(Resume::Flag("--resume".to_owned())),
            tail: vec![
                "--always-approve".to_owned(),
                "--sandbox".to_owned(),
                "off".to_owned(),
                "--no-alt-screen".to_owned(),
            ],
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

    fn written_in_worktree(&self) -> Option<&str> {
        None
    }
}
