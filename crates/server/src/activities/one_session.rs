//! **One session**: the inline build, the `submitting` session sent for a
//! pull request, and an `instruction` steer, each run under the built-in
//! Activity with the skill, prompt and evidence it is sent with — and what
//! each makes of the session once it is over.
//!
//! A session run here is ended because it said it was done (ADR-0018): what
//! differs between the three is only what that signal is checked against —
//! a commit past where the run stood for the build and the instruction, and
//! GitHub for the pull request. See [`crate::done::Evidence`].

use crate::AppState;
use crate::activities::built_in;
use crate::drivers::Driving;
use crate::lending::Lent;
use crate::runner::{anything_to_work, backlog_runner, documents, stop, worktree};
use crate::sessions::Ended;
use crate::skills;
use crate::store;

/// Carry work that is built and committed on to the pull request it ends on —
/// and where the session that should have opened one did not, ask for one before
/// anything stops.
///
/// **Every run here ends on a pull request**, and each of them ends on it the
/// same way: the session commits the last of the work and then follows the
/// repository's own finish sequence to push and open one. A backlog's finish
/// step, an inline implementation, a roadmap's own session — three endings, one
/// shape. So they have one failure too: a session that landed its commit and
/// stopped short of the push leaves the work built, committed and unreviewable,
/// and the stop that used to follow said exactly that and left the human to open
/// the pull request themselves.
///
/// **So the missing thing is asked for by a session of its own.** It is the
/// cheapest ask there is — the work is committed, and what is left is a push and
/// a `gh pr create` — and it is the one thing the run cannot go on without. A
/// fresh session rather than the one that stopped short: that one is over by the
/// time this is asked, and a context that already stopped short of the last step
/// is not the one to send back to it. See [`skills::submitting`], which says the
/// work is already built and that opening the pull request is the whole of the
/// job.
///
/// **Only where GitHub says there is no pull request.** A `gh` that is missing,
/// logged out, or looking at a repository with no remote is a wall a session
/// would walk into at exactly the same place, so those stop where they always
/// did — the reasoning [`inline_again`] follows a phase earlier. The answer is
/// handed on to [`crate::wrapping::record`] whole, which makes of it what it
/// always has.
///
/// `writing` is the Timeline Event the session that stopped short printed into,
/// so that a stop written before anything else runs carries the tail of what *it*
/// last said, and `None` where there is no session left to read one off.
pub(crate) async fn to_a_pull_request(
    state: &AppState,
    conversation_id: i64,
    writing: Option<i64>,
) {
    let Some((repo_id, branch, found)) = crate::wrapping::asked(state, conversation_id).await
    else {
        return;
    };

    if !matches!(found, Err(crate::forges::Trouble::NoPullRequest)) {
        return crate::wrapping::record(
            state,
            conversation_id,
            repo_id,
            &branch,
            found,
            writing,
            crate::wrapping::Door::TheMove,
        )
        .await;
    }

    tracing::warn!(
        conversation_id,
        branch,
        "the work is committed and on no pull request, so a session is being sent to open one",
    );

    asked_for_a_pull_request(state, conversation_id).await
}

/// Send one session for the pull request the work should already be on, and make
/// of what it leaves what everything else here makes of it.
///
/// The move the automatic endings and a pressed Resume share, which is why it is
/// a function: a run that stopped short of its push and a human pressing Resume
/// on one are the same Conversation wanting the same thing, and answering them
/// differently would mean the press was worth less than the run. See
/// [`to_a_pull_request`] for the first and [`crate::runner::implementing_again`]
/// for the second — and [`crate::activities::follow_up_rounds`], where it is a
/// **Tinker**'s ending, whose component reads a press on one that got this far
/// as that ending again.
///
/// **Once per go.** What follows the session is GitHub asked again, and what it
/// says then is the whole of it: a pull request wraps the Conversation up, and no
/// pull request stops it in the words it has always stopped in — over the session
/// that was sent to open one, whose last words are where the reason there is
/// still none is written down. Verkstead does not go round a second time by
/// itself, because two agents that both stopped short of the same push is
/// something for the human to look at. What they have then is Resume, and a press
/// is another go through here: the work is still built, so there is still exactly
/// one thing to ask for.
pub(crate) async fn asked_for_a_pull_request(state: &AppState, conversation_id: i64) {
    let Some(writing) = submitted(state, conversation_id).await else {
        return;
    };

    crate::wrapping::opened(state, conversation_id, Some(writing)).await
}

/// Run the one session sent to open a pull request the finish step did not, and
/// wait until it is over.
///
/// **Ended because it said it was done** — see [`crate::done`] — and once it is
/// next idle after that. What this session is sent to do happens on GitHub
/// rather than in the repository, so there is no path to check the signal
/// against and no commit it has to make — a branch that only wanted pushing is
/// one it finishes without writing a line. What holds is what every signal has:
/// nothing left uncommitted, and a Set of its own still open locked as it is
/// taken. Nothing ends it on quiet, so one that waits on a slow push is left to
/// finish it.
///
/// **And one that will not ask is spoken to**, until it signals — see
/// [`crate::rescues`], which tells the human where it will not answer and leaves
/// the session running for them to move.
///
/// The Timeline Event it printed into, or `None` where nothing ran: no session
/// could be started, or the run was stopped from outside while this one did. Both
/// of those have already said whatever there was to say.
async fn submitted(state: &AppState, conversation_id: i64) -> Option<i64> {
    let documents = documents(state, conversation_id).await?;

    // Every caller is a driver holding a registration of its own already; this
    // one is held across the session alone, as the Activity's host asks.
    let driving = state.drivers.driving(conversation_id);

    // No signal given and nothing open, with the session sitting there: spoken
    // to, and the human told where it will not answer — see [`crate::rescues`].
    //
    // Nothing on the branch for the signal to be checked against: what this
    // session does is GitHub's to say, so GitHub is what it is checked against.
    // See [`crate::done::Evidence::Nothing`] and [`crate::done::Ends`].
    let ran = run(
        state,
        conversation_id,
        &driving,
        Sent {
            skill: "verkstead/submitting",
            prompt: skills::submitting(
                documents.skills,
                &documents.brief,
                documents.handoff(),
                None,
            ),
            evidence: crate::done::Evidence::Nothing,
            ends: crate::done::Ends::OnAPullRequest,
            building: None,
            doing: "opening the pull request",
        },
    )
    .await?;

    let (event_id, ended) = ran.seen();

    match ended {
        // Said it is done and gone quiet since. Ended rather than left holding
        // the Worktree, and what it did is GitHub's to say.
        None => {
            tracing::info!(
                conversation_id,
                event_id,
                "the session sent to open the pull request said it is done and has gone \
                 quiet since, so it is being ended",
            );

            state.sessions.end(conversation_id).await;
        }
        // Verkstead ended it — the human closed the Conversation or force-stopped
        // it, or the account it was spending ran out of window. The stop is on the
        // record already, so there is nothing to ask GitHub about. See
        // [`crate::sessions::Ended::on_purpose`].
        Some(ended) if ended.on_purpose() => {
            tracing::info!(
                conversation_id,
                event_id,
                "the session sent to open the pull request was stopped from outside, so \
                 nothing is asked about it",
            );

            return None;
        }
        // And one that ended by itself is read by what it left on GitHub rather
        // than by how it exited: an agent that opened the pull request and then
        // fell over has done the job, and one that exited cleanly having done
        // nothing has not. The ask that follows this is what tells them apart.
        Some(_) => {}
    }

    Some(event_id)
}

/// Launch an inline implementation session and see it out, and carry the
/// Conversation on to wrapping the pull request it opened — or stop the run if
/// it ends having landed nothing.
///
/// The whole of the work in one session, so there is no next step to launch and
/// nothing to poll a Worktree for: what says an inline session did anything is
/// what it committed, which the branch watcher is putting on the Timeline while
/// it runs.
///
/// **Ended because it said it was done**, which is [`instructed`]'s rule on a
/// session of the same shape — see [`crate::done`]. What the signal is checked
/// against is a commit, there being no path to watch and a commit being the one
/// report an agent cannot half make; a signal with nothing committed since the
/// session began is refused. Nothing ends the session on the commit itself: work
/// does not stop at the first one, and a session that commits a slice and then
/// waits on its tests is a session still at work. Once the signal is taken the
/// session is ended when it is next idle, so what it says on the way out is
/// kept. Waiting for the process to exit instead would be waiting for something
/// that never comes: every session here is an interactive agent that idles when
/// its work is done.
///
/// **And one that will not ask is spoken to**, until it signals. Idle with no
/// signal given and nothing put to the human is an inline run come to nothing,
/// or finished and not said, with a process still holding the Worktree — a
/// Conversation nobody can move, driven so nothing sweeps it and silent so
/// nothing says so. Where it will not answer the human is told, and the session
/// left running, as every other driver here does it. See [`crate::rescues`].
///
/// Committing is measured against what was already there rather than against
/// zero, which is what makes a second go answerable: a first attempt that
/// committed twice and then died leaves two commits behind, and a second that
/// commits nothing has committed nothing. What such a second session is sent for
/// is the pull request, so that is what its signal is checked against instead —
/// see [`crate::done`]'s `missing`.
///
/// What follows a session that landed something is the same ending a backlog's
/// finish step has: the session followed the repository's own review process
/// before it signalled, so the branch is pushed and on a pull request by then,
/// and [`to_a_pull_request`] is what finds that pull request and
/// moves the Conversation on — or, where the session stopped short of the push,
/// sends for the one thing missing before anything stops. An inline
/// implementation is work like any other work and goes for review like any other
/// work.
///
/// Which is why landing nothing is not the end of it either. A second session on
/// the branch — the one [`inline_again`] launches where GitHub has no pull
/// request yet — is sent to check work that is already built and carry it to
/// one, and has nothing to commit when it finds nothing left to finish. So a
/// clean session that committed nothing is asked about before it is called an
/// empty one, and the pull request is what tells the two apart.
///
/// The registration it is handed is held until the session is over and whatever
/// it left behind has been read, so an inline run is a driven Conversation for
/// exactly as long as somebody is watching it.
pub(crate) async fn follow_inline(state: AppState, conversation_id: i64, driving: Driving) {
    // Read before the session starts, so it is where the run stood before this
    // session rather than somewhere that includes what it goes on to do.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let Some(documents) = documents(&state, conversation_id).await else {
        return;
    };

    // One that is idle with nothing open and no signal given is the whole of an
    // inline run come to nothing, or finished and not said, with a process
    // still holding the Worktree and nothing on the page to press. Spoken to,
    // and the human told where it will not answer — see [`crate::rescues`].
    //
    // What the session's Done signal is checked against: a commit past where the
    // run stood, and a pull request open on the branch. See [`crate::done`].
    //
    // What an inline session that has done its work looks like from here: it
    // said so, the branch bore that out, and it has been idle since. Nothing on
    // the branch ends it, because work does not stop at the first commit — a
    // session may commit a slice and then wait on its tests. See
    // [`signalled_and_idle`].
    let Some(ran) = run(
        &state,
        conversation_id,
        &driving,
        Sent {
            skill: "verkstead/implementing",
            prompt: skills::implementing(documents.skills, &documents.brief, documents.handoff()),
            evidence: crate::done::Evidence::Committed { already },
            ends: crate::done::Ends::OnAPullRequest,
            building: Some(crate::memos::Building::Inline),
            doing: "implementing the work inline",
        },
    )
    .await
    else {
        return;
    };

    let (event_id, ended) = ran.seen();

    // Said it is done over a commit, and gone quiet since, which is an inline
    // implementation done. The session is ended rather than waited out, and what
    // follows is the ending a landed run has always had: the skill carried the
    // branch to a pull request on its way out, and [`to_a_pull_request`] is what
    // finds it — or sends for it, and then stops naming what it still could not
    // find.
    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            "an inline session said it is done and has gone quiet since, so it is being ended",
        );

        state.sessions.end(conversation_id).await;

        return to_a_pull_request(&state, conversation_id, Some(event_id)).await;
    };

    // Verkstead ended it — the human closed the Conversation or force-stopped
    // it, or the account it was spending ran out of window. Whichever it was,
    // the stop is already on the record and nothing was left to land, so there
    // is nothing to ask about. Answered before the branch is read, since a run
    // stopped from outside has committed nothing and would otherwise read as a
    // session that did nothing. See [`crate::sessions::Ended::on_purpose`].
    if ended.on_purpose() {
        tracing::info!(
            conversation_id,
            event_id,
            "the inline run was stopped from outside, so nothing is asked about it"
        );
        return;
    }

    // Read after the session is over, which is after its relay has waited out the
    // final sweep of the branch: a session's last act is usually a commit, and it
    // lands a poll after the process that made it has gone.
    let landed = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed > already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what an inline session committed failed");
            return;
        }
    };

    let how = match (ended.badly(), landed) {
        // Ended well and committed something, which is an inline implementation
        // done. What becomes of the work from here is the wrap-up phase's, and
        // the session's own Timeline Event goes with it, so that a halt written
        // there carries the tail of what it last said — which is where the
        // reason it opened no pull request is usually written down.
        (None, true) => {
            tracing::info!(
                conversation_id,
                event_id,
                "an inline session has landed its work"
            );

            to_a_pull_request(&state, conversation_id, Some(event_id)).await;
            return;
        }
        // Exited cleanly having committed nothing at all. An interactive agent
        // that decides there is nothing to do exits zero, so this is exactly the
        // case a status could not have caught.
        //
        // But it is also what the *second* session on a branch looks like when
        // it does its job. The skill sends one that finds the work already built
        // to check it over and carry it to the pull request — see
        // [`inline_again`], which is what launches it — and a session that finds
        // nothing left to finish has nothing left to commit either. So GitHub is
        // asked before this is called nothing: a pull request on the branch is
        // that session having done the whole of what it was sent for.
        (None, false) => match crate::wrapping::asked(&state, conversation_id).await {
            // Nothing committed, and GitHub saying there is nothing on the
            // branch either, which is the session that really did do nothing.
            // Said in its own words rather than `gh`'s: what is wrong here is
            // the empty session, and the missing pull request is only how it
            // shows.
            //
            // A branch that could not be asked about at all falls the same way,
            // that being in the log already and no more of an answer than it
            // was.
            //
            // And so does a Repo on no Forge, where there is no pull request to
            // be asked about at all.
            Some((
                _,
                _,
                Err(crate::forges::Trouble::NoPullRequest | crate::forges::Trouble::NoForge(_)),
            ))
            | None => "The session ended without committing anything.".to_owned(),
            // Anything else is what the landed arm above makes of it, made in
            // the one place that knows how: a pull request found is a wrap-up,
            // and a `gh` that cannot answer at all is a halt naming what the
            // human can go and fix.
            Some(_) => {
                tracing::info!(
                    conversation_id,
                    event_id,
                    "an inline session committed nothing, so its pull request is what says \
                     whether it did anything"
                );

                crate::wrapping::opened(&state, conversation_id, Some(event_id)).await;
                return;
            }
        },
        // Ended badly, whether or not it got some of the way: the human is owed
        // the telling either way, and what it committed is on the Timeline above
        // the Notice for them to read.
        (Some(badly), _) => badly,
    };

    stop(
        &state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "implementing the work inline",
        &how,
        Some(event_id),
    )
    .await;
}

/// Run an inline implementation again — or, where the branch is already on a
/// pull request, wrap that up instead.
///
/// The question in front of the session is the one a roadmap's Resume asks a
/// phase earlier: an inline implementation ends on a pull request, so a branch
/// that already has one has nothing left to implement. Two ways it gets there,
/// and neither of them is noticed by anything else — a session that pushed and
/// opened the pull request before it died, and a human who opened one by hand
/// off the halt's own advice. Asked of GitHub rather than of the branch,
/// because a pull request is GitHub's fact and the branch cannot say whether
/// there is one.
///
/// So `gh` is asked first, and what comes back decides between three things:
///
/// - a pull request, and the wrap-up takes it from here without a session being
///   spent on work that is already done;
/// - [`crate::forges::Trouble::NoPullRequest`], which is the ordinary case — the work
///   really is unfinished, so a fresh session builds it exactly as before;
/// - anything else, which is `gh` unable to answer at all, and that halts. A
///   session launched into it could only dead-end on the same missing thing
///   when it came to push, and the halt is what reaches the human on their
///   phone.
pub(crate) async fn inline_again(state: AppState, conversation_id: i64, driving: Driving) {
    // A run the human is still being asked where to start has not started, and
    // its open Set is the wait — a restart reaching it included. See
    // [`crate::placing::inline_waiting`].
    if crate::placing::inline_waiting(&state, conversation_id).await {
        return;
    }

    let Some((_, branch, found)) = crate::wrapping::asked(&state, conversation_id).await else {
        return;
    };

    match found {
        Ok(opened) => {
            tracing::info!(
                conversation_id,
                number = opened.number,
                "the work is already on a pull request, so this wraps it up rather than \
                 building it again"
            );

            // With no Event to read a tail off: there is no session behind this
            // one, the last of them having gone before the human pressed
            // Resume.
            crate::wrapping::opened(&state, conversation_id, None).await
        }
        // On no Forge there is no pull request to say the work is done, so it
        // is built again, the session finding what the branch already holds.
        Err(crate::forges::Trouble::NoPullRequest | crate::forges::Trouble::NoForge(_)) => {
            follow_inline(state, conversation_id, driving).await
        }
        Err(trouble) => {
            tracing::warn!(
                conversation_id,
                branch,
                why = trouble.why(),
                "GitHub cannot be asked whether the work is on a pull request, so nothing \
                 was started again",
            );

            crate::wrapping::stopped(&state, conversation_id, &trouble.why(), None).await
        }
    }
}

/// Do the one thing the human wrote when they steered the Conversation into
/// Implementing, and then carry the pipeline on from what the branch holds.
///
/// **A driver rather than an errand beside the work.** The registration it is
/// handed says the Conversation is being driven for as long as this runs, so
/// nothing sweeps it as standing still; it is judged by the rules every other
/// session here is judged by, so one that ends badly stops the Conversation
/// with the ordinary Notice; and what follows a clean finish is [`onwards`]
/// rather than nothing.
///
/// **Ended because it said it was done**, checked against a commit — see
/// [`crate::done`]. An instruction can ask for anything, so there is no path to
/// watch and no done file to read — what there is, is a commit, which is the
/// one report an agent cannot half make, and a signal with nothing committed
/// since the session began is refused. Work does not always stop at the commit,
/// so nothing ends the session on one; once its signal is taken it is ended when
/// it is next idle.
///
/// **And a session that commits nothing stops the Conversation**, exactly as an
/// inline implementation that commits nothing does. An interactive agent that
/// decides there is nothing to do exits zero, so a clean exit is not by itself a
/// report that anything happened — and the pipeline reads the branch to decide
/// what is next, so a branch nothing was written to is one there is nothing
/// honest to carry on from.
///
/// **Including one that never ends at all.** Idle, with no signal given and
/// nothing put to the human, is the same instruction come to nothing with a
/// process still holding the Worktree — so it is spoken to, and the human told
/// where it will not answer. See [`crate::rescues`].
pub(crate) async fn instructed(
    state: AppState,
    conversation_id: i64,
    instruction: String,
    driving: Driving,
) {
    // Read before the session starts, so it is where the branch stood before the
    // instruction rather than somewhere that includes what it goes on to do.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let Some(documents) = documents(&state, conversation_id).await else {
        return;
    };

    // No signal given and nothing asked, with the session sitting there: an
    // instruction that has come to nothing, or finished and not said, and
    // nobody to say so to. Spoken to, and the human told where it will not
    // answer — see [`crate::rescues`].
    //
    // What the session's Done signal is checked against: a commit past where the
    // branch stood. See [`crate::done`].
    let Some(ran) = run(
        &state,
        conversation_id,
        &driving,
        Sent {
            skill: "verkstead/instruction",
            prompt: skills::instruction(
                documents.skills,
                &documents.brief,
                documents.handoff(),
                &instruction,
            ),
            evidence: crate::done::Evidence::Committed { already },
            ends: crate::done::Ends::WithItsWork,
            building: Some(crate::memos::Building::Instruction),
            doing: "doing what the instruction said",
        },
    )
    .await
    else {
        return;
    };

    let (event_id, ended) = ran.seen();

    let Some(ended) = ended else {
        tracing::info!(
            conversation_id,
            event_id,
            "an instruction session said it is done and has gone quiet since, so it is being \
             ended",
        );

        state.sessions.end(conversation_id).await;

        return onwards(state, conversation_id, event_id, driving).await;
    };

    // The session is over on its own account. It may have committed as its last
    // act and exited before a poll caught it, which is the ordinary shape of a
    // session that finishes rather than idles — so the record is asked once more
    // before this is read as an instruction that came to nothing. Asked
    // whichever way it ended, exactly as a step's landing is: what was committed
    // is committed, and an agent that did the work and then fell over on its way
    // out has left the human nothing to decide about.
    let landed = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed > already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what an instruction session committed failed");
            return;
        }
    };

    if landed {
        return onwards(state, conversation_id, event_id, driving).await;
    }

    // Verkstead ended it — the human closed the Conversation or force-stopped it
    // out from under the instruction, or the account it was spending ran out of
    // window. Each of the three has already written the stop this would
    // otherwise write. See [`crate::sessions::Ended::on_purpose`].
    if ended.on_purpose() {
        tracing::info!(
            conversation_id,
            event_id,
            "the instruction session was stopped from outside, so nothing is asked about it",
        );
        return;
    }

    // How it ended, where the ending itself was the problem; otherwise the
    // nothing it committed is, which is the case no exit status could have
    // shown.
    let how = ended
        .badly()
        .unwrap_or_else(|| "The session ended without committing anything.".to_owned());

    stop(
        &state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        "doing what the instruction said",
        &how,
        Some(event_id),
    )
    .await;
}

/// Carry the pipeline on from whatever the branch holds, once an instruction
/// session has done what it was sent to do.
///
/// The point of an instruction session being a driver at all: the human wrote
/// one thing, and what happens after it is the machine's rather than theirs.
/// What that is, is read off the branch and nowhere else, exactly as every other
/// turn of a run reads it.
///
/// **The backlog first**, because a list with work left in it is an answer to
/// what is next that nothing else can give — and it is the answer even where the
/// branch is already on a pull request, which is the shape a wrap-up that split
/// its findings out into tasks leaves behind. The finish that follows the last
/// task wraps the pull request up again, so nothing is skipped by going this way
/// round.
///
/// **Then the pull request**, asked of `gh` the way the finish step asks — see
/// [`crate::wrapping::opened`], which finds it, moves the Conversation into
/// Wrapping over the record it already had, and starts the wrap-up's watchers
/// afresh.
///
/// Of `gh` rather than of the record, which is the difference between wrapping a
/// Conversation up and stopping it a second time. A record is what Verkstead
/// wrote down; a pull request is GitHub's fact. Where the two disagree it is
/// because the writing down failed — and a Conversation whose ending failed
/// after the push is exactly the Conversation a human steers an instruction into
/// to get it moving. Asking the record would tell it what it already believes.
///
/// **And a stop where the branch holds neither**, because there is nothing left
/// that could be started and a Conversation left implementing with nothing
/// driving it would be one the stall sweep stopped a minute later with a worse
/// sentence. A `gh` that cannot answer stops it too, saying which trouble it
/// was: a session launched into that could only dead-end on the same missing
/// thing. `writing` is the Event the session printed into, so the Notice
/// carries the tail of what it last said.
async fn onwards(state: AppState, conversation_id: i64, writing: i64, driving: Driving) {
    let Some(worktree) = worktree(&state, conversation_id).await else {
        return;
    };

    if anything_to_work(&worktree).await {
        tracing::info!(
            conversation_id,
            "the instruction is done and the backlog holds more, so the run carries on",
        );

        return backlog_runner(state, conversation_id, driving).await;
    }

    // Held until the wrap-up's own watchers have registrations of their own, or
    // until the stop is written: dropping first would leave a moment where a
    // sweep could find the Conversation undriven and stop it all over again.
    let _driving = driving;

    let Some((_repo_id, branch, found)) = crate::wrapping::asked(&state, conversation_id).await
    else {
        return;
    };

    match found {
        Ok(opened) => {
            tracing::info!(
                conversation_id,
                number = opened.number,
                "the instruction is done and the branch is on a pull request, so it is \
                 wrapped up again",
            );

            crate::wrapping::opened(&state, conversation_id, Some(writing)).await;
        }
        Err(crate::forges::Trouble::NoForge(host)) => {
            crate::wrapping::unforged(&state, conversation_id, host.as_deref()).await;
        }
        Err(crate::forges::Trouble::NoPullRequest) => {
            tracing::info!(
                conversation_id,
                "the instruction is done and the branch holds nothing to carry on with",
            );

            stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "carrying the work on from what the instruction left",
                "The branch holds no backlog to work and no pull request to wrap up.",
                Some(writing),
            )
            .await;
        }
        Err(trouble) => {
            tracing::warn!(
                conversation_id,
                branch,
                why = trouble.why(),
                "GitHub cannot be asked what the branch an instruction left is on",
            );

            stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "carrying the work on from what the instruction left",
                &trouble.why(),
                Some(writing),
            )
            .await;
        }
    }
}

/// One session sent under the **One session** Activity: what it is sent to do,
/// in the shape the Activity's options take it.
struct Sent<'a> {
    /// `<pack>/<skill>`, which the Activity's block declares.
    skill: &'a str,
    /// The prompt in the words the [`skills`] builder wrote it in.
    prompt: String,
    evidence: crate::done::Evidence,
    ends: crate::done::Ends,
    /// What its Memos and previews are permitted by, where it builds anything.
    building: Option<crate::memos::Building>,
    /// What the rescue says it is doing.
    doing: &'a str,
}

/// What a session run under the **One session** Activity came to, where its
/// caller has anything left to do about it.
enum Ran {
    /// It gave its Done signal, had it taken, and has been idle since: the
    /// Timeline Event it printed into.
    Done(i64),
    /// It ended without one, and how.
    Refused(i64, Ended),
}

impl Ran {
    /// The Event it printed into, and how it ended — `None` where its Done
    /// signal was taken — which is how a driver read [`Lent::see`] before.
    fn seen(self) -> (i64, Option<Ended>) {
        match self {
            Ran::Done(event_id) => (event_id, None),
            Ran::Refused(event_id, ended) => (event_id, Some(ended)),
        }
    }
}

/// Run one session under the **One session** Activity, the built-in Activities
/// plugin's component, and say what it came to.
///
/// The way the inline build, the `submitting` step and an `instruction` steer
/// launch their session: the component launches it under the skill, expecting
/// the evidence, with the prompt written here, and answers `done` or `refused`
/// — and what either means is still the caller's, which reads how the session
/// ended back off the host.
///
/// `None` where nothing is left for the caller: no session launched — the
/// component stopped the Conversation over the host's refusal, or this server
/// has no skills, which is said in its log as every launch here always has —
/// or the Conversation was stopped or steered elsewhere while it ran.
async fn run(
    state: &AppState,
    conversation_id: i64,
    _driving: &Driving,
    sent: Sent<'_>,
) -> Option<Ran> {
    if state.sessions.skills().is_none() {
        tracing::error!(
            conversation_id,
            skill = sent.skill,
            "this server has no skills to send a session into, so none was started"
        );
        return None;
    }

    let Some((shipped, activity)) = built_in(ONE_SESSION) else {
        tracing::error!(
            conversation_id,
            "this build carries no One session Activity, so no session was started"
        );

        stop(
            state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            sent.doing,
            "This build of Verkstead can't run a session.",
            None,
        )
        .await;

        return None;
    };

    let mut options = vec![
        ("skill".to_owned(), sent.skill.to_owned()),
        ("role".to_owned(), "implementation".to_owned()),
        ("prompt".to_owned(), sent.prompt),
        (
            "evidence".to_owned(),
            crate::lending::evidence_option(&sent.evidence),
        ),
        (
            "ends".to_owned(),
            crate::lending::ends_option(sent.ends).to_owned(),
        ),
    ];

    if let Some(building) = sent.building {
        options.push(("building".to_owned(), building.option()));
    }

    let (outcome, seen) = crate::activities::run_seeing(
        state,
        conversation_id,
        shipped.files(),
        &activity,
        &[],
        options,
        Lent::new(state, conversation_id).doing(sent.doing),
    )
    .await;

    let event_id = seen.event_id?;

    match outcome.as_deref() {
        Some("done") => Some(Ran::Done(event_id)),
        Some("refused") => Some(Ran::Refused(event_id, seen.ended.unwrap_or(Ended::Unknown))),
        _ => None,
    }
}

/// The id the **One session** Activity has in the built-in Activities plugin's
/// manifest.
const ONE_SESSION: &str = "one-session";
