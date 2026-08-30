//! An Agent Profile's account running out of window, and the stop that makes
//! the wait answerable from a phone.
//!
//! **One stop, the same as every other.** A run whose account is out of window
//! is a run that has stopped — one Notice, one *blocked on you*, one Resume —
//! and the only thing that tells it apart is the words it carries about when
//! the account comes back. See [`crate::stopping`], which writes it.
//!
//! **The agent waits too, and the stop ends its session.** The claude in use
//! holds its session when the limit lands and carries on by itself when the
//! window comes back, under a setting of its own. Nothing here reaches into that
//! configuration — but nothing waits for it either, and that is what makes the
//! session go: no stop resumes itself, so an agent left holding would wake at
//! the reset and work on inside a Conversation that reads as stopped, and the
//! press that came the next morning would launch over whatever it had done.
//! What the stop costs, then, is the window the session would have worked
//! through unwatched; what it buys is one rule about stopping.
//!
//! The stop is written first and the ending follows it, which is the order a
//! Force stop uses and for the same reason: a session Verkstead ended advances
//! nothing, so the driver seeing it out goes straight to its next launch, and
//! the stop has to be there when it looks. And the ending is the relay's rather
//! than this module's, because the watcher runs inside the relay task of the
//! very session it is ending — see [`Watch::look`].
//!
//! What the stop adds beyond that is that the wait is *said*: the Timeline names
//! the account that ran out and the words it printed about coming back, the
//! devices are told, and there is a press to start again — instead of a session
//! that has gone quiet for no stated reason.
//!
//! **Recognition is one sentence per backend, said once.** [`exhausted_phrase`]
//! is the whole of what is matched, because the wording is the backend's and will
//! move: claude 2.1.234 draws it as `Usage limit reached · continuing
//! automatically at 3pm · esc to cancel`, and codex 0.149.0 opens with `You've
//! hit your usage limit` and decorates what follows by plan. Every part of either
//! but the phrase itself is decoration this build has no business depending on.
//! It is read off what the session leaves behind — the Capture of what it printed
//! and the Transcript its backend wrote — rather than out of anything the agent
//! is asked.
//!
//! One phrase apiece, kept in the one place, so a wording that moves is one edit
//! rather than a search — the same bargain the idle signature makes, and for the
//! same reason. Which phrase a session is read against is a fact about its
//! Profile's agent type, which is what the Watch is told when it is made.
//!
//! A line that *says* it rather than one that mentions it: the phrase has to open
//! the line, once the terminal's own decoration is off the front of it — and
//! decoration is what a terminal draws with rather than every character that is
//! not a letter, which is what keeps a session reading this very file from
//! pausing itself. See [`says_so`]. What it cannot rule out is an agent that
//! opens a line with the phrase and nothing in front of it, for reasons of its
//! own — and that is the cheap failure of the two, being one press to undo, where
//! a limit nobody noticed is a run that goes quiet for five hours.
//!
//! **Nothing here switches accounts.** An exhausted Profile is a wait, never a
//! reason to spend a different one: no Conversation moves to another Profile
//! because the one it is on ran out, and there is no code here that could.
//!
//! **Nothing here touches the Worktree.** The stop holds Verkstead off advancing
//! and does nothing else, so a run picked up again finds the repository exactly
//! as the session left it — the same promise every other stop makes.

use sqlx::SqlitePool;

use crate::nudge::Nudges;
use crate::store;

/// The sentence a session of `agent_type` prints when its account is out of
/// window.
///
/// Matched without regard to case and to nothing around it. One phrase per
/// backend is the whole of the coupling to somebody else's display, and they are
/// here together so that a wording that moves is one edit rather than a search.
///
/// A backend whose phrase this build has never seen would take an arm of its own
/// here; until it has one, its limit lands as the ordinary stall it would have
/// been anyway (ADR-0011).
fn exhausted_phrase(agent_type: store::AgentType) -> &'static str {
    match agent_type {
        store::AgentType::Claude => CLAUDE_EXHAUSTED,
        store::AgentType::Codex => CODEX_EXHAUSTED,
    }
}

/// What claude 2.1.234 opens its banner with, the rest of the line being the
/// reset words and the key to press.
const CLAUDE_EXHAUSTED: &str = "usage limit reached";

/// And what codex 0.149.0 opens its own with.
///
/// The stable prefix and no more: what follows it is the plan's — an upgrade to
/// Plus, an upgrade to Pro, credits to buy, an admin to ask — and every one of
/// those sentences carries this one in front of it. Read off the binary rather
/// than guessed at.
///
/// The apostrophe is the ASCII one codex writes, which is what keeps [`says_so`]
/// honest about it: a line of prose quoting the phrase opens with a quotation
/// mark, and ASCII punctuation is not decoration there.
const CODEX_EXHAUSTED: &str = "you've hit your usage limit";

/// How much of a session's printing is held between looks.
///
/// Only a guard. What is kept between one look and the next is the line being
/// printed now, and a session that has printed this much without a newline is
/// not one whose earliest bytes hold the sentence.
const HELD: usize = 8 * 1024;

/// What a session said about its account being out of window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exhausted {
    /// The line that said so, as it was printed and tidied of the terminal's own
    /// sequences.
    ///
    /// Kept rather than reduced to the phrase that matched it, because it is the
    /// record: somebody reading the stop a week later is reading the backend's
    /// own sentence rather than this build's opinion of it.
    pub(crate) said: String,
}

/// The sentence in `text` that says an account of `agent_type` is out of window,
/// or `None`.
///
/// The agent type rather than nothing, because the phrase is the backend's: a
/// session is read against the wording its own backend draws and against no
/// other, so a line one backend would stop on is a line another prints in the
/// middle of its work.
///
/// Pure and cheap on purpose: this runs on every flush of every session, twice a
/// second while an agent talks, so everything the stop needs beyond *whether* —
/// the words the reset is in — is worked out afterwards and only on a hit.
pub(crate) fn exhausted(text: &str, agent_type: store::AgentType) -> Option<Exhausted> {
    let phrase = exhausted_phrase(agent_type);

    text.lines()
        .map(crate::capture::plain)
        .find(|line| says_so(line, phrase))
        .map(|said| Exhausted { said })
}

/// Whether one tidied line *says* an account is out of window in `phrase`, rather
/// than mentioning that such a thing exists.
///
/// The phrase has to open the line once the decoration is off the front — the box
/// border, the spinner, the bullet, the spaces. That is the difference between a
/// status line the agent's display drew and a sentence about limits inside the
/// work.
///
/// **Decoration is what a terminal draws with**, which is the narrower half of
/// [`DECORATION`]: whitespace, and the symbols outside ASCII that a display
/// reaches for. ASCII punctuation is not decoration, because it is what *code*
/// opens a line with — a quotation mark, a backtick, a dash, a hash. Verkstead
/// builds this repository, so a session grepping the file this matcher lives in
/// prints a dozen lines that quote either phrase, and reading one of those as its
/// own account running out would stop the run it was in the middle of.
fn says_so(line: &str, phrase: &str) -> bool {
    let said: String = line
        .trim_start_matches(DECORATION)
        .chars()
        .take(phrase.chars().count())
        .collect();

    said.eq_ignore_ascii_case(phrase)
}

/// What a terminal puts in front of a status line, and nothing else.
///
/// Whitespace and the non-ASCII symbols a display draws with — the box borders,
/// the bullets, the spinner glyphs. Deliberately not every non-alphanumeric
/// character: see [`says_so`] for what the wider rule cost.
fn decoration(character: char) -> bool {
    character.is_whitespace() || (!character.is_ascii() && !character.is_alphanumeric())
}

/// The same as a pattern, so the trim above reads as what it does.
const DECORATION: fn(char) -> bool = decoration;

/// When the window resets, in the words the sentence said it in — or `None`
/// where it named no such thing.
///
/// Words to show and nothing else: the reset time is information beside the
/// Resume button, not a moment anything acts on, so what is kept is the word
/// the display drew rather than this build's reading of it. `3pm` stays `3pm`,
/// which is what somebody sitting at that machine will look at their own clock
/// for.
///
/// Two shapes are recognised, which are the two a display writes:
///
/// - a clock time — `3pm`, `3:30pm`, `15:00` — which is what claude 2.1.234
///   actually prints;
/// - an instant written whole, which is what a record written by a machine for
///   a machine looks like.
///
/// The first word of either shape, decoration trimmed off both ends. Pure and
/// cheap, because it is read on the flush that found the sentence.
fn resets(said: &str) -> Option<String> {
    said.split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_alphanumeric()))
        .find(|word| written_whole(word) || clock(word))
        .map(str::to_owned)
}

/// Whether one word is an instant written whole: a date, `T`, and a time of day
/// under it.
///
/// By shape rather than by parsing, deliberately: nothing here reads a reset
/// time as a moment, and a word only has to be recognisable as one to be worth
/// showing. What comes after the minutes — seconds, a fraction, a zone — is the
/// instant's own business.
fn written_whole(word: &str) -> bool {
    let Some((date, time)) = word.split_once(['T', 't']) else {
        return false;
    };

    let dated = matches!(date.split('-').collect::<Vec<&str>>()[..], [year, month, day]
        if digits(year, 4) && digits(month, 2) && digits(day, 2));

    let Some((hour, rest)) = time.split_once(':') else {
        return false;
    };

    let minute: String = rest.chars().take(2).collect();

    dated && digits(hour, 2) && digits(&minute, 2)
}

/// And whether one word is a time of day.
///
/// The three ways a display writes one: `3pm`, `3:30pm` and `15:00` — with the
/// twelve-hour ones taking their meridiem attached, because that is how they
/// are drawn. A word ending in a meridiem it cannot use, or naming an hour
/// there is none of, is not a time.
///
/// A bare number is never one. It is a number — a version, a count of files,
/// the 40 in "40 paths" — and showing one as a reset would tell the human a
/// time nobody said anything about.
fn clock(word: &str) -> bool {
    let lowered = word.to_ascii_lowercase();

    let (figures, meridiem) = match lowered.strip_suffix("am") {
        Some(digits) => (digits, true),
        None => match lowered.strip_suffix("pm") {
            Some(digits) => (digits, true),
            None => (lowered.as_str(), false),
        },
    };

    let Some((hour, minute)) = figures.split_once(':').map(|(hour, minute)| {
        (
            hour,
            minute.parse::<u8>().ok().filter(|minute| *minute < 60),
        )
    }) else {
        // Only with a meridiem on it, and then only an hour of the twelve.
        return meridiem && matches!(figures.parse::<u8>(), Ok(hour) if hour <= 12);
    };

    minute.is_some() && matches!(hour.parse::<u8>(), Ok(hour) if hour <= 23)
}

/// Whether a run of characters is exactly `count` ASCII digits.
fn digits(said: &str, count: usize) -> bool {
    said.len() == count && said.bytes().all(|byte| byte.is_ascii_digit())
}

/// One session, watched for the sentence that says its account is out of window.
///
/// Held by the relay and fed everything the session leaves behind — what it
/// printed and what its backend wrote down — because both arrive there and
/// nowhere else. See [`crate::sessions`], where it is fed.
pub(crate) struct Watch {
    conversation_id: i64,
    event_id: i64,

    /// What the Agent Profile this session runs under is called, kept because
    /// that is what the stop names — see [`crate::stopping::Decided`].
    profile: String,

    /// And which agent that Profile runs, kept because that is what says which
    /// sentence this session is read against — see [`exhausted_phrase`]. Taken
    /// with the name and for the same reason: the Profile a session was launched
    /// under is the one it is watched as, whatever is edited while it runs.
    agent_type: store::AgentType,
    /// What the session has printed since the last look, less whatever has
    /// already been looked at.
    ///
    /// Held across looks so that a sentence arriving in two chunks is still one
    /// sentence: a read off a terminal lands wherever the kernel put it, and the
    /// phrase falling across the boundary is the ordinary case rather than the
    /// strange one.
    printed: String,

    /// Whether the banner on screen now has already been stopped on.
    ///
    /// A latch, because the display redraws its banner for as long as the wait
    /// lasts and this is looked at twice a second: without one, five hours of
    /// waiting would be five hours of reading the reset time — a parse, a
    /// question to the machine about its offset, a transaction and a log line,
    /// all to be told the run has already stopped.
    ///
    /// Latched on *whether* rather than on the sentence, because the sentence is
    /// not stable: what is kept is the line as the terminal drew it, decoration
    /// and all, and a display that animates the glyph in front of its banner
    /// writes a different string every frame.
    ///
    /// Let go when the session prints something that is *not* the banner, which
    /// is it having moved on: an account that runs out again after that is a new
    /// wait and gets a stop of its own. Not when a look finds nothing, which is
    /// a look landing between two repaints.
    ///
    /// And never on the wait ending, which is the case that decides the shape of
    /// this. The human saying *go on without waiting* leaves the agent seeing
    /// its own limit out with the banner still up — that is what a stop never
    /// ends — so a latch let go there would read the next repaint as a fresh
    /// limit and stop the run again a half-second after they started it.
    raised: bool,
}

impl Watch {
    /// Watch the session printing into `event_id`, run under `profile`, which
    /// runs `agent_type`.
    pub(crate) fn on(
        conversation_id: i64,
        event_id: i64,
        profile: String,
        agent_type: store::AgentType,
    ) -> Watch {
        Watch {
            conversation_id,
            event_id,
            profile,
            agent_type,
            printed: String::new(),
            raised: false,
        }
    }
    /// Take a chunk of what the session printed.
    ///
    /// Cheap on purpose: this is called with everything that comes off the
    /// terminal, and the reading happens at the flush.
    pub(crate) fn printed(&mut self, text: &str) {
        self.printed.push_str(text);
    }

    /// Look at what has arrived, and stop the run if it says the account is out
    /// of window.
    ///
    /// `said` is the last thing the agent wrote in its own log, which is the other
    /// record a session leaves behind — see [`crate::transcript`]. It is looked at
    /// beside the terminal rather than instead of it: a backend that says so in
    /// its log and not on its display would otherwise go unnoticed.
    ///
    /// `true` is *this session is to end*, which is what a stop having just been
    /// written comes to. The relay is what ends it, because the relay is what
    /// this is running inside: ending a session the ordinary way waits for that
    /// relay to be over, and a call to it here would be waiting on itself. See
    /// the module head for why the session goes at all.
    #[must_use = "a stop for an exhausted window ends the session it stopped"]
    pub(crate) async fn look(
        &mut self,
        pool: &SqlitePool,
        nudges: &Nudges,
        said: Option<&str>,
    ) -> bool {
        // Whether the terminal said anything at all since the last look, which is
        // what tells a session going on from a session standing still — see
        // [`Watch::raised`], which is let go on the first of those and never on
        // the second. The terminal alone: the log's last line is the same last
        // line between repaints, so a look that read it as news would read news
        // twice a second for ever.
        let printed_something = !self.printed.is_empty();

        let found = exhausted(&self.printed, self.agent_type)
            .or_else(|| exhausted(said.unwrap_or_default(), self.agent_type));

        // Everything up to the last newline has been looked at now. What is kept
        // is the line still being printed, which is where the next chunk carries
        // on from — bounded, because a session that has printed this much
        // without one is not one whose earliest bytes hold the sentence.
        let printed = self.printed.len();

        let kept = match self.printed.rfind('\n') {
            Some(ends) => ends + 1,
            None => printed.saturating_sub(HELD),
        };

        // On a character boundary, because what is being held is text.
        if let Some(from) = self
            .printed
            .char_indices()
            .map(|(at, _)| at)
            .find(|at| *at >= kept)
        {
            self.printed.drain(..from);
        } else if kept > 0 {
            self.printed.clear();
        }

        let Some(found) = found else {
            // The session printed something that was not the banner, so it has
            // moved on and whatever it says next about its account is a new
            // wait. A look that read nothing is a look landing between two
            // repaints — the banner is still up, and the latch holds.
            if printed_something {
                self.raised = false;
            }

            return false;
        };

        if self.raised {
            return false;
        }

        self.raised = true;

        out_of_window(
            pool,
            nudges,
            self.conversation_id,
            self.event_id,
            &self.profile,
            &found.said,
        )
        .await
    }
}
/// Stop the run: write the stop, its Notice and the reset words, tell the
/// human's devices, and say that the session is to end.
///
/// The one stop and nothing of its own — see [`crate::stopping::stop`]. What is
/// particular to a window is only what the stop is told: the account that ran
/// out, the line the session printed, and when the window comes back. So a
/// Conversation stopped here reads as one stopped by anything else — one Notice,
/// one badge, one Resume — with the reset words the only thing telling it apart.
///
/// The pool and the Nudges, because that is all this half of the server has:
/// the watcher runs inside the relay task of the session that printed the line.
///
/// `true` is *end the session*, which is every way of this returning but one:
/// a Conversation that reads as stopped must have nothing running behind it,
/// whether this stop is the one that stopped it or the second to arrive. The
/// exception is a stop that could not be written at all — nothing is refused
/// for here, but ending a session over a stop that is not on the record would
/// be a run advancing with nothing to hold it off, so the log is where that
/// goes and the session is left alone.
async fn out_of_window(
    pool: &SqlitePool,
    nudges: &Nudges,
    conversation_id: i64,
    session: i64,
    profile: &str,
    said: &str,
) -> bool {
    // Only now, and only once per stop: the flush this was reached from runs
    // twice a second, and the words the reset is in are worth a look at the
    // sentence only once the sentence has said something.
    let resets = resets(said);

    // What ought to have been happening, in the words every other stop names it
    // in: the Notice opens with it, and a stop for a window is not a different
    // kind of sentence.
    let lifecycle = match store::load_conversation(pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state,
        Ok(None) => return false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation whose account ran out failed");
            return false;
        }
    };

    let stopped = crate::stopping::stop(
        pool,
        nudges,
        conversation_id,
        crate::stopping::Decided::OutOfWindow {
            profile,
            resets: resets.as_deref(),
        },
        crate::stalls::driving(lifecycle),
        &crate::stopping::out_of_window(profile, said),
        Some(session),
    )
    .await;

    match stopped {
        // A run that is already stopped, which is what the banner redrawing in
        // different words comes to. The first Notice is the one the human was
        // told about.
        Ok(None) => {
            tracing::info!(
                conversation_id,
                session,
                "an account ran out of window on a run that had already stopped"
            );
            true
        }
        Ok(Some(notice)) => {
            tracing::warn!(
                conversation_id,
                notice,
                session,
                profile,
                resets = resets.as_deref().unwrap_or("unread"),
                "an account ran out of window, so the run has stopped"
            );
            true
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "a run could not be stopped on a usage limit");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use store::AgentType::{Claude, Codex};

    /// The line claude 2.1.234 draws, decoration and all: the phrase opens it
    /// once the terminal's own marks are off the front.
    #[test]
    fn the_line_a_session_draws_when_its_account_runs_out_is_recognised() {
        let printed = "\u{1b}[2m✻\u{1b}[0m Usage limit reached · continuing automatically at 3pm · esc to cancel\n";

        assert_eq!(
            exhausted(printed, Claude).map(|found| found.said),
            Some(
                "✻ Usage limit reached · continuing automatically at 3pm · esc to cancel"
                    .to_owned()
            ),
        );
    }

    /// And the lines codex 0.149.0 draws, which are one sentence with four
    /// endings: what follows the phrase is the plan's, so what is matched stops
    /// where the plans stop agreeing.
    ///
    /// Read off the binary rather than guessed at.
    #[test]
    fn the_lines_codex_draws_when_its_account_runs_out_are_recognised() {
        for line in [
            "You've hit your usage limit.",
            "You've hit your usage limit. Upgrade to Plus to continue using Codex (https://chatgpt.com/explore/plus)",
            "You've hit your usage limit. Visit https://chatgpt.com/codex/settings/usage to purchase more credits",
            "You've hit your usage limit. To get more access now, send a request to your admin",
            "▌ You've hit your usage limit. Upgrade to Pro (https://chatgpt.com/explore/pro)",
        ] {
            assert!(
                exhausted(line, Codex).is_some(),
                "{line:?} was not recognised",
            );
        }
    }

    /// Each backend is read against its own sentence and against no other.
    ///
    /// Which matters both ways round. A Codex session that printed claude's
    /// wording would be a session quoting somebody else's display in the middle
    /// of its work — and codex has a `Usage limit reached` heading of its own,
    /// drawn over an offer to ask an admin rather than over a wait — while
    /// claude never says anything about hitting a limit at all.
    #[test]
    fn a_session_is_read_against_its_own_backends_sentence() {
        assert_eq!(
            exhausted("Usage limit reached · continuing shortly", Codex),
            None,
            "claude's banner is not codex's account running out",
        );
        assert_eq!(
            exhausted("You've hit your usage limit.", Claude),
            None,
            "nor codex's sentence claude's",
        );
    }

    /// The wording is the backend's and will move, so what is matched is the
    /// phrase and nothing around it.
    #[test]
    fn the_phrase_is_matched_whatever_the_sentence_around_it_says() {
        for line in [
            "Usage limit reached · continuing shortly · esc to cancel",
            "Usage limit reached again after you continued",
            "usage limit reached — check plan",
            "  │ USAGE LIMIT REACHED",
        ] {
            assert!(
                exhausted(line, Claude).is_some(),
                "{line:?} was not recognised",
            );
        }
    }

    /// A line that mentions limits is not a line that says one has been reached.
    /// Verkstead's own sessions read this file, and a run that stopped itself on
    /// its own source would be the worst kind of false alarm.
    #[test]
    fn a_line_about_limits_is_not_a_line_saying_one_was_reached() {
        for line in [
            "The wording to match is \"usage limit reached\", said once.",
            "matching on `usage limit reached` in one place",
            "reading whether the usage limit reached the account",
            "No limit was reached.",
        ] {
            assert_eq!(exhausted(line, Claude), None, "{line:?} stopped a run");
        }
    }

    /// And the same for codex's, whose phrase opens with a word rather than with
    /// one this repository writes about limits — so the lines that could be
    /// mistaken for it are the ones that quote it.
    #[test]
    fn a_line_about_codexs_phrase_is_not_a_line_saying_it() {
        for line in [
            "The phrase to match is \"You've hit your usage limit\", said once.",
            "matching on `You've hit your usage limit` in one place",
            "- Codex opens with You've hit your usage limit, and decorates after it",
            "> You've hit your usage limit, says codex",
            "# You've hit your usage limit",
        ] {
            assert_eq!(exhausted(line, Codex), None, "{line:?} stopped a run");
        }
    }

    /// And this repository's own lines, quoted exactly as they sit in it.
    ///
    /// Verkstead builds this repository: a session that greps for a phrase, or
    /// opens the file the matcher lives in, prints these on its terminal. Reading
    /// one as its own account running out would stop the run it was in the middle
    /// of — a false alarm this build could not have anywhere else, and the one it
    /// is most likely to hit.
    ///
    /// Every line against both backends, because a session grepping for one
    /// phrase prints the lines that hold the other beside it.
    #[test]
    fn this_repositorys_own_fixtures_do_not_pause_the_session_reading_them() {
        for line in [
            // `crates/store/tests/pauses.rs`, and every other test that stands a
            // sentence up to be recognised.
            r#"            "Usage limit reached · continuing shortly","#,
            r#"        "Usage limit reached · continuing automatically at 3pm · esc to cancel","#,
            // The tests at the foot of this very file.
            r#"            "usage limit reached — check plan","#,
            r#"            "  │ USAGE LIMIT REACHED","#,
            r#"            "You've hit your usage limit.","#,
            r#"            "You've hit your usage limit. Upgrade to Plus to continue using Codex (https://chatgpt.com/explore/plus)","#,
            // And a line of prose about either, in a document or a commit
            // message.
            "- Usage limit reached is the phrase, and only the phrase",
            "# Usage limit reached",
            "> Usage limit reached, said the display",
            "- You've hit your usage limit is codex's, and only that much of it",
        ] {
            for agent_type in [Claude, Codex] {
                assert_eq!(
                    exhausted(line, agent_type),
                    None,
                    "{line:?} stopped the run reading it",
                );
            }
        }
    }

    /// The one that arrives in two chunks, because a read off a terminal lands
    /// wherever the kernel put it.
    #[test]
    fn a_sentence_split_across_two_chunks_is_still_one_sentence() {
        let mut watch = Watch::on(1, 2, "fable".to_owned(), Claude);

        watch.printed("Usage limit reac");
        assert_eq!(
            exhausted(&watch.printed, Claude),
            None,
            "half a sentence is not one"
        );

        watch.printed("hed · continuing shortly\n");
        assert!(exhausted(&watch.printed, Claude).is_some());
    }

    /// The reset is kept in the words the display drew it in. A clock time is
    /// what claude 2.1.234 prints, and `3pm` beside a Resume button is what
    /// somebody looks at their own clock for.
    #[test]
    fn the_reset_is_kept_in_the_words_the_display_drew_it_in() {
        assert_eq!(
            resets("✻ Usage limit reached · continuing automatically at 3pm · esc to cancel"),
            Some("3pm".to_owned()),
        );
    }

    /// The shapes a display writes a time in, and the instant a machine writes
    /// for another machine. Each read back as the word it was.
    #[test]
    fn the_shapes_a_reset_is_written_in_are_all_recognised() {
        for (said, expected) in [
            ("resets at 3pm", "3pm"),
            ("resets at 3:30pm", "3:30pm"),
            ("resets at 11:05am", "11:05am"),
            ("resets at 15:00", "15:00"),
            ("resets at 12am", "12am"),
            ("resets (2026-08-24T15:00:00Z)", "2026-08-24T15:00:00Z"),
            ("resets at 2026-08-24T15:00Z", "2026-08-24T15:00Z"),
        ] {
            assert_eq!(resets(said), Some(expected.to_owned()), "{said:?}");
        }
    }

    /// A sentence with no time in it carries no reset words, which is a stop
    /// that says only that the account ran out. A bare number is never read as
    /// an hour: the display writes "3pm" or "15:00", and showing "40" as four in
    /// the afternoon would name a time nobody said anything about.
    #[test]
    fn a_sentence_with_no_time_in_it_carries_none() {
        for said in [
            "Usage limit reached · continuing shortly · esc to cancel",
            "Usage limit reached · continuing automatically when it resets",
            "Usage limit reached after 40 turns",
            "Usage limit reached on claude 2.1.234",
        ] {
            assert_eq!(resets(said), None, "{said:?}");
        }
    }
}
