//! `verkstead memo`: leave what this session learned for the sessions after it
//! on the same Conversation (ADR-0023).
//!
//! Markdown on stdin, the way `verkstead ask` takes its Set, and the
//! Conversation in `VERKSTEAD_SERVER` the way every verb here has it. An empty
//! memo and one over the maximum are refused here before anything is sent — a
//! courtesy, since the server holds the same rule — and whether this session
//! may leave one at all is the server's to say, because it knows what the
//! session was sent for.
//!
//! **Stdout carries the id and nothing else**, as YAML, so an agent can read it
//! back; the confirmation in words goes to stderr.

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use verkstead_schema::memo_refused;

use crate::client::Client;

/// Leave the memo on stdin, and print the Event it became.
pub fn memo(server: &str) -> Result<()> {
    let mut markdown = String::new();
    std::io::stdin()
        .read_to_string(&mut markdown)
        .context("reading the memo from stdin")?;

    if let Some(why) = memo_refused(&markdown) {
        bail!("{why}");
    }

    let left = Client::new(server)?.memo(&markdown)?;

    let yaml = serde_saphyr::to_string(&left).context("writing the memo's id")?;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(yaml.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing to stdout")?;

    eprintln!(
        "Verkstead has left memo {} for every later session of this Conversation, which will be \
         shown it in its prompt.",
        left.id
    );

    Ok(())
}

/// Take memo `id` back, with the rest of the line as the reason, and say so on
/// stderr alone: there is nothing for an agent to read back.
pub fn retire(id: i64, reason: &[String], server: &str) -> Result<()> {
    let reason = reason.join(" ");

    Client::new(server)?.retire_memo(id, reason.trim())?;

    eprintln!(
        "Verkstead has retired memo {id}: no later session of this Conversation will be shown it."
    );

    Ok(())
}
