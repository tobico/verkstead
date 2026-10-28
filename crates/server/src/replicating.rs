//! The **Replicated records** on the server's side: the kinds this server
//! declares on the store's facility, the route every device serves its whole
//! table of a kind on, and the puller that merges every member's (ADR-0028).
//!
//! The record — a uuid, an `updated_at`, a deletion stamp and the device that
//! wrote it — and the two rules a record is merged and stamped by are the
//! store's, said once for every kind — see [`store::replicated`]. What lives
//! here is each kind's own: what its fields are, where it was kept before it
//! was a record, and who holds it in memory. Authors are the first, the
//! Agent Profiles with their bindings the second, the Instructions text the
//! third, and the declared MCP servers the fourth.
//!
//! **Every device serves its whole table** of a kind on [`RECORDS`], behind
//! the Member Gate — live records and removed ones, whoever wrote them — so a
//! record whose writer is off, or unlinked, still reaches a newcomer through
//! whoever holds it. Nothing is relayed to a home, and nothing is pushed on a
//! write: **every device pulls**, and merges what each member serves by the
//! merge rule — at the start, on the kind's *moved* Nudge from a member, when
//! a member's Nudge stream is taken up again ([`Nudge::Everything`]), and when
//! the membership moves. A pull is idempotent, so a Nudge missed costs
//! nothing but the wait for the next one, and a Nudge that was not needed
//! costs a read.
//!
//! **A member too old to answer the route** passes the Member Gate and answers
//! `404` — the gate's own refusal being a `403` — which is a failed read like
//! any other: [`FailedReads`] puts it on the member's row with both releases
//! (ADR-0027), and its records are absent here until it updates.
//!
//! Named for what it does rather than `records`, which is OpenCode's store
//! reader's.

pub mod authors;
pub mod instructions;
pub mod mcp_servers;
pub mod profiles;

use std::future::Future;

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_schema::Nudge;

use crate::AppState;
use crate::relaying::{FailedReads, Read, Unread, answer_of};
use crate::store::replicated::{self, Kind, Record};

/// Where a member asks for this device's whole table of one kind, by the
/// kind's name — `authors` for the Authors, `profiles` for the Agent Profiles
/// and `profile_bindings` for their bindings.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace and is drawn by a
/// browser: what crosses here is records, removed ones included, which no
/// page draws.
pub const RECORDS: &str = "/api/peer/v1/records/{kind}";

/// The most a member's table of one kind may be before the answer is dropped
/// rather than merged: **the Profiles' megabyte**, for its reason. An Author
/// is a few hundred bytes, and a removed one is kept.
const MOST_A_TABLE_IS: usize = crate::mirroring::MOST_THE_PROFILES_ARE;

/// A kind of record this server replicates: the store's [`Kind`], and what it
/// takes to tell this device about one.
pub(crate) trait Replicating: Kind + 'static
where
    Self::Fields: Send + Sync,
{
    /// The Nudge a member announces when its records of this kind moved, and
    /// this device announces on a write of one.
    const MOVED: Nudge;

    /// What a failed read of a member's table is said as on its row.
    const READ: Read;

    /// Hand what a merge moved on to whatever reads the kind in memory.
    /// Nothing, for a kind read out of the database every time. The pages and
    /// the members are told by the puller, with [`Self::MOVED`].
    fn merged(_state: &AppState) -> impl Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// What else a member's records being read says to do, whether or not the
    /// merge moved anything — fetching what the records name and this device
    /// lacks, which a member not answering last time may have now. Nothing,
    /// for a kind with nothing beside its records.
    fn pulled(_state: &AppState) -> impl Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }
}

/// `GET /api/peer/v1/records/{kind}` — every record of the kind this device
/// holds, removed ones included, whoever wrote them. A kind this release does
/// not know is `404`, which is what a member newer than this one is told.
async fn served(State(state): State<AppState>, Path(kind): Path<String>) -> Response {
    match kind.as_str() {
        authors::Authors::NAME => table::<authors::Authors>(&state).await,
        crate::store::Profiles::NAME => table::<crate::store::Profiles>(&state).await,
        crate::store::Bindings::NAME => table::<crate::store::Bindings>(&state).await,
        instructions::Instructions::NAME => table::<instructions::Instructions>(&state).await,
        mcp_servers::McpServers::NAME => table::<mcp_servers::McpServers>(&state).await,
        _ => (
            StatusCode::NOT_FOUND,
            "this device keeps no records of that kind\n",
        )
            .into_response(),
    }
}

async fn table<K: Kind>(state: &AppState) -> Response
where
    K::Fields: serde::Serialize,
{
    match replicated::records::<K>(&state.pool).await {
        Ok(records) => Json(records).into_response(),
        Err(error) => {
            tracing::error!(
                error = ?error,
                kind = K::NAME,
                "a member asked for this device's records and they could not be read",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own records\n",
            )
                .into_response()
        }
    }
}

/// The route, over the state the workbench answers out of — mounted by
/// [`crate::peer::workbench::served`] beside the members' other calls.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(RECORDS, get(served))
}

/// Pull every member's records of every kind, for as long as this server runs.
///
/// Spawned beside the merged list's refresher and for its reason: what it does
/// first is read every member, some of which are laptops that are shut, and it
/// never returns. Nothing at all on a router with no identity, which is linked
/// to nothing.
pub(crate) fn pulling(state: &AppState) {
    if state.devices.is_none() {
        return;
    }

    let state = state.clone();

    tokio::spawn(async move {
        // Which device this is, said to the Profiles before any is served or
        // pulled — a binding is keyed by the device it is of, and one written
        // before that was said is keyed by none. The serve says it before
        // anything else and this finds nothing to do; a router stood up
        // without one, which is a suite's, is said it here.
        if let Err(error) = crate::store::stamp_the_profiles(&state.pool, &state.device).await {
            tracing::error!(
                error = ?error,
                "which device this is could not be said to the Profiles, so none were pulled",
            );
            return;
        }

        // And what it is called, which a device with members names its own
        // unnamed Profiles after, once — see [`crate::store::this_device_called`].
        // Before the pullers, so a member's first pull of them finds them named.
        named(&state).await;

        // And whether each account bound here holds a login, said on its
        // binding before a member asks — see [`crate::profiles::looked`].
        crate::profiles::looked_at_all(&state).await;

        tokio::spawn(pulled::<authors::Authors>(state.clone()));
        tokio::spawn(pulled::<crate::store::Profiles>(state.clone()));
        tokio::spawn(pulled::<crate::store::Bindings>(state.clone()));
        tokio::spawn(pulled::<instructions::Instructions>(state.clone()));
        tokio::spawn(pulled::<mcp_servers::McpServers>(state));
    });
}

/// Say what this device is called to the Profiles, as the Devices pane draws
/// it, and tell the members where that named any.
async fn named(state: &AppState) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    match crate::store::this_device_called(&state.pool, &devices.name()).await {
        Ok(named) if named.is_empty() => {}
        Ok(named) => {
            tracing::info!(
                profiles = named.len(),
                "this device's unnamed Profiles are named after it, it having members",
            );
            state.nudges.announce(Nudge::Profiles);
        }
        Err(error) => tracing::error!(
            error = ?error,
            "what this device is called could not be said to the Profiles, so its unnamed \
             ones are not named after it",
        ),
    }
}

/// Every member pulled once, and then whatever a Nudge says to pull again —
/// subscribed before the first round, so a Nudge landing during it is heard.
async fn pulled<K: Replicating>(state: AppState)
where
    K::Fields: Send + Sync,
{
    let mut moved = state.nudges.subscribe();

    pull_every_member::<K>(&state).await;

    loop {
        match moved.recv().await {
            Ok(moved) => match (moved.device, &moved.moved) {
                // A member saying its records of the kind moved, or a stream
                // to one taken up after a machine came back.
                (Some(device), nudge) if *nudge == K::MOVED || *nudge == Nudge::Everything => {
                    pull_member::<K>(&state, &device).await
                }

                // The membership moving: a device linked while this server is
                // up holds records nothing here has heard of. Not a row's line
                // moving, which is not the membership — see [`FailedReads`].
                (None, Nudge::Devices) if !moved.kept_here => pull_every_member::<K>(&state).await,

                _ => {}
            },

            Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                tracing::debug!(
                    missed,
                    kind = K::NAME,
                    "the records fell behind the Nudges, so every member's are pulled again",
                );
                pull_every_member::<K>(&state).await;
            }

            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        }
    }
}

/// Pull every member's records of kind `K`, one after another.
///
/// **A device unlinked takes nothing with it**: a record is the cluster's,
/// and what this device merged from it stays.
async fn pull_every_member<K: Replicating>(state: &AppState)
where
    K::Fields: Send + Sync,
{
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let members = match devices.membership().rows().await {
        Ok(members) => members,
        Err(why) => {
            tracing::error!(
                error = ?why,
                kind = K::NAME,
                "the devices this one is linked to could not be read, so no member's records \
                 were pulled",
            );
            return;
        }
    };

    for member in members {
        pull_member::<K>(state, &member.device).await;
    }
}

/// Pull one member's records of kind `K` and merge them into this device's.
///
/// **A read that could not be made merges nothing**, and so does an answer
/// this device cannot read: every record here stands as it is. One that
/// answered is said on its row, with both releases — see [`FailedReads`].
///
/// **And what moved is this device's news**, told to its pages and to every
/// member: a device linked to this one and not yet to the writer — one an
/// introduction has not reached — hears of the record only from here. The
/// member that served it pulls it back and merges nothing, and a merge that
/// moved nothing says nothing, so the news stops there.
async fn pull_member<K: Replicating>(state: &AppState, device: &str)
where
    K::Fields: Send + Sync,
{
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let reads: &FailedReads = devices.failed_reads();
    let failed = |why| reads.failed(&state.nudges, device, K::READ, why);

    let said = match answer_of(
        devices,
        device,
        RECORDS.replace("{kind}", K::NAME),
        K::NAME,
        MOST_A_TABLE_IS,
    )
    .await
    {
        Ok(said) => said,
        Err(why) => return failed(why),
    };

    let records = match serde_json::from_slice::<Vec<Record<K::Fields>>>(&said) {
        Ok(records) => records,
        Err(why) => {
            tracing::warn!(
                device,
                kind = K::NAME,
                "a member served its records in a way this device cannot read, so none of \
                 them were merged: {why}",
            );
            return failed(Unread::Answered);
        }
    };

    reads.held(&state.nudges, device, K::READ);

    let merged = match replicated::merge::<K>(&state.pool, &records).await {
        Ok(moved) => moved,
        Err(error) => {
            tracing::error!(
                error = ?error,
                device,
                kind = K::NAME,
                "a member's records could not be merged",
            );
            return;
        }
    };

    if merged {
        if let Err(error) = K::merged(state).await {
            tracing::error!(
                error = ?error,
                kind = K::NAME,
                "a member's records were merged and could not be handed on",
            );
        }

        state.nudges.announce(K::MOVED);
    }

    if let Err(error) = K::pulled(state).await {
        tracing::error!(
            error = ?error,
            kind = K::NAME,
            "a member's records were pulled and what they name could not be fetched",
        );
    }
}
