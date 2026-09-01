//! Handing a URL to whatever this desktop opens links with.

use anyhow::{Context, Result};

/// Open `url` in the default browser, without waiting for it.
///
/// Detached because what starts is somebody else's program: a browser that
/// takes ten seconds to come up, or one that stays in the foreground until it is
/// closed, is not something the process serving Verkstead should be waiting on.
pub fn open(url: &str) -> Result<()> {
    ::open::that_detached(url).with_context(|| format!("opening {url} in a browser"))
}
