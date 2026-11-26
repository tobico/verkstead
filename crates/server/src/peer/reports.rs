//! A stage's report, as its home answers it: a member holding the live copy of
//! a stage born here, saying where that stage now stands (ADR-0029, *Home
//! drives a roadmap across devices*).
//!
//! **Home keeps the roadmap**, home being the device the stage was born on or,
//! once that device has left the cluster, its **Heir** — which adopts a stage it
//! has never heard of the first time its live copy reports (see [`adopted`]).
//! Every stage is made on the device driving its
//! roadmap, and a stage moved away leaves a tombstone here that the standings,
//! the places and the chain go on reading. So what arrives is written onto that
//! tombstone — see [`store::follow`] — and the roadmap is read where it always
//! was. Where the report moved the tombstone into Done, the roadmap is carried
//! on from it exactly as a settle here would, and what that would have said on
//! the tombstone's Timeline is the answer: nobody opens a tombstone, so the copy
//! says it on its own — see [`crate::continuing::reported`].
//!
//! **Only the device the work is on may report.** The tombstone names it, and
//! the certificate the call was made under is what says which device is
//! calling — see [`super::Members::presenting`]. A report about work this
//! device holds no tombstone for is refused, and so is one whose tombstone names
//! some other machine — unless the marks lead from that machine to the caller: a
//! stage moved on from the device it was first moved to leaves this tombstone
//! naming that first device, so home asks it where its copy went, and follows
//! the answer, and re-points its own mark where the trail ends at the caller —
//! see [`followed_to`]. A device that has handed the work on has nothing left to
//! say about it, and is never where the trail ends.
//!
//! **A member's own call, so it stands inside [`super::gate`]**, beside the
//! news, which is its nearest precedent: a member saying something about its
//! own work.

use axum::Json;
use axum::Router;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use verkstead_render::{
    BirthKey, HandedOn, StageAdopting, StageReport, StageReported, StageWhereabouts,
};
use verkstead_schema::Nudge;

use crate::relaying::{self, Call, Streamed, as_json};
use crate::{AppState, store};

use super::Caller;

/// Where the device holding a stage away from home says where it stands.
///
/// Naming no device and no Conversation, for the news's reason: the caller is
/// read off the handshake, and the stage is named by its Birth Key in the body,
/// which is the one name both ends share for it.
pub const REPORTS: &str = "/api/peer/v1/reports";

/// Where home asks a member where its copy of a stage went — see
/// [`followed_to`], which asks, and [`whereabouts`], which answers.
///
/// Naming the stage in the body for the reports' reason: its Birth Key is the
/// one name both ends share for it.
pub const WHEREABOUTS: &str = "/api/peer/v1/whereabouts";

/// How many devices home follows a stage's marks through before giving up on a
/// caller: more moves than any piece of work makes, and a bound on a trail that
/// somehow led round in a circle.
const MOST_HOPS: usize = 8;

/// The most a device's answer about where its copy went may be: one Device Id
/// and one id.
const MOST_AN_ANSWER_IS: usize = 4 * 1024;

/// `POST /api/peer/v1/reports` — write a stage's standing onto its tombstone,
/// and answer with what carrying the roadmap on said.
pub(crate) async fn heard(
    State(state): State<AppState>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    Json(report): Json<StageReport>,
) -> Response {
    let adopting = report
        .adopting
        .as_ref()
        .map(|adopting| (adopting, crate::ui::state_of(report.state)));

    let tombstone = match tombstone_of(&state, &caller, &report.born, adopting).await {
        Ok(tombstone) => tombstone,
        Err(refusal) => return refusal,
    };

    let standing = store::LiveStanding {
        state: crate::ui::state_of(report.state),
        done: report.done,
        wrapped: report.wrapped,
        stopped: report.stopped,
        joined: report.joined.map(|joined| store::JoinedAway {
            branch: joined.branch,
            number: joined.number,
            title: joined.title,
            url: joined.url,
            base: joined.base,
        }),
    };

    let followed = match store::follow(&state.pool, tombstone, &standing).await {
        Ok(followed) => followed,

        Err(why) => {
            tracing::error!(error = ?why, tombstone, "writing a stage's standing onto its tombstone failed");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not write the stage's standing down",
            );
        }
    };

    tracing::info!(
        tombstone,
        state = ?standing.state,
        settled = followed.settled,
        "a stage away from home reported where it stands, and its tombstone follows it",
    );

    // What reads the roadmap here has moved: the card, and the list.
    state.nudges.announce(Nudge::Conversation {
        conversation: tombstone,
    });

    let said = match followed.settled {
        true => crate::continuing::reported(state.clone(), tombstone).await,
        false => Vec::new(),
    };

    Json(StageReported { said }).into_response()
}

/// The tombstone a member is calling about, where it is one this device keeps
/// and the work on it is on that member — or the refusal to answer with.
///
/// Shared with [`super::stage_joins`], which is the same member asking about the
/// same stage: only the device the work is on has anything to say or ask.
pub(super) async fn tombstone_of(
    state: &AppState,
    caller: &Caller,
    born: &BirthKey,
    adopting: Option<(&StageAdopting, store::Lifecycle)>,
) -> Result<i64, Response> {
    let Some(presented) = caller.fingerprint() else {
        return Err(refused(
            StatusCode::FORBIDDEN,
            "a device that presented no certificate has no stage to report on",
        ));
    };

    let Some(devices) = state.devices.as_ref() else {
        tracing::error!("a call about a stage arrived at a device that does not know what it is");

        return Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "this device cannot say which of its members is calling",
        ));
    };

    let member = match devices.membership().presenting(&presented).await {
        Ok(Some(member)) => member,

        // Through the gate and unlinked a moment later: nobody this device
        // would take a report from.
        Ok(None) => {
            return Err(refused(
                StatusCode::FORBIDDEN,
                "this device holds no membership for the caller",
            ));
        }

        Err(why) => {
            tracing::error!(%why, "the membership a call about a stage is attributed to could not be read");

            return Err(refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read the membership",
            ));
        }
    };

    // Home is the birth device, or the heir it left what it kept to.
    let home = match store::homes(&state.pool).await {
        Ok(homes) => homes.of(&born.device),
        Err(why) => {
            tracing::error!(error = ?why, "reading the heirs failed");

            return Err(refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own record",
            ));
        }
    };

    // Each refusal below is said in the log as well as to the caller: the
    // caller's Timeline has the status alone, and this end is where the reason
    // is.
    if home != state.device {
        tracing::warn!(
            caller = %member.device,
            born = %born.device,
            born_as = born.id,
            home,
            "a member called about a stage whose home is not this device, and was refused",
        );

        return Err(refused(
            StatusCode::NOT_FOUND,
            "that stage was not born on this device, so this is not its home",
        ));
    }

    let born = store::Birth {
        device: born.device.clone(),
        id: born.id,
    };

    let tombstone = match store::born_as(&state.pool, &born).await {
        Ok(Some(id)) => id,

        // A stage of a device this one is the heir of, never heard of here
        // until now: the tombstone the device it was born on would have held
        // is made, naming the caller as where the work is.
        Ok(None) if born.device != state.device && adopting.is_some() => {
            let (adopting, state_now) = adopting.expect("matched as there");

            adopted(state, &born, &member.device, adopting, state_now).await?
        }

        Ok(None) => {
            tracing::warn!(
                caller = %member.device,
                born = born.key(),
                adopting = adopting.is_some(),
                "a member called about a stage this device holds no copy of, and said nothing \
                 to adopt it from, so it was refused",
            );

            return Err(refused(
                StatusCode::NOT_FOUND,
                "this device holds no copy of that stage",
            ));
        }

        Err(why) => {
            tracing::error!(error = ?why, "looking for the stage a call is about failed");

            return Err(refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own record",
            ));
        }
    };

    match store::transferred(&state.pool, tombstone).await {
        Ok(Some(mark)) if mark.device == member.device => {}

        // A tombstone naming another machine, which may be the one the work was
        // first moved to and has moved on from since: followed, and re-pointed
        // at the caller where that is where the work went.
        Ok(Some(mark)) => {
            let Some(live) = followed_to(state, &born, mark, &member.device).await else {
                tracing::warn!(
                    tombstone,
                    caller = %member.device,
                    "a member called about a stage whose live record is not on it, and was refused",
                );

                return Err(refused(
                    StatusCode::CONFLICT,
                    "the work on that stage is not on the device calling",
                ));
            };

            if let Err(why) = store::transfer_away(&state.pool, tombstone, &live).await {
                tracing::error!(error = ?why, tombstone, "pointing a tombstone at where its work went failed");

                return Err(refused(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "this device could not write down where the work is",
                ));
            }

            tracing::info!(
                tombstone,
                live_on = %live.device,
                live_as = live.id,
                "a stage moved on from the device it was first moved to, and its tombstone follows it there",
            );
        }

        Ok(None) => {
            tracing::warn!(
                tombstone,
                caller = %member.device,
                "a member called about a stage whose live record is here, and was refused",
            );

            return Err(refused(
                StatusCode::CONFLICT,
                "the work on that stage is not on the device calling",
            ));
        }

        Err(why) => {
            tracing::error!(error = ?why, tombstone, "reading where a stage's live record is failed");

            return Err(refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own record",
            ));
        }
    }

    Ok(tombstone)
}

/// Make the tombstone of stage `born`, live on `caller`, on this device as the
/// heir of the device it was born on (ADR-0029, *A roadmap's home outlives its
/// device*) — see [`store::adopt`].
///
/// The mark names the caller and the id it gave in the report, so the check
/// that follows finds the work on the device calling, as it would for any
/// tombstone. A Repo that is not registered here is refused: the caller matched
/// it against this registry a moment ago, and it went in between.
async fn adopted(
    state: &AppState,
    born: &store::Birth,
    caller: &str,
    adopting: &StageAdopting,
    state_now: store::Lifecycle,
) -> Result<i64, Response> {
    let live = store::Transferred {
        device: caller.to_owned(),
        id: adopting.live,
    };

    let facts = store::Adopting {
        repo_id: adopting.repo,
        roadmap: adopting.roadmap.clone(),
        stage: adopting.stage.clone(),
        branch: adopting.branch.clone(),
        stacks_on: adopting.stacks_on.clone(),
        state: state_now,
    };

    match store::adopt(&state.pool, born, &live, &facts).await {
        Ok(Some(tombstone)) => {
            tracing::info!(
                tombstone,
                born_on = %born.device,
                roadmap = facts.roadmap,
                stage = facts.stage,
                live_on = caller,
                "a stage whose home left the cluster was adopted by its heir",
            );

            Ok(tombstone)
        }

        Ok(None) => {
            tracing::warn!(
                born = born.key(),
                repo = facts.repo_id,
                caller,
                "a stage whose home left the cluster names a repo this device has not registered, \
                 so it was not adopted",
            );

            Err(refused(
                StatusCode::NOT_FOUND,
                "this device has no repo registered for that stage",
            ))
        }

        Err(why) => {
            tracing::error!(error = ?why, born = born.key(), "adopting a stage failed");

            Err(refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not write the stage down",
            ))
        }
    }
}

/// Where the work on the stage `born` went from the device `mark` names, where
/// the marks lead from there to `caller` — the mark to put on home's tombstone
/// instead — and `None` where they do not.
///
/// Each device asked says where its own copy was handed on to, and the next is
/// asked in turn. A device that cannot be reached, holds no copy, or holds the
/// live one itself ends the trail short of the caller, and the call is refused
/// as it always was: asked again, as a report or a join is, it is followed
/// again.
async fn followed_to(
    state: &AppState,
    born: &store::Birth,
    mark: store::Transferred,
    caller: &str,
) -> Option<store::Transferred> {
    let asking = StageWhereabouts {
        born: BirthKey {
            device: born.device.clone(),
            id: born.id,
        },
    };

    let mut at = mark;

    for _ in 0..MOST_HOPS {
        // Never this device: home's own copy is the tombstone the trail
        // started from, and a mark naming it is a trail gone round in a circle.
        if at.device == state.device {
            return None;
        }

        let answered = relaying::word_from::<Option<HandedOn>>(
            state.devices.as_ref(),
            &at.device,
            Call {
                method: reqwest::Method::POST,
                onwards: WHEREABOUTS.to_owned(),
                headers: as_json(),
                body: Streamed::saying(serde_json::to_vec(&asking).unwrap_or_default()),
            },
            MOST_AN_ANSWER_IS,
        )
        .await;

        let next = match answered {
            Ok(Some(next)) => next,

            Ok(None) => return None,

            Err(refusal) => {
                tracing::info!(
                    asked = %at.device,
                    "a device home's tombstone names could not say where the work went: {}",
                    refusal.saying,
                );
                return None;
            }
        };

        at = store::Transferred {
            device: next.device,
            id: next.id,
        };

        if at.device == caller {
            return Some(at);
        }
    }

    None
}

/// `POST /api/peer/v1/whereabouts` — where this device's copy of a stage was
/// handed on to: its tombstone's mark, or `null` where it holds no copy or the
/// live one.
///
/// Home's question, asked as it follows a stage that moved on — see
/// [`followed_to`]. Read straight off the mark, which is the one thing this
/// device knows about where the work went and the thing its own URL redirects
/// by.
pub(crate) async fn whereabouts(
    State(state): State<AppState>,
    Json(asking): Json<StageWhereabouts>,
) -> Response {
    let born = store::Birth {
        device: asking.born.device,
        id: asking.born.id,
    };

    let held = match store::born_as(&state.pool, &born).await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(error = ?why, "looking for this device's copy of a stage failed");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own record",
            );
        }
    };

    let Some(copy) = held else {
        return Json(None::<HandedOn>).into_response();
    };

    match store::transferred(&state.pool, copy).await {
        Ok(mark) => Json(mark.map(|mark| HandedOn {
            device: mark.device,
            id: mark.id,
        }))
        .into_response(),

        Err(why) => {
            tracing::error!(error = ?why, copy, "reading where this device's copy of a stage went failed");

            refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own record",
            )
        }
    }
}

/// The routes, over the state the workbench answers out of — mounted by
/// [`super::workbench::served`], for the news's reason: the roadmap this writes
/// to is that state's.
pub(crate) fn route() -> Router<AppState> {
    Router::new()
        .route(REPORTS, post(heard))
        .route(WHEREABOUTS, post(whereabouts))
}

/// A refusal on this listener, in the plain text the rest of it refuses in.
fn refused(status: StatusCode, why: &'static str) -> Response {
    (status, format!("{why}\n")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A member's call, so not one of the three the gate stands aside for —
    /// and nor is home asking where a copy went.
    #[test]
    fn the_reports_are_not_one_of_the_un_gated_three() {
        for route in [REPORTS, WHEREABOUTS] {
            assert_ne!(route, super::super::IDENTITY);
            assert_ne!(route, super::super::joining::JOIN);
            assert_ne!(route, super::super::exchange::SETTLED);
        }
    }

    /// And outside the prefixes this device keeps to itself, or nothing could
    /// ever reach either.
    #[test]
    fn the_reports_are_not_in_a_namespace_kept_back() {
        for route in [REPORTS, WHEREABOUTS] {
            for prefix in super::super::workbench::KEPT_TO_ITSELF {
                assert!(
                    !route.starts_with(prefix),
                    "{route} would be held back with {prefix}/",
                );
            }
        }
    }
}
