//! The ordered stop: a server told to stop takes its advertisement off every
//! other machine's list before it ends (ADR-0020, *Discovery*).
//!
//! **Its own test binary, and the only test in it**, because what it does is
//! send this process a signal: a `SIGTERM` raised while another test of the same
//! binary was running would be a signal that test never asked for, and one
//! raised before the handler is in place would end the run where it stood.
//! `update_opt_out.rs` is here for the same kind of reason — a process may only
//! safely do this much to itself when nothing else in it is looking.
//!
//! **Unix only.** The signal an ordered stop arrives as there is `SIGTERM` or
//! `SIGINT`, and a process can raise one at itself; on Windows it is the
//! console's own interrupt, which is not a thing a test can hand itself.
//!
//! What is *not* being proved here is that a row cannot go stale. A killed
//! server withdraws nothing and its row runs out on its own TTL, which is the
//! same thing that covers a machine whose lid shut — the withdrawal is what
//! makes a restart tidy rather than what makes a stale row impossible.

#![cfg(unix)]

use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent};
use verkstead_server::discovery::{Advertisement, Announcement, SERVICE, ToldToStop};

/// The port this test speaks mDNS over, which is nobody else's: 5353 is where
/// every real implementation on the runner is, and what is being asked here is
/// about this process rather than about the LAN it is plugged into.
const MDNS: u16 = 5464;

/// The device that stops, named by the id its instance is named by.
const STOPPING: &str = "5566778899aabbccddeeff0011223344";

/// How long the advertisement is waited for, and then the goodbye.
///
/// The first is an announcement the daemon probes before it makes; the second is
/// a packet that is already out by the time the withdrawal answers. Neither is
/// spent when the thing arrives.
const PATIENCE: Duration = Duration::from_secs(20);

#[tokio::test]
async fn a_server_told_to_stop_withdraws_its_advertisement() {
    let advertised = Announcement {
        device: STOPPING.to_owned(),
        name: "kitchen-mini".to_owned(),
        os: "macOS".to_owned(),
        peer: 9423,
    };

    let advertising = Advertisement::of_this_device_on(MDNS, true, &advertised);

    // Listening from before the signal is raised, which is the whole reason it
    // is made rather than awaited: a `SIGTERM` arriving with no handler in place
    // ends this test run where it stands.
    let mut stopping = ToldToStop::listening();

    let browsing = ServiceDaemon::new_with_port(MDNS).expect("a daemon to browse with");
    let events = browsing.browse(SERVICE).expect("a browse of the service");
    let fullname = format!("{STOPPING}.{SERVICE}");

    // On the wire first, because a goodbye for something nobody ever heard
    // would prove nothing at all.
    loop {
        match awaited(&events).await {
            ServiceEvent::ServiceResolved(service) if service.get_fullname() == fullname => break,
            _ => continue,
        }
    }

    // SAFETY: the only test in this binary, so nothing else in this process is
    // waiting on a signal or would be ended by one — and the handler above is
    // already in place, which is what makes this a stop rather than a kill.
    let raised = unsafe { libc::raise(libc::SIGTERM) };
    assert_eq!(raised, 0, "the test could not send itself a SIGTERM");

    assert_eq!(
        tokio::time::timeout(PATIENCE, stopping.told())
            .await
            .expect("the signal this process was asked to stop on"),
        "SIGTERM",
        "which is what a service manager sends, and what an ordered stop is",
    );

    // Which is the whole of the stop: the advertisement withdrawn, and the
    // process left to end the way it always did.
    advertising.withdrawn().await;

    loop {
        match awaited(&events).await {
            ServiceEvent::ServiceRemoved(_, gone) if gone == fullname => return,
            _ => continue,
        }
    }
}

/// The next event off the browse, or a failure saying what was being waited for.
///
/// Awaited rather than blocked on: the runtime this test is on is the one the
/// signal handler and the withdrawal are on too, and a thread parked on a
/// channel would be a test that stopped the thing it is watching.
async fn awaited(events: &mdns_sd::Receiver<ServiceEvent>) -> ServiceEvent {
    tokio::time::timeout(PATIENCE, events.recv_async())
        .await
        .expect("an event off the browse before the patience ran out")
        .expect("the browse to still be running")
}
