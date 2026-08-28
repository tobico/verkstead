//! `verkstead ask`: read a Question Set, check it, enrich it, send it, and —
//! unless it was deferred — wait.

use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use verkstead_schema::QuestionSet;

use crate::client::Client;
use crate::repo;

/// Put a Question Set to the human and block until it is answered — or, where
/// `deferred` says so, leave it with them and return.
///
/// The Set is refused here, before anything is sent, if it breaks the question
/// grammar — the agent gets the same violations the server would have given it,
/// only sooner and naming the Question at fault.
///
/// A Deferred Ask is the same submission and the same Set on the same Timeline.
/// The only difference is on this end: nothing opens a wait, so the session goes
/// on working, and what stdout carries is the stored Set instead of a Response —
/// which is the whole of what there is to say at that point, and says which Set
/// it was.
pub fn ask(file: Option<&Path>, deferred: bool, server: &str) -> Result<()> {
    let yaml = read(file)?;

    let mut set = QuestionSet::from_yaml(&yaml)
        .map_err(|error| anyhow!("the Question Set is not well-formed YAML: {error}"))?;

    set.validate().map_err(|invalid| {
        let listed: Vec<String> = invalid
            .violations
            .iter()
            .map(|violation| format!("  {violation}"))
            .collect();
        anyhow!(
            "the Question Set breaks the question grammar:\n{}",
            listed.join("\n")
        )
    })?;

    // Whatever the agent put in these, the working directory is the authority
    // (ADR-0001), so they are overwritten rather than trusted. The Diff is the
    // authority's too and is not here: the server reads it off the Worktree the
    // Set is asked from, and overwrites whatever arrives in the same spirit.
    let cwd = std::env::current_dir().context("reading the working directory")?;
    let derived = repo::enrichment(&cwd);
    set.project = derived.project;
    set.branch = derived.branch;

    // A wait that goes to plan is silent. A harness runs this in the background
    // and captures both streams into one file, so anything said here on the way
    // would arrive ahead of the Response the agent came for.
    let client = Client::new(server);
    let created = client.submit(&set, deferred)?;

    // The Response is the CLI's whole output: the agent parses stdout, so
    // nothing else has ever been written there. A Deferred Ask has no Response
    // to print and never will have on this end, so what goes there is the one
    // thing that did happen — the Set was stored, and this is which one.
    let mut yaml = match deferred {
        true => created
            .to_yaml()
            .context("rendering the stored Question Set as YAML")?,
        false => client
            .wait(created.id)?
            .to_yaml()
            .context("rendering the Response as YAML")?,
    };

    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }

    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(yaml.as_bytes())
        .and_then(|()| stdout.flush())
        .context("writing to stdout")
}

/// The Set as the agent gave it: from a file, or from stdin when there is no
/// file argument.
fn read(file: Option<&Path>) -> Result<String> {
    let yaml = match file {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("reading the Question Set from {}", path.display()))?,
        None => {
            let mut yaml = String::new();
            std::io::stdin()
                .read_to_string(&mut yaml)
                .context("reading the Question Set from stdin")?;
            yaml
        }
    };

    if yaml.trim().is_empty() {
        match file {
            Some(path) => bail!("{} is empty", path.display()),
            None => bail!("no Question Set arrived on stdin"),
        }
    }

    Ok(yaml)
}
