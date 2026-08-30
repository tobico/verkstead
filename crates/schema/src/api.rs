//! The small YAML envelopes the API answers with, kept beside the Set types so
//! the CLI reads back exactly what the server writes.

use serde::{Deserialize, Serialize};

use crate::validate::Violation;

// The refusal shape is the whole server's, viewer included — see `Response` for
// why the emitter is gated.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// What `POST …/api/v1/sets` returns once a Set is stored: the identity the
/// server stamped on it, and whether there is anything to wait on. The CLI waits
/// on `id` where there is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCreated {
    pub id: i64,

    /// When the server accepted the Set, RFC 3339.
    pub created_at: String,

    /// Whether the server stored the Set rather than holding a wait open on it
    /// — a Deferred Ask, or an ask from a backend whose sessions cannot hold a
    /// shell command open for hours (ADR-0011).
    ///
    /// The server's word rather than the CLI's, because which channel a Set was
    /// asked on is a fact about the backend: the CLI asks the same way
    /// everywhere, and the server is what knows the agent type of the session
    /// that asked. A CLI old enough to ignore this opens a wait, which is the
    /// shipped-together case and not one to design around.
    #[serde(default)]
    pub stored: bool,
}

impl SetCreated {
    /// The stored Set as the CLI prints it: there is no Response coming to that
    /// agent, so what it is owed on stdout is that the Set was stored and which
    /// one it is.
    ///
    /// The identity and no more. Whether a wait was opened is how the CLI was
    /// answered rather than anything about the Set, and the agent reading this
    /// has the id it came for either way.
    pub fn to_yaml(&self) -> Result<String, serde_saphyr::SerializeError> {
        serde_saphyr::to_string(&Identity {
            id: self.id,
            created_at: &self.created_at,
        })
    }
}

/// The half of [`SetCreated`] that is about the Set, which is what is printed.
#[derive(Serialize)]
struct Identity<'a> {
    id: i64,
    created_at: &'a str,
}

/// What `POST …/api/v1/sets/{id}/response` returns once a Response is stored.
/// The human's device gets this; the agent gets the Response itself, off the
/// wait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAccepted {
    /// The Set that has just been answered.
    pub set_id: i64,

    /// When the server accepted the Response, RFC 3339.
    pub submitted_at: String,
}

/// What the API returns when it refuses a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ApiError {
    /// One line saying what was refused.
    pub error: String,

    /// The grammar violations behind the refusal, each naming its question.
    /// Empty when the request failed for some other reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<Violation>,
}

impl ApiError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            violations: Vec::new(),
        }
    }

    pub fn with_violations(error: impl Into<String>, violations: Vec<Violation>) -> Self {
        Self {
            error: error.into(),
            violations,
        }
    }
}
