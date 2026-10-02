//! **Forwards**: a server started in a terminal on another device of the
//! cluster, reachable on this device's own `localhost` at the same port for as
//! long as the terminal's tab is open here.
//!
//! **The device whose browser has the tab open is the one that forwards.** That
//! is the hub bridging the attach, and it is the only device the member counts
//! as attached — see [`crate::terminals::Terminals::attach`] — so it is the only
//! one the member reads that terminal's ports to. A terminal this device's own
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
//! **And read again wherever that Nudge may have been missed.** A `ports` Nudge
//! is said once, so one said while the stream to its member was down — a link
//! that blipped, this server starting while a tab was open — is one nobody
//! heard. So this device counts the terminal attaches it relays to each member
//! (see [`Forwards::attached`]), and every Conversation of a member with one of
//! those or a Forward is read again when its stream is taken up, when this
//! device falls behind on its news, and when a member a turn found not
//! answering answers again.
//!
//! **A port this device cannot take is skipped and said.** Already bound here —
//! its own dev server, or another member's port of the same number already
//! forwarded — and the Forward is recorded as *skipped* with that reason and
//! tried again on every [`TURN`] while it stands. Last-one-wins and remapping
//! were both rejected: a `localhost:3000` that meant something different from
//! one minute to the next would be worse than one that plainly is not this.
//!
//! **A Forward ends** [`GRACE`] after it leaves the reading, so a pane swap or
//! a reload — an attach let go of and taken again — keeps it. Leaving the
//! reading is every ending there is: the attach it stands on ending, the port
//! closing, the terminal ending. And it ends at once where the member reads as
//! unreachable or is no member at all, there being nothing to connect to — and
//! is taken up again once it answers, while the tab is still open.
//! Ending closes the listener, and the connections crossing it go with it.
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
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::Instant;
use verkstead_render::{ForwardSkip, ForwardStanding, ForwardView, ForwardsView, PortsView};
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
/// watcher's grace, for the file watcher's reason: a pane swap or a reload is an
/// attach let go of and taken again, and that gap is a repaint rather than
/// anybody leaving. See [`crate::watchers::GRACE`].
const GRACE: Duration = crate::watchers::GRACE;

/// How often the Forwards are looked over: a grace that has run out, a skip to
/// try again, a member that has stopped answering or has come back.
///
/// A second, which is a look at a handful of entries in memory and, while there
/// are any Forwards or relayed attaches, a read of the membership — and it is
/// what a grace ending is late by at most.
const TURN: Duration = Duration::from_secs(1);

/// The most one member's reading of a Conversation's ports may be: **64 KiB**.
/// A terminal number and a list of ports is a few dozen bytes; what this is for
/// is the machine that answers and then writes without stopping.
const MOST_A_READING_IS: usize = 64 * 1024;

/// How many connections a listener holds in its backlog — the number every
/// ordinary listener takes.
const BACKLOG: u32 = 1024;

/// The Forwards this device holds, by what each one is.
#[derive(Clone, Default)]
pub(crate) struct Forwards {
    held: Arc<Mutex<BTreeMap<Key, Forward>>>,

    /// And the terminal attaches this device is relaying to its members, by
    /// member and Conversation, each with how many sockets are open — see
    /// [`Attaching`]. What is read again when a member's news may have been
    /// missed: a Forward that ended, or was never taken up, while its tab stayed
    /// open is one nothing else would ever read again.
    attaching: Arc<Mutex<BTreeMap<(String, i64), usize>>>,
}

/// One terminal attach this device is relaying to a member, counted for as long
/// as this is held — which is as long as the two halves of the socket are
/// joined. See [`Forwards::attaching`].
pub(crate) struct Attaching {
    forwards: Forwards,
    on: (String, i64),
}

impl Drop for Attaching {
    fn drop(&mut self) {
        let mut attaching = self.forwards.attaching();

        if let Some(count) = attaching.get_mut(&self.on) {
            *count -= 1;

            if *count == 0 {
                attaching.remove(&self.on);
            }
        }
    }
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

/// One Forward as it is held: listening or skipped, and since when it has been
/// missing from the member's reading, where it is.
struct Forward {
    held: Held,

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
    /// None held, which is every start: a Forward stands on a tab open in a
    /// browser, and nothing survives a restart of the server that browser asks.
    pub(crate) fn new() -> Forwards {
        Forwards::default()
    }

    /// The register, locked. Never held across an await.
    fn held(&self) -> MutexGuard<'_, BTreeMap<Key, Forward>> {
        self.held.lock().expect("nothing panics holding this")
    }

    /// The relayed attaches, locked. Never held across an await, nor while the
    /// register above is.
    fn attaching(&self) -> MutexGuard<'_, BTreeMap<(String, i64), usize>> {
        self.attaching.lock().expect("nothing panics holding this")
    }

    /// Count a terminal attach on `device`'s `conversation` as relayed for as
    /// long as what comes back is held — see [`crate::relaying`], which holds
    /// it for as long as it joins the socket.
    pub(crate) fn attached(&self, device: &str, conversation: i64) -> Attaching {
        let on = (device.to_owned(), conversation);

        *self.attaching().entry(on.clone()).or_default() += 1;

        Attaching {
            forwards: self.clone(),
            on,
        }
    }
}

/// The Conversation a relayed path attaches a terminal of —
/// `/api/ui/conversations/{id}/terminals/{number}/attach`, its query aside —
/// or `None` where it is anything else.
pub(crate) fn attach_of(onwards: &str) -> Option<i64> {
    let path = onwards.split_once('?').map_or(onwards, |(path, _)| path);
    let rest = path.strip_prefix("/api/ui/conversations/")?;

    match rest.split('/').collect::<Vec<_>>()[..] {
        [id, "terminals", number, "attach"] if !number.is_empty() => id.parse().ok(),
        _ => None,
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
async fn held(state: AppState) {
    let mut moved = state.nudges.subscribe();
    let mut turning = tokio::time::interval(TURN);

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

                        // A stream taken up again, which says nothing about what
                        // it missed: every Conversation of that member this
                        // device forwards for, or relays a terminal of, is read
                        // again — the second for the `ports` Nudge an attach
                        // made while the stream was down, which nobody heard.
                        Nudge::Everything => {
                            for (device, conversation) in looked_at(&state, Some(&device)) {
                                reconciled(&state, &device, conversation).await;
                            }
                        }

                        _ => {}
                    }
                }

                // Fallen behind, so every Conversation this device forwards for
                // or relays a terminal of is read again: what was missed is
                // unknowable.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    for (device, conversation) in looked_at(&state, None) {
                        reconciled(&state, &device, conversation).await;
                    }
                }

                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            },

            _ = turning.tick() => {
                // A member answering again after a turn that found it not, which
                // ended its Forwards: their tabs may never have closed, and the
                // member has nothing new to say about ports that did not move.
                for device in turned(&state, &mut away).await {
                    for (device, conversation) in looked_at(&state, Some(&device)) {
                        reconciled(&state, &device, conversation).await;
                    }
                }
            }
        }
    }
}

/// Every member and Conversation this device holds a Forward for or relays a
/// terminal attach to, of `device` alone where it is said.
fn looked_at(state: &AppState, device: Option<&str>) -> BTreeSet<(String, i64)> {
    let mut looked_at: BTreeSet<(String, i64)> = state
        .forwards
        .held()
        .keys()
        .filter(|key| device.is_none_or(|device| key.device == device))
        .map(|key| (key.device.clone(), key.conversation))
        .collect();

    looked_at.extend(
        state
            .forwards
            .attaching()
            .keys()
            .filter(|(on, _)| device.is_none_or(|device| on == device))
            .cloned(),
    );

    looked_at
}

/// Read `conversation`'s ports off `device` over the link, and make the
/// Forwards held for it match.
///
/// A port in the reading and not held is taken; one held and still in it has
/// its grace called off; one held and not in it starts its grace, and is ended
/// by [`turned`] once that has run out. A read that could not be made changes
/// nothing — a member that has stopped answering is [`turned`]'s to notice.
async fn reconciled(state: &AppState, device: &str, conversation: i64) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let Some(said) = read_of(
        devices,
        device,
        format!("/api/ui/conversations/{conversation}/ports"),
        "ports",
        MOST_A_READING_IS,
    )
    .await
    else {
        return;
    };

    let reading = match serde_json::from_slice::<PortsView>(&said) {
        Ok(reading) => reading,
        Err(why) => {
            tracing::warn!(
                device,
                conversation,
                "a member answered its ports in a way this device cannot read, so what is \
                 forwarded for it stands: {why}",
            );
            return;
        }
    };

    let offered: BTreeSet<Key> = reading
        .terminals
        .iter()
        .flat_map(|terminal| {
            terminal.ports.iter().map(|&port| Key {
                port,
                device: device.to_owned(),
                conversation,
                terminal: terminal.number,
            })
        })
        .collect();

    let changed = {
        let mut forwards = state.forwards.held();
        let now = Instant::now();
        let mut changed = false;

        for (key, forward) in forwards.iter_mut() {
            if key.device != device || key.conversation != conversation {
                continue;
            }

            match (offered.contains(key), forward.missing) {
                (true, _) => forward.missing = None,
                (false, None) => forward.missing = Some(now),
                (false, Some(_)) => {}
            }
        }

        for key in offered {
            if forwards.contains_key(&key) {
                continue;
            }

            let held = taken(devices, &key);

            forwards.insert(
                key,
                Forward {
                    held,
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
/// a member that is not answering, and try again the ones that were skipped.
///
/// **And say which members have come back** — answering now, where they were
/// in `away` on the last turn — so that what this device relays to them is read
/// again. `away` is left holding the members this turn found not answering.
async fn turned(state: &AppState, away: &mut BTreeSet<String>) -> BTreeSet<String> {
    let looking: BTreeSet<String> = looked_at(state, None)
        .into_iter()
        .map(|(device, _)| device)
        .collect();

    if looking.is_empty() {
        away.clear();
        return BTreeSet::new();
    }

    let Some(devices) = state.devices.as_ref() else {
        return BTreeSet::new();
    };

    // The members answering, read before the register is taken: a membership
    // that could not be read ends nothing, there being no saying who is gone.
    let answering: Option<BTreeSet<String>> = match devices.membership().rows().await {
        Ok(rows) => Some(
            rows.into_iter()
                .filter(|member| member.reachable)
                .map(|member| member.device)
                .collect(),
        ),
        Err(why) => {
            tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so no Forward was ended \
                 for a member that has gone",
            );
            None
        }
    };

    let changed = {
        let mut forwards = state.forwards.held();
        let before = forwards.len();
        let now = Instant::now();

        forwards.retain(|key, forward| {
            let gone = answering
                .as_ref()
                .is_some_and(|answering| !answering.contains(&key.device));
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
                        true => "its member is not answering",
                        false => "its port has left the member's reading",
                    },
                );
            }

            !(gone || over)
        });

        let mut changed = forwards.len() != before;

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

    let Some(answering) = answering else {
        return BTreeSet::new();
    };

    let back = away
        .iter()
        .filter(|device| answering.contains(*device))
        .cloned()
        .collect();

    *away = looking
        .into_iter()
        .filter(|device| !answering.contains(device))
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
/// Conversation and its title, and whether it is forwarding or skipped and why.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A terminal's attach is read for its Conversation, a query and all, and
    /// nothing else under the Conversation is.
    #[test]
    fn only_a_terminal_attach_is_counted_as_one() {
        assert_eq!(
            attach_of("/api/ui/conversations/7/terminals/2/attach"),
            Some(7)
        );
        assert_eq!(
            attach_of("/api/ui/conversations/7/terminals/2/attach?cols=80"),
            Some(7)
        );

        for other in [
            "/api/ui/conversations/7/attach",
            "/api/ui/conversations/7/terminals",
            "/api/ui/conversations/7/terminals/2/ports/3000",
            "/api/ui/conversations/7/terminals//attach",
            "/api/ui/conversations/seven/terminals/2/attach",
            "/api/ui/nudges",
        ] {
            assert_eq!(attach_of(other), None, "{other}");
        }
    }

    /// Each relayed attach is counted while it is held, and the last one let go
    /// of takes the Conversation off the count.
    #[test]
    fn an_attach_is_counted_for_as_long_as_it_is_held() {
        let forwards = Forwards::new();

        let first = forwards.attached("b", 7);
        let second = forwards.attached("b", 7);
        assert_eq!(forwards.attaching().get(&("b".to_owned(), 7)), Some(&2));

        drop(first);
        assert_eq!(forwards.attaching().get(&("b".to_owned(), 7)), Some(&1));

        drop(second);
        assert!(forwards.attaching().is_empty());
    }
}
