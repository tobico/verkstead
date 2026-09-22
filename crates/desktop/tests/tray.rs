//! The tray icon reaches a panel.
//!
//! **Linux's alone**, which is what the `cfg` below says: this is the one
//! platform where the icon is not drawn by anything of Verkstead's. The app
//! publishes a StatusNotifierItem onto the session bus and a panel draws it, so
//! what there is to assert here is the half of that exchange the app does not
//! do — that something out there was handed an icon. macOS and Windows put the
//! icon in a menu bar and a notification area that belong to a logged-in
//! session, and there is nothing a headless suite can stand in for either.
//!
//! **Why it is a test rather than a line in the release workflow.** It used to
//! be the latter: the tray was drawn over libayatana-appindicator, which the
//! AppImage copied in by hand because nothing links it by name, and only
//! running the bundle with a screen would ever find out whether the file copied
//! in was the right one. The tray stopped being that in `22579ba8` — it is Rust
//! on the bus now, and the artwork is compiled into the binary — so the tray is
//! no longer a claim about a bundle, and asserting it in the leg that builds one
//! meant asserting it a few times a month, by hand, at the moment of a release.
//! It is asserted here instead, on every pull request, where a backend that
//! stops registering is a red check on the change that did it.
//!
//! **Ignored by default**, because it wants a session bus that is its own: the
//! name it takes is the one a real panel holds, so on a developer's desktop the
//! bus already has an owner and this would refuse rather than run. `ci.yml`
//! gives it one with `dbus-run-session`, and so can anybody:
//!
//! ```text
//! dbus-run-session -- cargo test -p verkstead-desktop --test tray -- --ignored --test-threads=1
//! ```
//!
//! **One at a time**, which is what that last flag is for: both tests here put a
//! watcher on the bus under the one name the specification gives it, and one of
//! them begins by asserting there is no watcher at all. Run beside each other
//! they would be two tests arguing over a name rather than two tests.
//!
//! **No screen and no toolkit.** The GTK in this crate is the dialogs' and the
//! loop's, and neither is on the way to an icon here — see
//! `crates/desktop/src/toolkit.rs`. So this needs a bus and nothing else, and
//! `$DISPLAY` is not read by anything it touches.
#![cfg(target_os = "linux")]

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use verkstead_desktop::{panel, tray};
use zbus::blocking::{Connection, connection};
use zbus::interface;

/// How long the icon is given to arrive.
///
/// It is two threads and a round trip on a bus that was started for this test
/// alone, so the honest figure is milliseconds; this is the one a loaded CI
/// runner needs, and a test that passes takes none of it.
const PATIENCE: Duration = Duration::from_secs(10);

/// The panel's half of the specification, as much of it as an icon's arrival
/// goes through.
///
/// A real one draws what it is given and keeps a list of it. This one keeps the
/// names and draws nothing, which is the whole of what is being asked: the item
/// is published by the app at a path of its own, and *registering* it is the app
/// saying so to whoever is drawing. That call is the thing that fails when there
/// is no panel, and the thing this suite exists to watch arrive.
struct Watcher {
    /// Every name handed to [`Self::register_status_notifier_item`], in arrival
    /// order. Shared with the test's own thread, which is where it is read.
    registered: Arc<Mutex<Vec<String>>>,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    /// What the app calls, naming the bus name its item is published under.
    fn register_status_notifier_item(&self, service: String) {
        self.registered
            .lock()
            .expect("nothing panics while holding this")
            .push(service);
    }

    /// What a second panel would call. Nothing here is a panel, so this is the
    /// method being present rather than the method doing anything — an
    /// interface missing it is an interface the app could ask about and be
    /// refused.
    fn register_status_notifier_host(&self, _service: String) {}

    /// The list a panel drawing these would read. Empty: what this stands in
    /// for is the call above, and nothing asks this of a watcher it has just
    /// registered with.
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        Vec::new()
    }

    /// **True, or the app gives up before the icon is drawn.** `ksni` reads
    /// this straight after registering and treats a `false` as an icon that
    /// will never be shown. Every watcher a human actually runs answers `true`
    /// here and never meaningfully implements the host half — KDE's returns a
    /// constant, GNOME's refuses `RegisterStatusNotifierHost` outright — so
    /// answering anything else would make this stub the one watcher in the
    /// world that behaves differently, and the test a test of that difference.
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    /// The version of the specification spoken, which has been 0 since it was
    /// written.
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

/// Put the watcher on the bus, and hand back the list it fills in.
///
/// The connection comes back with it and has to be held: dropping it takes the
/// watcher off the bus, and an app registering with a name nobody owns is the
/// failure this whole file is about.
fn watching() -> (Connection, Arc<Mutex<Vec<String>>>) {
    let registered = Arc::new(Mutex::new(Vec::new()));

    let bus = connection::Builder::session()
        .expect(
            "this test speaks to a session bus and there is none here — run it under \
             `dbus-run-session`, as ci.yml does",
        )
        .name("org.kde.StatusNotifierWatcher")
        .expect("the watcher's name is a well-formed bus name")
        .serve_at(
            "/StatusNotifierWatcher",
            Watcher {
                registered: Arc::clone(&registered),
            },
        )
        .expect("the watcher's path is a well-formed object path")
        .build()
        .expect(
            "a panel already owns org.kde.StatusNotifierWatcher on this bus — run this test \
             under a session bus of its own with `dbus-run-session`",
        );

    (bus, registered)
}

/// Wait for this process's own icon to be registered with the watcher.
///
/// The name an item takes carries the id of the process that published it —
/// that is the form the specification asks for — so what is waited for here is
/// not merely *an* icon but this one.
fn wait_for_the_icon(registered: &Mutex<Vec<String>>) {
    let mine = format!("org.kde.StatusNotifierItem-{}-", std::process::id());
    let until = Instant::now() + PATIENCE;

    loop {
        let names = registered
            .lock()
            .expect("nothing panics while holding this")
            .clone();

        if names.iter().any(|name| name.starts_with(&mine)) {
            return;
        }

        assert!(
            Instant::now() < until,
            "the icon was raised without being registered with the watcher on the bus: \
             {names:?} arrived in {PATIENCE:?}, and none of them is this process's {mine}*"
        );

        // Short enough that a test which passes is over before a reader
        // notices, and long enough not to be a spin.
        sleep(Duration::from_millis(20));
    }
}

/// The app hands its icon to whoever is drawing the tray.
#[test]
#[ignore = "needs a session bus of its own; run it under dbus-run-session, as ci.yml does"]
fn the_icon_reaches_a_watcher() {
    let (_watcher, registered) = watching();

    // Held for as long as there is a test to have one: dropping a `TrayIcon`
    // takes it out of the tray, which here would be the app unregistering from
    // the watcher mid-assertion.
    let _icon = tray::show(Some(false), |_| {}).expect("the app could not raise its tray icon");

    wait_for_the_icon(&registered);
}

/// A tray that starts after Verkstead does still gets the icon.
///
/// The case this is about is the ordinary one on a desktop that launches
/// Verkstead at login: the app comes up first, offers its icon to a bus with no
/// panel on it, and is refused. What it does then is wait, which is
/// [`panel::when_one_arrives`] — and this is that wait, from nothing on the bus
/// through to the icon registered.
///
/// What the app does when told is hop onto the loop's thread and offer the icon
/// again; the hop is [`verkstead_desktop::toolkit::later`], which every menu
/// pick already goes through, so what is asserted here is the telling and the
/// offer either side of it.
#[test]
#[ignore = "needs a session bus of its own; run it under dbus-run-session, as ci.yml does"]
fn a_tray_that_arrives_late_still_gets_the_icon() {
    assert!(
        !panel::is_there(),
        "this test starts with a bus that has no tray on it, and this one has one — run it \
         under a session bus of its own with `dbus-run-session`",
    );

    let (told, being_told) = channel();
    panel::when_one_arrives(move || {
        let _ = told.send(());
    })
    .expect("the app could not watch the bus for a tray");

    // Nothing has arrived, so nothing should have been said. A watch that fires
    // on its own is a watch that would have the app offering its icon into an
    // empty bus and settling for the refusal.
    assert!(
        being_told.recv_timeout(Duration::from_millis(500)).is_err(),
        "the app was told a tray had arrived while there was none on the bus",
    );

    let (_watcher, registered) = watching();

    being_told
        .recv_timeout(PATIENCE)
        .expect("a tray arrived on the bus and the app was never told");

    // Which is what the app does next, and where this test stands in for the
    // loop the app hops onto to do it.
    let _icon = tray::show(Some(false), |_| {}).expect("the app could not raise its tray icon");

    wait_for_the_icon(&registered);
}
