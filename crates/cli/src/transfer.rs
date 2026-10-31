//! `verkstead transfer <device>`: ask for this session's work to be moved onto
//! another device of the cluster (ADR-0020, *The agent's call*).
//!
//! `verkstead done`'s shape: a bare request the server writes down and the mover
//! acts on once this session has ended, with the Conversation in
//! `VERKSTEAD_SERVER` and nothing else naming it. The server decides there and
//! then — a name that is nobody's or two machines', and whatever the preflight
//! finds missing are each refused on stderr with a non-zero exit, and nothing
//! is written down.
//!
//! **A device not ticked, or ticked and asleep, is a request rather than a
//! refusal** (ADR-0029): the server answers 202 rather than 200, the session
//! ends the same way, and the human is asked whether the work moves there.

use anyhow::Result;

use crate::client::{Called, Client};

/// Ask the server to move this Conversation onto `device`, by its name or id,
/// and say what it made of that.
pub fn transfer(device: &str, server: &str) -> Result<()> {
    let (named, called) = Client::new(server)?.transfer(device)?;

    match called {
        Called::Moving => println!(
            "Verkstead will move this work to {named} once this session has ended, and it will \
             be carried on there. Say anything you still have to say here now, then stop."
        ),
        Called::Asked => println!(
            "{named} can't take the work yet, so Verkstead is asking the human whether to move \
             it there. This session ends once this turn is over either way: the work moves to \
             {named} and is carried on there, or this conversation carries on here with a note \
             saying it stays. Say anything you still have to say here now, then stop."
        ),
    }

    Ok(())
}
