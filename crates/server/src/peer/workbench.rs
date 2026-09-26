//! The workbench over the link: the viewer's own namespace answered on the Peer
//! Listener, behind the Member Gate rather than behind the Workbench Key's
//! (ADR-0020, *The opened device relays*).
//!
//! **One router of routes rather than two.** What a member asks of this device
//! is what the human's own browser asks of it — the Timeline, a Set answered,
//! the Repo dropdown's presses, an attachment put on a Brief — so the namespace
//! is [`crate::ui::routes`] built once and mounted twice, and what differs is
//! who gets through. A second namespace written for peers would be the viewer's
//! namespace drifting from itself one endpoint at a time.
//!
//! **And over the one state**, which is the whole of why the two routers are
//! built together — see [`crate::standing`]. A Set answered through a relay has
//! to end a wait this device is genuinely holding, and a terminal attached
//! through one has to be the terminal this device is running; two states over
//! one database would be two servers disagreeing about their own work.
//!
//! **What admits a caller here is the certificate it presented at the
//! handshake.** No key cookie is sent and none is needed: a member's Workbench
//! Key is that member's, and it never leaves the device it was issued on. The
//! gate is [`super::members_only`], put on by [`super::router`] — this module
//! hands it the routes and names the ones it holds back.
//!
//! **Three prefixes are this device's own and are not served here at all** —
//! [`KEPT_TO_ITSELF`]. That is what makes *a member's Workbench Key never leaves
//! it* a fact about the mechanism rather than about which pages happen to exist:
//! the Remote access reading carries the login link with that key on it, and a
//! namespace served whole would hand it to whoever holds the other device's
//! cookie. So the three are refused **by name**, rather than quietly missing, so
//! that a caller can tell *this is not relayed* from *this Verkstead is too old
//! to have it* — the same distinction the Member Gate's own refusal is a
//! `Forbidden` rather than a `Not Found` for.
//!
//! **And the agents' half is not here.** A session's Conversation-scoped API
//! answers the loopback and the named pipe, which is all a session ever dials,
//! and the health check is nobody's Conversation; the viewer's fallback is a
//! page a browser asks its own device for. None of the three is a route on this
//! router, so none of them is on this listener — see [`crate::Routers`], where
//! the two are told apart.

use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::AppState;

/// The prefixes this device keeps to itself, whatever a member asks.
///
/// `/api/ui/remote/` is the Remote access pane, whose reading carries the login
/// link and with it this device's Workbench Key. `/api/ui/devices/` is the
/// Devices section — the joins, the presses that settle one, the unlink — which
/// is one human at one machine deciding who this device is linked to, and not
/// something another device asks on their behalf. And `/api/ui/push/` is the
/// subscriptions of the browsers *this* device pushes to, which are its own
/// phones rather than anybody else's.
///
/// Spelled without their trailing slash and matched as whole segments below, so
/// that the bare `/api/ui/remote` the pane reads is held back with everything
/// under it, and a path that merely begins with those letters is not.
pub(crate) const KEPT_TO_ITSELF: [&str; 3] = ["/api/ui/remote", "/api/ui/devices", "/api/ui/push"];

/// The viewer's own namespace over this device's state: what a member reaches,
/// once [`super::router`] has put the gate in front of it.
///
/// The state is [`crate::standing`]'s, made once and shared with the workbench's
/// own router — see this module's own documentation, and [`crate::Routers`].
pub(crate) fn served(state: AppState) -> Router {
    crate::ui::routes().with_state(state)
}

/// And the three prefixes held back, over everything the gate admits.
///
/// **A layer over the whole of it rather than a route apiece**, which is what
/// makes the refusal a fact about the namespace rather than about the endpoints
/// that happen to be in it: `/api/ui/devices/discovered` is a route and
/// `/api/ui/devices/whatever-comes-next` is not, and a member asking either is
/// asking after something this device keeps to itself. Routes would refuse the
/// first and miss the second, and the stage that adds the second would be the
/// stage that quietly served it.
///
/// Put on inside [`super::members_only`] and therefore *behind* the Member Gate:
/// a stranger is refused for not being a member, and has no business learning
/// which of this device's namespaces are relayed and which are not.
pub(crate) fn keeping_three_back(routes: Router) -> Router {
    routes.layer(axum::middleware::from_fn(kept_back))
}

/// The check itself: one of the three, or on to the routes.
async fn kept_back(request: Request, next: Next) -> Response {
    match ours(request.uri().path()) {
        Some(prefix) => refused(prefix),
        None => next.run(request).await,
    }
}

/// Which of [`KEPT_TO_ITSELF`] `path` is under, where it is under one.
///
/// The prefix itself or a segment below it, rather than the letters it starts
/// with: `/api/ui/remote` is the pane's own reading and `/api/ui/remote/serve`
/// is a press on it, while a `/api/ui/remotes` somebody adds later is a
/// different endpoint and is not held back by this one's spelling.
fn ours(path: &str) -> Option<&'static str> {
    KEPT_TO_ITSELF.into_iter().find(|prefix| {
        path == *prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// And the refusal, which names the namespace it is about.
///
/// **`Forbidden` and a sentence**, for the reason the Member Gate's refusal is
/// both: a caller that could not tell this from a missing route could not tell
/// a namespace this device keeps to itself from a Verkstead too old to have the
/// endpoint, and those want different things of it — the first is nothing to
/// retry anywhere, and the second is an upgrade on this machine.
fn refused(prefix: &'static str) -> Response {
    (
        StatusCode::FORBIDDEN,
        format!(
            "{prefix}/ is a namespace this device keeps to itself, and is not served \
             over its peer listener\n",
        ),
    )
        .into_response()
}
