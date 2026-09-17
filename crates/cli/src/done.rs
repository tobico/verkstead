//! `verkstead done`: say this session's work is finished.
//!
//! The one thing that ends a session on purpose (ADR-0018). A bare verb, because
//! the server knows which session is running in the Conversation and what it was
//! sent for — the Conversation is in `VERKSTEAD_SERVER`, exactly as it is for
//! `verkstead ask`. What the server does with it is check the repository there
//! and then: accepted, the session is ended once it next goes idle; refused, it
//! is left running, and what is missing comes back on stderr to be put right in
//! the same turn.

use anyhow::Result;

use crate::client::Client;

/// Tell the server this session is done, and say what it made of that.
pub fn done(server: &str) -> Result<()> {
    Client::new(server)?.done()?;

    println!(
        "Verkstead has taken this session as done, and will end it once you stop. \
         Say anything you still have to say now."
    );

    Ok(())
}
