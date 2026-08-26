//! What became of the two things the human can do to a Set: answer it, or close
//! it unanswered.
//!
//! Both are named outcomes rather than status codes, because every one of them
//! is something the viewer has to say in words — a Set answered from another
//! device, one locked in another tab, a Response the page should never have
//! built. None of them is an error the human can act on by trying again, and
//! none of them is silent.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// What became of the human's Response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Submitted {
    /// Stored as the Set's answer; whoever was waiting has been woken.
    Accepted,

    /// The Set was answered before this Response arrived — the first stands,
    /// and this one was discarded.
    AlreadyAnswered,

    /// There is no such Set, though there was one when the page loaded.
    NoSuchSet,

    /// The Set was locked unanswered before this Response arrived — from
    /// another device, or another tab. Locking closes a Set for good, so it
    /// cannot also become an answered one.
    Locked,

    /// The Response does not resolve the Set. The viewer builds Responses that
    /// do, so this is a bug rather than something the human can fix — but it
    /// is carried back and shown rather than swallowed.
    Rejected(Vec<String>),
}

/// What became of the human closing a Set unanswered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Locked {
    /// Closed: the Set has stopped waiting on the human, stands on its
    /// Conversation's Timeline as the Set nobody answered, and a CLI still
    /// holding a wait on it has been told.
    Closed,

    /// It was answered before this arrived, so it stands as the decision that
    /// was made. Nothing was changed — a decision is not something to close.
    AlreadyAnswered,

    /// It had already been locked, from another device or another tab.
    AlreadyLocked,

    /// There is no such Set, though there was one when the page loaded.
    NoSuchSet,
}
