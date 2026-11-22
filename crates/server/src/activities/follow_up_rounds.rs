//! **Follow-up rounds**: a follow-up's rounds, and an investigation's — the
//! same Activity configured with the investigating skill and evidence — and
//! where each lands the Conversation once the human has nothing else.

use verkstead_render::Resumed;
use verkstead_schema::Nudge;

use crate::AppState;
use crate::activities::built_in;
use crate::drivers::Driving;
use crate::investigations::Investigation;
/// The id the **Follow-up rounds** Activity has in the Activities plugin's
/// manifest, which is also what a Process's state names to run it.
use crate::processes::FOLLOW_UP_ROUNDS;
use crate::runner::{documents, stop};
use crate::skills;
use crate::store;

use super::one_session::asked_for_a_pull_request;

/// Run a follow-up's rounds under the **Follow-up rounds** Activity, and land
/// the Conversation wherever its Outcome says.
///
/// The way in for a steer into Follow-up, for a **Tinker** start, and — through
/// [`again`] — for a follow-up picked up again. The component is the
/// built-in Activities plugin's, and it reaches the server through the host
/// functions alone: it reads what to follow up on off the Timeline, writes the
/// session's prompt, launches it expecting the human's Nothing-else mark, and
/// reads the branches at the end. What it answers is an Outcome, and where that
/// leaves the Conversation is decided here — a Process says it as data from
/// stage 11 on.
///
/// **Nothing is kept for it between starts.** What it is handed beyond the
/// start facts is two readings of the Timeline as it now stands, which a
/// component has no way to make for itself: the rounds already answered inside
/// this follow-up, as the digest a fresh session is primed with — see
/// [`crate::follow_ups`] — and, for a follow-up on no pull request, whether
/// the newest of them carries the Nothing-else mark. Both are empty for a
/// follow-up just opened, and both are what a restarted one derives where it
/// was from.
///
/// - **On a pull request**, either Outcome is the follow-up over: back to the
///   wrap-up it was opened over — see [`back_to_the_wrap_up`], whose `pushed`
///   is read against where the commits stood before the session launched, which
///   is this host's reading and never the component's.
/// - **`committed` with none** is a Tinker that built something, here or in a
///   companion: a `submitting` session is sent for its pull request. Which is
///   also what a Tinker whose ending sent for one and got none comes to when it
///   is picked up again: a marked round with work standing ends `committed` at
///   once rather than starting another round.
/// - **`nothing` with none** is a Tinker that built nothing anywhere: Done.
/// - **`steered`**, or a stop, has said where the Conversation went already,
///   and nothing is done here.
///
/// The registration it is handed is held across the component and whatever its
/// Outcome sets going, as every driver here holds one.
pub(crate) async fn run(state: AppState, conversation_id: i64, driving: Driving) {
    // A server with no skills installed starts no session of any kind, and
    // says so in its log rather than on the Timeline, as every launch here
    // always has: the component would otherwise stop the Conversation over a
    // skill it was never going to be handed.
    if state.sessions.skills().is_none() {
        tracing::error!(
            conversation_id,
            "this server has no skills to send a session into, so no follow-up was started"
        );
        return;
    }

    // Read before the session starts, so what it lands is this follow-up's own:
    // whether the wrap-up's checks go back to waiting turns on whether *this*
    // follow-up pushed anything, and a Conversation on a pull request has a run
    // of commits behind it already.
    let already = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a Conversation had committed failed");
            return;
        }
    };

    let settled = match crate::follow_ups::settled(&state, conversation_id).await {
        Ok(settled) => settled.unwrap_or_default(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the rounds a follow-up has been through failed");
            return;
        }
    };

    let mut options = vec![("settled".to_owned(), settled)];

    // Said only of a follow-up on no pull request, which is the one whose
    // ending can stand owed: one on a pull request always holds commits past
    // its base, so a marked round there would end it at once — and a restart
    // between the mark and the work that round asked for would carry that work
    // back to the wrap-up undone. Picked up again, it runs another session,
    // which finds the mark and ends on it.
    if marked(&state, conversation_id, store::Lifecycle::FollowUp).await
        && owed_ending(&state, conversation_id).await
    {
        options.push(("marked".to_owned(), "true".to_owned()));
    }

    let Some((shipped, activity)) = built_in(FOLLOW_UP_ROUNDS) else {
        tracing::error!(
            conversation_id,
            "this build carries no Follow-up rounds Activity, so no follow-up was started"
        );

        return stop(
            &state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            "following the work up",
            "This build of Verkstead can't run follow-up rounds.",
            None,
        )
        .await;
    };

    let outcome = crate::activities::run(
        &state,
        conversation_id,
        &driving,
        shipped.files(),
        &activity,
        &[],
        options,
        "following the work up",
        crate::continuing::Brought::Settle,
    )
    .await
    .map(|ran| ran.outcome);

    let committed = match outcome.as_deref() {
        Some("committed") => true,
        Some("nothing") => false,
        _ => return,
    };

    over(&state, conversation_id, already, committed, driving).await
}

/// Whether the Conversation's work is on no pull request, which is the only
/// follow-up whose ending can stand owed — see [`run`]. Anything
/// the record will not say reads as *no*, which starts another round.
async fn owed_ending(state: &AppState, conversation_id: i64) -> bool {
    match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => on_a_pull_request(state, &conversation).await == Some(false),
        Ok(None) => false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation a follow-up is on failed");
            false
        }
    }
}

/// Pick a follow-up up again, on a press of Resume or a restart: the
/// **Follow-up rounds** Activity started over, on the Timeline as it now
/// stands — see [`run`].
///
/// **A question the session before left standing is locked first.** A Blocking
/// Ask outlives the session that asked it, and nobody is ever handed somebody
/// else's — so a question left over from the follow-up that died is one the
/// human could answer for ever with nothing reading it, and one this follow-up
/// would never end while it stood. Locked unanswered as the fresh session
/// starts, which is what a relaunched grilling does with its own. Only here: a
/// steer that opens a follow-up displaces what it displaces itself.
///
/// **Nor is this always another round.** A **Tinker** whose rounds are over
/// sits in Follow-up while the `submitting` session sent for its pull request
/// runs, and goes on sitting there where that session left none — `gh` logged
/// out, a push that failed. The component reads that off its start — the newest
/// round marked, work standing on the branches — and ends `committed` at once,
/// which with no pull request sends for one again. Which is the promise
/// [`asked_for_a_pull_request`] makes in its own words: *what they have then is
/// Resume, and a press is another go through here*.
pub(crate) async fn again(state: AppState, conversation_id: i64, driving: Driving) {
    left_open(&state, conversation_id).await;

    run(state, conversation_id, driving).await
}

/// The human has said there is nothing else: end the session, and land the
/// Conversation wherever this follow-up ends.
///
/// **Two endings, and the pull request is what tells them apart.** A follow-up
/// steered into is something taken up about work that is already on one, so where
/// it ends is where it started — see [`back_to_the_wrap_up`]. A **Tinker** starts
/// straight into Follow-up on a branch nobody has opened anything on, and what
/// its rounds committed is the whole of what decides: anything at all means the
/// pull request is sent for and the ordinary wrap-up runs over what is opened,
/// and nothing anywhere means Done.
///
/// **The branches decide, and not the `pushed` flag.** `pushed` is *more
/// commits than when this session launched*, and a follow-up picked up again
/// reads that baseline afresh, so a Tinker that commits, loses its session, is
/// relaunched and then ends on a round that committed nothing reads
/// `pushed: false`. Landing Done on that would leave the work on a branch with
/// no pull request and nothing watching it. So `committed` is what stands past
/// the commits the branches were cut from — the Conversation's own and every
/// read-write companion's — which the **Follow-up rounds** Activity reads and
/// answers as its Outcome; see [`run`]. `pushed` goes on meaning
/// what it has always meant.
///
/// The session is ended first, because the Worktree is about to be handed to
/// whatever comes next: a review or a `submitting` session queueing behind an
/// agent that has nothing left to do would wait for a session Verkstead is
/// finished with.
async fn over(
    state: &AppState,
    conversation_id: i64,
    already: i64,
    committed: bool,
    driving: Driving,
) {
    tracing::info!(
        conversation_id,
        "the human has nothing else, so the follow-up is over and its session is being ended",
    );

    state.sessions.end(conversation_id).await;

    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to land a follow-up on",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation a follow-up was on failed");
            return;
        }
    };

    let Some(on_a_pull_request) = on_a_pull_request(state, &conversation).await else {
        return;
    };

    if on_a_pull_request {
        return back_to_the_wrap_up(state, conversation_id, already, driving).await;
    }

    // Where the Process's manifest says this Outcome goes, for one that is
    // data: a Tinker's `committed` is its wrapping state, so the pull request
    // is sent for, and its `nothing` is Done.
    let outcome = if committed { "committed" } else { "nothing" };
    let leaving = crate::processes::leaving(
        state,
        &conversation.process,
        store::Lifecycle::FollowUp.stored(),
        outcome,
    );
    let committed = match leaving {
        crate::processes::Leaving::AsEver => committed,
        crate::processes::Leaving::ToTheWrapUp => true,
        crate::processes::Leaving::Done => false,
        crate::processes::Leaving::Closed => {
            drop(driving);
            if let Err(error) = crate::conversations::close(state, conversation_id).await {
                tracing::error!(error = ?error, conversation_id, "closing a follow-up its process ends Closed failed");
            }
            return;
        }
        leaving => {
            drop(driving);
            return crate::processes::stopped_by(
                state,
                conversation_id,
                "following the work up",
                "Follow-up rounds",
                outcome,
                &leaving,
            )
            .await;
        }
    };

    if committed {
        tracing::info!(
            conversation_id,
            "the follow-up built something, here or in a companion, and the branch is on no \
             pull request, so a session is being sent to open one",
        );

        // Held across the session and the wrap-up it starts, which is what
        // [`asked_for_a_pull_request`] leaves behind it: dropping first would
        // leave a moment where a sweep could find the Conversation undriven.
        let _driving = driving;

        return asked_for_a_pull_request(state, conversation_id).await;
    }

    finished(state, conversation_id, driving).await
}

/// Land a follow-up back in the wrap-up it was opened over.
///
/// **Where a follow-up ends is where it started.** It is something taken up
/// about work that is already on a pull request, so what is left when it is over
/// is that pull request and a wrap-up to see it out — over whatever the branch
/// now holds, with the review left settled and the fix attempts forgotten, which
/// is exactly what a steer into Wrapping recomputes. *Back to Done* is that
/// wrap-up's own settling rule and nothing decided here.
///
/// **The checks go back to waiting where the follow-up pushed.** `already` is
/// where the Conversation's commits stood before the session started, so
/// standing past it now is a follow-up that gave GitHub a new run to make up its
/// mind about — and a settle standing over it is yesterday's green, which the
/// settling loop could reach Done on before the checks watcher's first poll had
/// looked. A follow-up that was questions and answers alone lands with
/// everything settled and passes straight through to Done. A marker that will
/// not read counts as
/// *pushed*, which is the right way round: the cost is one poll of GitHub, and
/// the cost the other way is a wrap-up finished over a suite nobody watched.
///
/// Which is the one place `pushed` is asked for at all, and the reason it is not
/// what a Tinker's ending reads — see [`over`].
async fn back_to_the_wrap_up(
    state: &AppState,
    conversation_id: i64,
    already: i64,
    driving: Driving,
) {
    let pushed = match store::commits_landed(&state.pool, conversation_id).await {
        Ok(landed) => landed > already,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a follow-up had committed failed");
            true
        }
    };

    match store::follow_up_over(&state.pool, conversation_id, pushed).await {
        Ok(store::Ending::Wrapped) => {}
        Ok(outcome) => {
            tracing::info!(
                conversation_id,
                ?outcome,
                "there was no follow-up left to end, so nothing was moved",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "landing a follow-up back in its wrap-up failed");
            return;
        }
    }

    tracing::info!(
        conversation_id,
        pushed,
        "the follow-up is over, so the Conversation is wrapping up again",
    );

    // The Timeline has a move on it and the card reads differently, and an open
    // page should say so without being reloaded.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    // Held until the wrap-up's watchers have registrations of their own, which
    // is what [`crate::wrapping::watching`] takes as it spawns them: dropping
    // first would leave a moment where a sweep could find the Conversation
    // undriven and stop what has just been started.
    let _driving = driving;

    crate::wrapping::afresh(state.clone(), conversation_id).await;
}

/// And land one in Done, because its rounds built nothing anywhere.
///
/// A Tinker that was questions and answers alone: there is no pull request to
/// carry anything to and nothing to carry — in its own repository or in any
/// companion beside it — so there is nothing for a wrap-up to be about and
/// nothing to dispatch. The move and the line on the Timeline are
/// the whole of it, and the Worktree stays as it is for any Done Conversation.
///
/// Nothing is pushed to the devices and nothing is stamped unseen either, which
/// is what parts this from a wrap-up settling: the human is the one who ended
/// this, a moment ago, by ticking **Nothing else** — see `crate::settling`, whose
/// news is for a milestone nobody was there for.
async fn finished(state: &AppState, conversation_id: i64, driving: Driving) {
    // Held until the move is written, which is what every driver here holds it
    // for: dropping first would leave a moment where a sweep could find the
    // Conversation undriven and stop it over a follow-up that had finished.
    let _driving = driving;

    match store::follow_up_done(&state.pool, conversation_id).await {
        Ok(store::Ending::Finished) => {}
        Ok(outcome) => {
            tracing::info!(
                conversation_id,
                ?outcome,
                "there was no follow-up left to end, so nothing was moved",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "finishing a follow-up that built nothing failed");
            return;
        }
    }

    tracing::info!(
        conversation_id,
        "the follow-up built nothing anywhere and its branch is on no pull request, so the \
         work is done",
    );

    // The Timeline has a move on it and the card reads differently, and an open
    // page should say so without being reloaded.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });
}

/// See out an investigating session: the one session an Investigating has,
/// run under the **Follow-up rounds** Activity configured for it, and the
/// driver the Conversation is held up by for as long as it runs.
///
/// `investigation` is what the session is started on — the question this
/// Investigating was opened with, and the rounds it has already been through
/// where it is being picked up again. See [`crate::investigations`], which is
/// where a press of Resume reads both back from.
///
/// **Follow-up rounds, not an Activity of its own** (ADR-0030, *Activity*):
/// the component started with the skill `verkstead/investigating`, the
/// evidence `investigated` and the prompt written here — the question under
/// *What I want found out* and nowhere else, as a Tinker's follow-up brief
/// is, an investigation building nothing. The component reads no question off
/// the Timeline for it, a follow-up's reading knowing nothing of a steer into
/// Investigating or an Investigate's Brief.
///
/// **A driver rather than an errand beside the work**, exactly as a follow-up's
/// session is: the registration it is handed says the Conversation is being
/// driven for as long as this runs, so nothing sweeps it as standing still
/// while the human is composing an answer on a phone.
///
/// **What ends it is the human saying there is nothing else**: the newest round
/// they answered carries the mark, the session says it is done, and the
/// Conversation lands back where it came from — see [`found_out`]. A signal
/// without the mark is refused so that the next round goes to them as a Set,
/// and the refusal is worded for an investigation rather than for a follow-up;
/// a signal over whatever scratch the Worktree holds is taken, that being what
/// the session was sent to write. That is the evidence's to hold rather than
/// the state's — see [`crate::done::Evidence::Investigated`], which carries
/// both. A session that finishes on a round already marked ends the same way;
/// see [`crate::lending::Lent`]'s reading of a gone session.
///
/// **Either Outcome is the investigation over.** The component reads the
/// branches as it does for a follow-up, `committed` or `nothing`, and an
/// investigation lands where it came from whichever it is, as it always has:
/// nothing it committed is carried anywhere by it.
///
/// **A session that is gone is a stop**, which the component raises in an
/// investigation's words, any question it left the human holding going off
/// with it. And one that will not ask is spoken to, the human told where it
/// will not answer, by the host's wait.
pub(crate) async fn investigating(
    state: AppState,
    conversation_id: i64,
    investigation: Investigation,
    driving: Driving,
) {
    // What the session before this one left standing, where there was one. A
    // Blocking Ask outlives the session that asked it, and nobody is ever
    // handed somebody else's — so a question left over from the session that
    // died is one the human could answer for ever with nothing reading it.
    // Locked unanswered as the fresh session starts, which is what a relaunched
    // grilling and a relaunched follow-up both do with their own.
    if investigation.again {
        left_open(&state, conversation_id).await;
    }

    let Some(documents) = documents(&state, conversation_id).await else {
        return;
    };

    let prompt = skills::investigating(
        documents.skills,
        &investigation.brief,
        &investigation.settled,
    );

    let Some((shipped, activity)) = built_in(FOLLOW_UP_ROUNDS) else {
        tracing::error!(
            conversation_id,
            "this build carries no Follow-up rounds Activity, so no investigation was started"
        );

        return stop(
            &state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            INVESTIGATING,
            "This build of Verkstead can't run an investigation.",
            None,
        )
        .await;
    };

    let options = vec![
        ("skill".to_owned(), "verkstead/investigating".to_owned()),
        (
            "evidence".to_owned(),
            crate::lending::evidence_option(&crate::done::Evidence::Investigated),
        ),
        ("prompt".to_owned(), prompt),
    ];

    let outcome = crate::activities::run(
        &state,
        conversation_id,
        &driving,
        shipped.files(),
        &activity,
        &[],
        options,
        INVESTIGATING,
        crate::continuing::Brought::Settle,
    )
    .await
    .map(|ran| ran.outcome);

    // `driving` is held until whatever this writes is written, and handed to
    // the ending, because what the Conversation lands in may be a state
    // something has to go on driving.
    let Some(outcome) = outcome.filter(|outcome| outcome == "committed" || outcome == "nothing")
    else {
        return;
    };

    // Where the Process's manifest says it goes, for one that is data: an
    // Investigate's every Outcome is Done, which is where its landing is too.
    let process = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.process,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation an investigation was on failed");
            return;
        }
    };

    match crate::processes::leaving(
        &state,
        &process,
        store::Lifecycle::Investigating.stored(),
        &outcome,
    ) {
        crate::processes::Leaving::AsEver | crate::processes::Leaving::Done => {
            found_out(&state, conversation_id, driving).await
        }
        crate::processes::Leaving::Closed => {
            drop(driving);
            state.sessions.end(conversation_id).await;
            if let Err(error) = crate::conversations::close(&state, conversation_id).await {
                tracing::error!(error = ?error, conversation_id, "closing an investigation its process ends Closed failed");
            }
        }
        leaving => {
            drop(driving);
            crate::processes::stopped_by(
                &state,
                conversation_id,
                INVESTIGATING,
                "Follow-up rounds",
                &outcome,
                &leaving,
            )
            .await
        }
    }
}

/// What an investigating session is said to be doing, by the rescue and by a
/// stop over it.
const INVESTIGATING: &str = "finding out what was asked";

/// The human has nothing else they want found out: end the session, and land the
/// Conversation back where the investigation came from.
///
/// **Where it came from, which is one of two things.** An Investigate
/// Conversation — one whose Process is Investigate, which ran Draft to
/// Investigating — ends **Done**: there is nowhere for it to go back to, the
/// investigation being the whole of the work. One *steered* into Investigating
/// goes back to the state the steer found it in, with nothing else about it
/// changed — its settles and its pull request exactly as they were, because
/// nothing here touches either. Which of the two this is, and which state, is
/// [`crate::investigations::landing`]'s to say: it reads a fact somebody recorded
/// beside the Steer Event rather than inferring one from the Timeline's shape.
///
/// A landing that cannot be read is a landing in Done, which is the safe way
/// round: an investigation that ended is over whatever the record will say about
/// it, and Done is the state nothing has to be driving. Reading it *before* the
/// move, so that a store that will not answer is a Conversation left in
/// Investigating with its session still there rather than one moved nowhere in
/// particular.
///
/// **Nothing is dispatched for a landing in Done**, there being nothing to drive:
/// no wrap-up, no watchers, no `submitting` session and no pull request sent for.
/// An investigation writes nothing down on the branch, so there is nothing for a
/// wrap-up to be about and nothing to carry anywhere, and the Worktree stays as it
/// does for any Done Conversation — with whatever scratch the session left in it,
/// which every Set's Diff has already shown.
///
/// **And for any other landing, whatever a pressed Resume would be driving it
/// with.** A Conversation put back into Wrapping or Implementing with nothing
/// driving it is exactly what the stall sweep raises a stop about, so the ending
/// starts the drive rather than leaving it to be found — see
/// [`crate::resume::landed`], which is the press's own recompute and already
/// exhaustive over the states. The registration is held across it and handed on,
/// as every driver here hands one on.
///
/// **Nothing is pushed to the devices and nothing is stamped unseen either**,
/// which is what parts this from a wrap-up settling: the human ended this
/// themselves a moment ago by ticking **Nothing else**. See [`finished`], which is
/// the same ending a Tinker that built nothing gets, for the same reason.
///
/// The session is ended first, as a follow-up's is, so that nothing is left
/// holding the Worktree while the move is written.
async fn found_out(state: &AppState, conversation_id: i64, driving: Driving) {
    let landing = match crate::investigations::landing(&state.pool, conversation_id).await {
        Ok(landing) => landing,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading where an investigation came from failed, so it is ending Done");
            crate::investigations::Landing {
                state: store::Lifecycle::Done,
                steered: None,
            }
        }
    };

    tracing::info!(
        conversation_id,
        ?landing,
        "the human has nothing else they want found out, so the investigation is over and its \
         session is being ended",
    );

    state.sessions.end(conversation_id).await;

    match store::investigation_over(&state.pool, conversation_id, landing.state).await {
        Ok(store::Investigated::Landed) => {}
        Ok(outcome) => {
            tracing::info!(
                conversation_id,
                ?outcome,
                "there was no investigation left to end, so nothing was moved",
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "landing an investigation that is over failed");
            return;
        }
    }

    tracing::info!(
        conversation_id,
        ?landing,
        "the investigation is over, so the Conversation is back where it came from",
    );

    // The Timeline has a move on it and the card reads differently, and an open
    // page should say so without being reloaded.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    // An Investigate Conversation, or one steered in from a Draft or from a close:
    // Done is where the work has got to and nothing is supposed to be driving it.
    // Its scratch stays where the session wrote it, as it does for any Done
    // Conversation — the Worktree goes with the close, and nothing is going to
    // work in there before then.
    if landing.state == store::Lifecycle::Done {
        drop(driving);
        return;
    }

    // Every other landing hands the checkouts back to sessions that commit and
    // push, so what the investigation wrote in them comes out first — after the
    // move, which is what says this investigation is the one that ended, and
    // before the driver below, which is the first thing that would work in there.
    // See [`crate::investigations::tidied`].
    crate::investigations::tidied(state, conversation_id, landing.steered).await;

    // Held across the recompute and handed to whatever it starts, which is what
    // [`crate::resume::landed`] takes it for: dropping first would leave a moment
    // where a sweep could find the Conversation in a driven state with nothing
    // driving it.
    match crate::resume::landed(state, conversation_id, driving).await {
        Ok(Resumed::Resumed) => {}
        // Nothing was started, and the Conversation is left in a state something
        // is supposed to be driving. Said here rather than written down: there is
        // nobody in front of this to answer, and the stall sweep is a minute away
        // with a sentence that names the state — see [`crate::stalls::sweeping`].
        Ok(refusal) => {
            tracing::error!(
                conversation_id,
                ?landing,
                ?refusal,
                "an investigation landed where it came from and nothing could be started to \
                 drive it",
            );
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, ?landing, "starting to drive what an investigation landed in failed");
        }
    }
}

/// Whether the Conversation's own work is on a pull request, or `None` where the
/// record would not say.
///
/// The Conversation's own repository's, which is the pull request that makes a
/// wrap-up: a companion's is something a wrap-up covers rather than something to
/// end into. A steer into Follow-up is refused without one, so this is exactly
/// the question *is this a Tinker's branch* — asked of the record, because a
/// Tinker that has already been round once is on a pull request and ends the
/// ordinary way.
///
/// Asked by the ending — see [`over`] — which lands nothing it cannot be sure
/// about, so `None` there lands nothing at all.
async fn on_a_pull_request(state: &AppState, conversation: &store::Conversation) -> Option<bool> {
    match store::pull_request(&state.pool, conversation.id, conversation.repo.id).await {
        Ok(found) => Some(found.is_some()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = conversation.id, "reading whether a follow-up's work is on a pull request failed");
            None
        }
    }
}

/// Take off any Question Set the follow-up left standing on the Conversation.
///
/// Verkstead reaching for the lock on the human's behalf because it knows
/// something they cannot see — that there is nobody behind the question any
/// more — exactly as a wrap-up closes what its gone session left open. See
/// [`crate::review::closed`].
///
/// The Conversation's rather than the session's, which is what the follow-up's
/// own rule asks everywhere: what matters is whether the human is left holding a
/// question, and a question is one whoever put it up.
///
/// Asked at both ends of a follow-up that lost its session: as the stop over one
/// is raised, and again as a fresh session is started over one nothing stopped —
/// a restart leaves no stop behind, so what a dead session left standing is
/// still standing when the next server picks the follow-up up.
pub(crate) async fn left_open(state: &AppState, conversation_id: i64) {
    let standing = match store::open_set(&state.pool, conversation_id).await {
        Ok(standing) => standing,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a follow-up left open failed");
            return;
        }
    };

    if let Some(set_id) = standing {
        crate::review::closed(state, conversation_id, set_id).await;
    }
}

/// Whether the newest round the human answered inside `within`'s own window
/// carries the Nothing-else mark.
///
/// `within` is the state the rounds are being had in —
/// [`store::Lifecycle::FollowUp`] for a follow-up,
/// [`store::Lifecycle::Investigating`] for an investigation — because each
/// windows its rounds by the newest move into itself and a Conversation can
/// have been through both. See [`store::nothing_else`].
///
/// A store that will not answer reads as *not marked*, which leaves the rounds
/// running: the same way round as [`crate::runner::open`], read from the other
/// side.
pub(crate) async fn marked(
    state: &AppState,
    conversation_id: i64,
    within: store::Lifecycle,
) -> bool {
    match store::nothing_else(&state.pool, conversation_id, within).await {
        Ok(marked) => marked,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, ?within, "reading whether the rounds were over failed");
            false
        }
    }
}
