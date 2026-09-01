//! The icon in the system tray, which is the whole of Verkstead's own interface
//! (ADR-0012).
//!
//! There is no window behind it. The viewer is the interface and the browser
//! draws it, so what the tray is for is the two things a browser cannot do for
//! itself: put the viewer back in front of the human, and stop the process that
//! is serving it.
//!
//! **The menu is short on purpose.** An item that does nothing is a worse first
//! impression than a menu that is honestly short, so what is on it is what
//! works — View Logs and Launch on Startup arrive with the things that make
//! them work.
//!
//! **Linux draws it as an appindicator**, which is a menu and nothing else: the
//! panel opens the menu when the icon is clicked and reports no click of its
//! own. So the icon's default action *is* [`Chosen::Open`], by being the first
//! item on the menu the click opens. Where a platform reports a double-click of
//! its own — Windows and macOS, in the stages that package for them — [`show`]
//! has it run that same action, so the icon and its menu never mean two
//! different things.

use std::io::Cursor;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// What the tray calls itself, which is what Verkstead is called everywhere a
/// platform asks for an identifier rather than a name (ADR-0012).
const APP_ID: &str = "net.tobico.Verkstead";

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
    /// Stop Verkstead.
    Exit,
}

impl Chosen {
    /// The menu, in the order it is drawn.
    ///
    /// Open first because it is the default action, and the first item is what
    /// a menu means by default.
    pub const MENU: [Chosen; 2] = [Chosen::Open, Chosen::Exit];

    /// What the item says.
    pub fn label(self) -> &'static str {
        match self {
            Chosen::Open => "Open",
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

/// Put the icon in the tray, and call `chosen` with whatever is picked off it.
///
/// **`chosen` runs on the thread the loop is on**, which is where it has to
/// run: ending the loop is one of the two things it does, and GTK is only ever
/// spoken to from the thread that started it.
///
/// The [`TrayIcon`] that comes back *is* the icon — dropping it takes it out of
/// the tray — so the caller holds it for as long as there is an app to have
/// one.
pub fn show(chosen: impl Fn(Chosen) + Send + Sync + 'static) -> Result<TrayIcon> {
    // First, because it is the one step here that can fail on its own account
    // and the rest of this sets handlers that are the process's rather than
    // this call's — a refusal is worth having before any of that.
    let icon = artwork()?;

    let menu = Menu::new();

    for offered in Chosen::MENU {
        let item = MenuItem::with_id(offered.id(), offered.label(), true, None);
        menu.append(&item)
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

    TrayIconBuilder::new()
        .with_id(APP_ID)
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Verkstead")
        .build()
        .context("putting the icon in the tray")
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
    fn the_menu_is_open_and_then_exit() {
        assert_eq!(Chosen::MENU.map(Chosen::label), ["Open", "Exit"]);
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
