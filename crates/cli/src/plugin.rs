//! `verkstead plugin`: what a plugin's author runs against the directory they
//! are writing one in.
//!
//! **`check` reads the manifest exactly as the server will** — the same reader,
//! linked from the server crate, so that a directory this passes is one the
//! Plugins page loads and a refusal here is the sentence that page would draw
//! (ADR-0030). It reads for every platform rather than this machine's alone,
//! because a `windows:` block is as much the author's as the common
//! declaration, and one wrong there is found on a machine the author may not
//! have; a refusal met on some platforms and not others says which.
//!
//! **And it starts the plugin's component**, where there is one, under the
//! capabilities the manifest asks for and through the server's own loader: a
//! component that is not one, needs what Verkstead does not provide, or traps
//! while it starts is the trouble the page would draw on its row, printed the
//! same way.
//!
//! Nothing here needs a server: a manifest is a file, and the check is a read
//! and an instantiation in this process.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Subcommand;
use verkstead_server::platform::{Environment, Platform, home_dir};
use verkstead_server::plugins::component;
use verkstead_server::plugins::manifest::{Files, Manifest, platform_key};

/// Which of the plugin verbs this is.
#[derive(Debug, Subcommand)]
pub enum What {
    /// Say whether a directory is a plugin Verkstead would load, and why not.
    ///
    /// Reads its `plugin.yaml` as each of Linux, macOS and Windows would and
    /// starts its component, and prints either what the plugin is or the
    /// refusal naming the field at fault, exiting non-zero on a refusal.
    Check {
        /// The plugin's directory: the one with `plugin.yaml` at its root.
        dir: PathBuf,
    },
}

pub fn plugin(what: What) -> Result<()> {
    match what {
        What::Check { dir } => check(&dir),
    }
}

fn check(dir: &Path) -> Result<()> {
    match Manifest::check(dir) {
        Ok(manifest) => {
            if manifest.component.is_some() {
                let home = home_dir(Platform::HERE, &Environment::of_the_process());
                let loaded = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(component::load(Files::Dir(dir), &manifest, home.as_deref()));
                if let Err(trouble) = loaded {
                    bail!(
                        "{} isn't a plugin Verkstead would load.\n{trouble}",
                        dir.display()
                    );
                }
            }

            let provides: Vec<&str> = manifest
                .provides
                .iter()
                .map(|provision| provision.category.word())
                .collect();
            let provides = if provides.is_empty() {
                "nothing".to_owned()
            } else {
                provides.join(", ")
            };

            println!(
                "{} is a plugin Verkstead would load: {} {} by {}, written against interface {}, \
                 providing {provides}.",
                dir.display(),
                manifest.name,
                manifest.version,
                manifest.author.name,
                manifest.interface,
            );
            Ok(())
        }
        Err(refusals) => {
            let lines: Vec<String> = refusals
                .iter()
                .map(|(refusal, platforms)| {
                    if platforms.len() == 3 {
                        refusal.why.clone()
                    } else {
                        let platforms: Vec<&str> = platforms
                            .iter()
                            .map(|platform| platform_key(*platform))
                            .collect();
                        format!("{} (read for {})", refusal.why, platforms.join(", "))
                    }
                })
                .collect();

            bail!(
                "{} isn't a plugin Verkstead would load.\n{}",
                dir.display(),
                lines.join("\n")
            )
        }
    }
}
