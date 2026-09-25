//! What a device reads off the machine it is on when another one asks what it
//! is: its name, its OS and its addresses (ADR-0020).
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

use verkstead_render::DeviceIdentity;

use crate::device::Device;
use crate::platform::{self, Platform};
use crate::remote::Tailscale;

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
        }
    }

    /// What `device` answers a caller who asked who it is: the id and the
    /// fingerprint it keeps, and the three things read off the machine now.
    ///
    /// Assembled here rather than in the route, because the route is the one
    /// place with nothing to say about any of this: what it knows is that a
    /// caller asked and that nobody has to be anybody to ask.
    pub(crate) async fn identity(&self, device: &Device) -> DeviceIdentity {
        DeviceIdentity {
            device: device.id().to_owned(),
            fingerprint: device.fingerprint().to_owned(),
            name: platform::hostname(),
            os: platform::os_word(self.platform, self.kernel.as_deref()),
            addresses: self.addresses().await,
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
    async fn addresses(&self) -> Vec<String> {
        let tailnet = self.tailscale.tailnet().await;

        let lan = match &self.lan {
            Lan::OfThisMachine => of_this_machine(),
            Lan::Stated(stated) => stated.clone(),
        };

        addresses(tailnet, &lan)
    }
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

    /// And a device on neither a tailnet nor a network answers with an empty
    /// list rather than failing: it still has an id and a fingerprint, which is
    /// what somebody typing an address by hand is looking at.
    #[test]
    fn a_device_with_nowhere_to_be_reached_still_answers() {
        assert!(addresses(Vec::new(), &["127.0.0.1".parse().unwrap()]).is_empty());
    }
}
