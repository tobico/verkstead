//! The Guide's tailored half, asked for by `verkstead guide` inside a session:
//! how the harness the session runs on asks, and its *Running the ask* section
//! (ADR-0030, *Agent*; Set 1764, Q18).
//!
//! **Fetched rather than carried by the CLI**, because which harnesses there
//! are is the server's to say: a built-in's section is written beside its
//! harness, and a plugin's is what its manifest wrote. The core is the CLI's
//! own and stays in the binary, so a Guide printed where this cannot be reached
//! is the core alone.

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;
use verkstead_schema::{ApiError, GuideSection};

use crate::harness;
use crate::reply::yaml;
use crate::store;

/// What the CLI asks with: the id `VERKSTEAD_AGENT` holds.
#[derive(Debug, Deserialize)]
pub(crate) struct Asked {
    agent: String,
}

/// `GET /conversations/{conversation}/api/v1/guide?agent=<id>` — the section
/// of the harness `agent` names, whichever Conversation asks: what a harness
/// says about asking is the same in every one of them.
///
/// 200 with the [`GuideSection`]. 404 where this server has no harness of that
/// id, saying so — which the CLI tells apart from a server too old to have this
/// route by the body.
pub(crate) async fn section(
    Path(_conversation_id): Path<i64>,
    Query(asked): Query<Asked>,
) -> Response {
    match harness::running(store::AgentType::read(&asked.agent)) {
        Some((channel, running)) => yaml(
            StatusCode::OK,
            &GuideSection {
                blocking: channel == store::Channel::Blocking,
                running: running.to_owned(),
            },
        ),
        None => yaml(
            StatusCode::NOT_FOUND,
            &ApiError::new(format!("this server has no harness {:?}", asked.agent)),
        ),
    }
}
