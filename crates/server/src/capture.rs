//! Reading a session's output as it arrives: bytes off a pseudo-terminal turned
//! into the Capture that is kept and the summary the Timeline shows.
//!
//! Kept as it was printed, control sequences and all. What the session sent a
//! terminal is what the session said, and a Capture that had been tidied on
//! the way in would be a record of something else — so the tidying happens here,
//! for the one line the Timeline shows, and nowhere else.
//!
//! The tidying is the fallback rather than the first answer. A session whose
//! backend keeps a log has its own prose on the Transcript beside this, and the
//! two places that quote a session in miniature — the Timeline row and an
//! Interruption's evidence — read that instead. What is left here is every
//! session that has no such log, for which the terminal is the whole record.
//!
//! What the sandbox's own plumbing said goes through this too, though it came off
//! a pipe rather than the terminal — see `plumbing` in [`crate::sessions`]. It
//! is written down like anything else a session produced, which is what makes it
//! count towards the lines the Timeline shows and become the last thing said: a
//! session that failed to start is one whose summary should be the reason.
//!
//! Both jobs need the same thing, which is why they are one type: a reading that
//! remembers where it got to. Bytes arrive in chunks that fall wherever the
//! kernel put them — through the middle of a character, through the middle of a
//! line, through the middle of an escape sequence — and none of the three can be
//! made sense of a chunk at a time.

use verkstead_store::Summary;

/// The longest a summarised line is worth carrying. A Timeline row shows one
/// line of text, and a session that printed a thousand characters without a
/// newline has not said a thousand characters' worth.
const LATEST_LIMIT: usize = 200;

/// How much of an unfinished line to hold on to. Only a guard: output arrives
/// in lines, and a session that printed megabytes without one is a session
/// whose earliest bytes are not what the Timeline is going to show.
const LINE_LIMIT: usize = 64 * 1024;

/// One session's output, read as far as it has arrived.
#[derive(Debug, Default)]
pub(crate) struct Reading {
    /// Bytes that are the beginning of a character whose rest has not arrived.
    pending: Vec<u8>,

    /// The line being printed now, as it stands.
    current: String,

    /// How many lines have been finished.
    lines: i64,

    /// The last finished line that said anything, tidied.
    latest: String,
}

impl Reading {
    /// Take a chunk of a session's output: what it adds to the Capture, with
    /// the summary moved on by it.
    ///
    /// What comes back is text rather than the bytes that went in, because that
    /// is what is stored — and the difference between the two is only the
    /// characters this had to wait for the rest of.
    pub(crate) fn take(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);

        let text = decode(&mut self.pending);
        self.follow(&text);

        text
    }

    /// The last of it, once the session has stopped printing.
    ///
    /// Whatever is left of a character that never finished arriving becomes a
    /// replacement character: the session has gone, so the rest of it is never
    /// coming.
    pub(crate) fn finish(&mut self) -> String {
        if self.pending.is_empty() {
            return String::new();
        }

        let text = String::from_utf8_lossy(&std::mem::take(&mut self.pending)).into_owned();
        self.follow(&text);

        text
    }

    /// What the Timeline shows: how many lines, and the last thing said.
    ///
    /// `said` is the agent's own latest prose, off the log it keeps of the
    /// conversation it is having — see [`crate::transcript`]. Where there is
    /// such a log that is what the row reads, because it is what the session
    /// said: the terminal underneath it is a display being redrawn, and the last
    /// line of one is as likely to be a spinner or the edge of a box as a
    /// sentence.
    ///
    /// `None` reads the terminal instead, and that is a whole answer rather than
    /// an apology. It is every stub agent the test suite runs, every backend
    /// that keeps no such log, and every session that has not said anything yet
    /// — and for a session that never started, the terminal is the only place
    /// the reason exists.
    ///
    /// The line being printed now counts as the latest where it has anything in
    /// it, and is not counted in the total. A session goes quiet in the middle
    /// of a line exactly when it has stopped to ask something, and a summary
    /// that waited for the newline would say nothing at the one moment somebody
    /// is reading it.
    pub(crate) fn summary(&self, said: Option<&str>) -> Summary {
        let current = plain(&self.current);

        let printed = match current.is_empty() {
            true => &self.latest,
            false => &current,
        };

        Summary {
            lines: self.lines,
            latest: shorten(said.unwrap_or(printed)),
        }
    }

    /// Move the summary on by text that has just arrived.
    fn follow(&mut self, text: &str) {
        let mut rest = text;

        while let Some((line, tail)) = rest.split_once('\n') {
            self.current.push_str(line);
            self.lines += 1;

            let plain = plain(&self.current);
            if !plain.is_empty() {
                self.latest = plain;
            }

            self.current.clear();
            rest = tail;
        }

        self.current.push_str(rest);

        if self.current.len() > LINE_LIMIT {
            // On a character boundary, because what is being kept is text — and
            // from the end, because the newest of it is what a summary is about.
            let from = self
                .current
                .char_indices()
                .map(|(at, _)| at)
                .find(|at| *at >= self.current.len() - LINE_LIMIT / 2)
                .unwrap_or(0);

            self.current = self.current.split_off(from);
        }
    }
}

/// The characters at the front of `pending` that are whole, taken out of it.
///
/// What is left behind is the beginning of a character whose rest has not
/// arrived — which is the whole reason this exists, a chunk boundary falling
/// through the middle of one being the ordinary case rather than the strange
/// one. Bytes that are not the beginning of anything become a replacement
/// character and the reading goes on: a pseudo-terminal carries whatever was
/// written to it, and one bad byte is not a session to give up on.
fn decode(pending: &mut Vec<u8>) -> String {
    let mut text = String::new();

    loop {
        match std::str::from_utf8(pending) {
            Ok(whole) => {
                text.push_str(whole);
                pending.clear();
                return text;
            }
            Err(error) => {
                let good = error.valid_up_to();

                // Whole by `valid_up_to`'s own account, so there is nothing here
                // to handle a failure of.
                text.push_str(std::str::from_utf8(&pending[..good]).unwrap_or_default());

                match error.error_len() {
                    // The beginning of a character. Keep it for the next chunk.
                    None => {
                        pending.drain(..good);
                        return text;
                    }
                    // Not one at all, and never going to be.
                    Some(len) => {
                        text.push(char::REPLACEMENT_CHARACTER);
                        pending.drain(..good + len);
                    }
                }
            }
        }
    }
}

/// One line as it was left on the terminal: the escape sequences gone, and a
/// carriage return read as what it does.
///
/// A carriage return sends the cursor back to the start of the line, so what
/// follows one is what a reader sees — which is how a progress line that
/// redrew itself forty times comes out as the fortieth. Approximate, and
/// deliberately: a redraw shorter than what it drew over leaves the tail of the
/// old text behind on a real terminal, and carrying that into a one-line
/// summary would be carrying the noise rather than the message.
fn plain(line: &str) -> String {
    let mut plain = String::new();
    let mut characters = line.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '\u{1b}' => skip_escape(&mut characters),
            // Only where something follows it. A line off a pseudo-terminal
            // ends `\r\n`, and that return goes back over nothing at all.
            '\r' if characters.peek().is_some() => plain.clear(),
            '\r' => {}
            '\t' => plain.push(' '),
            // Everything else the C0 range holds was an instruction to the
            // terminal rather than something drawn on it.
            character if (character as u32) < 0x20 || character == '\u{7f}' => {}
            character => plain.push(character),
        }
    }

    plain.trim().to_owned()
}

/// The last `lines` of a Capture that said anything, tidied of the terminal's
/// own control sequences.
///
/// What an Interruption keeps as evidence. The tail and not the whole, because
/// the whole is on the Timeline already as the session's own Event and this is
/// meant to be readable on a phone — and tidied for the same reason the one-line
/// summary is, a wall of cursor moves being a record of a display rather than of
/// what was said.
///
/// A line that redrew itself comes out as its last state, exactly as [`plain`]
/// leaves it: what a reader would have seen on the terminal is what a session
/// said.
pub(crate) fn tail(capture: &str, lines: usize) -> String {
    let mut said: Vec<&str> = capture.split('\n').collect();

    // A Capture ends with the newline of its last line, so the split leaves an
    // empty piece behind that is not a line the session printed.
    if said.last().is_some_and(|last| last.is_empty()) {
        said.pop();
    }

    let tidied: Vec<String> = said
        .iter()
        .rev()
        .map(|line| plain(line))
        .filter(|line| !line.is_empty())
        .take(lines)
        .collect();

    tidied.into_iter().rev().collect::<Vec<String>>().join("\n")
}

/// Step over the escape sequence that has just begun, leaving the reading on
/// whatever follows it.
///
/// Three shapes, which is all a terminal has. A control sequence — `ESC [` and
/// the colours, cursor moves and clears that make up most of what an agent's
/// display is — runs to a final byte in `@`–`~`. A string sequence — `ESC ]` and
/// its four rarer siblings, which is how a window title or a hyperlink is set —
/// runs to a bell or a string terminator. Everything else is `ESC`, however many
/// intermediates, and one final character.
fn skip_escape(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match characters.next() {
        Some('[') => {
            for character in characters.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&character) {
                    break;
                }
            }
        }
        Some(']' | 'P' | 'X' | '^' | '_') => {
            while let Some(character) = characters.next() {
                match character {
                    '\u{7}' => break,
                    // A string terminator: `ESC \`, of which the escape has
                    // just been read and the backslash is next.
                    '\u{1b}' => {
                        characters.next();
                        break;
                    }
                    _ => {}
                }
            }
        }
        // An intermediate, so the sequence runs on to its final character —
        // `ESC ( B`, which is how a terminal is told to go back to ASCII, being
        // the one an agent's display prints constantly.
        Some('\u{20}'..='\u{2f}') => {
            for character in characters.by_ref() {
                if !('\u{20}'..='\u{2f}').contains(&character) {
                    break;
                }
            }
        }
        _ => {}
    }
}

/// A line cut to what a Timeline row can show, on a character boundary.
fn shorten(line: &str) -> String {
    match line.chars().count() > LATEST_LIMIT {
        true => line.chars().take(LATEST_LIMIT).collect(),
        false => line.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Capture is what was printed, whatever the chunks it arrived in —
    /// which is the whole of what "byte for byte" means from in here.
    #[test]
    fn a_character_split_across_two_chunks_is_one_character() {
        let mut reading = Reading::default();

        let held = "héllo — ✓\n";
        let bytes = held.as_bytes();

        let mut kept = String::new();
        for byte in bytes {
            kept.push_str(&reading.take(&[*byte]));
        }
        kept.push_str(&reading.finish());

        assert_eq!(kept, held, "a chunk boundary is not a character boundary");
    }

    #[test]
    fn a_byte_that_is_no_character_at_all_does_not_stop_the_reading() {
        let mut reading = Reading::default();

        let kept = reading.take(b"before\xffafter\n");

        assert_eq!(kept, "before\u{fffd}after\n");
        assert_eq!(reading.summary(None).latest, "before\u{fffd}after");
    }

    #[test]
    fn the_summary_counts_the_lines_and_keeps_the_last_that_said_anything() {
        let mut reading = Reading::default();

        reading.take(b"first\r\nsecond\r\n\r\n");

        let summary = reading.summary(None);
        assert_eq!(summary.lines, 3, "a blank line is a line that was printed");
        assert_eq!(
            summary.latest, "second",
            "the last line with anything in it is what there is to show"
        );
    }

    /// What the agent said beats what its interface drew. A terminal in the
    /// middle of a redraw says nothing anybody wants a row to read.
    #[test]
    fn what_the_agent_said_is_what_the_summary_says() {
        let mut reading = Reading::default();

        reading.take("╭──────────╮\n│ Thinking…\r".as_bytes());

        let summary = reading.summary(Some("The counter lives in the limiter."));
        assert_eq!(summary.latest, "The counter lives in the limiter.");
        assert_eq!(
            summary.lines, 1,
            "and how much was printed is still how much was printed"
        );
    }

    #[test]
    fn a_statement_off_the_transcript_is_cut_to_a_row_like_any_other() {
        let said = "x".repeat(LATEST_LIMIT * 2);
        let summary = Reading::default().summary(Some(&said));

        assert_eq!(summary.latest.chars().count(), LATEST_LIMIT);
    }

    /// A session goes quiet mid-line exactly when it has stopped to ask
    /// something, which is the one moment somebody is reading the Timeline.
    #[test]
    fn a_line_still_being_printed_is_the_latest_statement() {
        let mut reading = Reading::default();

        reading.take(b"thinking\nWhat should happen when the queue is full?");

        let summary = reading.summary(None);
        assert_eq!(summary.lines, 1, "only the finished line is counted");
        assert_eq!(summary.latest, "What should happen when the queue is full?");
    }

    #[test]
    fn the_terminals_own_sequences_are_not_part_of_the_statement() {
        let mut reading = Reading::default();

        reading.take(b"\x1b[2m\x1b[38;5;244mdimmed\x1b[0m and bold\x1b[1m\x1b(B\n");

        assert_eq!(reading.summary(None).latest, "dimmed and bold");
    }

    #[test]
    fn an_operating_system_command_is_stepped_over_whole() {
        let mut reading = Reading::default();

        reading.take(b"\x1b]0;a title nobody printed\x07what was printed\n");

        assert_eq!(reading.summary(None).latest, "what was printed");
    }

    /// A spinner redraws its line over and over. What it says is the last of
    /// them, not all of them run together.
    #[test]
    fn a_line_that_redrew_itself_reads_as_the_last_drawing() {
        let mut reading = Reading::default();

        reading.take("Thinking.\rThinking..\rThinking…\n".as_bytes());

        let summary = reading.summary(None);
        assert_eq!(summary.latest, "Thinking…");
        assert_eq!(summary.lines, 1, "one line, drawn three times");
    }

    #[test]
    fn a_statement_longer_than_a_row_is_cut_to_one() {
        let mut reading = Reading::default();

        reading.take("x".repeat(LATEST_LIMIT * 2).as_bytes());

        assert_eq!(reading.summary(None).latest.chars().count(), LATEST_LIMIT);
    }

    /// Everything printed is kept whatever the summary does with it — the
    /// Capture is the record, and the summary is a line about it.
    #[test]
    fn nothing_is_dropped_from_what_is_kept() {
        let mut reading = Reading::default();

        let printed = "\x1b[2mdim\x1b[0m\r\nplain\r";
        let kept = reading.take(printed.as_bytes());

        assert_eq!(kept, printed);
    }
}
