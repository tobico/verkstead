//! The Guide: what an agent needs in order to ask well, carried by the binary
//! that does the asking.
//!
//! The core is markdown in this repo, embedded at compile time rather than
//! fetched — so what an agent reads of it is exactly what was reviewed, and
//! the binary alone is the whole of it wherever nothing else answers.
//!
//! One document, tailored at print time. What differs between backends is how
//! an ask is run and what comes back from it, which is a fact about the backend
//! and not about the Set. So the two sections that describe this end are
//! spliced into the core at the markers below, and everything about writing a
//! Set is common and is written once.
//!
//! **The two are tailored at different grains, and that is the backends rather
//! than a scheme.** Which kinds of ask a backend has is its channel's — see
//! [`store::Channel`] — because that is the whole of what the channel decides:
//! whether an ask waits or is stored. *Running* one is the backend's own: the
//! two blocking backends share a channel and not a mechanism, Claude Code
//! holding the ask open in a background shell call its harness wakes it from
//! and opencode holding it in a synchronous shell call given a long enough
//! timeout. A Guide that told either to do the other's would send it after a
//! harness feature it has not got.
//!
//! **Both are the server's to say** (ADR-0030, *Agent*; Set 1764, Q18): a
//! harness may be a plugin's, whose section is what its manifest wrote, so the
//! binary carries the core and asks the server for the rest by the harness
//! `VERKSTEAD_AGENT` names, which Verkstead sets in every sandbox it starts a
//! session in. A Guide printed with nothing set — outside a sandbox, by a human
//! at a terminal — or where the server cannot be reached is the core alone:
//! the blocking kinds, the ask a human at a terminal has, and no section on
//! running one.

use std::io::Write;

use anyhow::{Context, Result, bail};
use verkstead_server::sandbox::AGENT_TYPE;

use crate::client::{Client, Guided};

/// The core Guide, and since the gates Topic was retired the whole of it:
/// nothing in the pipeline gates a commit any more, so there is no task left
/// whose reading is worth deferring.
const CORE: &str = include_str!("../guide/core.md");

/// Where the two kinds of ask are described, per channel.
const KINDS_MARKER: &str = "<!-- the two kinds of ask, per channel -->";
const KINDS_BLOCKING: &str = include_str!("../guide/kinds-blocking.md");
const KINDS_STORE_AND_NUDGE: &str = include_str!("../guide/kinds-store-and-nudge.md");

/// And where running one is, which is the backend's own and the server's to
/// say.
const RUNNING_MARKER: &str = "<!-- running the ask, per backend -->";

/// Where the server is when the environment does not say, as every verb that
/// reaches it has it.
const SERVER: &str = "VERKSTEAD_SERVER";

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

    let guide = tailored(fetched()?);

    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(guide.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing the Guide to stdout")
}

/// The tailored half for the harness this Guide is printed for, or `None`
/// where nothing says which or the server cannot be asked.
///
/// An id the server answers it has no harness of is refused by name rather
/// than printed past as the core: the variable is Verkstead's own to set, and
/// a Guide that left a backend's mechanism out for a mistake in it is a
/// session sent after an ask it does not know how to hold.
fn fetched() -> Result<Option<verkstead_schema::GuideSection>> {
    let Ok(agent) = std::env::var(AGENT_TYPE) else {
        return Ok(None);
    };

    let server = std::env::var(SERVER).unwrap_or_else(|_| crate::DEFAULT_SERVER.to_owned());
    let Ok(client) = Client::new(&server) else {
        return Ok(None);
    };

    match client.guide(&agent) {
        Guided::Section(section) => Ok(Some(section)),
        Guided::NoSuchHarness(said) => {
            bail!("{AGENT_TYPE} names the agent type {agent:?}, which the server refused: {said}")
        }
        Guided::Unreached => Ok(None),
    }
}

/// The core with each tailored section spliced in where it goes — the
/// blocking kinds and no running section where there is none.
///
/// A marker that is not there is this crate's own file having drifted from this
/// module, which is a build to fix rather than a Guide to print — so it is
/// asserted rather than skipped past, and the suite is what catches it.
fn tailored(section: Option<verkstead_schema::GuideSection>) -> String {
    let (blocking, running) = section.map_or((true, String::new()), |section| {
        (section.blocking, section.running)
    });

    let kinds = match blocking {
        true => KINDS_BLOCKING,
        false => KINDS_STORE_AND_NUDGE,
    };

    spliced(
        &spliced(CORE, KINDS_MARKER, kinds),
        RUNNING_MARKER,
        &running,
    )
}

/// One marker replaced by the section that belongs there.
fn spliced(guide: &str, marker: &str, section: &str) -> String {
    assert!(
        guide.contains(marker),
        "the core Guide should carry the {marker} marker"
    );

    match section.trim_end() {
        // Nothing to splice, and the blank line it would have stood between
        // goes with it.
        "" => guide
            .replacen(&format!("{marker}\n\n"), "", 1)
            .replace(marker, ""),
        section => guide.replace(marker, section),
    }
}
