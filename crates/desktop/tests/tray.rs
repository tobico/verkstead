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
//! dbus-run-session -- cargo test -p verkstead-desktop --test tray -- --ignored
//! ```
//!
//! **No screen and no toolkit.** The GTK in this crate is the dialogs' and the
//! loop's, and neither is on the way to an icon here — see
//! `crates/desktop/src/toolkit.rs`. So this needs a bus and nothing else, and
//! `$DISPLAY` is not read by anything it touches.
#![cfg(target_os = "linux")]

use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use verkstead_desktop::tray;
use zbus::blocking::connection;
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

/// The app hands its icon to whoever is drawing the tray.
///
/// The name the item takes carries this process's id — that is the form the
/// specification asks for — so the assertion is not merely that *an* icon
/// arrived but that this one did.
#[test]
#[ignore = "needs a session bus of its own; run it under dbus-run-session, as ci.yml does"]
fn the_icon_reaches_a_watcher() {
    let registered = Arc::new(Mutex::new(Vec::new()));

    // Held to the end of the test: dropping this takes the watcher off the bus,
    // and an app registering with a name nobody owns is the failure this whole
    // file is about.
    let _watcher = connection::Builder::session()
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

    // The icon, held for as long as there is a test to have one: dropping a
    // `TrayIcon` takes it out of the tray, which here would be the app
    // unregistering from the watcher mid-assertion.
    let _icon = tray::show(Some(false), |_| {}).expect("the app could not raise its tray icon");

    let mine = format!("org.kde.StatusNotifierItem-{}-", std::process::id());
    let until = Instant::now() + PATIENCE;

    loop {
        let names = registered
            .lock()
            .expect("nothing panics while holding this")
            .clone();

        if names.iter().any(|name| name.starts_with(&mine)) {
            break;
        }

        assert!(
            Instant::now() < until,
            "the app raised its tray icon without registering it with the watcher on the bus: \
             {names:?} arrived in {PATIENCE:?}, and none of them is this process's {mine}*"
        );

        // Short enough that a test which passes is over before a reader
        // notices, and long enough not to be a spin.
        sleep(Duration::from_millis(20));
    }
}
