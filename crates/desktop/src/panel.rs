//! Whatever is drawing the tray on this desktop, and whether it is there yet.
//!
//! **Linux's alone.** A Mac's menu bar and a Windows notification area belong to
//! the session, are there before any app is, and take an icon whenever one is
//! offered. A Linux tray is a program like any other — part of a panel, or a
//! shell extension, or nothing at all — and it says it is there by owning
//! `org.kde.StatusNotifierWatcher` on the session bus. An app publishes its icon
//! and registers it with that name; what has nobody at that name has nowhere to
//! put an icon.
//!
//! **Which is a moment rather than a fact, and that is what this module is
//! for.** A Verkstead started with the session — the Launch on Startup box is
//! exactly that — races the very panel it needs, and a panel restarted by the
//! human who is using it goes away and comes back under the same name. Asked
//! once at startup, the answer is "no tray here" and the app would settle for
//! being a daemon with a screen it never draws on, for as long as the session
//! lasted. So the question is asked again, by being subscribed to rather than
//! polled: the bus reports who owns a name as it changes, and the arrival of an
//! owner for this one is the moment an icon can go up.
//!
//! **Only the first arrival matters here.** Once the icon is registered the
//! backend keeps that relationship up itself — a watcher that goes away and
//! comes back is re-registered with, without anything from this app. What this
//! module answers is the one case the backend gives up on: no watcher at all at
//! the moment the icon was first offered.
//!
//! See [`crate::tray`] for what is published, and `crates/desktop/Cargo.toml`
//! for why it is published rather than drawn.

use std::thread;

use anyhow::{Context, Result};
use zbus::blocking::Connection;
use zbus::blocking::fdo::DBusProxy;
use zbus::names::BusName;

/// The name a tray owns on the session bus.
///
/// KDE's, and every desktop's: the specification was written there and the name
/// went with it, so a GNOME shell extension and a COSMIC panel own this one too.
const TRAY: &str = "org.kde.StatusNotifierWatcher";

/// Whether there is somewhere to put an icon right now.
///
/// False where there is nobody at the name, and false where the question could
/// not be asked at all — a session with no bus in it, which is a container or a
/// login shell rather than a desktop. The two are told apart by
/// [`when_one_arrives`], which is what the caller reaches for next: a bus that
/// is not there is a bus no panel will ever appear on.
pub fn is_there() -> bool {
    look().unwrap_or(false)
}

/// Call `then` when a tray appears on the session bus, or at once if one is
/// there already.
///
/// **On a thread of its own**, parked on the bus until something arrives —
/// which may be never, and costs a parked thread and a bus connection to wait
/// for. The caller gets on with being an app in the meantime; there is nothing
/// to wait for here and nothing to cancel, because a process that is ending
/// takes the thread with it.
///
/// `then` runs on that thread rather than on the loop's. Everything an icon
/// leads to is the loop's, so what a caller hands over is the hop and not the
/// work — see [`crate::toolkit::later`].
///
/// The error is the bus itself: no session bus, or one that would not answer.
/// A caller told that has been told there will be no panel at all, rather than
/// none yet.
pub fn when_one_arrives(then: impl FnOnce() + Send + 'static) -> Result<()> {
    let bus = session()?;
    let dbus = DBusProxy::new(&bus).context("asking the session bus about the tray")?;

    // Subscribed to before the name is asked about, and in that order on
    // purpose: a tray that appears between the two would otherwise be an
    // arrival that had already happened by the time anything was listening for
    // it, and the app would wait out the session for a panel that was already
    // drawing.
    let arrivals = dbus
        .receive_name_owner_changed_with_args(&[(0, TRAY)])
        .context("watching the session bus for a tray")?;

    let already = look()?;

    thread::Builder::new()
        .name("verkstead-tray-watch".into())
        .spawn(move || {
            if !already {
                for arrival in arrivals {
                    let Ok(who) = arrival.args() else {
                        // A signal this could not read is the bus daemon
                        // failing to keep its own specification, and there is
                        // nothing here to do about one but wait for the next.
                        continue;
                    };

                    // The same name losing its owner is reported here too — a
                    // panel being shut down — and what this waits for is
                    // somebody arriving at it.
                    if who.new_owner.is_some() {
                        break;
                    }
                }
            }

            then();
        })
        .context("starting the thread that waits for a tray")?;

    Ok(())
}

/// Whether the tray's name has an owner, and the bus's own answer about it.
fn look() -> Result<bool> {
    let bus = session()?;
    let dbus = DBusProxy::new(&bus).context("asking the session bus about the tray")?;
    let name = BusName::try_from(TRAY).expect("the tray's name is a well-formed bus name");

    dbus.name_has_owner(name)
        .context("asking the session bus whether a tray is running")
}

/// The session bus.
fn session() -> Result<Connection> {
    Connection::session().context("there is no session bus here to publish a tray icon on")
}
