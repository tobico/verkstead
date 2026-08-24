//! The Guide: what an agent needs in order to ask well, carried by the binary
//! that does the asking.
//!
//! The text is a markdown file in this repo, embedded at compile time rather
//! than assembled at run time — so what an agent reads is exactly what was
//! reviewed, and the binary alone is the whole documentation.

use std::io::Write;

use anyhow::{Context, Result, bail};

/// The core Guide, and since the gates Topic was retired the whole of it:
/// nothing in the pipeline gates a commit any more, so there is no task left
/// whose reading is worth deferring.
const CORE: &str = include_str!("../guide/core.md");

/// Print the Guide on stdout.
///
/// A Topic is still what `verkstead guide <topic>` asks for, and there are
/// none: an agent asking for one is carrying an instruction from before the
/// gates Topic went, so it is told the reading is gone rather than handed the
/// core Guide as though it were the Topic it asked for.
pub fn guide(topic: Option<&str>) -> Result<()> {
    if let Some(name) = topic {
        bail!(
            "no Guide topic named {name:?}. The Guide has no Topics: \
             `verkstead guide` prints the whole of it"
        );
    }

    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(CORE.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing the Guide to stdout")
}
