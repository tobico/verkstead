//! The rule that ends a wrap-up, and the condition it passes through on the way.
//!
//! A Conversation leaves Wrapping for **Done** when three kinds of thing are
//! true together: every pull request's checks are green, the self-review's
//! Question Set has been answered, and nothing said on any of the pull requests
//! is left unaddressed. Any one of them missing keeps it where it is.
//!
//! Three kinds rather than three things, because a Conversation ends on a pull
//! request per repository it was worked in and each of them has a suite and a
//! conversation of its own: the review is one review across the whole of it, and
//! the checks and the comments are one settlement each per pull request. Which
//! pull requests those are is read off the record every time the rule is asked —
//! a companion's found a poll after the Conversation's own is two more things to
//! wait on, and a wrap-up that had already counted its three would have finished
//! in between. See [`store::finish_wrap_up`].
//!
//! Verkstead decides that itself. There is nobody at the workbench to press
//! anything, which is the whole of what running unattended means — and each of
//! the three is already a fact Verkstead knows rather than an opinion somebody
//! would have to form.
//!
//! What it does **not** wait for is the merge. Stages stack on unmerged
//! predecessors, so a Conversation that stayed in Wrapping until its pull request
//! landed would hold up every stage behind it — and merging is the human act this
//! pipeline is built around rather than a step in it. Done means Verkstead has
//! finished with the work, not that it is on `main`.
//!
//! **And never over a stop.** Every other stop a wrap-up can take leaves
//! something unsettled behind it — red checks, a review nobody finished, a batch
//! nobody answered — so the rule never had to ask whether the run was stopped.
//! A companion whose pull request was never found leaves nothing unsettled,
//! because nothing was recorded to be unsettled about: the pull requests that
//! were found could all go green and the Conversation would sail to Done past
//! its own Notice. So this asks too, the way every watcher already asks before
//! it dispatches anything — see [`crate::stopping::stopped`].
//!
//! A loop rather than a call from each of the watchers, and deliberately so: the
//! things that settle are in three different places — a poll of GitHub, another
//! poll of GitHub, and the endpoint that takes a Response — and a wrap-up left
//! for ever because one of them forgot to ask would be the failure nobody
//! notices. Asking costs a few reads of a table.
//!
//! Which is also why the condition on the way is noticed here — see
//! [`narrowing`]. A wrap-up whose review and comments have settled and whose
//! checks have not, with nothing running in its Worktree, is **Waiting on
//! checks**: a label on the card and a line on the Timeline, drawn off the same
//! facts this loop already reads on a cadence, and nothing on the Lifecycle.

use verkstead_schema::Nudge;

use crate::AppState;
use crate::store;

/// Ask whether `conversation_id`'s wrap-up is over, until it is or there is
/// nothing left to ask about.
///
/// Nothing here is refused for. This runs unattended with nobody watching, and
/// what it has to say it says on the Timeline or in the log.
pub(crate) async fn watch(state: AppState, conversation_id: i64) {
    loop {
        // Before the rule rather than inside it, because what a stop means here
        // is the same as what it means in front of a launch: the run does not
        // advance past one, and Done is as much an advance as a session is.
        // Started again by the press that clears it — see [`crate::resume`],
        // which starts the whole of a wrap-up over.
        //
        // And before the narrowing below for the same reason: a run that has
        // stopped is not a wrap-up waiting on its checks, so it says nothing.
        if crate::stopping::stopped(&state, conversation_id).await {
            tracing::info!(
                conversation_id,
                "driving has stopped, so the wrap-up is not being finished",
            );
            return;
        }

        // Before the rule itself, because the two are readings of the same
        // facts a moment apart and this is the one that has something to say
        // about a wrap-up that is *not* over: the narrowing is what is left
        // when everything but the checks has settled.
        narrowing(&state, conversation_id).await;

        match store::finish_wrap_up(&state.pool, conversation_id).await {
            Ok(store::Finished::StillWaiting) => {}
            Ok(store::Finished::Done) => {
                tracing::info!(
                    conversation_id,
                    "every pull request's checks are green and nothing said on any of \
                     them is left unaddressed, and the review is answered, so the work \
                     is done",
                );

                // The sidebar keeps the news until the human has looked at it,
                // which is what a push nobody was there for needs behind it: a
                // notification read on a phone and swiped away is a milestone
                // the laptop would otherwise never mention. Stamped here rather
                // than wherever Done is reached, because it is this push it
                // marks the trail of — a steer to Done is the human's own act,
                // pushes nothing and stamps nothing.
                //
                // Before the Nudge, so that the sidebar the Nudge sends every
                // open page back to read is one this has already written to.
                if let Err(error) = store::stamp_unseen(&state.pool, conversation_id).await {
                    tracing::error!(
                        error = ?error,
                        conversation_id,
                        "stamping a finished Conversation unseen failed",
                    );
                }

                // The Timeline has a move on it, and an open page should say so
                // without being reloaded.
                state.nudges.announce(Nudge::Conversation {
                    conversation: conversation_id,
                });

                // And the devices are told: nobody pressed anything to get here
                // and nobody was watching it happen, which is exactly what a
                // milestone notification is for. Behind the move, which the
                // store has already made.
                crate::push::told(&state.pool, conversation_id, crate::push::News::Done);

                // And a settled wrap-up is what lets the next roadmap stage
                // start, which is the whole of what makes a staged roadmap
                // execute itself — see [`crate::continuing`]. Asked of every
                // Conversation rather than of the ones somebody thought were
                // stages: whether this is a stage of anything is read off the
                // branch, and one that has written to no roadmap starts nothing.
                //
                // Here rather than anywhere else because this is the one place
                // that knows a wrap-up has just ended, and awaited rather than
                // spawned: this loop has nothing left to do after it, and the
                // work it is waiting on is a git read and a session starting.
                crate::continuing::carry_on(state, conversation_id).await;
                return;
            }
            // Closed out from under the watchers, or finished by something else
            // — Resume starts the whole wrap-up watching again, so two of these
            // can be running at once and the second finds the move made.
            Ok(store::Finished::NotWrapping) => {
                tracing::debug!(
                    conversation_id,
                    "the Conversation is not wrapping up any more, so nothing is left to settle",
                );
                return;
            }
            Ok(store::Finished::NoSuchConversation) => {
                tracing::error!(conversation_id, "there is no Conversation left to settle");
                return;
            }
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "asking whether a wrap-up was over failed");
            }
        }

        tokio::time::sleep(state.sessions.pace().checks).await;
    }
}

/// Notice a wrap-up that has narrowed to its checks, and say so once.
///
/// **Waiting on checks** is a condition of Wrapping rather than a state: the
/// review answered, the comments dealt with, the checks alone outstanding and
/// nothing running in the Worktree. Nothing is stored about it beyond the mark
/// that says the Notice has been written — see [`store::narrowing`] — and the
/// Lifecycle is untouched. It is read off the settle facts and the sessions
/// register the same way *blocked on you* is read off a stop.
///
/// Here rather than anywhere else because this loop already reads those facts
/// on a cadence, and the narrowing is exactly what it finds when the answer to
/// *is this over* is nearly yes.
///
/// One Notice per narrowing, which is the whole of what the mark is for: leaving
/// the condition takes the mark with it, so a fix session dispatched or a
/// comment landing and the wrap-up quietening again writes a fresh line rather
/// than a duplicate of the first or nothing at all.
///
/// **No device push.** There is nothing for the human to do about it — the
/// checks are GitHub's to finish — so it is a line on the Timeline and a label
/// on the card, and neither is worth a phone lighting up.
async fn narrowing(state: &AppState, conversation_id: i64) {
    // The register rather than the record: what says a wrap-up is waiting is
    // that nobody is in it, and a fix session working a red check is a wrap-up
    // getting on with it.
    let working = state.sessions.working().contains(&conversation_id);

    match store::narrowing(&state.pool, conversation_id, working).await {
        Ok(store::Narrowing::Narrowed) => {
            tracing::info!(
                conversation_id,
                "the review is answered and nothing is left unaddressed, so the wrap-up is \
                 waiting on its checks",
            );

            if let Err(error) = store::note(
                &state.pool,
                conversation_id,
                "**Waiting on checks.** The review is answered and nothing said on the pull \
                 request is left unaddressed, so the checks going green is the whole of what \
                 this wrap-up is still waiting on.",
            )
            .await
            {
                tracing::error!(error = ?error, conversation_id, "saying that a wrap-up was down to its checks failed");

                // And the mark comes off, so the next poll is told to write it
                // again: one standing over a line that never landed would be a
                // narrowing said nowhere at all.
                if let Err(error) = store::forget_narrowing(&state.pool, conversation_id).await {
                    tracing::error!(error = ?error, conversation_id, "taking back the mark on a line that was never written failed");
                }

                return;
            }

            // The Timeline has a line on it and the card a label, and an open
            // page should say so without being reloaded. The one kind carries
            // both — a Conversation that moved is a sidebar row that reads
            // differently, which is exactly what this is.
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });
        }
        // Said already, and still true: the label goes on standing and there is
        // nothing to write.
        Ok(store::Narrowing::NoticedAlready) => {}
        // And not narrowed — which includes every state that is not Wrapping,
        // so a Conversation steered away leaves no mark behind for the round
        // after it to be quiet on.
        Ok(store::Narrowing::NotNarrowed) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking whether a wrap-up was down to its checks failed");
        }
    }
}
