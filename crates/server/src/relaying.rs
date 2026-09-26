//! The hop: this device taking an ordinary call for one of its members, and
//! putting it to that member over the Peer Listener (ADR-0020, *The opened
//! device relays*).
//!
//! **The browser stays same-origin and knows nothing about this.** It asks the
//! device it opened, over the origin it already has and the cookie it already
//! holds, and what it gets back is what the far end said — method, path, query,
//! body and status carried through, and the answer handed over untouched. The
//! other half of the hop is [`crate::peer::workbench`], which is where a
//! member's call lands.
//!
//! **A prefix of its own** — `/api/ui/members/{device}/…` — rather than a
//! segment under `/api/ui/devices/`, that being the Devices section's own
//! namespace already: a Device Id is sixteen random hex bytes and could not
//! collide with the words under it, but two namespaces one segment apart read
//! as one thing. What stands after the Device Id is the path on the far end
//! with its `/api/ui` back on the front, so
//! `/api/ui/members/{device}/conversations` is that device's own
//! `/api/ui/conversations` and nothing else.
//!
//! **On the workbench listener alone.** This is what a browser asks of the
//! device it opened, and nothing over the Peer Listener has any business asking
//! it: a member reaching this would be a relay of a relay, and a cluster where
//! a call's route depends on which way round two devices were opened. So it is
//! merged into [`crate::serving`] beside the gated namespace rather than put in
//! [`crate::ui::routes`], which is the router that is mounted twice.
//!
//! **The body is streamed rather than held**, in both directions. An attachment
//! is an ordinary `POST` here — raw bytes, the name in the path — and the route
//! at the far end carries a limit of its own and answers `413` over it, which
//! the composer reads by name. So this hop puts no limit of its own in front of
//! it: nothing here extracts the body, so nothing here applies a
//! `DefaultBodyLimit`, and what comes back is the member's own refusal rather
//! than a judgement made after buffering a file to make it. See [`Streamed`].
//!
//! **And this device's own cookie does not travel.** Everything else that
//! matters is forwarded verbatim — see [`forwarded`] — but the `Cookie` header
//! carries this device's Workbench Key, and a hop that passed it on would hand
//! the key to every machine this one is linked to. *A device's Workbench Key
//! never leaves it* is a fact about both ends of the hop or about neither.
//!
//! **And a socket is carried too, on the same route.** Three of the endpoints in
//! that namespace answer an upgrade rather than a request — a Conversation
//! terminal, a session's Screen and a Code pane's watcher — and what a browser
//! opens on one of them has to reach the member that holds the Conversation. So
//! this module tells the two apart by the headers and hands a socket to
//! [`bridging`], which puts the same upgrade to the member and joins the two
//! connections. Everything about a socket that is different from a call is over
//! there.

mod bridging;

use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use tokio_stream::Stream;
use verkstead_schema::ApiError;

use crate::AppState;
use crate::device::Unrelayed;
use crate::ui::{refused, unavailable};

/// The prefix a relayed call stands under, with the Device Id as the segment
/// after it.
const UNDER: &str = "/api/ui/members/";

/// The namespace on the far end that everything under that prefix is a call
/// into: the viewer's own, which is the whole of what a member serves.
const NAMESPACE: &str = "/api/ui/";

/// The headers that are the connection's rather than the call's, plus the two
/// this device does not pass on.
///
/// The hop-by-hop headers of RFC 9110 are the connection's own, and forwarding
/// one would be describing this hop's socket to a machine on the other side of
/// another. `Host` is where the request was sent and the dial writes its own.
/// `Content-Length` is the length of what the browser sent and the far end is
/// written to in chunks, so the dial frames the body itself rather than
/// repeating a number it is not going to honour.
///
/// And `Cookie`, which is this device's Workbench Key — see this module's own
/// documentation.
///
/// **Two of these travel after all where the call is a socket**: `Connection`
/// and `Upgrade` are what say it is one, so on that route they are the request
/// rather than a description of this hop's own — see [`bridging::asking`], which
/// puts exactly those two back over what this list leaves.
const KEPT_BACK: [&str; 11] = [
    "connection",
    "content-length",
    "cookie",
    "host",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// And the ones the answer's are stripped of on the way back, for the first
/// half of the same reason: the far end's framing is between it and this
/// device, and what goes to the browser is framed again here.
///
/// **A `101` is stripped of neither**, for the mirror of the reason above: an
/// upgrade's answer has no framing to redo and its `Connection` is what the
/// browser's own client reads — see [`bridging::bridged`], which is where such
/// an answer is handed back instead.
const NOT_HANDED_BACK: [&str; 2] = ["connection", "transfer-encoding"];

/// The relay's one route: anything at all, under a member's Device Id.
///
/// **One route rather than a list of them**, which is the whole point of a
/// relay: what this device forwards is whatever the far end answers, and a
/// route apiece would be this device holding an opinion about which of its
/// members' endpoints exist. Every method for the same reason — the namespace
/// has reads and presses in it, and nothing here reads either.
///
/// The three attach endpoints are on this route too rather than beside it: what
/// tells a socket from a call is the headers the browser sent, so a route of
/// their own would be a second list of endpoints to keep — see [`bridging`].
pub(crate) fn routes() -> Router<AppState> {
    Router::new().route(&format!("{UNDER}{{device}}/{{*rest}}"), any(relay))
}

/// `ANY /api/ui/members/{device}/…` — put this call to `device` and hand back
/// what it answered, or, where it is a socket, hold the two ends of it together.
async fn relay(State(state): State<AppState>, request: Request) -> Response {
    let Some(devices) = state.devices.clone() else {
        return unavailable("this server holds no device identity to relay through");
    };

    // The route above cannot match a path this fails on. Said as a refusal
    // rather than as an unwrap because the two facts are a route table apart,
    // and a panic in a handler is the worst of the ways to find that out.
    let Some((device, onwards)) = addressed(request.uri()) else {
        return refused(
            StatusCode::NOT_FOUND,
            ApiError::new("a relayed call stands under a Device Id and a path"),
        );
    };

    let (mut parts, body) = request.into_parts();

    // Whether this is a socket rather than a call, and the browser's own half of
    // it where it is — taken out of the connection before anything is dialled,
    // for the reason [`bridging::taken`] gives.
    let mut taking = None;

    if bridging::upgrading(&parts.headers) {
        match bridging::taken(&mut parts.extensions) {
            Some(half) => taking = Some(half),
            None => return bridging::not_upgradable(),
        }
    }

    let call = Call {
        method: parts.method,
        onwards,
        headers: match taking.is_some() {
            true => bridging::asking(&parts.headers),
            false => forwarded(&parts.headers),
        },
        body: Streamed::of(&parts.headers, body),
    };

    match devices.relay(&device, call).await {
        Ok(answered) => match taking {
            Some(taking) => bridging::bridged(taking, answered),
            None => handed_back(answered),
        },

        Err(Unrelayed::ThisDevice) => refused(
            StatusCode::BAD_REQUEST,
            ApiError::new(format!(
                "device {device} is this device, and a call to it is made at the path it \
                 stands at rather than relayed",
            )),
        ),

        Err(Unrelayed::NoSuchMember) => refused(
            StatusCode::NOT_FOUND,
            ApiError::new(format!("this device knows no device {device}")),
        ),

        Err(Unrelayed::Unreachable(why)) => {
            refused(StatusCode::BAD_GATEWAY, ApiError::new(format!("{why:#}")))
        }

        Err(Unrelayed::Unreadable(why)) => unavailable(&format!(
            "the devices this one is linked to could not be read: {why:#}",
        )),
    }
}

/// Which device a call is for, and what it is a call for over there.
///
/// Read off the raw path rather than out of a `Path` extractor, because
/// *verbatim* is the whole of what this hop promises: a name with a separator
/// in it arrives percent-encoded and has to leave that way, and an extractor
/// decodes. The query rides along untouched for the same reason.
fn addressed(uri: &Uri) -> Option<(String, String)> {
    let (device, rest) = uri.path().strip_prefix(UNDER)?.split_once('/')?;

    if device.is_empty() {
        return None;
    }

    let mut onwards = format!("{NAMESPACE}{rest}");

    if let Some(query) = uri.query() {
        onwards.push('?');
        onwards.push_str(query);
    }

    Some((device.to_owned(), onwards))
}

/// The browser's headers, less the ones that are this hop's own — see
/// [`KEPT_BACK`].
fn forwarded(headers: &HeaderMap) -> HeaderMap {
    headers
        .iter()
        .filter(|(name, _)| !KEPT_BACK.contains(&name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

/// And the member's answer, as this device's own: the status, the headers and
/// the body, carried through.
///
/// The body is the far end's stream rather than its bytes, which is the same
/// promise the request half makes: a file read back through the hop is written
/// to the browser as it arrives.
fn handed_back(answered: reqwest::Response) -> Response {
    let status = answered.status();
    let headers: HeaderMap<HeaderValue> = answered
        .headers()
        .iter()
        .filter(|(name, _)| !NOT_HANDED_BACK.contains(&name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();

    (status, headers, Body::from_stream(answered.bytes_stream())).into_response()
}

/// A call on its way to a member: what [`crate::peer::dialling::Peers::relay`]
/// makes a request out of, at each address in turn.
pub(crate) struct Call {
    /// The method the browser used.
    pub(crate) method: reqwest::Method,

    /// The path and query on the far end — `/api/ui/…`, exactly as the browser
    /// wrote it under the prefix.
    pub(crate) onwards: String,

    /// Its headers, less this hop's own.
    pub(crate) headers: HeaderMap,

    /// And its body, which is read as it is written rather than held.
    pub(crate) body: Streamed,
}

/// The browser's body, as something one dial at a time may send.
///
/// **Read once, and read as it is written.** The bytes never sit anywhere: what
/// this holds is the browser's own stream, and the far end reads it at the
/// speed the browser is writing it. An attachment is thirty-two megabytes, and
/// the alternative is thirty-two megabytes of this device's memory per upload
/// in flight to buy nothing at all.
///
/// **Which is also why a dial's walk down a member's addresses is shorter
/// here.** A request builder owns its body, so each attempt is handed a handle
/// on the one stream — see [`Streamed::for_one_attempt`] — and a second attempt
/// that had read part of it would send the rest of a body rather than a body.
/// So [`crate::peer::dialling::Peers::relay`] tries the next address only where
/// the attempt failed to *connect*, which is the one failure that happens
/// before a single byte of the body is asked for. Anything later is a machine
/// that answered and then stopped, and there is nothing left to send it again.
pub(crate) enum Streamed {
    /// No body at all, which is every read and every press that carries none.
    /// Sent as no body rather than as an empty one, so that a relayed `GET`
    /// reaches the far end looking like the `GET` the browser made.
    Nothing,

    /// And the browser's own bytes, read by whichever attempt gets far enough
    /// to ask for them.
    Bytes(Arc<TheOneStream>),
}

impl Streamed {
    /// What `body` is, as the request's own headers say: bytes on the way, or
    /// nothing at all.
    ///
    /// Read off the headers rather than off the stream, there being no way to
    /// ask a stream whether it is empty without reading it — which is the one
    /// thing this must not do.
    fn of(headers: &HeaderMap, body: Body) -> Streamed {
        if !sends_bytes(headers) {
            return Streamed::Nothing;
        }

        Streamed::Bytes(Arc::new(TheOneStream(Mutex::new(Box::pin(
            body.into_data_stream(),
        )))))
    }

    /// The body for one attempt at one address.
    ///
    /// Every attempt gets a handle on the one stream rather than a copy of it,
    /// there being no copying a body: see this type's own documentation for
    /// what keeps a second attempt from finding it half read.
    pub(crate) fn for_one_attempt(&self) -> Option<reqwest::Body> {
        match self {
            Streamed::Nothing => None,
            Streamed::Bytes(held) => Some(reqwest::Body::wrap_stream(OneAttempt(Arc::clone(held)))),
        }
    }
}

/// Whether the browser is sending anything: a `Transfer-Encoding`, or a
/// `Content-Length` that is not nought.
fn sends_bytes(headers: &HeaderMap) -> bool {
    if headers.contains_key(TRANSFER_ENCODING) {
        return true;
    }

    headers
        .get(CONTENT_LENGTH)
        .and_then(|length| length.to_str().ok())
        .and_then(|length| length.parse::<u64>().ok())
        .is_some_and(|length| length > 0)
}

/// The browser's body as something to read: the shape it is boxed into so that
/// one attempt at a time can be handed it.
type Reading = Pin<Box<dyn Stream<Item = Result<Bytes, axum::Error>> + Send>>;

/// The one stream, held where every attempt can reach it.
///
/// A lock rather than an owner, because a request builder owns the body it is
/// given and there is one body between however many addresses a dial works
/// down.
pub(crate) struct TheOneStream(Mutex<Reading>);

/// And one attempt's handle on it, which is a stream in its own right: whatever
/// is read through this is read off the one below.
///
/// Locked per poll and never held across an await, there being no await here to
/// hold it across: a dial is one attempt at a time, so the lock is uncontended
/// and is about ownership rather than about two readers.
struct OneAttempt(Arc<TheOneStream>);

impl Stream for OneAttempt {
    type Item = Result<Bytes, axum::Error>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0
            .0
            .lock()
            .expect("nothing panics holding this")
            .as_mut()
            .poll_next(context)
    }
}

/// A name for a header this hop keeps back, for a test to name one by.
#[cfg(test)]
fn kept_back(name: &str) -> bool {
    KEPT_BACK.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path and the query cross the hop as they were written, and the
    /// Device Id comes off the front.
    #[test]
    fn a_call_is_addressed_at_the_far_ends_own_path() {
        assert_eq!(
            addressed(&"/api/ui/members/abc123/conversations".parse().unwrap()),
            Some(("abc123".to_owned(), "/api/ui/conversations".to_owned())),
        );

        assert_eq!(
            addressed(
                &"/api/ui/members/abc123/conversations/4/timeline?after=7&archived=1"
                    .parse()
                    .unwrap(),
            ),
            Some((
                "abc123".to_owned(),
                "/api/ui/conversations/4/timeline?after=7&archived=1".to_owned(),
            )),
        );
    }

    /// And an escape in the path is still an escape on the far end: an
    /// attachment named with a separator in it is the shape this is for.
    #[test]
    fn what_was_escaped_stays_escaped() {
        assert_eq!(
            addressed(
                &"/api/ui/members/abc123/conversations/4/attachments/one%2Ftwo.txt"
                    .parse()
                    .unwrap(),
            ),
            Some((
                "abc123".to_owned(),
                "/api/ui/conversations/4/attachments/one%2Ftwo.txt".to_owned(),
            )),
        );
    }

    /// The Workbench Key's cookie is not among the headers that travel, whatever
    /// else is.
    #[test]
    fn this_devices_cookie_does_not_travel() {
        let mut headers = HeaderMap::new();

        headers.insert("cookie", HeaderValue::from_static("workbench_key=secret"));
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("content-length", HeaderValue::from_static("12"));

        let forwarded = forwarded(&headers);

        assert!(kept_back("cookie"), "the cookie is one of the kept back");
        assert!(
            !forwarded.contains_key("cookie"),
            "this device's Workbench Key must not reach a member",
        );
        assert!(
            !forwarded.contains_key("content-length"),
            "the dial frames the body itself",
        );
        assert_eq!(
            forwarded.get("content-type").unwrap(),
            "application/json",
            "and everything that is the call's own is carried",
        );
    }

    /// What says there is a body to stream, and what says there is not.
    #[test]
    fn a_body_is_read_off_the_headers() {
        let mut none = HeaderMap::new();
        assert!(!sends_bytes(&none), "a bare GET sends nothing");

        none.insert("content-length", HeaderValue::from_static("0"));
        assert!(!sends_bytes(&none), "and nor does a press with no body");

        let mut some = HeaderMap::new();
        some.insert("content-length", HeaderValue::from_static("32000001"));
        assert!(sends_bytes(&some), "an attachment does");

        let mut chunked = HeaderMap::new();
        chunked.insert("transfer-encoding", HeaderValue::from_static("chunked"));
        assert!(
            sends_bytes(&chunked),
            "and so does a body of no known length"
        );
    }
}
