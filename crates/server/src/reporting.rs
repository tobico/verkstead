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

use verkstead_render::{
    BirthKey, Lifecycle as Drawn, StageAdopting, StageJoined, StageReport, StageReported,
};
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
    let away = match store::away_from_home(&state.pool, conversation_id, &state.device).await {
        Ok(Some(away)) => away,
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

    // An heir holds no tombstone of a stage until it adopts one, so it is told
    // what the stage is; the device the stage was born on made the stage, and
    // is told nothing it does not hold already.
    let adopting = match away.home != away.born.device {
        true => described(state, conversation_id, &away.home).await.told(),
        false => None,
    };

    let report = StageReport {
        born: BirthKey {
            device: away.born.device.clone(),
            id: away.born.id,
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
        adopting,
    };

    let answered = relaying::word_from::<StageReported>(
        state.devices.as_ref(),
        &away.home,
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
                home = %away.home,
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

/// What `conversation_id` is, for an heir at `home` that has never heard of it
/// (ADR-0029, *A roadmap's home outlives its device*): which roadmap and stage,
/// its branches, its state, the id it goes by here, and which of the heir's
/// Repos it is in.
///
/// Asked by the report and by the join ask alike, each of which adopts the stage
/// on an heir that has not yet — see [`Described`] for what each makes of an
/// answer that is not a description.
pub(crate) async fn described(state: &AppState, conversation_id: i64, home: &str) -> Described {
    let Some(devices) = state.devices.as_ref() else {
        return Described::Untold;
    };

    let read = async {
        let of = store::stage_roadmap(&state.pool, conversation_id).await?;
        let conversation = store::load_conversation(&state.pool, conversation_id).await?;
        let stacks_on = store::stacks_on(&state.pool, conversation_id).await?;

        anyhow::Ok((of, conversation, stacks_on))
    };

    let (of, conversation, stacks_on) = match read.await {
        Ok((Some(of), Some(conversation), stacks_on)) => (of, conversation, stacks_on),
        Ok(_) => return Described::Untold,
        Err(why) => {
            tracing::error!(error = ?why, conversation_id, "reading what a stage is, for its home's heir, failed");
            return Described::Untold;
        }
    };

    let Some(stage) = of.stage else {
        return Described::Untold;
    };

    let there = match crate::matching::across(devices, home, &conversation.repo).await {
        Ok(Some(there)) => there,
        Ok(None) => {
            tracing::warn!(
                conversation_id,
                heir = home,
                repo = conversation.repo.name,
                "the heir of a stage's home holds no copy of its repository, so it cannot adopt it",
            );
            return Described::NoRepo {
                repo: conversation.repo.name,
            };
        }
        Err(refusal) => {
            tracing::info!(
                conversation_id,
                heir = home,
                "the heir of a stage's home could not be asked which of its repos the stage is in: {}",
                refusal.saying,
            );
            return Described::Untold;
        }
    };

    Described::Told(StageAdopting {
        repo: there.id,
        roadmap: of.roadmap,
        stage,
        branch: conversation.branch,
        stacks_on: stacks_on.flatten(),
        live: conversation_id,
        state: Some(drawn(conversation.state)),
    })
}

/// What [`described`] could tell an heir about a stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Described {
    /// Everything the heir makes its tombstone from.
    Told(StageAdopting),

    /// The heir holds no repository this stage's matches — by origin, as every
    /// match across devices is made — so it has nowhere to keep the stage. Its
    /// name here, which is the one the human knows it by.
    ///
    /// Nothing the device asking can mend: a join ask says so on the Timeline,
    /// and a report goes without, to be refused and sent again.
    NoRepo { repo: String },

    /// Something in the way that the log has already said: the heir not
    /// answering, or this device's own record not reading. Asked again on the
    /// next round, and the call goes without it meanwhile — an heir that has
    /// adopted the stage already needs none of it.
    Untold,
}

impl Described {
    /// The description, where there is one.
    pub(crate) fn told(self) -> Option<StageAdopting> {
        match self {
            Described::Told(adopting) => Some(adopting),
            Described::NoRepo { .. } | Described::Untold => None,
        }
    }
}

/// The state as it crosses the link, which is the viewer's word for it.
fn drawn(state: store::Lifecycle) -> Drawn {
    crate::ui::lifecycle(state)
}
