//! The wrap-up self-review: the one session that reads the branch whole, and
//! what becomes of what it finds.
//!
//! There are no per-commit review states anywhere in Verkstead. Commits are
//! events to read, and this is where problems get raised instead — once, about a
//! branch, by a session with fresh context. The sessions that wrote the work each
//! saw one task and none of them saw the pull request; this one sees nothing else.
//!
//! **It proposes, and then it fixes what was agreed to.** What it produces first
//! is one Question Set on the Timeline, a Question per finding, offering each
//! credible way of fixing that finding as an Option of its own with leaving it
//! alone always among them — which is what puts the human in the loop without
//! putting them at a terminal. Then it stays where it is: the ask blocks until
//! they answer, and when the answers come back the same session fixes each
//! finding they accepted, the way they chose, commits, pushes and ends. A
//! finding they declined is never raised again.
//!
//! One session for the lot of it, because a handful of fixes is not a handful of
//! pieces of work. The session that raised them is the one that read the branch,
//! and the one whose findings the human answered with words beside them; a fresh
//! session per finding would throw all of that away and re-read the diff to get
//! half of it back.
//!
//! **What is already on the pull request is part of what it reads.** A human who
//! commented before the review started has said something about this branch, and
//! this is the session that reads the branch — so those comments go into its
//! prompt whole and what they ask for is proposed in the same Set, beside the
//! findings it made itself. Which is what stops them being acted on ungated: they
//! are recorded as addressed as this session is dispatched, so no batch session is
//! later sent to do what nobody agreed to. See [`crate::comments::for_the_review`].
//!
//! A review that finds nothing and was given nothing asks nothing. It says so as
//! the last thing it prints — which is what the Timeline shows of a session — and
//! ends. A Set with no findings in it would be a row for the human to dismiss,
//! and the point of the phase is to spend their attention only where there is a
//! decision. Comments asking for work are that decision's other source: a review
//! with nothing of its own to raise still proposes about them.
//!
//! **The review settles when its session ends cleanly**, which is the one moment
//! everything it was sent to do is certainly over: the branch read, the findings
//! put, whatever the human picked carried out. Answering the Set settles nothing
//! — the Response is what the session acts on, and it is still acting when it
//! arrives.
//!
//! Nothing is asked of the record about what it did with the answers. The
//! session is the one thing that read what the human picked and the one thing
//! that carried it out, so *it ended cleanly* is the whole of the report: a
//! Verkstead that read the picks back and audited the branch against them would
//! be second-guessing the only participant that was there.
//!
//! **A finding too big for the sitting can be split out instead.** Where the
//! review judges one more work than it can do between the answers and its push,
//! it offers that as an Option of its own — and where the human picks it, what
//! the session writes for that finding is a `.tasks/` backlog rather than a fix.
//!
//! **The backlog on the branch is what says so**, and it is the only thing that
//! does. A review session that ends cleanly having committed a fresh `.tasks/`
//! list is a Conversation with work to build: it goes back down the ladder,
//! Wrapping to Implementing, the list worked a session at a time as any other
//! is, and the finish that follows the last task wraps it up again on the pull
//! request it already had, reviewed afresh. See
//! [`crate::runner::build_the_split_out`].
//!
//! Read off the Worktree rather than off what was picked, because the Worktree
//! is where the picking landed: the session was answered, wrote what it was
//! answered into writing and committed it, and a list sitting on the branch is
//! that having happened. An Option nothing was ever written for is a session
//! that changed its mind between the ask and the doing, which is its to change.
//!
//! **A review whose session is gone is a stop.** Two things arrive there — a
//! session that fell over, and a server that came back up over a wrap-up whose
//! review was still running — and both are the same fact, because a session
//! lives and dies with the process that started it. Either way the one thing
//! that could have finished this review is not there, and no other session is
//! ever sent to finish somebody else's: what was decided, what was half done and
//! what was never started are all beyond asking, so the run stops and the human
//! says what happens next.
//!
//! Resume is the review over from the start, in a session as fresh as the first.
//! It reads the branch as it now stands — anything the gone session did land
//! included — so whatever is still worth raising is raised again, and whatever
//! was already put right is not.
//!
//! **And nothing the human was asked is left standing behind it.** The
//! propose-then-fix shape has one session hold the whole of a review, its ask
//! included, so a session that goes between the asking and the answering leaves
//! a Set on the Timeline with nobody behind it: nothing is coming to read what
//! the human says into it, and no other session is ever handed somebody else's
//! ask. Any Set a gone session left unanswered is closed as the stop is raised,
//! which says on the Timeline that the question is off — and is what makes
//! Resume mean something, since a Set left standing would still be this wrap's
//! review and the fresh reading would be a second review nothing recognised.
//!
//! So the review is asked about every time anything starts a wrap-up's watchers,
//! rather than only where nobody has read the branch — and the press is the one
//! of them that reads past what it finds rather than stopping over it a second
//! time, because the press is the human having read the Notice and asked for
//! another go. See [`afresh`].
//!
//! **One agent in one Worktree**, which is what the turns are for. The checks are
//! being watched at the same time as this runs, and a fix session dispatched
//! mid-review would end the review where it stood — starting a session for a
//! Conversation ends the one it already has. So the review waits for the
//! Worktree and holds it until its session is done, the wait on the human
//! included, and the checks watcher tries for it and comes back later.
//!
//! Which leaves a check that goes red mid-wait with nobody to fix it, so **the
//! woken session fixes it too**: the reviewing skill has it read the pull
//! request's own check state once the answers arrive and put right whatever is
//! failing, beside the findings they accepted and before its push. Queueing it
//! instead would be a fix session dispatched hours later about a suite nobody had
//! looked at since. It costs the check none of its two attempts — those are spent
//! where a session is dispatched, and the watcher never dispatched this one — so
//! anything still red once the Worktree is free meets the watcher's ordinary
//! flow, whole. See [`crate::checks`].

use verkstead_schema::Nudge;

use crate::AppState;
use crate::runner::Reviewed;
use crate::store;

/// See to `conversation_id`'s review: read the branch where nobody has, and pick
/// up what a review whose session is gone left behind.
///
/// Both, because this is spawned by everything that might have left a wrap-up
/// with no review running — the finish step opening the pull request, and a
/// server coming back up over a Conversation it left wrapping — and *no review
/// running* is two different situations. One is a branch nobody has read. The
/// other is a review whose session is no longer there to finish it, which no
/// amount of waiting resolves by itself and which stops the run.
///
/// A Resume the human pressed goes to [`afresh`] instead, which is the same
/// entry with the second of those written off: they have read what stopped and
/// asked for another go.
///
/// Returns as soon as there is neither — a review already settled, a Conversation
/// that has stopped wrapping up, or driving that has stopped. None of those is
/// a failure: most of the time something else has already seen to it.
///
/// Nothing is refused for. This runs unattended with nobody watching, and what it
/// has to say it says on the Timeline or in the log.
pub(crate) async fn run(state: AppState, conversation_id: i64) {
    if matches!(wanted(&state, conversation_id).await, Wanted::Nothing) {
        return;
    }

    // Waited for rather than tried for: nothing else will start this review on
    // its behalf, so a Worktree busy with a fix session is a queue to join rather
    // than a reason to give up. It may be a long wait — and once taken, it is
    // held for as long as the review session lives, which is across the human's
    // answering too. That is the shape of one agent in one Worktree.
    //
    // It is also what tells a review whose session is gone from one whose session
    // is sitting on its ask: a live review holds this, so anything that gets it
    // is looking at a Worktree with no agent in it.
    let _turn = state.sessions.turn(conversation_id).await;

    // Asked again on the other side of the wait, because everything it asked
    // about moves while it waits: the fix session that held the Worktree may have
    // been the last of its attempts, the review may have finished under it, and
    // the Conversation may have been closed out from under this altogether.
    match wanted(&state, conversation_id).await {
        Wanted::Nothing => {}
        Wanted::Review => reading(&state, conversation_id).await,
        Wanted::Unattended(set_id) => unattended(&state, conversation_id, set_id).await,
    }
}

/// Read the branch from the start, whatever this wrap's review has already left
/// behind.
///
/// What a press means for the review, and the review's half of
/// [`crate::checks::afresh`] — there for the reason the attempts are forgotten
/// beside it. The human has read the Notice of what stopped and asked for
/// another go, and a review found already asking would be a run that stopped all
/// over again on its first look without reading a line.
///
/// So whatever the gone session left standing is closed and the branch is read
/// afresh, in a session as new as the first: it reads the branch as it now
/// stands — anything that session did land included — and raises whatever is
/// still worth raising.
pub(crate) async fn afresh(state: AppState, conversation_id: i64) {
    if matches!(wanted(&state, conversation_id).await, Wanted::Nothing) {
        return;
    }

    // Waited for on the same terms [`run`] waits on it, and for the same reason:
    // a Worktree with an agent in it is a queue to join, and getting it is what
    // says there is no session left behind whatever the record shows.
    let _turn = state.sessions.turn(conversation_id).await;

    match wanted(&state, conversation_id).await {
        Wanted::Nothing => return,
        Wanted::Review => {}
        Wanted::Unattended(set_id) => closed(&state, conversation_id, set_id).await,
    }

    reading(&state, conversation_id).await
}

/// Run the one review session a wrap-up gets, and see out whatever it leaves.
async fn reading(state: &AppState, conversation_id: i64) {
    tracing::info!(
        conversation_id,
        "the work is on a pull request nobody has read, so a review session is starting"
    );

    // Read inside the Turn, which is what makes *what was said before the review
    // started* a fact rather than a race: nothing can dispatch about a comment
    // while this holds the Worktree, and one that lands from here on is the next
    // batch session's. Recorded as addressed as this session is dispatched, so
    // nothing is later sent to do ungated what the Set is about to propose.
    let said = crate::comments::for_the_review(state, conversation_id).await;

    match crate::runner::review(state, conversation_id, said).await {
        Reviewed::Done => over(state, conversation_id, None).await,
        Reviewed::Stopped { how, writing } => {
            over(state, conversation_id, Some((how, writing))).await
        }
        Reviewed::Nothing => {}
    }
}

/// Pick up a review whose session is no longer there, whatever the human has or
/// has not answered.
///
/// The Worktree is this task's — see [`run`] — so there is no agent left in it,
/// and the Set on `set_id` is a proposal with nobody behind it. Which is a stop
/// either way: the one session that could have finished this review is gone, and
/// no other is ever sent to finish somebody else's.
///
/// **Unanswered**, and the questions go off as the run stops — see
/// [`abandoned`]. **Answered**, and the deciding was done with nothing here
/// knowing how much of the doing followed it — see [`dropped`]. Resume is the
/// branch read again either way.
async fn unattended(state: &AppState, conversation_id: i64, set_id: i64) {
    if unanswered(state, set_id).await {
        return abandoned(state, conversation_id, set_id, None).await;
    }

    dropped(state, conversation_id, set_id).await
}

/// The review session is over: settle the review, send the work back to be
/// built, or stop the run.
///
/// Nothing is asked of the record about what it did. The session is the one
/// thing that read the human's picks and the one thing that carried them out, so
/// how it ended is the whole of its report — and what it left on the branch is
/// the rest of it.
///
/// `ended_badly` is how it ended where it did not end well, and the Timeline
/// Event it was printing into. A session that ended badly is a review that did
/// not finish, whatever it had got to by then, and that stops the run.
async fn over(state: &AppState, conversation_id: i64, ended_badly: Option<(String, i64)>) {
    // A session that put its findings up and then went — cleanly or otherwise —
    // leaves a Set with nobody to read what the human writes into it, so the
    // questions go off and the run stops rather than waiting on an answer
    // nothing would ever act on.
    if let Some(set_id) = asked(state, conversation_id).await {
        if unanswered(state, set_id).await {
            return abandoned(state, conversation_id, set_id, ended_badly).await;
        }
    }

    if let Some((how, writing)) = ended_badly {
        return stopped(state, conversation_id, &how, writing).await;
    }

    // Everything it was sent to do is done: the branch read, whatever it found
    // put to the human, and whatever they picked carried out.
    carried_out(state, conversation_id).await
}

/// What a review that did everything it was sent to do leaves behind: a wrap-up
/// with one less thing to wait on, or a Conversation on its way back to being
/// built.
///
/// Which of the two is a fact about the branch. A session answered into writing
/// a `.tasks/` backlog rather than fixing something where it stood has committed
/// one, and a wrap-up that settled over the top of it would reach Done with
/// agreed work nobody had worked. Anything else is the ordinary end: whatever
/// was accepted is pushed and the review is over.
///
/// The list is the whole signal, and it is asked of the Worktree rather than of
/// any record of what was picked. A wrap-up reaches here having had its backlog
/// taken away by the finish step that opened the pull request, so a `.tasks/`
/// list on the branch is one this review's own session wrote.
async fn carried_out(state: &AppState, conversation_id: i64) {
    if backlog(state, conversation_id).await {
        return built_instead(state, conversation_id).await;
    }

    settle(state, conversation_id).await;

    tracing::info!(
        conversation_id,
        "the review is over, so the wrap-up carries on"
    );
}

/// Send the Conversation back down the ladder to build what the review split
/// out.
///
/// The one move out of Wrapping there is, and the review's settle goes with it —
/// see [`store::implement_again`], which does both in one transaction. So there
/// is nothing to settle here and nothing to unsettle: this review is over
/// without ever having been settled, and the one the second wrap runs is a fresh
/// reading of a branch that has since been built on.
///
/// The backlog is then worked exactly as any other is, in a task of its own —
/// spawned rather than awaited, because the Turn this Conversation's Worktree is
/// under is still held by the review that is calling this, and the first session
/// of the backlog needs it.
///
/// Anything but a move that was made leaves it where it is, with the reason in
/// the log: a Conversation closed out from under the session that wrote the
/// backlog is not one to start building.
async fn built_instead(state: &AppState, conversation_id: i64) {
    match store::implement_again(&state.pool, conversation_id).await {
        Ok(store::Rebuilding::Started) => {}
        Ok(store::Rebuilding::NotWrapping) => {
            return tracing::info!(
                conversation_id,
                "the Conversation stopped wrapping up, so the backlog its review wrote \
                 was not started"
            );
        }
        Ok(store::Rebuilding::NoSuchConversation) => {
            return tracing::error!(
                conversation_id,
                "there is no Conversation left to build the backlog its review wrote"
            );
        }
        Err(error) => {
            return tracing::error!(
                error = ?error,
                conversation_id,
                "sending a Conversation back to build what its review split out failed"
            );
        }
    }

    tracing::info!(
        conversation_id,
        "the review split findings out into a backlog, so the Conversation goes back to \
         build it"
    );

    // The Timeline has a move on it and a task list pinned above it, and an open
    // page should say so without being reloaded.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    crate::runner::build_the_split_out(state, conversation_id);
}

/// Whether a `.tasks/` backlog is on the branch, committed as it stands.
///
/// What says a review was answered into splitting work out rather than fixing it
/// where it stood, asked by exactly the rule a breakdown's own step is judged by
/// — the list being there and git having nothing pending for it. A `TODO.md`
/// written and not committed is a session still mid-write, and a wrap-up that
/// read that as done would send the Conversation back to build a backlog that is
/// about to be swept away.
///
/// A Conversation with no Worktree left has nowhere for one to be, and a store
/// that will not answer has said nothing about whether it is there. Both read as
/// *no backlog*, which settles the review rather than sending a Conversation
/// back down the ladder to build a list nothing can find.
async fn backlog(state: &AppState, conversation_id: i64) -> bool {
    let worktree = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.worktree,
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to look for a backlog in"
            );
            return false;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to look for a backlog in failed");
            return false;
        }
    };

    match worktree {
        Some(worktree) => crate::runner::backlog_landed(&worktree).await,
        None => false,
    }
}

/// What [`run`] has to do about this Conversation's review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wanted {
    /// Nothing at all, which is the ordinary answer and never a failure: the
    /// Conversation has stopped wrapping up, the review has already settled, or
    /// driving has stopped — the same rule the runner and the checks watcher
    /// keep, that nothing is launched behind a stop.
    Nothing,

    /// Nobody has read the branch, so a review session reads it.
    Review,

    /// The review put its findings up on this Set and nothing is running for it,
    /// which is a stop whatever the human answered — see [`unattended`]. Where
    /// the press is what is looking, it is a Set to close on the way past
    /// instead — see [`afresh`].
    Unattended(i64),
}

/// Which of the three there is, asked of the record.
///
/// The order matters and is the order the questions rule each other out in: a
/// Conversation that is not wrapping up has no wrap-up to see to, a review that
/// has settled is over whatever Sets are on the Timeline, and a stop holds off
/// everything below it. Only then is the Set worth looking
/// for, because only then does its being there mean anything.
///
/// A store that will not answer reads as *nothing*, which is the right way round
/// for the one thing this decides: on the other side of it is an agent being let
/// loose in a Worktree.
async fn wanted(state: &AppState, conversation_id: i64) -> Wanted {
    if !crate::wrapping::still_going(state, conversation_id).await {
        return Wanted::Nothing;
    }

    match store::wrap_up_settled(&state.pool, conversation_id).await {
        Ok(settled) if settled.contains(&store::WaitingOn::Review) => {
            tracing::debug!(
                conversation_id,
                "this Conversation has been reviewed already"
            );
            return Wanted::Nothing;
        }
        Ok(_) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a wrap-up had settled failed");
            return Wanted::Nothing;
        }
    }

    if crate::stopping::stopped(state, conversation_id).await {
        return Wanted::Nothing;
    }

    match asked(state, conversation_id).await {
        Some(set_id) => Wanted::Unattended(set_id),
        None => Wanted::Review,
    }
}

/// Which Set this Conversation's wrap-up has last put to the human, where it has
/// asked anything at all.
///
/// The newest of this wrap's asks rather than its first, because a wrap can read
/// its branch more than once: a review the human pressed Resume over is read
/// afresh — see [`afresh`] — and the Set that reading puts up is the one there is
/// anybody behind. An unsettled wrap's asks are the review's own, because it is
/// the session a wrap-up starts with and the batch sessions that propose the same
/// way about what was said on the pull request are none of them dispatched until
/// it is over. See [`crate::comments::once`].
///
/// Nothing marks a Set as the review's — see [`store::last_proposal`] — so a Set
/// some other session of this wrap left standing is read as one of these too.
/// Which is the safe way round: what is on the other side of it is a question
/// nobody is coming back to answer, and that is worth stopping over whoever asked
/// it.
///
/// A Deferred Ask is none of them, for the reason no session was ever held open
/// by one: it idles nobody, its Answers reach a later session by design, and a
/// wrap-up that stopped over one would be stopping over a question that is
/// working exactly as it was meant to.
///
/// A store that will not answer reads as *it never asked*, which is the same way
/// round the rest of this module reads one: everything on the other side of a
/// `None` here waits on the Worktree and asks the record again.
async fn asked(state: &AppState, conversation_id: i64) -> Option<i64> {
    match store::last_proposal(&state.pool, conversation_id).await {
        Ok(asked) => asked,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether the review had asked failed");
            None
        }
    }
}

/// Whether a Set is still waiting on the human — neither answered nor closed.
///
/// A store that will not answer reads as *unanswered*, which is the right way
/// round for what hangs on it: an unanswered Set stops the run and an answered
/// one lets a wrap-up settle, and a database that will not say which is not
/// grounds for the second.
///
/// Shared with the batch half, which asks it of its own proposals — see
/// [`crate::responding`].
pub(crate) async fn unanswered(state: &AppState, set_id: i64) -> bool {
    match store::settlement(&state.pool, set_id).await {
        Ok(settled) => settled.is_none(),
        Err(error) => {
            tracing::error!(error = ?error, set_id, "reading whether a Set had been answered failed");
            true
        }
    }
}

/// Stop the run: the review's findings are up and the session that would have
/// acted on them is gone.
///
/// **The Set is closed as this is raised**, which is the deliberate half of it. A
/// Set left standing would be a question whose answer nothing would ever read —
/// the session that asked it is not there to be woken, and no other session is
/// ever handed somebody else's ask. Closing it is Verkstead reaching for the
/// lock on the human's behalf because it knows something they cannot see,
/// exactly as a relaunched grilling closes what its dead session left open — see
/// [`crate::grillings`]. And it is what makes a Resume mean something: with
/// nothing left standing, a fresh reading of the branch is recognised as this
/// wrap's review rather than mistaken for a second one.
///
/// `ended_badly` is how the session went where this is being raised as one ends
/// and it did not end well, with the Timeline Event it was printing into — which
/// is where a review that fell over mid-ask says why. Absent where the session
/// was already gone before anybody looked, which is what a restarted server
/// finds.
///
/// [`store::Decision::Verkstead`]: what to do about it is Resume, which reads the
/// branch again in a session as fresh as the first, or taking the branch over,
/// or closing the Conversation with the branch exactly as it stands.
async fn abandoned(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
    ended_badly: Option<(String, i64)>,
) {
    closed(state, conversation_id, set_id).await;

    let left = "the review put its findings to you and the session that was to act on \
                them is gone, so its questions have been closed unanswered. Resuming \
                reads the branch again.";

    let (how, writing) = match ended_badly {
        Some((how, writing)) => (format!("{how}, and {left}"), Some(writing)),
        None => (left.to_owned(), None),
    };

    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "acting on the answers to what the review found",
        &how,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a review's findings were left with nobody to act on them and the stop \
             saying so could not be recorded"
        );
    }
}

/// Close a proposal nobody is left to act on, so that nothing waits on it and
/// nothing counts it.
///
/// Shared with the batch half, which closes its own the same way and for the same
/// reason — see [`crate::responding`].
pub(crate) async fn closed(state: &AppState, conversation_id: i64, set_id: i64) {
    match store::lock_set(&state.pool, &state.settlements, set_id).await {
        Ok(store::Locking::Locked(_)) => tracing::info!(
            conversation_id,
            set_id,
            "the session that asked is gone, so its questions are closed unanswered"
        ),
        Ok(other) => tracing::info!(
            conversation_id,
            set_id,
            outcome = ?other,
            "a proposal nobody was left to act on was not closed"
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, set_id, "closing a proposal nobody was left to act on failed");
        }
    }
}

/// Record that the review is over, so wrap-up has one less thing to wait on.
///
/// Once its session has ended cleanly and never before: what wrap-up is waiting
/// on is the whole of the review — the branch read, the findings put, the ones
/// the human accepted landed — and the session ending well is the one thing that
/// says all of it happened. Answering the Set says only that the decisions are
/// made.
async fn settle(state: &AppState, conversation_id: i64) {
    if let Err(error) =
        store::settle_wrap_up(&state.pool, conversation_id, store::WaitingOn::Review).await
    {
        tracing::error!(error = ?error, conversation_id, "recording that the review was over failed");
    }
}

/// Stop the run: the review did not finish, and what to do about it is the
/// human's.
///
/// The evidence is the tail of what the session said, which is where a review
/// that fell over says why — and what to do about it is Resume, read the branch
/// themselves, or close the Conversation.
///
/// [`store::Decision::Verkstead`], because a wrap-up that goes on without its
/// review is a branch nobody read: Verkstead stops rather than pass a session
/// that crashed off as a clean bill of health, and going again is the human's
/// press.
async fn stopped(state: &AppState, conversation_id: i64, how: &str, writing: i64) {
    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "reviewing the branch the pull request is on",
        how,
        Some(writing),
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a review did not finish and the stop saying so could not be recorded"
        );
    }
}

/// Stop the run: the review's findings were answered and the session that was to
/// act on them is gone.
///
/// What became of the answers is beyond asking. The session may have landed
/// every fix, some of them or none of them, and the only participant that knew
/// is not there — so the run stops rather than settle a review nothing saw the
/// end of, and the human says what happens next.
///
/// [`store::Decision::Verkstead`]: what to do about it is Resume, which reads
/// the branch again in a session as fresh as the first and raises whatever is
/// still worth raising, or reading it themselves, or closing the Conversation
/// with the branch exactly as it stands.
async fn dropped(state: &AppState, conversation_id: i64, set_id: i64) {
    tracing::info!(
        conversation_id,
        set_id,
        "the review was answered and the session that was to act on it is gone, so the \
         run stops"
    );

    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "acting on the answers to what the review found",
        "you answered what the review found and the session that was to act on it is \
         gone, so what became of it is not known here. Resuming reads the branch again.",
        None,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a review's answers were left with nobody to act on them and the stop \
             saying so could not be recorded"
        );
    }
}
