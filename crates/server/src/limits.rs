//! An Agent Profile's account running out of window, and the Pause that makes
//! the wait answerable from a phone.
//!
//! **The agent waits too, and Verkstead neither turns that off nor depends on
//! it.** The claude in use holds its session when the limit lands and carries on
//! by itself when the window comes back, under a setting of its own. Nothing here
//! reaches into that configuration. What the Pause adds is that the wait is
//! *said*: the Timeline names the account that ran out and when it comes back,
//! the devices are told, and there is a press to start again — instead of a
//! session that has gone quiet for no stated reason.
//!
//! **Recognition is one sentence, said once.** [`EXHAUSTED`] is the whole of what
//! is matched, because the wording is the backend's and will move: claude 2.1.234
//! draws it as `Usage limit reached · continuing automatically at 3pm · esc to
//! cancel`, and every part of that but the phrase itself is decoration this build
//! has no business depending on. It is read off what the session leaves behind —
//! the Capture of what it printed and the Transcript its backend wrote — rather
//! than out of anything the agent is asked.
//!
//! A line that *says* it rather than one that mentions it: the phrase has to open
//! the line, once the terminal's own decoration is off the front of it. That is
//! what keeps a session reading this very file from pausing itself. What it
//! cannot rule out is an agent that opens a line with the phrase for reasons of
//! its own — and that is the cheap failure of the two, being one press to undo,
//! where a limit nobody noticed is a run that goes quiet for five hours.
//!
//! **Nothing here switches accounts.** An exhausted Profile is a wait, never a
//! reason to spend a different one: no Conversation moves to another Profile
//! because the one it is on ran out, and there is no code here that could.
//!
//! **Nothing here touches the Worktree.** A Pause stops Verkstead advancing and
//! does nothing else, so a run picked up again finds the repository exactly as
//! the session left it — the same promise every Remedy makes.

use std::time::Duration;

use sqlx::SqlitePool;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, Time, UtcOffset};
use verkstead_render::PauseResumed;
use verkstead_schema::Nudge;

use crate::AppState;
use crate::nudge::Nudges;
use crate::store;

/// The sentence a session prints when its account is out of window.
///
/// Matched without regard to case and to nothing around it. One phrase is the
/// whole of the coupling to somebody else's display, and it is here so that a
/// wording that moves is one edit rather than a search.
const EXHAUSTED: &str = "usage limit reached";

/// How much of a session's printing is held between looks.
///
/// Only a guard. What is kept between one look and the next is the line being
/// printed now, and a session that has printed this much without a newline is
/// not one whose earliest bytes hold the sentence.
const HELD: usize = 8 * 1024;

/// How often the runs waiting a window out are looked over, as [`crate::Pace`]
/// has it by default.
///
/// A minute. A window resets on the hour and a Pause is a wait rather than a
/// race, so noticing one a minute late costs nothing — and the sweep is one
/// indexed read that nearly always comes back with nothing, which is not a thing
/// to do in a tight loop for the years a server is up.
pub(crate) const SWEPT_EVERY: Duration = Duration::from_secs(60);

/// How long to give the machine to say what offset it keeps local time at.
///
/// Asked of `date`, once per Pause raised. A machine that will not answer within
/// this leaves the reset time unread, which is a Pause the human ends.
const ASKED_WITHIN: Duration = Duration::from_secs(5);

/// What a session said about its account being out of window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exhausted {
    /// The line that said so, as it was printed and tidied of the terminal's own
    /// sequences.
    ///
    /// Kept rather than reduced to the phrase that matched it, because it is the
    /// record: somebody reading the Pause a week later is reading the backend's
    /// own sentence rather than this build's opinion of it.
    pub(crate) said: String,
}

/// The sentence in `text` that says an account is out of window, or `None`.
///
/// Pure and cheap on purpose: this runs on every flush of every session, twice a
/// second while an agent talks, so everything a Pause needs beyond *whether* —
/// the reset time, the machine's offset — is worked out afterwards and only on a
/// hit.
pub(crate) fn exhausted(text: &str) -> Option<Exhausted> {
    text.lines()
        .map(crate::capture::plain)
        .find(|line| says_so(line))
        .map(|said| Exhausted { said })
}

/// Whether one tidied line *says* an account is out of window, rather than
/// mentioning that such a thing exists.
///
/// The phrase has to open the line once the decoration is off the front — the box
/// border, the spinner, the bullet, the spaces. That is the difference between a
/// status line the agent's display drew and a sentence about limits inside the
/// work.
fn says_so(line: &str) -> bool {
    let said: String = line
        .trim_start_matches(|character: char| !character.is_alphanumeric())
        .chars()
        .take(EXHAUSTED.len())
        .collect();

    said.eq_ignore_ascii_case(EXHAUSTED)
}

/// When the window resets, out of the sentence that said it was exhausted.
///
/// Two shapes, and both of them carry enough to be one moment:
///
/// - an instant written whole, RFC 3339, which is what a record written by a
///   machine for a machine looks like;
/// - a clock time — `3pm`, `3:30pm`, `15:00` — which is what a display written
///   for somebody sitting at the machine looks like, and which is the shape
///   claude 2.1.234 actually prints. It has no date and no zone in it, so the
///   missing halves are supplied here: `offset` is the one the machine keeps
///   local time at, and the date is whichever of today and tomorrow makes the
///   time still to come.
///
/// `now` and `offset` are handed in rather than read, which is what makes this
/// answerable without a clock: the same sentence at the same moment reads as the
/// same instant every time.
fn resets_at(said: &str, now: OffsetDateTime, offset: UtcOffset) -> Option<OffsetDateTime> {
    written_whole(said).or_else(|| next_clock(said, now, offset))
}

/// An RFC 3339 instant anywhere in the sentence.
///
/// Every word of it is tried rather than the shape being anticipated: an instant
/// is long, distinctive and self-checking, so a word that parses as one is one.
fn written_whole(said: &str) -> Option<OffsetDateTime> {
    said.split(|character: char| character.is_whitespace())
        .map(|word| word.trim_matches(|character: char| !character.is_alphanumeric()))
        .find_map(|word| OffsetDateTime::parse(word, &Rfc3339).ok())
}

/// The next moment the machine's clock reads `said`'s clock time.
///
/// Strictly ahead of `now`, which is what makes "resets at 3pm" read as this
/// afternoon before three and as tomorrow afternoon after it. A window that has
/// only just reset is never read as one a day away: the sentence is written when
/// the limit lands, and the reset it names is always still to come.
fn next_clock(said: &str, now: OffsetDateTime, offset: UtcOffset) -> Option<OffsetDateTime> {
    let clock = said.split_whitespace().find_map(clock)?;

    let here = now.to_offset(offset);
    let today = here.replace_time(clock);

    let at = match today > here {
        true => today,
        false => here
            .date()
            .next_day()?
            .with_time(clock)
            .assume_offset(offset),
    };

    Some(at.to_offset(UtcOffset::UTC))
}

/// One word read as a time of day, or `None` where it is an ordinary word.
///
/// The three ways a display writes one: `3pm`, `3:30pm` and `15:00` — with the
/// twelve-hour ones taking their meridiem attached, because that is how they are
/// drawn. A word ending in a meridiem it cannot use, or naming an hour there is
/// none of, is not a time.
fn clock(word: &str) -> Option<Time> {
    let word = word.trim_matches(|character: char| !character.is_alphanumeric());
    let lowered = word.to_ascii_lowercase();

    let (digits, meridiem) = match lowered.strip_suffix("am") {
        Some(digits) => (digits, Some(false)),
        None => match lowered.strip_suffix("pm") {
            Some(digits) => (digits, Some(true)),
            None => (lowered.as_str(), None),
        },
    };

    let digits = digits.trim();
    let (hour, minute) = match digits.split_once(':') {
        Some((hour, minute)) => (hour, minute.parse::<u8>().ok()?),
        // Only with a meridiem on it. A bare number is a number — a version, a
        // count of files, the 40 in "40 paths" — and reading one as an hour would
        // pause a run on arithmetic.
        None => (digits, 0),
    };

    let hour: u8 = hour.parse().ok()?;

    let hour = match meridiem {
        // Noon is 12pm and midnight is 12am, which is the one place the
        // twelve-hour clock does not simply add twelve.
        Some(true) => 12 + hour % 12,
        Some(false) => hour % 12,
        None if digits.contains(':') => hour,
        None => return None,
    };

    Time::from_hms(hour, minute, 0).ok()
}

/// The same against the clock, asking the machine for its offset only where the
/// sentence needs one.
///
/// An instant written whole carries its own zone, so a machine that will not say
/// what offset it keeps costs nothing there. It is only a clock time that has a
/// half missing — see [`next_clock`].
async fn resets_when(said: &str) -> Option<OffsetDateTime> {
    if let Some(at) = written_whole(said) {
        return Some(at);
    }

    resets_at(said, OffsetDateTime::now_utc(), local_offset().await?)
}

/// The offset the machine keeps local time at, asked of the machine itself.
///
/// A clock time is written for somebody sitting at that machine, so the zone it
/// is missing is that machine's. Asked of `date`, which every system has, rather
/// than carried as a timezone database this build has no other use for — and
/// asked per Pause rather than held, so a server that has been up across a
/// daylight-saving change still reads the sentence in the offset the sentence was
/// written in.
///
/// `None` is a machine that would not answer, or an answer that is not an offset.
/// The Pause then carries no reset time, which is a wait the human ends.
async fn local_offset() -> Option<UtcOffset> {
    let asking = tokio::process::Command::new("date").arg("+%z").output();

    let said = match tokio::time::timeout(ASKED_WITHIN, asking).await {
        Ok(Ok(said)) if said.status.success() => said.stdout,
        Ok(Ok(said)) => {
            tracing::warn!(status = %said.status, "the machine would not say what offset it keeps");
            return None;
        }
        Ok(Err(error)) => {
            tracing::warn!(error = ?error, "asking the machine what offset it keeps failed");
            return None;
        }
        Err(_) => {
            tracing::warn!("the machine took too long to say what offset it keeps");
            return None;
        }
    };

    read_offset(String::from_utf8_lossy(&said).trim())
}

/// `+1000`, `-0430`, `+00` or `Z` as an offset.
///
/// Split out from the asking so that what the machine says can be read without a
/// machine to say it.
fn read_offset(said: &str) -> Option<UtcOffset> {
    if said == "Z" || said == "z" {
        return Some(UtcOffset::UTC);
    }

    let (sign, digits) = said.split_at_checked(1)?;

    let sign: i8 = match sign {
        "+" => 1,
        "-" => -1,
        _ => return None,
    };

    let digits: String = digits.chars().filter(char::is_ascii_digit).collect();

    let (hours, minutes) = match digits.len() {
        2 => (digits.parse::<i8>().ok()?, 0),
        4 => (
            digits[..2].parse::<i8>().ok()?,
            digits[2..].parse::<i8>().ok()?,
        ),
        _ => return None,
    };

    UtcOffset::from_hms(sign * hours, sign * minutes, 0).ok()
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
    /// that is what the Pause names — see [`store::Pause::profile`].
    profile: String,

    /// What the session has printed since the last look, less whatever has
    /// already been looked at.
    ///
    /// Held across looks so that a sentence arriving in two chunks is still one
    /// sentence: a read off a terminal lands wherever the kernel put it, and the
    /// phrase falling across the boundary is the ordinary case rather than the
    /// strange one.
    printed: String,

    /// The sentence the last Pause was raised on, or `None` while none has been.
    ///
    /// A latch rather than a flag, because the display redraws its banner for as
    /// long as the wait lasts, and a Pause attempted twice a second for five
    /// hours would be a transaction twice a second for five hours. A sentence
    /// that has *changed* is worth another attempt — an account that ran out
    /// again after the agent continued says so in different words — and the
    /// store refuses it where the Pause it would raise is already open.
    raised: Option<String>,
}

impl Watch {
    /// Watch the session printing into `event_id`, run under `profile`.
    pub(crate) fn on(conversation_id: i64, event_id: i64, profile: String) -> Watch {
        Watch {
            conversation_id,
            event_id,
            profile,
            printed: String::new(),
            raised: None,
        }
    }

    /// Take a chunk of what the session printed.
    ///
    /// Cheap on purpose: this is called with everything that comes off the
    /// terminal, and the reading happens at the flush.
    pub(crate) fn printed(&mut self, text: &str) {
        self.printed.push_str(text);
    }

    /// Look at what has arrived, and pause the run if it says the account is out
    /// of window.
    ///
    /// `said` is the last thing the agent wrote in its own log, which is the other
    /// record a session leaves behind — see [`crate::transcript`]. It is looked at
    /// beside the terminal rather than instead of it: a backend that says so in
    /// its log and not on its display would otherwise go unnoticed.
    pub(crate) async fn look(&mut self, pool: &SqlitePool, nudges: &Nudges, said: Option<&str>) {
        let found = exhausted(&self.printed).or_else(|| exhausted(said.unwrap_or_default()));

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
            return;
        };

        if self.raised.as_deref() == Some(found.said.as_str()) {
            return;
        }

        self.raised = Some(found.said.clone());

        pause(
            pool,
            nudges,
            self.conversation_id,
            self.event_id,
            &self.profile,
            &found.said,
        )
        .await;
    }
}

/// Stop the run: put the wait on the Timeline and tell the human's devices.
///
/// Nothing is refused for. A Pause that could not be written is a run that waits
/// with nothing saying so, which is a thing to see in the log and the same thing
/// either way: the session goes on waiting, because the agent's own wait is not
/// Verkstead's to end.
async fn pause(
    pool: &SqlitePool,
    nudges: &Nudges,
    conversation_id: i64,
    session: i64,
    profile: &str,
    said: &str,
) {
    // Only now, and only once per Pause: reading the reset time is a word or two
    // of parsing and, for a clock time, a question to the machine — and the
    // flush this was reached from runs twice a second.
    let resets_at = resets_when(said)
        .await
        .and_then(|at| at.format(&Rfc3339).ok());

    let recorded =
        store::record_pause(pool, conversation_id, profile, said, resets_at.as_deref()).await;

    let event_id = match recorded {
        Ok(Some(event_id)) => event_id,
        // A run that is already waiting, which is what the banner redrawing in
        // different words comes to. The first Pause is the one the human was
        // told about.
        Ok(None) => {
            tracing::info!(
                conversation_id,
                session,
                "an account ran out of window on a run that was already waiting"
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "a run could not be paused on a usage limit");
            return;
        }
    };

    tracing::warn!(
        conversation_id,
        event_id,
        session,
        profile,
        resets_at = resets_at.as_deref().unwrap_or("unread"),
        "an account ran out of window, so the run is waiting"
    );

    // The Timeline has something on it that is waiting on the human, and an open
    // page should say so without being reloaded.
    nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    crate::push::paused(pool, conversation_id, profile, resets_at.as_deref());
}

/// Start the work again: close the wait, and get the Conversation driving where
/// nothing is.
///
/// The two ways in meet here, which is the point of them meeting: the human's
/// press and the reset time passing do the same thing, and the record keeps which
/// it was — see [`store::By`].
///
/// Nothing is reverted, reset or stashed. A Pause never touched the Worktree, so
/// there is nothing to put back: the run picks up from wherever the session left
/// the repository.
///
/// What it does beyond closing depends on what is there. A session still waiting
/// its own limit out is left alone — the agent comes back by itself, and the
/// Pause was only ever what stopped Verkstead launching the *next* thing. A
/// Conversation with nothing driving it is started driving again the way a
/// stalled one is, which is the same question answered from the state it is in.
pub(crate) async fn resume(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    by: store::By,
) -> anyhow::Result<PauseResumed> {
    let resuming = store::resume_pause(&state.pool, conversation_id, event_id, by).await?;

    match resuming {
        store::Resuming::NoSuchPause => return Ok(PauseResumed::NoSuchPause),
        store::Resuming::AlreadyResumed => return Ok(PauseResumed::AlreadyResumed),
        store::Resuming::Resumed => {}
    }

    tracing::info!(
        conversation_id,
        event_id,
        by = ?by,
        "a run that was waiting on a usage limit is going on again"
    );

    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    driving_again(state, conversation_id).await;

    Ok(PauseResumed::Resumed)
}

/// Start driving the Conversation again, where nothing is driving it.
///
/// Asked before anything is started, because the ordinary case is that something
/// already is: a session waiting its own limit out is a run somebody is still
/// seeing out, and starting a second driver over the top of it would be two
/// agents in one Worktree.
async fn driving_again(state: &AppState, conversation_id: i64) {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to go on with"
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to go on with failed");
            return;
        }
    };

    if state.drivers.driven(
        &state.sessions.working(),
        conversation_id,
        conversation.state,
    ) {
        tracing::info!(
            conversation_id,
            "the run was already being driven, so the wait ending is the whole of it"
        );
        return;
    }

    // Registered here and handed on, exactly as a retried Interruption's is: the
    // gap between deciding to drive and something driving is where a stall sweep
    // would find a Conversation with nothing on it.
    let driving = state.drivers.driving(conversation_id);

    crate::stalls::retried(state.clone(), conversation_id, String::new(), driving).await;
}

/// End every Pause whose window has come back, from now until the process stops.
///
/// A sweep rather than a timer per Pause, and that is what makes the reset
/// survive a restart: nothing holds a clock across the process, so a window that
/// came back while the server was down is one the first sweep finds already due.
///
/// On [`crate::Pace::pauses`], which is a knob of its own rather than the stall
/// sweep's: the two look for different things, and a server tuned to notice one
/// briskly has not asked to be told about the other any sooner.
pub(crate) fn sweeping(state: &AppState) {
    let state = state.clone();

    tokio::spawn(async move {
        loop {
            sweep(&state).await;

            tokio::time::sleep(state.sessions.pace().pauses).await;
        }
    });
}

/// One look over every Pause that is still waiting.
///
/// Nothing is refused for and nothing is returned: this runs unattended with
/// nobody watching, and what it has to say it says on the Timeline or in the log.
pub(crate) async fn sweep(state: &AppState) {
    let waiting = match store::waiting_pauses(&state.pool).await {
        Ok(waiting) => waiting,
        Err(error) => {
            tracing::error!(error = ?error, "listing the runs waiting on a usage limit failed");
            return;
        }
    };

    let now = OffsetDateTime::now_utc();

    for pause in waiting {
        // A Pause whose sentence carried no time this build could read is a wait
        // the human ends, and there is nothing here for it.
        let Some(resets_at) = pause.resets_at.as_deref() else {
            continue;
        };

        let Ok(resets_at) = OffsetDateTime::parse(resets_at, &Rfc3339) else {
            tracing::error!(
                conversation_id = pause.conversation_id,
                event_id = pause.event_id,
                resets_at,
                "a Pause names a reset time nothing can read"
            );
            continue;
        };

        if resets_at > now {
            continue;
        }

        if let Err(error) = resume(
            state,
            pause.conversation_id,
            pause.event_id,
            store::By::Reset,
        )
        .await
        {
            tracing::error!(
                error = ?error,
                conversation_id = pause.conversation_id,
                event_id = pause.event_id,
                "a window came back and the run could not be started again",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The line claude 2.1.234 draws, decoration and all: the phrase opens it
    /// once the terminal's own marks are off the front.
    #[test]
    fn the_line_a_session_draws_when_its_account_runs_out_is_recognised() {
        let printed = "\u{1b}[2m✻\u{1b}[0m Usage limit reached · continuing automatically at 3pm · esc to cancel\n";

        assert_eq!(
            exhausted(printed).map(|found| found.said),
            Some(
                "✻ Usage limit reached · continuing automatically at 3pm · esc to cancel"
                    .to_owned()
            ),
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
            assert!(exhausted(line).is_some(), "{line:?} was not recognised");
        }
    }

    /// A line that mentions limits is not a line that says one has been reached.
    /// Verkstead's own sessions read this file, and a run that paused itself on
    /// its own source would be the worst kind of false alarm.
    #[test]
    fn a_line_about_limits_is_not_a_line_saying_one_was_reached() {
        for line in [
            "The wording to match is \"usage limit reached\", said once.",
            "matching on `usage limit reached` in one place",
            "reading whether the usage limit reached the account",
            "No limit was reached.",
        ] {
            assert_eq!(exhausted(line), None, "{line:?} paused a run");
        }
    }

    /// The one that arrives in two chunks, because a read off a terminal lands
    /// wherever the kernel put it.
    #[test]
    fn a_sentence_split_across_two_chunks_is_still_one_sentence() {
        let mut watch = Watch::on(1, 2, "fable".to_owned());

        watch.printed("Usage limit reac");
        assert_eq!(
            exhausted(&watch.printed),
            None,
            "half a sentence is not one"
        );

        watch.printed("hed · continuing shortly\n");
        assert!(exhausted(&watch.printed).is_some());
    }

    /// A clock time on the machine's own offset: this afternoon before three,
    /// and tomorrow afternoon after it.
    #[test]
    fn a_clock_time_is_the_next_moment_the_machines_clock_reads_it() {
        let sydney = UtcOffset::from_hms(10, 0, 0).unwrap();
        let said = "Usage limit reached · continuing automatically at 3pm · esc to cancel";

        // 2026-08-24 13:00 in Sydney, which is 03:00 UTC.
        let before = OffsetDateTime::parse("2026-08-24T03:00:00Z", &Rfc3339).unwrap();
        assert_eq!(
            resets_at(said, before, sydney),
            Some(OffsetDateTime::parse("2026-08-24T05:00:00Z", &Rfc3339).unwrap()),
            "3pm today, which is 05:00 UTC",
        );

        // And 16:00 in Sydney, which is past it.
        let after = OffsetDateTime::parse("2026-08-24T06:00:00Z", &Rfc3339).unwrap();
        assert_eq!(
            resets_at(said, after, sydney),
            Some(OffsetDateTime::parse("2026-08-25T05:00:00Z", &Rfc3339).unwrap()),
            "3pm tomorrow",
        );
    }

    /// The three ways a display writes a time of day.
    #[test]
    fn the_shapes_a_display_writes_a_time_in_are_all_read() {
        let utc = UtcOffset::UTC;
        let now = OffsetDateTime::parse("2026-08-24T01:00:00Z", &Rfc3339).unwrap();

        for (said, expected) in [
            ("resets at 3pm", "2026-08-24T15:00:00Z"),
            ("resets at 3:30pm", "2026-08-24T15:30:00Z"),
            ("resets at 11:05am", "2026-08-24T11:05:00Z"),
            ("resets at 15:00", "2026-08-24T15:00:00Z"),
            ("resets at 12am", "2026-08-25T00:00:00Z"),
            ("resets at 12pm", "2026-08-24T12:00:00Z"),
        ] {
            assert_eq!(
                resets_at(said, now, utc),
                Some(OffsetDateTime::parse(expected, &Rfc3339).unwrap()),
                "{said:?}",
            );
        }
    }

    /// An instant written whole, which is what a record written for a machine
    /// looks like. Read wherever in the sentence it sits.
    #[test]
    fn an_instant_written_whole_is_read_as_it_stands() {
        let now = OffsetDateTime::parse("2026-08-24T01:00:00Z", &Rfc3339).unwrap();

        assert_eq!(
            resets_at(
                "Usage limit reached (resets 2026-08-24T15:00:00Z)",
                now,
                UtcOffset::UTC,
            ),
            Some(OffsetDateTime::parse("2026-08-24T15:00:00Z", &Rfc3339).unwrap()),
        );
    }

    /// A sentence with no time in it leaves the Pause with none, which is a wait
    /// the human ends. A bare number is never read as an hour: the display writes
    /// "3pm" or "15:00", and reading "40" as four in the afternoon would end a
    /// wait on arithmetic.
    #[test]
    fn a_sentence_with_no_time_in_it_carries_none() {
        let now = OffsetDateTime::parse("2026-08-24T01:00:00Z", &Rfc3339).unwrap();

        for said in [
            "Usage limit reached · continuing shortly · esc to cancel",
            "Usage limit reached · continuing automatically when it resets",
            "Usage limit reached after 40 turns",
        ] {
            assert_eq!(resets_at(said, now, UtcOffset::UTC), None, "{said:?}");
        }
    }

    /// What `date +%z` says, in the shapes it says it in.
    #[test]
    fn the_machines_own_offset_is_read_as_it_prints_it() {
        assert_eq!(read_offset("+1000"), UtcOffset::from_hms(10, 0, 0).ok());
        assert_eq!(read_offset("-0430"), UtcOffset::from_hms(-4, -30, 0).ok());
        assert_eq!(read_offset("+0000"), Some(UtcOffset::UTC));
        assert_eq!(read_offset("Z"), Some(UtcOffset::UTC));
        assert_eq!(read_offset("nowhere"), None);
    }
}
