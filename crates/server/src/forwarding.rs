//! **Forwards**: a server started in a terminal on another device of the
//! cluster, reachable on this device's own `localhost` at the same port from
//! the moment this device's browser first opens the terminal's tab until the
//! server stops or the terminal ends. **Held, not attached** (ADR-0019,
//! *Forwards*, as amended): the tab closing keeps it.
//!
//! **The device whose browser opened the tab is the one that forwards.** That
//! is the hub bridging the attach, and the member offers the terminal to every
//! device that has attached it, from the first attach until the terminal ends —
//! see [`crate::terminals::Terminals::attach`] — so it is the only one the
//! member reads that terminal's ports to. A terminal this device's own
//! browser attached locally is no Forward at all: the port is on this machine
//! already, and there is nothing to cross.
//!
//! **Driven by the member's Nudge.** The hub already holds a Nudge stream to
//! each member and re-announces what comes down one under that device (see
//! [`crate::relaying::freshness`]); a `ports` Nudge among them is this module
//! reading that Conversation's ports again over the link and reconciling the
//! Forwards it holds against the reading — see [`reconciled`]. For every port
//! in it a listener is held on this device's loopback at the same number, IPv4
//! and IPv6 both, and each connection accepted on one is one upgrade over the
//! link to the member, joined byte for byte — see [`carried`].
//!
//! **And everything a member offers, read wherever that Nudge may have been
//! missed.** A `ports` Nudge is said once, so one said while the stream to its
//! member was down — a link that blipped, a lid shut, this server restarting —
//! is one nobody heard. So this device reads everything the member offers it,
//! every Conversation at once (see [`crate::terminals::offered_to`]), whenever
//! its stream to that member is taken up — which this server's own start does
//! for every member — when this device falls behind on its news, and when a
//! member a turn found not answering answers again; and reconciles every
//! Forward it holds on that member against it — see [`everything`]. Nothing is
//! persisted: the member is the record of what is still listening, and a
//! member that restarted, its terminals ended with it, answers with nothing.
//!
//! **A port this device cannot take is skipped and said.** Already bound here —
//! its own dev server, or another member's port of the same number already
//! forwarded — and the Forward is recorded as *skipped* with that reason and
//! tried again on every [`TURN`] while it stands. Last-one-wins and remapping
//! were both rejected: a `localhost:3000` that meant something different from
//! one minute to the next would be worse than one that plainly is not this.
//!
//! **A Forward ends** [`GRACE`] after it leaves the reading, so a dev server
//! restarting on a file change keeps it. Leaving the reading is every ending
//! there is: the port closing, the terminal ending. And it ends at once where
//! the member is no member at all, there being nothing to wait for.
//! Ending closes the listener, and the connections crossing it go with it.
//!
//! **A member not answering keeps its Forwards.** Asleep, off the network, the
//! internet gone for a while: each of its Forwards stands as *unreachable*, its
//! listener still bound — `localhost` at that port is still this Forward's and
//! nobody else's, and a connection made to it fails because the member cannot
//! be dialled. Once it answers, everything it offers is read again: a port
//! still offered is forwarding again, and one gone starts its grace.
//!
//! **And it ends by hand**, a × on its row: see [`dismiss`]. The dismissal is
//! remembered, so that the member's next reading does not take the Forward
//! again — until the port leaves the reading, or this device attaches the
//! terminal again, which is the plainest way there is of saying *I want this
//! after all* (see [`attached`]). Memory, like the Forwards themselves: this
//! server restarting forgets it, and a second press is the answer.
//!
//! **This device's own business.** The reading is [`forwards`], for this
//! device's own browser, and every change is a `forwards` Nudge — both held
//! back from the link the way the Remote access, Devices and push namespaces
//! are: a listener on this machine's `localhost` is a fact about this machine.
//! See [`crate::peer::workbench::KEPT_TO_ITSELF`].

use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::Instant;
use verkstead_render::{
    ForwardSkip, ForwardStanding, ForwardView, ForwardsView, OfferedView, PortsView, TerminalPorts,
};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::device::Devices;
use crate::relaying::{Call, Streamed, read_of};

/// Where a Forward listens unless a device was told otherwise: both loopbacks,
/// so that `localhost` reaches it whichever of the two a client resolves it to.
///
/// The loopback and nowhere else. A port in a member's terminal is offered to
/// the human at this machine, and a listener on every interface would be
/// offering it to everybody on this machine's network too.
pub(crate) const LOOPBACKS: [IpAddr; 2] = [
    IpAddr::V4(Ipv4Addr::LOCALHOST),
    IpAddr::V6(Ipv6Addr::LOCALHOST),
];

/// How long a Forward outlives its port leaving the reading — the file
/// watcher's grace: a dev server restarting on a file change closes its port
/// and opens it again, and that gap is a restart rather than the server
/// stopping. See [`crate::watchers::GRACE`].
const GRACE: Duration = crate::watchers::GRACE;

/// How often the Forwards are looked over: a grace that has run out, a skip to
/// try again, a member that has stopped answering or has come back.
///
/// A second, which is a look at a handful of entries in memory and, while there
/// are any Forwards or members a turn found not answering, a read of the
/// membership — and it is what a grace ending is late by at most.
const TURN: Duration = Duration::from_secs(1);

/// The most one member's reading of its ports may be: **64 KiB**. A terminal
/// number and a list of ports is a few dozen bytes, and every Conversation's a
/// few dozen of those; what this is for is the machine that answers and then
/// writes without stopping.
const MOST_A_READING_IS: usize = 64 * 1024;

/// How many connections a listener holds in its backlog — the number every
/// ordinary listener takes.
const BACKLOG: u32 = 1024;

/// The Forwards this device holds, by what each one is.
#[derive(Clone, Default)]
pub(crate) struct Forwards {
    held: Arc<Mutex<BTreeMap<Key, Forward>>>,

    /// The Forwards ended by hand that their members still offer, which a
    /// reading does not take again — see [`dismiss`]. Only ever locked after
    /// `held`, where both are.
    dismissed: Arc<Mutex<BTreeSet<Key>>>,
}

/// What one Forward is: a port of one terminal of one Conversation on one
/// member. Ordered by port first, which is the order the reading lists them in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    port: u16,
    device: String,
    conversation: i64,
    terminal: i64,
}

/// One Forward as it is held: listening or skipped, whether its member is
/// answering, and since when it has been missing from the member's reading,
/// where it is.
struct Forward {
    held: Held,

    /// Whether the last turn found its member not answering — what reads as
    /// *unreachable*, whatever is held.
    unreachable: bool,

    /// When the port was last seen leaving the reading — `None` while it is in
    /// it. What [`GRACE`] is counted from.
    missing: Option<Instant>,
}

enum Held {
    /// A listener on each loopback, carrying what it accepts — see
    /// [`Listening`], which is held for its dropping: nothing reads it.
    Forwarding { _listening: Listening },

    /// Nothing held, and why.
    Skipped(ForwardSkip),
}

/// The task that holds a Forward's listeners and every connection crossing
/// them. **Aborted as it is dropped**, which is the whole of ending a Forward:
/// the listeners close, and every connection joined through them goes with
/// them.
struct Listening(JoinHandle<()>);

impl Drop for Listening {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Forwards {
    /// None held, which is every start: what is forwarded is read back off the
    /// members as their streams are taken up — see [`everything`].
    pub(crate) fn new() -> Forwards {
        Forwards::default()
    }

    /// The register, locked. Never held across an await.
    fn held(&self) -> MutexGuard<'_, BTreeMap<Key, Forward>> {
        self.held.lock().expect("nothing panics holding this")
    }

    /// The dismissals, locked. Never held across an await, and only taken
    /// after [`Forwards::held`] where both are.
    fn dismissed(&self) -> MutexGuard<'_, BTreeSet<Key>> {
        self.dismissed.lock().expect("nothing panics holding this")
    }
}

/// Hold this device's Forwards for as long as the server runs.
///
/// Nothing at all on a router with no identity: a Verkstead stood up without a
/// Data Directory is linked to nothing, and has nobody's ports to forward.
pub(crate) fn holding(state: &AppState) {
    if state.devices.is_none() {
        return;
    }

    let state = state.clone();

    tokio::spawn(async move { held(state).await });
}

/// The loop itself: a member's `ports` Nudge reconciled as it lands, and the
/// Forwards looked over every [`TURN`] while there are any.
///
/// **Every member read at the start**, once the loop is listening: the streams
/// to them are taken up by a task spawned before this one, so the
/// [`Nudge::Everything`] the first of them says may have been said before
/// anything here was listening for it.
async fn held(state: AppState) {
    let mut moved = state.nudges.subscribe();
    let mut turning = tokio::time::interval(TURN);

    read_everywhere(&state);

    // The members [`turned`] last found not answering, so that it can tell one
    // that has come back.
    let mut away = BTreeSet::new();

    turning.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            moved = moved.recv() => match moved {
                Ok(moved) => {
                    // This device's own news is never about a member's ports:
                    // a terminal here attached from here forwards nothing.
                    let Some(device) = moved.device else { continue };

                    match moved.moved {
                        Nudge::Ports { conversation } => {
                            reconciled(&state, &device, conversation).await;
                        }

                        // A stream taken up, which says nothing about what it
                        // missed: everything that member offers is read again.
                        Nudge::Everything => everything(&state, &device).await,

                        _ => {}
                    }
                }

                // Fallen behind, so everything every member offers is read
                // again: what was missed is unknowable.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    read_everywhere(&state);
                }

                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            },

            _ = turning.tick() => {
                // A member answering again after a turn that found it not: what
                // it offers may have moved while nobody could hear it say so.
                for device in turned(&state, &mut away).await {
                    everything(&state, &device).await;
                }
            }
        }
    }
}

/// Every member this device holds a Forward on.
fn holding_on(state: &AppState) -> BTreeSet<String> {
    state
        .forwards
        .held()
        .keys()
        .map(|key| key.device.clone())
        .collect()
}

/// Read everything every member offers, in a task of its own: a member switched
/// off is a dial that takes its time to fail, and the loop is not to wait on it.
fn read_everywhere(state: &AppState) {
    let state = state.clone();

    tokio::spawn(async move {
        let Some(devices) = state.devices.as_ref() else {
            return;
        };

        let members = match devices.membership().rows().await {
            Ok(rows) => rows,
            Err(why) => {
                tracing::error!(
                    error = ?why,
                    "the devices this one is linked to could not be read, so what they offer \
                     to forward was not read back",
                );
                return;
            }
        };

        for member in members {
            everything(&state, &member.device).await;
        }
    });
}

/// Read `conversation`'s ports off `device` over the link, and make the
/// Forwards held for it match — see [`matched`].
async fn reconciled(state: &AppState, device: &str, conversation: i64) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let Some(reading) = read::<PortsView>(
        devices,
        device,
        format!("/api/ui/conversations/{conversation}/ports"),
        "ports",
    )
    .await
    else {
        return;
    };

    let offered = keys(device, conversation, &reading.terminals).collect();

    matched(state, devices, device, Some(conversation), offered);
}

/// Read everything `device` offers this one over the link, every Conversation
/// at once, and make every Forward held on it match — a Conversation it offers
/// nothing on any more having its Forwards start their grace with the rest.
async fn everything(state: &AppState, device: &str) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let Some(reading) =
        read::<OfferedView>(devices, device, "/api/ui/ports".to_owned(), "offered ports").await
    else {
        return;
    };

    let offered = reading
        .conversations
        .iter()
        .flat_map(|offered| keys(device, offered.conversation, &offered.terminals))
        .collect();

    matched(state, devices, device, None, offered);
}

/// One reading of a member, as what it says. A read that could not be made, or
/// not understood, is `None`: it changes nothing, and a member that has stopped
/// answering is [`turned`]'s to notice.
async fn read<T: serde::de::DeserializeOwned>(
    devices: &Devices,
    device: &str,
    onwards: String,
    about: &str,
) -> Option<T> {
    let said = read_of(devices, device, onwards, about, MOST_A_READING_IS).await?;

    match serde_json::from_slice(&said) {
        Ok(reading) => Some(reading),
        Err(why) => {
            tracing::warn!(
                device,
                "a member answered its {about} in a way this device cannot read, so what is \
                 forwarded for it stands: {why}",
            );
            None
        }
    }
}

/// The Forwards one Conversation's terminals on `device` offer.
fn keys<'a>(
    device: &'a str,
    conversation: i64,
    terminals: &'a [TerminalPorts],
) -> impl Iterator<Item = Key> + 'a {
    terminals.iter().flat_map(move |terminal| {
        terminal.ports.iter().map(move |&port| Key {
            port,
            device: device.to_owned(),
            conversation,
            terminal: terminal.number,
        })
    })
}

/// Make the Forwards held on `device` — of `conversation` alone, where one is
/// said — match what it `offered`.
///
/// A port offered and not held is taken, unless it was dismissed; one held and
/// still offered has its grace called off; one held and not offered starts its
/// grace, and is ended by [`turned`] once that has run out. And a dismissal of
/// a port no longer offered is forgotten, so that a server stopped and started
/// again is forwarded afresh.
fn matched(
    state: &AppState,
    devices: &Devices,
    device: &str,
    conversation: Option<i64>,
    offered: BTreeSet<Key>,
) {
    let within = |key: &Key| {
        key.device == device && conversation.is_none_or(|only| key.conversation == only)
    };

    let changed = {
        let mut forwards = state.forwards.held();
        let mut dismissed = state.forwards.dismissed();
        let now = Instant::now();
        let mut changed = false;

        dismissed.retain(|key| !within(key) || offered.contains(key));

        for (key, forward) in forwards.iter_mut() {
            if !within(key) {
                continue;
            }

            match (offered.contains(key), forward.missing) {
                (true, _) => forward.missing = None,
                (false, None) => forward.missing = Some(now),
                (false, Some(_)) => {}
            }
        }

        for key in offered {
            if forwards.contains_key(&key) || dismissed.contains(&key) {
                continue;
            }

            let held = taken(devices, &key);

            forwards.insert(
                key,
                Forward {
                    held,
                    unreachable: false,
                    missing: None,
                },
            );
            changed = true;
        }

        changed
    };

    if changed {
        state.nudges.announce_here(Nudge::Forwards);
    }
}

/// Look the Forwards over: end the ones whose grace has run out and the ones on
/// a device that is no member any more, mark the ones on a member that is not
/// answering as unreachable — and the ones on a member answering again as not —
/// and try again the ones that were skipped.
///
/// **And say which members have come back** — answering now, where they were
/// in `away` on the last turn — so that everything they offer is read again.
/// `away` is left holding the members this turn found not answering: the ones
/// it held Forwards on, and the ones already away, for as long as each is still
/// a member at all.
async fn turned(state: &AppState, away: &mut BTreeSet<String>) -> BTreeSet<String> {
    let looking = holding_on(state);

    if looking.is_empty() && away.is_empty() {
        return BTreeSet::new();
    }

    let Some(devices) = state.devices.as_ref() else {
        return BTreeSet::new();
    };

    // The members answering, read before the register is taken: a membership
    // that could not be read ends nothing, there being no saying who is gone.
    let rows = match devices.membership().rows().await {
        Ok(rows) => Some(rows),
        Err(why) => {
            tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so no Forward was ended \
                 for a member that has gone",
            );
            None
        }
    };

    let answering: Option<BTreeSet<String>> = rows.as_ref().map(|rows| {
        rows.iter()
            .filter(|member| member.reachable)
            .map(|member| member.device.clone())
            .collect()
    });

    let members: Option<BTreeSet<String>> = rows
        .as_ref()
        .map(|rows| rows.iter().map(|member| member.device.clone()).collect());

    let changed = {
        let mut forwards = state.forwards.held();
        let before = forwards.len();
        let now = Instant::now();

        forwards.retain(|key, forward| {
            let gone = members
                .as_ref()
                .is_some_and(|members| !members.contains(&key.device));
            let over = forward
                .missing
                .is_some_and(|since| now.duration_since(since) >= GRACE);

            if gone || over {
                tracing::info!(
                    port = key.port,
                    device = key.device,
                    conversation = key.conversation,
                    terminal = key.terminal,
                    "a Forward ended: {}",
                    match gone {
                        true => "its device is no member any more",
                        false => "its port has left the member's reading",
                    },
                );
            }

            !(gone || over)
        });

        let mut changed = forwards.len() != before;

        if let Some(answering) = answering.as_ref() {
            for (key, forward) in forwards.iter_mut() {
                let unreachable = !answering.contains(&key.device);

                if forward.unreachable != unreachable {
                    tracing::info!(
                        port = key.port,
                        device = key.device,
                        "a Forward's member is {}",
                        match unreachable {
                            true => "not answering, so it stands as unreachable",
                            false => "answering again",
                        },
                    );
                    forward.unreachable = unreachable;
                    changed = true;
                }
            }
        }

        // The order matters for the ones skipped as busy: a Forward that ended
        // above has let go of its port, and a skip of the same number is free to
        // take it on this very turn.
        for (key, forward) in forwards.iter_mut() {
            if let Held::Skipped(_) = forward.held {
                let held = taken(devices, key);

                if matches!(held, Held::Forwarding { .. }) {
                    changed = true;
                }

                forward.held = held;
            }
        }

        changed
    };

    if changed {
        state.nudges.announce_here(Nudge::Forwards);
    }

    let (Some(members), Some(answering)) = (members, answering) else {
        return BTreeSet::new();
    };

    let back = away
        .iter()
        .filter(|device| answering.contains(*device))
        .cloned()
        .collect();

    *away = looking
        .into_iter()
        .chain(std::mem::take(away))
        .filter(|device| members.contains(device) && !answering.contains(device))
        .collect();

    back
}

/// Take `key`'s port on every loopback this device forwards on, and start
/// carrying what arrives — or say why it could not be taken.
///
/// **All or nothing**, where the port is busy at any of them: a `localhost`
/// that reached the member's server over one family and this machine's own over
/// the other would be the worst of the answers. A loopback this machine does not
/// have — an IPv6 stack that is switched off — is left out rather than refused
/// for, so long as one of them was taken.
fn taken(devices: &Devices, key: &Key) -> Held {
    let mut listeners = Vec::new();
    let mut refused = None;

    for &loopback in devices.forwarding_loopbacks() {
        match bound(SocketAddr::new(loopback, key.port)) {
            Ok(listener) => listeners.push(listener),

            Err(why) if why.kind() == std::io::ErrorKind::AddrInUse => {
                return skipped(key, ForwardSkip::PortBusy);
            }

            Err(why) if why.kind() == std::io::ErrorKind::PermissionDenied => {
                return skipped(key, ForwardSkip::NotPermitted);
            }

            Err(why) => {
                tracing::debug!(
                    port = key.port,
                    %loopback,
                    "a Forward is not held on this loopback, which this machine would not \
                     listen on: {why}",
                );
                refused = Some(why);
            }
        }
    }

    if listeners.is_empty() {
        tracing::warn!(
            port = key.port,
            device = key.device,
            "a Forward could not be held on any loopback: {refused:?}",
        );
        return skipped(key, ForwardSkip::NotPermitted);
    }

    tracing::info!(
        port = key.port,
        device = key.device,
        conversation = key.conversation,
        terminal = key.terminal,
        "a Forward is being held on this device's localhost",
    );

    Held::Forwarding {
        _listening: listening(listeners, devices.clone(), key.clone()),
    }
}

/// The skip, said in the log once per try — at debug, there being one a turn
/// while it stands.
fn skipped(key: &Key, skip: ForwardSkip) -> Held {
    tracing::debug!(
        port = key.port,
        device = key.device,
        ?skip,
        "a Forward could not be held, and is tried again on the next turn",
    );

    Held::Skipped(skip)
}

/// One listener on `at`.
///
/// Built by hand rather than with [`TcpListener::bind`] for the one thing that
/// differs by platform: the address is made reusable on Linux, which is what
/// lets a Forward ended a moment ago be taken again while its last connections
/// sit out their `TIME_WAIT` — and nowhere else, because Linux alone keeps the
/// option to that. On a Mac it also lets a bind to `127.0.0.1` stand beside
/// somebody else's listener on every address at the same port, and on Windows
/// it lets a listener take a port somebody else is already listening on: both
/// would have a Forward take over this machine's own server, which is the one
/// thing a Forward must never do. A Forward taken again there within its
/// `TIME_WAIT` reads *port busy here* for a moment, and is taken on a later
/// turn.
fn bound(at: SocketAddr) -> std::io::Result<TcpListener> {
    let socket = match at {
        SocketAddr::V4(_) => TcpSocket::new_v4()?,
        SocketAddr::V6(_) => TcpSocket::new_v6()?,
    };

    #[cfg(target_os = "linux")]
    socket.set_reuseaddr(true)?;

    socket.bind(at)?;
    socket.listen(BACKLOG)
}

/// Hold `listeners`, carrying every connection made to any of them to the port
/// `key` names on its member.
///
/// **A set of tasks inside a task**, so that aborting the one — which is what
/// dropping the [`Listening`] does — drops the sets, and a dropped set aborts
/// everything in it: every listener closes and every connection crossing one
/// closes with it.
fn listening(listeners: Vec<TcpListener>, devices: Devices, key: Key) -> Listening {
    Listening(tokio::spawn(async move {
        let mut accepting = JoinSet::new();

        for listener in listeners {
            accepting.spawn(accepted(listener, devices.clone(), key.clone()));
        }

        while accepting.join_next().await.is_some() {}
    }))
}

/// Accept on one listener for as long as the Forward stands, each connection
/// carried in a task of its own.
async fn accepted(listener: TcpListener, devices: Devices, key: Key) {
    let mut crossing = JoinSet::new();

    loop {
        tokio::select! {
            taken = listener.accept() => match taken {
                Ok((stream, _)) => {
                    crossing.spawn(carried(stream, devices.clone(), key.clone()));
                }

                // Out of descriptors, most likely, which passes: the next accept
                // is tried a moment later rather than at once and for ever.
                Err(why) => {
                    tracing::warn!(port = key.port, "a Forward could not accept a connection: {why}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            },

            // Reaped as they finish, so a long-lived Forward is not a set that
            // only grows.
            Some(_) = crossing.join_next(), if !crossing.is_empty() => {}
        }
    }
}

/// One connection made to a Forward, carried to the member: one upgrade over the
/// link to the terminal's port, and the two joined byte for byte — see
/// [`crate::terminals::connect`], which is the far end, and
/// [`crate::relaying::bridging::crossing`], which is the join on both.
///
/// **A refusal ends the connection**, which is what a client of a port nothing
/// is listening on would have had anyway: the server in the terminal has gone
/// since the reading, or the member is not answering. Said in the log rather
/// than to anybody, there being nobody on a raw socket to say it to.
async fn carried(stream: TcpStream, devices: Devices, key: Key) {
    let answered = match devices.relay(&key.device, upgrading(&key)).await {
        Ok(answered) => answered,
        Err(_) => {
            tracing::debug!(
                port = key.port,
                device = key.device,
                "a forwarded connection could not reach its member, so it was ended",
            );
            return;
        }
    };

    if answered.status() != StatusCode::SWITCHING_PROTOCOLS {
        tracing::debug!(
            port = key.port,
            device = key.device,
            status = %answered.status(),
            "a member refused a forwarded connection, so it was ended",
        );
        return;
    }

    match answered.upgrade().await {
        Ok(far) => crate::relaying::bridging::crossing(stream, far).await,
        Err(why) => tracing::debug!(
            port = key.port,
            device = key.device,
            %why,
            "a member answered a forwarded connection and then would not hand it over",
        ),
    }
}

/// The upgrade one connection is carried over: the terminal's port on the
/// member, asked for as [`crate::terminals::FORWARD`].
fn upgrading(key: &Key) -> Call {
    let mut headers = HeaderMap::new();

    headers.insert(
        axum::http::header::CONNECTION,
        HeaderValue::from_static("upgrade"),
    );
    headers.insert(
        axum::http::header::UPGRADE,
        HeaderValue::from_static(crate::terminals::FORWARD),
    );

    Call {
        method: reqwest::Method::GET,
        onwards: format!(
            "/api/ui/conversations/{}/terminals/{}/ports/{}",
            key.conversation, key.terminal, key.port,
        ),
        headers,
        body: Streamed::Nothing,
    }
}

/// `GET /api/ui/forwards` — every Forward this device holds, for its own
/// browser: the port, the device it reaches with that device's name and OS, the
/// Conversation and its title, and whether it is forwarding, skipped and why, or
/// waiting on its member to answer.
///
/// **Never served over the link** — the prefix is one this device keeps to
/// itself, see [`crate::peer::workbench::KEPT_TO_ITSELF`].
pub(crate) async fn forwards(State(state): State<AppState>) -> Json<ForwardsView> {
    let held: Vec<(Key, ForwardStanding)> = state
        .forwards
        .held()
        .iter()
        .map(|(key, forward)| {
            let standing = match forward.held {
                _ if forward.unreachable => ForwardStanding::Unreachable,
                Held::Forwarding { .. } => ForwardStanding::Forwarding,
                Held::Skipped(reason) => ForwardStanding::Skipped { reason },
            };

            (key.clone(), standing)
        })
        .collect();

    let members = match state.devices.as_ref() {
        Some(devices) if !held.is_empty() => devices.membership().rows().await.unwrap_or_default(),
        _ => Vec::new(),
    };

    let forwards = held
        .into_iter()
        .map(|(key, standing)| {
            let member = members.iter().find(|member| member.device == key.device);

            ForwardView {
                port: key.port,
                name: member.map_or_else(|| key.device.clone(), |member| member.name.clone()),
                os: member.map(|member| member.os.clone()).unwrap_or_default(),
                title: state.merged.title_of(&key.device, key.conversation),
                device: key.device,
                conversation: key.conversation,
                terminal: key.terminal,
                standing,
            }
        })
        .collect();

    Json(ForwardsView { forwards })
}

/// `DELETE /api/ui/forwards/{device}/{conversation}/{terminal}/{port}` — the ×
/// on a Forward's row: end it on this device, and remember that, so that the
/// member's next reading does not take it again.
///
/// Ended the way every Forward ends — its listener closed and every connection
/// crossing it with it — whether it was forwarding, skipped or unreachable: a
/// skipped one stops being tried. Answered with nothing, and the same where the
/// Forward named is not one this device holds: it is already what the press
/// asked for, so there is nothing to refuse.
///
/// **Never served over the link**, as the reading is not — see
/// [`crate::peer::workbench::KEPT_TO_ITSELF`].
pub(crate) async fn dismiss(
    State(state): State<AppState>,
    Path((device, conversation, terminal, port)): Path<(String, i64, i64, u16)>,
) -> Response {
    let key = Key {
        port,
        device,
        conversation,
        terminal,
    };

    let ended = {
        let mut forwards = state.forwards.held();
        let ended = forwards.remove(&key);

        if ended.is_some() {
            state.forwards.dismissed().insert(key.clone());
        }

        ended
    };

    if ended.is_some() {
        tracing::info!(
            port = key.port,
            device = key.device,
            conversation = key.conversation,
            terminal = key.terminal,
            "a Forward ended: it was dismissed on this device",
        );

        state.nudges.announce_here(Nudge::Forwards);
    }

    StatusCode::NO_CONTENT.into_response()
}

/// This device's browser has just attached `onwards` on `device` through the
/// relay — where that is one of its terminals: what this device dismissed on
/// that terminal is forgotten, and its ports are read again, so that what it
/// is still listening on is forwarded afresh. Opening the tab is the way back
/// from a ×.
///
/// Read again here rather than left to the member's `ports` Nudge, which it
/// says on a device's first socket on the terminal and not on a second.
pub(crate) fn attached(state: &AppState, device: &str, onwards: &str) {
    let Some((conversation, terminal)) = attaching(onwards) else {
        return;
    };

    state.forwards.dismissed().retain(|key| {
        !(key.device == device && key.conversation == conversation && key.terminal == terminal)
    });

    let state = state.clone();
    let device = device.to_owned();

    tokio::spawn(async move { reconciled(&state, &device, conversation).await });
}

/// The Conversation and the terminal an attach is on, where `onwards` is one —
/// `/api/ui/conversations/{id}/terminals/{n}/attach`, its query aside.
fn attaching(onwards: &str) -> Option<(i64, i64)> {
    let path = onwards.split_once('?').map_or(onwards, |(path, _)| path);
    let rest = path.strip_prefix("/api/ui/conversations/")?;
    let (conversation, rest) = rest.split_once('/')?;
    let (terminal, rest) = rest.strip_prefix("terminals/")?.split_once('/')?;

    if rest != "attach" {
        return None;
    }

    Some((conversation.parse().ok()?, terminal.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::attaching;

    #[test]
    fn an_attach_is_read_off_the_path_it_was_relayed_to() {
        assert_eq!(
            attaching("/api/ui/conversations/7/terminals/2/attach"),
            Some((7, 2)),
        );
        assert_eq!(
            attaching("/api/ui/conversations/7/terminals/2/attach?cols=80"),
            Some((7, 2)),
        );
        assert_eq!(attaching("/api/ui/conversations/7/terminals/2"), None);
        assert_eq!(attaching("/api/ui/conversations/7/screen/attach"), None);
        assert_eq!(
            attaching("/api/ui/conversations/7/terminals/2/ports/3000"),
            None
        );
    }
}
