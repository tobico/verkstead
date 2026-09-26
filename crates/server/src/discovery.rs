//! How a device is found by one nobody has typed an address into: this one
//! says what it is on the LAN, and says it over mDNS (ADR-0020, *Discovery*).
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
//! **And it is withdrawn on the way out**, which is the one ordered stop this
//! server has — see [`ToldToStop`] and [`Advertisement::withdrawn`]. A signal
//! this process is asked to stop on sends the goodbye that takes the row off
//! every other machine's list at once, and then the process ends as it always
//! did: nothing else is waited on and nothing else is shut down in order. A
//! *killed* server withdraws nothing and its row runs out on its own TTL
//! instead, which is the same thing that covers a machine whose lid shut — so
//! the withdrawal is what makes a restart tidy rather than what makes a stale
//! row impossible.

use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceInfo, UnregisterStatus};

use crate::device::Device;
use crate::device::reading::Reading;

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
