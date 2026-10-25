//! What a device reads off the machine it is on when another one asks what it
//! is: its name, its OS and its addresses (ADR-0020) — and, off the binary
//! answering rather than the machine, the release it runs (ADR-0027).
//!
//! The other half of an identity. [`super`] keeps what a device *is* — an id
//! and a certificate, invented once and read off the disk at every start — and
//! this reads what it is *like*, none of which is kept anywhere: the hostname
//! the machine answers to, the word for the operating system it is running, and
//! every address a peer could reach it on.
//!
//! **Read at the moment they are answered rather than held from a start.** A
//! laptop moves between the LAN and the tailnet and DHCP moves everybody, so an
//! address remembered from a start weeks ago is an address a peer would dial
//! into nothing. The name and the OS change far less often and are read the same
//! way all the same: three readings at one moment is one answer about one
//! machine, where three readings at three moments is an answer that could
//! describe no machine at all.
//!
//! **The tailnet half stands for five seconds all the same** — see
//! [`HELD_FOR`], which is a bound on what asking costs rather than a cache of
//! the answer. Reading it is a command run on this machine, and the endpoint
//! that wants it is the one nobody has to be anybody to read, on a port facing
//! the LAN: without the hold, a request from a stranger is a process, as fast
//! as they care to ask. Five seconds is short enough that nothing here is stale
//! in any sense a peer could notice, and the LAN half is still read every time.
//!
//! **Nothing here is configured and nothing is typed.** The name is the
//! hostname, the OS is the platform's own word — *Linux (WSL)* where the kernel
//! says so, because a Windows machine and its WSL share a hostname — and the
//! addresses are read off the machine's own interfaces and its Tailscale. A
//! device that could be *told* what it is called would be a device two of which
//! could be told the same thing.
//!
//! **The seams are what a suite states.** What a Linux runner would answer
//! about itself is the one answer no test of this may stand on: the platform,
//! the kernel release and the LAN addresses all go in rather than being read
//! where they are wanted, which is how [`crate::onboarding::Machine`] is put in
//! front of four machines on the one machine a suite runs on.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use verkstead_render::DeviceIdentity;

use crate::device::Device;
use crate::peer::dialling::Peers;
use crate::platform::{self, Platform};
use crate::remote::Tailscale;
use crate::settings::{KeptBy, Settings};

/// How long the tailnet half of the addresses stands before it is read again.
///
/// **Because reading it is a process, and asking is free.** The tailnet
/// addresses come out of `tailscale status --json`, which is a command run on
/// this machine — and the identity endpoint that wants them is the one route
/// nobody has to be anybody to read, on a port facing the LAN. Without this,
/// one request from a stranger is one process, as fast as they care to ask; and
/// the discovery stage has every device on a tailnet probing every other
/// device's identity each time a pane is opened.
///
/// Five seconds, because what is being avoided is a burst rather than a reading
/// that is a day old: a pane opened and a peer probing at the same moment cost
/// one command between them, and a laptop that moved is right again before
/// anybody has finished noticing. The LAN half is read every time regardless —
/// it is a syscall, and the reason for the hold does not apply to it.
///
/// A failed reading is held as well as a good one. A daemon that is not
/// answering is exactly when the calls would otherwise pile up, and an empty
/// list is what every way of not knowing comes to anyway.
const HELD_FOR: Duration = Duration::from_secs(5);

/// The machine this device is on, as it is read when somebody asks.
///
/// A handle rather than three strings, because the whole of it is read per
/// answer: what this holds is where each reading comes *from*, and the readings
/// themselves live for as long as the response they went into.
#[derive(Debug, Clone)]
pub struct Reading {
    /// This machine's Tailscale, which is where the tailnet half of the
    /// addresses comes from — the same binary and the same `status --json` the
    /// Remote access pane stands on, rather than a second way of asking the
    /// same daemon the same question. A machine with no Tailscale contributes
    /// nothing rather than failing the answer.
    tailscale: Tailscale,

    /// Whose word the OS reads. [`Platform::HERE`] on a running server, and
    /// stated by a suite that is asking about one of the other two.
    platform: Platform,

    /// And what the kernel calls itself, which is the one thing that says a WSL
    /// apart from the Linux it looks like from every other angle.
    ///
    /// Read once, where it is read at all: a machine does not reboot into
    /// another kernel underneath a running server, and the file behind it is
    /// the only reading here that would be a syscall per request for an answer
    /// that cannot have changed.
    kernel: Option<String>,

    /// And where the LAN addresses come from — see [`Lan`].
    lan: Lan,

    /// The last tailnet reading and when it was taken, held for [`HELD_FOR`] —
    /// which is what keeps a stranger's request from being a process.
    ///
    /// Shared by every clone rather than one apiece, because the one handle
    /// this server builds is cloned into both the things that read it: the
    /// identity endpoint on the peer listener and the Devices section of the
    /// Remote access pane. What they are describing is one machine, so what
    /// they hold about it is one reading.
    ///
    /// An async lock held across the read, so that two callers arriving
    /// together cost one command rather than two — which is the half of this a
    /// hold on its own would not do, and the half that matters when the asking
    /// is somebody else's.
    held: Arc<Mutex<Option<Held>>>,

    /// And where this device is told whether it is an **Annex**: its own
    /// `config.yaml`, read at each answer for the reason everything here is —
    /// see [`Reading::told_by`]. `None` is a reading nobody tells anything,
    /// which is a device nobody keeps.
    told: Option<Settings>,

    /// And how an Annex asks its Keeper what to advertise — see
    /// [`Reading::asking_its_keeper`]. `None` on every device that is not an
    /// Annex, and on a suite's reading that only asks who keeps it.
    asking: Option<Peers>,

    /// The Keeper's last answer and when it came, which is what an Annex
    /// advertises between two asks and once its Keeper cannot be reached.
    ///
    /// Shared by every clone for the reason the tailnet hold is, and held
    /// across the ask for the same one: two readings arriving together cost
    /// one dial between them.
    answered: Arc<Mutex<Option<Held>>>,

    /// How long that answer stands before the Keeper is asked again —
    /// [`HELD_FOR`] on a running server.
    answer_held_for: Duration,

    /// And where this device's own **Annex Forward** listens, which is the
    /// port it tells its Annex to advertise its addresses at — see
    /// [`Reading::forwarded`]. `None` on a reading of a device that forwards
    /// nothing.
    forwarding_at: Option<u16>,

    /// And the release of Verkstead this device says it runs: [`RUNNING`] on a
    /// running server, and stated by a suite that is putting two releases in
    /// one cluster — see [`Reading::running`].
    ///
    /// Shared by every clone, so a suite can restart the member on another
    /// release under a running server — see [`Reading::now_running`].
    version: Arc<std::sync::RwLock<Option<String>>>,

    /// And the name and OS word it answers with where a suite has said them,
    /// in place of the hostname and the platform's word — see
    /// [`Reading::calling_itself`]. `None` on a running server.
    called: Option<(String, String)>,
}

/// The release this binary is, which is what a device says it runs: the
/// workspace's own version, the same one the Update Notice compares against.
pub const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// The tailnet addresses as they were last read, and when.
#[derive(Debug, Clone)]
struct Held {
    taken: Instant,
    addresses: Vec<String>,
}

/// Where the LAN half of the addresses is read.
///
/// A seam rather than a call, for the reason the platform above is one: what
/// interfaces the machine running the tests happens to have is the one thing a
/// test about ordering and filtering must not depend on, and a suite that
/// asserted against its own runner's addresses would assert something different
/// on every machine.
#[derive(Debug, Clone)]
enum Lan {
    /// The interfaces this machine really has, read at each answer.
    OfThisMachine,

    /// The ones a suite says it has.
    Stated(Vec<IpAddr>),

    /// Or the whole advertised list, said outright — the tailnet half included,
    /// which is to say instead of one.
    ///
    /// **Because an address a device advertises may carry a port and neither
    /// half above can.** The tailnet half is a daemon's answer and the LAN half
    /// is an interface's address, and what a device *advertises* is places to
    /// dial: a peer reads the list and knocks at each in turn, at the peer port
    /// unless the entry says otherwise — see [`crate::peer::dialling`]. On a
    /// running server nothing ever says otherwise; in a suite where two devices
    /// really dial each other the port is the one the operating system picked,
    /// and there is nowhere else to put it.
    ///
    /// Shared by every clone, so a suite can move the machine under a running
    /// server — see [`Reading::moving_to`].
    Advertised(Arc<std::sync::RwLock<Vec<String>>>),
}

impl Reading {
    /// The machine this server is running on, with `tailscale` as its way of
    /// asking after the tailnet.
    pub fn of_this_machine(tailscale: Tailscale) -> Reading {
        Reading {
            tailscale,
            platform: Platform::HERE,
            kernel: platform::kernel_release(),
            lan: Lan::OfThisMachine,
            held: Arc::default(),
            told: None,
            asking: None,
            answered: Arc::default(),
            answer_held_for: HELD_FOR,
            forwarding_at: None,
            version: Arc::new(std::sync::RwLock::new(Some(RUNNING.to_owned()))),
            called: None,
        }
    }

    /// A machine stated rather than read: a platform, whatever its kernel calls
    /// itself, and the addresses its interfaces have.
    ///
    /// What a suite puts a WSL, a Mac and a Windows in front of the identity
    /// endpoint with — see [`crate::onboarding::Machine::stated`], which is the
    /// same seam for the same reason.
    pub fn stated(
        tailscale: Tailscale,
        platform: Platform,
        kernel: Option<String>,
        lan: Vec<IpAddr>,
    ) -> Reading {
        Reading {
            tailscale,
            platform,
            kernel,
            lan: Lan::Stated(lan),
            held: Arc::default(),
            told: None,
            asking: None,
            answered: Arc::default(),
            answer_held_for: HELD_FOR,
            forwarding_at: None,
            version: Arc::new(std::sync::RwLock::new(Some(RUNNING.to_owned()))),
            called: None,
        }
    }

    /// The same again, with the whole advertised list said outright rather than
    /// assembled out of a daemon and an interface list.
    ///
    /// For the suites where two devices really dial each other: what a device
    /// is reachable at inside a test is the loopback on a port the operating
    /// system picked, and a port is the one thing neither half of the reading
    /// can say — see [`Lan::Advertised`]. `tailscale` is taken for the shape of
    /// it and never asked, this being the list rather than a way of finding
    /// one.
    pub fn advertising(
        tailscale: Tailscale,
        platform: Platform,
        kernel: Option<String>,
        advertised: Vec<String>,
    ) -> Reading {
        Reading {
            tailscale,
            platform,
            kernel,
            lan: Lan::Advertised(Arc::new(std::sync::RwLock::new(advertised))),
            held: Arc::default(),
            told: None,
            asking: None,
            answered: Arc::default(),
            answer_held_for: HELD_FOR,
            forwarding_at: None,
            version: Arc::new(std::sync::RwLock::new(Some(RUNNING.to_owned()))),
            called: None,
        }
    }

    /// The same, told by `settings` whether it is an **Annex** — which a
    /// running server always is, over its own Data Directory.
    ///
    /// The one thing a reading says that is configured rather than read off the
    /// machine, and configured by somebody else: a Keeper writes `kept_by` into
    /// its Annex's `config.yaml`, and an Annex is told it is kept rather than
    /// finding out (ADR-0024). Read at each answer rather than once, because the
    /// Keeper rewrites it whenever its own certificate moves.
    pub fn told_by(self, settings: Settings) -> Reading {
        Reading {
            told: Some(settings),
            ..self
        }
    }

    /// The same, asking its Keeper what to advertise through `peers` — which a
    /// running server always does, and which only an Annex ever acts on.
    ///
    /// **An Annex's address is its Keeper's** (ADR-0024, *advertise as told*).
    /// Inside WSL it has interfaces only its Keeper can dial, so what it
    /// advertises is wherever the Keeper forwards for it: the Keeper's own
    /// addresses at the forward's port, asked of the Keeper over a member's
    /// route — see [`crate::peer::annex_forward::FORWARDED`]. Asked rather than
    /// worked out here, because a laptop's addresses move and the Keeper's own
    /// reading is what already keeps up with them.
    pub fn asking_its_keeper(self, peers: Peers) -> Reading {
        Reading {
            asking: Some(peers),
            ..self
        }
    }

    /// The same, holding its Keeper's answer for `held_for` rather than
    /// [`HELD_FOR`].
    ///
    /// For a suite asking what an Annex advertises once its Keeper has gone:
    /// an answer that still stood would be the hold answering rather than the
    /// fallback.
    pub fn holding_its_keepers_answer_for(self, held_for: Duration) -> Reading {
        Reading {
            answer_held_for: held_for,
            ..self
        }
    }

    /// The same, with its own **Annex Forward** listening on `port` — which a
    /// running server's always is, on wherever its start bound it.
    pub fn forwarding_at(self, port: u16) -> Reading {
        Reading {
            forwarding_at: Some(port),
            ..self
        }
    }

    /// The same, saying it runs `version` rather than [`RUNNING`] — or saying
    /// nothing at all, which is what a Verkstead from before the version was
    /// said answers.
    ///
    /// For a suite putting a member a release behind in front of this one: a
    /// binary is the one release it is, and the cluster being asked about is
    /// two of them.
    pub fn running(self, version: Option<&str>) -> Reading {
        Reading {
            version: Arc::new(std::sync::RwLock::new(version.map(str::to_owned))),
            ..self
        }
    }

    /// The same, answering as `name` on `os` rather than this machine's
    /// hostname and its platform's word.
    ///
    /// For a suite whose devices are all one machine: a member's row is what it
    /// last said of itself, and a take-up reads that again — so a suite that
    /// writes a member down as *the-laptop* has the member say so, rather than
    /// writing down the one hostname every device in it shares and reading
    /// right for the wrong reason.
    pub fn calling_itself(self, name: &str, os: &str) -> Reading {
        Reading {
            called: Some((name.to_owned(), os.to_owned())),
            ..self
        }
    }

    /// Say `version` from now on, in place of the release this reading was
    /// stated with — for every clone of it at once.
    ///
    /// For a suite restarting a member on another binary: what is being asked
    /// is what this device's view of it makes of the new release, and a server
    /// started again would be a second server rather than the same member.
    pub fn now_running(&self, version: Option<&str>) {
        *self
            .version
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = version.map(str::to_owned);
    }

    /// Say `advertised` from now on, in place of the list this reading was
    /// stated with — for every clone of it at once.
    ///
    /// For a suite moving a machine between networks under a running server:
    /// a laptop's addresses change with nothing restarted, and what is being
    /// asked is what the cluster makes of that. A reading of the machine's own
    /// interfaces moves by itself, and one stated any other way is left as it
    /// was.
    pub fn moving_to(&self, advertised: Vec<String>) {
        if let Lan::Advertised(standing) = &self.lan {
            *standing
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = advertised;
        }
    }

    /// What this device tells its Annex to advertise: its own addresses, each
    /// at the forward's port — or `None` for a device that forwards nothing.
    ///
    /// Its own addresses as it would advertise them itself, in the same order,
    /// so a third device tries the Annex where it would try the Keeper. A bare
    /// address gains the port and one that carries a port has it replaced,
    /// which is the shape [`crate::peer::dialling`] already reads.
    pub(crate) async fn forwarded(&self) -> Option<Vec<String>> {
        let port = self.forwarding_at?;

        Some(
            self.own_addresses()
                .await
                .iter()
                .map(|address| at_port(address, port))
                .collect(),
        )
    }

    /// Whose Annex this device is, as its `config.yaml` says now — or `None`
    /// for a device nobody keeps, which is every device but an Annex.
    pub(crate) fn keeper(&self) -> Option<KeptBy> {
        self.told
            .as_ref()
            .and_then(|settings| settings.config().kept_by().cloned())
    }

    /// What `device` answers a caller who asked who it is: the id and the
    /// fingerprint it keeps, the three things read off the machine now, and
    /// whose Annex it is where it is one.
    ///
    /// Assembled here rather than in the route, because the route is the one
    /// place with nothing to say about any of this: what it knows is that a
    /// caller asked and that nobody has to be anybody to ask.
    pub(crate) async fn identity(&self, device: &Device) -> DeviceIdentity {
        DeviceIdentity {
            device: device.id().to_owned(),
            fingerprint: device.fingerprint().to_owned(),
            name: self.name(),
            os: self.os(),
            addresses: self.addresses().await,
            kept_by: self.keeper().map(|keeper| keeper.device().to_owned()),
            version: self
                .version
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
        }
    }

    /// What this machine is called, which is its hostname.
    ///
    /// Named beside the identity above because the advertisement is the other
    /// thing that says it — see [`crate::discovery`], which puts this and the
    /// OS below in a TXT record on the LAN. One reading for both, so that the
    /// machine a discovered row draws and the machine an identity answers for
    /// cannot disagree, and so that a suite stating a WSL has stated it for
    /// both at once.
    pub(crate) fn name(&self) -> String {
        match &self.called {
            Some((name, _)) => name.clone(),
            None => platform::hostname(),
        }
    }

    /// And the word for the operating system it is running: the platform's own
    /// everywhere but under WSL.
    pub(crate) fn os(&self) -> String {
        match &self.called {
            Some((_, os)) => os.clone(),
            None => platform::os_word(self.platform, self.kernel.as_deref()),
        }
    }

    /// Every address this device can be reached on, in the order a peer should
    /// try them: the tailnet name and its addresses first, then the LAN.
    ///
    /// **The tailnet first because it is the half that crosses.** A LAN address
    /// is worth nothing to a peer on another network and a tailnet address is
    /// worth something to one on either, so a peer working down this list
    /// reaches the machine on its first try wherever both devices are — and a
    /// machine with no Tailscale still answers with a list rather than with a
    /// failure.
    ///
    /// **An Annex advertises none of that**, but what its Keeper told it —
    /// see [`Reading::asking_its_keeper`].
    async fn addresses(&self) -> Vec<String> {
        if let (Some(keeper), Some(peers)) = (self.keeper(), &self.asking) {
            return self.as_told(keeper.device(), peers).await;
        }

        self.own_addresses().await
    }

    /// The addresses of this device's own, which is everything above but an
    /// Annex's.
    async fn own_addresses(&self) -> Vec<String> {
        // Said outright, which is the whole list rather than a half of it: the
        // daemon is not asked, there being nothing for its answer to be added
        // to.
        if let Lan::Advertised(advertised) = &self.lan {
            return advertised
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
        }

        let tailnet = self.tailnet().await;

        let lan = match &self.lan {
            Lan::OfThisMachine => of_this_machine(),
            Lan::Stated(stated) => stated.clone(),
            Lan::Advertised(_) => unreachable!("answered above"),
        };

        addresses(tailnet, &lan)
    }

    /// The tailnet half, read afresh or answered out of what was read within
    /// the last [`HELD_FOR`].
    ///
    /// The lock is held across the reading rather than taken twice, so that
    /// callers arriving together cost one command between them: this is the
    /// reading a stranger can ask for, and one process per request as fast as
    /// they care to ask is what the hold is here to stop. What the ones behind
    /// it wait out is bounded by the deadline `tailscale` is given — see
    /// [`Tailscale::tailnet`].
    async fn tailnet(&self) -> Vec<String> {
        let mut held = self.held.lock().await;

        if let Some(standing) = held
            .as_ref()
            .filter(|standing| standing.taken.elapsed() < HELD_FOR)
        {
            return standing.addresses.clone();
        }

        let addresses = self.tailscale.tailnet().await;

        *held = Some(Held {
            taken: Instant::now(),
            addresses: addresses.clone(),
        });

        addresses
    }

    /// What an Annex advertises: what `keeper` last said it forwards at, asked
    /// afresh once the answer has stood for its hold.
    ///
    /// **The last answer when the Keeper cannot be asked, and nothing before
    /// the first.** A Keeper that is off is a forward that is off, so the
    /// address it last gave is no worse than any other — and it is still the
    /// right one the moment the Keeper is back. An Annex that has never had an
    /// answer has no address anybody outside could use: its own interfaces are
    /// the ones only its Keeper can dial.
    ///
    /// A failed ask is not held, unlike the tailnet half's. That hold is
    /// against a stranger's requests costing a process each; an Annex's peer
    /// listener is reached through its Keeper's forward, so while the Keeper
    /// cannot be asked nobody outside can reach this reading either — and the
    /// ask made while the Keeper's Join is still in flight, which the Keeper
    /// refuses for not holding this device yet, would otherwise stand over the
    /// one made a moment later that it answers.
    async fn as_told(&self, keeper: &str, peers: &Peers) -> Vec<String> {
        let mut answered = self.answered.lock().await;

        if let Some(standing) = answered
            .as_ref()
            .filter(|standing| standing.taken.elapsed() < self.answer_held_for)
        {
            return standing.addresses.clone();
        }

        match peers.forwarded_by(keeper).await {
            Ok(addresses) => {
                *answered = Some(Held {
                    taken: Instant::now(),
                    addresses: addresses.clone(),
                });

                addresses
            }

            Err(why) => {
                tracing::debug!(
                    %keeper,
                    %why,
                    "this Annex could not ask its Keeper what to advertise, so it advertises \
                     the last answer it had",
                );

                answered
                    .as_ref()
                    .map(|last| last.addresses.clone())
                    .unwrap_or_default()
            }
        }
    }
}

/// `address` at `port`: a bare one gains it, and one carrying a port of its own
/// has it replaced — `[ip]:port` for IPv6, as a URL authority writes it.
///
/// What a Keeper's addresses become in its Annex's advertisement, every one of
/// them pointing at the Annex Forward rather than the peer listener.
fn at_port(address: &str, port: u16) -> String {
    let host = if let Ok(ip) = address.parse::<IpAddr>() {
        ip
    } else if let Ok(socket) = address.parse::<std::net::SocketAddr>() {
        socket.ip()
    } else {
        // A name, with or without a port of its own.
        let name = match address.rsplit_once(':') {
            Some((name, port)) if !name.is_empty() && port.parse::<u16>().is_ok() => name,
            _ => address,
        };

        return format!("{name}:{port}");
    };

    std::net::SocketAddr::new(host, port).to_string()
}

/// The two halves in the order a peer should try them, with nothing said twice.
///
/// **The tailnet address is on an interface too.** `tailscale0` — or the
/// platform's own name for it — carries the same `100.` address the daemon just
/// named, so a list built by appending one half to the other would advertise it
/// twice and tell the peer nothing the second time. The same goes for an
/// address a machine has on two interfaces at once, which is what a bridge or a
/// second link makes: a peer is being handed places to dial, and one place is
/// one entry.
fn addresses(tailnet: Vec<String>, lan: &[IpAddr]) -> Vec<String> {
    let mut advertised = tailnet;

    for address in lan.iter().filter(|address| worth_advertising(address)) {
        let address = address.to_string();

        if !advertised.contains(&address) {
            advertised.push(address);
        }
    }

    advertised
}

/// The addresses of this machine's own interfaces.
///
/// Nothing where they cannot be read at all. An interface list this process is
/// not allowed to see is a machine that says nothing about its LAN rather than
/// a device that cannot answer for itself: the tailnet half may still have
/// something, and a device with no address at all is still a device with an id
/// and a fingerprint, which is what a human typing an address needs to see.
fn of_this_machine() -> Vec<IpAddr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .map(|interface| interface.addr.ip())
        .collect()
}

/// Whether an address is one to hand a peer.
///
/// The loopback is the machine talking to itself, and a link-local address is
/// only meaningful to something already on the link that handed it out — both
/// are addresses a peer would dial and reach either nothing or, worse, itself.
/// Everything else goes in, private ranges included: a LAN is exactly where the
/// devices this list is for are.
fn worth_advertising(address: &IpAddr) -> bool {
    !address.is_loopback() && !address.is_unspecified() && !link_local(address)
}

/// And what a link-local address is on each family: `169.254.0.0/16` on IPv4,
/// and `fe80::/10` on IPv6.
///
/// Written out because the standard library's own answer for the second of
/// them is not on a stable release, and half a check would let exactly the
/// addresses an IPv6 machine has most of through.
fn link_local(address: &IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => address.is_link_local(),
        IpAddr::V6(address) => (address.segments()[0] & 0xffc0) == 0xfe80,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a machine on a tailnet contributes: the node's name, then the two
    /// addresses behind it.
    fn tailnet() -> Vec<String> {
        vec![
            "workbench.tailnet-name.ts.net".to_owned(),
            "100.64.0.1".to_owned(),
            "fd7a:115c:a1e0::1".to_owned(),
        ]
    }

    /// The tailnet half comes first, because it is the half a peer on another
    /// network can reach.
    #[test]
    fn the_tailnet_leads_and_the_lan_follows() {
        let lan = vec!["192.168.1.24".parse().unwrap()];

        assert_eq!(
            addresses(tailnet(), &lan),
            vec![
                "workbench.tailnet-name.ts.net",
                "100.64.0.1",
                "fd7a:115c:a1e0::1",
                "192.168.1.24",
            ],
        );
    }

    /// And the address the tailnet already named is not named again by the
    /// interface it is on.
    #[test]
    fn the_tailnet_address_is_not_advertised_twice() {
        let lan = vec![
            "100.64.0.1".parse().unwrap(),
            "fd7a:115c:a1e0::1".parse().unwrap(),
            "192.168.1.24".parse().unwrap(),
        ];

        assert_eq!(
            addresses(tailnet(), &lan),
            vec![
                "workbench.tailnet-name.ts.net",
                "100.64.0.1",
                "fd7a:115c:a1e0::1",
                "192.168.1.24",
            ],
            "the tailnet address is on tailscale0 as well, and a peer told it twice \
             learns nothing the second time",
        );
    }

    /// A machine with no Tailscale, or one whose daemon is not up, answers with
    /// its LAN addresses rather than with nothing — see
    /// [`crate::remote::Tailscale::tailnet`], where every way of not knowing
    /// comes to the same empty list.
    #[test]
    fn a_machine_with_no_tailscale_answers_with_its_lan() {
        let lan = vec!["192.168.1.24".parse().unwrap(), "10.0.0.7".parse().unwrap()];

        assert_eq!(
            addresses(Vec::new(), &lan),
            vec!["192.168.1.24", "10.0.0.7"],
        );
    }

    /// And an address the machine has on two interfaces at once is one place to
    /// dial rather than two.
    #[test]
    fn an_address_on_two_interfaces_is_advertised_once() {
        let lan = vec![
            "192.168.1.24".parse().unwrap(),
            "192.168.1.24".parse().unwrap(),
        ];

        assert_eq!(addresses(Vec::new(), &lan), vec!["192.168.1.24"]);
    }

    /// The loopback and the link-local addresses are left out of both families:
    /// a peer that dialled either would reach nothing, or itself.
    #[test]
    fn what_a_peer_could_not_reach_is_left_out() {
        let lan = vec![
            "127.0.0.1".parse().unwrap(),
            "::1".parse().unwrap(),
            "169.254.3.4".parse().unwrap(),
            "fe80::1".parse().unwrap(),
            "0.0.0.0".parse().unwrap(),
            "192.168.1.24".parse().unwrap(),
        ];

        assert_eq!(addresses(Vec::new(), &lan), vec!["192.168.1.24"]);
    }

    /// A Keeper's addresses at its forward: a bare address of either family
    /// gains the port, one carrying the peer port has it replaced, and the
    /// tailnet name is a name with a port.
    #[test]
    fn a_keepers_addresses_are_said_at_the_forward() {
        assert_eq!(at_port("100.64.0.1", 8424), "100.64.0.1:8424");
        assert_eq!(
            at_port("fd7a:115c:a1e0::1", 8424),
            "[fd7a:115c:a1e0::1]:8424"
        );
        assert_eq!(at_port("127.0.0.1:53000", 8424), "127.0.0.1:8424");
        assert_eq!(at_port("[::1]:53000", 8424), "[::1]:8424");
        assert_eq!(
            at_port("workbench.tailnet-name.ts.net", 8424),
            "workbench.tailnet-name.ts.net:8424",
        );
        assert_eq!(
            at_port("workbench.tailnet-name.ts.net:8423", 8424),
            "workbench.tailnet-name.ts.net:8424",
        );
    }

    /// And a device on neither a tailnet nor a network answers with an empty
    /// list rather than failing: it still has an id and a fingerprint, which is
    /// what somebody typing an address by hand is looking at.
    #[test]
    fn a_device_with_nowhere_to_be_reached_still_answers() {
        assert!(addresses(Vec::new(), &["127.0.0.1".parse().unwrap()]).is_empty());
    }
}
