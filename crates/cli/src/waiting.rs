//! `verkstead waiting`: say this session is about to end its turn with work of
//! its own still running in the background.
//!
//! A session that ends its turn to wait on a build or a test run is idle by
//! every reading Verkstead has, and would be spoken to by the Rescue and then
//! put to the human. This is how it says so first (ADR-0018). A statement rather
//! than a command, and it returns at once: `verkstead ask` is the verb that
//! blocks.
//!
//! The length is checked by the server, which is where a wait stands: a length
//! that does not parse, or one longer than the maximum, comes back refused on
//! stderr naming the maximum, with a non-zero exit.

use anyhow::Result;

use crate::client::Client;

/// Tell the server this session is waiting for `length`, and say what it made
/// of that.
pub fn waiting(length: Option<&str>, server: &str) -> Result<()> {
    let taken = Client::new(server)?.waiting(length)?;
    let length = taken.strip_prefix("waiting ").unwrap_or(&taken);

    println!(
        "Verkstead has taken this session as waiting on work of its own for {length}. End your \
         turn; if the work runs over, run `verkstead waiting` again."
    );

    Ok(())
}
