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
//!
//! **And the news comes the other way over the same link.** A page that reads a
//! member through this hop would draw it once and go stale, nothing over here
//! having heard that anything moved on that machine — so this device holds a
//! Nudge stream to each of its members and announces what comes down one
//! locally, under the Device Id it came from. That is [`freshness`].
//!
//! **And so do the reads that news sets going**, which are the other calls here
//! no browser asked for: a member's Conversations for the merged sidebar, and
//! its Agent Profiles for the mirror rows. Both are one shape — a dial, a
//! status, a bound, and a line in the log rather than a failure — so they go
//! through [`read_of`] rather than being written twice.
//!
//! **And the presses nobody asked for, the same way.** Some of what crosses a
//! link is this device writing to a member rather than reading it: a Rank minted
//! on the machine that merged the lists, and an edit or a removal over a
//! **mirror** put to the device that Agent Profile is at home on. Those go
//! through [`put_to`] and [`word_from`], which are [`read_of`]'s counterparts —
//! and what comes back is the far end's own answer, so a name already taken over
//! there is said in the words a local clash is said in. A machine that did not
//! take the press is named in a [`Refusal`] the page draws under the control that
//! was pressed.

pub(crate) mod bridging;
pub(crate) mod freshness;

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
use crate::device::{Devices, Unrelayed};
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

    /// A body this device is sending of its own accord: bytes it has in hand
    /// rather than a browser's stream.
    ///
    /// The one call in this tree that puts something *to* a member without a
    /// browser behind it — a rank written to the device that owns a row, see
    /// [`crate::ranking`] — and it is a couple of dozen bytes. So it is held
    /// rather than streamed, which the type below is built for anyway: a stream
    /// of one chunk is read by whichever attempt gets far enough to ask.
    pub(crate) fn saying(body: Vec<u8>) -> Streamed {
        Streamed::Bytes(Arc::new(TheOneStream(Mutex::new(Box::pin(
            tokio_stream::iter([Ok(Bytes::from(body))]),
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

/// One read this device makes of a member **of its own accord**: what it said,
/// up to `most` bytes of it, or nothing and a line in the log saying why what
/// this device last held stands.
///
/// **The calls nobody asked for.** Everything above is a browser's request
/// carried across; this is the other kind — the sidebar's account of a member's
/// Conversations, and the mirrors of a member's Agent Profiles — so it carries
/// no header of a browser's and no body at all. Both readers want the same four
/// things of it: the dial, the status, a bound on what is read, and a sentence
/// in the log rather than a failure, so it is written once here rather than
/// twice beside each of them.
///
/// `about` is what the read was for, in a word, so that two of them are told
/// apart in the log. `most` is the caller's own bound, because what a long
/// answer means differs: see [`crate::merging::MOST_A_LIST_IS`] and
/// [`crate::mirroring::MOST_THE_PROFILES_ARE`].
pub(crate) async fn read_of(
    devices: &crate::device::Devices,
    device: &str,
    onwards: String,
    about: &str,
    most: usize,
) -> Option<Vec<u8>> {
    let answered = match devices.relay(device, asking(onwards)).await {
        Ok(answered) => answered,
        Err(why) => {
            tracing::debug!(
                device,
                "a member's {about} could not be read, so what it last said stands: {}",
                unrelayed(&why),
            );
            return None;
        }
    };

    if !answered.status().is_success() {
        tracing::warn!(
            device,
            status = %answered.status(),
            "a member refused the read of its {about}, so what it last said stands",
        );
        return None;
    }

    match bounded(answered, most).await {
        Ok(body) => Some(body),
        Err(why) => {
            tracing::warn!(
                device,
                "a member's {about} could not be read to the end: {why:#}"
            );
            None
        }
    }
}

/// The call one of those is made with: a `GET` of `onwards` asking for JSON, on
/// nobody's behalf.
///
/// Shared with the readings that want a **word** back rather than a body to log
/// about — see [`word_from`], and [`crate::matching`], which asks a member for
/// its Repos this way and is refused by name where the machine is not there.
pub(crate) fn asking(onwards: String) -> Call {
    let mut headers = HeaderMap::new();

    headers.insert(
        axum::http::header::ACCEPT,
        HeaderValue::from_static("application/json"),
    );

    Call {
        method: reqwest::Method::GET,
        onwards,
        headers,
        body: Streamed::Nothing,
    }
}

/// What a member answered, up to `most` of it.
///
/// Read chunk by chunk rather than in one call, so the bound is applied as the
/// bytes arrive: a body read whole and then measured is a body this device has
/// already held.
async fn bounded(answered: reqwest::Response, most: usize) -> anyhow::Result<Vec<u8>> {
    use tokio_stream::StreamExt;

    let mut body = answered.bytes_stream();
    let mut held: Vec<u8> = Vec::new();

    while let Some(chunk) = body.next().await {
        held.extend_from_slice(&chunk?);

        if held.len() > most {
            anyhow::bail!("it is longer than the {most} bytes this device will hold of one");
        }
    }

    Ok(held)
}

/// What a dial that was never made is said in the log as: the four findings, in
/// as many words.
///
/// The browser's own sentences for the same four are [`refused`]'s — each
/// carries a status code and names the device, those being an answer to a call
/// somebody made. This is the other reader: a line about a read nobody asked
/// for.
fn unrelayed(why: &Unrelayed) -> String {
    match why {
        Unrelayed::ThisDevice => "the id is this device's own".to_owned(),
        Unrelayed::NoSuchMember => "the device is no member of this cluster".to_owned(),
        Unrelayed::Unreachable(why) => format!("{why:#}"),
        Unrelayed::Unreadable(why) => {
            format!("the devices this one is linked to could not be read: {why:#}")
        }
    }
}

/// The headers a press this device composes of its own accord carries: JSON going
/// out, and JSON expected back.
///
/// Written once here rather than beside each of the presses, a `Content-Type` and
/// an `Accept` being the whole of what any of them sends. The browser's own
/// headers go through [`forwarded`] instead, this being a call no browser made.
pub(crate) fn as_json() -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        axum::http::header::ACCEPT,
        HeaderValue::from_static("application/json"),
    );

    headers
}

/// Why a **press** this device put to one of its members was not made, in the
/// shape the page draws under the control that was pressed.
///
/// A status and a sentence, because both are the page's: the status is what the
/// viewer's fetch reads a refusal off, and the sentence is what it shows —
/// naming the machine by the name the human gave it rather than by sixteen bytes
/// of hex. See [`called`].
///
/// **Its own two answers rather than an [`Unrelayed`] handed on**, because the
/// four findings of a dial are not four things to tell a human: a member that is
/// switched off and one that has stopped being a member both come to *that
/// machine did not take it*, and something wrong on this side of the link is a
/// different sentence under a different status.
///
/// **Public, and re-exported at the crate root** — see [`crate::Refusal`]. It is
/// what a reading across the link answers with when there was no answer to be
/// had, and [`crate::matching`]'s is one a suite standing two devices up reads
/// the sentence of: *refused naming the machine* and *nothing over there matched*
/// are two different things to say, and a test that could not tell them apart
/// would be no test of that.
#[derive(Debug)]
pub struct Refusal {
    pub status: StatusCode,
    pub saying: String,
}

impl Refusal {
    /// This device's own trouble: a store it could not read, a membership it
    /// could not read, an identity it does not have.
    pub(crate) fn ours(saying: String) -> Refusal {
        Refusal {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            saying,
        }
    }

    /// And a device on the other side of a link that did not answer, or
    /// answered a refusal.
    pub(crate) fn theirs(saying: String) -> Refusal {
        Refusal {
            status: StatusCode::BAD_GATEWAY,
            saying,
        }
    }

    /// Whether this is the far end's trouble rather than this device's.
    ///
    /// The one thing a caller ever asks of a refusal beyond its sentence, and it
    /// is asked by the transfer preflight: a device that did not answer is
    /// *unreachable* and is the whole of what there is to say about it, where
    /// something wrong on this side is a failure to report rather than a finding
    /// about that machine. Read off the status because that is where the two are
    /// already told apart — see [`Refusal::ours`] and [`Refusal::theirs`].
    pub fn from_them(&self) -> bool {
        self.status == StatusCode::BAD_GATEWAY
    }
}

/// One **press** this device puts to a member of its own accord, and what it
/// answered — or the sentence naming the machine that did not take it.
///
/// [`read_of`]'s counterpart in the other direction, and the calls are the same
/// kind of call: nobody's browser asked for either. A Rank is written to the
/// device that owns the row it belongs to — see [`crate::ranking`] — and an edit
/// or a removal over a **mirror** is put to the device that Agent Profile is at
/// home on, see [`crate::mirroring`]. One function rather than the arms written
/// out twice, the four findings of a dial reading the same either way.
///
/// **A refusal from the far end is a refusal here** rather than an answer handed
/// back for the caller to read a status off. What the human has to be told about
/// a press their own hand made is *that machine did not take it*, and which of
/// the statuses it was belongs in the sentence rather than in the shape.
///
/// **The cluster handle rather than the whole state**, because that is all any of
/// this wants: the dial is the handle's, and so is the membership the machine is
/// named off. `None` is a Verkstead stood up without a Data Directory, which
/// invented no identity and is linked to nothing — its own trouble, and the one
/// arm here that is not about the far end. Callers holding an [`AppState`] pass
/// `state.devices.as_ref()`; [`crate::matching`] is asked with a handle and
/// nothing else.
pub(crate) async fn put_to(
    devices: Option<&Devices>,
    device: &str,
    call: Call,
) -> Result<reqwest::Response, Refusal> {
    let Some(devices) = devices else {
        return Err(Refusal::ours(
            "this server holds no device identity to relay through".to_owned(),
        ));
    };

    let named = called(Some(devices), device).await;

    match devices.relay(device, call).await {
        Ok(answered) if answered.status().is_success() => Ok(answered),

        Ok(answered) => Err(Refusal::theirs(format!(
            "{named} refused it: {}",
            answered.status(),
        ))),

        Err(Unrelayed::Unreachable(_)) => {
            Err(Refusal::theirs(format!("{named} could not be reached")))
        }

        Err(Unrelayed::NoSuchMember) => Err(Refusal::theirs(format!(
            "{named} is no longer one of this device's members",
        ))),

        Err(Unrelayed::ThisDevice) => Err(Refusal::ours(
            "something of this device's own was addressed as a member's".to_owned(),
        )),

        Err(Unrelayed::Unreadable(why)) => Err(Refusal::ours(format!(
            "the devices this one is linked to could not be read: {why:#}",
        ))),
    }
}

/// And one whose answer is a **word** rather than a status: what the far end said
/// it did, read back into the very type a browser pressing over there would have
/// received.
///
/// Which is the whole of how a press relayed home answers in the home device's
/// own vocabulary. A name already taken on that machine comes back as the word
/// its own store refused with, so the page says it in the words a local clash is
/// said in and this device composes no second opinion about somebody else's
/// rows.
///
/// `most` is the caller's bound, applied as [`read_of`]'s is: an outcome is one
/// short word, and what a bound is for is the machine that answers and then
/// writes without stopping.
pub(crate) async fn word_from<T: serde::de::DeserializeOwned>(
    devices: Option<&Devices>,
    device: &str,
    call: Call,
    most: usize,
) -> Result<T, Refusal> {
    let answered = put_to(devices, device, call).await?;

    let body = match bounded(answered, most).await {
        Ok(body) => body,
        Err(why) => return Err(unreadable(devices, device, format!("{why:#}")).await),
    };

    match serde_json::from_slice(&body) {
        Ok(said) => Ok(said),
        Err(why) => Err(unreadable(devices, device, why.to_string()).await),
    }
}

/// A member that answered something this device cannot make a word of: the far
/// end's trouble rather than this one's, so the machine is named.
///
/// The name is read here rather than carried down from [`put_to`] because this is
/// the one path that would need it twice, and a membership read on the way to a
/// sentence nobody usually sees is cheaper than a second one on every press.
async fn unreadable(devices: Option<&Devices>, device: &str, why: String) -> Refusal {
    Refusal::theirs(format!(
        "{} answered in a way this device cannot read: {why}",
        called(devices, device).await,
    ))
}

/// What a device is called, for the sentence a [`Refusal`] carries.
///
/// The name off the membership rather than the Device Id, because the id is
/// sixteen bytes of hex and the human named the machine. The id where there is
/// no name to be had, which is a membership that could not be read at the moment
/// something else about it went wrong.
pub(crate) async fn called(devices: Option<&Devices>, device: &str) -> String {
    let Some(devices) = devices else {
        return device.to_owned();
    };

    let Ok(members) = devices.membership().rows().await else {
        return device.to_owned();
    };

    members
        .into_iter()
        .find(|member| member.device == device)
        .map_or_else(|| device.to_owned(), |member| member.name)
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
