//! The Guide: what an agent needs in order to ask well, carried by the binary
//! that does the asking.
//!
//! The text is markdown in this repo, embedded at compile time rather than
//! fetched or assembled from anything outside — so what an agent reads is
//! exactly what was reviewed, and the binary alone is the whole documentation.
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
//! Which backend the reader is comes out of the environment: Verkstead sets the
//! agent type in every sandbox it starts a session in. A Guide printed with
//! nothing set — outside a sandbox, by a human at a terminal — is Claude's,
//! which is the blocking ask a human at a terminal has.

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

/// And where running one is, which is the backend's own — one apiece for the
/// two that block, and one the store-and-nudge backends share because what
/// they share is real: the Set is stored, the turn ends, and `verkstead
/// answers` is what fetches the Answers when the nudge lands.
const RUNNING_MARKER: &str = "<!-- running the ask, per backend -->";
const RUNNING_CLAUDE: &str = include_str!("../guide/running-claude.md");
const RUNNING_OPENCODE: &str = include_str!("../guide/running-opencode.md");
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

    let guide = tailored(reader()?);

    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(guide.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing the Guide to stdout")
}

/// Which backend this Guide is being printed for, and `None` where nothing
/// says.
///
/// Unset is a Guide read outside a sandbox, where nothing has said which
/// backend is reading it — a human at a terminal, whose ask blocks. A word this
/// binary does not know is refused by name rather than read past as one of
/// them: printing one backend's mechanism to another is a session sent after a
/// feature it has not got, or an ask wedged for hours.
fn reader() -> Result<Option<store::AgentType>> {
    let Ok(word) = std::env::var(AGENT_TYPE) else {
        return Ok(None);
    };

    let Ok(agent_type) = store::AgentType::read(&word) else {
        bail!("{AGENT_TYPE} names the agent type {word:?}, which this binary has not got");
    };

    Ok(Some(agent_type))
}

/// The core with each tailored section spliced in where it goes.
///
/// A marker that is not there is this crate's own file having drifted from this
/// module, which is a build to fix rather than a Guide to print — so it is
/// asserted rather than skipped past, and the suite is what catches it.
fn tailored(agent_type: Option<store::AgentType>) -> String {
    let channel = agent_type.map_or(store::Channel::Blocking, store::AgentType::channel);

    let kinds = match channel {
        store::Channel::Blocking => KINDS_BLOCKING,
        store::Channel::StoreAndNudge => KINDS_STORE_AND_NUDGE,
    };

    spliced(
        &spliced(CORE, KINDS_MARKER, kinds),
        RUNNING_MARKER,
        running(agent_type),
    )
}

/// How an ask is run, which is the backend's own rather than its channel's.
///
/// Matched on the type rather than on the channel so that a backend landing
/// beside these is a decision taken here rather than one inherited from
/// whichever channel it was given: the two that share a section share a
/// mechanism, and the two that block do not.
///
/// Nothing set reads Claude's, which is what a Guide printed outside a sandbox
/// has always been.
fn running(agent_type: Option<store::AgentType>) -> &'static str {
    match agent_type {
        None | Some(store::AgentType::Claude) => RUNNING_CLAUDE,
        Some(store::AgentType::OpenCode) => RUNNING_OPENCODE,
        Some(store::AgentType::Codex | store::AgentType::Grok) => RUNNING_STORE_AND_NUDGE,
    }
}

/// One marker replaced by the section that belongs there.
fn spliced(guide: &str, marker: &str, section: &str) -> String {
    assert!(
        guide.contains(marker),
        "the core Guide should carry the {marker} marker"
    );

    guide.replace(marker, section.trim_end())
}
