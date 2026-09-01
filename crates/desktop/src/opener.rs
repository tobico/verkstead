//! Handing something to whatever this desktop opens that kind of thing with.
//!
//! Two things are ever handed over: the viewer, which goes to a browser, and the
//! log file, which goes to whatever reads text here. One module for both,
//! because to this binary they are one act — the platform's own opener is asked,
//! and what it decides to start is the desktop's business rather than
//! Verkstead's.

use std::path::Path;

use anyhow::{Context, Result};

/// Open `url` in the default browser, without waiting for it.
///
/// Detached because what starts is somebody else's program: a browser that
/// takes ten seconds to come up, or one that stays in the foreground until it is
/// closed, is not something the process serving Verkstead should be waiting on.
pub fn url(url: &str) -> Result<()> {
    open::that_detached(url).with_context(|| format!("opening {url} in a browser"))
}

/// Open `path` with whatever this desktop reads that kind of file with,
/// without waiting for it either — and for the same reason.
pub fn file(path: &Path) -> Result<()> {
    open::that_detached(path).with_context(|| format!("opening {}", path.display()))
}
