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
//!
//! **A session that is still running has its Screen held rather than replayed**
//! — see [`Live`]. The relay feeds it the same text it is writing the Capture
//! from, so what a watcher sees is where the session has got to rather than
//! where the store last got, and [`attach`] is the socket they see it over: the
//! repaint first, then everything printed after it. One Screen however many
//! devices are watching, and the latest window size wins for all of them,
//! because there is one terminal underneath and it is only one size at a time.
//!
//! Watching commits the human to nothing. Nothing here writes to the store,
//! puts anything on a Timeline or moves a Conversation — a Screen with somebody
//! looking at it and a Screen with nobody looking at it are the same Screen.

use std::sync::{Arc, Mutex};

use avt::Vt;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response as HttpResponse;
use tokio::sync::broadcast;
use verkstead_render::{Shown, Size, Watching};

use crate::AppState;
use crate::terminal::{COLUMNS, ROWS, Terminal};

/// How far behind a watcher may fall before it is repainted instead of caught
/// up.
///
/// Falling behind is a slow connection rather than a broken one, and the cure
/// is the thing a socket opens with anyway: a repaint says what the grid *is*,
/// so a watcher that missed the middle of a redraw is put right by one rather
/// than left showing half of it. Generous enough that a watcher on a working
/// connection never reaches it — this is thousands of reads of a terminal.
const WATCHER_BACKLOG: usize = 256;

/// The largest window a watcher may ask the session's terminal to be, in either
/// direction.
///
/// A browser reports the size of a pane it has drawn, so a number past this is a
/// client that is not a browser. Bounded because a grid is columns times rows of
/// cells held here: the size arrives from outside, so it is checked on the way
/// in rather than trusted.
const LARGEST: u16 = 1000;

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

    /// Make the grid `size` big, reflowing what is on it.
    fn resize(&mut self, size: Size) {
        self.vt
            .resize(usize::from(size.columns), usize::from(size.rows));
    }

    /// The whole of it, as the message a watcher is sent.
    fn painted(&self) -> Shown {
        let (columns, rows) = self.size();

        Shown::Painted(verkstead_render::Screen {
            repaint: self.repaint(),
            columns,
            rows,
        })
    }
}

/// The Screen of a session that is still running: held rather than replayed, and
/// shared by everything that touches one.
///
/// The relay feeds it the same text it writes the Capture from, so this is
/// current where a replay is as far as the store last got — half a second is
/// nothing to a record and a long time to a terminal somebody is watching.
///
/// One of these per running session, and one Screen inside it however many
/// devices are attached: a watcher is a window onto this rather than a terminal
/// of its own, which is why the size the latest of them asked for is the size
/// they all get.
#[derive(Clone)]
pub(crate) struct Live {
    /// The grid. Behind a lock because the relay writes it while watchers read
    /// it, and because attaching has to happen at a definite moment — see
    /// [`Live::watched`].
    screen: Arc<Mutex<Screen>>,

    /// Word to every watcher of what has just happened to it.
    shown: broadcast::Sender<Shown>,

    /// The session's own terminal, which is what a resize has to reach: a grid
    /// made wider here and nowhere else would be a Screen the session never
    /// heard about and went on drawing a hundred columns into.
    terminal: Arc<Terminal>,
}

impl Live {
    /// The Screen of a session about to start on `terminal`, empty and the size
    /// the terminal was opened at.
    pub(crate) fn on(terminal: Arc<Terminal>) -> Live {
        Live {
            screen: Arc::new(Mutex::new(Screen::sized(COLUMNS, ROWS))),
            shown: broadcast::Sender::new(WATCHER_BACKLOG),
            terminal,
        }
    }

    /// The session printed `text`: put it on the grid, and pass it on to
    /// whoever is watching.
    ///
    /// Both under the one lock, which is what makes [`Live::watched`] exact: a
    /// watcher is either sent this or repainted with it already on the grid, and
    /// never both or neither.
    pub(crate) fn printed(&self, text: &str) {
        if text.is_empty() {
            return;
        }

        let mut screen = self.held();
        screen.feed(text);

        // A send that fails is nobody watching, which is the ordinary case.
        let _ = self.shown.send(Shown::Printed(text.to_owned()));
    }

    /// Somebody is watching: the grid as it stands, and everything that happens
    /// to it from this moment on.
    ///
    /// Taken together under the lock rather than one after the other, so that
    /// nothing the session printed falls between the repaint and the first thing
    /// heard on the stream — which is a watcher whose terminal is missing a line
    /// nothing will ever send again.
    pub(crate) fn watched(&self) -> (Shown, broadcast::Receiver<Shown>) {
        let screen = self.held();
        let watching = self.shown.subscribe();

        (screen.painted(), watching)
    }

    /// The grid as it stands, for a watcher that has to be put right rather than
    /// caught up — see [`WATCHER_BACKLOG`].
    pub(crate) fn painted(&self) -> Shown {
        self.held().painted()
    }

    /// A watcher's window is `size` big, so that is what the Screen is now.
    ///
    /// The latest wins for everybody, which is what one Screen means. The
    /// session's own terminal is resized first — that is the half the session
    /// hears about, and what makes its interface redraw to fit — and then the
    /// grid, so that what arrives next is drawn for the size it is being drawn
    /// on. Everyone watching is repainted, because a repaint is the only thing
    /// that says how big the grid is.
    ///
    /// A size outside what a window can be is ignored rather than clamped: it
    /// came from outside, and a Screen quietly a different size from the one
    /// asked for is worse than one that did not move.
    pub(crate) fn resized(&self, size: Size) {
        if !(1..=LARGEST).contains(&size.columns) || !(1..=LARGEST).contains(&size.rows) {
            tracing::warn!(
                columns = size.columns,
                rows = size.rows,
                "a watcher asked for a window that is not a size a window comes in",
            );
            return;
        }

        let mut screen = self.held();

        if let Err(error) = self.terminal.resize(size.columns, size.rows) {
            tracing::error!(error = ?error, "a session's terminal would not be resized");
            return;
        }

        screen.resize(size);
        let _ = self.shown.send(screen.painted());
    }

    /// The grid, locked.
    fn held(&self) -> std::sync::MutexGuard<'_, Screen> {
        self.screen.lock().expect("a live Screen is not poisoned")
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

/// `GET /api/ui/conversations/{id}/screen/{event}/attach` — the Screen of a
/// session that is still running, watched as it is drawn.
///
/// The first bidirectional transport in the codebase, and it earns being one:
/// SSE and a refetch are how everything else stays fresh because everything else
/// is a document that changed, and a terminal is neither a document nor
/// something a watcher only reads. What goes up is the size of the window it is
/// being watched in.
///
/// Every live session is attachable, grilling included. There is no auth of its
/// own: the tailnet is the perimeter, as it is for every other endpoint here.
///
/// A session that is not running is refused rather than replayed. Its Screen is
/// the one the plain [`screen`](crate::ui) endpoint hands over — read-only,
/// being the screen it last stood on — and a socket that offered to relay a
/// session that has ended would be offering to relay nothing at all.
pub(crate) async fn attach(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
    watcher: WebSocketUpgrade,
) -> HttpResponse {
    // Read as permissively as every other pair of ids here: neither of them
    // naming a number cannot name a Screen.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return crate::ui::no_such_screen();
    };

    if state.sessions.screen(id, event).is_none() {
        return crate::ui::no_such_screen();
    }

    // Looked up again inside rather than carried in, and the same on every
    // resize: a session ends while it is being watched, and the register is what
    // knows. A `Live` held across the socket would be a session that had gone
    // still answering for one.
    watcher.on_upgrade(move |socket| watch(socket, state, id, event))
}

/// Follow one session's Screen down one socket until either end has finished
/// with it.
///
/// The repaint goes first, so a watcher joining an hour in sees the session
/// rather than the rest of it, and everything the session prints follows. What
/// comes back up is the watcher's window size, which is the Screen's size from
/// then on for everybody.
async fn watch(mut socket: WebSocket, state: AppState, conversation_id: i64, event_id: i64) {
    let Some(live) = state.sessions.screen(conversation_id, event_id) else {
        return;
    };

    let (painted, mut shown) = live.watched();

    // Nothing of the session is held past this line. What keeps the stream alive
    // is the relay's own copy, so the moment the session is over every watcher
    // hears the channel close rather than waiting on a Screen nothing will feed
    // again.
    drop(live);

    if say(&mut socket, painted).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            next = shown.recv() => match next {
                Ok(shown) => {
                    if say(&mut socket, shown).await.is_err() {
                        return;
                    }
                }
                // Too far behind to be caught up, so put right instead — see
                // [`WATCHER_BACKLOG`]. A session that ended in the meantime has
                // nothing to repaint from, and this watcher is about to hear so.
                Err(broadcast::error::RecvError::Lagged(missed)) => {
                    tracing::warn!(
                        conversation_id,
                        event_id,
                        missed,
                        "a watcher fell behind a Screen, so it is being repainted",
                    );

                    if let Some(live) = state.sessions.screen(conversation_id, event_id)
                        && say(&mut socket, live.painted()).await.is_err()
                    {
                        return;
                    }
                }
                // The session is over: every sender has gone with it.
                Err(broadcast::error::RecvError::Closed) => return,
            },

            said = socket.recv() => match said {
                Some(Ok(Message::Text(said))) => {
                    let Ok(Watching::Resized(size)) = serde_json::from_str(&said) else {
                        tracing::warn!(
                            conversation_id,
                            event_id,
                            "a watcher said something a Screen does not take",
                        );
                        continue;
                    };

                    if let Some(live) = state.sessions.screen(conversation_id, event_id) {
                        live.resized(size);
                    }
                }
                // A ping is answered by the transport underneath, and a binary
                // frame is nothing a watcher sends: the socket speaks JSON both
                // ways.
                Some(Ok(_)) => {}
                // The watcher has gone, or the connection has. Either way there
                // is nothing here to keep, and the session goes on exactly as it
                // was — see the Hold, which is the one thing watching can leave
                // behind and is not this.
                Some(Err(_)) | None => return,
            },
        }
    }
}

/// Send one message down the socket. An error is a watcher that has gone.
async fn say(socket: &mut WebSocket, shown: Shown) -> Result<(), ()> {
    // Serialising cannot fail for these — a repaint and a chunk of a session's
    // output are both strings — and a socket that could not be told is a socket
    // to give up on, which is what the caller does with either.
    let said = serde_json::to_string(&shown).map_err(|_| ())?;

    socket
        .send(Message::Text(said.into()))
        .await
        .map_err(|_| ())
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
