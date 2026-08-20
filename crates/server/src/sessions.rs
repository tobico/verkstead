//! The sessions a Conversation's work is actually done by: an agent running
//! inside its sandbox, and everything it prints on its way to the Timeline.
//!
//! The technique is roadrunner's `session.ts`, and only the technique — that is
//! TypeScript on Bun and this is Rust, so nothing is carried over but the shape
//! and the reasons for it. The session runs on a pseudo-terminal of its own,
//! allocated by `script --quiet --return --command … /dev/null`, because claude
//! needs a terminal to behave like itself and `script` is already fluent in the
//! raw modes and window-size handling this would otherwise be reimplementing.
//! Its output is relayed, kept whole, and summarised.
//!
//! `script` runs *inside* the sandbox rather than around it. The pseudo-terminal
//! is then the sandbox's own — bwrap makes it one when it makes `/dev` — and
//! what the server spawns stays one argument vector instead of a bwrap command
//! line rendered back into a shell string for `script --command` to take apart
//! again.
//!
//! The session is interactive and never `-p`: it idles when it has nothing to
//! do, which is what a blocking ask depends on (ADR 0001 in tobico-skills).
//!
//! Whether a session is running is held here and nowhere else. A running session
//! is a process, and no table can hold one — a restarted server has no sessions
//! at all, and that is exactly what a Conversation should then say rather than
//! reading back a live one out of a database.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use sqlx::SqlitePool;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::nudge::Nudges;
use crate::sandbox::{Home, Sandbox, SandboxConfig, under_dev_shell};
use crate::skills::Skills;
use crate::store;
use crate::transcript::Reading;

/// How much of a session's output to take off the pseudo-terminal at once.
const CHUNK: usize = 8 * 1024;

/// How often what a session has printed reaches the store and the open pages.
///
/// A terminal application redraws many times a second, and a store written and
/// a page nudged on every one of those would be a cost that scaled with how
/// busy the agent's spinner was. Half a second is under what a human reads as a
/// delay and two orders of magnitude off what a redraw costs.
const FLUSH_EVERY: Duration = Duration::from_millis(500);

/// How a Conversation's agents are run: the home a sandbox reads the machine's
/// identity out of, the extra binds Sandbox Configuration asks for, the skills
/// every sandbox is given, and what an agent is on the command line.
///
/// Resolved once at startup and shared by every session, because each of the
/// four is a fact about the machine rather than about any one Conversation.
#[derive(Debug, Clone)]
pub struct Agents {
    home: Home,
    config: SandboxConfig,
    skills: Skills,

    /// What a Profile's agent is run as, before its model and its prompt.
    ///
    /// A field rather than a match on the agent type, and the reason is that
    /// what this module has to be able to prove is that a session's output
    /// reaches the Timeline while it is still running. Proving it against the
    /// real claude would be a test that needed an account, a network and a
    /// model's patience — so a test stands its own program where claude goes,
    /// and everything from the sandbox outwards is the same code the server
    /// runs.
    agent: Vec<String>,
}

impl Agents {
    /// The real thing: claude, under whichever account the Profile names.
    pub fn new(home: Home, config: SandboxConfig, skills: Skills) -> Agents {
        Agents::running(vec!["claude".to_owned()], home, config, skills)
    }

    /// The same, with something else where claude goes — see [`Agents::agent`].
    pub fn running(
        agent: Vec<String>,
        home: Home,
        config: SandboxConfig,
        skills: Skills,
    ) -> Agents {
        Agents {
            home,
            config,
            skills,
            agent,
        }
    }

    /// What a session for `profile` on `prompt` runs.
    ///
    /// The model is the Profile's, said on the command line rather than left to
    /// whatever the account's own settings hold: which model a session runs is
    /// half of what an Agent Profile *is*. The prompt goes last, which is where
    /// an interactive claude takes the thing it is to start on.
    fn argv(&self, profile: &store::Profile, prompt: &str) -> Vec<String> {
        let mut argv = self.agent.clone();
        argv.push("--model".to_owned());
        argv.push(profile.model.clone());
        argv.push(prompt.to_owned());
        argv
    }
}

/// The sessions this server has running, by the Conversation each belongs to.
///
/// One per Conversation: a Conversation is one piece of work, and two agents in
/// one worktree would be two agents editing each other's files.
#[derive(Clone)]
pub(crate) struct Sessions {
    /// How to run one, or `None` where this server cannot run any — which is
    /// every router but the one the binary serves, and is why starting a
    /// grilling makes the worktree either way.
    agents: Option<Arc<Agents>>,

    running: Arc<Mutex<HashMap<i64, Running>>>,
}

/// One running session, as whatever wants to stop it sees it.
struct Running {
    /// The Timeline Event its output is being written into.
    event_id: i64,

    /// Word to the relay that this session is to end. Dropped rather than sent
    /// where the session ended by itself.
    stop: oneshot::Sender<()>,

    /// The relay itself, so that ending a session can wait for it to be over
    /// rather than only ask.
    relay: JoinHandle<()>,
}

impl Sessions {
    /// A server that can run sessions, under `agents`.
    pub(crate) fn under(agents: Agents) -> Sessions {
        Sessions {
            agents: Some(Arc::new(agents)),
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// One that cannot: nothing is launched, and everything else about starting
    /// a grilling holds.
    pub(crate) fn none() -> Sessions {
        Sessions {
            agents: None,
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Which Timeline Event a Conversation's running session is printing into,
    /// or `None` where it has none running.
    ///
    /// Asked per Event rather than per Conversation because that is what the
    /// answer is about: a Timeline holds every session a Conversation has ever
    /// had, and only one of them can still be talking.
    pub(crate) fn writing(&self, conversation_id: i64) -> Option<i64> {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .get(&conversation_id)
            .map(|running| running.event_id)
    }

    /// Run `profile`'s agent on `prompt`, inside `conversation`'s sandbox, and
    /// put what it prints on the Timeline as it arrives.
    ///
    /// Whether one was started. A server with no way to run agents starts none,
    /// and a sandbox that cannot be built — a Conversation with no worktree, or
    /// one git will not own — is the same answer: there is nothing here to
    /// launch. Both are logged, because both mean a Conversation that is
    /// grilling with nothing grilling it.
    ///
    /// The Timeline Event is made after the process is, so that a session that
    /// never started leaves no transcript of nothing.
    pub(crate) async fn start(
        &self,
        pool: &SqlitePool,
        nudges: &Nudges,
        conversation: &store::Conversation,
        profile: &store::Profile,
        prompt: &str,
    ) -> Result<bool> {
        let Some(agents) = self.agents.clone() else {
            tracing::warn!(
                conversation_id = conversation.id,
                "this server has no way to run an agent, so no session was started"
            );
            return Ok(false);
        };

        let argv = agents.argv(profile, prompt);
        let conversation_id = conversation.id;

        // The sandbox asks git where the worktree's object database is, and the
        // dev-shell question is a `nix eval` or two. Both block, and both are
        // decided before anything is spawned.
        let built = tokio::task::spawn_blocking({
            let conversation = conversation.clone();
            let profile = profile.clone();
            let home = agents.home.clone();
            let skills = agents.skills.clone();
            let extra = agents.config.binds_for(&conversation.repo.name);

            move || {
                let sandbox =
                    Sandbox::for_conversation(&conversation, &profile, home, &skills, extra)?;
                let worktree = conversation.worktree.clone()?;

                Some((sandbox, under_dev_shell(&worktree, &argv)))
            }
        })
        .await?;

        let Some((sandbox, argv)) = built else {
            tracing::error!(
                conversation_id,
                "there is no sandbox to run a session in, so none was started"
            );
            return Ok(false);
        };

        let mut child = match captured(&sandbox, &argv).spawn() {
            Ok(child) => child,
            Err(error) => {
                tracing::error!(
                    error = ?error,
                    conversation_id,
                    "a grilling session could not be started"
                );
                return Ok(false);
            }
        };

        let event_id = store::start_transcript(pool, conversation_id).await?;

        let (stop, stopping) = oneshot::channel();

        // Registered under the lock the relay will want in order to take itself
        // off again, and started while it is held. A session that ends the
        // instant it starts — an agent that is not installed, a prompt it
        // refuses — would otherwise be one that finished before it was written
        // down, and a Conversation left claiming a session nothing could stop.
        {
            let mut running = self
                .running
                .lock()
                .expect("the sessions registry is not poisoned");

            let relay = tokio::spawn({
                let sessions = self.clone();
                let pool = pool.clone();
                let nudges = nudges.clone();

                async move {
                    relay(&pool, &nudges, event_id, &mut child, stopping).await;

                    // Off the register before the last Nudge, so that a page
                    // reading the Conversation back reads a session that has
                    // ended.
                    sessions.forget(conversation_id, event_id);
                    nudges.announce();
                }
            });

            running.insert(
                conversation_id,
                Running {
                    event_id,
                    stop,
                    relay,
                },
            );
        }

        // The Event is on the Timeline as of now, empty. Whoever is watching
        // should see the session appear rather than see it once it says
        // something.
        nudges.announce();

        tracing::info!(conversation_id, event_id, "a grilling session is running");

        Ok(true)
    }

    /// End a Conversation's session, and wait until it is over.
    ///
    /// Waited on rather than only asked for, because of what happens next: the
    /// worktree goes, and a session still writing in a directory being removed
    /// is the failure this is here to make impossible.
    ///
    /// A Conversation with no session running is nothing to do, which is every
    /// Conversation that was never started and every one whose session has
    /// already ended.
    pub(crate) async fn end(&self, conversation_id: i64) {
        let running = self
            .running
            .lock()
            .expect("the sessions registry is not poisoned")
            .remove(&conversation_id);

        let Some(running) = running else {
            return;
        };

        // A relay that has already finished has dropped its end of this, which
        // is the same instruction arriving too late to be needed.
        let _ = running.stop.send(());

        if let Err(error) = running.relay.await {
            tracing::error!(error = ?error, conversation_id, "a session's relay ended badly");
        }
    }

    /// Take a session off the register, where it is still the one registered.
    ///
    /// By the Event it was writing into as well as by its Conversation: a relay
    /// finishing is the last thing a session does, and by then another one may
    /// have been started against the same Conversation.
    fn forget(&self, conversation_id: i64, event_id: i64) {
        let mut running = self
            .running
            .lock()
            .expect("the sessions registry is not poisoned");

        if running
            .get(&conversation_id)
            .is_some_and(|running| running.event_id == event_id)
        {
            running.remove(&conversation_id);
        }
    }
}

/// `argv` as a session: run inside `sandbox`, on a pseudo-terminal of its own.
///
/// `script` takes one command line rather than an argument vector, so the
/// vector is quoted into one — and `exec`ed, so that the shell `script` starts
/// becomes the session rather than standing between it and whatever ends it.
fn captured(sandbox: &Sandbox, argv: &[String]) -> Command {
    let line = shell_command(argv);

    let session: Vec<&OsStr> = vec![
        OsStr::new("script"),
        OsStr::new("--quiet"),
        // The session's own exit status rather than `script`'s.
        OsStr::new("--return"),
        OsStr::new("--command"),
        OsStr::new(&line),
        // Nothing is being kept here: what `script` writes to its typescript
        // file is a second copy of what this reads off the pipe.
        OsStr::new("/dev/null"),
    ];

    let mut command = Command::from(sandbox.command(&session));

    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // The relay ends the session itself, and a child left behind by a
        // panicking task is one nothing would ever reap.
        .kill_on_drop(true);

    command
}

/// A command as the single string `script --command` takes, every word quoted.
fn shell_command(argv: &[String]) -> String {
    let words: Vec<String> = argv
        .iter()
        .map(|word| format!("'{}'", word.replace('\'', r"'\''")))
        .collect();

    format!("exec {}", words.join(" "))
}

/// Follow a session until it is over, putting what it prints on the Timeline as
/// it arrives.
async fn relay(
    pool: &SqlitePool,
    nudges: &Nudges,
    event_id: i64,
    child: &mut Child,
    mut stopping: oneshot::Receiver<()>,
) {
    let mut output = match child.stdout.take() {
        Some(output) => output,
        None => {
            tracing::error!(event_id, "a session was started without an output to read");
            return;
        }
    };

    // What is left on the error pipe is `script` and bwrap talking about
    // themselves: the session's own errors come back over the pseudo-terminal
    // with the rest of what it printed. Read all the same — a sandbox that
    // refused to start says so here and nowhere else.
    let complaints = child.stderr.take().map(|mut errors| {
        tokio::spawn(async move {
            let mut said = String::new();
            let _ = errors.read_to_string(&mut said).await;
            said
        })
    });

    let mut reading = Reading::default();
    let mut buffer = vec![0u8; CHUNK];
    let mut pending = String::new();
    let mut flushed = Instant::now();
    let mut ending = false;

    loop {
        let deadline = tokio::time::Instant::from_std(flushed + FLUSH_EVERY);

        tokio::select! {
            read = output.read(&mut buffer) => match read {
                // The pseudo-terminal is closed, which is the session gone.
                Ok(0) => break,
                Ok(taken) => pending.push_str(&reading.take(&buffer[..taken])),
                Err(error) => {
                    tracing::error!(error = ?error, event_id, "reading a session's output failed");
                    break;
                }
            },
            _ = tokio::time::sleep_until(deadline), if !pending.is_empty() => {
                flush(pool, nudges, event_id, &mut pending, &reading).await;
                flushed = Instant::now();
            }
            _ = &mut stopping, if !ending => {
                ending = true;
                // The whole sandbox, which is what makes this reach the session
                // inside it: bwrap's child is the first process of a namespace
                // of its own, and a namespace whose first process is gone is a
                // namespace with nothing left in it.
                if let Err(error) = child.start_kill() {
                    tracing::error!(error = ?error, event_id, "a session would not be ended");
                }
            }
        }
    }

    pending.push_str(&reading.finish());
    flush(pool, nudges, event_id, &mut pending, &reading).await;

    match child.wait().await {
        Ok(status) if status.success() || ending => {}
        Ok(status) => tracing::warn!(event_id, %status, "a session ended badly"),
        Err(error) => tracing::error!(error = ?error, event_id, "a session could not be reaped"),
    }

    if let Some(complaints) = complaints
        && let Ok(said) = complaints.await
        && !said.trim().is_empty()
    {
        tracing::debug!(
            event_id,
            complaints = said.trim(),
            "a session's own plumbing"
        );
    }
}

/// Put what has been printed since last time in the store, and tell whoever is
/// watching that it is there.
async fn flush(
    pool: &SqlitePool,
    nudges: &Nudges,
    event_id: i64,
    pending: &mut String,
    reading: &Reading,
) {
    if pending.is_empty() {
        return;
    }

    match store::append_transcript(pool, event_id, pending, &reading.summary()).await {
        // Kept rather than dropped: the next flush carries it, and a store that
        // is briefly unwritable should cost latency rather than a hole in a
        // record nothing can go back and fill.
        Err(error) => {
            tracing::error!(error = ?error, event_id, "keeping a session's output failed")
        }
        Ok(()) => {
            pending.clear();
            nudges.announce();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn profile() -> store::Profile {
        store::Profile {
            id: 1,
            name: "fable".to_owned(),
            claude_dir: PathBuf::from("/srv/accounts/fable/.claude"),
            config_file: PathBuf::from("/srv/accounts/fable/.claude.json"),
            model: "claude-fable-5".to_owned(),
            agent_type: store::AgentType::Claude,
        }
    }

    fn agents(agent: Vec<String>, state: &std::path::Path) -> Agents {
        Agents::running(
            agent,
            Home {
                path: PathBuf::from("/home/verkstead"),
                gh_config: PathBuf::from("/home/verkstead/.config/gh"),
            },
            SandboxConfig::default(),
            Skills::installed(state).expect("this binary carries skills"),
        )
    }

    /// The prompt is what the grilling starts from, and an interactive claude
    /// takes what it is to start on as its last argument.
    #[test]
    fn a_session_runs_the_profiles_model_on_the_prompt() {
        let state = tempfile::tempdir().unwrap();
        let argv =
            agents(vec!["claude".to_owned()], state.path()).argv(&profile(), "# Rate limiting\n");

        assert_eq!(
            argv,
            vec![
                "claude".to_owned(),
                "--model".to_owned(),
                "claude-fable-5".to_owned(),
                "# Rate limiting\n".to_owned(),
            ]
        );
    }

    /// Everything about a Brief that a shell would otherwise read as its own —
    /// and a Brief is markdown a human wrote, so all of it turns up sooner or
    /// later.
    #[test]
    fn nothing_in_a_brief_can_get_out_of_its_quotes() {
        for prompt in [
            "it's a brief",
            "$(rm -rf /)",
            "`whoami`",
            "a \"quoted\" thing; and another",
            "'; echo pwned; '",
        ] {
            let line = shell_command(&["claude".to_owned(), prompt.to_owned()]);

            let printed = std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(line.replace("exec 'claude'", "exec printf '%s'"))
                .output()
                .expect("a shell to read it back with");

            assert_eq!(
                String::from_utf8_lossy(&printed.stdout),
                prompt,
                "the shell read {prompt:?} as something else"
            );
        }
    }
}
