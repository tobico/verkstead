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
//! **The same condition in every state.** A grilling writing the artifact its
//! pick asked for, a backlog step, an inline implementation, an instruction, a
//! fix, a follow-up: each of them is a session that should be either working,
//! asking or finished, and *none of the three* is the shape this watches for. What differs from one to
//! the next is only what *finished* looks like — a path on the branch, a commit,
//! the human's own mark — so that is the parameter and the loop is not. See
//! [`Done`], and [`until_it_will_not_ask`], which is the whole of the mechanism.
//!
//! **And sessions legitimately waiting are never spoken to.** One sitting on a
//! Blocking Ask has a Set open, which is the middle third of the condition —
//! the Conversation's rather than the session's, because what the rescue is for
//! is a human with nothing in front of them. One still printing is not idle, and
//! anything it prints puts the whole grace back on the clock. And a session that
//! has landed what it was sent for is not spoken to either — the driver beside
//! this is already ending it.
//!
//! Nothing is written to the Timeline for the rescue itself. It is Verkstead
//! prodding an agent rather than anything the work has got to, and the session's
//! own Capture holds the line and whatever the agent made of it.

use std::path::PathBuf;
use std::time::Instant;

use crate::AppState;
use crate::runner::{Landing, Pace};
use crate::sessions::Quiet;

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
/// [`crate::sessions::Sessions::alive`] asks for and what keeps this from typing
/// into whatever is running now: the session being rescued is the one the caller
/// has been watching go quiet.
///
/// `false` is a session that is not there any more — it ended between the last
/// look and this one — which is not a rescue that failed but a rescue that had
/// nothing to rescue. The caller is waiting on that ending too, so what to do
/// about it is already in hand.
///
/// **Asked of the process rather than of the register**, which is the difference
/// between the two answers and matters exactly here: a session stays on the
/// register through its last sweep of the branch, and a line typed in over that
/// stretch would go into a terminal nothing is reading and be counted against a
/// session that had already finished.
pub(crate) async fn rescue(state: &AppState, conversation_id: i64, event_id: i64) -> bool {
    if !state.sessions.alive(conversation_id, event_id) {
        tracing::info!(
            conversation_id,
            event_id,
            "the session to be rescued had already ended, so nothing was typed into it",
        );
        return false;
    }

    let Some(screen) = state.sessions.screen(conversation_id, event_id) else {
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

/// What would say this session's work is done, which is the one thing about the
/// rescue that differs from one state to the next.
///
/// The condition is the same everywhere — a running session that is idle, with
/// nothing open on the Conversation, and nothing to show for itself — and only
/// the last third of it is a fact about the state. So it is a parameter rather
/// than four copies of the loop below, and a state added later brings an
/// indicator rather than a mechanism.
#[derive(Debug, Clone)]
pub(crate) enum Done {
    /// A grilling's artifact, or a backlog step's task file: the path is where
    /// it should be and git has nothing pending for it — see
    /// [`crate::runner::Landing`], which is the same reading the step is ended
    /// on.
    Landed {
        /// The Worktree the session is working in.
        worktree: PathBuf,

        /// And what landing looks like there.
        landing: Landing,
    },

    /// An instruction or a fix: more commits on the Conversation than it carried
    /// when the session started. There is no path to watch — an instruction can
    /// ask for anything — and a commit is the one report an agent cannot half
    /// make.
    Committed {
        /// What the Conversation had committed before the session started.
        already: usize,
    },

    /// A follow-up: the newest round the human answered carries the
    /// Nothing-else mark. Nothing on the branch says whether a follow-up is
    /// over, because what it commits is the human's to have asked for and a
    /// round that was a question and an answer commits nothing at all.
    NothingElse,
}

impl Done {
    /// Whether the session has anything to show for itself yet.
    ///
    /// Every one of these reads *not done* where it cannot be answered, which is
    /// the right way round for what it decides: on the other side is a line
    /// typed into a working session, and each of the three readers already
    /// errs that way for the ending it also decides.
    async fn reached(&self, state: &AppState, conversation_id: i64) -> bool {
        match self {
            Done::Landed { worktree, landing } => crate::runner::check(worktree, landing).await,
            Done::Committed { already } => {
                crate::runner::committed_since(state, conversation_id, *already).await
            }
            Done::NothingElse => crate::runner::marked(state, conversation_id).await,
        }
    }
}

/// Watch a running session for the one shape nothing else can move, speak to it
/// when it takes that shape, and return once it will not be talked out of it.
///
/// **Three things at once, and none of them is enough alone.** *Idle*, because a
/// session still printing is one at work — and anything it prints puts the whole
/// grace back on the clock, so one mid-sentence is never spoken to. *Nothing
/// open*, because a session sitting on a Blocking Ask is doing exactly what it
/// should: the ask blocks for as long as the human takes, and that may be the
/// next morning. *And not done*, because a session that has landed what it was
/// sent for is one the driver beside this is already ending — see [`Done`],
/// which is the whole of what differs from state to state.
///
/// The open Set is the Conversation's rather than this session's — see
/// [`crate::runner::open`]. What the rescue is for is a human with nothing in
/// front of them, and a human with something in front of them has it whoever
/// put it there.
///
/// An answer arriving starts the grace again, exactly as a rescue does and for
/// the same reason: a session that has just been given something to act on has
/// had no time to act on it yet.
///
/// **Returns only where the rescue is spent** — twice typed in, and still idle
/// with nothing open and nothing landed. What follows is the caller's, and it is
/// the same thing everywhere: the session is ended where it stands and the
/// Conversation stops with a Notice saying it would not ask. Otherwise this
/// never returns, which is what makes it an arm of the `select!` every driver
/// here waits on.
pub(crate) async fn until_it_will_not_ask(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    quiet: &Quiet,
    pace: Pace,
    done: Done,
) {
    // When the session was last stirred: a Set of the Conversation's seen open,
    // or a rescue typed in. Both start the grace again, and for the one reason —
    // each is something the session has just been given to act on. `None` while
    // neither has happened, which is a session that has asked nothing since it
    // started.
    let mut stirred: Option<Instant> = None;

    // How many times it has been told. Never reset: a session that asked and
    // then went quiet again has had its round, and the bound is on this
    // session's whole life rather than on a run of silences.
    let mut spent = 0;

    loop {
        // The cheap half first: a session still talking is not one to ask the
        // store or the Worktree about.
        let owed = pace.proposing.saturating_sub(quiet.for_how_long());

        if !owed.is_zero() {
            tokio::time::sleep(owed).await;
            continue;
        }

        if crate::runner::open(state, conversation_id).await {
            stirred = Some(Instant::now());
            tokio::time::sleep(pace.poll).await;
            continue;
        }

        let owed = stirred
            .map(|at| pace.proposing.saturating_sub(at.elapsed()))
            .unwrap_or_default();

        if !owed.is_zero() {
            tokio::time::sleep(owed).await;
            continue;
        }

        // Idle and silent, but with something to show for it: the driver beside
        // this is ending the session on exactly that, and a line typed into one
        // that has done its job would be Verkstead prodding an agent for
        // finishing.
        if done.reached(state, conversation_id).await {
            tokio::time::sleep(pace.poll).await;
            continue;
        }

        // Spent, and the session still there to have spent it on. One that has
        // gone in the meantime is the ending's to report and not this: the
        // driver beside this is waiting on it, and *it finished* is a truer
        // account of a session than *it would not ask*.
        if spent >= AT_MOST && state.sessions.alive(conversation_id, event_id) {
            return;
        }

        // Counted only where it reached a session. One that has ended between
        // the last look and this one is not a rescue that failed — the ending is
        // being waited on beside this, and it is the ending that decides.
        if rescue(state, conversation_id, event_id).await {
            spent += 1;
            stirred = Some(Instant::now());
        }

        tokio::time::sleep(pace.poll).await;
    }
}

/// What a stop over a session that would not ask says beyond what it was doing.
///
/// The rescue spent: it was idle with nothing open and nothing landed, it was
/// told twice that a Question Set is the whole of how the human is spoken to,
/// and it went on saying nothing. Which leaves a Conversation nobody can move —
/// nothing to answer and nothing to read — so it stops rather than sitting
/// there, and Resume is what the human has.
///
/// [`crate::stopping::Decided::Verkstead`] wherever it is written: Verkstead
/// looked at this session and decided it was not going to ask.
pub(crate) const WOULD_NOT_ASK: &str = "the session went quiet without asking you anything or finishing what it was doing, and \
     asked nothing after being told twice that a Question Set is the only thing that reaches you";
