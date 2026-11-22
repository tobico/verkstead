//! The **Interview**: where the grilling session is started under the
//! built-in Activity, and where the Direction it ends in takes the
//! Conversation — the backlog run, the inline build or the roadmap's pull
//! request — until a Process says that as data (stage 11).
//!
//! Called by name from [`crate::conversations::start_grilling`], and from
//! [`crate::grillings::again`] for one started again.

use verkstead_schema::Direction;

use crate::AppState;
use crate::activities::built_in;
use crate::drivers::Driving;
use crate::lending::Lent;
/// The id the **Interview** Activity has in the Activities plugin's manifest,
/// which is also what a Process's state names to run it.
use crate::processes::INTERVIEW;
use crate::runner::{base, broken_down, staged};
use crate::skills;

use super::one_session::follow_inline;

/// Run a grilling under the **Interview** Activity, and carry the Conversation
/// on wherever the Direction picked says.
///
/// The component launches the grilling session on `verkstead/grilling` under
/// the Grilling role, with `prompt` — the [`skills`] builder's words, a fresh
/// grilling's or a relaunch's with its digest — and waits on it expecting
/// `picked`: nothing before the human picks, and after it the picked
/// Direction's artifact, which [`crate::runner::tail`] says. The proposals the
/// session puts are the host's to settle, as ever — see
/// [`crate::conversations::settle_a_proposal`]. Breaking down and staging go on
/// inside the same session, so the interview is one session and launches no
/// second.
///
/// The three Directions differ in what the artifact is and in where landing it
/// leaves the Conversation, which is this function's mapping until a Process
/// says it (stage 11). A backlog is the start of a run — the tasks it names are
/// worked one fresh session each, under the Profile that builds — so the
/// Conversation goes on to Implementing. The handoff an inline pick asks for
/// leaves it in the same place, one session rather than a run standing on the
/// other side of it. A roadmap is the whole of this Conversation's own work,
/// because the building belongs to the Stages it plans, so the same session
/// carries the branch to a pull request and the Conversation goes straight on to
/// wrapping that up.
///
/// `lent` is the host the component is lent, carrying what only an in-process
/// caller can say: that nothing else is driving a grilling just started, or the
/// Turn a relaunch took and the Sets it holds open.
///
/// **A grilling is driven by its session until the human picks** — see
/// [`crate::drivers`] — so `driving`, the press's registration, is held across
/// the launch alone, and a session that dies before a pick leaves a
/// Conversation the stall sweep finds. The pick is what takes a registration of
/// the host's own, held through to wherever the Direction leaves the
/// Conversation.
///
/// **It offers the Directions the Conversation's Process declares** (Set 1765,
/// Q27): its manifest's `directions`, which for Develop is all three, each
/// leading to `implementing` — handed to the component as its `offers` start
/// fact, read back by the host for what the chooser draws, and said to the
/// session under its prompt where they are fewer than three. A narrowing the
/// bench declared wins over the manifest, which is how the suite narrows a
/// Process Verkstead still runs itself — see
/// [`crate::followers::Followers::narrowed`].
///
/// Whether a session was launched at all, which is what a relaunch holding Sets
/// open owes them on: see [`crate::grillings::again`]. Answered once the
/// interview is over.
pub(crate) async fn run(
    state: AppState,
    conversation_id: i64,
    driving: Driving,
    prompt: String,
    lent: Lent,
) -> bool {
    let Some((shipped, activity)) = built_in(INTERVIEW) else {
        tracing::error!(
            conversation_id,
            "this build carries no Interview Activity, so no grilling session was started"
        );
        return false;
    };

    let offers = offered(&state, conversation_id).await;
    let words: Vec<&str> = offers.iter().map(|direction| direction.word()).collect();

    let (outcome, seen) = crate::activities::run_seeing(
        &state,
        conversation_id,
        shipped.files(),
        &activity,
        &[],
        vec![
            ("prompt".to_owned(), skills::offering(prompt, &offers)),
            ("offers".to_owned(), words.join(",")),
        ],
        lent.doing("grilling the work").launching(driving),
    )
    .await;

    let Some(writing) = seen.event_id else {
        return false;
    };

    // Taken as the pick was, so there is no moment between the session being
    // seen out and the move where nothing drives the Conversation.
    let driving = seen
        .driving
        .unwrap_or_else(|| state.drivers.driving(conversation_id));

    // The Outcome, or — where a stop landed as the artifact did, and took the
    // Outcome away — the Direction whose artifact was written: a step that
    // landed is landed, so it is taken, and the launch past it is what asks
    // after the stop.
    let picked = match outcome.as_deref() {
        Some(outcome) => outcome.strip_prefix("direction:"),
        None => seen.picked.as_deref(),
    };

    let direction = match picked {
        Some("inline") => Direction::Inline,
        Some("task-list") => Direction::TaskList,
        Some("roadmap") => Direction::Roadmap,
        // Steered elsewhere, which is the steer's to carry on; or gone before
        // anything was picked, which leaves the Conversation to the stall sweep
        // as a grilling session dying always has; or stopped.
        _ => return true,
    };

    tracing::info!(
        conversation_id,
        event_id = writing,
        ?direction,
        "a grilling session wrote what the pick asked for and has gone quiet since, so it is being ended",
    );

    state.sessions.end(conversation_id).await;

    match direction {
        Direction::Inline => handed_off(state, conversation_id, driving).await,
        Direction::TaskList => broken_down(state, conversation_id, driving).await,
        Direction::Roadmap => {
            if let Some(base) = base(&state, conversation_id).await {
                staged(&state, conversation_id, &base, writing).await;
            }
        }
    }

    true
}

/// The Directions `conversation_id`'s interview offers: what the bench
/// narrowed them to where it narrowed them, and otherwise what the
/// Conversation's Process declares — see [`crate::processes::directions`]. A
/// record that will not be read offers all three, which is what every
/// interview offered before a Process said otherwise.
async fn offered(state: &AppState, conversation_id: i64) -> Vec<Direction> {
    if let Some(narrowed) = state.followers.narrowed(conversation_id) {
        return narrowed;
    }

    match crate::store::process(&state.pool, conversation_id).await {
        Ok(process) => crate::processes::directions(state, &process),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which Directions a Conversation's Process offers failed");
            Direction::ALL.to_vec()
        }
    }
}

/// Start the session that builds from the handoff the grilling session wrote.
///
/// The inline counterpart to [`broken_down`], and the same move with a
/// different artifact: the session that settled the work is the one that wrote
/// down what it settled, after the pick rather than before it, so what the human
/// said beside the pick is part of what the handoff says.
///
/// Then the three things the far side of an inline pick needs, in the order they
/// have to happen in. The handoff goes on the Timeline, which is where a
/// Conversation's documents live and the one moment this one is certainly
/// finished. The Conversation moves, so that what says it is being built has
/// everything the grilling left beside it. And the work starts, in a fresh
/// session under the implementation Profile — fresh because the two run as
/// accounts the Conversation fixed separately and a session cannot change the
/// one it is running as, which is the whole reason the handoff exists.
///
/// The registration is held across all of it — the move, and the run on the
/// other side of it — so there is no moment between the handoff landing and the
/// implementation session starting where the Conversation reads as one nothing
/// is driving.
async fn handed_off(state: AppState, conversation_id: i64, driving: Driving) {
    crate::conversations::hand_over(&state, conversation_id).await;
    crate::conversations::grilling_over(&state, conversation_id).await;

    // And an inline run whose proposal wants one platform this device is not is
    // moved to one that is before its session starts, the way a declared task
    // is: the handoff is on the record by now, so it crosses with it, and the
    // far end's arrival presses its own Resume, which launches the run there
    // without asking again — see [`crate::placing::inline`]. Where the human is
    // asked where it runs instead, their answer launches it.
    if !crate::placing::inline(&state, conversation_id).await.here() {
        return;
    }

    follow_inline(state, conversation_id, driving).await
}
