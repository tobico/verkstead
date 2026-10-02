//! The news, as the member being told answers it: a device this one is already
//! linked to, saying that something worth a phone has happened to a piece of its
//! work (ADR-0020, *The opened device relays*).
//!
//! **What this is for.** A phone is installed from one device and subscribes to
//! that device's browsers, so a cluster whose news never crossed the link would
//! be a cluster the human needed a phone per machine for. So a member tells its
//! members as the news happens, and each of them pushes it to its own phones
//! with the device leading the title — *the-laptop — pwa-and-push is done* — so
//! that one phone is enough and a lock screen says which machine the work was
//! on.
//!
//! **A hub cannot subscribe to a member's push instead**, which is what settles
//! the direction: the subscriptions a device pushes to are its own browsers, and
//! `/api/ui/push/` is one of the prefixes never served over the link — see
//! [`super::workbench::KEPT_TO_ITSELF`]. And a **Nudge** is the wrong carrier by
//! construction: it says what *kind* of thing moved and never what it was, and a
//! notification is a sentence.
//!
//! **A member's own call, so it stands inside [`super::gate`]** with the
//! announcement, the unlink and the renewal. A device whose certificate nothing
//! here has recorded has no news this device has any reason to show a human, and
//! saying which device it *is* is exactly what it cannot prove.
//!
//! **What arrives is prose**, which is what makes the call survive a version
//! skew: the sentence was written on the device the work is on, where the kind of
//! news is known, so a news kind a newer member has and this one has not still
//! reads — there is no variant here to match on. See
//! [`verkstead_render::RelayedNews`], and `News::title` in [`crate::push`], where
//! every title in a cluster is written.
//!
//! **The title is this device's to write**, rather than a sentence passed through
//! untouched. It puts the member's name in front of what it was told, and it
//! bounds both halves the way [`crate::push`] already bounds the one thing in a
//! notification that another machine wrote: a device that called itself something
//! enormous, or sent a sentence that was not one, must not be the whole of a lock
//! screen.
//!
//! **And the tap opens the Conversation where the phone can reach it.** The path
//! is composed onto the *sending* device's segment —
//! `/devices/{device}/conversations/{id}` — because that is the URL the phone's
//! own workbench draws a member's Conversation at (see [`crate::relaying`]). A
//! path taken verbatim would open this device's Conversation of the same number,
//! which is the id collision a cluster addresses every row by device for.
//!
//! **And nothing goes on from here.** The push this sends is local, so the news
//! stops at the device it was told to: in a cluster everybody holds a link to
//! everybody, and a device passing on what a third one told it would be saying
//! that news was its own — which is the rule the Nudge streams are held under
//! too.

use axum::Json;
use axum::Router;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use verkstead_render::RelayedNews;

use crate::AppState;

use super::Caller;

/// Where a member says it has something worth a phone.
///
/// **Not under `/members`, where the announcement and the unlink are**, and not
/// under `/api/ui/` either. Those two are a device being put on this one's list
/// and taken off it; this is a device saying something about its own work, and
/// the caller is the device it is about — so the path names nobody, exactly as
/// the renewal beside it does.
pub const NEWS: &str = "/api/peer/v1/news";

/// How much of a sentence another machine wrote this device will keep.
///
/// Generous against what a Verkstead really sends — every title
/// [`crate::push::News`] writes is a line and a branch name — and refused rather
/// than trimmed past it, for the reason a join's own bounds refuse: a sentence
/// cut in half is a notification nobody can read, and a well-behaved member never
/// comes near this. What *is* trimmed is the title, which is a separate job with
/// a lock screen's width behind it.
const LONGEST_SENTENCE: usize = 1024;

/// And how much of a repository name, which is the other string that rides along.
///
/// A repository's name is a directory's, so this is the bound a hostname is held
/// to on a join — see [`super::joining`].
const LONGEST_PROJECT: usize = 255;

/// `POST /api/peer/v1/news` — a member, saying what has just happened to a piece
/// of its work.
///
/// **Behind the gate, so the caller is a member before this is read at all.**
/// What is left to judge is the payload: a sentence to show, a Conversation to
/// open, and nothing enormous. And *which* member is asked of the certificate the
/// handshake took rather than of the body — see [`super::Members::presenting`] —
/// because the device is what the title leads with and what the tap is routed by,
/// and an id in a payload is a string anybody may write.
///
/// **`204` and nothing in the body.** What the caller needed to know is that the
/// call landed; whether a phone ever lights up is between this device and a push
/// service, and the caller is not waiting on that any more than it waits on its
/// own.
pub(crate) async fn heard(
    State(state): State<AppState>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    Json(news): Json<RelayedNews>,
) -> Response {
    // The gate let this caller through on a certificate, so there is one; a
    // refusal here rather than an unwrap because the two are separate readings of
    // one connection and nothing in this file enforces the order.
    let Some(presented) = caller.fingerprint() else {
        return refused(
            StatusCode::FORBIDDEN,
            "a device that presented no certificate has no news of its own to tell",
        );
    };

    if news.said.trim().is_empty() {
        return refused(
            StatusCode::BAD_REQUEST,
            "a piece of news with nothing said is nothing to show anybody",
        );
    }

    if news.said.chars().count() > LONGEST_SENTENCE {
        return refused(
            StatusCode::BAD_REQUEST,
            "a piece of news that long is not a sentence",
        );
    }

    if news
        .project
        .as_ref()
        .is_some_and(|project| project.chars().count() > LONGEST_PROJECT)
    {
        return refused(
            StatusCode::BAD_REQUEST,
            "a repository name that long is not one",
        );
    }

    // A Conversation is a row id, and those begin at one. A number below it names
    // nothing on the far end either, so there would be nowhere for a tap to go.
    if news.conversation < 1 {
        return refused(
            StatusCode::BAD_REQUEST,
            "a piece of news about no Conversation has nowhere to be opened",
        );
    }

    // What this device is, which is the handle the membership is read off. A
    // router stood up without a Data Directory invented no identity and holds no
    // membership, so it has no way of saying which device is calling — which is
    // every router but a served one.
    let Some(devices) = state.devices.as_ref() else {
        tracing::error!("a piece of news arrived at a device that does not know what it is");

        return refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "this device cannot say which of its members is calling",
        );
    };

    let caller_is = match devices.membership().presenting(&presented).await {
        Ok(caller_is) => caller_is,

        Err(why) => {
            tracing::error!(%why, "the membership a piece of news is attributed to could not be read");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read the membership",
            );
        }
    };

    // Through the gate and off the list a moment later, which is an unlink
    // between the two readings. Nothing to show and nothing to fail: that device
    // is not a member, so there is no row to name it by and no URL to open it at.
    let Some(member) = caller_is else {
        return StatusCode::NO_CONTENT.into_response();
    };

    tracing::info!(
        device = %member.device,
        conversation = news.conversation,
        "a member has news worth a phone, so this device's own phones are told",
    );

    crate::push::heard_from(&state.pool, &member.device, &member.name, news);

    StatusCode::NO_CONTENT.into_response()
}

/// The route, over the state the workbench answers out of.
///
/// **Built here and mounted by [`super::workbench::served`]**, which is where the
/// rest of what a member reaches is built, and for its reason: the push
/// subscriptions this news is shown to are the state the *other* listener answers
/// out of, and a router over a second state would be pushing to a second
/// device's phones. [`super::router`] is what puts the Member Gate over it.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(NEWS, post(heard))
}

/// A refusal on this listener, in the plain text the rest of it refuses in.
fn refused(status: StatusCode, why: &'static str) -> Response {
    (status, format!("{why}\n")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The news is its own path rather than any of the three the gate stands
    /// aside for: it is a member's call, and it goes where a member's calls go.
    #[test]
    fn the_news_is_not_one_of_the_un_gated_three() {
        assert_ne!(NEWS, super::super::IDENTITY);
        assert_ne!(NEWS, super::super::joining::JOIN);
        assert_ne!(NEWS, super::super::exchange::SETTLED);
    }

    /// And it names no device, because the caller is the device it is about and
    /// the handshake is what says which — a path with an id in it would invite a
    /// member to tell this device news in somebody else's name.
    #[test]
    fn the_news_names_no_device() {
        assert!(
            !NEWS.contains('{'),
            "the news path carries no segment somebody fills in: {NEWS}",
        );
    }

    /// And it is outside the prefixes this device keeps to itself, which is
    /// what it has to be: a member's news arriving on the one namespace a member
    /// is never served would be a route nothing could ever reach.
    #[test]
    fn the_news_is_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !NEWS.starts_with(prefix),
                "the news would be held back with {prefix}/",
            );
        }
    }
}
