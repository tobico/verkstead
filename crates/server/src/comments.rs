//! What is said on a wrapping Conversation's pull requests, and the sessions it
//! dispatches.
//!
//! The other half of what a human can do to an open pull request. They read the
//! branch and they write on it, and Verkstead reads what they wrote through the
//! host's `gh` — for as long as the Conversation is Wrapping and no longer, on
//! the same interval the checks are asked about.
//!
//! **One watcher per pull request**, because a conversation is a fact about a
//! pull request rather than about a Conversation: a Conversation ends on one per
//! repository it was worked in, each read in its own repository — `#7` means
//! something else in another one, or nothing. [`watching`] starts one for every
//! pull request on the record, and [`crate::wrapping::covering`] starts one for
//! each companion's as it finds it.
//!
//! All three places they write count: the pull request's conversation, the words
//! at the top of a review, and the comments left on the lines of the diff. The
//! last is where a review of code mostly happens, so a watcher that read only
//! the conversation would miss the feedback it most needs to act on — see
//! [`crate::github::comments`], which is where the three are read as one. Two
//! `gh` calls a poll per pull request, so four a minute beside the checks
//! watcher's two, and a Conversation rarely ends on more than a couple.
//!
//! **What was already said belongs to the review.** A wrap-up starts with one
//! session that reads the branch and proposes what should change, and the
//! comments standing on every one of the pull requests when it starts are part
//! of what it reads — see [`for_the_review`]. They are recorded as addressed the
//! moment that session is dispatched, so nothing here dispatches about them
//! afterwards, and what they ask for reaches the human as a Question like every
//! other finding rather than as work somebody was quietly sent to do.
//!
//! Which is why nothing is dispatched from here until the review is over. A
//! comment sitting on a pull request while it runs is one the review has folded
//! in or is about to, and a batch session started over the top of that would be
//! the ungated half of this all over again. What lands afterwards is the batch's
//! again, once the review has settled and the Worktree is free.
//!
//! Comments said after that dispatch a **batch session**: a fresh session under
//! the Conversation's implementation Profile, inside the bundled responding
//! skill, given what was said and nothing about what to do with it. It proposes
//! what it would do as one small Set, waits, and lands what the human accepted —
//! the review's shape one turn later, and for the review's reason. What becomes
//! of it is [`crate::responding`]'s.
//!
//! **The batch session is told which repository, which pull request and where to
//! work**, exactly as a fix session is. A session starts in the Conversation's
//! own worktree and both `git` and `gh` read their repository from wherever they
//! run, so one sent at a companion's pull request would otherwise read the wrong
//! repository's diff and push to the wrong branch. Every companion's worktree is
//! bound into the sandbox already, so the directory the feedback names is one the
//! session can simply work in — see [`feedback`], and the bundled responding
//! skill, which is written for a session that may be sent outside the worktree it
//! starts in.
//!
//! **One session per batch rather than one per comment.** A human writing three
//! replies in a minute is making one point, and three sessions racing each other
//! in one Worktree is the thing a batch prevents.
//!
//! **And one batch at a time, still.** A session takes the Conversation's Turn,
//! so comments on two pull requests are answered one after the other rather than
//! by two agents in overlapping worktrees: the watcher that cannot get the Turn
//! comes back to its own pull request a poll later rather than queueing behind
//! somebody else's batch.
//!
//! Which comments have been dispatched for is written down rather than held in
//! memory — see [`store::record_addressed_comments`]. A server that came back up
//! and read every comment as new would dispatch a session about feedback that
//! was addressed yesterday. Written down per pull request, which is what lets one
//! of them go quiet while another is still being answered: what settles is *this
//! pull request has nothing outstanding* rather than one answer covering all of
//! them.
//!
//! Written down as a session is dispatched rather than as it finishes, which
//! buys that at a price: a batch whose session is gone leaves a record saying
//! somebody dealt with what was said and nobody who did. So nothing is settled
//! here until that has been asked about — see
//! [`crate::responding::unattended`], which is where a proposal nobody is left
//! behind is picked up.
//!
//! Nor while the Worktree is busy. The same trade means a batch that has only
//! just started reads here exactly as one that is over — its comments are
//! addressed and it has not asked anything yet — and a wrap-up has more than one
//! of these watchers whenever a press starts its five over the top of the five
//! already running. So the Turn is asked for before anything is settled, and a
//! Worktree with a session in it settles nothing: without that, one watcher
//! could settle over the batch the other had just dispatched, and the wrap-up
//! reach Done with the session that was going to put the question still working.
//!
//! A `gh` that cannot answer changes nothing at all — it does not settle, it does
//! not unsettle, and it dispatches nothing. That is the only honest reading of
//! it: Verkstead does not know what has been said, and *nobody said anything* is
//! not a thing to conclude from not knowing.
//!
//! **Except what Verkstead itself said.** The comment Share to Pull Request
//! leaves ends with a marker of its own — see [`verkstead_render::SHARE_MARKER`]
//! — and a comment carrying it at the start of a line is dropped wherever the
//! comments are read: the fresh ones a batch session would be dispatched about
//! and the standing ones folded into the review alike, on a companion's pull
//! request as much as on the Conversation's own. It is posted by the configured
//! token, which is usually the human's own account, so nothing about who said it
//! could tell it from what they write themselves. See [`verkstead_said_it`].
//!
//! **And except what the human said to ignore.** `config.yaml` holds a list of
//! rules — a regex over who said it, a regex over what it says, or both — and a
//! comment any one of them matches is skipped in both readers exactly as the
//! share's own is. See [`ignored`]. The rules are read off the file every poll,
//! so one added on a phone takes effect on the next one without a restart.
//!
//! The one difference from the share's drop is that **a skipped comment is
//! written down as addressed**. The share's marker is Verkstead's own for as
//! long as that comment exists, so inspection answers for it for ever; a rule is
//! the human's and they may delete it, and a rule deleted after months of a bot
//! nagging would otherwise bring the whole of it back as sessions in one poll.
//! Writing each one down as it is skipped is what makes taking a rule away
//! change what happens next rather than what happened.
//!
//! Nothing here ever asks the human itself. What asks is the session dispatched
//! about a batch, which puts what it would do to them rather than what they
//! said: their own words back at them would be the one question with nothing
//! behind it.

use crate::AppState;
use crate::github::Comment;
use crate::store;
use crate::wrapping::{Watched, named};

/// Read every pull request `conversation_id` is on, one watcher each.
///
/// What a wrap-up starts with, and what a server coming back up and a Resume
/// start again — each of which starts the whole of a wrap-up rather than some of
/// it. The pull requests are read here rather than passed in, for the reason
/// every other watcher reads the record: what this is looking at is a wrap-up
/// with nothing running, whatever put it there.
///
/// One task each, so that a pull request nobody can ask about holds up nothing
/// else, and awaited together, so that the whole of this counts as one driver of
/// the Conversation for as long as any of them is going — see
/// [`crate::wrapping::watching`].
///
/// A companion's pull request found after this started gets its own watcher
/// where it is recorded, that being the moment there is one to read: see
/// [`crate::wrapping::covering`].
pub(crate) async fn watching(state: AppState, conversation_id: i64) {
    let opened = match store::pull_requests(&state.pool, conversation_id).await {
        Ok(opened) => opened,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which pull requests to read the comments of failed");
            return;
        }
    };

    let watchers: Vec<_> = opened
        .into_iter()
        .map(|(repo, _)| tokio::spawn(watch(state.clone(), conversation_id, repo.id)))
        .collect();

    for watcher in watchers {
        if let Err(error) = watcher.await {
            tracing::error!(error = ?error, conversation_id, "a comments watcher ended badly");
        }
    }
}

/// Watch what is said on the pull request `conversation_id` opened in `repo_id`
/// until it stops wrapping up.
///
/// Returns when there is nothing left to watch: the Conversation has moved on or
/// gone, that repository has no pull request on the record any more, or driving
/// stopped. Idle rather than looping, for the checks watcher's reason — nothing
/// advances past a stop, and a watcher that dispatched sessions behind one would
/// be working on a run the human has stopped.
///
/// Nothing here is refused for. This runs unattended with nobody watching, and
/// what it has to say it says on the Timeline or in the log.
pub(crate) async fn watch(state: AppState, conversation_id: i64, repo_id: i64) {
    loop {
        if let Watching::Done(why) = once(&state, conversation_id, repo_id).await {
            tracing::info!(
                conversation_id,
                repo_id,
                why,
                "a pull request's comments are no longer being read"
            );
            return;
        }

        tokio::time::sleep(state.sessions.pace().checks).await;
    }
}

/// What one look at the comments decided.
enum Watching {
    /// Look again after the interval.
    Again,

    /// Stop watching, for this reason.
    Done(&'static str),
}

/// Take one look: ask GitHub what has been said, and dispatch a session for
/// whatever is new.
async fn once(state: &AppState, conversation_id: i64, repo_id: i64) -> Watching {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return Watching::Done("there is no Conversation left to read comments for"),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to read comments for failed");
            return Watching::Again;
        }
    };

    // The one thing that ends the watching by itself. Everything a Conversation
    // leaves Wrapping for — Done, or closed from the menu — arrives here as the
    // same fact: this is not a wrap-up any more.
    if conversation.state != store::Lifecycle::Wrapping {
        return Watching::Done("the Conversation is not wrapping up any more");
    }

    // Asked before anything is dispatched, for the runner's reason: *the run does
    // not advance past a stop* means no session is launched while the human is
    // the only thing that can start one — an account out of window included, see
    // [`crate::stopping::stopped`].
    if crate::stopping::stopped(state, conversation_id).await {
        return Watching::Done("driving has stopped");
    }

    // Before anything is read of GitHub or settled here: a batch session that
    // asked and is no longer running has left the human a question with nobody
    // behind it, and the comments it was dispatched about written down as dealt
    // with. Settling over that would take the Conversation to Done with the Set
    // still open — see [`crate::responding::unattended`], which sees to whatever
    // is outstanding and says whether it found anything.
    //
    // Asked by every one of these watchers rather than by one of them, and asked
    // of the Conversation rather than of this pull request: a batch is the
    // Conversation's Turn spent, whichever pull request it was answering, and a
    // wrap-up with one left behind is one no pull request settles under.
    //
    // Asked only once the review is over, because a batch is only dispatched
    // once it is: until then there is nothing here for anybody to have left
    // behind, and the review's own Set is [`crate::review`]'s to see to whatever
    // becomes of it.
    if reviewed(state, conversation_id).await
        && crate::responding::unattended(state, conversation_id).await
    {
        return Watching::Again;
    }

    let opened = match store::pull_request(&state.pool, conversation_id, repo_id).await {
        Ok(Some(opened)) => opened,
        // A Conversation wrapping up has a pull request in the repository whose
        // watcher this is — a watcher is started where one is recorded and never
        // before — so this is a record that has been got at rather than a wrap-up
        // to carry on with.
        Ok(None) => return Watching::Done("that repository has no pull request to read"),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, repo_id, "reading the pull request to read comments on failed");
            return Watching::Again;
        }
    };

    // Which repository to ask in and which checkout its work is done in, read off
    // the Conversation every poll rather than held: a companion taken away is one
    // there is nowhere left to ask about.
    let Some(watched) = crate::wrapping::watched(&conversation, repo_id, opened.number) else {
        return Watching::Done("there is no repository left to read that pull request in");
    };

    // Nothing is touched where GitHub could not be asked, and the next poll asks
    // again — of a `gh` that may by then have been logged in.
    let Some(fresh) = unaddressed(state, conversation_id, &watched).await else {
        return Watching::Again;
    };

    if fresh.is_empty() {
        // Not until the review is over. *Nothing is unaddressed* is a reading of
        // a moment, and one taken before the review started outlives it: a
        // comment that lands while it runs puts nothing back to waiting, because
        // nothing was waiting to be put back — so the wrap-up can reach Done on
        // the older reading before the next poll has seen what was said, and what
        // was written goes unanswered on a Conversation that is over. So the
        // settling waits for the review exactly as the dispatching below does,
        // and the first poll after it is the one that decides. Which is the same
        // rule read twice: until the review has settled, nothing said on any of
        // the pull requests is anybody else's to act on — settling it away
        // included.
        if !reviewed(state, conversation_id).await {
            tracing::debug!(
                conversation_id,
                repo = watched.repo.name,
                "the review has not finished, so nothing said is settled yet",
            );

            return Watching::Again;
        }

        // Nor while something is working in the Worktree. A batch is written down
        // as addressed the moment it is dispatched and before its session has said
        // a word, so *nothing is unaddressed* is also what a batch that has only
        // just started looks like from a second watcher — and a wrap-up has more
        // than one of these the moment a press starts its watchers over the top of
        // the ones already running. Settling over a batch that has not asked yet
        // is the same failure [`crate::responding::unattended`] refuses to settle
        // over once it has: a Conversation carried to Done with the session that
        // was going to put the question still working.
        //
        // The Turn is what tells the two apart, tried rather than waited for
        // because this is a poll. A Worktree that is busy settles nothing and the
        // next poll asks again, of a Worktree that by then is free — and the Turn
        // is held across the settling itself, so that a batch dispatched between
        // the asking and the writing cannot be settled over either.
        if let Some(_turn) = state.sessions.try_turn(conversation_id) {
            settle(state, conversation_id, &watched).await;
        } else {
            tracing::debug!(
                conversation_id,
                repo = watched.repo.name,
                "something is working in the Worktree, so what was said is settled later",
            );
        }

        return Watching::Again;
    }

    // Said before anything is dispatched, because what wrap-up waits on is
    // nothing being left unaddressed on any of its pull requests, and something
    // is on this one.
    unsettle(state, conversation_id, &watched).await;

    // And nothing is dispatched at all until the review is over: everything
    // standing on the pull requests while it runs is the review's to propose
    // about, and a batch session started over the top of that would be acting on
    // a comment nobody had agreed to act on.
    if !reviewed(state, conversation_id).await {
        tracing::debug!(
            conversation_id,
            repo = watched.repo.name,
            comments = fresh.len(),
            "the review has not finished, so what has been said is left for it",
        );
        return Watching::Again;
    }

    dispatch(state, conversation_id, &watched, &fresh).await
}

/// What is on this pull request that nobody has been sent to deal with yet.
///
/// The two readers of the comments share this, because *what is new* is one
/// question however differently the two answer it: the watcher looks again after
/// the interval and the review goes on without them.
///
/// Asked of one pull request rather than of the Conversation, which is what
/// keeps two of them apart: `gh` is run in that pull request's own repository,
/// and what has already been dispatched for is read for that pull request alone.
///
/// `None` is GitHub not having been asked, which is neither *nothing was said*
/// nor *something was*. An empty list is the answer that there is nothing new,
/// which is every pull request the moment it opens.
///
/// What a share left is not new and never was — see [`verkstead_said_it`], which
/// is the one comment neither of the two readers is ever given. Nor is anything
/// the human's own ignore rules match — see [`ignored`], which differs from the
/// share's drop in one thing: what it skips is written down as addressed on the
/// way past.
async fn unaddressed(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
) -> Option<Vec<Comment>> {
    let asked = {
        let gh = state.github.clone();
        let repo = watched.repo.path.clone();
        let number = watched.number;

        // Off the runtime's threads: this is a process, and one that goes to the
        // network.
        tokio::task::spawn_blocking(move || crate::github::comments(&gh, &repo, number)).await
    };

    let said = match asked {
        Ok(Ok(said)) => said,
        Ok(Err(trouble)) => {
            tracing::warn!(
                conversation_id,
                repo = watched.repo.name,
                number = watched.number,
                why = trouble.why(),
                "the comments could not be read, so nothing is decided about them",
            );
            return None;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking gh about the comments failed");
            return None;
        }
    };

    let already = match store::addressed_comments(&state.pool, conversation_id, watched.repo.id)
        .await
    {
        Ok(already) => already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "reading which comments had been dispatched for failed");
            return None;
        }
    };

    // Read off the file every poll rather than held, which is what makes a rule
    // added on a phone take effect on the next one. One small file, read here
    // rather than on a blocking thread for [`crate::checks`]'s reason: a poll
    // that has just run `gh` twice is not a hot path.
    let config = state.settings.config();
    let rules = config.ignored_comments();

    let mut fresh = Vec::new();
    let mut skipped = Vec::new();

    for comment in said {
        if already.contains(&comment.which) || verkstead_said_it(&comment.markdown) {
            continue;
        }

        match ignored(rules, &comment) {
            true => skipped.push(comment.which),
            false => fresh.push(comment),
        }
    }

    if !skipped.is_empty() {
        tracing::info!(
            conversation_id,
            repo = watched.repo.name,
            number = watched.number,
            comments = skipped.len(),
            "the ignore rules match what was said, so nobody is being sent to address it",
        );

        // Written down as addressed at the moment it is skipped, which is what
        // makes taking a rule away non-retroactive: a bot's months of nagging
        // would otherwise all come back as sessions on the day the human deletes
        // the rule that silenced it.
        //
        // A recording that failed does not put the comment back. What is
        // dispatched about is decided by the rules as they stand, so a store
        // that would not answer costs this the writing down and nothing else —
        // the next poll skips the comment again, and writes it down again.
        if let Err(error) = store::record_addressed_comments(
            &state.pool,
            conversation_id,
            watched.repo.id,
            &skipped,
        )
        .await
        {
            tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "recording which comments the ignore rules skipped failed");
        }
    }

    Some(fresh)
}

/// Whether `comment` is one of the classes the human never wants addressed.
///
/// The rules are theirs rather than Verkstead's — a review service filing the
/// same word about billing on every pull request, say — and they are read off
/// `config.yaml` fresh every poll. Any one rule matching is enough, and a rule
/// matches where every field it gives does: the author pattern against the login
/// of whoever said it, the body pattern against the markdown they wrote. See
/// [`crate::settings::IgnoreRule::matches`], which is where the whole of the
/// matching lives.
///
/// An account GitHub no longer has arrives with an empty login, which is matched
/// as the empty string: a rule naming an author does not match it, and a rule
/// about only the body goes on matching what it says.
fn ignored(rules: &[crate::settings::IgnoreRule], comment: &Comment) -> bool {
    rules
        .iter()
        .any(|rule| rule.matches(&comment.author, &comment.markdown))
}

/// Whether this is the comment a share left rather than something somebody wants
/// addressed.
///
/// Share to Pull Request writes as the configured token, which is usually the
/// human's own account — so no rule about who said it could tell Verkstead's own
/// comment from theirs, and the marker in the body is what does instead. Built
/// in and never configurable: it is Verkstead answering for what Verkstead
/// wrote.
///
/// **At the start of a line**, which is what leaves a reply to the share
/// something to answer: quote-replying on GitHub prefixes every line with `>`,
/// so the marker travels along inside the quote without ever beginning a line,
/// and what the human wrote under it is dispatched for like anything else.
///
/// Dropped by inspection on every poll rather than written down as addressed.
/// The comment is Verkstead's own for as long as it exists, so there is nothing
/// to remember: a server that came back up reads it as its own again.
fn verkstead_said_it(markdown: &str) -> bool {
    markdown
        .lines()
        .any(|line| line.starts_with(verkstead_render::SHARE_MARKER))
}

/// Whether the wrap-up's review is over, which is what says a comment is a batch
/// session's rather than the review's.
///
/// One review across every pull request, so this is the Conversation's own
/// question rather than any pull request's: until it has settled, nothing said on
/// any of them is anybody else's to act on.
///
/// A store that will not answer reads as *not yet*, which is the right way round
/// for the one thing this decides: on the other side of it is an agent being let
/// loose on feedback nobody has been asked about.
async fn reviewed(state: &AppState, conversation_id: i64) -> bool {
    match store::wrap_up_settled(&state.pool, conversation_id).await {
        Ok(settled) => settled.contains(&store::WaitingOn::Review),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether the review was over failed");
            false
        }
    }
}

/// Everything said on every one of `conversation_id`'s pull requests that nobody
/// has been sent to deal with, written out for the review session that is about
/// to start — and recorded as addressed, because that session is who deals with
/// it.
///
/// **Across all of them**, because there is one review and it reads the work
/// whole: a Conversation ends on a pull request per repository it was worked in,
/// and a review given one of them would leave the rest for a batch session to be
/// dispatched about ungated. Each comment says which pull request it was left on
/// — *this is the wrong way round* is an instruction with the repository and the
/// line and a riddle without.
///
/// `None` where there is nothing to fold in, which covers pull requests nobody
/// has written on and a `gh` that could not be asked. The second of those is the
/// module's rule again: *nobody said anything* is not a thing to conclude from
/// not knowing, so the review runs on the branches alone and the batch that comes
/// after it picks up what was there. Read pull request by pull request, so one
/// that cannot be asked about costs the review only that one's comments.
///
/// Written down as the session is dispatched rather than as it ends, for the
/// reason a batch's are — see [`store::record_addressed_comments`]. Recording
/// them and then failing to launch would lose them, which is the same trade every
/// dispatch here makes and the same one that keeps a restarted server from
/// dispatching twice.
///
/// Nothing races this for a comment. [`once`] dispatches nothing until the review
/// has settled, so every comment standing here is one the watchers have left
/// alone on purpose — and the caller is holding the Worktree's Turn besides,
/// which is what makes *present at review start* a moment rather than an
/// approximation. What lands after this reads is the batch's, once the review is
/// over.
pub(crate) async fn for_the_review(state: &AppState, conversation_id: i64) -> Option<String> {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to read comments for failed");
            return None;
        }
    };

    let opened = match store::pull_requests(&state.pool, conversation_id).await {
        Ok(opened) => opened,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the pull requests to read comments on failed");
            return None;
        }
    };

    let mut said = Vec::new();
    let mut comments = 0;

    for (repo, pull_request) in opened {
        let Some(watched) = crate::wrapping::watched(&conversation, repo.id, pull_request.number)
        else {
            continue;
        };

        // A `gh` that could not be asked reads the same as a pull request nobody
        // has written on: the review goes ahead without that one's comments, and
        // the batch after it picks up whatever was there.
        let Some(fresh) = unaddressed(state, conversation_id, &watched).await else {
            continue;
        };

        if fresh.is_empty() {
            continue;
        }

        let which: Vec<String> = fresh.iter().map(|comment| comment.which.clone()).collect();

        // Recorded before it is folded in, so that a recording that failed leaves
        // the comments for the batch session after the review rather than in a
        // prompt nothing says anybody has seen.
        if let Err(error) =
            store::record_addressed_comments(&state.pool, conversation_id, repo.id, &which).await
        {
            tracing::error!(error = ?error, conversation_id, repo = repo.name, "recording which comments the review was given failed");
            continue;
        }

        comments += fresh.len();
        said.push(said_by(&watched, &fresh));
    }

    if said.is_empty() {
        return None;
    }

    tracing::info!(
        conversation_id,
        comments,
        pull_requests = said.len(),
        "the pull requests have been commented on, so the review is given what was said",
    );

    Some(said.join(BETWEEN))
}

/// Start one session about the whole batch, if the Worktree is free.
///
/// One agent in one Worktree. Tried rather than waited for, exactly as the checks
/// watcher tries: what else is in there is the review session, a fix the human
/// accepted, or a batch about another pull request's comments, all of which take
/// as long as they take. Nothing is lost by coming back — the comments are still
/// there, and a batch that grew while this waited is one session about more of
/// what was said, which is what a batch is for.
///
/// Once taken, the Turn is held for as long as the session lives, the wait on
/// the human included — the review's shape again, and the reason the watcher
/// takes no further look while one runs. A comment said in the meantime is the
/// next batch's, once this one is over.
async fn dispatch(
    state: &AppState,
    conversation_id: i64,
    watched: &Watched,
    fresh: &[Comment],
) -> Watching {
    let Some(_turn) = state.sessions.try_turn(conversation_id) else {
        tracing::debug!(
            conversation_id,
            repo = watched.repo.name,
            "something else is working in the Worktree, so the comments are read again later",
        );
        return Watching::Again;
    };

    let which: Vec<String> = fresh.iter().map(|comment| comment.which.clone()).collect();

    // Written down as the session is dispatched rather than as it ends, so that a
    // batch a server dispatched for and then restarted over is not dispatched for
    // twice.
    if let Err(error) =
        store::record_addressed_comments(&state.pool, conversation_id, watched.repo.id, &which)
            .await
    {
        tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "recording which comments were being dispatched for failed");
        return Watching::Again;
    }

    tracing::info!(
        conversation_id,
        repo = watched.repo.name,
        number = watched.number,
        comments = fresh.len(),
        "a pull request has been commented on, so a session is starting on it",
    );

    crate::responding::run(
        state,
        conversation_id,
        watched.repo.id,
        &feedback(watched, fresh),
        &which,
    )
    .await;

    Watching::Again
}

/// What a batch session is told: which pull request was commented on, where to
/// work, and what was said.
///
/// The worktree, for the reason a fix session is told one. A session starts in
/// the Conversation's own worktree and both `git` and `gh` read their repository
/// from wherever they run, so one sent at a companion's pull request would read
/// the wrong repository's diff and push its answer to the wrong branch. Every
/// worktree is bound into the sandbox at the path named here, so it is a
/// directory the session can simply work in.
///
/// Named the same way whichever repository it is, the Conversation's own
/// included: a session is told where it is working rather than left to infer
/// that it has not been sent anywhere.
fn feedback(watched: &Watched, fresh: &[Comment]) -> String {
    format!(
        "These comments were left on {}. Work in that repository's worktree, at `{}` — both \
         `git` and `gh` read the repository from wherever they are run, so a diff read or a \
         commit made anywhere else is a different repository's.\n\n{}",
        named(watched),
        watched.worktree.display(),
        said_by(watched, fresh),
    )
}

/// What separates one comment from the next, and one pull request's comments
/// from another's.
///
/// A rule, which is what a human reading markdown on a phone sees a break as.
const BETWEEN: &str = "\n\n---\n\n";

/// What was said, in the order it was said in, and where each of it was said.
///
/// The comments whole rather than summarised, and in the markdown they were
/// written in. This is a human talking to whoever wrote the branch, and the
/// session that reads it is the nearest thing to that: a summary would be
/// Verkstead deciding which half of the feedback mattered.
///
/// The pull request travels with every comment, and the file and line with one
/// left on the diff, because that is what it means. "This is the wrong way
/// round" is an instruction with the repository and the line and a riddle
/// without them — and a Conversation now ends on a pull request per repository
/// it was worked in, so which of them a comment is about is no longer something
/// a session could infer.
///
/// Two sessions read this and both read it the same way: the review is given
/// what was standing on every pull request when it started and a batch session
/// is given what was said on one of them after, and each of them proposes about
/// it rather than acting on it. So this is the whole of what either is told about
/// the words — what to do with them is the skill each is running inside.
fn said_by(watched: &Watched, fresh: &[Comment]) -> String {
    fresh
        .iter()
        .map(|comment| {
            let who = match comment.author.is_empty() {
                true => "Somebody".to_owned(),
                false => format!("**{}**", comment.author),
            };

            let where_said = match comment.about.is_empty() {
                true => String::new(),
                false => format!(", on {}", comment.about),
            };

            format!(
                "{who} said on {}{where_said}:\n\n{}",
                named(watched),
                comment.markdown.trim(),
            )
        })
        .collect::<Vec<String>>()
        .join(BETWEEN)
}

/// Record that nothing said on this pull request is left unaddressed, so wrap-up
/// has one less thing to wait on.
///
/// One of however many it is waiting on: a Conversation ends on a pull request
/// per repository it was worked in, and every one of them has to be quiet before
/// the wrap-up is over — see [`store::finish_wrap_up`].
async fn settle(state: &AppState, conversation_id: i64, watched: &Watched) {
    if let Err(error) = store::settle_wrap_up(
        &state.pool,
        conversation_id,
        store::WaitingOn::Comments(watched.repo.id),
    )
    .await
    {
        tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "recording that the comments are all addressed failed");
    }
}

/// And that something is, which is a comment nobody has been sent to deal with
/// yet.
async fn unsettle(state: &AppState, conversation_id: i64, watched: &Watched) {
    if let Err(error) = store::unsettle_wrap_up(
        &state.pool,
        conversation_id,
        store::WaitingOn::Comments(watched.repo.id),
    )
    .await
    {
        tracing::error!(error = ?error, conversation_id, repo = watched.repo.name, "putting the comments back to waiting failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(author: &str, markdown: &str) -> Comment {
        Comment {
            which: format!("IC_{author}_{}", markdown.len()),
            author: author.to_owned(),
            at: "2026-08-21T09:00:00Z".to_owned(),
            about: String::new(),
            markdown: markdown.to_owned(),
        }
    }

    /// The same, left on a line of the diff rather than in the conversation.
    fn on_a_line(about: &str, markdown: &str) -> Comment {
        Comment {
            about: about.to_owned(),
            ..comment("tobico", markdown)
        }
    }

    /// A companion's pull request, which is the one a batch session has to be
    /// sent somewhere for.
    fn watched() -> Watched {
        Watched {
            repo: store::Repo {
                id: 2,
                path: std::path::PathBuf::from("/watched/askance"),
                name: "askance".to_owned(),
                default_branch: "main".to_owned(),
            },
            number: 7,
            worktree: std::path::PathBuf::from("/state/worktrees/rate-limiting-askance"),
        }
    }

    /// What either session is told about one comment: who said it, which pull
    /// request they said it on, and what they wrote, whole.
    #[test]
    fn a_session_is_told_who_said_it_and_what_they_wrote() {
        let said = said_by(
            &watched(),
            &[comment("tobico", "Rename the `window` field.")],
        );

        assert!(
            said.contains("tobico") && said.contains("Rename the `window` field."),
            "who said it and what they wrote: {said}",
        );
        assert!(
            said.contains("#7") && said.contains("askance"),
            "and which pull request in which repository they said it on: {said}",
        );
    }

    /// A batch is one session's worth of feedback, so all of it reaches that one
    /// session — three replies in a minute are one point being made.
    #[test]
    fn every_comment_in_the_batch_reaches_the_one_session() {
        let said = said_by(
            &watched(),
            &[
                comment("tobico", "Rename the `window` field."),
                comment("tobico", "And the test that pins it."),
            ],
        );

        assert!(
            said.contains("Rename the `window` field.") && said.contains("And the test that pins"),
            "both of them: {said}",
        );
        assert!(
            said.find("Rename the `window`") < said.find("And the test that pins"),
            "in the order they were said in: {said}",
        );
    }

    /// Where a comment on the diff was left travels with it, because that is
    /// half of what it means: *this is the wrong way round* is an instruction
    /// with the file and the line and a riddle without them.
    #[test]
    fn a_comment_left_on_the_diff_carries_where_it_was_left() {
        let said = said_by(
            &watched(),
            &[on_a_line(
                "`src/window.rs` line 12",
                "This is the wrong way round.",
            )],
        );

        assert!(
            said.contains("on `src/window.rs` line 12"),
            "the file and the line: {said}",
        );
        assert!(
            said.contains("#7") && said.contains("askance"),
            "and the pull request they are a file and a line of: {said}",
        );
        assert!(
            said.contains("This is the wrong way round."),
            "and what they said about it: {said}",
        );
    }

    /// And one said about the pull request as a whole names the pull request and
    /// no place on it, rather than trailing an empty *on*.
    #[test]
    fn a_comment_about_the_whole_pull_request_names_no_place() {
        let said = said_by(
            &watched(),
            &[comment("tobico", "Rename the `window` field.")],
        );

        assert!(
            said.contains("**tobico** said on pull request #7 of `askance`:"),
            "{said}",
        );
    }

    /// A comment left by an account that has since gone is still a comment to
    /// answer, and it reads as somebody rather than as nobody.
    #[test]
    fn a_comment_with_no_author_left_is_still_something_to_answer() {
        let said = said_by(&watched(), &[comment("", "Rename the `window` field.")]);

        assert!(
            said.contains("Somebody said") && said.contains("Rename the `window` field."),
            "{said}",
        );
    }

    /// Neither session is given any instruction along with the words: both of
    /// them propose about what was said rather than doing it, and a prompt
    /// telling either to push would be the ungated half arriving by the other
    /// door.
    #[test]
    fn what_a_session_is_given_is_what_was_said_and_not_what_to_do_about_it() {
        let said = said_by(
            &watched(),
            &[
                comment("tobico", "Rename the `window` field."),
                on_a_line("`src/window.rs` line 12", "This is the wrong way round."),
            ],
        );

        assert!(
            said.contains("**tobico** said on pull request #7 of `askance`:")
                && said.contains("Rename the `window` field."),
            "who said it and what they wrote: {said}",
        );
        assert!(
            said.contains("on `src/window.rs` line 12"),
            "and where, for the one left on the diff: {said}",
        );
        assert!(
            !said.contains("push") && !said.contains("do it"),
            "with nothing telling it to act on them: {said}",
        );
    }

    /// The comment Share to Pull Request leaves is Verkstead's own, so neither
    /// session is ever given it: it is written as the configured token, which is
    /// usually the human's own account, and the marker in the body is the whole
    /// of how it is told from what they write themselves.
    #[test]
    fn the_comment_a_share_left_is_verksteads_own() {
        assert!(verkstead_said_it(&shared()));
    }

    /// And a human quote-replying to it is somebody talking. GitHub prefixes
    /// every line of a quote with `>`, so the marker rides along in the middle of
    /// a line rather than at the start of one — which is why the rule is written
    /// about the start of a line.
    #[test]
    fn a_quote_reply_of_the_share_is_still_somebody_to_answer() {
        let quoted: String = shared()
            .lines()
            .map(|line| format!("> {line}\n"))
            .collect::<String>()
            + "\nWhich of these is the one to keep?\n";

        assert!(!verkstead_said_it(&quoted), "{quoted}");
    }

    /// Nothing else is. A comment that happens to say what the marker is — this
    /// one — is a human writing about it rather than a share leaving one.
    #[test]
    fn a_comment_that_only_mentions_the_marker_is_addressed_like_any_other() {
        assert!(!verkstead_said_it(&format!(
            "The share writes {} at the end. Is that deliberate?",
            verkstead_render::SHARE_MARKER,
        )));
        assert!(!verkstead_said_it("Rename the `window` field."));
    }

    /// The share comment as it is left: the link, the itemization, and the
    /// marker on a line of its own at the end.
    fn shared() -> String {
        format!(
            "[Read this conversation](https://x/#9f1) — a read-only copy of \
             `rate-limiting`, taken 2026-08-30.\n\nA limiter that counts across \
             instances.\n\n{}\n",
            verkstead_render::SHARE_MARKER,
        )
    }

    /// A rule the human wrote is the other thing neither session is given, and
    /// what it is for: a review service filing the same word about billing on
    /// every pull request, where the alternative is a session spun up to address
    /// it each time.
    #[test]
    fn a_rule_the_human_wrote_is_what_it_says_it_is() {
        let rules = [rule(Some("coderabbitai"), Some("billing"))];

        assert!(ignored(
            &rules,
            &comment("coderabbitai", "Your billing information is missing."),
        ));
        assert!(!ignored(
            &rules,
            &comment("tobico", "Sort the billing out one of these days."),
        ));
        assert!(!ignored(
            &rules,
            &comment("coderabbitai", "This loop reads the vector twice."),
        ));
    }

    /// A rule giving one field constrains that one alone: an author with no body
    /// ignores everything that account writes, and a body with no author ignores
    /// that phrase from anybody.
    #[test]
    fn a_rule_giving_one_field_constrains_that_one_alone() {
        assert!(ignored(
            &[rule(Some("coderabbitai"), None)],
            &comment("coderabbitai", "This loop reads the vector twice."),
        ));
        assert!(ignored(
            &[rule(None, Some("billing"))],
            &comment("tobico", "Sort the billing out one of these days."),
        ));
    }

    /// And the rules combine with OR: a comment is ignored where any one of them
    /// matches, which is what makes the list a list rather than a rule with more
    /// fields.
    #[test]
    fn any_one_rule_matching_is_enough() {
        let rules = [
            rule(Some("dependabot"), None),
            rule(None, Some("(?i)billing")),
        ];

        assert!(ignored(
            &rules,
            &comment("dependabot", "Bump serde to 1.0.")
        ));
        assert!(ignored(
            &rules,
            &comment("coderabbitai", "Billing information is missing."),
        ));
        assert!(!ignored(
            &rules,
            &comment("tobico", "Rename the `window` field."),
        ));
    }

    /// A comment left on a line of the diff is matched the same way as one said
    /// in the conversation. What a rule reads is who said it and what they
    /// wrote, and both are the same comment whichever of the three places it
    /// was left in — see [`crate::github::comments`].
    #[test]
    fn a_comment_on_the_diff_is_matched_like_any_other() {
        assert!(ignored(
            &[rule(None, Some("nit:"))],
            &on_a_line("`src/window.rs` line 12", "nit: this reads oddly."),
        ));
    }

    /// A settings file nobody has written a rule into ignores nothing, which is
    /// every comment on every pull request being somebody's to address.
    #[test]
    fn no_rules_ignore_nothing() {
        assert!(!ignored(
            &[],
            &comment("coderabbitai", "Billing is missing.")
        ));
    }

    /// An account GitHub no longer has arrives with an empty login. A rule
    /// naming an author does not match it — the empty string is not
    /// `coderabbitai` — and a rule about what was said goes on matching it.
    #[test]
    fn a_comment_by_nobody_is_matched_on_what_it_says() {
        assert!(!ignored(
            &[rule(Some("coderabbitai"), None)],
            &comment("", "Your billing information is missing."),
        ));
        assert!(ignored(
            &[rule(None, Some("billing"))],
            &comment("", "Your billing information is missing."),
        ));
    }

    /// One rule as the settings page would have written it down.
    fn rule(author: Option<&str>, body: Option<&str>) -> crate::settings::IgnoreRule {
        crate::settings::IgnoreRule::of(author.map(str::to_owned), body.map(str::to_owned))
    }

    /// A batch session is told where to work, for the reason a fix session is:
    /// it starts in the Conversation's own worktree, and `git` and `gh` both read
    /// their repository from wherever they are run.
    #[test]
    fn a_batch_session_is_told_which_pull_request_and_where_to_work() {
        let told = feedback(
            &watched(),
            &[comment("tobico", "Rename the `window` field.")],
        );

        assert!(
            told.contains("#7") && told.contains("askance"),
            "which pull request, in which repository: {told}",
        );
        assert!(
            told.contains("/state/worktrees/rate-limiting-askance"),
            "and the worktree to work in: {told}",
        );
        assert!(
            told.contains("Rename the `window` field."),
            "with what was said under it: {told}",
        );
    }
}
