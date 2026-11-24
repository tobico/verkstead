//! `verkstead pull-request open`: how a session opens the pull request its work
//! goes on (ADR-0030, *Forge*).
//!
//! The server opens it, through the Repo's Forge and as the Repo's Author, and
//! records it on the Conversation — so a session says what the pull request
//! should hold and nothing about where the Repo's pull requests live. Which is
//! what makes a skill read the same on a Forge that is not GitHub: the lines
//! that named a forge's own tool are one subcommand now.
//!
//! **A draft, always.** Merging is the human's act, and nothing a session does
//! is allowed to look like it was theirs.
//!
//! **Stdout carries the URL and nothing else**, as YAML, so a session can read
//! it back; the confirmation in words goes to stderr.

use std::io::Write;

use anyhow::{Context, Result};
use clap::Subcommand;
use verkstead_schema::PullRequestOpening;

use crate::client::Client;

/// Which of the pull request verbs this is.
#[derive(Debug, Subcommand)]
pub enum What {
    /// Open the pull request this session's work goes on, as a draft.
    ///
    /// Everything it is given is optional: the title is the branch's own name
    /// where none is given, the body nothing at all, and the base the branch
    /// this work was cut from, which the Conversation's record holds.
    Open {
        /// What the pull request is called. The branch's own name where none
        /// is given.
        #[arg(long)]
        title: Option<String>,

        /// What it says, as markdown. Nothing where none is given.
        #[arg(long)]
        body: Option<String>,

        /// The branch to open it against. The base on the Conversation's
        /// record where none is given, which is what this branch was cut
        /// from.
        #[arg(long, value_name = "BRANCH")]
        base: Option<String>,

        /// Where the Verkstead server is: its base URL, or `pipe://<name>` for
        /// a named pipe, which is Windows' own and what a session in a
        /// container asks through.
        #[arg(long, env = "VERKSTEAD_SERVER", default_value = crate::DEFAULT_SERVER)]
        server: String,
    },
}

pub fn pull_request(what: What) -> Result<()> {
    match what {
        What::Open {
            title,
            body,
            base,
            server,
        } => open(title.as_deref(), body.as_deref(), base.as_deref(), &server),
    }
}

/// Ask the server to open the pull request, and print where it is.
fn open(title: Option<&str>, body: Option<&str>, base: Option<&str>, server: &str) -> Result<()> {
    let opened = Client::new(server)?.open_pull_request(&PullRequestOpening {
        title: title.map(str::to_owned),
        body: body.map(str::to_owned),
        base: base.map(str::to_owned),
    })?;

    let yaml = serde_saphyr::to_string(&opened).context("writing where the pull request is")?;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(yaml.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing to stdout")?;

    eprintln!(
        "Verkstead opened the pull request as a draft and recorded it on this Conversation. \
         Leave it a draft: merging it is the human's."
    );

    Ok(())
}
