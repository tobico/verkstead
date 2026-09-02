//! The icon in the system tray, which is the whole of Verkstead's own interface
//! (ADR-0012).
//!
//! There is no window behind it. The viewer is the interface and the browser
//! draws it, so what the tray is for is the two things a browser cannot do for
//! itself: put the viewer back in front of the human, and stop the process that
//! is serving it.
//!
//! **The menu is short on purpose**, and it is now all of it: Open, View Logs,
//! Launch on Startup and Exit, each arrived with the thing that makes it work.
//! An item that does nothing is a worse first impression than a menu that is
//! honestly short.
//!
//! **One of them is a checkbox rather than a button**, and it is drawn from
//! [`crate::startup`] rather than from anything this module keeps: the platform
//! registration is the state, so what the tick says is read at the moment the
//! menu is made, and put right again after it is picked — see
//! [`shows_launch_on_startup`]. What is *asked for* by picking it is the item's
//! own tick rather than that reading over again, because the two can have come
//! apart in between — see [`launch_on_startup_shows`].
//!
//! **Linux draws it as an appindicator**, which is a menu and nothing else: the
//! panel opens the menu when the icon is clicked and reports no click of its
//! own. So the icon's default action *is* [`Chosen::Open`], by being the first
//! item on the menu the click opens. Where a platform reports a double-click of
//! its own — macOS and Windows — [`show`] has it run that same action, so the
//! icon and its menu never mean two different things.
//!
//! **Windows is the one that has to be told which button is which**, because it
//! is the one that reports every click: the menu is the right button's, and the
//! left button's two clicks are the default action. See [`show`], which is
//! where that is said and why.

use std::cell::RefCell;
use std::io::Cursor;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::APP_ID;

/// The artwork, in the binary rather than beside it.
///
/// This binary ships as one file — an AppImage, a portable exe, an app bundle —
/// so an icon read off disk at startup is an icon that can go missing. The
/// packaging icons stage 03 generates are a different set for a different
/// purpose: those are what a desktop's launcher draws from a directory it was
/// installed into, and this is what the panel draws from the running process.
///
/// The 192px one of the generated set, which is more than any panel asks for.
/// The panel is what picks the height — around 22 points on most, twice that on
/// a HiDPI one — and scales what it is given to it, and an icon scaled up is the
/// one that looks wrong.
const ARTWORK: &[u8] = include_bytes!("../../../assets/icons/icon-192.png");

/// What the tray's menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chosen {
    /// Put the viewer back in front of the human.
    Open,
    /// Put this run's log in front of them instead — see [`crate::logs`] for
    /// why a tray app has one at all.
    ViewLogs,
    /// Turn starting Verkstead with the desktop session on, or off — the one
    /// item that is a checkbox, and see [`crate::startup`] for what the tick
    /// stands for.
    LaunchOnStartup,
    /// Stop Verkstead.
    Exit,
}

impl Chosen {
    /// The menu, in the order it is drawn.
    ///
    /// Open first because it is the default action, and the first item is what
    /// a menu means by default. Exit last for the same reason in reverse, with
    /// the two that are neither between them.
    pub const MENU: [Chosen; 4] = [
        Chosen::Open,
        Chosen::ViewLogs,
        Chosen::LaunchOnStartup,
        Chosen::Exit,
    ];

    /// What the item says.
    pub fn label(self) -> &'static str {
        match self {
            Chosen::Open => "Open",
            Chosen::ViewLogs => "View Logs",
            Chosen::LaunchOnStartup => "Launch on Startup",
            Chosen::Exit => "Exit",
        }
    }

    /// What the item is called in an event.
    ///
    /// Named here rather than left to muda, which would otherwise number the
    /// items as it made them: an id this module chose is one it can read back
    /// without a menu in front of it, which is the whole of what a machine with
    /// no screen can check about this.
    fn id(self) -> &'static str {
        match self {
            Chosen::Open => "open",
            Chosen::ViewLogs => "view-logs",
            Chosen::LaunchOnStartup => "launch-on-startup",
            Chosen::Exit => "exit",
        }
    }

    /// The item this id names, or `None` where it names none of them.
    ///
    /// muda's events are one stream for the whole process, so an id from
    /// somewhere else is a thing that can arrive rather than a thing that
    /// cannot.
    fn named(id: &MenuId) -> Option<Chosen> {
        Chosen::MENU
            .into_iter()
            .find(|offered| offered.id() == id.as_ref())
    }
}

thread_local! {
    /// The Launch on Startup item, kept where the handler that has to correct
    /// it can reach it.
    ///
    /// **On the thread rather than in the handler**, because the two ends will
    /// not meet any other way: muda's menu items are not `Send` and the handler
    /// it takes must be. They are the same thread anyway — the loop's, which is
    /// where the events are raised and where everything drawn is spoken to —
    /// which is the whole of why keeping it here works.
    static LAUNCH_ON_STARTUP: RefCell<Option<CheckMenuItem>> = const { RefCell::new(None) };
}

/// What the Launch on Startup item is ticked to, or `None` where this thread
/// has no item to read — the item not being that thread's to have, and nobody
/// having made one at all where the tray could not be raised.
///
/// **After a pick, this is what the human just asked for**: the item ticks
/// itself before the pick is reported, so what it shows is the state they were
/// reaching for rather than the one the menu was drawn from. The two are not
/// always the same — a desktop's own settings can turn the registration off
/// while Verkstead is running, and the menu is drawn once — and reading the
/// registration instead would have such a pick doing the opposite of what the
/// box in front of them said.
pub fn launch_on_startup_shows() -> Option<bool> {
    LAUNCH_ON_STARTUP.with(|item| item.borrow().as_ref().map(CheckMenuItem::is_checked))
}

/// Draw the Launch on Startup item as `on`.
///
/// For whoever handled [`Chosen::LaunchOnStartup`] to call once it knows what
/// actually holds: the item has already ticked itself by the time the pick is
/// reported, and a registration that was not written is a tick that has to go
/// back where it was. Called from the loop's thread, which is where the
/// handling happens; anywhere else it draws nothing and says nothing, the item
/// not being that thread's to have.
pub fn shows_launch_on_startup(on: bool) {
    LAUNCH_ON_STARTUP.with(|item| {
        if let Some(item) = item.borrow().as_ref() {
            item.set_checked(on);
        }
    });
}

/// Put the icon in the tray, and call `chosen` with whatever is picked off it.
///
/// **`chosen` runs on the thread the loop is on**, which is where it has to
/// run: ending the loop is one of the two things it does, and GTK is only ever
/// spoken to from the thread that started it.
///
/// `startup` is what the Launch on Startup box is ticked to as the menu is
/// made, or `None` where this machine has nowhere to keep the registration the
/// tick would stand for — where the item is drawn greyed rather than left off,
/// so that the menu says what Verkstead can do and what it cannot rather than
/// only one of the two.
///
/// The [`TrayIcon`] that comes back *is* the icon — dropping it takes it out of
/// the tray — so the caller holds it for as long as there is an app to have
/// one.
pub fn show(
    startup: Option<bool>,
    chosen: impl Fn(Chosen) + Send + Sync + 'static,
) -> Result<TrayIcon> {
    // First, because it is the one step here that can fail on its own account
    // and the rest of this sets handlers that are the process's rather than
    // this call's — a refusal is worth having before any of that.
    let icon = artwork()?;

    let menu = Menu::new();

    for offered in Chosen::MENU {
        let item: Box<dyn IsMenuItem> = match offered {
            Chosen::LaunchOnStartup => {
                let item = CheckMenuItem::with_id(
                    offered.id(),
                    offered.label(),
                    startup.is_some(),
                    startup.unwrap_or(false),
                    None,
                );
                LAUNCH_ON_STARTUP.with(|kept| *kept.borrow_mut() = Some(item.clone()));
                Box::new(item)
            }
            _ => Box::new(MenuItem::with_id(offered.id(), offered.label(), true, None)),
        };

        menu.append(item.as_ref())
            .with_context(|| format!("putting {} on the tray's menu", offered.label()))?;
    }

    let chosen = Arc::new(chosen);

    MenuEvent::set_event_handler(Some({
        let chosen = Arc::clone(&chosen);

        move |event: MenuEvent| {
            if let Some(picked) = Chosen::named(&event.id) {
                chosen(picked);
            }
        }
    }));

    // The icon's own default action, where the platform has one to report. On
    // Linux nothing ever arrives here — an appindicator hands the click to the
    // menu and says nothing to the app — and this is still where the binding
    // belongs: the default action is one thing, said once, whatever the panel
    // underneath does with it.
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::DoubleClick { .. } = event {
            chosen(Chosen::Open);
        }
    }));

    let raising = TrayIconBuilder::new()
        .with_id(APP_ID)
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Verkstead");

    // **The left button is left to the double-click on Windows**, which is the
    // one place a platform's own habits change what is built here. Windows
    // reports both buttons and both counts, and `tray-icon` would otherwise
    // open the menu on the first left button *up* — which is a menu over the
    // pointer before the second click of a double-click has happened, and a
    // double-click that never arrives because a menu is what took it. So the
    // menu is the right button's, which is where a Windows human reaches for
    // one, and the left button's two clicks are the default action this file
    // already binds. Nothing changes on the other two: an appindicator has only
    // a menu, and a Mac's status item is the same click either way.
    #[cfg(windows)]
    let raising = raising.with_menu_on_left_click(false);

    raising.build().context("putting the icon in the tray")
}

/// The artwork as the tray takes it: pixels, rather than the file they were
/// committed as.
fn artwork() -> Result<Icon> {
    let (pixels, width, height) = rgba(ARTWORK)?;

    Icon::from_rgba(pixels, width, height).context("reading the tray icon's artwork as an icon")
}

/// `encoded` decoded: its pixels as RGBA, its width and its height.
///
/// The one shape this accepts is the one the artwork is committed in, because
/// the artwork is committed rather than supplied — a PNG that is suddenly
/// greyscale or paletted is `tools/generate-icons.sh` having changed under this,
/// which is a thing to say plainly rather than to draw wrongly.
fn rgba(encoded: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let mut reading = png::Decoder::new(Cursor::new(encoded))
        .read_info()
        .context("reading the tray icon's artwork")?;

    let room = reading
        .output_buffer_size()
        .context("the tray icon's artwork is larger than it could be decoded into")?;
    let mut pixels = vec![0; room];

    let frame = reading
        .next_frame(&mut pixels)
        .context("decoding the tray icon's artwork")?;

    if frame.color_type != png::ColorType::Rgba || frame.bit_depth != png::BitDepth::Eight {
        bail!(
            "the tray icon's artwork is {:?} at {:?} and the tray takes 8-bit RGBA",
            frame.color_type,
            frame.bit_depth
        );
    }

    pixels.truncate(frame.buffer_size());

    Ok((pixels, frame.width, frame.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menu as it is drawn, which on a machine with no screen is the only
    /// part of the drawing there is to check.
    #[test]
    fn the_menu_is_open_the_log_the_startup_box_and_then_exit() {
        assert_eq!(
            Chosen::MENU.map(Chosen::label),
            ["Open", "View Logs", "Launch on Startup", "Exit"]
        );
    }

    /// What an event carries is an id, so this is the wiring: the id an item was
    /// made with is the id it is read back by.
    #[test]
    fn an_item_is_what_the_id_it_was_made_with_names() {
        for offered in Chosen::MENU {
            assert_eq!(Chosen::named(&MenuId::new(offered.id())), Some(offered));
        }
    }

    /// muda's events are one stream for the whole process, and an id off some
    /// other menu is not one of this one's items.
    #[test]
    fn an_id_from_somewhere_else_is_none_of_them() {
        assert_eq!(Chosen::named(&MenuId::new("1")), None);
        assert_eq!(Chosen::named(&MenuId::new("")), None);
    }

    /// The artwork is in the binary, so a renamed or re-generated file is a
    /// build that still compiles and an app with no icon. This is what says
    /// otherwise, and it needs no screen to say it.
    #[test]
    fn the_artwork_in_the_binary_decodes_into_an_icon() {
        let (pixels, width, height) = rgba(ARTWORK).expect("the artwork should decode");

        assert_eq!((width, height), (192, 192));
        assert_eq!(pixels.len(), 192 * 192 * 4, "8-bit RGBA, one frame of it");

        artwork().expect("the artwork should be an icon the tray takes");
    }
}
