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
//! only way in there is and which [`crate::typing`] is — putting to it the
//! moves that would end the silence: carrying on, where it has a next step;
//! running `verkstead done`, where it is a session ended on that signal and its
//! work is finished; and otherwise the one move that reaches the human at all,
//! which is where it has got to put to them as a Set. And it is told to declare
//! a wait where work of its own is still running in the background. An agent
//! that had finished its turn takes another one.
//!
//! **Both, because the line is sometimes wrong.** What it is read off is a
//! session watched from outside, and a session doing exactly what it should can
//! wear that shape for a moment — see [`watched`], which is mostly
//! the business of not being wrong. One told only to ask *asks*, and a Set that
//! nothing needed is noise on the human's phone in the middle of the work they
//! are being asked about. Told to carry on or to ask, a session that was never
//! stuck spends the line on one quiet turn and nobody is disturbed.
//!
//! **It escalates, and never stops a session.** A rescue the session answers —
//! the session seen at work since the line arrived — puts the count back to
//! nothing, however many times over its life that happens. Three in a row with
//! no answer and Verkstead tells the human instead: a Notice on the Timeline and
//! a push to their devices, which is what a stop sends, without the stop. The
//! session stays alive with its Worktree, the Conversation reads *blocked on
//! you*, and the rescue holds off until the session is seen working again —
//! one escalation per silence. What happens next is the human's: typing into
//! the Screen, steering, or pressing Stop. Any bound that ended the session
//! would be a guess about it read from outside, and it would go on killing a
//! session legitimately waiting on work of its own. See ADR-0018.
//!
//! **The same condition in every state.** A grilling writing the artifact its
//! pick asked for, a backlog step, an inline implementation, an instruction, a
//! fix, a follow-up: each of them is a session that should be either working,
//! asking or finished, and *none of the three* is the shape this watches for.
//! Finished is the same everywhere too — a Done signal given and borne out,
//! whatever the kind checks it against — so the loop takes the signal. See
//! [`watched`], which is the whole of the mechanism.
//!
//! **And sessions legitimately waiting are never spoken to.** One that has
//! declared a wait on work of its own is at work until the wait is over — see
//! [`crate::waiting`] — and one sitting on a Blocking Ask has a Set open, which
//! is the middle third of the condition —
//! the Conversation's rather than the session's, because what the rescue is for
//! is a human with nothing in front of them. One still at work is not idle —
//! which is its backend's judgement rather than one rule for all of them, see
//! [`crate::sessions::Idle`] — and anything that says it is working again puts
//! the whole grace back on the clock. And a session that
//! has finished is not spoken to either — the driver beside this is already
//! ending it. Finished is the kind's own reading: a session ended on its Done
//! signal has finished once it has given one, whatever is on the branch, so one
//! that landed its work and said nothing is exactly a session to speak to.
//!
//! **Nor is one that has been handed something and not yet said a word about
//! it.** An answer reaches a session down a chain Verkstead can see no hop of —
//! the CLI's long poll returning, the harness noticing its background command
//! exited, the model beginning its turn, the first bytes drawn — and a chain
//! slower than the grace is a session that was working perfectly well being told
//! it had gone quiet. So a *stir* — the session's launch, an answer arriving, a
//! rescue typed in — holds the rescue off until the session has said something
//! since, which is the one thing from out here that proves the stir landed. See
//! [`watched`], and [`crate::runner::Pace::waking`], which is the
//! ceiling on the holding off: a session that says nothing at all for that long
//! is one that died mid-wait, and it is rescued having never spoken.
//!
//! Nothing is written to the Timeline for the rescue itself. It is Verkstead
//! prodding an agent rather than anything the work has got to, and the session's
//! own Capture holds the line and whatever the agent made of it. The escalation
//! is the one thing written, because that is the human being told.

use std::time::{Duration, Instant};

use sqlx::SqlitePool;
use verkstead_schema::Nudge;

use crate::AppState;
use crate::done::Signal;
use crate::nudge::Nudges;
use crate::runner::Pace;
use crate::sessions::Idle;
use crate::store;

/// How many rescues in a row go unanswered before the human is told.
///
/// Three. Once is a turn that ended a moment early, and the line is enough to
/// start another; the same silence after three is something the human should
/// look at. Not a stop — the count is only how long Verkstead goes on speaking
/// to the session before it speaks to them instead.
pub(crate) const UNANSWERED: usize = 3;

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
/// get on with it, one whose work is finished is told to say so, and only a
/// session that is actually waiting on the human is told to say so as a Set.
/// Which is what makes a wrongly typed line cheap — one quiet turn, rather than
/// a Question Set manufactured for a human who did not need one. A session that
/// has done its work and not said so is exactly one to speak to now, because
/// nothing else ends it. And a session waiting on a build of its own that forgot
/// to say so is told how to, rather than prodded into doing something else.
///
/// One line and no newline of its own. The Enter is [`crate::typing`]'s, and a
/// line broken over two would be submitted half-written.
pub(crate) const LINE: &str = "If you have your next step, carry on with it now. If your work is finished, run \
     `verkstead done`. If background work you started is still running, run `verkstead waiting` \
     and keep waiting. If you are blocked or waiting on me, summarize your status and ask me \
     what to do next via `verkstead ask`.";

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
pub(crate) async fn rescue(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    line: &str,
) -> bool {
    if !crate::typing::typed(state, conversation_id, event_id, line).await {
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

/// Watch a running session for the one shape nothing else can move, speak to it
/// when it takes that shape, and tell the human when it will not be talked out
/// of it.
///
/// **Three things at once, and none of them is enough alone.** *Idle*, because a
/// session still printing is one at work — and anything it prints puts the whole
/// grace back on the clock, so one mid-sentence is never spoken to. A session
/// with a Declared wait standing is at work by the same judgement, so it is
/// never spoken to either, however quiet it is. *Nothing
/// open*, because a session sitting on a Blocking Ask is doing exactly what it
/// should: the ask blocks for as long as the human takes, and that may be the
/// next morning. *And not done*, because a session that has said so is one the
/// driver beside this is already ending — see [`crate::done`].
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
/// time to act on it yet. What follows a stir is the session's own first word:
/// it may take as long as it takes, and the grace begins from what it says. The
/// question was never *how long has it been* but *did the thing we handed it
/// get there*, and a word is the only answer to that from out here.
///
/// The ceiling on that is [`Pace::waking`], because a stir a session never
/// answers is exactly what a session dying mid-wait looks like. One that has said
/// nothing at all since the stir is rescued when it passes, having never spoken.
///
/// **A rescue answered puts the count back to nothing.** Answered is the session
/// seen at work since the line finished arriving — the stir's own reading, so
/// the echo of the line is not an answer. [`UNANSWERED`] in a row with no answer
/// and the human is told instead — see [`escalate`] — and the rescue holds off,
/// typing nothing and telling nobody again, until the session is seen at work.
/// That takes the escalation away and rearms the whole of it.
///
/// **Never returns**: the session ending is what ends this, as the arm of the
/// `select!` every driver here waits on that loses. Which is also when an
/// escalation standing goes — see [`Escalation`].
pub(crate) async fn watched(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
    idle: &Idle,
    pace: Pace,
    signal: Signal,
    what: &str,
) -> std::convert::Infallible {
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

    // How many rescues in a row have gone unanswered, and the moment the last
    // one finished arriving — which is what an answer is read against.
    let mut unanswered = 0;
    let mut rescued: Option<Instant> = None;

    // The human told about this silence, where they have been. Taken away as
    // the session is seen at work again, or with this loop as the session ends.
    let mut escalation = Escalation::none(state, conversation_id);

    loop {
        // Answered: seen at work since the last rescue arrived. The count goes
        // back to nothing, and so does anything the human was told about the
        // silence it answered.
        if rescued.is_some_and(|at| idle.since() > at) {
            unanswered = 0;
            rescued = None;
            escalation.settle().await;
        }

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

        // Told already, about this same silence: nothing more is typed and
        // nobody is told twice until the session is seen at work, which is the
        // top of this loop's to notice.
        if escalation.standing() {
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

        // Idle and silent, having said it is done: the driver beside this is
        // ending the session on exactly that, and a line typed into one that has
        // done its job would be Verkstead prodding an agent for finishing.
        if signal.given() {
            tokio::time::sleep(pace.poll).await;
            continue;
        }

        // Spoken to as many times as it will be, and the session still there to
        // have been spoken to. One that has gone in the meantime is the ending's
        // to report: the driver beside this is waiting on it.
        if unanswered >= UNANSWERED {
            if state.sessions.alive(conversation_id, event_id) {
                escalation.escalate(event_id, what).await;
            }

            tokio::time::sleep(pace.poll).await;
            continue;
        }

        // Counted only where it reached a session. One that has ended between
        // the last look and this one is not a rescue that went unanswered — the
        // ending is being waited on beside this, and it is the ending that
        // decides.
        if rescue(state, conversation_id, event_id, LINE).await {
            unanswered += 1;

            // Once what was typed has finished arriving back, rather than as it
            // was typed — see [`after_the_echo`]. A terminal echoes, so a stir
            // taken at the last keystroke is one the keystrokes answer
            // themselves.
            stirred = after_the_echo(idle).await;
            rescued = Some(stirred);
        }

        tokio::time::sleep(pace.poll).await;
    }
}

/// What the human is told about a session that went unanswered, and the mark it
/// leaves on the Conversation for as long as that lasts.
///
/// Owned by the loop that wrote it, so that it goes with the loop: a session
/// ended — by its signal, by itself, or by the human pressing Stop — drops the
/// loop watching it, and *blocked on you* over a session that is no longer
/// there would be a mark with nothing behind it.
pub(crate) struct Escalation {
    pool: SqlitePool,
    nudges: Nudges,
    conversation_id: i64,

    /// Whether the human has been told about this silence.
    told: bool,

    /// The Notice that told them, where this loop wrote it. `told` without one
    /// is an escalation that was already standing over the Conversation when
    /// this went to write: somebody has been told about this silence, and the
    /// mark is not this loop's to take away again.
    notice: Option<i64>,
}

impl Escalation {
    fn none(state: &AppState, conversation_id: i64) -> Self {
        Self {
            pool: state.pool.clone(),
            nudges: state.nudges.clone(),
            conversation_id,
            told: false,
            notice: None,
        }
    }

    fn standing(&self) -> bool {
        self.told
    }

    /// Tell the human: a Notice on the Timeline, the *blocked on you* mark over
    /// it, and a push to their devices. What a stop Verkstead decided on sends,
    /// without the stop — nothing is ended, and nothing is written as stopped.
    ///
    /// **Told is what the store took rather than what was attempted.** A write
    /// that fell over left nothing on the Timeline, nothing on the phone and no
    /// mark on the Conversation — so counting it as told would hold the whole of
    /// this off for the rest of the silence, no line typed into the session ever
    /// again and nobody told, over a session still sitting there holding its
    /// Worktree. Which is the one outcome this exists to prevent. So a failure is
    /// left to the next poll, and only a write that landed says the human knows.
    async fn escalate(&mut self, event_id: i64, what: &str) {
        let conversation_id = self.conversation_id;

        let said = format!(
            "**{}** has gone idle without finishing.\n\n{ESCALATED}\n\n{}",
            crate::stopping::opening(what),
            crate::stopping::evidence(
                &crate::stopping::worktree_status(&self.pool, conversation_id).await,
                &crate::stopping::session_tail(&self.pool, conversation_id, Some(event_id)).await,
            ),
        );

        match store::escalate(&self.pool, conversation_id, &said).await {
            Ok(Some(notice)) => {
                self.told = true;

                tracing::warn!(
                    conversation_id,
                    event_id,
                    notice,
                    "the session went unanswered after being spoken to {UNANSWERED} times, so \
                     the human is told and the session left running",
                );

                self.notice = Some(notice);

                self.nudges.announce(Nudge::Conversation {
                    conversation: conversation_id,
                });

                crate::push::told(
                    &self.pool,
                    conversation_id,
                    crate::push::News::Escalated {
                        idle: crate::stopping::opening(what),
                    },
                );
            }
            // One standing already, which is not this loop's: the human has been
            // told about this silence, so there is nothing to add and nothing
            // more to type into the session.
            Ok(None) => {
                self.told = true;

                tracing::info!(
                    conversation_id,
                    event_id,
                    "the session went unanswered, and an escalation already stands",
                );
            }
            // Nothing was written, so nobody has been told: left unsaid rather
            // than taken as said, and gone at again on the next poll.
            Err(error) => tracing::error!(
                error = ?error,
                conversation_id,
                event_id,
                "telling the human about a session gone unanswered failed, so it is tried \
                 again",
            ),
        }
    }

    /// The session is seen at work: take the mark away and rearm.
    async fn settle(&mut self) {
        self.told = false;

        if let Some(notice) = self.notice.take() {
            settled(&self.pool, &self.nudges, self.conversation_id, notice).await;
        }
    }
}

impl Drop for Escalation {
    fn drop(&mut self) {
        let Some(notice) = self.notice.take() else {
            return;
        };

        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };

        let (pool, nudges, conversation_id) =
            (self.pool.clone(), self.nudges.clone(), self.conversation_id);

        runtime.spawn(async move { settled(&pool, &nudges, conversation_id, notice).await });
    }
}

/// Take one escalation's mark away, and redraw an open page where it went.
///
/// The Notice stays on the Timeline: it is what happened.
async fn settled(pool: &SqlitePool, nudges: &Nudges, conversation_id: i64, notice: i64) {
    match store::settle_escalation(pool, conversation_id, notice).await {
        Ok(true) => nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        }),
        Ok(false) => {}
        Err(error) => tracing::error!(
            error = ?error,
            conversation_id,
            notice,
            "taking away the mark over a session gone unanswered failed",
        ),
    }
}

/// What the escalation's Notice says beyond what was being done.
///
/// What happened, that nothing was stopped, and what the human can do about it
/// — the three moves that are theirs, the session being still there to move.
pub(crate) const ESCALATED: &str = "The session went idle without saying it is done, asking you \
     anything or declaring a wait, and did not answer when it was spoken to three times. It is \
     still running, with its worktree. You can type into its Screen, steer the Conversation, or \
     press **Stop**.";
