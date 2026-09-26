//! Advertising: what this device says about itself on the LAN, read back off
//! the wire by a suite standing where the Verkstead on the next desk stands
//! (ADR-0020, *Discovery*).
//!
//! **A real mDNS daemon browsing a real multicast, on a port of the suite's
//! own.** Both halves of a discovery have to be on one port to hear each other,
//! and 5353 is the port every other implementation on this machine is on: a
//! test there would be advertising devices that do not exist to whatever real
//! Verkstead is on the LAN the runner happens to be plugged into, and reading
//! that LAN's own answers back as though they were its own. So each test here
//! speaks over a port of its own — which also keeps the tests in this file out
//! of each other's way, libtest running them side by side.
//!
//! **What is being asked is what a row is drawn from**, which is the TXT record
//! and the port the SRV record names: the device id, the name, the OS word and
//! the peer port the listener really landed on. A device advertising the port
//! its configuration asked for rather than the one it got would be a row nobody
//! could press Add on.

use std::time::{Duration, Instant};

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use verkstead_server::discovery::{Advertisement, Announcement, SERVICE};

/// How long a test waits to hear an advertisement it is expecting.
///
/// Generous, because what is being waited on is a probe and an announcement
/// over a multicast rather than a call: the daemon proves the instance name is
/// nobody else's before it says anything, and a runner under load takes as long
/// as it takes. Nothing here spends it when the answer arrives.
const PATIENCE: Duration = Duration::from_secs(20);

/// And how long it listens before calling an advertisement absent.
///
/// Shorter than [`PATIENCE`], and it is spent in full every time: proving a
/// thing is not on the wire is waiting for it and not hearing it. Long enough
/// to cover the announcement of the device advertised beside it, which is what
/// says the browse was listening at all.
const QUIET: Duration = Duration::from_secs(5);

/// The ids the devices here are named by, which is what the instance is named
/// by and what the TXT record carries.
const LOUD: &str = "aa11bb22cc33dd44ee55ff6677889900";
const HUSHED: &str = "00998877665544332211ffeeddccbbaa";

/// A second device on the same machine as the first, which is the case the id
/// was invented for: one hostname, two devices, a peer port each.
const BESIDE: &str = "1234567890abcdef1234567890abcdef";

/// What a device would say about itself, with `peer` as the port its listener
/// landed on.
///
/// The name and the OS are stated rather than read off the runner, for the
/// reason the device reading's own suites state them: what a Linux runner would
/// answer about itself is the one answer a test of this must not stand on.
fn announcement(device: &str, peer: u16) -> Announcement {
    Announcement {
        device: device.to_owned(),
        name: "kitchen-mini".to_owned(),
        os: "macOS".to_owned(),
        peer,
    }
}

/// Browse `mdns` until the device `wanted` resolves, and hand back what came
/// back.
fn resolved(mdns: u16, wanted: &str) -> ResolvedService {
    let browsing = ServiceDaemon::new_with_port(mdns).expect("a daemon to browse with");
    let events = browsing.browse(SERVICE).expect("a browse of the service");
    let deadline = Instant::now() + PATIENCE;

    while Instant::now() < deadline {
        if let Ok(ServiceEvent::ServiceResolved(service)) =
            events.recv_timeout(Duration::from_millis(100))
        {
            if service.get_property_val_str("id") == Some(wanted) {
                return *service;
            }
        }
    }

    panic!("nothing advertising device {wanted} was heard on the multicast");
}

/// And every device heard on `mdns` inside [`QUIET`], which is what a test
/// asking after one that should not be there listens for.
fn heard(mdns: u16) -> Vec<String> {
    let browsing = ServiceDaemon::new_with_port(mdns).expect("a daemon to browse with");
    let events = browsing.browse(SERVICE).expect("a browse of the service");
    let deadline = Instant::now() + QUIET;
    let mut heard = Vec::new();

    while Instant::now() < deadline {
        if let Ok(ServiceEvent::ServiceResolved(service)) =
            events.recv_timeout(Duration::from_millis(100))
        {
            if let Some(device) = service.get_property_val_str("id") {
                heard.push(device.to_owned());
            }
        }
    }

    heard
}

/// The four things a row on somebody else's Discovered list is drawn from, read
/// back off the wire.
#[test]
fn the_txt_record_carries_what_a_row_is_drawn_from() {
    let advertised = announcement(LOUD, 9423);
    let _advertising = Advertisement::of_this_device_on(5461, true, &advertised);

    let service = resolved(5461, LOUD);

    assert_eq!(
        service.get_property_val_str("id"),
        Some(LOUD),
        "the device id is what every record and URL in a cluster names a device by, \
         so it is what a discovered row is keyed on: {service:?}",
    );
    assert_eq!(
        service.get_property_val_str("name"),
        Some("kitchen-mini"),
        "the name is what the row is drawn under: {service:?}",
    );
    assert_eq!(
        service.get_property_val_str("os"),
        Some("macOS"),
        "the OS word is what the row's icon comes from — and the only thing that tells a \
         Windows machine from the WSL on it: {service:?}",
    );
    assert_eq!(
        service.get_property_val_str("port"),
        Some("9423"),
        "the peer port is what an Add dials: {service:?}",
    );

    assert_eq!(
        service.get_port(),
        9423,
        "and the service's own port says the same thing, so that resolving the instance \
         is enough to dial it: {service:?}",
    );
    assert_eq!(
        service.get_fullname(),
        format!("{LOUD}.{SERVICE}"),
        "the instance is named by the device id rather than by the hostname, because two \
         Verksteads on one machine are two devices and a hostname cannot tell them apart",
    );
    assert!(
        !service.get_addresses().is_empty(),
        "and the addresses are the daemon's own reading of this machine, which is what a \
         peer dials: {service:?}",
    );
}

/// Two Verksteads on one machine are two instances, each naming the port its
/// own listener landed on.
#[test]
fn two_devices_on_one_machine_advertise_a_port_each() {
    let first = announcement(LOUD, 9101);
    let second = announcement(BESIDE, 9102);

    let _advertising = Advertisement::of_this_device_on(5462, true, &first);
    let _beside_it = Advertisement::of_this_device_on(5462, true, &second);

    assert_eq!(
        resolved(5462, LOUD).get_port(),
        9101,
        "the first device is dialled where its own listener landed",
    );
    assert_eq!(
        resolved(5462, BESIDE).get_port(),
        9102,
        "and the second where its own did, the two sharing a hostname and nothing else",
    );
}

/// And a device whose advertising has been turned off says nothing at all,
/// while the one beside it goes on saying everything.
#[test]
fn nothing_is_advertised_where_the_switch_has_been_thrown() {
    let hushed = announcement(HUSHED, 9201);
    let loud = announcement(LOUD, 9202);

    let _turned_off = Advertisement::of_this_device_on(5463, false, &hushed);
    let _advertising = Advertisement::of_this_device_on(5463, true, &loud);

    let heard = heard(5463);

    assert!(
        heard.iter().any(|device| device == LOUD),
        "the device beside it is what says the browse was listening at all: {heard:?}",
    );
    assert!(
        !heard.iter().any(|device| device == HUSHED),
        "a device with advertising turned off puts nothing on the wire — not its hostname, \
         not its OS and not its id: {heard:?}",
    );
}
