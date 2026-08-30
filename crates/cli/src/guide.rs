//! The Guide: what an agent needs in order to ask well, carried by the binary
//! that does the asking.
//!
//! The text is markdown in this repo, embedded at compile time rather than
//! fetched or assembled from anything outside — so what an agent reads is
//! exactly what was reviewed, and the binary alone is the whole documentation.
//!
//! One document, tailored at print time. What differs between backends is how
//! an ask is run and what comes back from it, which is a fact about the backend
//! and not about the Set — see [`store::Channel`]. So the two sections that
//! describe this end come in one per channel, spliced into the core at the
//! markers below, and everything about writing a Set is common and is written
//! once.
//!
//! Which channel is the reader's own comes out of the environment: Verkstead
//! sets the agent type in every sandbox it starts a session in. A Guide printed
//! with nothing set — outside a sandbox, by a human at a terminal — is the
//! blocking one.

use std::io::Write;

use anyhow::{Context, Result, bail};
use verkstead_server::sandbox::AGENT_TYPE;
use verkstead_server::store;

/// The core Guide, and since the gates Topic was retired the whole of it:
/// nothing in the pipeline gates a commit any more, so there is no task left
/// whose reading is worth deferring.
const CORE: &str = include_str!("../guide/core.md");

/// Where the two kinds of ask are described, per channel.
const KINDS_MARKER: &str = "<!-- the two kinds of ask, per channel -->";
const KINDS_BLOCKING: &str = include_str!("../guide/kinds-blocking.md");
const KINDS_STORE_AND_NUDGE: &str = include_str!("../guide/kinds-store-and-nudge.md");

/// And where running one is, which is the other half of what a channel decides.
const RUNNING_MARKER: &str = "<!-- running the ask, per channel -->";
const RUNNING_BLOCKING: &str = include_str!("../guide/running-blocking.md");
const RUNNING_STORE_AND_NUDGE: &str = include_str!("../guide/running-store-and-nudge.md");

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

    let guide = tailored(channel()?);

    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(guide.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing the Guide to stdout")
}

/// Which channel this Guide is being printed for.
///
/// Unset is [`store::Channel::Blocking`] — a Guide read outside a sandbox, where
/// nothing has said which backend is reading it, and where the blocking ask is
/// what a human at a terminal has. A word this binary does not know is refused
/// by name rather than read past as blocking: printing hold-the-ask advice to a
/// backend that cannot hold one is a session wedged for hours.
fn channel() -> Result<store::Channel> {
    let Ok(word) = std::env::var(AGENT_TYPE) else {
        return Ok(store::Channel::Blocking);
    };

    let Ok(agent_type) = store::AgentType::read(&word) else {
        bail!("{AGENT_TYPE} names the agent type {word:?}, which this binary has not got");
    };

    Ok(agent_type.channel())
}

/// The core with each channel-specific section spliced in where it goes.
///
/// A marker that is not there is this crate's own file having drifted from this
/// module, which is a build to fix rather than a Guide to print — so it is
/// asserted rather than skipped past, and the suite is what catches it.
fn tailored(channel: store::Channel) -> String {
    let (kinds, running) = match channel {
        store::Channel::Blocking => (KINDS_BLOCKING, RUNNING_BLOCKING),
        store::Channel::StoreAndNudge => (KINDS_STORE_AND_NUDGE, RUNNING_STORE_AND_NUDGE),
    };

    spliced(&spliced(CORE, KINDS_MARKER, kinds), RUNNING_MARKER, running)
}

/// One marker replaced by the section that belongs there.
fn spliced(guide: &str, marker: &str, section: &str) -> String {
    assert!(
        guide.contains(marker),
        "the core Guide should carry the {marker} marker"
    );

    guide.replace(marker, section.trim_end())
}
