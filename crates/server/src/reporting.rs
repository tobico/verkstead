//! A stage away from home telling home where it stands (ADR-0029, *Home drives
//! a roadmap across devices*).
//!
//! **The far half of the report** — [`crate::peer::reports`] is the half home
//! answers with. A stage moved to this device was born on another one, which
//! keeps its roadmap and a tombstone for it; every lifecycle move made here —
//! into Wrapping, into Done, Closed, a stop and the stop lifting — is told to
//! home so the tombstone reads what this copy reads. And so is the pull request
//! its finish opened, which is what puts it in home's chain.
//!
//! **What is told is the standing as it is, not the move that changed it**, and
//! what home last took is written down — see [`store::told`]. So nothing has to
//! catch each move as it is made: anything that differs from what home last took
//! is owed, and whatever looks sends it. A Conversation's own Nudge looks at
//! once; home's Nudge stream being taken up again, which is home coming back,
//! looks at every stage owed; and a slow clock looks at them all regardless.
//! A move made while home was asleep is never dropped, only late.
//!
//! **And what home started is said here.** Where the report settled the stage,
//! home carries its roadmap on and answers with what it would have said on the
//! tombstone — which nobody opens — so this copy says it on its own Timeline.

use verkstead_render::{BirthKey, Lifecycle as Drawn, StageJoined, StageReport, StageReported};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::peer::reports::REPORTS;
use crate::relaying::{self, Call, Streamed, as_json};
use crate::store;

/// How often every stage away from home is looked at for a report still owed,
/// where nothing else has brought a look about.
pub(crate) const LOOKED_AT_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// The most a report's answer may be: a handful of sentences about stages
/// started, for [`relaying::word_from`]'s bound.
const MOST_AN_ANSWER_IS: usize = 64 * 1024;

/// Start telling home about the stages this device holds away from it, for as
/// long as the server is up.
pub(crate) fn reporting(state: &AppState) {
    if state.devices.is_none() {
        return;
    }

    tokio::spawn(looked(state.clone()));
}

/// The loop: every stage owed looked at once, then again whenever a Nudge or the
/// clock says to.
///
/// **Subscribed before the first round**, so a move made during it is heard.
async fn looked(state: AppState) {
    let mut moved = state.nudges.subscribe();

    // A clock of its own rather than a sleep beside the Nudges, so that a busy
    // device's stream of them cannot hold the retry off for ever. Its first tick
    // is now, which is the first round.
    let mut clock = tokio::time::interval(state.sessions.pace().reports);
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            heard = moved.recv() => match heard {
                // This device's own move on a Conversation, which may be a stage
                // of somebody else's roadmap.
                Ok(heard) => match (heard.device, heard.moved) {
                    (None, Nudge::Conversation { conversation }) => {
                        report(&state, conversation).await;
                    }

                    // A member's stream taken up again, which is a device that
                    // was not answering answering again — home among them.
                    (Some(_), Nudge::Everything) => every_owed(&state).await,

                    _ => {}
                },

                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    every_owed(&state).await;
                }

                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            },

            _ = clock.tick() => every_owed(&state).await,
        }
    }
}

/// Every stage away from home, each told whatever home has not yet taken.
async fn every_owed(state: &AppState) {
    let stages = match store::stages_away_from_home(&state.pool, &state.device).await {
        Ok(stages) => stages,
        Err(why) => {
            tracing::error!(error = ?why, "reading which stages are away from home failed");
            return;
        }
    };

    for stage in stages {
        report(state, stage).await;
    }
}

/// Tell `conversation_id`'s home where it stands, where it is a stage away from
/// home and home has not already taken that.
///
/// Only ever called from [`looked`], one report after another, so no two can
/// overlap and land an older standing on home after a newer one.
async fn report(state: &AppState, conversation_id: i64) {
    let home = match store::away_from_home(&state.pool, conversation_id, &state.device).await {
        Ok(Some(home)) => home,
        Ok(None) => return,
        Err(why) => {
            tracing::error!(error = ?why, conversation_id, "reading whether a Conversation is a stage away from home failed");
            return;
        }
    };

    let standing = match store::live_standing(&state.pool, conversation_id).await {
        Ok(Some(standing)) => standing,
        Ok(None) => return,
        Err(why) => {
            tracing::error!(error = ?why, conversation_id, "reading where a stage away from home stands failed");
            return;
        }
    };

    match store::told(&state.pool, conversation_id).await {
        Ok(Some(told)) if told == standing => return,
        Ok(_) => {}
        Err(why) => {
            tracing::error!(error = ?why, conversation_id, "reading what home was last told failed");
            return;
        }
    }

    let report = StageReport {
        born: BirthKey {
            device: home.device.clone(),
            id: home.id,
        },
        state: drawn(standing.state),
        done: standing.done,
        wrapped: standing.wrapped,
        stopped: standing.stopped,
        joined: standing.joined.as_ref().map(|joined| StageJoined {
            branch: joined.branch.clone(),
            number: joined.number,
            title: joined.title.clone(),
            url: joined.url.clone(),
            base: joined.base.clone(),
        }),
    };

    let answered = relaying::word_from::<StageReported>(
        state.devices.as_ref(),
        &home.device,
        Call {
            method: reqwest::Method::POST,
            onwards: REPORTS.to_owned(),
            headers: as_json(),
            body: Streamed::saying(serde_json::to_vec(&report).unwrap_or_default()),
        },
        MOST_AN_ANSWER_IS,
    )
    .await;

    let reported = match answered {
        Ok(reported) => reported,

        // Still owed, and looked at again when home answers or the clock comes
        // round. In the log alone: the copy is going on with its work, and a
        // home that is asleep is no news on its Timeline.
        Err(refusal) => {
            tracing::info!(
                conversation_id,
                home = %home.device,
                "a stage away from home could not tell home where it stands, and will again: {}",
                refusal.saying,
            );
            return;
        }
    };

    if let Err(why) = store::tell(&state.pool, conversation_id, &standing).await {
        tracing::error!(error = ?why, conversation_id, "recording what home was told failed");
    }

    for said in reported.said {
        crate::continuing::say(state, conversation_id, &said).await;
    }
}

/// The state as it crosses the link, which is the viewer's word for it.
fn drawn(state: store::Lifecycle) -> Drawn {
    crate::ui::lifecycle(state)
}
