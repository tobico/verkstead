//! The sessions a Conversation's work is actually done by: an agent running
//! inside its sandbox, and everything it prints on its way to the Timeline.
//!
//! The technique is roadrunner's `session.ts`, and only the technique — that is
//! TypeScript on Bun and this is Rust, so nothing is carried over but the shape
//! and the reasons for it. The session runs on a pseudo-terminal of its own,
//! because claude needs a terminal to behave like itself. Verkstead opens the
//! pair and hands the sandbox the far end as its stdin, stdout and stderr — see
//! [`crate::terminal`] — and relays what arrives at this one, kept whole and
//! summarised.
//!
//! One terminal and nothing beside it. The sandbox's own complaints come back
//! among what the session printed rather than on a pipe of their own, in the
//! order they happened, which is what a real terminal does: a sandbox that
//! refuses to start says so in the Capture of the session that failed.
//!
//! The session is interactive and never `-p`: it idles when it has nothing to
//! do, which is what a blocking ask depends on (ADR 0001 in tobico-skills).
//!
//! Whether a session is running is held here and nowhere else. A running session
//! is a process, and no table can hold one — a restarted server has no sessions
//! at all, and that is exactly what a Conversation should then say rather than
//! reading back a live one out of a database.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use sqlx::SqlitePool;
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;
use verkstead_schema::Nudge;

use crate::capture::{Reading, Told};
use crate::handoffs::Handoffs;
use crate::nudge::Nudges;
use crate::runner::Pace;
use crate::sandbox::{Executable, Home, Reachable, Sandbox, SandboxConfig, under_dev_shell};
use crate::screen::Live;
use crate::settings::Settings;
use crate::skills::Skills;
use crate::store;
use crate::terminal::Terminal;
use crate::transcript::Tail;

/// How much of a session's output to take off the pseudo-terminal at once.
const CHUNK: usize = 8 * 1024;

/// How often what a session has printed reaches the store and the open pages.
///
/// A terminal application redraws many times a second, and a store written and
/// a page nudged on every one of those would be a cost that scaled with how
/// busy the agent's spinner was. Half a second is under what a human reads as a
/// delay and two orders of magnitude off what a redraw costs.
const FLUSH_EVERY: Duration = Duration::from_millis(500);

/// How long a session has to print nothing before it is called idle.
///
/// Short, because what it measures is a terminal rather than an agent: claude
/// repaints its spinner many times a second while it is working, so a session
/// that has printed nothing for this long is one that has stopped — sitting on
/// a Blocking Ask, or waiting for the human. Three seconds is clear
/// of the longest gap a working session leaves, and short enough that the mark
/// says so while it still matters.
pub(crate) const IDLE_AFTER: Duration = Duration::from_secs(3);

/// How a Conversation's agents are run: the home a sandbox reads the machine's
/// identity out of, where Verkstead itself is reachable from inside one, the
/// extra binds Sandbox Configuration asks for, the skills every sandbox is
/// given, the executable every sandbox asks with, where a Conversation's handoff
/// directory is made, where the settings files are read from, and what an agent
/// is on the command line.
///
/// Resolved once at startup and shared by every session, because each of them is
/// a fact about the machine rather than about any one Conversation — including
/// the address and the handoff root, which are scoped to a Conversation only as
/// a session is started.
///
/// The settings are a *where* rather than a *what*, and deliberately: the files
/// are read as each session is spawned, so a token the human rotates through the
/// settings page reaches the next session without the server being restarted.
#[derive(Debug, Clone)]
pub struct Agents {
    home: Home,
    reachable: Reachable,
    config: SandboxConfig,
    skills: Skills,

    /// The executable every sandbox is given as `verkstead`: this server's own
    /// image — see [`Executable`].
    ///
    /// `None` where the server cannot find it, which is not a fallback to the
    /// machine's install but a session that does not start. Resolved at startup
    /// like everything else here, and reported per session rather than at
    /// startup, because what it costs is a session and the log line worth having
    /// is the one that says which — see [`Sessions::start`].
    verkstead: Option<Executable>,

    handoffs: Handoffs,
    settings: Settings,

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
        verkstead: Option<Executable>,
        handoffs: Handoffs,
        settings: Settings,
    ) -> Agents {
        Agents::running(
            vec!["claude".to_owned()],
            home,
            reachable,
            config,
            skills,
            verkstead,
            handoffs,
            settings,
        )
    }

    /// The same, with something else where claude goes — see [`Agents::agent`].
    #[allow(clippy::too_many_arguments)]
    pub fn running(
        agent: Vec<String>,
        home: Home,
        reachable: Reachable,
        config: SandboxConfig,
        skills: Skills,
        verkstead: Option<Executable>,
        handoffs: Handoffs,
        settings: Settings,
    ) -> Agents {
        Agents {
            home,
            reachable,
            config,
            skills,
            verkstead,
            handoffs,
            settings,
            agent,
            pace: Pace::default(),
        }
    }

    /// The same, working the backlog at `pace` — see [`Agents::pace`].
    pub fn at_pace(self, pace: Pace) -> Agents {
        Agents { pace, ..self }
    }

    /// What a session under `pairing` on `prompt`, named `session`, runs.
    ///
    /// The model is the Pairing's, said on the command line rather than left to
    /// whatever the account's own settings hold: which model a session runs is
    /// the half of the choice the Profile does not make. A Conversation that
    /// chose its Profile before there was a model to choose beside it runs on
    /// the one that Profile carried — see [`store::Pairing::runs_on`]. The
    /// prompt follows it as the one positional argument, which is where an
    /// interactive claude takes the thing it is to start on.
    ///
    /// A Profile listing no models is refused when it is saved, so the flag is
    /// only ever left off for a row somebody edited by hand — and left off
    /// rather than passed empty, for the reason the name below is.
    ///
    /// The name comes after the prompt rather than before it, and claude reads
    /// its options on either side of the positional one. What is on the other
    /// side of that choice is everything that already reads this line: an agent
    /// is run as its model and then its Brief, and a flag pushed in between the
    /// two would move the Brief under every stub agent the test suite stands
    /// where claude goes. Options added here go on the end, so nothing that was
    /// already there moves.
    ///
    /// `None` is a session Verkstead could not name — see [`session_name`] —
    /// and the flag is then left off entirely rather than passed empty: an agent
    /// told to run under no name at all would refuse to start, where one not
    /// told anything picks its own.
    fn argv(&self, pairing: &store::Pairing, prompt: &str, session: Option<&str>) -> Vec<String> {
        let mut argv = self.agent.clone();

        if let Some(model) = pairing.runs_on() {
            argv.push("--model".to_owned());
            argv.push(model.to_owned());
        }

        argv.push(prompt.to_owned());

        if let Some(session) = session {
            argv.push("--session-id".to_owned());
            argv.push(session.to_owned());
        }

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
///
/// Handed out more than once, at that: a grilling session is started by the
/// button and driven later by whoever the human's pick arms — see
/// [`Sessions::following`] — so what a driver holds has to be a second view of
/// one session rather than the only one.
pub(crate) struct Session {
    /// The Timeline Event it is printing into.
    pub(crate) event_id: i64,

    /// How long it has been printing nothing.
    pub(crate) quiet: Quiet,

    /// Word that it is over, and how it ended.
    ///
    /// Sent as the last thing a relay does, and read through the *closing* of
    /// this as well as through the value arriving: a relay that panicked would
    /// drop its end without sending, and a driver waiting on a value alone would
    /// then wait on a session that had already gone. So the channel closing says
    /// *over* too — and the value, where there is one, is what says how.
    ended: watch::Receiver<Option<Ended>>,
}

/// How a session ended, as whoever was driving it hears.
///
/// The distinction a stop turns on: [`Ended::Stopped`] is a session Verkstead
/// put an end to because its step had landed, and every other variant is a
/// session that stopped without being asked to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ended {
    /// Ran to the end and exited cleanly. Which is not the same as *did the
    /// work*: an interactive agent that decides there is nothing to do exits
    /// zero, and what says the step landed is the Worktree.
    Well,

    /// Exited badly, said as what happened to it — "exited with status 1".
    Badly(String),

    /// Verkstead ended it, however it came to: its step had landed and it had
    /// gone quiet, the human aborted the Conversation or pressed Force stop out
    /// from under it, or the account it was spending ran out of window and the
    /// stop written for that ended it — see [`crate::limits`].
    Stopped,

    /// It is over and nothing can say how — the relay itself failed, or could not
    /// reap the process. Read as badly, because a session nothing can account for
    /// is not one to carry on from.
    Unknown,
}

impl Ended {
    /// The sentence a stop's Notice records, or `None` where the session ended
    /// the way it was meant to.
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
    /// The one way of ending that never stops the run, whatever the Worktree
    /// then says. Everything else a driver sees is a session that stopped
    /// without being asked to, and the human is owed the telling about it; this
    /// is a session Verkstead meant to end, and stopping over one would either
    /// tell them driving had stopped about the thing they just stopped, or
    /// write a second stop behind one already on the record.
    ///
    /// Three ways in, and the driver treats them alike. The step landed and the
    /// session went quiet, so there is nothing to tell. The human aborted the
    /// Conversation or pressed Force stop, and the stop their press wrote is
    /// already there. Or the account ran out of window, and the stop
    /// [`crate::limits`] wrote before killing the sandbox is already there too —
    /// which is what this has to be read as for, because a driver reading it as
    /// the human's act alone would advance a run that has stopped.
    pub(crate) fn on_purpose(&self) -> bool {
        matches!(self, Self::Stopped)
    }
}

impl Session {
    /// Wait until the session is over, and say how it ended.
    pub(crate) async fn ended(&mut self) -> Ended {
        // A relay that dropped its end without sending is a relay that panicked,
        // and a session nothing can account for is exactly [`Ended::Unknown`].
        match self.ended.wait_for(Option::is_some).await {
            Ok(ended) => ended.clone().unwrap_or(Ended::Unknown),
            Err(_) => Ended::Unknown,
        }
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

    /// When it last said anything, for whoever wants to sleep until it has been
    /// quiet long enough rather than to ask how long it has been.
    fn since(&self) -> Instant {
        *self
            .0
            .lock()
            .expect("a session's quiet clock is not poisoned")
    }
}

/// A session that has been started, as its relay holds it.
///
/// The three travel together because none is any use without the others: what
/// the session says comes off the terminal, what ends the session is done to the
/// process, and the Screen is the terminal read as a grid. The terminal has to
/// outlive the process, because the last thing a session says is said on its way
/// out.
struct Launched {
    /// The terminal it was started on, which is where everything it prints
    /// arrives.
    ///
    /// Shared, because the relay is no longer the only thing that touches it: a
    /// watcher resizing its window resizes this, and what makes the session
    /// redraw to fit is the resize reaching the terminal it is on — see
    /// [`Live`].
    terminal: Arc<Terminal>,

    /// The sandbox around it, which is what killing it kills.
    child: Child,

    /// What it is drawing, fed the same text the Capture is written from — see
    /// [`Live`].
    screen: Live,
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

    /// What it is drawing, for anybody who wants to watch — see [`Live`].
    ///
    /// Held beside the session rather than under a register of its own, because
    /// it is the same fact: a Screen that is live is a session that is running,
    /// and the two would only ever be looked up together.
    screen: Live,

    /// The two halves a driver is handed, kept so that a session already running
    /// can be given one — see [`Sessions::following`].
    quiet: Quiet,
    ended: watch::Receiver<Option<Ended>>,
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

    /// Whether a Conversation's running session has stopped printing.
    ///
    /// `false` for a Conversation with nothing running, which is the answer
    /// that reads right wherever it is asked: idle is a thing a *running*
    /// session is, and a session that has ended is neither.
    ///
    /// Read at the moment a Conversation is drawn rather than stored, as
    /// [`Sessions::writing`] is and for the same reason — how long a process has
    /// been quiet is a fact about a process. The crossing is announced as it
    /// happens too, because a session going quiet is exactly when it stops
    /// producing the Nudges an open page re-reads on; see [`relay`].
    pub(crate) fn idling(&self, conversation_id: i64) -> bool {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .get(&conversation_id)
            .is_some_and(|running| running.quiet.for_how_long() >= IDLE_AFTER)
    }

    /// What a Conversation's running session is drawing, or `None` where the
    /// Event named is not one this Conversation has a session still running for.
    ///
    /// By the Event as well as by the Conversation, for the reason
    /// [`Sessions::forget`] is: a Timeline holds every session a Conversation
    /// has ever had, and a watcher who asked for one that has ended should be
    /// told so rather than handed the screen of whatever is running now.
    pub(crate) fn screen(&self, conversation_id: i64, event_id: i64) -> Option<Live> {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .get(&conversation_id)
            .filter(|running| running.event_id == event_id)
            .map(|running| running.screen.clone())
    }

    /// A driver's hold on the session a Conversation already has running, or
    /// `None` where it has none.
    ///
    /// What the grilling session is picked up by. It was started by the button
    /// and nothing was driving it, because until the human picks a Direction
    /// there is nothing to watch it for; the pick is what arms a driver, and by
    /// then the only place that session exists is this register.
    ///
    /// A second hold on one session rather than a transfer: the two halves it is
    /// made of — the relay's own quiet clock, and word that the relay has
    /// finished — are both shared, so a driver handed one here waits on exactly
    /// what a driver handed one at the start waits on.
    pub(crate) fn following(&self, conversation_id: i64) -> Option<Session> {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .get(&conversation_id)
            .map(|running| Session {
                event_id: running.event_id,
                quiet: running.quiet.clone(),
                ended: running.ended.clone(),
            })
    }

    /// Which Conversations have a session running right now.
    ///
    /// The whole set at once rather than a question per Conversation, because
    /// the one thing that asks is the sidebar and the sidebar is a list: taking
    /// the lock once and reading it once is what keeps drawing a list of
    /// Conversations from being a list of lock acquisitions.
    pub(crate) fn working(&self) -> HashSet<i64> {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .keys()
            .copied()
            .collect()
    }

    /// And which of those have stopped printing — [`Sessions::idling`] for the
    /// whole sidebar at once, and one lock rather than one per row for the same
    /// reason [`Sessions::working`] is.
    ///
    /// A subset of [`Sessions::working`] by construction, because both are the
    /// same register read: idle is a thing a running session is, and a
    /// Conversation with nothing in it is in neither set.
    pub(crate) fn quiet(&self) -> HashSet<i64> {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .iter()
            .filter(|(_, running)| running.quiet.for_how_long() >= IDLE_AFTER)
            .map(|(conversation_id, _)| *conversation_id)
            .collect()
    }

    /// Whether this server can launch an agent at all.
    ///
    /// The served router always can — see [`crate::router_with_ui`] — so this
    /// is only ever false for a router built without [`Agents`], which is every
    /// test about something other than sessions. What such a server cannot do
    /// is take a Conversation up: nothing is driving one, nothing ever will,
    /// and there is no session for Resume to start.
    pub(crate) fn runs_sessions(&self) -> bool {
        self.agents.is_some()
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

    /// Run `pairing`'s agent on `prompt`, inside `conversation`'s sandbox, and
    /// put what it prints on the Timeline as it arrives.
    ///
    /// The session that was started, for whoever is driving it — see
    /// [`Session`]. A server with no way to run agents starts none, a server
    /// that cannot find its own executable to equip one with starts none either,
    /// and a sandbox that cannot be built — a Conversation with no worktree, or
    /// one git will not own — is the same answer: there is nothing here to
    /// launch. All three are logged, because each of them means a Conversation
    /// that is grilling with nothing grilling it.
    ///
    /// The Timeline Event is made after the process is, so that a session that
    /// never started leaves no Capture of nothing.
    pub(crate) async fn start(
        &self,
        pool: &SqlitePool,
        nudges: &Nudges,
        conversation: &store::Conversation,
        pairing: &store::Pairing,
        prompt: &str,
    ) -> Result<Option<Session>> {
        let Some(agents) = self.agents.clone() else {
            tracing::warn!(
                conversation_id = conversation.id,
                "this server has no way to run an agent, so no session was started"
            );
            return Ok(None);
        };

        // What the session would ask with, which is the server's own image. Said
        // here rather than let fall through to a machine's install: the two are
        // separate builds, and a session asking one binary's Question Sets of
        // another binary's server is the failure this refuses — see
        // [`Executable`]. Which session it cost is the whole of what is worth
        // logging, and this is where that is known.
        let Some(verkstead) = agents.verkstead.clone() else {
            tracing::error!(
                conversation_id = conversation.id,
                "Verkstead cannot find its own executable, so this session could not be \
                 equipped with `verkstead` and was not started"
            );
            return Ok(None);
        };

        // Decided here rather than read back off the agent afterwards, which is
        // the whole of why the log it writes can be found at all — see
        // [`session_name`].
        let session = session_name();
        let argv = agents.argv(pairing, prompt, session.as_deref());
        let conversation_id = conversation.id;

        // The sandbox asks git where the worktree's object database is, and the
        // dev-shell question is a `nix eval` or two. Both block, and both are
        // decided before anything is spawned.
        let built = tokio::task::spawn_blocking({
            let conversation = conversation.clone();
            let profile = pairing.profile.clone();
            let home = agents.home.clone();
            let reachable = agents.reachable.clone();
            let skills = agents.skills.clone();
            let handoffs = agents.handoffs.clone();
            let settings = agents.settings.clone();
            let extra = agents.config.binds_for(&conversation.repo.name);

            move || {
                // Read here rather than held from startup: this is the moment a
                // session's credentials and identity are decided, and it is
                // already on a blocking thread because git is asked about the
                // worktree below.
                let secrets = settings.secrets();
                let config = settings.config();

                let sandbox = Sandbox::for_conversation(
                    &conversation,
                    &profile,
                    home,
                    &reachable,
                    &skills,
                    &verkstead,
                    &handoffs,
                    &secrets,
                    &config,
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

        // The terminal before the session, because the session is started *on*
        // it: a machine that will not give Verkstead a pseudo-terminal is one
        // there is nothing to launch on, the same as a sandbox that cannot be
        // built.
        let mut terminal = match Terminal::open() {
            Ok(terminal) => terminal,
            Err(error) => {
                tracing::error!(
                    error = ?error,
                    conversation_id,
                    "a session's terminal could not be opened, so none was started"
                );
                return Ok(None);
            }
        };

        let child = match terminal.spawn(&mut captured(&sandbox, &argv)) {
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

        // Shared from here on: the relay reads it, and — once somebody is
        // watching — a resize from the browser reaches it.
        let terminal = Arc::new(terminal);

        // And the Screen it is drawing, empty and the size the terminal was
        // opened at. Made before the session is registered, so that a watcher
        // that arrives with the first line of output has a Screen to attach to.
        let screen = Live::on(terminal.clone());

        let mut launched = Launched {
            terminal,
            child,
            screen: screen.clone(),
        };

        let event_id = store::start_capture(pool, conversation_id, session.as_deref()).await?;

        // The log the agent keeps of itself is followed under the name Verkstead
        // gave the session, inside the directory of the Profile it is running
        // under. A session with no name has no log to look for — see
        // [`crate::transcript`].
        let tail = session
            .as_deref()
            .map(|session| Tail::of(conversation_id, &pairing.profile, session));

        // And the same output watched for the one thing a session says that is
        // about the account rather than about the work: that its window is
        // spent. The Profile is taken now because that is what the stop names,
        // and a Profile renamed while a session runs was not the account this
        // one is on — see [`crate::limits`].
        let limits =
            crate::limits::Watch::on(conversation_id, event_id, pairing.profile.name.clone());

        let (stop, stopping) = oneshot::channel();

        // The two halves of what a driver holds a session by: the clock the
        // relay keeps as it reads, and the word that the relay has finished. A
        // watch rather than a oneshot, because one session may be handed to more
        // than one driver over its life — see [`Sessions::following`].
        let quiet = Quiet::started();
        let (over, ended) = watch::channel(None);

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

                    let ended = relay(
                        &pool,
                        &nudges,
                        Printing {
                            conversation_id,
                            event_id,
                        },
                        &mut launched,
                        &quiet,
                        tail,
                        limits,
                        stopping,
                    )
                    .await;

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
                    nudges.announce(Nudge::Conversation {
                        conversation: conversation_id,
                    });

                    // Last of all, for the reason the drop it replaced was last:
                    // whoever is driving acts on this — it stops the run or
                    // launches the next step — and everything above has to
                    // have happened by then. Chief among them the final sweep of
                    // the branch, because a session's last act is usually a
                    // commit and a driver told *over* before it landed would be
                    // reading a Conversation that had not finished happening.
                    //
                    // A send that fails is a driver that has gone, which is every
                    // session nothing is following.
                    let _ = over.send(Some(ended));
                }
            });

            running.insert(
                conversation_id,
                Running {
                    event_id,
                    stop,
                    relay,
                    screen,
                    quiet: quiet.clone(),
                    ended: ended.clone(),
                },
            );
        }

        // The Event is on the Timeline as of now, empty. Whoever is watching
        // should see the session appear rather than see it once it says
        // something.
        nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        });

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

/// A name for a session about to be started: a version 4 UUID, which is what
/// claude will take as a session id and nothing else is.
///
/// Verkstead picks it rather than reading back whatever the agent chose, because
/// what the name is *for* is finding the log the agent keeps of its own
/// conversation — see [`store::session_id`]. The alternative is working out the
/// name the backend would have picked, which is somebody else's private
/// algorithm and free to change under us; a name we chose ourselves cannot.
///
/// Sixteen bytes from the operating system's own generator and nothing around
/// it, for the reason a prefilled branch name is picked the same way: there is
/// no distribution to sample and no sequence to reproduce here either.
///
/// `None` where the generator would not answer, which is a machine in a state
/// nothing here can improve. The session then runs unnamed and picks its own id,
/// its Timeline Event records no name, and what it said is read back off the
/// Capture — the same as for every agent that keeps no log at all.
fn session_name() -> Option<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).ok()?;

    // Version 4 in the high nibble of the seventh byte and the variant in the
    // top bits of the ninth, which is the difference between sixteen random
    // bytes and a UUID something else will accept as one.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let mut name = String::with_capacity(36);

    for (at, byte) in bytes.iter().enumerate() {
        if matches!(at, 4 | 6 | 8 | 10) {
            name.push('-');
        }

        name.push_str(&format!("{byte:02x}"));
    }

    Some(name)
}

/// `argv` as a session: run inside `sandbox`, with nothing between the two.
///
/// One argument vector all the way down, and the three streams are left to
/// [`Terminal::spawn`] — which is the whole of what says a session runs on a
/// terminal.
fn captured(sandbox: &Sandbox, argv: &[String]) -> Command {
    let mut command = Command::from(sandbox.command(argv));

    // The relay ends the session itself, and a child left behind by a panicking
    // task is one nothing would ever reap.
    command.kill_on_drop(true);

    command
}

/// Where a running session's output goes: the Timeline Event it is written into,
/// and the Conversation that Event is on.
///
/// The two travel together everywhere below, because both are needed at every
/// write: the store is written by the Event, and the Nudge saying so is scoped
/// by the Conversation (ADR-0009).
#[derive(Debug, Clone, Copy)]
struct Printing {
    conversation_id: i64,
    event_id: i64,
}

/// Follow a session until it is over, putting what it prints on the Timeline as
/// it arrives — and, where it keeps a log of its own conversation, following
/// that too.
///
/// The two are followed by the one loop because they move together and on the
/// same cadence: the loop is awake every [`FLUSH_EVERY`] to write down what the
/// session printed, and the log it is writing beside that has grown by the same
/// amount of the session's talking. `tail` is `None` where there is no log to
/// look for, which is every session Verkstead could not name.
///
/// `quiet` is put back to now on everything read rather than on everything
/// written down: what it is measuring is whether the session is still talking,
/// and a redraw the summariser throws away is a session talking.
///
/// The session's Screen is fed the same text, and immediately rather than every
/// [`FLUSH_EVERY`]. The store is a record and half a second is nothing to one;
/// the Screen is a terminal somebody may be watching, and half a second is a
/// long time to watch a terminal not move.
///
/// And `limits` is fed the same text a third time, watching for the one thing a
/// session says that is about the account rather than about the work — see
/// [`crate::limits`].
///
/// The one thing this loop announces that is not something it wrote down is the
/// session falling quiet, and then waking: a page draws a session that has
/// stopped differently from one getting on with it, and going quiet is
/// precisely when a session stops producing the Nudges that would carry the
/// news. So both crossings are announced on the Conversation's own kind, once
/// each — into idle [`IDLE_AFTER`] after the last thing read, and out of it on
/// the first thing read after that. The waking one is for the sidebar alone:
/// what a session prints is announced on the Screen's kind, which reaches the
/// Conversation being watched and not the list of them.
///
/// What comes back is how it ended, which is what whoever is driving decides
/// between carrying on and stopping by.
///
/// One parameter per thing the loop reads or writes, which is what makes the
/// list long: gathering them would be a struct built at one call site and taken
/// apart at the top of this, which is the same list said twice.
#[allow(clippy::too_many_arguments)]
async fn relay(
    pool: &SqlitePool,
    nudges: &Nudges,
    printing: Printing,
    session: &mut Launched,
    quiet: &Quiet,
    mut tail: Option<Tail>,
    mut limits: crate::limits::Watch,
    mut stopping: oneshot::Receiver<()>,
) -> Ended {
    // The Event alone here: what this loop says for itself it says in the log,
    // and everything it writes down is written by [`flush`] and [`summarise`],
    // which take the whole of `printing` because a Nudge needs the other half.
    let Printing { event_id, .. } = printing;

    let Launched {
        terminal,
        child,
        screen,
    } = session;

    let mut reading = Reading::default();
    let mut buffer = vec![0u8; CHUNK];
    let mut pending = String::new();
    let mut flushed = Instant::now();
    let mut tailed = Instant::now();
    let mut ending = false;

    // Whether the session has already been said to be quiet, so that it is said
    // once per silence rather than every time round the loop.
    let mut idle = false;

    loop {
        let deadline = tokio::time::Instant::from_std(flushed + FLUSH_EVERY);
        let following = tokio::time::Instant::from_std(tailed + FLUSH_EVERY);
        let idling = tokio::time::Instant::from_std(quiet.since() + IDLE_AFTER);

        tokio::select! {
            read = terminal.read(&mut buffer) => match read {
                // The far end of the terminal is closed, which is the session
                // gone.
                Ok(0) => break,
                Ok(taken) => {
                    quiet.spoke();
                    // Coming back out of the silence is a crossing too, and the
                    // sidebar hears about it on nothing else: what a session
                    // prints is announced on the Screen's kind, which reaches
                    // the Conversation being watched rather than the list of
                    // them. Said once, on the way out.
                    if idle {
                        idle = false;

                        nudges.announce(Nudge::Conversation {
                            conversation: printing.conversation_id,
                        });
                    }

                    let text = reading.take(&buffer[..taken]);
                    screen.printed(&text);
                    limits.printed(&text);
                    pending.push_str(&text);
                }
                Err(error) => {
                    tracing::error!(error = ?error, event_id, "reading a session's output failed");
                    break;
                }
            },
            _ = tokio::time::sleep_until(deadline), if !pending.is_empty() => {
                flush(pool, nudges, printing, &mut pending, &reading, told(&tail)).await;
                // On the same cadence as the flush, and after it: what is being
                // looked for is in what was just written down, and a stop the
                // Timeline had no output under would be a wait with nothing
                // above it saying where the session had got to.
                //
                // A limit found there stops the run and ends this session, in
                // that order — see [`crate::limits`], which writes the stop and
                // leaves the ending here because this is the task it is running
                // inside.
                if limits
                    .look(pool, nudges, tail.as_ref().and_then(Tail::latest))
                    .await
                    && !ending
                {
                    ending = true;
                    end_the_sandbox(child, event_id);
                }

                flushed = Instant::now();
            }
            _ = tokio::time::sleep_until(following), if tail.is_some() => {
                // Summarised on the poll that moved it rather than waiting for
                // the next flush: an agent that has stopped to think is one
                // whose terminal has gone quiet, and that is exactly when the
                // row saying what it last said is being read.
                if let Some(followed) = tail.as_mut()
                    && followed.poll(pool, nudges, event_id).await
                {
                    summarise(pool, nudges, printing, &reading, told(&tail)).await;

                    // The other record a session leaves behind, looked at the
                    // moment it moves: a backend that says its window is spent
                    // in its own log and not on its display would otherwise go
                    // unnoticed until the terminal happened to say something.
                    if limits
                        .look(pool, nudges, tail.as_ref().and_then(Tail::latest))
                        .await
                        && !ending
                    {
                        ending = true;
                        end_the_sandbox(child, event_id);
                    }
                }

                tailed = Instant::now();
            }
            _ = tokio::time::sleep_until(idling), if !idle => {
                idle = true;

                // The row on the Timeline and the sidebar card alike, which is
                // what the Conversation's own kind reaches. Nothing was written
                // — this is the one Nudge that is about a session having done
                // nothing.
                nudges.announce(Nudge::Conversation {
                    conversation: printing.conversation_id,
                });
            }
            _ = &mut stopping, if !ending => {
                ending = true;
                end_the_sandbox(child, event_id);
            }
        }
    }

    let last = reading.finish();
    screen.printed(&last);
    pending.push_str(&last);

    flush(pool, nudges, printing, &mut pending, &reading, told(&tail)).await;

    // And no look at it, deliberately: everything from here down is a session
    // that has gone. A limit in a session's last words is one it did not come
    // back from, which is the stop whoever is driving is about to write — and a
    // stop written here would be the run stopped on two things at once, with
    // Resume launching nothing.

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

    // And the last of the log, after the process that was writing it has been
    // reaped rather than when its terminal closed: an agent's final lines are
    // written on its way out, and a poll that stopped at the terminal would
    // leave a Transcript ending before the session did.
    //
    // Which is also why the summary is written again here. Those final lines
    // are ordinarily the whole point of the row — an agent says what it did as
    // it goes — and by now there is no output left to carry them.
    if let Some(followed) = tail.as_mut()
        && followed.poll(pool, nudges, event_id).await
    {
        summarise(pool, nudges, printing, &reading, told(&tail)).await;
    }

    ended
}

/// End the sandbox a session runs in, which is what reaches the session itself:
/// bwrap's child is the first process of a namespace of its own, and a namespace
/// whose first process is gone is a namespace with nothing left in it.
///
/// Asked for rather than waited on — the relay reads the terminal to its close
/// and reaps the child on its way out, which is the one place a session is ever
/// waited for. Its two callers are the word from outside that a session is to
/// end, and a stop the relay itself has just written for an exhausted window.
///
/// Nothing is refused for. A child that will not take the signal is one already
/// gone, which is the same instruction arriving too late to be needed.
fn end_the_sandbox(child: &mut Child, event_id: i64) {
    if let Err(error) = child.start_kill() {
        tracing::error!(error = ?error, event_id, "a session would not be ended");
    }
}

/// What the session's log says of it, for the summary the Timeline reads —
/// nothing at all where there is no log to follow.
fn told(tail: &Option<Tail>) -> Told<'_> {
    match tail {
        Some(tail) => Told {
            turns: tail.turns(),
            said: tail.latest(),
        },
        None => Told::default(),
    }
}

/// Put what has been printed since last time in the store, and tell whoever is
/// watching that it is there.
///
/// `told` is what the session's agent has said and how many turns it has taken,
/// off the log it keeps of its own conversation — which is what the Timeline row
/// is summarised by where there is one, see [`Reading::summary`].
async fn flush(
    pool: &SqlitePool,
    nudges: &Nudges,
    printing: Printing,
    pending: &mut String,
    reading: &Reading,
    told: Told<'_>,
) {
    if pending.is_empty() {
        return;
    }

    let Printing {
        conversation_id,
        event_id,
    } = printing;

    match store::append_capture(pool, event_id, pending, &reading.summary(told)).await {
        // Kept rather than dropped: the next flush carries it, and a store that
        // is briefly unwritable should cost latency rather than a hole in a
        // record nothing can go back and fill.
        Err(error) => {
            tracing::error!(error = ?error, event_id, "keeping a session's output failed")
        }
        Ok(()) => {
            pending.clear();
            // What the session printed, which is what both the Capture and the
            // Screen painted from it are read back off.
            nudges.announce(Nudge::Screen {
                conversation: conversation_id,
            });
        }
    }
}

/// Say again what the Timeline row reads, where the session said something
/// without printing anything to carry it.
///
/// The other half of [`flush`], and the two are the same write: what a session
/// says reaches the Transcript and what it prints reaches the Capture, and the
/// row is a line about both. A session that has stopped to think has moved one
/// and not the other.
async fn summarise(
    pool: &SqlitePool,
    nudges: &Nudges,
    printing: Printing,
    reading: &Reading,
    told: Told<'_>,
) {
    let Printing {
        conversation_id,
        event_id,
    } = printing;

    match store::summarise_capture(pool, event_id, &reading.summary(told)).await {
        Err(error) => {
            tracing::error!(error = ?error, event_id, "summarising a session failed")
        }
        // The row on the Timeline and nothing under it: what moved is the line
        // saying what the session is doing, which is the Conversation's own.
        Ok(()) => nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        }),
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
            models: vec!["claude-fable-5".to_owned(), "claude-opus-5".to_owned()],
            agent_type: store::AgentType::Claude,
        }
    }

    /// That Profile, paired with the second of its models — so the argv below
    /// says the pick and not the list.
    fn pairing() -> store::Pairing {
        store::Pairing {
            profile: profile(),
            model: Some("claude-opus-5".to_owned()),
        }
    }

    fn agents(agent: Vec<String>, state: &std::path::Path) -> Agents {
        Agents::running(
            agent,
            Home {
                path: PathBuf::from("/home/verkstead"),
            },
            Reachable::at("127.0.0.1:8422".parse().unwrap()),
            SandboxConfig::default(),
            Skills::installed(state).expect("this binary carries skills"),
            // A test harness is its own executable, and what a sandbox does with
            // one is bind it: any file that is really there will do where nothing
            // here runs it.
            Executable::of_the_server(),
            Handoffs::under(state),
            Settings::in_data_dir(state),
        )
    }

    /// The prompt is what the grilling starts from, and an interactive claude
    /// takes what it is to start on as a positional argument.
    #[test]
    fn a_session_runs_the_pairings_model_on_the_prompt() {
        let state = tempfile::tempdir().unwrap();
        let argv = agents(vec!["claude".to_owned()], state.path()).argv(
            &pairing(),
            "# Rate limiting\n",
            None,
        );

        assert_eq!(
            argv,
            vec![
                "claude".to_owned(),
                "--model".to_owned(),
                "claude-opus-5".to_owned(),
                "# Rate limiting\n".to_owned(),
            ]
        );
    }

    /// And a Conversation that chose its Profile before there was a model to
    /// choose beside it runs on the one that Profile carries, which is how
    /// everything chosen before pairings existed goes on working.
    #[test]
    fn an_unpaired_choice_runs_on_the_profiles_own_model() {
        let state = tempfile::tempdir().unwrap();
        let unpaired = store::Pairing {
            profile: profile(),
            model: None,
        };
        let argv = agents(vec!["claude".to_owned()], state.path()).argv(
            &unpaired,
            "# Rate limiting\n",
            None,
        );

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

    /// And a named session says so on the same line, after the prompt — see
    /// [`Agents::argv`] for why the end is where it goes.
    #[test]
    fn a_named_session_is_run_under_the_name_it_was_given() {
        let state = tempfile::tempdir().unwrap();
        let argv = agents(vec!["claude".to_owned()], state.path()).argv(
            &pairing(),
            "# Rate limiting\n",
            Some("d3b07384-d9a0-4c9b-8f2a-1b7c5e6f0a12"),
        );

        assert_eq!(
            argv,
            vec![
                "claude".to_owned(),
                "--model".to_owned(),
                "claude-opus-5".to_owned(),
                "# Rate limiting\n".to_owned(),
                "--session-id".to_owned(),
                "d3b07384-d9a0-4c9b-8f2a-1b7c5e6f0a12".to_owned(),
            ]
        );
    }

    /// A name claude will take as a session id, which is a version 4 UUID and
    /// nothing else — a malformed one is refused, and the session never starts.
    #[test]
    fn a_session_is_named_something_claude_will_accept() {
        let mut seen = std::collections::HashSet::new();

        for _ in 0..64 {
            let name = session_name().expect("this machine has a random generator");

            let groups: Vec<&str> = name.split('-').collect();
            assert_eq!(
                groups.iter().map(|group| group.len()).collect::<Vec<_>>(),
                vec![8, 4, 4, 4, 12],
                "{name:?} is not shaped like a UUID"
            );
            assert!(
                name.chars().all(|c| c == '-' || c.is_ascii_hexdigit()),
                "{name:?} has something in it that is not a hex digit"
            );
            assert!(
                groups[2].starts_with('4'),
                "{name:?} does not say it is a version 4 UUID"
            );
            assert!(
                ['8', '9', 'a', 'b'].contains(&groups[3].chars().next().unwrap()),
                "{name:?} does not say which variant of UUID it is"
            );

            assert!(seen.insert(name.clone()), "{name:?} was handed out twice");
        }
    }
}
