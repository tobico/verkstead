//! Nowhere to keep a registration, which is what [Launch on Startup](super) is
//! on a platform whose own arm has not been written yet.
//!
//! Linux keeps one in an XDG autostart entry and macOS in a launch agent, which
//! are arms of [`super`]'s own; Windows keeps one in the Run key, and the stage
//! that packages for it writes that arm beside them. Until then the menu item
//! is drawn greyed rather than left off — a menu that says what Verkstead can
//! do and what it cannot is worth more than a box that ticks and does nothing —
//! and that is a state [`super::Startup`] already draws, for the Linux machine
//! that names no configuration directory to keep an entry in.
//!
//! **So there is never an entry here**, and this says so in the type rather than
//! in a comment: an entry with no way to be made is one nothing can read, write
//! or take away, and the three methods below are unreachable rather than empty.

use anyhow::Result;

/// A registration on a machine with nowhere to keep one — which is to say none
/// at all, this being an enum with nothing in it.
#[derive(Debug, Clone)]
pub(super) enum Entry {}

impl Entry {
    /// Nowhere, which is the whole of what this module has to say.
    pub(super) fn here() -> Option<Entry> {
        None
    }

    /// Whether Verkstead starts with the desktop session — unreachable, there
    /// being no entry to ask.
    pub(super) fn on(&self) -> bool {
        match *self {}
    }

    /// Register the running executable — unreachable, for [`Entry::on`]'s
    /// reason. What a machine with nowhere to keep one answers instead is
    /// [`super::Startup::set`]'s own refusal.
    pub(super) fn write(&self) -> Result<()> {
        match *self {}
    }

    /// And take the registration away — unreachable, for the same reason.
    pub(super) fn remove(&self) -> Result<()> {
        match *self {}
    }
}
