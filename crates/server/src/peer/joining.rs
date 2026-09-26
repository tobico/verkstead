//! The join, as the device being asked answers it: the post a stranger makes,
//! the ten minutes the question is held for, the human it raises a modal in
//! front of, and the cancel that takes it back (ADR-0020, *The join*).
//!
//! **This is the second of the three routes outside the member gate**, and the
//! one the whole arrangement was built around — see [`super::router`]. A join
//! arrives from a non-member, which is what a join *is*: the membership it is
//! asking for is the thing it has not got. A handshake that refused every
//! stranger would be a handshake no link could ever be made through, which is
//! why the verifier takes whatever arrives and the gate is per route.
//!
//! **And it is not un-authenticated for standing outside it.** The certificate
//! the handshake took is pinned into the request this post creates, and
//! everything that follows is matched against it: the dial back that answers an
//! Allow has to meet that certificate, and the cancel below has to be made under
//! it. What the post is trusted for is nothing at all — it writes down a
//! question for a human to answer, and a human pressing Allow is the whole of
//! what a join ever gets past.
//!
//! **Ten minutes, counted from when the request was made.** Written down as the
//! moment it runs out at rather than as a length, so a restart inside those ten
//! minutes is a question still being held rather than ten fresh minutes — the
//! human settled that the request survives a restart at all, and a clock that
//! began again at each start would be the wrong half of keeping it.
//!
//! **A call naming a request that has run out is refused in the same words as
//! one naming a request that was never there.** Which is what lets the expired
//! rows be swept whenever a fresh one arrives: what a caller is told cannot
//! depend on whether the housekeeping has been round yet.
//!
//! **And the human is told twice over, in the two places a human is.** Every
//! open workbench of this device is Nudged that the joins moved and reads them
//! back, which is what raises the modal; every phone subscribed to it gets a
//! push titled for the device asking. Both happen behind the row rather than in
//! front of it and neither can fail the post — a browser nobody has open costs
//! nothing, and a push service that cannot be reached costs a notification and
//! never the request. What settles the question is [`crate::device::Devices`],
//! over on the workbench where the press is.

use std::time::Duration;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sqlx::SqlitePool;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use verkstead_render::{AskingDevice, DeviceIdentity, JoinHeld, PendingJoin};
use verkstead_schema::Nudge;
use verkstead_store::{AskedJoin, HeldJoin};

use crate::device::Device;
use crate::device::reading::Reading;

use super::Caller;

/// Where a device asks to be let into this one's cluster.
///
/// Under `/api/peer/` with the identity endpoint, because it is this listener's
/// own business rather than any of the workbench traffic a member will come to
/// be served over it.
pub const JOIN: &str = "/api/peer/v1/join";

/// And where it takes that question back, which is Cancel on the asking
/// device's pending row.
///
/// The request in the path rather than in a body, because it is what is being
/// acted on. Matched against the certificate the request is holding, so it is
/// the asker's to cancel and nobody else's — the id need not be a secret, and
/// is not treated as one.
pub const CANCEL: &str = "/api/peer/v1/join/{request}/cancel";

/// That same path with a request in it, which is what a dial goes out to.
pub fn cancelling(request: &str) -> String {
    format!("/api/peer/v1/join/{request}/cancel")
}

/// How long a device holds a question its human has not answered.
///
/// Ten minutes, which is the ADR's: long enough to walk to the other machine,
/// and short enough that a request nobody ever saw is not still sitting there
/// tomorrow. What expires is the *question*, so a human who was not at the
/// screen presses Add again rather than finding a link they no longer remember
/// asking for.
pub const HELD: Duration = Duration::from_secs(10 * 60);

/// How many unanswered questions this device will hold at once.
///
/// **Because this is the one route a stranger reaches that writes.** Everything
/// a held request costs — a row, a push to this human's phones, a Nudge to every
/// open workbench, and a task that wakes ten minutes later to dial the addresses
/// the post named — is spent on the say-so of a device that has proved nothing
/// but that it holds the key to the certificate it presented. Without a ceiling,
/// how much of it is spent is the caller's to decide, and anybody who can reach
/// this port can decide it.
///
/// Sixteen, which is more devices than a human owns and nowhere near enough to
/// be worth anybody's while. A post that would go past it is refused rather than
/// queued, and the ones that have run out are swept first — so what fills this is
/// sixteen questions somebody is really being asked right now.
///
/// **What it does not do is keep the slots apart.** A device that filled all
/// sixteen would hold a real join out for the ten minutes until they expire, and
/// that is the trade this number makes: a ceiling nobody can get past costs the
/// machine nothing, and one per caller would key the accounting on a certificate
/// a stranger mints for free.
pub const HELD_AT_ONCE: usize = 16;

/// And how much of what a stranger says about itself this device will keep.
///
/// **A join is a payload from a non-member, and the row it writes is the list a
/// dial back later works down.** So the addresses are the one of these that is
/// more than storage: a post naming tens of thousands of them is a task that,
/// ten minutes later, spends a dial's patience apiece connecting to hosts and
/// ports the caller chose — with nobody having pressed anything, that walk being
/// the expiry telling the asker its request ran out. See
/// [`super::dialling::Peers::settle`], which is what works the list down.
///
/// Every one of them is generous against what a Verkstead really says: the id is
/// thirty-two hex characters, the OS word is one of four, a name is a hostname,
/// and no machine advertises sixteen addresses. A join saying more than this is
/// refused whole rather than trimmed — a name cut in half is a row nobody can
/// read, and a well-behaved device never comes near any of them.
const LONGEST_ID: usize = 64;
const LONGEST_NAME: usize = 255;
const LONGEST_OS: usize = 64;
const LONGEST_ADDRESS: usize = 255;
const MOST_ADDRESSES: usize = 16;

/// The device being asked, as the two routes above answer out of it: what to
/// say about itself, and where to keep the question.
#[derive(Debug, Clone)]
pub(crate) struct Holding {
    /// What this device is — the id and the certificate off the disk, which is
    /// what the answer to a join carries so the asker knows who it reached.
    pub(crate) device: Device,

    /// And the machine it is on, read at the moment it answers — see
    /// [`crate::device::reading`].
    pub(crate) reading: Reading,

    /// And where the question is written down.
    pub(crate) joins: Joins,

    /// And who to tell when it lands, which is every workbench of this device
    /// that happens to be open — see [`crate::nudge`].
    ///
    /// **The one handle that crosses from this listener to the other one.** A
    /// join arrives here and the modal it raises is drawn over there, so the
    /// stream the pages are listening on is made once at the start and given to
    /// both routers. Nothing is carried on it but the word that the joins moved:
    /// the page reads them back, as it does for every other kind.
    pub(crate) nudges: crate::nudge::Nudges,

    /// And how this device dials another, which is what an expiry is told over.
    ///
    /// **Here because the one thing that happens to a question without anybody
    /// pressing anything is that it runs out**, and the device that asked is
    /// owed that: without it a pending row reads *waiting* until somebody over
    /// there gets bored. The two presses are dialled back from the workbench
    /// side — see [`crate::device::Devices`] — and this is the third way a
    /// question ends, which is the one nobody is standing in front of.
    pub(crate) peers: super::dialling::Peers,
}

/// The joins in flight, as the things that ask after them do: the post that
/// holds one, the cancel that takes one back, the Devices section drawing a
/// pending row for each this device is waiting on, and the modal drawing one for
/// each it has been asked.
///
/// A handle over the store rather than the pool itself, the way [`super::Members`]
/// is one and for its reasons: each caller asks its own question rather than
/// writing its own query, and a router that was never given a database says so
/// rather than pretending to hold a request it has nowhere to put.
#[derive(Debug, Clone)]
pub struct Joins {
    /// The rows this Verkstead keeps, where it keeps any. `None` is a router
    /// stood up without a store behind it — see [`Joins::none`].
    kept: Option<SqlitePool>,
}

impl Joins {
    /// The joins this Verkstead really keeps: the rows in `pool`.
    pub fn recorded(pool: SqlitePool) -> Joins {
        Joins { kept: Some(pool) }
    }

    /// And a device with nowhere to keep one.
    ///
    /// Which is not a device that refuses links: it is a router stood up without
    /// a database, which every suite about *which routes there are* builds and
    /// no running server is. A join posted to one is refused saying so, and the
    /// two reads answer nothing rather than failing — a pane drawing no pending
    /// rows is right about a device that is holding none.
    pub fn none() -> Joins {
        Joins { kept: None }
    }

    /// Where the rows are, or the refusal for a device that has nowhere to keep
    /// one.
    ///
    /// Reachable from outside this module for the one thing beside a row that a
    /// held join sets off: the push that tells this human's phones a device is
    /// asking, which goes out of the same database the subscriptions are in —
    /// see [`crate::push`].
    pub(crate) fn store(&self) -> Result<&SqlitePool> {
        self.kept
            .as_ref()
            .context("this server has no store to keep a join in")
    }

    /// Hold a question somebody has just asked of this device, having first let
    /// go of the ones that ran out.
    ///
    /// The sweep rides along with the write because this is the moment there is
    /// a reason for one: a device nobody ever asks has nothing to sweep, and one
    /// asked twice a year should not be keeping last year's question. It changes
    /// nothing any caller is told — see the module note.
    ///
    /// **And the ceiling is read between the two**, which is why it is here
    /// rather than in the handler: what is being asked is how many questions are
    /// still live, and that is a different number before the sweep and after it.
    /// `false` is this device already holding [`HELD_AT_ONCE`] of them — nothing
    /// is written, and the caller is refused rather than queued.
    pub(crate) async fn hold(&self, held: &HeldJoin, now: OffsetDateTime) -> Result<bool> {
        let pool = self.store()?;

        verkstead_store::let_go_of_expired_joins(pool, &stamp(now)?)
            .await
            .context("letting go of the joins whose ten minutes had run out")?;

        let holding = verkstead_store::held_join_count(pool)
            .await
            .context("counting the joins this device is already holding")?;

        if holding >= HELD_AT_ONCE {
            return Ok(false);
        }

        verkstead_store::hold_join(pool, held)
            .await
            .with_context(|| format!("holding the join device {} asked", held.device))
            .map(|()| true)
    }

    /// One of them, where it is there and its ten minutes have not run out.
    ///
    /// **The two ways of not having it are one answer**, which is the whole of
    /// why the expiry is read here rather than by each caller: a request that
    /// was never made and one that has run out are both a request there is
    /// nothing to do about, and a caller that could tell them apart would be one
    /// that could ask this device what it had been asked.
    pub(crate) async fn held(
        &self,
        request: &str,
        now: OffsetDateTime,
    ) -> Result<Option<HeldJoin>> {
        let Some(held) = verkstead_store::held_join(self.store()?, request)
            .await
            .with_context(|| format!("reading the join held under {request}"))?
        else {
            return Ok(None);
        };

        Ok((!run_out(&held.expires_at, now)).then_some(held))
    }

    /// And one of them whether or not its ten minutes have run out, which is
    /// the one question the expiry itself asks.
    ///
    /// **Because what the ten minutes running out means is that nobody pressed
    /// anything**, and the only thing that says so is the row still being
    /// there: a request that was allowed, denied or cancelled was let go of at
    /// the press, and [`Joins::held`] cannot tell that from a request that ran
    /// out under the human's nose. So the timer reads it this way, and what it
    /// finds is what it tells the device that asked.
    pub(crate) async fn holding(&self, request: &str) -> Result<Option<HeldJoin>> {
        verkstead_store::held_join(self.store()?, request)
            .await
            .with_context(|| format!("reading the join held under {request}"))
    }

    /// And every one of them the human still has time to answer, as the modal
    /// draws it.
    ///
    /// **The ones that have run out are left out rather than drawn as expired**,
    /// which is the one place this list parts company with [`Joins::pending`]
    /// beside it. A pending row is somebody's own press and stays on the pane
    /// until they are done with it; a question nobody answered in ten minutes is
    /// a question there is nothing left to do about, and a modal is not
    /// something to be left holding two dead buttons. Which is also what takes
    /// the modal down when the ten minutes run out under it: the page reads this
    /// again and the request it was drawing is not in it.
    ///
    /// A device with nowhere to keep a join is being asked by nobody, which is
    /// an answer rather than a failure — the same shrug [`Joins::pending`]
    /// makes, for its reason.
    pub(crate) async fn asking(&self, now: OffsetDateTime) -> Result<Vec<AskingDevice>> {
        let Some(pool) = self.kept.as_ref() else {
            return Ok(Vec::new());
        };

        Ok(verkstead_store::held_joins(pool)
            .await
            .context("reading the joins this device is being asked")?
            .into_iter()
            .filter(|held| !run_out(&held.expires_at, now))
            .map(|held| AskingDevice {
                request: held.request,
                identity: DeviceIdentity {
                    device: held.device,
                    fingerprint: held.fingerprint,
                    name: held.name,
                    os: held.os,
                    addresses: held.addresses,
                },
            })
            .collect())
    }

    /// Let go of one, which is a cancel from the device that asked, and an Allow
    /// or a Deny pressed over here.
    pub(crate) async fn let_go(&self, request: &str) -> Result<()> {
        verkstead_store::let_go_of_join(self.store()?, request)
            .await
            .with_context(|| format!("letting go of the join held under {request}"))
    }

    /// Write down a join this device has just asked for.
    pub(crate) async fn ask(&self, asked: &AskedJoin) -> Result<()> {
        verkstead_store::ask_join(self.store()?, asked)
            .await
            .with_context(|| format!("writing down the join asked of {}", asked.address))
    }

    /// One of those, whether or not its ten minutes have run out.
    ///
    /// Unlike [`Joins::held`], because the two sides want different things of an
    /// expired request: the device holding one has nothing left to do about it,
    /// and the device that asked has a row to draw and a human to tell.
    pub(crate) async fn asked(&self, request: &str) -> Result<Option<AskedJoin>> {
        verkstead_store::asked_join(self.store()?, request)
            .await
            .with_context(|| format!("reading the join asked under {request}"))
    }

    /// Write down that the far end refused one of those, which is what a Deny
    /// dialled back comes to over here.
    ///
    /// The row stays: somebody pressed Add and is owed the answer, and the
    /// press that clears it is the same Dismiss an expired row carries — see
    /// [`verkstead_store::refuse_asked_join`].
    pub(crate) async fn refuse(&self, request: &str) -> Result<()> {
        verkstead_store::refuse_asked_join(self.store()?, request)
            .await
            .with_context(|| {
                format!("writing down that the join asked under {request} was refused")
            })
    }

    /// Take one off this device's own list, which is what Cancel and a dismissed
    /// expiry both come down to.
    pub(crate) async fn forget(&self, request: &str) -> Result<()> {
        verkstead_store::forget_asked_join(self.store()?, request)
            .await
            .with_context(|| format!("forgetting the join asked under {request}"))
    }

    /// And every one of them as the Devices section draws it.
    ///
    /// A device with nowhere to keep a join is waiting on none, which is an
    /// answer rather than a failure: the pane is drawing what this device is
    /// holding, and a router with no store is holding nothing.
    pub(crate) async fn pending(&self, now: OffsetDateTime) -> Result<Vec<PendingJoin>> {
        let Some(pool) = self.kept.as_ref() else {
            return Ok(Vec::new());
        };

        Ok(verkstead_store::asked_joins(pool)
            .await
            .context("reading the joins this device is waiting on")?
            .into_iter()
            .map(|asked| PendingJoin {
                expired: run_out(&asked.expires_at, now),
                refused: asked.refused,
                request: asked.request,
                address: asked.address,
                name: asked.name,
            })
            .collect())
    }
}

/// `POST /api/peer/v1/join` — a device asking to be let into this one's
/// cluster.
///
/// **What it takes is the asker saying what it is**: the id every record in a
/// cluster names it by, the fingerprint of the certificate it is presenting, the
/// name and OS word it is shown under, and every address it can be reached on —
/// which is the same thing it would answer a stranger with on the identity
/// endpoint, because it is the same thing said.
///
/// **What is pinned is the handshake's certificate rather than the one named**,
/// and the two are checked against each other first. A device that names one
/// certificate and presents another is not the device it says it is — the same
/// judgement a dial makes of an identity answer, made here of a join.
///
/// What comes back is the request's name, when this device lets go of it, and
/// what this device is: the asker draws a pending row from it, and its human
/// compares the fingerprint on that row with the one on the modal over here.
pub(crate) async fn join(
    State(holding): State<Holding>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    Json(saying): Json<DeviceIdentity>,
) -> Response {
    let Some(presented) = caller.fingerprint() else {
        return refused(
            StatusCode::FORBIDDEN,
            "a join has to be posted under the certificate the link would be pinned on, and \
             this one presented none",
        );
    };

    if saying.fingerprint != presented {
        return refused(
            StatusCode::FORBIDDEN,
            "this join names a certificate other than the one it was posted under, which is \
             a device that is not the one it says it is",
        );
    }

    if saying.device.trim().is_empty() {
        return refused(
            StatusCode::BAD_REQUEST,
            "a join has to say which device is asking",
        );
    }

    if let Some(why) = too_much(&saying) {
        tracing::warn!(
            fingerprint = %presented,
            why,
            "a device asking to link said more about itself than this one will keep, so the \
             request is refused",
        );

        return refused(StatusCode::BAD_REQUEST, why);
    }

    let now = OffsetDateTime::now_utc();

    let (asked_at, expires_at) = match (stamp(now), stamp(now + HELD)) {
        (Ok(asked_at), Ok(expires_at)) => (asked_at, expires_at),
        _ => return unreadable("this device could not say what the time is"),
    };

    let request = match crate::device::invented() {
        Ok(request) => request,
        Err(why) => {
            tracing::error!(%why, "a join could not be given a name to be answered under");

            return unreadable("this device could not name the request");
        }
    };

    let held = HeldJoin {
        request,
        device: saying.device,
        name: saying.name,
        os: saying.os,
        addresses: saying.addresses,
        fingerprint: presented,
        asked_at,
        expires_at: expires_at.clone(),
    };

    match holding.joins.hold(&held, now).await {
        Ok(true) => {}

        // The ceiling, which is what keeps a stranger from deciding how much of
        // this machine a join is worth — see [`HELD_AT_ONCE`]. Nothing is
        // written, so nothing is pushed, nothing is Nudged and no timer is set:
        // a refused post costs this device the read it took to refuse it.
        Ok(false) => {
            tracing::warn!(
                device = %held.device,
                fingerprint = %held.fingerprint,
                holding = HELD_AT_ONCE,
                "a device asked to link while this one was already holding as many \
                 questions as it will, so the request is refused",
            );

            return refused(
                StatusCode::TOO_MANY_REQUESTS,
                "this device is already holding as many join requests as it will, and lets \
                 go of each one ten minutes after it was made",
            );
        }

        Err(why) => {
            tracing::error!(%why, "a join could not be written down, so it is refused");

            return unreadable("this device could not write the request down");
        }
    }

    tracing::info!(
        device = %held.device,
        name = %held.name,
        fingerprint = %held.fingerprint,
        request = %held.request,
        "a device has asked to be let into this one's cluster, and the request is held \
         until it is answered or its ten minutes run out",
    );

    // And the human is asked, in the two places a human is: every workbench of
    // this device that is open, which raises the modal off the Nudge, and every
    // phone subscribed to it. Both after the row, and neither able to fail the
    // request — a push service that cannot be reached costs a notification, and
    // a browser nobody has open costs nothing at all.
    asked(&holding, held.request.clone());

    if let Ok(pool) = holding.joins.store() {
        crate::push::mentioned(
            pool,
            crate::push::Word::ADeviceIsAsking {
                name: held.name.clone(),
            },
        );
    }

    Json(JoinHeld {
        request: held.request,
        expires: expires_at,
        identity: holding.reading.identity(&holding.device).await,
    })
    .into_response()
}

/// `POST /api/peer/v1/join/{request}/cancel` — the device that asked, taking
/// its question back.
///
/// **Matched against the certificate the request is holding**, which is the
/// third thing that certificate is pinned for: the request is the asker's, and
/// a cancel from anybody else would be a stranger able to reach this port
/// deciding what this device's human gets to see.
///
/// A request that has run out is refused in the same words as one that was never
/// there, and neither is a failure to have asked about — see the module note.
pub(crate) async fn cancel(
    State(holding): State<Holding>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    Path(request): Path<String>,
) -> Response {
    let held = match holding
        .joins
        .held(&request, OffsetDateTime::now_utc())
        .await
    {
        Ok(held) => held,
        Err(why) => {
            tracing::error!(%why, "the joins this device is holding could not be read");

            return unreadable("this device could not read what it is holding");
        }
    };

    let Some(held) = held else {
        return refused(
            StatusCode::NOT_FOUND,
            "there is no such join request on this device",
        );
    };

    if caller.fingerprint().as_deref() != Some(held.fingerprint.as_str()) {
        return refused(
            StatusCode::FORBIDDEN,
            "that join was not asked under the certificate you presented",
        );
    }

    if let Err(why) = holding.joins.let_go(&request).await {
        tracing::error!(%why, "a cancelled join could not be let go of");

        return unreadable("this device could not let go of the request");
    }

    tracing::info!(
        device = %held.device,
        request = %request,
        "a device has taken back its request to join this one's cluster",
    );

    // And the modal it raised goes, wherever it is up: a question taken back is
    // one there is nothing left to answer.
    holding.nudges.announce(Nudge::Joins);

    StatusCode::NO_CONTENT.into_response()
}

/// Tell the open workbenches the joins moved, and set the one timer a request
/// is worth: the moment its ten minutes are up.
///
/// **Two announcements for one request, because a modal has to go by itself.**
/// The first raises it; the second is what takes it down where nobody pressed
/// anything, the page reading the list back and not finding the request in it —
/// see [`Joins::asking`], which is where a request that has run out stops being
/// answered.
///
/// **And the device that asked is told, which is the third way a join ends.**
/// The two presses are dialled back from the workbench side; this is the one
/// nobody is standing in front of, and without it a pending row over there
/// reads *waiting* long after there is anything to wait for. Told rather than
/// left to be worked out even so — the row's expiry is this device's own word
/// for the moment, written when the request was made, so what the call is worth
/// is the row redrawing as it happens.
///
/// A timer rather than a sweep on a schedule, because there is exactly one
/// moment worth waking for and this is the call that knows it. A restart inside
/// the ten minutes loses it and costs nothing that matters: every page reads the
/// world back whole when its stream comes back, a request that has run out is
/// not in what it reads, and the clock on the asking device's own row reaches
/// the same answer with nobody telling it.
fn asked(holding: &Holding, request: String) {
    holding.nudges.announce(Nudge::Joins);

    let holding = holding.clone();

    tokio::spawn(async move {
        tokio::time::sleep(HELD).await;

        // Still there is what says nobody pressed anything: an Allow, a Deny
        // and a cancel each let go of the row, and each has already dialled
        // whatever it had to dial.
        match holding.joins.holding(&request).await {
            Ok(Some(held)) => {
                if let Err(why) = holding
                    .peers
                    .settle(&held, &verkstead_render::JoinSettled::Expired)
                    .await
                {
                    tracing::info!(
                        %why,
                        device = %held.device,
                        %request,
                        "a device that asked to link could not be told its request ran \
                         out, which its own row says in ten minutes anyway",
                    );
                }
            }

            Ok(None) => {}

            Err(why) => tracing::warn!(
                %why,
                %request,
                "a request that was running out could not be looked up, so the device \
                 that asked is not told",
            ),
        }

        holding.nudges.announce(Nudge::Joins);
    });
}

/// Whether a join says more about itself than this device will keep, and what
/// it was.
///
/// **The one thing a stranger writes into this machine, read before any of it is
/// written.** What the row holds is what a dial back will later work down and
/// what a modal and a lock screen will later draw, so the bound is taken here
/// rather than at each of those: one reading, in front of the write, against
/// [`LONGEST_ID`] and the four beside it.
///
/// Refused whole rather than trimmed, and the reason is the addresses: a list cut
/// to sixteen would be a device quietly unreachable at the seventeenth address it
/// really has, where a refusal is a device saying plainly that it said too much.
/// A name or an OS word cut in half is the same kind of lie in smaller print.
///
/// Counted in characters rather than bytes, because what these are bounds on is
/// what somebody reads: a hostname in kanji is a hostname.
///
/// `None` is a join this device will keep, which is every join a Verkstead makes.
fn too_much(saying: &DeviceIdentity) -> Option<&'static str> {
    if saying.device.chars().count() > LONGEST_ID {
        return Some("that is longer than any device id");
    }

    if saying.name.chars().count() > LONGEST_NAME {
        return Some("that is longer than any hostname");
    }

    if saying.os.chars().count() > LONGEST_OS {
        return Some("that is longer than any word for an operating system");
    }

    if saying.addresses.len() > MOST_ADDRESSES {
        return Some("that is more addresses than a device is reachable at");
    }

    if saying
        .addresses
        .iter()
        .any(|address| address.chars().count() > LONGEST_ADDRESS)
    {
        return Some("that is longer than any address");
    }

    None
}

/// A moment in the spelling both sides of a join write one in: RFC 3339, UTC.
fn stamp(at: OffsetDateTime) -> Result<String> {
    at.format(&Rfc3339).context("saying what the time is")
}

/// Whether `expires_at` is behind `now`.
///
/// **A moment this build cannot read counts as run out.** The alternative is a
/// request held for ever on a row nothing can make sense of, and the cost of
/// being wrong is a human pressing Add again — which is what they would do about
/// an expiry anyway.
pub(crate) fn run_out(expires_at: &str, now: OffsetDateTime) -> bool {
    match OffsetDateTime::parse(expires_at, &Rfc3339) {
        Ok(expires) => expires <= now,
        Err(why) => {
            tracing::warn!(
                %why,
                %expires_at,
                "a join's expiry cannot be read, so the request counts as run out",
            );

            true
        }
    }
}

/// A refusal on this listener, said in the plain text the gate's own is said in:
/// there is no viewer at the far end of this port, only another Verkstead
/// putting what it was told into a line of its own log.
fn refused(status: StatusCode, why: &'static str) -> Response {
    (status, format!("{why}\n")).into_response()
}

/// And what this device answers when the failure is its own rather than the
/// caller's, with nothing about it in the answer: a peer can do nothing about
/// this machine's database, and the line that says which failure it was is in
/// this machine's log.
fn unreadable(why: &'static str) -> Response {
    refused(StatusCode::INTERNAL_SERVER_ERROR, why)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ten minutes are the ADR's, and they are what a request is held for
    /// rather than what a start counts from.
    #[test]
    fn a_request_is_held_ten_minutes() {
        assert_eq!(HELD, Duration::from_secs(600));
    }

    /// A moment behind now has run out, and one ahead has not — the comparison
    /// the whole expiry rests on.
    #[test]
    fn a_moment_behind_now_has_run_out() {
        let now = OffsetDateTime::now_utc();

        assert!(run_out(&stamp(now - HELD).unwrap(), now));
        assert!(!run_out(&stamp(now + HELD).unwrap(), now));
    }

    /// And a moment this build cannot read is one it will not go on waiting on.
    #[test]
    fn a_moment_that_cannot_be_read_has_run_out() {
        assert!(run_out("whenever", OffsetDateTime::now_utc()));
    }

    /// What a Verkstead really says about itself is nowhere near any of the
    /// bounds, which is the whole of what makes them safe to refuse on.
    #[test]
    fn a_join_a_verkstead_makes_is_well_inside_every_bound() {
        assert_eq!(
            too_much(&DeviceIdentity {
                device: "aa00bb11cc22dd33ee44ff5566778899".to_owned(),
                fingerprint: "3F:0A".to_owned(),
                name: "workbench".to_owned(),
                os: "Linux (WSL)".to_owned(),
                addresses: vec![
                    "workbench.tailnet-name.ts.net".to_owned(),
                    "100.64.0.1".to_owned(),
                    "192.168.1.24".to_owned(),
                ],
            }),
            None,
        );
    }

    /// And each of the five is refused on its own, saying which it was.
    #[test]
    fn a_join_that_says_too_much_is_refused_by_the_thing_it_said_too_much_of() {
        let fine = DeviceIdentity {
            device: "aa00bb11cc22dd33ee44ff5566778899".to_owned(),
            fingerprint: "3F:0A".to_owned(),
            name: "workbench".to_owned(),
            os: "Linux".to_owned(),
            addresses: vec!["192.168.1.24".to_owned()],
        };

        let said = |saying: DeviceIdentity| too_much(&saying).unwrap_or_default().to_owned();

        assert!(said(DeviceIdentity {
            device: "a".repeat(LONGEST_ID + 1),
            ..fine.clone()
        })
        .contains("device id"));

        assert!(said(DeviceIdentity {
            name: "a".repeat(LONGEST_NAME + 1),
            ..fine.clone()
        })
        .contains("hostname"));

        assert!(said(DeviceIdentity {
            os: "a".repeat(LONGEST_OS + 1),
            ..fine.clone()
        })
        .contains("operating system"));

        assert!(said(DeviceIdentity {
            addresses: vec!["192.168.1.24".to_owned(); MOST_ADDRESSES + 1],
            ..fine.clone()
        })
        .contains("more addresses"));

        assert!(said(DeviceIdentity {
            addresses: vec!["a".repeat(LONGEST_ADDRESS + 1)],
            ..fine
        })
        .contains("any address"));
    }

    /// The path a cancel is dialled at is the route that answers it, with the
    /// request in the one place the route takes one.
    #[test]
    fn a_cancel_goes_to_the_route_that_answers_it() {
        assert_eq!(
            CANCEL.replace("{request}", "0011223344556677"),
            cancelling("0011223344556677"),
        );
    }
}
