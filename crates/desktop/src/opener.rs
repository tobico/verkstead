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
    url_reported_as(url, url)
}

/// The same, naming `instead` rather than the url wherever the failure is
/// reported.
///
/// **For the one url this app opens that is a secret.** The login link is the
/// whole of logging in, so a failure naming it writes the key wherever that
/// failure is written — and on this install that is the log file **View Logs**
/// opens, which is the one place ADR-0015 says the key must not be. What is
/// named instead is the address the link is built on, which is what a reader of
/// the log needed from it anyway.
pub fn url_reported_as(url: &str, instead: &str) -> Result<()> {
    open::that_detached(url).with_context(|| format!("opening {instead} in a browser"))
}

/// Open `path` with whatever this desktop reads that kind of file with,
/// without waiting for it either — and for the same reason.
pub fn file(path: &Path) -> Result<()> {
    open::that_detached(path).with_context(|| format!("opening {}", path.display()))
}
