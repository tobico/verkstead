//! The models an account can run, asked of its harness — see
//! [`verkstead_render::ModelsListed`].
//!
//! **OpenCode alone has a way to say.** `opencode models` prints every model
//! that account reaches, one `provider/model` id to a line: the free ones its
//! own provider serves, every provider the account holds a key for, and the
//! local servers — LM Studio, Ollama — it found running. None of that is
//! anything a list kept in the viewer could know, which is why the viewer's
//! list for OpenCode went stale the week it was written. Every other harness
//! has no such command, or one behind a login a listing would have to run, and
//! is [`ModelsListed::Unasked`]: its ticks are the viewer's own.
//!
//! **Run on the host, as the server's own process, not in a sandbox.** A listing
//! is no session: nothing in it is an agent's, it asks no model anything, and it
//! has to answer while the human is looking at a form — which a sandbox's root,
//! and on Windows a boundary written onto the account's directories, does not.
//! What it is given is the account's own XDG directories, the two the sandbox
//! would bind, so what it reads is that account's configuration and nobody
//! else's. Plugins the account's configuration names are loaded by it all the
//! same; that is the account's owner's code, run as them.
//!
//! **`--standalone` first.** opencode 2 talks to a background service of the
//! human's by default, which serves whichever account started it — so a listing
//! for another account asks for a private server of its own. opencode 1 has no
//! such flag, and a refused first try is asked again without it.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use verkstead_render::{ModelsListed, ProfileAccount};

use crate::AppState;
use crate::sandbox::{OPENCODE_CONFIG_INSIDE_HOME, OPENCODE_DATA_INSIDE_HOME};
use crate::store;
use crate::unseen::Unseen;

/// How long one listing is waited on before it is given up: **thirty seconds**.
///
/// opencode 2.4.2 answers in about one, the free provider's list being read off
/// a cache it refreshes behind the answer. The bound is for the harness that
/// hangs — on a network that never answers, or on a prompt nobody will see —
/// and a form waiting on it longer than this is a form nobody is still at.
const LIMIT: Duration = Duration::from_secs(30);

/// What the account the form names can run, asked of its harness on this
/// device.
///
/// An account that would not save, a harness that has no listing, and a
/// harness that is not on this machine are all [`ModelsListed::Unasked`]: the
/// save says what is wrong with the paths, the dependencies step says what is
/// missing, and a form drawn over either offers its own list.
pub(crate) async fn listed(state: &AppState, account: &ProfileAccount) -> ModelsListed {
    let machine = state.onboarding.machine().clone();
    let account = account.clone();

    let asked = tokio::task::spawn_blocking(move || {
        let account = crate::profiles::inspect(&account).ok()?;

        match account {
            store::Account::OpenCode { home } => Some((
                machine.found(crate::sessions::binary(store::AgentType::OpenCode))?,
                home,
            )),
            _ => None,
        }
    })
    .await
    .ok()
    .flatten();

    match asked {
        Some((program, home)) => opencode(&program, &home, LIMIT).await,
        None => ModelsListed::Unasked,
    }
}

/// What `opencode models` lists for the account kept under `home`, run as
/// `program` and waited on for `limit`.
async fn opencode(program: &Path, home: &Path, limit: Duration) -> ModelsListed {
    match asking(program, home, &["models", "--standalone"], limit).await {
        Asked::Said(said) => ModelsListed::Listed(ids(&said)),

        // opencode 1, which has no `--standalone`, or an opencode 2 that
        // failed for its own reasons — asked once more without it, and what
        // that one says is the answer.
        Asked::Refused(_) => match asking(program, home, &["models"], limit).await {
            Asked::Said(said) => ModelsListed::Listed(ids(&said)),
            Asked::Refused(why) | Asked::Unanswered(why) => ModelsListed::Refused(why),
        },

        Asked::Unanswered(why) => ModelsListed::Refused(why),
    }
}

/// What one run of the harness came to.
enum Asked {
    /// It exited well, having printed this.
    Said(String),

    /// It exited badly — the sentence saying so, worth asking again.
    Refused(String),

    /// It could not be run, or did not answer in time — the sentence saying
    /// so, and nothing asking again would change.
    Unanswered(String),
}

/// Run `program` with `words`, given the account under `home` as its own.
async fn asking(program: &Path, home: &Path, words: &[&str], limit: Duration) -> Asked {
    let mut command = tokio::process::Command::new(program);

    command
        .args(words)
        .env("XDG_CONFIG_HOME", base(home, OPENCODE_CONFIG_INSIDE_HOME))
        .env("XDG_DATA_HOME", base(home, OPENCODE_DATA_INSIDE_HOME))
        .current_dir(home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .unseen();

    let output = match tokio::time::timeout(limit, command.output()).await {
        Ok(Ok(output)) => output,

        Ok(Err(why)) => {
            tracing::warn!(
                error = ?why,
                program = %program.display(),
                "opencode could not be run to list an account's models",
            );

            return Asked::Unanswered(format!("OpenCode couldn't be run: {why}."));
        }

        Err(_) => {
            return Asked::Unanswered(format!(
                "OpenCode didn't list its models within {} seconds.",
                limit.as_secs()
            ));
        }
    };

    if output.status.success() {
        return Asked::Said(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    let complained = String::from_utf8_lossy(&output.stderr);
    let last = complained
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty());

    Asked::Refused(match last {
        Some(line) => format!(
            "OpenCode couldn't list its models: {}.",
            line.trim_end_matches('.')
        ),
        None => format!("OpenCode couldn't list its models ({}).", output.status),
    })
}

/// The XDG base directory a path inside `home` sits under: `home/.config` for
/// `.config/opencode`.
fn base(home: &Path, inside: &str) -> PathBuf {
    let inside = Path::new(inside);

    home.join(inside.parent().unwrap_or(inside))
}

/// The ids in what `opencode models` printed, in its order, each once.
///
/// A line is an id where it is one word with a provider before a slash —
/// `opencode/big-pickle`, `ollama/qwen3-coder:30b`, `lmstudio/google/gemma` —
/// which is the whole of what the command prints. Anything else, a banner or a
/// warning a later release puts on standard output, is not a model.
fn ids(said: &str) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();

    for line in said.lines().map(str::trim) {
        let one_word = !line.is_empty() && !line.contains(char::is_whitespace);
        let provided = line
            .split_once('/')
            .is_some_and(|(provider, model)| !provider.is_empty() && !model.is_empty());

        if one_word && provided && !ids.iter().any(|id| id == line) {
            ids.push(line.to_owned());
        }
    }

    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_opencode_2_4_2_prints_is_read_as_its_ids() {
        let said = "lmstudio/google/gemma-4-e4b\n\
                    ollama/qwen3-coder:30b\n\
                    opencode/big-pickle\n\
                    opencode/big-pickle\n";

        assert_eq!(
            ids(said),
            [
                "lmstudio/google/gemma-4-e4b",
                "ollama/qwen3-coder:30b",
                "opencode/big-pickle",
            ],
        );
    }

    #[test]
    fn a_line_that_is_not_an_id_is_not_a_model() {
        let said = "\nUpdate available: 2.5.0\n/leading\ntrailing/\nplain\nopencode/beam-free\n";

        assert_eq!(ids(said), ["opencode/beam-free"]);
    }

    #[test]
    fn the_xdg_bases_are_the_parents_of_what_the_sandbox_binds() {
        let home = Path::new("/accounts/free");

        assert_eq!(
            base(home, OPENCODE_CONFIG_INSIDE_HOME),
            Path::new("/accounts/free/.config"),
        );
        assert_eq!(
            base(home, OPENCODE_DATA_INSIDE_HOME),
            Path::new("/accounts/free/.local/share"),
        );
    }
}
