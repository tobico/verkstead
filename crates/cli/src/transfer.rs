//! `verkstead transfer <device>`: ask for this session's work to be moved onto
//! another device of the cluster (ADR-0020, *The agent's call*).
//!
//! `verkstead done`'s shape: a bare request the server writes down and the mover
//! acts on once this session has ended, with the Conversation in
//! `VERKSTEAD_SERVER` and nothing else naming it. The server decides there and
//! then — a device the human has not ticked, a name that is nobody's or two
//! machines', and whatever the preflight finds missing are each refused on
//! stderr with a non-zero exit, and nothing is written down.

use anyhow::Result;

use crate::client::Client;

/// Ask the server to move this Conversation onto `device`, by its name or id,
/// and say what it made of that.
pub fn transfer(device: &str, server: &str) -> Result<()> {
    let named = Client::new(server)?.transfer(device)?;

    println!(
        "Verkstead will move this work to {named} once this session has ended, and it will be \
         carried on there. Say anything you still have to say here now, then stop."
    );

    Ok(())
}
