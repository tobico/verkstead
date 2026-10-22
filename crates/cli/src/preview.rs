//! `verkstead preview`: suggest how to try this session's work by hand
//! (ADR-0024).
//!
//! The title as the argument and the command on stdin, the way `verkstead
//! memo` takes its markdown — and under the command, after a blank line, the
//! markdown description its card shows, read the way a commit message is. A
//! suggestion the server would refuse for its shape — an empty title or
//! command, a command that runs onto a second line, or a command or a
//! description over its maximum — is refused here before anything is sent, as
//! a courtesy; whether this session may suggest one at all is the server's to
//! say.
//!
//! **Stdout carries the title and nothing else**, as YAML; the confirmation in
//! words goes to stderr.

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use verkstead_schema::PreviewSuggestion;

use crate::client::Client;

/// Suggest the command on stdin under `title`, and print the title it stands
/// under.
pub fn preview(title: &str, server: &str) -> Result<()> {
    let mut written = String::new();
    std::io::stdin()
        .read_to_string(&mut written)
        .context("reading the preview's command from stdin")?;

    let suggestion = PreviewSuggestion::read(title, &written);

    if let Some(why) = suggestion.refused() {
        bail!("{why}");
    }

    let left = Client::new(server)?.preview(&suggestion)?;

    let yaml = serde_saphyr::to_string(&left).context("writing the preview's title")?;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(yaml.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing to stdout")?;

    eprintln!(
        "Verkstead has suggested preview {:?}: the human can run it from the Code pane with a \
         press.",
        left.title
    );

    Ok(())
}

/// Take back the preview standing under `title`, and say so on stderr alone.
pub fn retire(title: &str, server: &str) -> Result<()> {
    Client::new(server)?.retire_preview(title.trim())?;

    eprintln!("Verkstead has retired preview {title:?}: it is no longer offered in the Code pane.");

    Ok(())
}
