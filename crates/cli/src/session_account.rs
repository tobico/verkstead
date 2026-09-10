//! The two elevated verbs: making the local account a Windows session runs as,
//! and taking it away again.
//!
//! **Why a verb rather than the installer** (ADR-0014, *Amended: the Sandbox is
//! an account*). The account and its password are made together and the
//! password has to land in the Data Directory — which the msi does not know and
//! may not have made yet — so what creates one is Verkstead's own binary, run
//! once from an elevated terminal, rather than a custom action in a package.
//! Everything after that is unprivileged: the server only ever reads the
//! account.
//!
//! **What they print is what happened**, one line per thing that was there to
//! do something about. A verb run twice says the account was already there
//! rather than failing, and a removal says which of the account, its profile
//! directory and its password it actually found — a machine that has run
//! sessions and then had Verkstead taken off it should look as it did, and
//! saying what was removed is how somebody checks.
//!
//! Windows' own. There is no such account on the other platforms and no such
//! verb: their sessions are a namespace and a policy, and neither is anybody's
//! to be.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use verkstead_server::sandbox::account::machine::{self, Made};

/// Which of the two this is.
#[derive(Debug, Subcommand)]
pub enum What {
    /// Create the account and write its password beside the other secrets.
    ///
    /// Needs an elevated terminal: creating a local account is an
    /// administrator's call. Run again on a machine that already has one, it
    /// says so rather than failing.
    Create(Which),

    /// Delete the account, its profile directory and its password.
    ///
    /// Needs an elevated terminal too. Nothing here is a failure for not having
    /// been there, so this is also how a half-finished install is tidied up.
    Remove(Which),
}

/// Whose account, which is to say which Data Directory's.
#[derive(Debug, Args)]
pub struct Which {
    /// The Data Directory whose sessions this account is for.
    ///
    /// The same flag `verkstead serve` takes and resolved the same way, because
    /// the account's name comes off this path: pointed at two different
    /// directories, the two verbs would make and remove two different accounts.
    #[arg(long, env = "VERKSTEAD_DATA_DIR", value_name = "DIR")]
    data_dir: Option<PathBuf>,
}

impl Which {
    /// The Data Directory itself, made where it is not there yet.
    ///
    /// Made rather than merely resolved, for the reason
    /// [`verkstead_server::Config::data_directory`] makes it: the name is
    /// fingerprinted off the *resolved* path, and a directory that is not there
    /// resolves to whatever was typed. So a create that made the directory and
    /// a remove that did not would be talking about two different accounts.
    fn directory(&self) -> Result<PathBuf> {
        let data_dir = verkstead_server::platform::data_dir(self.data_dir.as_deref())?;

        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating data directory {}", data_dir.display()))?;

        Ok(data_dir)
    }
}

/// Run whichever of the two this is, and say what it did.
pub fn session_account(what: What) -> Result<()> {
    match what {
        What::Create(which) => create(&which),
        What::Remove(which) => remove(&which),
    }
}

fn create(which: &Which) -> Result<()> {
    let data_dir = which.directory()?;
    let (name, made) = machine::create(&data_dir)?;

    match made {
        Made::Created => {
            println!("account   = {name} created");
            println!("password  = written to {}", secrets(&data_dir).display());
        }
        Made::AlreadyThere => {
            println!("account   = {name} was already there");
            println!(
                "password  = the one already in {}",
                secrets(&data_dir).display(),
            );
        }
        Made::PasswordSetAgain => {
            println!("account   = {name} was already there");
            println!(
                "password  = nothing held it, so a new one was set and written to {}",
                secrets(&data_dir).display(),
            );
        }
    }

    println!("next      = this Verkstead's Windows sessions will run as {name}");

    Ok(())
}

fn remove(which: &Which) -> Result<()> {
    let data_dir = which.directory()?;
    let (name, removed) = machine::remove(&data_dir)?;

    println!(
        "account   = {name} {}",
        said(removed.account, "deleted", "was not on this machine"),
    );
    println!(
        "profile   = {}",
        said(
            removed.profile,
            "its profile directory removed",
            "it had no profile directory, so no session had ever run as it",
        ),
    );
    println!(
        "password  = {}",
        said(
            removed.password,
            "taken out of the secrets file",
            "there was none in the secrets file",
        ),
    );

    Ok(())
}

/// Where the password went, said the way the settings page says it.
fn secrets(data_dir: &std::path::Path) -> PathBuf {
    verkstead_server::settings::Settings::in_data_dir(data_dir).secrets_path()
}

/// One of two words, so that every line here says what was true rather than
/// leaving a blank where nothing was found.
fn said<'a>(there: bool, was: &'a str, was_not: &'a str) -> &'a str {
    if there { was } else { was_not }
}
