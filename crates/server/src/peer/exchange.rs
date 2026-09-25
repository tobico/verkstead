//! The dial back, as the device that *asked* answers it: the call that closes a
//! join, and the roster that arrives on it (ADR-0020, *A cluster is a
//! membership*, *The join*).
//!
//! **This is the third route outside the member gate, and the last of them.**
//! [`joining`](super::joining) holds the first two — the join post and the
//! cancel that takes a question back — and this is the one that runs the other
//! way: the device that was asked dials the device that asked it, at the
//! addresses that device advertised, once its human has pressed something. It
//! arrives before the asker has recorded anybody, which is exactly why it
//! cannot be a member's call: the membership it would be checked against is the
//! thing this call is about to create.
//!
//! **And it is not un-authenticated for standing outside the gate.** The asker
//! met a certificate when it posted the join and pinned it into the pending
//! request, and this call is matched against that: a caller presenting anything
//! else is not the device that answered the join, whatever it says about
//! itself. The same pinning holds in the other direction — the dialling end
//! refuses to complete a handshake with any certificate but the one *its*
//! request is holding, so a machine that has taken over either end's address
//! meets a closed door rather than a link.
//!
//! The un-gated surface is three routes and stops there. Everything the stages
//! after this one add to the peer listener — the announcement, the unlink
//! broadcast, the renewed fingerprint — is a member's own call and goes behind
//! the gate.
//!
//! **What arrives is the whole roster rather than the one device.** A newcomer
//! joining a cluster of three is handed all three in this one call, so that
//! nothing is half-linked while some second call is made; in a cluster of two
//! the list beside the introducer is simply empty. None of those members is
//! confirmed by anybody over here: they came over a link this device has just
//! proved against a certificate it pinned itself, which is the same vouching
//! that lets the introducer announce this device to each of them.
//!
//! **And a refusal comes the same way.** Without it the asking device sits
//! reading *waiting* until somebody cancels it — so a Deny is dialled back too,
//! and the row is marked refused and drawn as such until it is dismissed. An
//! expiry is dialled back as well, and records nothing: the moment it is about
//! is the far end's own word, written on the row when the request was made, so
//! what that call is worth is the row redrawing at the moment it happens rather
//! than whenever the page next asks. A dial back that never gets through costs
//! nothing here either — the clock on the row reaches the same answer alone.

use anyhow::Result;
use axum::Json;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use verkstead_render::{DeviceIdentity, JoinSettled};
use verkstead_schema::Nudge;
use verkstead_store::Linking;

use crate::device::Device;

use super::joining::{Joins, run_out};
use super::{Caller, Members};

/// Where a device that was asked to link says what its human decided.
///
/// The request in the path, because it is what is being settled, and under
/// `/api/peer/` with the rest of this listener's own business. Answered by the
/// device that *asked*, which is the one thing about this path that is not like
/// the cancel beside it: that one is dialled at the device holding the
/// question, and this one at the device waiting on it.
pub const SETTLED: &str = "/api/peer/v1/join/{request}/settled";

/// That same path with a request in it, which is what a dial goes out to.
pub fn settling(request: &str) -> String {
    format!("/api/peer/v1/join/{request}/settled")
}

/// The device that asked, as this route answers out of it: what it is, what it
/// is waiting on, and where a member it is handed is written down.
#[derive(Debug, Clone)]
pub(crate) struct Settling {
    /// What this device is — held for one question, which is whether a roster
    /// names this device itself. A well-behaved introducer never sends one that
    /// does; a row recording this machine as a member of its own cluster is the
    /// kind of thing that would be drawn for ever, so it is dropped here rather
    /// than trusted not to arrive.
    pub(crate) device: Device,

    /// Where a member arriving on the roster is written.
    pub(crate) members: Members,

    /// And the pending request the whole call is matched against.
    pub(crate) joins: Joins,

    /// And who to tell once it has landed, which is every workbench of this
    /// device that happens to be open: the pending row has either become a
    /// member or become a refusal, and both are things a page is drawing.
    pub(crate) nudges: crate::nudge::Nudges,
}

/// `POST /api/peer/v1/join/{request}/settled` — the device that was asked,
/// saying what came of the question.
///
/// **Three checks before anything is believed**, and they are the whole of why
/// this route may stand outside the member gate. The request has to be one this
/// device is really waiting on; its ten minutes have to be unspent; and the
/// certificate the handshake took has to be the one that request pinned when
/// the join was posted. A call that fails any of them is refused and nothing is
/// written.
///
/// What it does then is one of two things. A refusal is written on to the
/// pending row, where it is drawn until the human dismisses it — or, for an
/// expiry, written nowhere at all, that being a moment this device can read for
/// itself off the row it already holds. An acceptance is the roster: the
/// introducer and every member it holds, each recorded, and the pending row let
/// go of, there being a membership where the waiting was.
pub(crate) async fn settled(
    State(settling): State<Settling>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    Path(request): Path<String>,
    Json(answer): Json<JoinSettled>,
) -> Response {
    let asked = match settling.joins.asked(&request).await {
        Ok(asked) => asked,
        Err(why) => {
            tracing::error!(%why, "the joins this device is waiting on could not be read");

            return unreadable("this device could not read what it is waiting on");
        }
    };

    let Some(asked) = asked else {
        return refused(
            StatusCode::NOT_FOUND,
            "this device is waiting on no such join request",
        );
    };

    if run_out(&asked.expires_at, time::OffsetDateTime::now_utc()) {
        return refused(
            StatusCode::FORBIDDEN,
            "that request's ten minutes have run out, and a link is not made after them",
        );
    }

    if caller.fingerprint().as_deref() != Some(asked.fingerprint.as_str()) {
        return refused(
            StatusCode::FORBIDDEN,
            "that request was not answered under the certificate this device met when it \
             asked, which is a device that is not the one it asked",
        );
    }

    match answer {
        // Nothing is recorded and nothing is drawn differently: the moment this
        // is about is the one already written on the row, so what the call is
        // worth is the word that the page should look again.
        JoinSettled::Expired => {
            tracing::info!(
                device = %asked.device,
                request = %request,
                "a device this one asked to link with let go of the request without \
                 anybody answering it",
            );
        }

        JoinSettled::Denied => {
            if let Err(why) = settling.joins.refuse(&request).await {
                tracing::error!(%why, "a refused join could not be written down as refused");

                return unreadable("this device could not write the refusal down");
            }

            tracing::info!(
                device = %asked.device,
                request = %request,
                "a device this one asked to link with has refused",
            );
        }

        JoinSettled::Joined {
            introducer,
            members,
        } => {
            // The certificate named against the one presented, which is the
            // judgement every answer in a cluster is held to — and against the
            // one this device pinned, which the handshake has already proved.
            if introducer.fingerprint != asked.fingerprint {
                return refused(
                    StatusCode::FORBIDDEN,
                    "this answer names a certificate other than the one it was dialled \
                     under, which is a device that is not the one it says it is",
                );
            }

            if introducer.device != asked.device {
                return refused(
                    StatusCode::FORBIDDEN,
                    "this answer comes under a device id other than the one that answered \
                     the join, under the certificate recorded for the first of them",
                );
            }

            let recorded = record(&settling, &introducer, members).await;

            match recorded {
                Ok(recorded) => tracing::info!(
                    device = %asked.device,
                    request = %request,
                    recorded,
                    "a device this one asked to link with has let it in, and handed over \
                     itself and every member it holds",
                ),

                Err(why) => {
                    tracing::error!(%why, "a roster handed over on a join could not be recorded");

                    return unreadable("this device could not write the membership down");
                }
            }

            // Last, so that a failure above leaves the request still being
            // waited on rather than a link half made with nothing left naming
            // it: the far end holds the question until this call answers, so
            // the press can simply be made again.
            if let Err(why) = settling.joins.forget(&request).await {
                tracing::error!(%why, "an answered join could not be taken off the pending list");

                return unreadable("this device could not let go of the request");
            }
        }
    }

    // And every open workbench reads the section back: the pending row has
    // become a member, or a refusal, or has run out under the human's eyes.
    settling.nudges.announce(Nudge::Joins);

    StatusCode::NO_CONTENT.into_response()
}

/// Write down the introducer and everybody it holds, and say how many that came
/// to.
///
/// **Keyed on the device id**, which is what makes a roster safe to be handed
/// twice: a device already recorded is the same row said again rather than a
/// second one — see [`verkstead_store::record_member`].
///
/// Two things are dropped rather than recorded. This device itself, because a
/// cluster's membership is everybody *else* and a row for this machine would
/// sit on its own list for ever; and anything naming no device or no
/// certificate, a member being an id and a fingerprint before it is anything
/// else.
async fn record(
    settling: &Settling,
    introducer: &DeviceIdentity,
    members: Vec<DeviceIdentity>,
) -> Result<usize> {
    let mut recorded = 0;

    for identity in std::iter::once(introducer.clone()).chain(members) {
        if identity.device == settling.device.id() {
            continue;
        }

        if identity.device.trim().is_empty() || identity.fingerprint.trim().is_empty() {
            tracing::warn!(
                device = %identity.device,
                "a roster named a device with no id or no certificate, which is not a \
                 member, so it is left out",
            );

            continue;
        }

        settling
            .members
            .refreshed(&Linking {
                device: identity.device,
                name: identity.name,
                os: identity.os,
                addresses: identity.addresses,
                fingerprint: identity.fingerprint,
            })
            .await?;

        recorded += 1;
    }

    Ok(recorded)
}

/// A refusal on this listener, in the plain text the rest of it refuses in.
fn refused(status: StatusCode, why: &'static str) -> Response {
    (status, format!("{why}\n")).into_response()
}

/// And what this device answers when the failure is its own, with nothing about
/// it in the answer — the line saying which failure it was is in this machine's
/// log.
fn unreadable(why: &'static str) -> Response {
    refused(StatusCode::INTERNAL_SERVER_ERROR, why)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path a dial back goes to is the route that answers it, with the
    /// request in the one place the route takes one.
    #[test]
    fn a_dial_back_goes_to_the_route_that_answers_it() {
        assert_eq!(
            SETTLED.replace("{request}", "0011223344556677"),
            settling("0011223344556677"),
        );
    }

    /// And it is a path of its own rather than the cancel's: the two are
    /// dialled at opposite ends of one join.
    #[test]
    fn the_dial_back_is_not_the_cancel() {
        assert_ne!(SETTLED, super::super::joining::CANCEL);
    }
}
