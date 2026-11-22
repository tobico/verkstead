//! The **Wrap-up**: the one Activity that watches a Conversation's pull
//! requests through to their landing — checks, comments, the review and
//! mergeability, each a watcher inside the component — and where its Outcome
//! takes the Conversation, until a Process says that as data (stage 11).
//!
//! Started by [`crate::wrapping::watching`] alone, which every start of a
//! wrap-up goes through.

use crate::AppState;
use crate::activities::built_in;
use crate::drivers::Driving;
use crate::lending::Lent;
use crate::runner::{documents, stop};
use crate::skills;
use crate::store;

/// Run a wrap-up under the **Wrap-up** Activity, and land the Conversation
/// wherever its Outcome says.
///
/// The way in for every start of a wrap-up — a pull request recorded, a
/// restart, a Resume, the resolve press — through [`crate::wrapping::wrap_up`],
/// which holds the registration this is handed. The component watches the
/// whole of it — the companions covered, every pull request's checks, merge,
/// comments and landing, and the branch reviewed — and the host keeps the
/// record of what it waits on.
///
/// - **`against`** is the base a **Review** or **Fix Merge Issues** taken up
///   over a bare branch is owed its pull request against: the component is
///   handed the `submitting` session's prompt and opens it itself — see
///   [`crate::conversations::take_up`].
/// - **The Process's narrowing is its options**: Fix Merge Issues turns the
///   review and the comments off, which the component settles at once — see
///   [`crate::conversations::narrows_the_wrap_up`].
/// - **`reviewing`** is a Resume asking for the branch read afresh, whatever
///   the last review left standing — see [`crate::review::wanted_now`].
///
/// And the Outcomes, mapped here until a Process says them as data:
///
/// - **`settled`** is Done, the news, the share and the next stage, as the rule
///   always ended a wrap-up — see [`crate::settling::settled`]. A rule that is
///   no longer true by the time it is applied starts the component again.
/// - **`merged`** is the same Done without the rule: every pull request has
///   landed, and nothing is left for the wrap-up to wait on — so a session it
///   still had running is ended and its Set closed first, see [`landed`].
/// - **`closed`** is a stop: a pull request closed unmerged is the human's to
///   decide about. A session still running is seen out the same way.
/// - **`given-up`** writes the stop the host gave up with, which is exactly the
///   stop it always wrote — see [`crate::wrapping::give_up`].
/// - **`steered`**, or a stop, has said where the Conversation went already.
pub(crate) async fn run(
    state: AppState,
    conversation_id: i64,
    against: Option<String>,
    reviewing: crate::wrapping::Reviewing,
    _driving: Driving,
) {
    let Some((shipped, activity)) = built_in(WRAP_UP) else {
        tracing::error!(
            conversation_id,
            "this build carries no Wrap-up Activity, so nothing is wrapping up"
        );

        stop(
            &state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            "wrapping up",
            "This build of Verkstead can't wrap up.",
            None,
        )
        .await;

        return;
    };

    let process = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.process,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to wrap up failed");
            return;
        }
    };

    let mut options = vec![
        (
            "poll".to_owned(),
            state.sessions.pace().checks.as_millis().to_string(),
        ),
        ("attempts".to_owned(), crate::checks::ATTEMPTS.to_string()),
        // Zero in a server, and a window a test holds open — see
        // [`Pace::reviewing`].
        (
            "review-after".to_owned(),
            state.sessions.pace().reviewing.as_millis().to_string(),
        ),
    ];

    if crate::conversations::narrows_the_wrap_up(&process) {
        options.push(("review".to_owned(), "off".to_owned()));
        options.push(("comments".to_owned(), "off".to_owned()));
    } else if reviewing == crate::wrapping::Reviewing::Afresh {
        options.push(("review".to_owned(), "afresh".to_owned()));
    }

    if let Some(against) = against {
        let Some(documents) = documents(&state, conversation_id).await else {
            return;
        };

        options.push((
            "prompt".to_owned(),
            skills::submitting(
                documents.skills,
                &documents.brief,
                documents.handoff(),
                Some(&against),
            ),
        ));
    }

    let outcome = {
        let _running = state.wrap_ups.running(conversation_id);

        // Its dispatches try for the Turn rather than wait for it, and hold it
        // for as long as their sessions run — see [`Lent::trying`].
        crate::activities::run_seeing(
            &state,
            conversation_id,
            shipped.files(),
            &activity,
            &[],
            options,
            Lent::new(&state, conversation_id)
                .doing("wrapping up")
                .trying(),
        )
        .await
        .0
    };

    // A stop handed over as it ended some other way is still the stop: it is
    // what the host's watchers wrote before the wrap-up was an Activity. Under
    // a stop of the Conversation's own, it is let go.
    let outcome = match outcome {
        Some(_) if state.wrap_ups.given_up(conversation_id) => Some("given-up".to_owned()),
        Some(outcome) => Some(outcome),
        None => {
            state.wrap_ups.let_go(conversation_id);
            None
        }
    };

    match outcome.as_deref() {
        Some("settled") => crate::settling::settled(&state, conversation_id, true).await,
        Some("merged") => {
            landed(&state, conversation_id).await;
            crate::settling::settled(&state, conversation_id, false).await
        }
        Some("closed") => {
            landed(&state, conversation_id).await;
            stop(
                &state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                "wrapping up",
                "A pull request this work is on was closed without being merged. Reopen \
                 it and press **Resume** to carry on wrapping up.",
                None,
            )
            .await
        }
        Some("given-up") => crate::wrapping::given_up(&state, conversation_id).await,
        _ => {}
    }
}

/// See out whatever the wrap-up still had going as its pull requests landed:
/// the session it dispatched, ended, and the Set that session left open,
/// closed unanswered.
///
/// A merge or a close is looked for every round, a session running or not —
/// the human merges from GitHub whenever they like, the review's Set standing
/// or not — and once the wrap-up has answered it nothing is waiting on that
/// session any more. Left running it would hold the Worktree for nobody, and
/// its Set would carry an answer to a session that pushes to a branch already
/// merged or closed. Ended before the move, so the Conversation never reads as
/// Done, or stopped, with a session still working in it.
async fn landed(state: &AppState, conversation_id: i64) {
    state.sessions.end(conversation_id).await;
    crate::activities::follow_up_rounds::left_open(state, conversation_id).await;
}

/// The id the **Wrap-up** Activity has in the built-in Activities plugin's
/// manifest.
const WRAP_UP: &str = "wrap-up";
