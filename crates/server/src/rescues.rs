//! The rescue: a line typed into a session that has gone idle without asking.
//!
//! A session Verkstead launched is one the human reaches in exactly one way —
//! the Question Set it sends them. So a session that goes quiet with no Set open
//! and nothing to show for itself is a Conversation nobody can move: the human
//! cannot answer, because nothing was asked, and they cannot end it either. The
//! agent is sitting there with the turn finished and nothing to do, which from
//! this side looks the same as a session that has died and is not one.
//!
//! **So it is spoken to.** Verkstead types a canned line into the running
//! session — through the terminal a watcher's keystrokes go through, which is the
//! only way in there is — telling it plainly what it cannot see from inside: that
//! nothing it prints reaches anybody, and that a Set is the whole of how the
//! human is spoken to. An agent that had finished its turn takes another one.
//!
//! **Twice at most**, because the second time it fails to work is evidence rather
//! than bad luck. What follows is a stop like any other: the Conversation lands
//! in front of the human with a Notice saying the session would not ask, and
//! Resume is what they have.
//!
//! Nothing is written to the Timeline for the rescue itself. It is Verkstead
//! prodding an agent rather than anything the work has got to, and the session's
//! own Capture holds the line and whatever the agent made of it.

use crate::AppState;

/// How many times one session is spoken to before Verkstead stops asking.
///
/// Two, and the third time round is the stop. Once is a turn that ended a moment
/// early and the line is enough to start another; twice with the same silence
/// after it is a session that is not going to ask, whatever it is told.
pub(crate) const AT_MOST: usize = 2;

/// What is typed in.
///
/// Written to the agent as the human would write it, because that is what it is:
/// a line arriving at the session's own terminal, indistinguishable from one
/// somebody watching had typed. What it says is the one thing an agent cannot
/// find out from inside its own session — that the screen it is printing to has
/// nobody in front of it.
///
/// One line and no newline of its own. The Enter is [`rescue`]'s, and a line
/// broken over two would be submitted half-written.
pub(crate) const LINE: &str = "I am not at this terminal and nothing you print here reaches me — the only thing I ever \
     see is a Question Set you send with `verkstead ask`. You have gone quiet with none of them \
     open, so there is nothing here for me to answer and no way for me to say we are done. Put \
     what you are waiting on to me as a Set now, with an ordinary postscript under it.";

/// Type [`LINE`] into the session, and say whether it reached one.
///
/// By the Event as well as by the Conversation, which is what
/// [`crate::sessions::Sessions::screen`] asks for and what keeps this from
/// typing into whatever is running now: the session being rescued is the one the
/// caller has been watching go quiet.
///
/// `false` is a session that is not there any more — it ended between the last
/// look and this one — which is not a rescue that failed but a rescue that had
/// nothing to rescue. The caller is waiting on that ending too, so what to do
/// about it is already in hand.
pub(crate) async fn rescue(state: &AppState, conversation_id: i64, event_id: i64) -> bool {
    let Some(screen) = state.sessions.screen(conversation_id, event_id) else {
        tracing::info!(
            conversation_id,
            event_id,
            "the session to be rescued had already ended, so nothing was typed into it",
        );
        return false;
    };

    // The carriage return an Enter arrives as, which is what a terminal
    // application reads a line on — see the viewer's Screen, whose keystrokes
    // take this same path.
    screen.put_in(&format!("{LINE}\r")).await;

    tracing::info!(
        conversation_id,
        event_id,
        "the session had gone idle without asking, so it was told to put what it is waiting \
         on to the human",
    );

    true
}
