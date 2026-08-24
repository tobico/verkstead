//! The rule that ends a wrap-up.
//!
//! A Conversation leaves Wrapping for **Done** when three things are true
//! together: the pull request's checks are green, the self-review's Question Set
//! has been answered, and nothing said on the pull request is left unaddressed.
//! Any one of the three missing keeps it where it is.
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
//! A loop rather than a call from each of the three, and deliberately so: the
//! things that settle are in three different places — a poll of GitHub, another
//! poll of GitHub, and the endpoint that takes a Response — and a wrap-up left
//! for ever because one of them forgot to ask would be the failure nobody
//! notices. Asking costs one read of a table.

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
        match store::finish_wrap_up(&state.pool, conversation_id).await {
            Ok(store::Finished::StillWaiting) => {}
            Ok(store::Finished::Done) => {
                tracing::info!(
                    conversation_id,
                    "the checks are green, the review is answered and nothing is left \
                     unaddressed, so the work is done",
                );

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
            // Aborted out from under the watchers, or finished by something else
            // — a retried Interruption starts the whole wrap-up again, so two of
            // these can be running at once and the second finds the move made.
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
