//! The other proposal a wrap-up makes: what a batch of comments left on one of
//! its pull requests comes to.
//!
//! Everything standing on every pull request when the review starts belongs to
//! the review — see [`crate::comments::for_the_review`]. This is about what is
//! said *after* it: a batch of comments the human wrote on one of them while the
//! branch sat there, which nothing has been asked about yet.
//!
//! **A batch is one pull request's.** A Conversation ends on one per repository
//! it was worked in, and a human writes on the one they are reading — so the
//! session is told which repository, which pull request and which worktree to
//! work in, and the comments it is dispatched about are recorded as addressed
//! against that pull request alone. What is settled is *this pull request has
//! nothing outstanding*, which is what lets one of them go quiet while another is
//! still being answered.
//!
//! **It proposes, and then it fixes what was agreed to**, exactly as the review
//! does and for the same reason. A comment is the human talking, but the words
//! on a pull request are not an instruction to a session: "this is the wrong way
//! round" says what is wrong and not what to do about it, and a session sent to
//! do it would be deciding in their place. So the batch session reads what was
//! said and the code it is about, puts one small Set saying what it would do,
//! waits, and lands what they accepted. One session for the batch and one Set
//! for it, because three replies in a minute are one point being made.
//!
//! A batch asking for nothing asks nothing. A question the commits since have
//! answered, a note saying this reads well — the session says so as the last
//! thing it prints, which is what the Timeline shows of one, and the batch
//! stays settled as addressed. Spending the human's attention only where there
//! is a decision, which is the review's rule one turn later.
//!
//! **The batch is dealt with when its session ends cleanly**, which is the one
//! moment everything it was sent to do is certainly over: what was said read,
//! whatever it would do put to the human, whatever they accepted carried out.
//! Answering the Set deals with nothing by itself — the Response is what the
//! session acts on, and it is still acting when it arrives.
//!
//! Nothing is asked of the record about what it did with those answers. The
//! session is the one thing that read what the human picked and the one thing
//! that carried it out, so *it ended cleanly* is the whole of its report: the
//! comments stay written down as addressed, and the watcher settles them on its
//! next poll. A Verkstead that read the picks back and audited the branch
//! against them would be second-guessing the only participant that was there —
//! the review's rule one turn later again, and for the review's reason. See
//! [`crate::review`].
//!
//! **A batch session that is gone is a stop.** Two things arrive there — one
//! that fell over, and a server that came back up over a batch that was still
//! running — and both are the same fact, because a session lives and dies with
//! the process that started it. Either way the one session that could have
//! finished this batch is not there, and no other is ever sent to finish
//! somebody else's: what was decided, what was half done and what was never
//! started are all beyond asking, so the run stops and the human says what
//! happens next.
//!
//! **And nothing the human was asked is left standing behind it**, which is the
//! failure the addressing-as-dispatched trade opens. The comments are written
//! down as dealt with before the session that deals with them has done anything,
//! so a batch session that goes between the asking and the answering leaves a
//! record saying somebody saw to what was said and a Set nobody is behind. Left
//! alone, the watcher finds nothing new, settles the comments, and the wrap-up
//! reaches Done with the human's questions still open. So any Set a gone session
//! left unanswered is closed as the stop is raised, which says on the Timeline
//! that the question is off.
//!
//! What was said goes back to being unread as that stop is recorded, so that the
//! human's feedback outlives the session that lost it. The record cannot say
//! which comments the dead session was dealing with and which the review folded
//! in before it, so every one of them is read again — a comment read twice costs
//! a session's work, and one dropped costs the human theirs. Which is safe
//! because of what a batch session is: it reads the code as it now stands, and
//! a question the commits since have answered is one it says so about and asks
//! nothing more of.
//!
//! Resume is the batch over again, in a session as fresh as the first. Nothing
//! is dispatched from the record, answered or not: the deciding was the human's
//! and the doing was the gone session's, and a session handed decisions off a
//! Set nobody reported on would be Verkstead acting on somebody else's reading
//! of what was said.
//!
//! **One agent in one Worktree.** The caller holds the Conversation's Turn
//! across the whole of a batch session, the wait on the human included, exactly
//! as the review's caller does — see [`crate::comments::dispatch`].

use crate::AppState;
use crate::runner::Reviewed;
use crate::store;

/// Run one batch session about `said`, and see out whatever it leaves.
///
/// The caller is holding the Conversation's Turn and has recorded `which` — the
/// comments this batch is made of — as addressed against the pull request opened
/// in `repo_id`. Both are what make this the one session that will act on them.
///
/// `repo_id` is which pull request the batch was left on, carried through so
/// that whatever has to be put back where it was — the comments unread, the
/// settlement back to waiting — is put back for that one rather than for all of
/// them.
///
/// Nothing is refused for. This runs unattended with nobody watching, and what
/// it has to say it says on the Timeline or in the log.
pub(crate) async fn run(
    state: &AppState,
    conversation_id: i64,
    repo_id: i64,
    said: &str,
    which: &[String],
) {
    match crate::runner::respond(state, conversation_id, said).await {
        Reviewed::Done => over(state, conversation_id, repo_id, which, None).await,
        Reviewed::Stopped { how, writing } => {
            over(state, conversation_id, repo_id, which, Some((how, writing))).await
        }
        Reviewed::Nothing => {}
    }
}

/// The batch session is over: leave the wrap-up to carry on, or stop the run.
///
/// Nothing is asked of the record about what it did. The session is the one
/// thing that read the human's picks and the one thing that carried them out, so
/// how it ended is the whole of its report.
///
/// `ended_badly` is how it ended where it did not end well, and the Timeline
/// Event it was printing into. A session that ended badly is a batch that was
/// not seen to, whatever it had got to by then, and that stops the run.
async fn over(
    state: &AppState,
    conversation_id: i64,
    repo_id: i64,
    which: &[String],
    ended_badly: Option<(String, i64)>,
) {
    // A session that put its proposal up and then went — cleanly or otherwise —
    // leaves a Set with nobody to read what the human writes into it, so the
    // questions go off and the run stops rather than waiting on an answer
    // nothing would ever act on.
    if let Some(set_id) = proposed(state, conversation_id).await {
        if crate::review::unanswered(state, set_id).await {
            return abandoned(
                state,
                conversation_id,
                set_id,
                Some((repo_id, which)),
                ended_badly,
            )
            .await;
        }
    }

    if let Some((how, writing)) = ended_badly {
        return stopped(state, conversation_id, repo_id, which, &how, writing).await;
    }

    // Everything it was sent to do is done: what was said read, whatever it would
    // do put to the human, and whatever they accepted fixed and pushed — or the
    // batch asked for nothing and it said so. The comments are addressed either
    // way, and the watcher settles them on its next poll.
    tracing::info!(
        conversation_id,
        comments = which.len(),
        "what was said on the pull request has been answered, so the wrap-up carries on"
    );
}

/// See to whatever a batch session that is no longer running left behind, and say
/// whether it left anything.
///
/// `true` means *there is something outstanding here*, which is the comments
/// watcher's answer to two questions at once: settle nothing, and dispatch
/// nothing about anything new until this is dealt with. `false` is the ordinary
/// answer and the cheap one — two reads of the record and no Worktree taken.
///
/// What it looks for is a batch's own proposal still waiting on the human — see
/// [`proposed`]. That is the whole of what a record can be asked here. A
/// proposal they have already answered is one the session behind it acted on,
/// because acting on the answers is the last thing a batch session does and
/// ending cleanly is its report that it did; the same session lost to a restart
/// between the two is the one thing this cannot tell from that, and the record
/// is not asked to guess. A session that died in front of the server that
/// started it is caught where it ends instead — see [`over`].
///
/// Asked of a batch's own proposal and never of the review's, for the same
/// reason: how a review ended is the report of the session that ran it, and
/// stopping over its Set here would be this half acting on somebody else's
/// half-read report.
///
/// **The Worktree is what says the session is gone.** A batch session holds the
/// Conversation's Turn across the whole of its own life, the wait on the human
/// included, so a Turn that cannot be taken is a session still working and
/// nothing here to do but come back. Tried rather than waited for, because this
/// is a poll: waiting would hold a watcher for however long the session takes.
pub(crate) async fn unattended(state: &AppState, conversation_id: i64) -> bool {
    let Some(set_id) = proposed(state, conversation_id).await else {
        return false;
    };

    if !crate::review::unanswered(state, set_id).await {
        return false;
    }

    let Some(_turn) = state.sessions.try_turn(conversation_id) else {
        tracing::debug!(
            conversation_id,
            set_id,
            "a batch session is still working, so what it has asked is left to it",
        );
        return true;
    };

    // Asked with the Worktree in hand, for the reason the review asks twice: a
    // Conversation closed while this looked has nowhere left to work.
    if !crate::wrapping::still_going(state, conversation_id).await {
        return false;
    }

    abandoned(state, conversation_id, set_id, None, None).await;

    true
}

/// Which Set the newest proposal a *batch* session put up is, where a batch has
/// put one up at all.
///
/// A batch's own and never the review's, which the review's settle is the line
/// between — see [`store::last_batch_proposal`]. It has to be: the review's Set
/// is the newest proposal on the Timeline until a batch asks anything, and how a
/// review ended is the report of the session that ran it rather than something
/// for this half to read off its Set afterwards.
///
/// A store that will not answer reads as *nothing has been proposed*, which is
/// the right way round for what this decides: on the other side of it is a run
/// being stopped.
async fn proposed(state: &AppState, conversation_id: i64) -> Option<i64> {
    match store::last_batch_proposal(&state.pool, conversation_id).await {
        Ok(proposed) => proposed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what was last put to the human failed");
            None
        }
    }
}

/// Stop the run: a batch session's proposal is up and nobody is left to act on
/// it.
///
/// The Set is closed as this stops, for the reason the review closes its own — a
/// question whose answer nothing would read is one to take off the Timeline
/// rather than leave standing. And what was said goes back to being unread with
/// it, so that the human's feedback outlives the session that lost it and a
/// Resume is a fresh session about the same words.
///
/// `which` is the pull request the batch was left on and the batch itself, where
/// the caller is the driver that dispatched it and therefore knows. `None` is a
/// caller that cannot know — a server that came back up over somebody else's
/// batch — and every comment on every one of the Conversation's pull requests is
/// read again there, because the record says neither which pull request the gone
/// session was answering nor which of that one's comments were its batch rather
/// than what the review folded in before it. A comment read twice costs a
/// session's work and one dropped costs the human theirs, and a batch session
/// reads the code as it now stands: a question the commits since have answered is
/// one it says so about and asks nothing more of.
///
/// Forgotten before the stop is recorded, exactly as [`stopped`] forgets them,
/// so that a forgetting that fails leaves the run stopped rather than quietly
/// going round again.
///
/// `ended_badly` is how the session went where this is being raised as one ends
/// and it did not end well, with the Timeline Event it was printing into.
///
/// [`store::Decision::Verkstead`]: what to do about it is Resume, which answers
/// what was said again, or reading the comments yourself, or ending the run.
async fn abandoned(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
    which: Option<(i64, &[String])>,
    ended_badly: Option<(String, i64)>,
) {
    crate::review::closed(state, conversation_id, set_id).await;

    forget(state, conversation_id, which).await;
    unsettle(state, conversation_id, which.map(|(repo_id, _)| repo_id)).await;

    let left = "a session read what was said on the pull request and put what it would \
                do to you, and it is gone, so its questions have been closed unanswered. \
                Resuming reads what was said again.";

    let (how, writing) = match ended_badly {
        Some((how, writing)) => (format!("{how}, and {left}"), Some(writing)),
        None => (left.to_owned(), None),
    };

    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "acting on the answers to what was proposed about the pull request's comments",
        &how,
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a batch session's proposal was left with nobody to act on it and the stop \
             saying so could not be recorded"
        );
    }
}

/// Put comments back to being unread: the batch's own, on the pull request it was
/// left on, or every one of them on every pull request where the caller cannot
/// say which those were.
async fn forget(state: &AppState, conversation_id: i64, which: Option<(i64, &[String])>) {
    let forgotten = match which {
        Some((repo_id, which)) => {
            store::forget_addressed_comments(&state.pool, conversation_id, repo_id, which).await
        }
        None => store::forget_every_addressed_comment(&state.pool, conversation_id).await,
    };

    if let Err(error) = forgotten {
        tracing::error!(error = ?error, conversation_id, "forgetting what a gone session was reading failed");
    }
}

/// And record that something said on a pull request is left unaddressed, which
/// is what a proposal nobody is behind amounts to.
///
/// The pull request the batch was left on, or every one of the Conversation's
/// where the caller cannot say which that was — the comments have all gone back
/// to being unread there, so every one of them has something outstanding on it
/// again.
///
/// Said before the run is stopped, because wrap-up's rule is decided by a loop
/// of its own: a Conversation whose checks went green in the meantime would
/// otherwise reach Done over the top of a proposal nobody is behind.
async fn unsettle(state: &AppState, conversation_id: i64, repo_id: Option<i64>) {
    let opened = match repo_id {
        Some(repo_id) => vec![repo_id],
        None => match store::pull_requests(&state.pool, conversation_id).await {
            Ok(opened) => opened.into_iter().map(|(repo, _)| repo.id).collect(),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading which pull requests to put back to waiting failed");
                return;
            }
        },
    };

    for repo_id in opened {
        if let Err(error) = store::unsettle_wrap_up(
            &state.pool,
            conversation_id,
            store::WaitingOn::Comments(repo_id),
        )
        .await
        {
            tracing::error!(error = ?error, conversation_id, repo_id, "putting the comments back to waiting failed");
        }
    }
}

/// Stop the run: the batch session did not finish, and what to do about it is
/// the human's.
///
/// The comments are forgotten first, because going again is the batch over again
/// and they were written down as addressed the moment it was dispatched.
/// Forgotten rather than left, so that the session Resume's watcher starts is one
/// about the same words rather than one about nothing — and forgotten before the
/// stop is recorded, so that a forgetting that fails leaves the run stopped
/// rather than quietly going round again.
///
/// The evidence is the tail of what the session said, which is where one that
/// fell over says why.
///
/// [`store::Decision::Verkstead`]: what to do about it is Resume, which answers what
/// was said again, or reading the comments yourself, or ending the run.
async fn stopped(
    state: &AppState,
    conversation_id: i64,
    repo_id: i64,
    which: &[String],
    how: &str,
    writing: i64,
) {
    if let Err(error) =
        store::forget_addressed_comments(&state.pool, conversation_id, repo_id, which).await
    {
        tracing::error!(error = ?error, conversation_id, repo_id, "forgetting a batch nobody answered failed");
    }

    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "answering what was said on the pull request",
        how,
        Some(writing),
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a batch session did not finish and the stop saying so could not be recorded"
        );
    }
}
