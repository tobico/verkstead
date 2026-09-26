//! How a device is found by one nobody has typed an address into: this one says
//! what it is on the LAN and listens for the others saying the same, both over
//! mDNS — and asks the nodes of its tailnet what *they* are, there being no
//! multicast on one to hear (ADR-0020, *Discovery*).
//!
//! **Because the setup cluster mode was written for is a human with two or
//! three machines**, and asking them for an address apiece is asking them for
//! the one thing about a machine that DHCP is free to change underneath them. A
//! Verkstead that announces itself is one the Verkstead on the next desk can
//! draw a row for, with an Add on it and nothing to type.
//!
//! **In this process rather than through a daemon.** `mdns-sd` advertises and
//! browses from inside the server, so there is no avahi to install on Linux and
//! no Bonjour to find on Windows: one behaviour on three platforms, and nothing
//! an operator has to have running for discovery to work. Shelling out to
//! `avahi-publish` or `dns-sd` was rejected for exactly that.
//!
//! **What goes on the wire is [`SERVICE`] and a TXT record of four things** —
//! see [`Announcement`], which is what a row on somebody else's **Discovered**
//! list is drawn from: the device id, the name and the OS word out of the
//! [`Reading`], and the port the peer listener answers on. The instance is
//! named by the device id rather than by the hostname, because two Verksteads
//! on one machine are two devices and a hostname cannot tell them apart — see
//! [`crate::device`], where that is the same reason the id was invented in the
//! first place.
//!
//! **Against the port the listener landed on** rather than the one the
//! configuration asked for. A `:0` is a port the operating system chose, which
//! is what a suite binds and what the startup line already says: an
//! advertisement naming any other number is one nothing can be dialled at.
//!
//! **And it can be turned off** — `--no-advertising`, its environment variable,
//! and `advertising` in the NixOS module beside `peerListen`. What this puts on
//! a LAN that may not be the human's alone is a hostname, an operating system
//! and a device id, and anything that says that much about a machine to whoever
//! is on the wire has to be able to be told not to. On by default for the
//! reason `openFirewall` is on by default: a discovery nothing can hear is a
//! feature that silently does not work, with nothing on either machine saying
//! why.
//!
//! **And the other half of it is the browse** — see [`Browse`] and [`Found`],
//! which is what the **Discovered** list is drawn from: the same service read
//! rather than written, keyed by device id, and held while somebody is looking
//! rather than for the life of the server. What the rows say is the name, the
//! mark for the OS, the addresses the advertisement named with the port it
//! landed on, and where it was found. What is *not* in them is the three kinds
//! of device left out — a member, this device, and one a join is already pending
//! for — which is [`crate::device::Devices::discovered`], the answer being where
//! a membership is known.
//!
//! **A browse finds things after the fact**, so the first read of that list is
//! empty or short and the rows arrive over the seconds after it. What draws them
//! is [`Nudge::Discovered`], announced as the found list moves: a device
//! appearing, and one that stopped advertising — its goodbye, or its records
//! running out on their TTL, which is a machine whose lid shut and arrives as
//! the same event. Nothing polls (ADR-0009).
//!
//! **And the other half of the finding is the tailnet, which is asked rather
//! than heard** — see [`Probe`]. There is no multicast on a tailnet for an
//! advertisement to go out over, so what stands where the browse stands is the
//! peer list out of `tailscale status` and a question put to each online peer on
//! the peer port. Made as the Discovered reading is asked for rather than held
//! open, a tailnet having nothing to announce itself with: the answer is what the
//! probe found, and it is bounded — see [`MOST_PEERS`], [`AT_ONCE`] and
//! [`PROBING`] — because how many nodes a tailnet has is not this machine's to
//! choose. The two halves come together in [`merged`], by device id, so a machine
//! on one network and one tailnet is one row that says both.
//!
//! **And it is withdrawn on the way out**, which is the one ordered stop this
//! server has — see [`ToldToStop`] and [`Advertisement::withdrawn`]. A signal
//! this process is asked to stop on sends the goodbye that takes the row off
//! every other machine's list at once, and then the process ends as it always
//! did: nothing else is waited on and nothing else is shut down in order. A
//! *killed* server withdraws nothing and its row runs out on its own TTL
//! instead, which is the same thing that covers a machine whose lid shut — so
//! the withdrawal is what makes a restart tidy rather than what makes a stale
//! row impossible.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use mdns_sd::{
    Receiver, ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent, ServiceInfo, UnregisterStatus,
};
use verkstead_render::{DiscoveredDevice, FoundOn};
use verkstead_schema::Nudge;

use crate::device::Device;
use crate::device::reading::Reading;
use crate::nudge::Nudges;
use crate::peer::PEER_PORT;
use crate::peer::dialling::Peers;
use crate::remote::{TailnetPeer, Tailscale};

/// What two Verksteads on one LAN find each other under, browsed and
/// advertised as spelled here.
///
/// `_tcp` because what the record points at is the peer listener, which is a
/// TLS socket — the service being advertised is the thing a peer dials rather
/// than the multicast this was said over.
pub const SERVICE: &str = "_verkstead._tcp.local.";

/// The port mDNS is spoken on, which is every implementation's and is not
/// Verkstead's to choose: RFC 6762 says 5353, and a browser on the port next
/// door would hear nothing and be heard by nobody.
///
/// Named here all the same because a suite advertises on a port of its own —
/// see [`Advertisement::of_this_device_on`]. The multicast this machine really
/// answers on is whatever a real Verkstead is using, and a test that joined it
/// would be a test whose subject was the LAN it happened to be run on.
const MDNS_PORT: u16 = 5353;

/// The key the device id is under in the TXT record, and the three beside it.
///
/// Short words rather than a prefix apiece: the whole record is four pairs and
/// the service name has already said whose they are.
const ID: &str = "id";

/// The hostname the far end is shown under — see [`Reading`], which reads it.
const NAME: &str = "name";

/// The word for the operating system it is running, which is what tells a
/// Windows machine from the WSL on it.
const OS: &str = "os";

/// And the port its peer listener answers on, which is what an Add dials.
///
/// In the TXT record as well as in the SRV record the service already has. The
/// SRV port is what resolving the instance answers with and the two never
/// disagree — a device reading this record has the whole of what a row needs in
/// one place, which is what ADR-0020 asked for.
const PORT: &str = "port";

/// How long the goodbye is waited on before the process is let go of.
///
/// **Because the stop is ordered and is not a shutdown.** What is being waited
/// for is a packet already on its way out — the daemon sends it before it
/// answers — so this is a bound on a wait that should not happen rather than
/// time set aside for one. Two seconds, after which the row on the other
/// machines runs out on its TTL the way a killed server's does, and the process
/// ends either way.
const WITHDRAWING: Duration = Duration::from_secs(2);

/// How long a browse outlives the last read of the Discovered list: **five
/// minutes**.
///
/// **A spell rather than a pane saying it has closed**, because a pane cannot:
/// a phone that locks, a tab that is closed and a laptop whose lid shut all say
/// nothing at all, and a browse held until somebody announced they had stopped
/// looking would be one held for ever. So what keeps it alive is the reading
/// being read — the pane's first draw, every announcement the browse itself
/// makes, and the re-read the viewer does on coming back to a page.
///
/// **And an open pane re-reads on an interval of its own inside this one**, which
/// is what makes a read something there certainly *is*. A browse that has heard
/// nothing new announces nothing, so on a quiet LAN the reads above amount to one
/// at the moment the pane was opened — and a spell measured from that would run
/// out under somebody who is still sitting in front of the list, leaving a device
/// started a few minutes later unheard for the rest of the visit. The viewer's own
/// `LOOKING` is that interval: see `useDiscovered` in `web/src/settings/Remote.tsx`.
///
/// Five minutes because the cost of being wrong is lopsided. Held too long, a
/// browse nobody is reading costs a multicast group and a thread; dropped too
/// soon, a pane somebody is still looking at stops hearing about the machine
/// they are waiting to appear. Comfortably more than that interval, so a handful
/// of lost ticks is still a browse.
const SPELL: Duration = Duration::from_secs(5 * 60);

/// And how long the browse waits on the wire before looking at the clock.
///
/// The spell has to be noticed on a LAN where nothing is happening, and nothing
/// wakes a browse but an event: this is what makes the wait for one bounded.
/// Fifteen seconds, which is a fifth of a minute's worth of doing nothing and
/// well inside [`SPELL`].
const LOOKING: Duration = Duration::from_secs(15);

/// What this device says about itself on the wire: the four things a row on
/// somebody else's **Discovered** list is drawn from.
///
/// Values rather than the handles they were read off, because an advertisement
/// is registered once at a start and what it says then is what it says: the
/// daemon holds the record and re-announces it, and nothing here is asked
/// again. The addresses are the one thing not in it — those the daemon reads
/// off this machine's interfaces itself, and keeps up to date as they move.
#[derive(Debug, Clone)]
pub struct Announcement {
    /// The device id, which is what the instance is named by as well as what
    /// the TXT record carries: what every record and URL in a cluster calls
    /// this device.
    pub device: String,

    /// The hostname it is shown under.
    pub name: String,

    /// The word for its operating system.
    pub os: String,

    /// And the port its peer listener landed on.
    pub peer: u16,
}

impl Announcement {
    /// What `device` on this machine advertises, with `peer` as the port its
    /// listener really bound.
    ///
    /// The name and the OS come off the [`Reading`] rather than being read here
    /// again, so that the machine a discovered row draws and the machine the
    /// identity endpoint answers for are one reading — and so that a suite
    /// stating a WSL has stated it for both.
    pub(crate) fn of(device: &Device, reading: &Reading, peer: u16) -> Announcement {
        Announcement {
            device: device.id().to_owned(),
            name: reading.name(),
            os: reading.os(),
            peer,
        }
    }

    /// The service as `mdns-sd` registers one: the instance named by the id,
    /// the four pairs in its TXT record, and the port to dial.
    ///
    /// **The host name is the device id too**, which is the one string on this
    /// machine that is certainly a legal label and certainly not somebody
    /// else's: two Verksteads on one machine share a hostname, and a machine
    /// that will not say what it is called reads as a sentence with a space in
    /// it. What a human reads is the `name` in the record above; this is what
    /// the address records are hung off.
    ///
    /// **And the addresses are the daemon's to fill in** — `enable_addr_auto`
    /// is what says so. The addresses a peer should try are read off this
    /// machine's interfaces as they are, and kept right as they move, which is
    /// the same answer [`Reading`] gives for the same reason and is not a list
    /// this has to assemble.
    fn service(&self) -> mdns_sd::Result<ServiceInfo> {
        let properties = [
            (ID, self.device.clone()),
            (NAME, self.name.clone()),
            (OS, self.os.clone()),
            (PORT, self.peer.to_string()),
        ];

        ServiceInfo::new(
            SERVICE,
            &self.device,
            &format!("{}.local.", self.device),
            "",
            self.peer,
            &properties[..],
        )
        .map(ServiceInfo::enable_addr_auto)
    }
}

/// This device as the LAN hears it, and the handle that takes it back off.
///
/// One type whether or not anything is being advertised, so that the stop is
/// one path rather than two: a server with advertising turned off is asked to
/// stop the same way and has nothing to withdraw.
pub struct Advertisement {
    /// The daemon and the name it registered — or nothing at all, which is
    /// advertising turned off and also a daemon that would not start.
    announced: Option<Announced>,
}

/// The half of an [`Advertisement`] there is something to withdraw from.
struct Announced {
    /// The mDNS daemon, which is a thread of its own: it announces the service
    /// as it is registered, answers the queries that arrive for it, and follows
    /// this machine's addresses as they change.
    daemon: ServiceDaemon,

    /// And the name it was registered under, which is what the goodbye names.
    fullname: String,
}

impl Advertisement {
    /// What a start makes of the switch and the four things to say: the service
    /// registered where `advertising` is on, and nothing at all where it has
    /// been turned off.
    ///
    /// **A daemon that will not start is a warning rather than a refusal.** The
    /// peer listener's port is a promise this server made and one it cannot
    /// keep is a start to stop, but a machine with no multicast to speak over
    /// still serves a workbench and still answers a typed address — so what
    /// cannot be advertised is said in the log and the start goes on.
    pub fn of_this_device(advertising: bool, announcement: &Announcement) -> Advertisement {
        Advertisement::of_this_device_on(MDNS_PORT, advertising, announcement)
    }

    /// The same, spoken over `mdns` rather than [`MDNS_PORT`].
    ///
    /// For a suite: both halves of a discovery have to be on one port to hear
    /// each other, and a test on 5353 would be advertising a device that does
    /// not exist to every real Verkstead on whatever LAN it was run on — and
    /// reading that LAN's own answers back.
    pub fn of_this_device_on(
        mdns: u16,
        advertising: bool,
        announcement: &Announcement,
    ) -> Advertisement {
        if !advertising {
            tracing::info!(
                "advertising is turned off, so this device says nothing about itself on the \
                 LAN and is found only by an address somebody types",
            );

            return Advertisement { announced: None };
        }

        Advertisement {
            announced: announced(mdns, announcement),
        }
    }

    /// The goodbye: the advertisement taken off every other machine's list at
    /// once, rather than left to run out on its TTL.
    ///
    /// **A row that goes now rather than in two minutes.** Nothing depends on
    /// this — a killed server, a lid that shut and a network that went away all
    /// leave the row to expire, and the list on the other machine is right
    /// again either way — so what this buys is a restart that does not leave a
    /// device that is no longer there drawn as though it were.
    ///
    /// Nothing to do where there was nothing advertised, which is a server
    /// started with the switch thrown or one whose daemon never came up.
    pub async fn withdrawn(&self) {
        let Some(announced) = &self.announced else {
            return;
        };

        let withdrawn = match announced.daemon.unregister(&announced.fullname) {
            Ok(status) => tokio::time::timeout(WITHDRAWING, status.recv_async()).await,
            Err(why) => {
                tracing::warn!(
                    why = %why,
                    "this device's advertisement could not be withdrawn, so its row on the \
                     other machines will run out on its own TTL instead",
                );

                return;
            }
        };

        match withdrawn {
            Ok(Ok(UnregisterStatus::OK)) => tracing::info!(
                "this device's advertisement has been withdrawn, so its row is off the other \
                 machines' lists now rather than at the end of its TTL",
            ),

            // Either the daemon has nothing under that name any more or it
            // stopped answering. Both come to the same thing — a row that runs
            // out rather than one taken back — and neither is worth holding a
            // stop for.
            outcome => tracing::warn!(
                ?outcome,
                "this device's advertisement was not withdrawn before the stop, so its row on \
                 the other machines will run out on its own TTL instead",
            ),
        }

        // And the daemon's own thread let go of, which is not waited on: the
        // goodbye is out by the time the status above arrived, and what is left
        // is a thread in a process that is ending.
        let _ = announced.daemon.shutdown();
    }
}

/// The daemon started and the service registered on `mdns`, or nothing and a
/// warning saying which of the two would not happen.
fn announced(mdns: u16, announcement: &Announcement) -> Option<Announced> {
    let service = match announcement.service() {
        Ok(service) => service,
        Err(why) => {
            tracing::warn!(
                why = %why,
                device = %announcement.device,
                "this device cannot be advertised on the LAN, so it is found only by an \
                 address somebody types",
            );

            return None;
        }
    };

    let fullname = service.get_fullname().to_owned();

    // Bound rather than started: the daemon opens its sockets on its own thread
    // once it is running, so a machine with no multicast to speak over is a
    // warning from that thread rather than an error here.
    let daemon = match ServiceDaemon::new_with_port(mdns) {
        Ok(daemon) => daemon,
        Err(why) => {
            tracing::warn!(
                why = %why,
                "no mDNS daemon could be started, so this device says nothing about itself on \
                 the LAN and is found only by an address somebody types",
            );

            return None;
        }
    };

    if let Err(why) = daemon.register(service) {
        tracing::warn!(
            why = %why,
            fullname,
            "this device's advertisement was refused by its own mDNS daemon, so it is found \
             only by an address somebody types",
        );

        return None;
    }

    tracing::info!(
        fullname,
        peer = announcement.peer,
        "this device is advertising itself on the LAN",
    );

    Some(Announced { daemon, fullname })
}

/// One device this one has heard of, as the half that heard it has it: an
/// advertisement off the LAN, or a peer's own answer over the tailnet.
///
/// **What a row on the Discovered list is drawn from, and nothing more.** There
/// has been no pinned handshake and nothing has been agreed: an advertisement
/// says a device is somewhere and proves nothing at all about it, and a
/// stranger's answer was taken under whatever certificate it happened to show —
/// which is why there is no fingerprint here, and why an **Add** on the row is a
/// **Join** like any other rather than a link being made.
///
/// **One shape for both halves**, because a row is a row: the two are told apart
/// by which list a [`Found`] came out of, and [`merged`] is where that becomes
/// the *LAN* or *Tailscale* a row reads.
///
/// Compared whole — see [`Browsing::resolved`], where a re-resolution of a
/// device already held is the browse hearing the same thing twice rather than
/// news for the open pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The Device Id out of the TXT record, or out of the identity a peer
    /// answered with — which is what this is keyed by either way: two Verksteads
    /// on one machine share a hostname and an address, and the id is the one
    /// thing about either that is nobody else's.
    pub device: String,

    /// The name it is shown under.
    pub name: String,

    /// The word for its operating system.
    pub os: String,

    /// And where to find it, in the order to try them: every address it
    /// advertised with the port its own listener landed on, or the tailnet
    /// address it answered at with the peer port assumed.
    ///
    /// **The port on every one of them**, because that is what makes each of
    /// these a thing a dial can be made to as written — see
    /// [`crate::peer::dialling`], which takes an address with a port on it as
    /// the address it was given.
    pub addresses: Vec<String>,
}

/// What this device has heard of the others, and the browse it hears them over
/// (ADR-0020, *Discovery*).
///
/// **Held while somebody is looking rather than for the life of the server.**
/// The browse starts when the Discovered reading is first asked for and is
/// dropped once nothing has asked for it in [`SPELL`] — a phone that closes a
/// tab says nothing, so what keeps it alive is the reading being read rather
/// than a pane announcing itself, and an open pane re-reads on an interval well
/// inside the spell for exactly that. What it costs while it is running is a
/// multicast group and a thread; what it costs when it is not is nothing.
///
/// **And a browse finds things after the fact.** A cold one has heard nothing,
/// so the first read is empty or short and the rows arrive over the seconds
/// after it: what draws them is [`Nudge::Discovered`], announced as the found
/// list moves, which is the arrangement ADR-0009 put every other list in this
/// viewer on. Nothing polls for the rows — the pane's interval says somebody is
/// looking and nothing else, and a read that answers what it answered last
/// leaves the page as it stands.
#[derive(Debug, Clone)]
pub struct Browse {
    heard: Heard,
}

/// Where the Discovered list's rows come from.
#[derive(Debug, Clone)]
enum Heard {
    /// A real browse of [`SERVICE`] over a multicast, held while the reading is
    /// being read. Which is every running server.
    OverTheLan(Arc<Browsing>),

    /// And what a fixture says it heard, which is heard from nowhere.
    ///
    /// Here for the reason [`Reading::stated`] and `Device::stated` are: what a
    /// suite about the three exclusions is asking is which rows an answer leaves
    /// out, and standing a multicast up to ask it would be a test whose subject
    /// was the LAN the runner happened to be on. It is also what the committed
    /// fixtures of this list are written through — a golden file cannot be
    /// written off whatever is advertising on a build machine.
    ///
    /// Behind a lock for the reason the real one's rows are: a press that reached
    /// nobody forgets the row it was made on — see [`Browse::forgotten`] — and a
    /// fixture whose rows were a plain list would be a forget that stuck to
    /// whichever clone of this handle happened to make it.
    Stated(Arc<Mutex<Vec<Found>>>),
}

impl Browse {
    /// A browse of the multicast every other implementation is on, ready to
    /// start when the reading is first asked for.
    pub fn of_this_device(nudges: Nudges) -> Browse {
        Browse::of_this_device_on(MDNS_PORT, nudges)
    }

    /// The same, spoken over `mdns` rather than [`MDNS_PORT`] — for a suite, and
    /// for [`Advertisement::of_this_device_on`]'s reason: both halves of a
    /// discovery have to be on one port to hear each other, and a test on 5353
    /// would be reading whatever real Verksteads are on the runner's LAN.
    pub fn of_this_device_on(mdns: u16, nudges: Nudges) -> Browse {
        Browse {
            heard: Heard::OverTheLan(Arc::new(Browsing {
                mdns,
                nudges,
                listening: Mutex::new(Listening::default()),
            })),
        }
    }

    /// What a fixture states it heard, browsing nothing — see [`Heard::Stated`].
    pub fn stated(found: Vec<Found>) -> Browse {
        Browse {
            heard: Heard::Stated(Arc::new(Mutex::new(found))),
        }
    }

    /// And a device that has heard nothing and never will, which is every
    /// router stood up without one.
    pub fn heard_nothing() -> Browse {
        Browse::stated(Vec::new())
    }

    /// What is held now, and the browse started or kept alive by the asking.
    ///
    /// **The read is what says somebody is looking**, which is the whole of how
    /// the browse is governed: this is called once per answer of the Discovered
    /// reading, and [`SPELL`] is measured from the last of them.
    pub(crate) fn found(&self) -> Vec<Found> {
        match &self.heard {
            Heard::Stated(found) => found
                .lock()
                .expect("a fixture's rows are not poisoned")
                .clone(),
            Heard::OverTheLan(browsing) => browsing.asked(),
        }
    }

    /// And `device` taken off what is held, which is a press on its **Add** that
    /// reached nobody — see [`crate::device::Devices::add_found`].
    ///
    /// **Because the row was wrong.** A browse holds what it last heard, and a
    /// device that has gone off the LAN says nothing on its way out unless it was
    /// asked to stop: the press is the moment this device *learns* the row is
    /// stale, so the row goes then rather than at the end of a TTL nobody is
    /// watching.
    ///
    /// **And a forget is not cheaply undone, which is why only a press that
    /// reached nobody makes one.** `mdns-sd` announces a resolution when a record
    /// *changes*, so a device whose advertisement is exactly what this browse
    /// already cached is not heard again for it: what brings a forgotten row back
    /// is that device saying something new — a restart, a move, an address — or
    /// this browse being dropped at the end of its [`SPELL`] and a later read
    /// starting a fresh one, which begins with nothing cached. So a device that
    /// answered and merely said no keeps its row: see
    /// [`crate::device::Devices::add_found`], where that is told from a row
    /// nothing answered at.
    ///
    /// **The browse is the half a forget lands on**, because it is the half that
    /// holds rows at all: the tailnet half is asked afresh on every read, so a
    /// peer that has gone is off the next answer without anybody forgetting
    /// anything.
    pub(crate) fn forgotten(&self, device: &str) {
        match &self.heard {
            Heard::Stated(found) => found
                .lock()
                .expect("a fixture's rows are not poisoned")
                .retain(|row| row.device != device),

            Heard::OverTheLan(browsing) => {
                browsing.forget(device);
            }
        }
    }
}

/// A browse of [`SERVICE`], and what it has heard.
struct Browsing {
    /// The port the multicast is spoken over, which is [`MDNS_PORT`] anywhere
    /// but a suite.
    mdns: u16,

    /// Word to the open pages that the found list moved, which is what draws a
    /// device that appeared without a reload and without a poll.
    nudges: Nudges,

    /// And what is held: the rows, and when the reading was last asked for.
    listening: Mutex<Listening>,
}

/// What a browse holds between the reads of it.
#[derive(Debug, Default)]
struct Listening {
    /// Whether one is running, which is what says a read has to start one.
    ///
    /// Written here rather than read off the task, because what a read has to
    /// know is whether to spawn: a flag under the same lock as the rows cannot
    /// disagree with them, and two reads arriving together start one browse.
    browsing: bool,

    /// What it has heard, by Device Id — see [`Found`].
    ///
    /// Ordered, so that two reads a moment apart are the same rows in the same
    /// order: the list is drawn under a heading rather than sorted by the page,
    /// and an order that came out of a hash would move a row under a thumb.
    found: BTreeMap<String, Found>,

    /// And when the reading was last asked for, which [`SPELL`] is measured
    /// from. `None` is a browse nobody has asked for yet.
    asked: Option<Instant>,
}

impl Browsing {
    /// The rows, the ask noted, and a browse started where none is running.
    fn asked(self: &Arc<Browsing>) -> Vec<Found> {
        let (found, start) = {
            let mut listening = self.listening();

            listening.asked = Some(Instant::now());

            let start = !listening.browsing;
            listening.browsing = true;

            (listening.found.values().cloned().collect(), start)
        };

        // Spawned once the lock has been let go of: the browse takes it as it hears
        // things, and has nothing to wait on the read that started it for.
        if start {
            tokio::spawn(browsing(Arc::clone(self)));
        }

        found
    }

    /// One event off the browse, and the open pages told where it moved the
    /// list.
    fn hearing(&self, event: ServiceEvent) {
        match event {
            ServiceEvent::ServiceResolved(service) => self.resolved(&service),
            ServiceEvent::ServiceRemoved(_, fullname) => self.gone(&fullname),

            // A search started or stopped, and an instance found before it
            // resolved: each of them is this browse describing itself rather
            // than a device to draw a row for.
            _ => {}
        }
    }

    /// A device heard whole: held under its id, and announced where that is news.
    ///
    /// **A re-resolution of what is already held is not news.** A browse
    /// re-resolves as records are refreshed, and an announcement apiece would be
    /// the open pane re-reading this list every couple of minutes for an answer
    /// that had not moved.
    ///
    /// And an advertisement this build cannot read a row out of is left alone:
    /// something on the wire under this service name that names no device is
    /// either not a Verkstead or one from a future that says more than this one
    /// knows how to draw.
    fn resolved(&self, service: &ResolvedService) {
        let Some(found) = row(service) else {
            tracing::debug!(
                fullname = service.get_fullname(),
                "something advertising this service said too little about itself to draw a \
                 row for, so it is left off the Discovered list",
            );

            return;
        };

        {
            let mut listening = self.listening();

            if listening.found.get(&found.device) == Some(&found) {
                return;
            }

            tracing::debug!(
                device = %found.device,
                name = %found.name,
                addresses = ?found.addresses,
                "a device was heard advertising itself on the LAN",
            );

            listening.found.insert(found.device.clone(), found);
        }

        self.nudges.announce(Nudge::Discovered);
    }

    /// And one that is gone: the goodbye a device told to stop sends, or its
    /// records running out on their TTL — which is a machine whose lid shut, and
    /// arrives here as the same event.
    ///
    /// The instance is named by the Device Id, so what a removal names is the
    /// row to take away. One this device is not holding is one it never heard,
    /// which is a browse that started after a goodbye rather than anything to
    /// say.
    fn gone(&self, fullname: &str) {
        let Some(device) = fullname.strip_suffix(&format!(".{SERVICE}")) else {
            return;
        };

        if self.forget(device) {
            tracing::debug!(device, "a device stopped advertising itself on the LAN");
        }
    }

    /// One device off the rows, and the open pages told where that moved the list.
    /// Whether there was a row to take is handed back, a list that did not move
    /// being nothing to announce and nothing to say.
    ///
    /// Two things call it: a device that said goodbye or ran out of TTL, and a
    /// press on its **Add** that reached nobody — see [`Browse::forgotten`].
    fn forget(&self, device: &str) -> bool {
        if self.listening().found.remove(device).is_none() {
            return false;
        }

        self.nudges.announce(Nudge::Discovered);

        true
    }

    /// Whether nobody has asked for the reading in [`SPELL`], which is what ends
    /// a browse.
    fn abandoned(&self) -> bool {
        self.listening()
            .asked
            .is_none_or(|asked| asked.elapsed() > SPELL)
    }

    /// And the browse over: nothing held, and the next read free to start
    /// another.
    ///
    /// **What was heard goes with it**, rather than being kept for a reader who
    /// may come back: what a stopped browse holds is what was on the LAN when
    /// somebody was last looking, and a machine switched off in the meantime
    /// would be drawn as a device to press Add on. The read that starts the next
    /// browse is answered short and the rows arrive over the seconds after it,
    /// which is what every first read of this list does.
    ///
    /// **And nothing is announced.** A list emptied because nobody has read it
    /// in five minutes is a change with nobody to tell.
    fn stopped(&self) {
        let mut listening = self.listening();

        listening.browsing = false;
        listening.found.clear();
    }

    /// What is held, locked.
    fn listening(&self) -> MutexGuard<'_, Listening> {
        self.listening
            .lock()
            .expect("what a browse has heard is not poisoned")
    }
}

impl std::fmt::Debug for Browsing {
    /// Said as the port and what is held, the [`Nudges`] in it being a channel
    /// with nothing to say about itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Browsing")
            .field("mdns", &self.mdns)
            .field("listening", &self.listening)
            .finish_non_exhaustive()
    }
}

/// The browse itself: held while the reading is being read, and let go of once
/// it has not been for [`SPELL`].
///
/// **The daemon is this task's own**, rather than the one an advertisement
/// registered through: a server may be advertising or not — the switch is the
/// advertising half alone — and a browse that borrowed that daemon would be a
/// discovery turned off by the switch that is there to stop this machine
/// *saying* anything.
///
/// A daemon that will not start is a warning and no rows, which is the stance
/// the advertisement takes for its reason: a machine with no multicast to speak
/// over still serves a workbench and still answers a typed address.
async fn browsing(browse: Arc<Browsing>) {
    let Some((daemon, events)) = looking(browse.mdns) else {
        browse.stopped();

        return;
    };

    loop {
        // The wire, or the clock: what is waited on is an event, and the
        // timeout is what makes the spell something this task notices on a LAN
        // where nothing at all is happening.
        match tokio::time::timeout(LOOKING, events.recv_async()).await {
            Ok(Ok(event)) => browse.hearing(event),

            // The daemon has let go of this browse, which is not something
            // anything here asks for: what is left is to stop holding rows off
            // a browse that is not running.
            Ok(Err(gone)) => {
                tracing::debug!(%gone, "the browse of the LAN ended, so nothing is being heard");

                break;
            }

            Err(_quiet) => {}
        }

        if browse.abandoned() {
            tracing::debug!(
                "nothing has read the Discovered list for a while, so this device has \
                 stopped browsing the LAN",
            );

            break;
        }
    }

    let _ = daemon.stop_browse(SERVICE);
    let _ = daemon.shutdown();

    browse.stopped();
}

/// A daemon on `mdns` with a browse of [`SERVICE`] running, or a warning saying
/// which of the two would not happen.
fn looking(mdns: u16) -> Option<(ServiceDaemon, Receiver<ServiceEvent>)> {
    let daemon = match ServiceDaemon::new_with_port(mdns) {
        Ok(daemon) => daemon,
        Err(why) => {
            tracing::warn!(
                why = %why,
                "no mDNS daemon could be started, so this device cannot hear the others on \
                 the LAN and they are linked by an address somebody types",
            );

            return None;
        }
    };

    match daemon.browse(SERVICE) {
        Ok(events) => {
            tracing::debug!("this device is browsing the LAN for the others");

            Some((daemon, events))
        }

        Err(why) => {
            tracing::warn!(
                why = %why,
                "this device's own mDNS daemon refused to browse for the others, so they \
                 are linked by an address somebody types",
            );

            let _ = daemon.shutdown();

            None
        }
    }
}

/// The row an advertisement is worth, or nothing where it said too little to
/// draw one.
///
/// **The three words out of the TXT record and the port out of the SRV**, which
/// is the record the resolution answers with: the two carry the same number and
/// the one that cannot be a string that is not a number is the one to stand on.
///
/// Nothing at all where the id, the name or the OS word is missing **or blank**:
/// a row with no id cannot be keyed, excluded or pressed, and one with no name or
/// mark is a row that says nothing to the person reading it. Blank as well as
/// missing because `mdns-sd` reads a TXT value that is empty — and one whose
/// bytes are not UTF-8 — as the empty string, so a broken advertisement arrives
/// here looking like a present one.
///
/// **And nothing where it said too much**, by the bounds a join is refused over —
/// see [`crate::peer::joining::too_much_said`]. This is [`row_of`]'s judgement of
/// what a probed peer answered, made of what a device advertised, and it is the
/// same judgement rather than a second one like it: what is at stake either way
/// is a stranger's words drawn on somebody's page.
fn row(service: &ResolvedService) -> Option<Found> {
    let device = service.get_property_val_str(ID)?;
    let name = service.get_property_val_str(NAME)?;
    let os = service.get_property_val_str(OS)?;

    if device.trim().is_empty() || name.trim().is_empty() || os.trim().is_empty() {
        return None;
    }

    // Sorted, the resolution answering a set: IPv4 before IPv6 and each of them
    // in order, so that the addresses a row draws are the same two reads running
    // and the order a dial works down is settled here rather than by a hash.
    let mut addresses: Vec<IpAddr> = service
        .get_addresses()
        .iter()
        .map(ScopedIp::to_ip_addr)
        .filter(dialable)
        .collect();

    addresses.sort_unstable();

    if addresses.is_empty() {
        return None;
    }

    let addresses: Vec<String> = addresses
        .into_iter()
        .map(|address| SocketAddr::new(address, service.get_port()).to_string())
        .collect();

    if let Some(too_much) = crate::peer::joining::too_much_said(device, name, os, &addresses) {
        tracing::debug!(
            fullname = service.get_fullname(),
            too_much,
            "something advertising this service said more about itself than a device says, \
             so it is left off the Discovered list",
        );

        return None;
    }

    Some(Found {
        device: device.to_owned(),
        name: name.to_owned(),
        os: os.to_owned(),
        addresses,
    })
}

/// Whether an address a device advertised is one a dial could be made to as it
/// is written.
///
/// **Which a link-local IPv6 address is not.** It means nothing without the
/// interface it was heard on, and what carries that is a scope nothing on the
/// dialling side of this takes — so a row drawn with one would be an Add that
/// could only fail. Everything else is kept, the loopback included: two
/// Verksteads on one machine really do reach each other there.
fn dialable(address: &IpAddr) -> bool {
    match address {
        IpAddr::V4(_) => true,
        IpAddr::V6(address) => address.segments()[0] & 0xffc0 != 0xfe80,
    }
}

/// The tailnet half of a discovery: the peers `tailscale status` names, asked on
/// the peer port what they are (ADR-0020, *Discovery*).
///
/// **Asked rather than heard, because a tailnet carries no multicast.** There is
/// nothing on one for an advertisement to go out over, so what stands where the
/// browse stands is a list and a question: the online peers out of
/// [`Tailscale::peers`], and each of them asked over the un-gated identity
/// endpoint — see [`Peers::stranger`], which takes whatever certificate a
/// stranger presents for the one call exactly as a **Join** does.
///
/// **When the reading is asked for rather than on a schedule.** A tailnet is not
/// a thing that announces itself, so there is nothing to hold open and nothing
/// arrives after the fact: the probe is made as the Discovered list is read and
/// the answer is what it found. Which is the other half of why that list is a
/// reading of its own — it costs a handful of dials, and the membership beside it
/// must not.
///
/// **Bounded, because a tailnet is somebody else's size.** A pane opened on a
/// machine in a tailnet of three hundred nodes must not be three hundred TLS
/// handshakes: [`MOST_PEERS`] is the most that are asked at all, [`AT_ONCE`] the
/// most in flight, and [`PROBING`] what one of them may cost.
///
/// **And a node that is not a Verkstead is no row.** It refuses, it answers
/// something that is not an identity, or it answers nothing at all, and all three
/// come to the same absent row — which is what most of a tailnet does, most of it
/// being phones and servers with something else on that port.
///
/// **Not behind the `advertising` switch**, which is the mDNS half's alone. What
/// that switch is for is a LAN that may not be the human's — a café's, an
/// office's — and a tailnet is the opposite of that: every node on it is one this
/// machine's own human let in. What a probe shows them is this device's
/// certificate and the shape of one dial at a port, which is what any peer of a
/// cluster already sees.
#[derive(Debug, Clone)]
pub struct Probe {
    asking: Asking,
}

/// Where the tailnet half of the Discovered list comes from.
#[derive(Debug, Clone)]
enum Asking {
    /// A real probe of this machine's own tailnet, made as the list is read.
    /// Which is every running server.
    OverTheTailnet(Probing),

    /// And what a fixture says it found, found by asking nobody — here for
    /// [`Heard::Stated`]'s reason, and it is the same reason: a suite about which
    /// rows an answer merges or leaves out has no business standing a tailnet up,
    /// and a committed fixture cannot be written off whatever tailnet the machine
    /// that wrote it was on.
    Stated(Vec<Found>),
}

/// How long one peer has to be asked what it is, walk of its addresses and all.
///
/// **Per peer rather than per address**, which is the opposite of what a dial to
/// a member spends — see [`crate::peer::dialling::REACHING`]. A member's
/// addresses are the places that one device has been and the live one may be the
/// last of them, so each is worth its own patience; a peer's addresses are one
/// machine's v4 and v6 on one tailnet, reached over one link, and a peer that is
/// slow at the first is slow at the second. What this bounds is the pane: the
/// list is read as somebody opens it, and the wait for it is theirs.
///
/// Three seconds, which is a tailnet round trip and a `tailscale status` on the
/// far end with room to spare, and short enough that a wedged peer is a pause
/// nobody notices. A peer cut off here is one that will be asked again the next
/// time somebody opens the pane.
const PROBING: Duration = Duration::from_secs(3);

/// The most peers asked at all, however many the status names: **sixty-four**.
///
/// **Because the number of nodes on a tailnet is not this machine's to choose.**
/// A personal tailnet is three machines and a company's is three hundred, and a
/// pane opened on the second must not be a probe of every one of them — what the
/// human is looking for there is the laptop on their desk, and a discovery that
/// went through the whole estate to find it would be a pane that took a minute
/// and a burst of handshakes at three hundred machines that did not ask for one.
///
/// Sixty-four because it is comfortably more than anybody's own machines and
/// nowhere near an estate. A tailnet with more than this in it is one where the
/// typed address is the answer, which is the same thing that covers a Windows
/// machine and the WSL on it. The peers asked are the first sixty-four by name,
/// that being a settled order rather than a hash's — see [`Tailscale::peers`].
pub const MOST_PEERS: usize = 64;

/// And the most of those probes in flight at once: **sixteen**.
///
/// A ceiling on the cost rather than on the count. Each probe in flight is a
/// socket, a task and a TLS handshake, and the whole of a bounded fan-out is that
/// the machine doing the asking knows how many it is paying for at any moment.
/// Sixteen is enough that a personal tailnet is one round — every machine asked
/// at once, and the list drawn in the time one peer takes — and a ceiling that
/// only bites on the estate [`MOST_PEERS`] is for.
pub const AT_ONCE: usize = 16;

impl Probe {
    /// A probe of this machine's own tailnet, dialling the peer port a device
    /// answers on where nobody has said otherwise.
    pub fn of_this_tailnet(tailscale: Tailscale) -> Probe {
        Probe::of_this_tailnet_on(PEER_PORT, tailscale)
    }

    /// The same, asking on `peer` rather than [`PEER_PORT`].
    ///
    /// **For a suite**, and for the reason [`Advertisement::of_this_device_on`]
    /// takes a port: what a peer list names is an address and never a port, so a
    /// probe assumes one — and a test cannot stand a device on the real one,
    /// which is a port on the machine running it and may be another Verkstead's.
    pub fn of_this_tailnet_on(peer: u16, tailscale: Tailscale) -> Probe {
        Probe {
            asking: Asking::OverTheTailnet(Probing {
                tailscale,
                peer,
                probing: PROBING,
            }),
        }
    }

    /// What a fixture states it found, asking nobody — see [`Asking::Stated`].
    pub fn stated(found: Vec<Found>) -> Probe {
        Probe {
            asking: Asking::Stated(found),
        }
    }

    /// And a device with no tailnet to ask, which is every router stood up
    /// without one.
    pub fn asked_nothing() -> Probe {
        Probe::stated(Vec::new())
    }

    /// The same probe, giving each peer `patience` rather than [`PROBING`].
    ///
    /// For a suite standing in front of a peer that answers nothing: what is
    /// being asked there is that the list costs one deadline and comes back
    /// without it, and a test that waited out the real one would be spending its
    /// time on the clock rather than on the question.
    pub fn waiting(self, patience: Duration) -> Probe {
        match self.asking {
            Asking::OverTheTailnet(probing) => Probe {
                asking: Asking::OverTheTailnet(Probing {
                    probing: patience,
                    ..probing
                }),
            },
            stated => Probe { asking: stated },
        }
    }

    /// Every peer of this machine's tailnet that answered as a device, asked now.
    ///
    /// `peers` is how the asking is done — this device's certificate presented
    /// and whatever the far end shows taken — rather than something this holds,
    /// so that a suite shortening what a dial waits shortens these too.
    pub(crate) async fn found(&self, peers: &Peers) -> Vec<Found> {
        match &self.asking {
            Asking::Stated(found) => found.clone(),
            Asking::OverTheTailnet(probing) => probing.asked(peers).await,
        }
    }
}

/// A probe of this machine's tailnet: what to ask, and what one asking costs.
#[derive(Debug, Clone)]
struct Probing {
    /// This machine's Tailscale, which is where the peer list comes from — the
    /// same `status --json` the Remote access pane stands on, asked a different
    /// question.
    tailscale: Tailscale,

    /// The port a peer is asked on, which is [`PEER_PORT`] anywhere but a suite.
    ///
    /// **Assumed rather than known, and that is a limit of this half.** An
    /// advertisement carries the port a device really bound; a peer list carries
    /// no port at all, so a device told to listen somewhere else is found on the
    /// LAN and not on the tailnet. Which is a known limit of the same kind as a
    /// Windows machine and the WSL on it, with the typed address as what is left.
    peer: u16,

    /// And how long one peer has: [`PROBING`] on a running server.
    probing: Duration,
}

impl Probing {
    /// The peer list read, the ceiling taken off it, and what answered as a
    /// device.
    async fn asked(&self, peers: &Peers) -> Vec<Found> {
        let asking = self.tailscale.peers().await;

        if asking.len() > MOST_PEERS {
            tracing::debug!(
                peers = asking.len(),
                asked = MOST_PEERS,
                "this machine's tailnet holds more peers than a discovery asks, so the rest \
                 are left to an address somebody types",
            );
        }

        // Spawned rather than awaited in order, and harvested as each finishes:
        // what is being bounded is how many are in flight, and a round that
        // waited for its slowest peer before starting the next one would be a
        // ceiling on the concurrency and a multiple of [`PROBING`] on the pane.
        let mut probes: tokio::task::JoinSet<Option<Found>> = tokio::task::JoinSet::new();
        let mut found = Vec::new();

        for peer in asking.into_iter().take(MOST_PEERS) {
            while probes.len() >= AT_ONCE {
                answered(&mut probes, &mut found).await;
            }

            let asking = Asked {
                peers: peers.clone(),
                peer,
                port: self.peer,
                probing: self.probing,
            };

            probes.spawn(async move { asking.answer().await });
        }

        while !probes.is_empty() {
            answered(&mut probes, &mut found).await;
        }

        // In a settled order, the answers having arrived in whatever order the
        // peers happened to answer in: the list is drawn under a heading rather
        // than sorted by the page, and two reads a moment apart must be the same
        // rows the same way round — which is what [`Browsing::found`] holds its
        // own rows in a map for.
        found.sort_by(|one, other| one.device.cmp(&other.device));

        found
    }
}

/// One peer, and everything asking it what it is takes.
///
/// A struct because the asking is spawned and a spawned future owns what it
/// holds: four things handed over rather than borrowed, and one of them a clone
/// of the dialling handle.
struct Asked {
    /// How the asking is done: this device's certificate presented, and whatever
    /// the far end shows taken for the one call.
    peers: Peers,

    /// The peer: what the tailnet calls it, and the addresses to try.
    peer: TailnetPeer,

    /// The port it is asked on, a peer list naming none — see [`Probing::peer`].
    port: u16,

    /// And how long the whole of it has.
    probing: Duration,
}

impl Asked {
    /// The row this peer is worth, or nothing — which is a peer that is not a
    /// Verkstead, one that would not answer inside its deadline, and one that
    /// answered something this build cannot draw a row from.
    async fn answer(&self) -> Option<Found> {
        match tokio::time::timeout(self.probing, self.asking()).await {
            Ok(found) => found,

            Err(_) => {
                tracing::debug!(
                    peer = %self.peer.name,
                    addresses = ?self.peer.addresses,
                    "a peer of this machine's tailnet did not say what it was inside the \
                     deadline, so it is left off the Discovered list",
                );

                None
            }
        }
    }

    /// Its addresses in the order the peer list named them, until one answers.
    ///
    /// **An address that answered ends the walk**, exactly as a dial to a member
    /// ends on one: the v4 and the v6 of one machine on one tailnet answer the
    /// same way, so a second try after an answer would be a second handshake for
    /// a row already drawn.
    async fn asking(&self) -> Option<Found> {
        for address in &self.peer.addresses {
            let at = at(address, self.port);

            match self.peers.stranger(&at).await {
                Ok(identity) => return row_of(&identity, at),

                Err(why) => tracing::debug!(
                    peer = %self.peer.name,
                    %at,
                    why = format!("{why:#}"),
                    "a peer of this machine's tailnet did not answer as a device, so it is \
                     left off the Discovered list",
                ),
            }
        }

        None
    }
}

/// The next probe to finish taken off `probes`, and its row kept where there was
/// one.
///
/// A task that panicked is a probe that found nothing: there is nothing on a
/// Discovered list worth taking a request down over, and the row it would have
/// drawn is one the next read asks for again.
async fn answered(probes: &mut tokio::task::JoinSet<Option<Found>>, found: &mut Vec<Found>) {
    if let Some(Ok(Some(row))) = probes.join_next().await {
        found.push(row);
    }
}

/// Where a peer is asked: the address the peer list named, on the port a probe
/// assumes.
///
/// Through [`SocketAddr`] wherever the address is one, which is what puts the
/// brackets on a v6 address — see [`crate::peer::dialling`], which takes an
/// address carrying a port as the address it was given. Anything else is a name,
/// and a name with a port on it is dialled at it.
fn at(address: &str, port: u16) -> String {
    match address.parse::<IpAddr>() {
        Ok(address) => SocketAddr::new(address, port).to_string(),
        Err(_) => format!("{address}:{port}"),
    }
}

/// The row a peer's answer is worth, or nothing where it said too little or too
/// much to draw one.
///
/// **Nothing where the id, the name or the OS word is missing**, which is
/// [`row`]'s own judgement of an advertisement said of an answer: a row with no
/// id cannot be keyed, excluded or pressed, and one with no name or mark says
/// nothing to the person reading it.
///
/// **And nothing where it said too much**, by the same bounds a join is refused
/// over — see [`crate::peer::joining::too_much`]. What is on this row is a
/// stranger's words drawn on somebody's page, and a device that will not keep to
/// a hostname's length is one there is no reason to draw at all.
///
/// The address is the one that answered rather than the list the device gave of
/// itself: what a Discovered row says is where this device found it, and the
/// whole of what that device says it is reachable on arrives with the **Join**
/// the press makes.
fn row_of(identity: &verkstead_render::DeviceIdentity, at: String) -> Option<Found> {
    if identity.device.trim().is_empty()
        || identity.name.trim().is_empty()
        || identity.os.trim().is_empty()
    {
        return None;
    }

    if let Some(too_much) = crate::peer::joining::too_much(identity) {
        tracing::debug!(
            device = %identity.device,
            %at,
            too_much,
            "a peer of this machine's tailnet said more about itself than a device says, so \
             it is left off the Discovered list",
        );

        return None;
    }

    Some(Found {
        device: identity.device.clone(),
        name: identity.name.clone(),
        os: identity.os.clone(),
        addresses: vec![at],
    })
}

/// The **Discovered** rows the two halves of a discovery come to: what was heard
/// on the LAN and what answered over the tailnet, merged by **Device Id**.
///
/// **One row per device rather than one per way of finding it**, because it is
/// one machine: a laptop on the same network as this one and on the same tailnet
/// is found twice, and two rows offering to link it would be two presses about
/// one device. What the row says instead is both ways — *LAN and Tailscale* —
/// and its addresses are every place either half found it.
///
/// **The LAN first, which is the order the addresses come out in.** Two machines
/// on one network reach each other without a tailnet in the middle, so the
/// shorter road is the one an **Add** should try first. The name and the mark are
/// the LAN's too where both halves have them, for no stronger reason than that
/// one of them has to be and they agree: both are that device's own account of
/// itself, said once in a TXT record and once over a handshake.
///
/// **And the exclusions are not made here** — see
/// [`crate::device::Devices::discovered`], which is where a membership is known.
/// What this does is one thing: two lists into the rows a pane draws.
pub(crate) fn merged(lan: Vec<Found>, tailnet: Vec<Found>) -> Vec<DiscoveredDevice> {
    let mut rows: BTreeMap<String, DiscoveredDevice> = BTreeMap::new();

    let heard = lan.into_iter().map(|found| (found, FoundOn::Lan));
    let asked = tailnet.into_iter().map(|found| (found, FoundOn::Tailscale));

    for (found, on) in heard.chain(asked) {
        match rows.get_mut(&found.device) {
            Some(row) => {
                if !row.found.contains(&on) {
                    row.found.push(on);
                }

                // Every place either half found it, and no address twice: a
                // device on one network with one address answers the same string
                // to both halves only where the probe's port is the one it
                // advertised, and a row listing it twice would be one address
                // drawn as two.
                for address in found.addresses {
                    if !row.addresses.contains(&address) {
                        row.addresses.push(address);
                    }
                }
            }

            None => {
                rows.insert(
                    found.device.clone(),
                    DiscoveredDevice {
                        device: found.device,
                        name: found.name,
                        os: found.os,
                        addresses: found.addresses,
                        found: vec![on],
                    },
                );
            }
        }
    }

    rows.into_values().collect()
}

/// What says this process has been told to stop, which is the one thing this
/// server has ever waited for beyond a request.
///
/// **Listening from the moment it is made rather than from the moment it is
/// awaited.** A signal arriving before the handler is in place is a signal that
/// kills the process, so the two are separate calls: the start makes this
/// before it serves anything, and a suite that raises one at itself cannot beat
/// it into place.
///
/// **And there is nothing graceful behind it.** What an ordered stop does is
/// withdraw the advertisement — see [`Advertisement::withdrawn`] — and then let
/// the process end. No request is drained, no session is stopped and no
/// listener is closed in order, because none of that was ever true of this
/// server and none of it is what a row on somebody else's list is waiting on.
pub struct ToldToStop {
    /// The signals, or nothing where the handlers could not be installed —
    /// which is a process that dies on one the way it always did.
    #[cfg(unix)]
    signals: Option<(tokio::signal::unix::Signal, tokio::signal::unix::Signal)>,
}

impl ToldToStop {
    /// Listen for them, from now.
    ///
    /// A handler that will not install is a warning rather than a refusal, for
    /// the reason a daemon that will not start is one: what is lost is the
    /// goodbye, and a server that refused to come up over it would be a server
    /// that would not serve for want of something nothing depends on.
    #[cfg(unix)]
    pub fn listening() -> ToldToStop {
        use tokio::signal::unix::{SignalKind, signal};

        // SIGTERM is what a service manager sends and SIGINT is what a terminal
        // sends, and an ordered stop is what both of them are: there is no third
        // signal here, and a SIGKILL is by definition not one anything can hear.
        let signals = match (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
        ) {
            (Ok(terminate), Ok(interrupt)) => Some((terminate, interrupt)),
            (terminate, interrupt) => {
                tracing::warn!(
                    why = ?terminate.err().or(interrupt.err()),
                    "this process cannot hear the signal it would be asked to stop on, so its \
                     advertisement will run out on its own TTL rather than being withdrawn",
                );

                None
            }
        };

        ToldToStop { signals }
    }

    /// And the Windows half, where an ordered stop is the console's own
    /// interrupt and there is no `SIGTERM` to hear.
    #[cfg(windows)]
    pub fn listening() -> ToldToStop {
        ToldToStop {}
    }

    /// Wait until one arrives, and say which.
    ///
    /// Pending for ever where the handlers are not there, which is what leaves
    /// such a process dying on a signal exactly as it did before there was
    /// anything to withdraw.
    #[cfg(unix)]
    pub async fn told(&mut self) -> &'static str {
        let Some((terminate, interrupt)) = &mut self.signals else {
            return std::future::pending().await;
        };

        tokio::select! {
            _ = terminate.recv() => "SIGTERM",
            _ = interrupt.recv() => "SIGINT",
        }
    }

    /// See the Unix one above.
    #[cfg(windows)]
    pub async fn told(&mut self) -> &'static str {
        match tokio::signal::ctrl_c().await {
            Ok(()) => "Ctrl-C",
            Err(why) => {
                tracing::warn!(
                    why = %why,
                    "this process cannot hear the interrupt it would be asked to stop on, so \
                     its advertisement will run out on its own TTL rather than being withdrawn",
                );

                std::future::pending().await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two devices, one of which is on this machine's LAN and its tailnet both.
    const OVER_THERE: &str = "0011223344556677889900aabbccddee";
    const A_THIRD: &str = "99887766554433221100aabbccddeeff";

    /// What a browse or a probe found of `device`, at `addresses`.
    fn found(device: &str, addresses: &[&str]) -> Found {
        Found {
            device: device.to_owned(),
            name: "laptop".to_owned(),
            os: "macOS".to_owned(),
            addresses: addresses.iter().map(|at| (*at).to_owned()).collect(),
        }
    }

    /// A device found by one half alone reads that half, and nothing about the row
    /// says there was another.
    #[test]
    fn one_half_alone_is_the_row_it_found() {
        assert_eq!(
            merged(vec![found(OVER_THERE, &["192.168.1.31:8423"])], Vec::new())
                .into_iter()
                .map(|row| (row.device, row.found, row.addresses))
                .collect::<Vec<_>>(),
            vec![(
                OVER_THERE.to_owned(),
                vec![FoundOn::Lan],
                vec!["192.168.1.31:8423".to_owned()]
            )],
        );

        assert_eq!(
            merged(Vec::new(), vec![found(OVER_THERE, &["100.64.0.2:8423"])])
                .into_iter()
                .map(|row| (row.device, row.found))
                .collect::<Vec<_>>(),
            vec![(OVER_THERE.to_owned(), vec![FoundOn::Tailscale])],
        );
    }

    /// And one found both ways is one row saying both, with every address either
    /// half found on it and the LAN's first.
    ///
    /// **One row because it is one machine.** Two rows offering to link it would
    /// be two presses about one device, and the shorter road is the one an Add
    /// should try first: two machines on one network reach each other without a
    /// tailnet in the middle.
    #[test]
    fn a_device_found_both_ways_is_one_row() {
        let rows = merged(
            vec![found(OVER_THERE, &["192.168.1.31:8423"])],
            vec![found(OVER_THERE, &["100.64.0.2:8423"])],
        );

        assert_eq!(rows.len(), 1, "one machine is one row: {rows:?}");
        assert_eq!(rows[0].found, vec![FoundOn::Lan, FoundOn::Tailscale]);
        assert_eq!(
            rows[0].addresses,
            vec!["192.168.1.31:8423".to_owned(), "100.64.0.2:8423".to_owned()],
        );
    }

    /// An address both halves found is on the row once. A device on one network
    /// that advertised the port a probe assumes answers the same string to both,
    /// and a row listing it twice would be one address drawn as two.
    #[test]
    fn an_address_both_halves_found_is_drawn_once() {
        let rows = merged(
            vec![found(OVER_THERE, &["192.168.1.31:8423"])],
            vec![found(OVER_THERE, &["192.168.1.31:8423"])],
        );

        assert_eq!(rows[0].addresses, vec!["192.168.1.31:8423".to_owned()]);
        assert_eq!(
            rows[0].found,
            vec![FoundOn::Lan, FoundOn::Tailscale],
            "and both halves still found it, which is what the row says",
        );
    }

    /// And the rows come out in a settled order, whatever order the two halves
    /// were found in: the list is drawn under a heading rather than sorted by the
    /// page, and an order that moved between two reads would move a row under a
    /// thumb.
    #[test]
    fn the_rows_are_in_a_settled_order() {
        let by_id = vec![OVER_THERE.to_owned(), A_THIRD.to_owned()];

        assert_eq!(
            merged(
                vec![found(A_THIRD, &["10.0.0.9:8423"])],
                vec![found(OVER_THERE, &["100.64.0.2:8423"])],
            )
            .into_iter()
            .map(|row| row.device)
            .collect::<Vec<String>>(),
            by_id,
        );

        assert_eq!(
            merged(
                vec![found(OVER_THERE, &["10.0.0.9:8423"])],
                vec![found(A_THIRD, &["100.64.0.2:8423"])],
            )
            .into_iter()
            .map(|row| row.device)
            .collect::<Vec<String>>(),
            by_id,
        );
    }
}
