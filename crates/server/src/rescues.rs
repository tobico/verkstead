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
//! only way in there is and which [`crate::typing`] is — putting to it the two
//! moves that would end the silence: carrying on, where it has a next step, and
//! otherwise the one move that reaches the human at all, which is where it has
//! got to put to them as a Set. An agent that had finished its turn takes
//! another one.
//!
//! **Both, because the line is sometimes wrong.** What it is read off is a
//! session watched from outside, and a session doing exactly what it should can
//! wear that shape for a moment — see [`until_it_will_not_ask`], which is mostly
//! the business of not being wrong. One told only to ask *asks*, and a Set that
//! nothing needed is noise on the human's phone in the middle of the work they
//! are being asked about. Told to carry on or to ask, a session that was never
//! stuck spends the line on one quiet turn and nobody is disturbed.
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
//! is a human with nothing in front of them. One still at work is not idle —
//! which is its backend's judgement rather than one rule for all of them, see
//! [`crate::sessions::Idle`] — and anything that says it is working again puts
//! the whole grace back on the clock. And a session that
//! has landed what it was sent for is not spoken to either — the driver beside
//! this is already ending it.
//!
//! **Nor is one that has been handed something and not yet said a word about
//! it.** An answer reaches a session down a chain Verkstead can see no hop of —
//! the CLI's long poll returning, the harness noticing its background command
//! exited, the model beginning its turn, the first bytes drawn — and a chain
//! slower than the grace is a session that was working perfectly well being told
//! it had gone quiet. So a *stir* — the session's launch, an answer arriving, a
//! rescue typed in — holds the rescue off until the session has said something
//! since, which is the one thing from out here that proves the stir landed. See
//! [`until_it_will_not_ask`], and [`crate::runner::Pace::waking`], which is the
//! ceiling on the holding off: a session that says nothing at all for that long
//! is one that died mid-wait, and it is rescued having never spoken.
//!
//! Nothing is written to the Timeline for the rescue itself. It is Verkstead
//! prodding an agent rather than anything the work has got to, and the session's
//! own Capture holds the line and whatever the agent made of it.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::AppState;
use crate::runner::{Landing, Pace};
use crate::sessions::Idle;

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
/// somebody watching had typed.
///
/// **Conditional, because it is typed on a guess.** Everything a line like this
/// could say about the session is read from outside it, and outside is where a
/// session that is working and one that is stuck look alike. So it names the
/// condition rather than the move: a session that has its next step is told to
/// get on with it, and only a session that is actually waiting on the human is
/// told to say so as a Set. Which is what makes a wrongly typed line cheap —
/// one quiet turn, rather than a Question Set manufactured for a human who did
/// not need one.
///
/// One line and no newline of its own. The Enter is [`crate::typing`]'s, and a
/// line broken over two would be submitted half-written.
pub(crate) const LINE: &str = "If you have your next step, carry on with it now. If you are blocked or waiting on me, \
     summarize your status and ask me what to do next via `verkstead ask`.";

/// How long the wait for that echo goes on for before the stir is taken anyway.
///
/// The ceiling on [`after_the_echo`], and there are two terminals it is for: the
/// one that takes the keystrokes and draws nothing back for them, where there is
/// no echo to wait out at all; and the one whose session took the line and got
/// straight on with its work, which is a session printing for reasons of its own
/// and no longer anything this loop is waiting to see.
///
/// Long against a turnaround, because what it bounds is the machine rather than
/// the terminal: an echo is written the moment the keystroke is, and seconds
/// later is not an echo running late but a terminal that was never going to send
/// one.
const FOR_THE_ECHO: Duration = Duration::from_secs(2);

/// Type [`LINE`] into the session, and say whether it reached one.
///
/// By the Event as well as by the Conversation, which is what
/// [`crate::typing::typed`] asks for and what keeps this from typing into
/// whatever is running now: the session being rescued is the one the caller has
/// been watching go quiet.
///
/// `false` is a session that is not there any more — it ended between the last
/// look and this one — which is not a rescue that failed but a rescue that had
/// nothing to rescue. The caller is waiting on that ending too, so what to do
/// about it is already in hand.
pub(crate) async fn rescue(state: &AppState, conversation_id: i64, event_id: i64) -> bool {
    if !crate::typing::typed(state, conversation_id, event_id, LINE).await {
        tracing::info!(
            conversation_id,
            event_id,
            "the session to be rescued had already ended, so nothing was typed into it",
        );
        return false;
    }

    tracing::info!(
        conversation_id,
        event_id,
        "the session had gone idle without asking, so it was told to put what it is waiting \
         on to the human",
    );

    true
}

/// Wait for what was just typed to finish arriving back, and give the moment it
/// had as the stir.
///
/// **Watched rather than waited out.** What the stir was taken after used to be
/// a fixed pause, long enough for an echo on a machine with nothing else on it —
/// and a pause is a guess at something that can be looked at instead. An echo a
/// moment slower than the guess landed *after* the stir, where it reads as the
/// session stirring: the second rescue then armed on the bare grace the first
/// one did, which is the one thing the stir is there to prevent.
///
/// So it waits for the terminal to stir and then to settle. A stir later than
/// the keystrokes is the echo arriving, [`crate::typing::AFTER_THE_ECHO`] with
/// nothing after it is the echo all in, and the stir is taken from there — which
/// is after every word the keyboard put in the session's mouth, however long the
/// machine took to carry them.
///
/// Read off [`Idle::since`] rather than off the bare byte clock, because that is
/// what the caller compares the stir against: on a backend judged by its screen
/// the echo is what takes the prompt off it, and the moment that reading moves
/// is the moment the keystrokes landed.
///
/// [`FOR_THE_ECHO`] bounds the whole of it, for the terminals that never settle.
async fn after_the_echo(idle: &Idle) -> Instant {
    let typed = Instant::now();

    while typed.elapsed() < FOR_THE_ECHO {
        let seen = idle.since();

        tokio::time::sleep(crate::typing::AFTER_THE_ECHO).await;

        // Something since the keystrokes, and nothing after it in the window
        // just waited out: what was typed has come back and the terminal has
        // gone quiet behind it.
        if seen > typed && idle.since() == seen {
            break;
        }
    }

    Instant::now()
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

    /// An instruction or a fix: the Conversation's commits standing somewhere
    /// past where they stood when the session started. There is no path to watch
    /// — an instruction can ask for anything — and a commit is the one report an
    /// agent cannot half make.
    Committed {
        /// Where the Conversation's commits stood before the session started —
        /// see [`store::commits_landed`], which is what a marker like this
        /// means.
        already: i64,
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
/// **And it is asked about every poll rather than once the grace is out**, alone
/// among the loops here, because this one reads it for two things: whether the
/// human is holding a question now, and when they were last handed an answer to
/// give. The second is only ever seen by looking while it is happening.
///
/// **And it waits for a word after every stir.** A session's launch, an answer
/// arriving, and a line typed in by this loop are all something it has just been
/// given to act on, and a session that has just been given something has had no
/// time to act on it yet. What used to follow a stir was the grace over again,
/// which was a guess at how long the answer takes to arrive at a session — down
/// a chain of hops Verkstead cannot see one of, and one that is slower than the
/// grace more often than it looks. What follows a stir now is the session's own
/// first word: it may take as long as it takes, and the grace begins from what
/// it says. Which is the near half of the condition read properly rather than a
/// longer number — the question was never *how long has it been* but *did the
/// thing we handed it get there*, and a word is the only answer to that from out
/// here.
///
/// The ceiling on that is [`Pace::waking`], because a stir a session never
/// answers is exactly what a session dying mid-wait looks like. One that has said
/// nothing at all since the stir is rescued when it passes, having never spoken.
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
    idle: &Idle,
    pace: Pace,
    done: Done,
) {
    // When the session was last stirred: a Set of the Conversation's seen open,
    // a rescue typed in, or the moment this began. Each is something the session
    // has just been given to act on, and none of them is a moment it can be
    // judged from until it has said a word since.
    //
    // Now, because being watched starts at a stir every time. Every caller here
    // reaches this either straight after launching a session — the launch being
    // the stir — or, where a grilling is being seen out, straight after the pick
    // that gave it its direction was handed back to it.
    let mut stirred = Instant::now();

    // How many times it has been told. Never reset: a session that asked and
    // then went quiet again has had its round, and the bound is on this
    // session's whole life rather than on a run of silences.
    let mut spent = 0;

    loop {
        // The store first, and every poll — which is the one place here that
        // does not put the cheap half first, and it is deliberate. What this
        // asks is not only whether the human has something in front of them
        // now but *when they last did*, and the last look that saw a Set open
        // is the whole of what says an answer arrived. A Set put up and
        // answered inside the grace — the human picking within the minute,
        // which is most of the picks there are — is one that was never open at
        // any look taken after the grace, so a loop that only looked then would
        // see a session that had never been stirred at all and type its line
        // into one that had been answered seconds ago. It costs an indexed read
        // a poll, beside the git the step's own watcher runs at the same
        // cadence.
        if crate::runner::open(state, conversation_id).await {
            // The last look that saw it open, rather than the answer itself,
            // which is the same moment to within `pace.poll`.
            stirred = Instant::now();
            tokio::time::sleep(pace.poll).await;
            continue;
        }

        // Then how long it has been idle, in poll-sized steps rather than in one
        // sleep to the end of the grace: what is above has to be asked all the
        // way through it, and a session working its way past the grace is one
        // this comes back to anyway. Which backend's reading of idle that is is
        // the session's own — see [`crate::sessions::Idle`].
        let owed = pace.proposing.saturating_sub(idle.for_how_long());

        if !owed.is_zero() {
            tokio::time::sleep(owed.min(pace.poll)).await;
            continue;
        }

        // Idle, and nothing open — but not seen at work since it was last
        // stirred, so nothing yet says the stir ever arrived. Which is the
        // shape a session wears while the answer is still on its way to it, and
        // the shape it wears having died waiting for one. They are told apart
        // by waiting: the first goes back to work and the second does not.
        //
        // Seen at work rather than heard from, because a byte is free on a
        // backend that repaints for ever: it is the same judgement the grace
        // above is measured by, read as a moment.
        if idle.since() <= stirred {
            let owed = pace.waking.saturating_sub(stirred.elapsed());

            if !owed.is_zero() {
                tokio::time::sleep(owed).await;
                continue;
            }
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

            // Once what was typed has finished arriving back, rather than as it
            // Once what was typed has finished arriving back, rather than as it
            // was typed — see [`after_the_echo`]. A terminal echoes, so a stir
            // taken at the last keystroke is one the keystrokes answer
            // themselves.
            stirred = after_the_echo(idle).await;
        }

        tokio::time::sleep(pace.poll).await;
    }
}

/// What a stop over a session that would not ask says beyond what it was doing.
///
/// The rescue spent: it was idle with nothing open and nothing landed, it was
/// twice told to carry on or else to say where it had got to and put the next
/// move to the human, and it did neither. Which leaves a Conversation nobody
/// can move — nothing to answer and nothing to read — so it stops rather than
/// sitting there, and Resume is what the human has.
///
/// [`crate::stopping::Decided::Verkstead`] wherever it is written: Verkstead
/// looked at this session and decided it was not going to ask.
pub(crate) const WOULD_NOT_ASK: &str = "the session went quiet without asking you anything or finishing what it was doing, and \
     went on saying nothing after being told twice to carry on with its next step or else say \
     where it had got to and ask you what to do next";
