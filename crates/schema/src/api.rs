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

/// The most a Memo may hold, in bytes of its markdown (ADR-0023).
///
/// Here rather than on either end, because both ends hold it: the CLI refuses
/// a longer one before sending anything, as a courtesy, and the server refuses
/// one sent by hand, as the rule.
pub const MAX_MEMO_BYTES: usize = 8 * 1024;

/// Why a Memo's markdown is refused before it goes anywhere, or `None` where
/// it may be left: nothing in it, or more than [`MAX_MEMO_BYTES`].
pub fn memo_refused(markdown: &str) -> Option<String> {
    if markdown.trim().is_empty() {
        return Some(
            "the memo is empty: write what the sessions after this one should know".to_owned(),
        );
    }

    (markdown.len() > MAX_MEMO_BYTES).then(|| {
        format!(
            "the memo is {} bytes, over the maximum of {} (8 KiB): keep it to what the next \
             session will walk into, and leave a second memo for a second thing",
            markdown.len(),
            MAX_MEMO_BYTES
        )
    })
}

/// What `POST …/api/v1/memos` returns once a Memo is on the Timeline: the
/// Event it is, which is the number it goes by. The CLI prints this and nothing
/// else on stdout, so an agent can read it back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoLeft {
    pub id: i64,
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
