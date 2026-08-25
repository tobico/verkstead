//! The wrap-up self-review: the one session that reads the branch whole, and
//! what becomes of what it finds.
//!
//! There are no per-commit review states anywhere in Verkstead. Commits are
//! events to read, and this is where problems get raised instead — once, about a
//! branch, by a session with fresh context. The sessions that wrote the work each
//! saw one task and none of them saw the pull request; this one sees nothing else.
//!
//! **It proposes, and then it fixes what was agreed to.** What it produces first
//! is one Question Set on the Timeline, a Question per finding, with Options that
//! amount to *fix it* or *leave it* — which is what puts the human in the loop
//! without putting them at a terminal. Then it stays where it is: the ask blocks
//! until they answer, and when the answers come back the same session fixes each
//! finding they accepted, commits, pushes and ends. A finding they declined is
//! never raised again.
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
//! **The review settles when its session ends cleanly and its fixes have
//! landed**, which is the one moment everything it was sent to do is certainly
//! over: the branch read, the findings put, the accepted ones committed.
//! Answering the Set settles nothing — the Response is what the session acts on,
//! and it is still acting when it arrives.
//!
//! **A finding too big for the sitting can be split out instead.** Where the
//! review judges one more work than it can do between the answers and its push,
//! it offers a third Option on that finding — and where the human picks it, what
//! the session writes for that finding is a `.tasks/` backlog rather than a fix.
//! The Conversation then goes back down the ladder: Wrapping to Implementing,
//! the backlog worked a session at a time as any other is, and the finish that
//! follows the last task wraps it up again on the pull request it already had,
//! reviewed afresh. See [`crate::runner::build_the_split_out`].
//!
//! Offered rarely and never by default: a Set that put the choice on every
//! finding would be asking the human to plan the work as well as decide it, and
//! the ordinary handful of fixes is what this whole phase exists to keep in one
//! session. A review that offers none is the common case and is not the poorer
//! for it.
//!
//! **Nothing the human accepted is allowed to go quietly.** A session that asked,
//! was answered and then went — cleanly or otherwise — with nothing committed
//! since is a wrap-up owing work nobody is left to do, and a review that settled
//! there would reach Done with approved fixes lost. So the record is asked
//! afterwards rather than the session trusted: the findings they accepted are on
//! the Set, their words are on the Response, and a branch with no commit since
//! the answers is the doing never having happened. That stops the run with a
//! Notice saying what is owed, and Resume is the doing over again — one fix
//! session handed every accepted finding at once, because the decisions were
//! made and only the carrying out failed. Nothing is asked again. A split pick
//! is owed the same way and reads the same rule the other way round: what it is
//! owed is the backlog, so the branch is asked whether one is on it rather than
//! whether anything was committed.
//!
//! A review session that ends badly having been owed nothing is not a review that
//! had nothing to do: it is a review that did not finish. That stops the run like
//! every other stop, and going again is the review over from the start, in a
//! session as fresh as the first.
//!
//! **And nothing the human was asked is allowed to go quietly either.** The
//! propose-then-fix shape has one session hold the whole of a review, its ask
//! included, so a session that goes between the asking and the answering leaves a
//! Set on the Timeline with nobody behind it: nothing is coming to read what the
//! human says, and no other session is ever handed somebody else's ask. Two
//! things arrive there — a session that fell over mid-ask, and a server that came
//! back up over a wrap-up whose review was still waiting — and both are the same
//! fact, because a session lives and dies with the process that started it.
//!
//! So the review is asked about every time anything starts a wrap-up's watchers,
//! rather than only where nobody has read the branch. A review that asked and has
//! no session is picked up by what the record says about its Set. **Answered**,
//! and the deciding is done: what is left is the doing, dispatched exactly as an
//! owed-fixes stop has it dispatched on Resume, with nobody asked for anything.
//! **Unanswered**, and there is nothing to carry out and nobody to carry it out —
//! so the Set is closed unanswered, saying on the Timeline that the question is
//! off, and the run stops with Resume meaning the branch read again. Closing it
//! is also what makes that work: a Set left standing would still be this wrap's
//! review, and the fresh reading would be a second review nothing recognised.
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
/// with no review running — the finish step, a server coming back up, a Resume
/// pressed on a stopped wrap-up — and *no review running* is two different
/// situations. One is a branch nobody has read. The other is a review that asked
/// and whose session is no longer there to act on the answers, which no amount
/// of waiting resolves by itself.
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
    // the Conversation may have been aborted out from under this altogether.
    match wanted(&state, conversation_id).await {
        Wanted::Nothing => {}
        Wanted::Review => reading(&state, conversation_id).await,
        Wanted::Unattended(set_id) => unattended(&state, conversation_id, set_id).await,
    }
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

/// Pick up a review that put its findings to the human and whose session is no
/// longer there to act on the answers.
///
/// The Worktree is this task's — see [`run`] — so there is no agent left in it,
/// and the Set on `set_id` is a proposal with nobody behind it. What that is owed
/// is a fact about whether the human got to it first.
///
/// **Answered**, and the deciding is done: what is left is the doing, which is
/// the same session a Resume after [`dropped`] runs and is dispatched here
/// without anybody being asked for anything. A Response with nothing owed against it is a
/// review that got everything done and lost only its own last breath, so it
/// settles.
///
/// **Unanswered**, and there is nothing here that can be carried out: the
/// decisions were never made, and the session that would have read them is gone.
/// That stops the run — see [`abandoned`].
async fn unattended(state: &AppState, conversation_id: i64, set_id: i64) {
    if unanswered(state, set_id).await {
        return abandoned(state, conversation_id, set_id, None).await;
    }

    let owed = owing_now(state, conversation_id).await;

    if owed.nothing() {
        tracing::info!(
            conversation_id,
            set_id,
            "the review's findings were answered and nothing is owed on them, so the \
             wrap-up carries on without the session that asked"
        );

        return carried_out(state, conversation_id).await;
    }

    tracing::info!(
        conversation_id,
        set_id,
        fixes = owed.fixes.len(),
        splits = owed.splits.len(),
        "the review was answered and the session that would have acted on it is gone, \
         so a session is starting on the doing alone"
    );

    land(state, conversation_id, owed).await
}

/// The review session is over: settle the review, send the work back to be
/// built, or stop the run.
///
/// One question first, and it is asked of the record and the branch rather than
/// of the session: is there anything the human decided about that never landed?
/// That is the failure this half exists for, and it reads the same whether the
/// session saw itself out or fell over — the decisions are made either way, and
/// what is owed is owed.
///
/// `ended_badly` is how it ended where it did not end well, and the Timeline
/// Event it was printing into. A session owed nothing that ended badly is a
/// review that did not finish, which is the other stop here.
async fn over(state: &AppState, conversation_id: i64, ended_badly: Option<(String, i64)>) {
    let owed = owing_now(state, conversation_id).await;

    if !owed.nothing() {
        let (how, writing) = match &ended_badly {
            Some((how, writing)) => (Some(how.as_str()), Some(*writing)),
            None => (None, None),
        };

        return dropped(state, conversation_id, &owed, how, writing).await;
    }

    // Owed nothing, which is two very different things: everything decided was
    // carried out, or nothing was ever decided. A session that put its findings
    // up and then went — cleanly or otherwise — is owed nothing because the
    // human never got to answer, and settling there would leave their Set on the
    // Timeline with nobody to read what they said.
    if let Some(set_id) = asked(state, conversation_id).await {
        if unanswered(state, set_id).await {
            return abandoned(state, conversation_id, set_id, ended_badly).await;
        }
    }

    if let Some((how, writing)) = ended_badly {
        return stopped(state, conversation_id, &how, writing).await;
    }

    // Everything it was sent to do is done: the branch read, whatever it found
    // put to the human, whatever they accepted fixed and pushed, and whatever
    // they split out written as a backlog.
    carried_out(state, conversation_id).await
}

/// What a review that did everything it was sent to do leaves behind: a wrap-up
/// with one less thing to wait on, or a Conversation on its way back to being
/// built.
///
/// Which of the two is a fact about what the human answered rather than a
/// choice. A Response that split nothing out is the ordinary end — the fixes are
/// pushed and the review is over. One that split anything out has a backlog on
/// the branch that nobody has worked, and a wrap-up that settled over the top of
/// it would reach Done with the work it agreed to still unwritten.
///
/// Asked of the record rather than of the branch, unlike [`owing_now`]: what
/// decides this is what the human picked, and the backlog being there has
/// already been established by the time this runs.
async fn carried_out(state: &AppState, conversation_id: i64) {
    if split_out(state, conversation_id).await.is_empty() {
        settle(state, conversation_id).await;

        tracing::info!(
            conversation_id,
            "the review is over, so the wrap-up carries on"
        );

        return;
    }

    built_instead(state, conversation_id).await
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
/// the log: a Conversation aborted out from under the session that wrote the
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

/// Do what the review was answered and never did, in one session that does
/// nothing else.
///
/// One session handed all of it together, which is what the review's own would
/// have done: the decisions were made, so there is nothing to propose and nothing
/// to read the branch for a second time. A session per finding would be a fresh
/// context per fix, each re-reading the diff to work out what the review already
/// wrote down.
///
/// The fixes and the backlog go to the same session and are told apart in what it
/// is handed — see [`feedback`]. They are one piece of work in the sense that
/// matters here: everything the human decided about, carried out on one branch by
/// one agent, once.
///
/// The caller is holding the Conversation's Turn, so a red check going red
/// mid-fix queues behind this rather than ending it — and the Conversation was
/// read as still wrapping up on the far side of that wait, which is what says
/// there is anywhere to work at all.
///
/// Asked of the record again afterwards, exactly as it was the first time: a fix
/// session that landed nothing has left the same work owed, and letting that one
/// through would be the failure this whole path exists to close.
async fn land(state: &AppState, conversation_id: i64, owed: Owing) {
    let writing = crate::runner::address(state, conversation_id, &feedback(&owed)).await;

    let owed = owing_now(state, conversation_id).await;

    if !owed.nothing() {
        return dropped(state, conversation_id, &owed, None, writing).await;
    }

    tracing::info!(
        conversation_id,
        "what the review was owed has landed, so the wrap-up carries on"
    );

    carried_out(state, conversation_id).await
}

/// What the fix session is told: every finding the human decided about, in the
/// words the review wrote for whoever would carry them out, and whatever they
/// said beside each answer.
///
/// Their words go under each finding rather than over it: the finding says what
/// is wrong, and this says what they thought about it. "Yes, but leave the
/// public signature alone" is only worth writing if it reaches the session that
/// can act on it.
///
/// Two instructions where the human answered both ways, and they are different
/// work: what was accepted is fixed on the branch, and what was split out is
/// written down as a backlog for sessions of its own rather than built here.
///
/// Nothing here is put as a question. The Set was answered and the answers are
/// what this is made of, so a session that came back with a proposal would be
/// asking the human to decide something they already have.
fn feedback(owed: &Owing) -> String {
    let mut told = String::from(
        "The review of this branch raised what is below, and the human has already said \
         what to do about each of them. The session that was to carry that out ended \
         without landing anything, so what is left is the doing rather than the deciding: \
         none of this is still a question.\n",
    );

    if !owed.fixes.is_empty() {
        told.push_str(&format!(
            "\nFix {each}, commit, and push so the pull request has {it}.\n\n{findings}\n",
            each = match owed.fixes.len() {
                1 => "this",
                _ => "each of these",
            },
            it = match owed.fixes.len() {
                1 => "it",
                _ => "them",
            },
            findings = written(&owed.fixes),
        ));
    }

    if !owed.splits.is_empty() {
        told.push_str(&format!(
            "\nAnd write {this} into a `.tasks/` backlog: a `TODO.md` listing {them} and one \
             numbered `NN-<slug>.md` task file each, carrying what the review wrote below so \
             that whoever works it needs nothing else. Commit the backlog. Do not build {them} \
             here — a backlog is worked a session at a time, and Verkstead runs this one once \
             it is on the branch.\n\n{findings}\n",
            this = match owed.splits.len() {
                1 => "this",
                _ => "these",
            },
            them = match owed.splits.len() {
                1 => "it",
                _ => "them",
            },
            findings = written(&owed.splits),
        ));
    }

    told
}

/// One run of findings as the session reads them: the review's words, and the
/// human's under each where they wrote any.
fn written(findings: &[store::Fixing]) -> String {
    findings
        .iter()
        .map(|finding| match finding.said.trim().is_empty() {
            true => finding.what.trim().to_owned(),
            false => format!(
                "{}\n\nWhat they said when they agreed:\n\n{}",
                finding.what.trim(),
                finding.said.trim(),
            ),
        })
        .collect::<Vec<String>>()
        .join("\n\n---\n\n")
}

/// The findings this Conversation's review was told to fix and nothing has
/// landed.
///
/// Empty is the ordinary answer and covers every way there is nothing owed: no
/// review has asked, the Set is still waiting on the human, they declined every
/// finding, or the session that was going to fix them did so.
///
/// A store that will not answer reads as *nothing owed*, which is the right way
/// round for what is on the other side of this: stopping the run and letting an
/// agent loose in a Worktree. The error is in the log, where a broken database
/// says everything else it has to say.
async fn unlanded(state: &AppState, conversation_id: i64) -> Vec<store::Fixing> {
    match store::unlanded_fixes(&state.pool, conversation_id).await {
        Ok(owed) => owed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a review was owed failed");
            Vec::new()
        }
    }
}

/// The findings this Conversation's review was told to split out into a backlog
/// of their own, landed or not.
///
/// A store that will not answer reads as *nothing split out*, which is the right
/// way round for both things this decides: whether to stop the run, and whether
/// to send a Conversation back down the ladder.
async fn split_out(state: &AppState, conversation_id: i64) -> Vec<store::Fixing> {
    match store::split_out(&state.pool, conversation_id).await {
        Ok(split) => split,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a review split out failed");
            Vec::new()
        }
    }
}

/// Everything the human decided about that has nothing to show for it.
///
/// The two halves are owed differently because they land differently. A fix is
/// owed until something is committed after the Answers, which is the record's
/// question and the store's to answer. A split is owed until there is a backlog
/// on the branch — a `.tasks/` list, committed as it stands — which is the
/// Worktree's question and no reading of the record can stand in for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Owing {
    /// Findings the human said to fix here, with nothing committed since.
    fixes: Vec<store::Fixing>,

    /// Findings they said to split out, with no backlog written.
    splits: Vec<store::Fixing>,
}

impl Owing {
    /// Whether nothing at all is owed, which is the ordinary answer.
    fn nothing(&self) -> bool {
        self.fixes.is_empty() && self.splits.is_empty()
    }
}

/// What this Conversation's review is owed as things stand.
///
/// Asked afresh every time rather than remembered, for the reason the record is
/// asked rather than the session trusted: what is owed is a question about the
/// branch and the Set, and both move while a session runs.
///
/// The branch is only asked about where something was split out, which is the
/// rare case: an ordinary review owes a `git status` of one path to nobody.
async fn owing_now(state: &AppState, conversation_id: i64) -> Owing {
    let fixes = unlanded(state, conversation_id).await;
    let splits = split_out(state, conversation_id).await;

    if splits.is_empty() || backlog(state, conversation_id).await {
        return Owing {
            fixes,
            splits: Vec::new(),
        };
    }

    Owing { fixes, splits }
}

/// Whether the backlog a review split its findings out into is on the branch.
///
/// A Conversation with no Worktree left has nowhere for one to be, and a store
/// that will not answer has said nothing about whether it is there. Both read as
/// *not written*, which is the right way round for what is on the other side of
/// this: a stop the human can clear with a glance at the branch,
/// against a wrap-up that carried on as though work nobody had written was done.
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

    /// The review put its findings up on this Set and nothing is running for it.
    /// Whether that is work to dispatch or a run to stop is [`unattended`]'s to
    /// say.
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

/// Which Set this Conversation's review put its findings on, where it has put
/// them anywhere.
///
/// A store that will not answer reads as *it never asked*, which is the same way
/// round the rest of this module reads one: everything on the other side of a
/// `None` here waits on the Worktree and asks the record again.
async fn asked(state: &AppState, conversation_id: i64) -> Option<i64> {
    match store::review_asked(&state.pool, conversation_id).await {
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
/// archive on the human's behalf because it knows something they cannot see,
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
/// [`store::Decision::Deliberate`]: what to do about it is Resume, which reads the
/// branch again in a session as fresh as the first, or taking the branch over,
/// or aborting the run with it exactly as it stands.
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
    match store::archive_set(&state.pool, &state.settlements, set_id).await {
        Ok(store::Archiving::Archived(_)) => tracing::info!(
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
/// themselves, or abort.
///
/// [`store::Decision::Deliberate`], because a wrap-up that goes on without its
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

/// Stop the run: the human decided about findings that nothing came of, and only
/// they can say what happens now.
///
/// The Notice names the doing rather than the reading, because that is the half
/// that failed — and it says what is owed in the review's own words, so what to
/// do about it is answerable without opening the Set again.
///
/// [`store::Decision::Deliberate`]: what to do is Resume, which is the doing over
/// again in one session, or the human doing it themselves, or aborting the run
/// with the branch exactly as the session left it.
///
/// `how` is how the session ended where it ended badly, and `writing` the Event
/// it was printing into — both absent for a session that saw itself out and
/// simply never pushed.
async fn dropped(
    state: &AppState,
    conversation_id: i64,
    owed: &Owing,
    how: Option<&str>,
    writing: Option<i64>,
) {
    if let Err(error) = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        what_failed(owed),
        &owing(owed, how),
        writing,
    )
    .await
    {
        tracing::error!(
            error = ?error,
            conversation_id,
            "a review's decided findings were never acted on and the stop saying so \
             could not be recorded"
        );
    }
}

/// Which half of the doing failed, as the step line above the evidence says it.
///
/// Three answers because there are three ways to owe something, and the human
/// reads this line before they read anything else: a review that pushed its
/// fixes and never wrote the backlog is a different thing to go and look at from
/// one that did neither.
fn what_failed(owed: &Owing) -> &'static str {
    match (owed.fixes.is_empty(), owed.splits.is_empty()) {
        (false, true) => "landing the fixes the review's findings were accepted for",
        (true, false) => "writing the backlog the review's findings were split out into",
        _ => "carrying out what was decided about the review's findings",
    }
}

/// How much of a finding the Notice carries.
///
/// This is one line read on a phone, under the stop it belongs to, so
/// what it is for is recognising which fix rather than reading it. The whole of
/// each is on the Set the human answered, one row up the same Timeline.
const OWED_WIDTH: usize = 100;

/// What is owed, as the Notice says it: how many of each, and the review's
/// own words for every one of them.
///
/// The review's words rather than Verkstead's, because the human decided against
/// those words an hour ago and these are the ones they will recognise.
fn owing(owed: &Owing, how: Option<&str>) -> String {
    let mut halves = Vec::new();

    if !owed.fixes.is_empty() {
        halves.push(format!(
            "{} the human accepted {} landed: {}",
            match owed.fixes.len() {
                1 => "one fix".to_owned(),
                n => format!("{n} fixes"),
            },
            match owed.fixes.len() {
                1 => "was never",
                _ => "were never",
            },
            in_lines(&owed.fixes),
        ));
    }

    if !owed.splits.is_empty() {
        halves.push(format!(
            "{} the human split out {} written into a backlog: {}",
            match owed.splits.len() {
                1 => "one finding".to_owned(),
                n => format!("{n} findings"),
            },
            match owed.splits.len() {
                1 => "was never",
                _ => "were never",
            },
            in_lines(&owed.splits),
        ));
    }

    let what = halves.join(", and ");

    match how {
        Some(how) => format!("{how}, and {what}"),
        None => format!("it ended without pushing, and {what}"),
    }
}

/// A run of findings as one line of the Notice's evidence.
fn in_lines(findings: &[store::Fixing]) -> String {
    findings
        .iter()
        .map(|finding| format!("“{}”", in_a_line(&finding.what)))
        .collect::<Vec<String>>()
        .join("; ")
}

/// One finding on one line: whitespace collapsed, and clamped to what a line
/// holds.
///
/// Shared with the batch sessions' own stop, which says what it is owed the same
/// way and in the same line — see [`crate::responding`].
pub(crate) fn in_a_line(what: &str) -> String {
    let said = what.split_whitespace().collect::<Vec<&str>>().join(" ");

    match said.char_indices().nth(OWED_WIDTH) {
        Some((cut, _)) => format!("{}…", said[..cut].trim_end()),
        None => said,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(what: &str, said: &str) -> store::Fixing {
        store::Fixing {
            what: what.to_owned(),
            said: said.to_owned(),
        }
    }

    /// What is owed where the human said to fix everything here.
    fn fixing(findings: Vec<store::Fixing>) -> Owing {
        Owing {
            fixes: findings,
            splits: Vec::new(),
        }
    }

    /// And where they said to split everything out instead.
    fn splitting(findings: Vec<store::Fixing>) -> Owing {
        Owing {
            fixes: Vec::new(),
            splits: findings,
        }
    }

    /// What a fix session is told about the findings: the review's own words for
    /// whoever would fix them, and that these are decisions rather than
    /// proposals.
    #[test]
    fn a_fix_session_is_told_every_accepted_finding_at_once() {
        let told = feedback(&fixing(vec![
            finding("`window.rs` never resets the counter between windows.", ""),
            finding("`limits.rs` and `window.rs` each grew their own clock.", ""),
        ]));

        assert!(
            told.contains("`window.rs` never resets the counter")
                && told.contains("each grew their own clock"),
            "both findings, in the words the review wrote: {told}",
        );
        assert!(
            told.contains("Fix each of these") && told.contains("none of this is still a question"),
            "and that the deciding is over: {told}",
        );
        assert!(
            !told.contains("What they said"),
            "with nothing said about words nobody wrote: {told}",
        );
        assert!(
            !told.contains(".tasks/"),
            "and nothing about a backlog nobody asked for: {told}",
        );
    }

    /// And their qualification, where they wrote one — which is the whole reason
    /// the Answer's free text is kept on the Set at all.
    #[test]
    fn what_the_human_wrote_alongside_reaches_the_session_that_can_act_on_it() {
        let told = feedback(&fixing(vec![finding(
            "`window.rs` never resets the counter between windows.",
            "Yes, but leave the public signature alone.",
        )]));

        assert!(
            told.contains("leave the public signature alone"),
            "their words reach the session: {told}",
        );
        assert!(
            told.find("never resets the counter") < told.find("leave the public signature"),
            "under the finding rather than over it: {told}",
        );
    }

    /// A finding the human split out is owed a backlog rather than a fix, and the
    /// session is told the difference: write it down, and do not build it here.
    #[test]
    fn a_split_finding_is_told_as_a_backlog_to_write_rather_than_work_to_do() {
        let told = feedback(&splitting(vec![finding(
            "The whole clock abstraction wants rebuilding.",
            "Agreed, but keep the public signature.",
        )]));

        assert!(
            told.contains("`.tasks/` backlog") && told.contains("TODO.md"),
            "what to write: {told}",
        );
        assert!(
            told.contains("Do not build it here"),
            "and that writing it is the whole of the job: {told}",
        );
        assert!(
            told.contains("The whole clock abstraction")
                && told.contains("keep the public signature"),
            "carrying the review's words and the human's: {told}",
        );
        assert!(
            !told.contains("commit, and push"),
            "with nothing said about fixes nobody accepted: {told}",
        );
    }

    /// A Response answered both ways is two instructions, because it is two
    /// different pieces of work — and each finding is under the one it belongs
    /// to.
    #[test]
    fn a_mixed_response_tells_the_session_to_fix_one_and_write_the_other_down() {
        let told = feedback(&Owing {
            fixes: vec![finding("Reset the counter as the window rolls.", "")],
            splits: vec![finding("Rebuild the clock abstraction.", "")],
        });

        assert!(
            told.contains("Fix this") && told.contains("`.tasks/` backlog"),
            "both instructions: {told}",
        );
        assert!(
            told.find("Reset the counter") < told.find("`.tasks/` backlog"),
            "the fixes under the one that asks for them: {told}",
        );
        assert!(
            told.find("`.tasks/` backlog") < told.find("Rebuild the clock"),
            "and the split findings under theirs: {told}",
        );
    }

    /// What the Notice says: that the fixes never landed, and which ones.
    #[test]
    fn the_notice_says_what_is_unlanded_in_the_review_s_own_words() {
        let owed = fixing(vec![
            finding("Reset the counter as the window rolls.", ""),
            finding("Collapse the two clocks onto one.", ""),
        ]);

        let says = owing(&owed, None);

        assert!(
            says.contains("2 fixes") && says.contains("never landed"),
            "how much is owed: {says}",
        );
        assert!(
            says.contains("Reset the counter") && says.contains("Collapse the two clocks"),
            "and what, as the review wrote it: {says}",
        );
        assert_eq!(
            what_failed(&owed),
            "landing the fixes the review's findings were accepted for",
            "under the half of the doing that failed",
        );
    }

    /// A backlog nobody wrote is owed the same way and says so in its own words:
    /// what failed there is the writing rather than the landing.
    #[test]
    fn a_split_that_was_never_written_says_the_backlog_is_what_is_missing() {
        let owed = splitting(vec![finding("Rebuild the clock abstraction.", "")]);

        let says = owing(&owed, None);

        assert!(
            says.contains("one finding") && says.contains("never written into a backlog"),
            "what is owed: {says}",
        );
        assert!(
            says.contains("Rebuild the clock abstraction"),
            "and which: {says}",
        );
        assert_eq!(
            what_failed(&owed),
            "writing the backlog the review's findings were split out into",
        );
    }

    /// A session that did neither says both, which is the one card the human
    /// reads before they go and look at the branch.
    #[test]
    fn a_session_that_landed_neither_says_both_halves() {
        let owed = Owing {
            fixes: vec![finding("Reset the counter as the window rolls.", "")],
            splits: vec![finding("Rebuild the clock abstraction.", "")],
        };

        let says = owing(&owed, None);

        assert!(
            says.contains("one fix") && says.contains("one finding"),
            "both halves: {says}",
        );
        assert_eq!(
            what_failed(&owed),
            "carrying out what was decided about the review's findings",
        );
    }

    /// A session that fell over says both: how it ended, and what it left owed.
    #[test]
    fn a_session_that_ended_badly_says_so_beside_what_it_left() {
        let says = owing(
            &fixing(vec![finding("Reset the counter as the window rolls.", "")]),
            Some("exited with status 1"),
        );

        assert!(
            says.contains("exited with status 1") && says.contains("one fix"),
            "how it ended and what is owed: {says}",
        );
    }

    /// A finding written for an agent is paragraphs long, and this is a line on a
    /// card: it is collapsed and clamped, and says that it was.
    #[test]
    fn a_finding_too_long_for_a_card_is_clamped_rather_than_wrapped() {
        let what = format!("first line\n  second line\n\n{}", "x".repeat(200));
        let line = in_a_line(&what);

        assert!(
            line.starts_with("first line second line"),
            "collapsed onto one line: {line}",
        );
        assert!(line.ends_with('…'), "and clamped, visibly: {line}");
        assert!(
            line.chars().count() <= OWED_WIDTH + 1,
            "to what a card holds: {line}",
        );
    }
}
