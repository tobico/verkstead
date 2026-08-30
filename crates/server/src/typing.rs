//! Typing into a running session: the one way into one there is.
//!
//! A session Verkstead launched reads its terminal and nothing else. There is no
//! port on it, no file it watches and no signal it takes — so everything
//! Verkstead has to *say* to a session goes in as keystrokes, down the channel a
//! watcher's own typing takes.
//!
//! Two things say something. The **rescue** tells a session that has gone idle
//! without asking to carry on or else put what it is waiting on to the human —
//! see [`crate::rescues`]. The **nudge** tells one idling on a store-and-nudge
//! ask that its Answers are there to fetch — see [`crate::nudging`]. What they
//! say differs and everything about the saying is the same, which is why it is
//! here rather than in either of them.
//!
//! **And what is the same is two costs the channel has.** The line and its
//! carriage return are typed a moment apart, because an agent's terminal
//! interface reads a burst as a paste and a paste's return is a line break in
//! what is being written rather than a send — see [`BEFORE_THE_ENTER`]. And the
//! terminal echoes what is typed, so anything read straight back off the session
//! is the keyboard rather than the session — see [`AFTER_THE_ECHO`], which is
//! what a caller reading the session for a reply has to wait out first.

use std::time::Duration;

use crate::AppState;

/// How long the line is left sitting in the session's composer before the Enter
/// is typed after it.
///
/// **Because an agent's terminal interface reads a burst as a paste.** The
/// interfaces Verkstead launches take a line and its carriage return arriving in
/// one read for pasted text, and a return inside pasted text is a line break in
/// what is being written rather than a send — which leaves what was typed
/// sitting in the composer unsent and the session as quiet as it was. Typed a
/// moment apart they are two keystrokes, which is what they are meant to be.
///
/// Long enough for the interface to have drawn the line and short enough that
/// nothing else could have happened in between: this is one turnaround of a
/// terminal, not a wait on anything.
const BEFORE_THE_ENTER: Duration = Duration::from_millis(250);

/// And how long after the Enter what was typed is still arriving back.
///
/// **Because a terminal says what is typed into it.** The keystrokes go in and
/// are echoed straight back out — by the line discipline where the agent left it
/// alone, and by the interface's own composer where it did not — so the session
/// appears to say the very words Verkstead just said to it, within a moment of
/// their being said. Which is nothing about the session: a process that has hung
/// with its terminal open echoes exactly as well as one that is about to take a
/// turn.
///
/// So a caller that reads what the session says next has to let this pass first
/// — see [`crate::rescues::until_it_will_not_ask`], where a stir the typing
/// itself answered would be no stir at all.
///
/// One turnaround of a terminal again, for [`BEFORE_THE_ENTER`]'s reason: an
/// echo is drawn as it is read, and anything arriving later than this is the
/// session rather than the keyboard.
pub(crate) const AFTER_THE_ECHO: Duration = Duration::from_millis(250);

/// Type `line` into the session, and say whether it reached one.
///
/// By the Event as well as by the Conversation, which is what
/// [`crate::sessions::Sessions::alive`] asks for and what keeps this from typing
/// into whatever is running now: the session being spoken to is the one the
/// caller has been watching.
///
/// `false` is a session that is not there any more — it ended between the last
/// look and this one — which is not a failure but a line that had nobody to
/// arrive at. What to make of that is the caller's: the rescue is waiting on
/// that ending anyway, and the nudge has the folding rule behind it.
///
/// **Asked of the process rather than of the register**, which is the difference
/// between the two answers and matters exactly here: a session stays on the
/// register through its last sweep of the branch, and a line typed in over that
/// stretch would go into a terminal nothing is reading.
///
/// One line and no newline in it. The Enter is this function's, and a line
/// broken over two would be submitted half-written.
pub(crate) async fn typed(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    line: &str,
) -> bool {
    if !state.sessions.alive(conversation_id, event_id) {
        return false;
    }

    let Some(screen) = state.sessions.screen(conversation_id, event_id) else {
        return false;
    };

    // The line first, and the carriage return an Enter arrives as a moment
    // behind it — see [`BEFORE_THE_ENTER`], which is why the two are not one
    // write. Both take the path a watcher's keystrokes take, which is the whole
    // of the way into a running session.
    screen.put_in(line).await;
    tokio::time::sleep(BEFORE_THE_ENTER).await;
    screen.put_in("\r").await;

    true
}
