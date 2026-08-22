//! The Screen: the grid a session's Capture leaves on a terminal.
//!
//! The Capture is the record — every byte the session sent its terminal, escapes
//! and all — and a terminal is what those bytes were addressed to. A reader who
//! wants to know what the session *looked* like wants the thing at the other end
//! of them, which is a grid of characters with a cursor in it rather than a
//! transcript of the instructions that drew it.
//!
//! So Verkstead keeps the terminal. The bytes go through a virtual one held
//! here, and what comes back out is the escape sequences that would paint the
//! grid as it stands — which is what the browser draws, and what a fresh attach
//! will repaint from ([ADR 0007](../../../docs/adr/0007-server-held-terminal.md)).
//! The browser gets a window onto this rather than a copy of it: this is the
//! source of truth, and there is nothing in a repaint a terminal has to be told
//! twice.
//!
//! **The grid and nothing above it.** No scrollback is kept, however much the
//! session printed — a session prints for an hour and the Screen is what it is
//! showing, not everything it has ever shown. What was scrolled off is not lost:
//! it is in the Capture, which is where a reader who wants all of it is already
//! looking.

use avt::Vt;

use crate::terminal::{COLUMNS, ROWS};

/// One session's Screen: the grid its Capture leaves on a terminal.
pub(crate) struct Screen {
    /// The virtual terminal holding it. Built with no scrollback at all, which
    /// is the decision above spelled the one way the engine takes it.
    vt: Vt,
}

impl Screen {
    /// An empty Screen `columns` by `rows`, with nothing fed to it yet.
    fn sized(columns: u16, rows: u16) -> Screen {
        Screen {
            vt: Vt::builder()
                .size(usize::from(columns), usize::from(rows))
                .scrollback_limit(0)
                .build(),
        }
    }

    /// Send it what a session printed.
    fn feed(&mut self, text: &str) {
        // What comes back is which lines changed and what fell off the top —
        // the first is a live attach's business and the second is nothing's,
        // this being a Screen with no scrollback to fall into.
        self.vt.feed_str(text);
    }

    /// The escape sequences that would paint the grid as it stands.
    ///
    /// Everything a terminal has to be told to end up showing this: the
    /// characters, their colours, where the cursor is, and which of the two
    /// buffers is the one in front — a session that ended inside a full-screen
    /// display ended on the alternate screen, and a repaint that put its grid on
    /// the ordinary one would be showing the right characters in the wrong
    /// place.
    pub(crate) fn repaint(&self) -> String {
        self.vt.dump()
    }

    /// How wide the grid is, and how tall.
    pub(crate) fn size(&self) -> (u16, u16) {
        let (columns, rows) = self.vt.size();

        // Both were `u16` on the way in — see [`Screen::sized`] — and a terminal
        // is not resized from anywhere else.
        (columns as u16, rows as u16)
    }
}

/// The Screen a Capture leaves: the whole of it, played through a terminal.
///
/// Replayed rather than remembered. A session that ended has its Capture and
/// nothing else, and a live one has a Capture that has got as far as it has got
/// — either way the bytes are the record, and a terminal fed all of them from
/// the start is where they were always going.
///
/// At the size sessions are started on, because that is the size they were
/// printed for: nothing has resized one yet.
pub(crate) fn replay(capture: &str) -> Screen {
    let mut screen = Screen::sized(COLUMNS, ROWS);
    screen.feed(capture);
    screen
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a terminal handed `repaint` would be showing: the visible grid, row
    /// by row, with the blank rows at the bottom left off.
    ///
    /// Through a second terminal rather than read off the first, because that is
    /// the claim being made. A repaint is only worth anything if a terminal that
    /// has seen none of the session ends up showing what this one is showing.
    fn painted(repaint: &str, columns: u16, rows: u16) -> Vec<String> {
        let mut fresh = Screen::sized(columns, rows);
        fresh.feed(repaint);
        shown(&fresh)
    }

    /// The same, off a Screen directly.
    fn shown(screen: &Screen) -> Vec<String> {
        let mut rows: Vec<String> = screen
            .vt
            .view()
            .map(|line| line.text().trim_end().to_owned())
            .collect();

        while rows.last().is_some_and(|row| row.is_empty()) {
            rows.pop();
        }

        rows
    }

    /// A line drawn over by a shorter one leaves the tail of the old text behind
    /// — which is what a terminal does, and the whole reason a Screen is not the
    /// Capture with the escapes filtered out.
    #[test]
    fn a_line_redrawn_is_what_the_terminal_is_left_showing() {
        let mut screen = Screen::sized(20, 4);

        screen.feed("Thinking hard…\rDone\r\n");

        assert_eq!(shown(&screen), vec!["Doneking hard…".to_owned()]);
    }

    /// The cursor moves, the erases erase, and what is left is the grid — asked
    /// of the repaint, because the repaint is what leaves here.
    #[test]
    fn the_repaint_paints_what_the_bytes_drew() {
        let mut screen = Screen::sized(20, 4);

        screen.feed(
            // Three lines, then home, then over the first of them, then the
            // second cleared from where the cursor lands to the end of the line.
            "first\r\nsecond\r\nthird\r\n\x1b[H\x1b[1;1Hover\x1b[2;3H\x1b[K",
        );

        assert_eq!(
            painted(&screen.repaint(), 20, 4),
            vec!["overt".to_owned(), "se".to_owned(), "third".to_owned()],
        );
    }

    /// A session that ended inside a full-screen display ended on the alternate
    /// screen, and that is the screen it is showing.
    #[test]
    fn a_session_that_ended_on_the_alternate_screen_shows_that_one() {
        let mut screen = Screen::sized(20, 4);

        screen.feed("what the shell printed\r\n\x1b[?1049h\x1b[2J\x1b[Hthe display");

        assert_eq!(
            painted(&screen.repaint(), 20, 4),
            vec!["the display".to_owned()],
            "the alternate screen is in front, so it is what a repaint shows",
        );
    }

    /// And one that came back out of it is showing what was underneath again,
    /// which is the other half of the same claim.
    #[test]
    fn a_session_that_left_the_alternate_screen_shows_what_was_under_it() {
        let mut screen = Screen::sized(20, 4);

        screen.feed("what the shell printed\r\n\x1b[?1049h\x1b[2J\x1b[Hthe display\x1b[?1049l");

        assert_eq!(
            painted(&screen.repaint(), 20, 4),
            vec!["what the shell print".to_owned(), "ed".to_owned()],
        );
    }

    /// Whatever a session printed, what is kept is the grid.
    #[test]
    fn nothing_above_the_top_of_the_grid_is_kept() {
        let mut screen = Screen::sized(20, 4);

        for line in 0..500 {
            screen.feed(&format!("line {line}\r\n"));
        }

        assert_eq!(
            screen.vt.lines().count(),
            4,
            "a Screen is four rows of terminal, not five hundred lines of history",
        );

        assert_eq!(
            painted(&screen.repaint(), 20, 4),
            vec![
                "line 497".to_owned(),
                "line 498".to_owned(),
                "line 499".to_owned(),
            ],
            "and what it shows is the end of it",
        );
    }

    /// The size a session is started at is the size its Capture is replayed at.
    #[test]
    fn a_capture_is_replayed_at_the_size_it_was_printed_for() {
        let screen = replay("hello\r\n");

        assert_eq!(screen.size(), (COLUMNS, ROWS));
    }
}
