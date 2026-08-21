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

use crate::capture::Reading;
use crate::handoffs::Handoffs;
use crate::nudge::Nudges;
use crate::runner::Pace;
use crate::sandbox::{Home, Reachable, Sandbox, SandboxConfig, under_dev_shell};
use crate::skills::Skills;
use crate::store;

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
/// identity out of, where Verkstead itself is reachable from inside one, the
/// extra binds Sandbox Configuration asks for, the skills every sandbox is
/// given, where a Conversation's handoff directory is made, and what an agent is
/// on the command line.
///
/// Resolved once at startup and shared by every session, because each of them is
/// a fact about the machine rather than about any one Conversation — including
/// the address and the handoff root, which are scoped to a Conversation only as
/// a session is started.
#[derive(Debug, Clone)]
pub struct Agents {
    home: Home,
    reachable: Reachable,
    config: SandboxConfig,
    skills: Skills,
    handoffs: Handoffs,

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

    /// How fast the runner works the backlog these sessions are launched for.
    ///
    /// A field for the reason [`Agents::agent`] is one: what the runner has to
    /// prove is that a session is ended once its step has landed and it has gone
    /// quiet, and the grace period that makes that safe is measured in seconds —
    /// several of them, several times over, is a test that spends most of its
    /// life asleep. The pace a server runs at is [`Pace::default`] and nothing
    /// sets it otherwise.
    pace: Pace,
}

impl Agents {
    /// The real thing: claude, under whichever account the Profile names.
    pub fn new(
        home: Home,
        reachable: Reachable,
        config: SandboxConfig,
        skills: Skills,
        handoffs: Handoffs,
    ) -> Agents {
        Agents::running(
            vec!["claude".to_owned()],
            home,
            reachable,
            config,
            skills,
            handoffs,
        )
    }

    /// The same, with something else where claude goes — see [`Agents::agent`].
    pub fn running(
        agent: Vec<String>,
        home: Home,
        reachable: Reachable,
        config: SandboxConfig,
        skills: Skills,
        handoffs: Handoffs,
    ) -> Agents {
        Agents {
            home,
            reachable,
            config,
            skills,
            handoffs,
            agent,
            pace: Pace::default(),
        }
    }

    /// The same, working the backlog at `pace` — see [`Agents::pace`].
    pub fn at_pace(self, pace: Pace) -> Agents {
        Agents { pace, ..self }
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

    /// Whose turn it is in each Conversation's Worktree — see [`Sessions::turn`].
    turns: Arc<Mutex<HashMap<i64, Arc<tokio::sync::Mutex<()>>>>>,
}

/// The Worktree of one Conversation, held for as long as one thing is using it.
///
/// Dropping it is what hands it on, so it is held across the whole of a session
/// rather than taken to start one: what it is protecting is not the launching but
/// the working.
pub(crate) type Turn = tokio::sync::OwnedMutexGuard<()>;

/// A session that has been started, as whatever is driving it holds one.
///
/// Handed out rather than kept, because the two things a driver needs of a
/// session are things the register cannot answer: how long it has been quiet,
/// and when it is over. Both are about *this* session — a Conversation whose
/// session ended and was replaced would answer both questions about the wrong
/// one.
pub(crate) struct Session {
    /// The Timeline Event it is printing into.
    pub(crate) event_id: i64,

    /// How long it has been printing nothing.
    pub(crate) quiet: Quiet,

    /// Word that it is over, and how it ended.
    ///
    /// Sent as the last thing a relay does, and read through the *closing* of
    /// this rather than through the message arriving: a relay that panicked
    /// would drop its end without sending, and a driver waiting on a message
    /// would then wait on a session that had already gone. So the channel
    /// closing is what says *over* — and the value it carries, where there is
    /// one, is what says how.
    ended: oneshot::Receiver<Ended>,
}

/// How a session ended, as whoever was driving it hears.
///
/// The distinction the Interruptions turn on: [`Ended::Stopped`] is a session
/// Verkstead put an end to because its step had landed, and every other variant
/// is a session that stopped without being asked to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ended {
    /// Ran to the end and exited cleanly. Which is not the same as *did the
    /// work*: an interactive agent that decides there is nothing to do exits
    /// zero, and what says the step landed is the Worktree.
    Well,

    /// Exited badly, said as what happened to it — "exited with status 1".
    Badly(String),

    /// Verkstead ended it: its step had landed and it had gone quiet, or the
    /// human aborted the Conversation out from under it.
    Stopped,

    /// It is over and nothing can say how — the relay itself failed, or could not
    /// reap the process. Read as badly, because a session nothing can account for
    /// is not one to carry on from.
    Unknown,
}

impl Ended {
    /// The sentence an Interruption's evidence records, or `None` where the
    /// session ended the way it was meant to.
    ///
    /// One place decides what each way of ending is *called*, so the words the
    /// human reads on the Timeline and the words the log used are the same
    /// words.
    pub(crate) fn badly(&self) -> Option<String> {
        match self {
            Self::Stopped | Self::Well => None,
            Self::Badly(status) => Some(format!("the session {status}")),
            Self::Unknown => Some("the session ended and nothing could say how".to_owned()),
        }
    }

    /// Whether Verkstead is what ended it.
    ///
    /// The one way of ending that is never an Interruption, whatever the Worktree
    /// then says. Everything else a driver sees is a session that stopped without
    /// being asked to, and the human is owed the choice about it; this is the
    /// human having already made one — they aborted the Conversation — or the
    /// step having landed. Raising an Interruption about it would be asking them
    /// what to do about the thing they just did.
    pub(crate) fn on_purpose(&self) -> bool {
        matches!(self, Self::Stopped)
    }
}

impl Session {
    /// Wait until the session is over, and say how it ended.
    pub(crate) async fn ended(&mut self) -> Ended {
        // A relay that dropped its end without sending is a relay that panicked,
        // and a session nothing can account for is exactly [`Ended::Unknown`].
        (&mut self.ended).await.unwrap_or(Ended::Unknown)
    }
}

/// How long a session has been printing nothing.
///
/// Shared with the relay, which puts it back to now on everything that arrives.
/// That is what makes a grace period safe to end a session on: a session still
/// talking is never one to end, however long it goes on for, and the work a
/// session does after its commit — a message, a summary, a push — runs to
/// completion rather than being cut off mid-sentence.
#[derive(Debug, Clone)]
pub(crate) struct Quiet(Arc<Mutex<Instant>>);

impl Quiet {
    fn started() -> Quiet {
        Quiet(Arc::new(Mutex::new(Instant::now())))
    }

    /// The session said something, so it has been quiet for no time at all.
    fn spoke(&self) {
        *self
            .0
            .lock()
            .expect("a session's quiet clock is not poisoned") = Instant::now();
    }

    pub(crate) fn for_how_long(&self) -> Duration {
        self.0
            .lock()
            .expect("a session's quiet clock is not poisoned")
            .elapsed()
    }
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
            turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// One that cannot: nothing is launched, and everything else about starting
    /// a grilling holds.
    pub(crate) fn none() -> Sessions {
        Sessions {
            agents: None,
            running: Arc::new(Mutex::new(HashMap::new())),
            turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Wait for this Conversation's Worktree, and take it.
    ///
    /// One agent in one Worktree, kept true by whoever is about to put one there
    /// asking first. Nothing in [`Sessions::start`] enforces it — starting a
    /// second session for a Conversation *ends the first*, which is exactly what
    /// a run relaunching a step wants and exactly what a wrap-up must never do:
    /// a red check arriving mid-review would otherwise kill the review, and the
    /// human answering the review would kill the fix session halfway through a
    /// commit.
    ///
    /// So the wrap-up's three dispatchers take turns. This is the one that waits
    /// — for the review, which nothing will start again on its behalf, and for
    /// the findings the human has just accepted, which would otherwise be work
    /// quietly dropped. The checks watcher uses [`Sessions::try_turn`] instead.
    ///
    /// The map keeps a lock per Conversation for the life of the server, which is
    /// a pointer per Conversation ever wrapped up.
    pub(crate) async fn turn(&self, conversation_id: i64) -> Turn {
        self.turn_of(conversation_id).lock_owned().await
    }

    /// Take it if it is free, and hand back nothing if somebody else has it.
    ///
    /// What a poller wants. The checks watcher looks again in half a minute
    /// anyway, and a fix session queued behind a review that takes ten minutes
    /// would be dispatched about checks nobody has looked at since.
    pub(crate) fn try_turn(&self, conversation_id: i64) -> Option<Turn> {
        self.turn_of(conversation_id).try_lock_owned().ok()
    }

    /// The lock for one Conversation, made the first time anyone asks for it.
    fn turn_of(&self, conversation_id: i64) -> Arc<tokio::sync::Mutex<()>> {
        self.turns
            .lock()
            .expect("the turns registry is not poisoned")
            .entry(conversation_id)
            .or_default()
            .clone()
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

    /// The pace the runner works a backlog at — see [`Agents::pace`].
    ///
    /// [`Pace::default`] where this server runs no sessions at all, which is a
    /// pace nothing will ever keep: a server with no agents has no session to
    /// end.
    pub(crate) fn pace(&self) -> Pace {
        self.agents
            .as_ref()
            .map(|agents| agents.pace)
            .unwrap_or_default()
    }

    /// Run `profile`'s agent on `prompt`, inside `conversation`'s sandbox, and
    /// put what it prints on the Timeline as it arrives.
    ///
    /// The session that was started, for whoever is driving it — see
    /// [`Session`]. A server with no way to run agents starts none, and a
    /// sandbox that cannot be built — a Conversation with no worktree, or one
    /// git will not own — is the same answer: there is nothing here to launch.
    /// Both are logged, because both mean a Conversation that is grilling with
    /// nothing grilling it.
    ///
    /// The Timeline Event is made after the process is, so that a session that
    /// never started leaves no Capture of nothing.
    pub(crate) async fn start(
        &self,
        pool: &SqlitePool,
        nudges: &Nudges,
        conversation: &store::Conversation,
        profile: &store::Profile,
        prompt: &str,
    ) -> Result<Option<Session>> {
        let Some(agents) = self.agents.clone() else {
            tracing::warn!(
                conversation_id = conversation.id,
                "this server has no way to run an agent, so no session was started"
            );
            return Ok(None);
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
            let reachable = agents.reachable.clone();
            let skills = agents.skills.clone();
            let handoffs = agents.handoffs.clone();
            let extra = agents.config.binds_for(&conversation.repo.name);

            move || {
                let sandbox = Sandbox::for_conversation(
                    &conversation,
                    &profile,
                    home,
                    &reachable,
                    &skills,
                    &handoffs,
                    extra,
                )?;
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
            return Ok(None);
        };

        let mut child = match captured(&sandbox, &argv).spawn() {
            Ok(child) => child,
            Err(error) => {
                tracing::error!(
                    error = ?error,
                    conversation_id,
                    "a grilling session could not be started"
                );
                return Ok(None);
            }
        };

        let event_id = store::start_capture(pool, conversation_id).await?;

        let (stop, stopping) = oneshot::channel();

        // The two halves of what a driver holds a session by: the clock the
        // relay keeps as it reads, and the word that the relay has finished.
        let quiet = Quiet::started();
        let (over, ended) = oneshot::channel();

        // What the session is about to commit, which is the other half of what
        // it leaves behind. Watched for as long as it runs and once more as it
        // ends — see [`crate::commits`] — and its base commit is where the
        // branch started, which is recorded by the time any session exists.
        let branch = conversation.branch.clone();
        let repo = conversation.repo.path.clone();
        let base = conversation.base_commit.clone();

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
                let quiet = quiet.clone();

                async move {
                    // A Conversation with no base commit has never started
                    // grilling, and a session is running in it — so this is a
                    // record that has been got at rather than a Conversation
                    // whose branch is not worth watching.
                    let watching = match base {
                        Some(base) => {
                            let (stop, stopping) = oneshot::channel();

                            let watcher = tokio::spawn(crate::commits::watch(
                                pool.clone(),
                                nudges.clone(),
                                conversation_id,
                                repo,
                                branch,
                                base,
                                stopping,
                            ));

                            Some((stop, watcher))
                        }
                        None => {
                            tracing::error!(
                                conversation_id,
                                "a session is running on a Conversation with no base commit, \
                                 so its commits cannot be told from what it branched off"
                            );
                            None
                        }
                    };

                    let ended = relay(&pool, &nudges, event_id, &mut child, &quiet, stopping).await;

                    // The session is over, so the branch is finished moving.
                    // Waited on rather than only asked to stop, because what it
                    // does when told is one last sweep: a session's final act is
                    // usually a commit, and it lands a poll after the process
                    // that made it has gone.
                    if let Some((stop, watcher)) = watching {
                        let _ = stop.send(());

                        if let Err(error) = watcher.await {
                            tracing::error!(error = ?error, conversation_id, "a branch watcher ended badly");
                        }
                    }

                    // Off the register before the last Nudge, so that a page
                    // reading the Conversation back reads a session that has
                    // ended.
                    sessions.forget(conversation_id, event_id);
                    nudges.announce();

                    // Last of all, for the reason the drop it replaced was last:
                    // whoever is driving acts on this — it raises Interruptions
                    // and launches the next step — and everything above has to
                    // have happened by then. Chief among them the final sweep of
                    // the branch, because a session's last act is usually a
                    // commit and a driver told *over* before it landed would be
                    // reading a Conversation that had not finished happening.
                    //
                    // A send that fails is a driver that has gone, which is every
                    // session nothing is following.
                    let _ = over.send(ended);
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

        Ok(Some(Session {
            event_id,
            quiet,
            ended,
        }))
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
///
/// `quiet` is put back to now on everything read rather than on everything
/// written down: what it is measuring is whether the session is still talking,
/// and a redraw the summariser throws away is a session talking.
///
/// What comes back is how it ended, which is what whoever is driving decides
/// between carrying on and raising an Interruption by.
async fn relay(
    pool: &SqlitePool,
    nudges: &Nudges,
    event_id: i64,
    child: &mut Child,
    quiet: &Quiet,
    mut stopping: oneshot::Receiver<()>,
) -> Ended {
    let mut output = match child.stdout.take() {
        Some(output) => output,
        None => {
            tracing::error!(event_id, "a session was started without an output to read");
            return Ended::Unknown;
        }
    };

    // What is left on the error pipe is `script` and bwrap talking about
    // themselves: the session's own errors come back over the pseudo-terminal
    // with the rest of what it printed. Read, and put on the Capture once the
    // session is over — a sandbox that refused to start says so here and nowhere
    // else, and a Timeline reading `0 lines` with the reason in a log nobody
    // turned up is the whole of what makes that failure hard to see.
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
                Ok(taken) => {
                    quiet.spoke();
                    pending.push_str(&reading.take(&buffer[..taken]));
                }
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

    // `ending` first, because a session Verkstead killed exits by a signal and
    // that is not a session that went wrong: it is the step having landed.
    let ended = match child.wait().await {
        Ok(_) if ending => Ended::Stopped,
        Ok(status) if status.success() => Ended::Well,
        Ok(status) => {
            tracing::warn!(event_id, %status, "a session ended badly");

            // Worded here rather than by whoever reads it, because this is where
            // the two ways of ending badly are still told apart: a status is a
            // number the agent chose, and no status at all is a process something
            // else killed.
            Ended::Badly(match status.code() {
                Some(code) => format!("exited with status {code}"),
                None => format!("was killed — {status}"),
            })
        }
        Err(error) => {
            tracing::error!(error = ?error, event_id, "a session could not be reaped");
            Ended::Unknown
        }
    };

    if let Some(complaints) = complaints
        && let Ok(said) = complaints.await
        && !said.trim().is_empty()
    {
        // A warning rather than a note: this pipe is silent on a session that
        // ran, so anything on it is something that went wrong on the way in.
        tracing::warn!(
            event_id,
            complaints = said.trim(),
            "a session's own plumbing"
        );

        // Last, after everything the session printed, because that is what it
        // is: the account of how the session came to stop saying anything.
        let mut note = reading.take(plumbing(&said).as_bytes());
        flush(pool, nudges, event_id, &mut note, &reading).await;
    }

    ended
}

/// The plumbing's complaint as it goes on the Capture.
///
/// Marked, and it is the one thing in a Capture that is not the session's own
/// word. What a terminal was sent is the record — this arrived on the pipe
/// beside it, from `script` and bwrap rather than from the agent, and a reader
/// who could not tell the two apart would be reading the sandbox's failure as
/// something the agent said about itself.
///
/// On its own line either way, because what it follows is whatever the session
/// happened to leave the cursor in the middle of.
fn plumbing(said: &str) -> String {
    format!("\n[the sandbox, not the agent]\n{}\n", said.trim())
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

    match store::append_capture(pool, event_id, pending, &reading.summary()).await {
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
            Reachable::at("127.0.0.1:8422".parse().unwrap()),
            SandboxConfig::default(),
            Skills::installed(state).expect("this binary carries skills"),
            Handoffs::under(state),
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
