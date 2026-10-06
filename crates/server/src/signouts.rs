//! An Agent Profile's account signing out, and the stop that makes the login
//! answerable from a phone (ADR-0022).
//!
//! **The same shape as [`crate::limits`].** A session whose login expired or was
//! refused says so in a line and waits there for somebody at its terminal to
//! type `/login`. Nothing here types it: the run stops, the same as every other
//! stop — one Notice, *blocked on you*, the devices told — and the session is
//! ended, because a session left waiting would pick up again the moment
//! somebody logged in at it, inside a Conversation that reads as stopped. The
//! login is the account's business, and the Notice's Log in press is where it is
//! made: see [`crate::logins`].
//!
//! The same three records are read for the same reasons — what the session
//! printed, the frame that printing drew, and what its backend wrote in its own
//! log — and with the same latch: a display redraws its line for as long as it
//! waits, and one wait is one stop.
//!
//! **The phrase rule.** A session has signed out when one of its lines *ends*
//! with [`CLAUDE_SIGNED_OUT`], once the terminal's decoration is off both ends
//! of it. Claude says it as `<reason> · Please run /login`, the reason first and
//! the instruction last, so what has to be anchored is the end. Anchored, a line
//! that merely mentions the phrase — an agent reading ADR-0022 aloud, a grep of
//! this file — goes on: the phrase is in the middle of it, or a quotation mark
//! or a backtick comes after it, and ASCII punctuation is not decoration. See
//! [`says_so`].
//!
//! **Claude only.** The other harnesses log in differently, and none of them
//! has a login Verkstead can run yet — so they have no phrase, and a session on
//! one is never stopped here.

use sqlx::SqlitePool;

use crate::limits::{DECORATION, Held};
use crate::nudge::Nudges;
use crate::store;

/// What claude 2.1.283 ends a line with when its account's login is gone:
/// `Not logged in · Please run /login`, `Login expired · Please run /login`,
/// `OAuth token revoked · Please run /login`, and
/// `API Error: 401 Invalid API key · Please run /login`. Read off the binary.
///
/// Matched without regard to case. The reasons in front of it are claude's and
/// will move; the instruction is the part they all share.
const CLAUDE_SIGNED_OUT: &str = "please run /login";

/// The sentence a session of `agent_type` ends a line with when its account has
/// signed out — or nothing, where Verkstead cannot run that harness's login.
fn signed_out_phrase(agent_type: store::AgentType) -> Option<&'static str> {
    match agent_type {
        store::AgentType::Claude => Some(CLAUDE_SIGNED_OUT),
        store::AgentType::Codex | store::AgentType::Grok | store::AgentType::OpenCode => None,
    }
}

/// The line in `text` that says the account signed out, tidied of the
/// terminal's own sequences — or `None`.
fn saying(text: &str, phrase: Option<&str>) -> Option<String> {
    let phrase = phrase?;

    text.lines()
        .map(crate::capture::plain)
        .find(|line| says_so(line, phrase))
}

/// Whether one tidied line *says* the account signed out, rather than
/// mentioning the phrase.
///
/// The phrase has to end the line once the decoration is off the back — the
/// box border, the spaces — and there has to be something in front of it once
/// the decoration is off the front: claude always gives a reason, and a line
/// that is the phrase and nothing else is somebody typing it.
fn says_so(line: &str, phrase: &str) -> bool {
    let line = line
        .trim_start_matches(DECORATION)
        .trim_end_matches(DECORATION);

    let Some(reason) = line.len().checked_sub(phrase.len()) else {
        return false;
    };

    line.is_char_boundary(reason)
        && reason > 0
        && line[reason..].eq_ignore_ascii_case(phrase)
        && line[..reason].ends_with(char::is_whitespace)
}

/// Where the Profile a session runs under is at home, which decides what the
/// Notice offers: a Log in press here, or the name of the device to log in on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Home {
    /// At home on this device, so its login is made here.
    Here,

    /// A mirror, at home on the device with this Device Id.
    Away(String),
}

/// One session, watched for the line that says its account signed out.
///
/// Held by the relay beside [`crate::limits::Watch`] and fed the same records.
pub(crate) struct Watch {
    conversation_id: i64,
    event_id: i64,

    /// The Profile this session runs under, by its id on this device: what the
    /// stop records a login as waiting on.
    profile_id: i64,

    /// And what it is called, which is what the Notice and the push name.
    profile: String,

    /// And where it is at home.
    home: Home,

    /// The phrase, off the agent the Profile runs — see [`signed_out_phrase`].
    phrase: Option<&'static str>,

    printed: Held,

    /// Whether the line on screen now has already been stopped on — see
    /// [`crate::limits::Watch`], whose latch this is.
    raised: bool,
}

impl Watch {
    /// Watch the session printing into `event_id`, run under the Profile
    /// `profile_id`, called `profile`, which runs `agent_type`.
    pub(crate) fn on(
        conversation_id: i64,
        event_id: i64,
        profile_id: i64,
        profile: String,
        home: Home,
        agent_type: store::AgentType,
    ) -> Watch {
        Watch {
            conversation_id,
            event_id,
            profile_id,
            profile,
            home,
            phrase: signed_out_phrase(agent_type),
            printed: Held::default(),
            raised: false,
        }
    }

    /// Take a chunk of what the session printed.
    pub(crate) fn printed(&mut self, text: &str) {
        self.printed.push(text);
    }

    /// The line in any of the three records that says the account signed out.
    fn found(&self, drawn: &str, said: Option<&str>) -> Option<String> {
        saying(self.printed.text(), self.phrase)
            .or_else(|| saying(drawn, self.phrase))
            .or_else(|| saying(said.unwrap_or_default(), self.phrase))
    }

    /// Look at what has arrived, and stop the run if it says the account signed
    /// out.
    ///
    /// `true` is *this session is to end*, for [`crate::limits::Watch::look`]'s
    /// reason: the relay is what ends it.
    #[must_use = "a stop for a sign-out ends the session it stopped"]
    pub(crate) async fn look(
        &mut self,
        pool: &SqlitePool,
        nudges: &Nudges,
        drawn: &str,
        said: Option<&str>,
    ) -> bool {
        let printed_something = !self.printed.text().is_empty();

        let found = self.found(drawn, said);

        self.printed.looked();

        let Some(found) = found else {
            if printed_something {
                self.raised = false;
            }

            return false;
        };

        if self.raised {
            return false;
        }

        self.raised = true;

        signed_out(pool, nudges, self, &found).await
    }
}

/// Stop the run: write the stop and its Notice, tell the devices, and say that
/// the session is to end. `true` on every way of returning but a stop that
/// could not be written — see [`crate::limits`], whose rule this is.
async fn signed_out(pool: &SqlitePool, nudges: &Nudges, watch: &Watch, said: &str) -> bool {
    let conversation_id = watch.conversation_id;

    let lifecycle = match store::load_conversation(pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state,
        Ok(None) => return false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation whose account signed out failed");
            return false;
        }
    };

    let home = match &watch.home {
        Home::Here => None,
        Home::Away(device) => Some(home_named(pool, device).await),
    };

    let stopped = crate::stopping::stop(
        pool,
        nudges,
        conversation_id,
        crate::stopping::Decided::SignedOut {
            id: watch.profile_id,
            profile: &watch.profile,
        },
        crate::stalls::driving(lifecycle),
        &crate::stopping::signed_out(&watch.profile, said, home.as_deref()),
        Some(watch.event_id),
    )
    .await;

    match stopped {
        Ok(None) => {
            tracing::info!(
                conversation_id,
                session = watch.event_id,
                "an account signed out on a run that had already stopped"
            );
            true
        }
        Ok(Some(notice)) => {
            tracing::warn!(
                conversation_id,
                notice,
                session = watch.event_id,
                profile = watch.profile,
                "an account signed out, so the run has stopped"
            );
            true
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "a run could not be stopped on a sign-out");
            false
        }
    }
}

/// What the device with `device` for its Device Id is shown under, or words
/// that stand in for it where this device has never heard of it.
async fn home_named(pool: &SqlitePool, device: &str) -> String {
    match store::members(pool).await {
        Ok(members) => members
            .into_iter()
            .find(|member| member.device == device)
            .map(|member| member.name),
        Err(error) => {
            tracing::error!(error = ?error, device, "reading the device a Profile is at home on failed");
            None
        }
    }
    .unwrap_or_else(|| "the device it is at home on".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    use store::AgentType::{Claude, Codex, Grok, OpenCode};

    fn signed_out(text: &str) -> Option<String> {
        saying(text, signed_out_phrase(Claude))
    }

    /// The lines claude 2.1.283 ends with the phrase, as it draws them.
    #[test]
    fn the_lines_claude_draws_when_its_login_is_gone_are_recognised() {
        for line in [
            "Not logged in · Please run /login",
            "Login expired · Please run /login",
            "OAuth token revoked · Please run /login",
            "API Error: 401 Invalid API key · Please run /login",
            "  ⎿  Login expired · Please run /login",
            "│ Not logged in · Please run /login │",
            "\u{1b}[31mLogin expired · Please run /login\u{1b}[0m",
        ] {
            assert!(signed_out(line).is_some(), "{line:?} was not recognised");
        }
    }

    /// The line is kept as it was printed, tidied of the terminal's sequences.
    #[test]
    fn the_line_is_kept_as_it_was_printed() {
        assert_eq!(
            signed_out("\u{1b}[2m⎿\u{1b}[0m  Login expired · Please run /login\n"),
            Some("⎿  Login expired · Please run /login".to_owned()),
        );
    }

    /// The phrase in the middle of a line is a mention, and so is a line that
    /// quotes it: the end is anchored, and ASCII punctuation is not decoration.
    #[test]
    fn a_line_that_mentions_the_phrase_is_not_a_sign_out() {
        for line in [
            "Please run /login · Your session has expired",
            "Your session has expired. Please run /login to sign in again.",
            "A session has signed out when a line ends with `Please run /login`.",
            r#"const CLAUDE: &str = "Login expired · Please run /login";"#,
            r#"            "Login expired · Please run /login","#,
            "Claude says it as `<reason> · Please run /login`, once terminal",
            "- Login expired · Please run /login.",
            "Please run /login",
            "  │ Please run /login",
            "Please run /loginnow",
            "",
        ] {
            assert_eq!(signed_out(line), None, "{line:?} stopped a run");
        }
    }

    /// Only Claude's sessions are read: the others have no login Verkstead can
    /// run.
    #[test]
    fn only_a_claude_session_is_read() {
        for agent_type in [Codex, Grok, OpenCode] {
            assert_eq!(
                saying(
                    "Login expired · Please run /login",
                    signed_out_phrase(agent_type)
                ),
                None,
            );
        }
    }

    /// A line split across two chunks is still one line.
    #[test]
    fn a_line_split_across_two_chunks_is_still_one_line() {
        let mut watch = Watch::on(1, 2, 3, "fable".to_owned(), Home::Here, Claude);

        watch.printed("Login expired · Please r");
        watch.printed.looked();
        assert_eq!(watch.found("", None), None, "half a line is not one");

        watch.printed("un /login\n");
        assert!(watch.found("", None).is_some());
    }

    /// And the frame and the log are read as well as the terminal.
    #[test]
    fn the_frame_and_the_log_are_read_too() {
        let watch = Watch::on(1, 2, 3, "fable".to_owned(), Home::Here, Claude);
        let line = "Not logged in · Please run /login";

        assert!(watch.found(line, None).is_some(), "the frame");
        assert!(watch.found("", Some(line)).is_some(), "the log");
    }
}
