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
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use sqlx::SqlitePool;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;
use verkstead_schema::Nudge;

use crate::build_cache::{self, BuildCache};
use crate::capture::{Reading, Told};
use crate::handoffs::Handoffs;
use crate::nudge::Nudges;
use crate::platform::Platform;
use crate::runner::Pace;
use crate::sandbox::outliving;
use crate::sandbox::{Executable, Homes, Reachable, Sandbox, SandboxConfig, under_dev_shell};
use crate::screen::Live;
use crate::settings::Settings;
use crate::skills::{self, Skills};
use crate::store;
use crate::terminal::{Child, Terminal};
use crate::transcript::Tail;

/// How much of what is running on a pseudo-terminal to take off it at once.
///
/// A session's, and a Terminal's too — see [`crate::terminals`], which reads
/// its shell's output the same way and has no reason to read it in different
/// mouthfuls.
pub(crate) const CHUNK: usize = 8 * 1024;

/// How often what a session has printed reaches the store and the open pages.
///
/// A terminal application redraws many times a second, and a store written and
/// a page nudged on every one of those would be a cost that scaled with how
/// busy the agent's spinner was. Half a second is under what a human reads as a
/// delay and two orders of magnitude off what a redraw costs.
const FLUSH_EVERY: Duration = Duration::from_millis(500);

/// How long a session judged on what it prints has to print nothing before it
/// is called idle.
///
/// Short, because what it measures is a terminal rather than an agent: claude
/// repaints its spinner many times a second while it is working, so a session
/// that has printed nothing for this long is one that has stopped — sitting on
/// a Blocking Ask, or waiting for the human. Three seconds is clear
/// of the longest gap a working session leaves, and short enough that the mark
/// says so while it still matters.
///
/// Claude's, and calibrated on what claude draws. A backend that repaints a
/// full screen for ever is never silent for three seconds while it works and
/// need not be silent at all when it stops, so this says nothing about one —
/// see [`Judged`], which is where a session's backend decides how it is read.
const IDLE_AFTER: Duration = Duration::from_secs(3);

/// How a Conversation's agents are run: the home a sandbox reads the machine's
/// identity out of, where Verkstead itself is reachable from inside one, the
/// extra binds Sandbox Configuration asks for, the shared Rust build cache every
/// sandbox builds into, the skills every sandbox is given, the executable every
/// sandbox asks with, where a Conversation's handoff directory is made, where
/// the settings files are read from, and what an agent is on the command line.
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
    homes: Homes,
    reachable: Reachable,
    config: SandboxConfig,

    /// Where every session's Rust build goes — see [`BuildCache`]. Resolved at
    /// startup like the rest of this; whether a session gets it is the human's
    /// switch in `config.yaml`, read at every spawn.
    cache: BuildCache,

    skills: Skills,

    /// The executable every sandbox is given as `verkstead`: this server's own
    /// image — see [`Executable`].
    ///
    /// `None` where the server has none — one it could not find, or one it
    /// found and could not run — which is not a fallback to the machine's
    /// install but a session that does not start. Resolved *and probed* at
    /// startup like everything else here, so nothing is asked of the image
    /// again per spawn; which session a missing one costs is reported per
    /// session all the same, because that is the half of it startup cannot
    /// name — see [`Sessions::start`].
    verkstead: Option<Executable>,

    handoffs: Handoffs,
    settings: Settings,

    /// Something to run where a Profile's own binary goes, or `None` to run the
    /// one its agent type names — see [`binary`].
    ///
    /// An override rather than the answer itself, and the reason is that what
    /// this module has to be able to prove is that a session's output reaches
    /// the Timeline while it is still running. Proving it against the real
    /// claude would be a test that needed an account, a network and a model's
    /// patience — so a test stands its own program where the agent goes, and
    /// everything from the sandbox outwards is the same code the server runs.
    ///
    /// One override for every type rather than one per type: what a stub stands
    /// in for is *an agent*, and which line it is handed is still its Profile's
    /// type's own — see [`Agents::argv`] — so a backend's launch line stays
    /// provable without an account and the stubs go on reading what they read.
    ///
    /// `None` in a server, which runs the binary the Profile's type names.
    agent: Option<Vec<String>>,

    /// What a TUI backend's session has on its Screen, where anything is
    /// standing where that backend's binary goes — see [`Agents::signature`],
    /// whose answer this stands in for.
    ///
    /// A field for [`Agents::agent`]'s reason. What this module has to be able
    /// to prove is that a session drawing a full screen is judged idle off the
    /// frame rather than off its silence, and the backends that draw one are
    /// exactly the ones no test can launch — so a test stands a program that
    /// draws one where the backend goes, and hands its signature in here.
    ///
    /// `None` in a server, which is every signature a backend ships with — see
    /// [`Agents::signature`], where they are kept. Claude is judged on its
    /// silence whatever this holds: three seconds is its answer, and it draws no
    /// screen to read a prompt off.
    signature: Option<Signature>,

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
    /// The real thing: each Profile's own binary — see [`binary`] — under
    /// whichever account it names.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        homes: Homes,
        reachable: Reachable,
        config: SandboxConfig,
        cache: BuildCache,
        skills: Skills,
        verkstead: Option<Executable>,
        handoffs: Handoffs,
        settings: Settings,
    ) -> Agents {
        Agents {
            homes,
            reachable,
            config,
            cache,
            skills,
            verkstead,
            handoffs,
            settings,
            agent: None,
            signature: None,
            pace: Pace::default(),
        }
    }

    /// The same, with something else where every type's binary goes — see
    /// [`Agents::agent`].
    #[allow(clippy::too_many_arguments)]
    pub fn running(
        agent: Vec<String>,
        homes: Homes,
        reachable: Reachable,
        config: SandboxConfig,
        cache: BuildCache,
        skills: Skills,
        verkstead: Option<Executable>,
        handoffs: Handoffs,
        settings: Settings,
    ) -> Agents {
        Agents {
            agent: Some(agent),
            ..Agents::new(
                homes, reachable, config, cache, skills, verkstead, handoffs, settings,
            )
        }
    }

    /// The same, working the backlog at `pace` — see [`Agents::pace`].
    pub fn at_pace(self, pace: Pace) -> Agents {
        Agents { pace, ..self }
    }

    /// The same, with the prompt `signature` draws where a TUI backend's own
    /// goes — see [`Agents::signature`].
    pub fn drawing(self, signature: &str) -> Agents {
        Agents {
            signature: Some(Signature::AtThePrompt(signature.to_owned())),
            ..self
        }
    }

    /// What the installation configured as Sandbox Configuration, for the
    /// settings page rather than for a session: the page draws every bind there
    /// is and says which of the two places said each one, and these are the ones
    /// it may only show — see [`crate::paths`].
    pub fn binds(&self) -> &SandboxConfig {
        &self.config
    }

    /// The Sandbox a Conversation's work runs in, under the account `profile`
    /// names, with `argv` as it will be run inside it.
    ///
    /// Every sandboxed thing Verkstead starts in a Conversation comes through
    /// here: a session's agent, and the human's own shell in a Terminal
    /// ([ADR 0013](../../../docs/adr/0013-conversation-terminals.md)). What one
    /// gets is what the other gets — the Worktree as the working directory, the
    /// git directory, the handoff directory, the build cache, the Sandbox
    /// Configuration binds, the token, the git author and a `VERKSTEAD_SERVER`
    /// scoped to this Conversation — and one builder is what keeps that true as
    /// the surface moves.
    ///
    /// `argv` comes back wrapped in `nix develop --command` where the worktree's
    /// flake provides a dev shell, which is part of the same answer: what a
    /// session is run under is what a shell in the same worktree should be run
    /// under.
    ///
    /// **This blocks.** Git is asked where the worktree's object database is,
    /// the settings files are read, the handoff directory is made, and the
    /// dev-shell question is a `nix eval` or two — so callers hand it to a
    /// blocking thread.
    ///
    /// `None` where there is nothing to build one for: a Conversation with no
    /// Worktree, a checkout git will not own, a companion in the same state, a
    /// handoff directory that could not be made, or a server with no image of
    /// its own to equip a sandbox with.
    pub(crate) fn sandboxed(
        &self,
        conversation: &store::Conversation,
        profile: &store::Profile,
        argv: &[String],
    ) -> Option<(Sandbox, Vec<String>)> {
        // Read here rather than held from startup: this is the moment a
        // session's credentials and identity are decided, and it is already on
        // a blocking thread because git is asked about the worktree below.
        let secrets = self.settings.secrets();
        let config = self.settings.config();

        // And the one sccache server this machine compiles through, up before
        // whatever will reach for it — see [`BuildCache::compiling`]. Here
        // rather than at startup and only for a Repo that builds Rust, because
        // a machine that never builds Rust never needs one; and every time
        // rather than once, because the switch, the size and whether the server
        // is still alive are all read at this moment.
        if build_cache::builds_rust(&conversation.repo.path) {
            self.cache.compiling(config.rust_build_cache());
        }

        let sandbox = Sandbox::for_conversation(
            conversation,
            profile,
            &self.homes,
            &self.reachable,
            &self.skills,
            self.verkstead.as_ref()?,
            &self.handoffs,
            &secrets,
            &config,
            &self.cache,
            self.config.binds_for(conversation),
        )?;

        let worktree = conversation.worktree.clone()?;

        Some((
            sandbox,
            under_dev_shell(self.homes.platform(), &worktree, argv),
        ))
    }

    /// What a session of `conversation_id` under `pairing` on `prompt`, named
    /// `session`, working in `worktree`, runs.
    ///
    /// The binary is the Profile's agent type's — see [`binary`] — or whatever
    /// is standing where every type's goes, which is how a test proves this
    /// without an account.
    ///
    /// The model is the Pairing's, said on the command line rather than left to
    /// whatever the account's own settings hold: which model a session runs is
    /// the half of the choice the Profile does not make. A Conversation that
    /// chose its Profile before there was a model to choose beside it runs on
    /// the one that Profile carried — see [`store::Pairing::runs_on`]. What the
    /// flag is spelled is the backend's — see [`Line::model`]. The prompt
    /// follows it: as the one positional argument for the three backends that
    /// take it that way, and under the flag its own line names where a backend
    /// takes it flagged instead — see [`Line::prompt`].
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
    /// where an agent goes. Options added here go on the end, so nothing that
    /// was already there moves — which holds for every type but the one that
    /// asks for its prompt flagged, whose Brief the stub reads one place later
    /// and which is not a type the suite stands a stub in front of.
    ///
    /// `None` is a session Verkstead could not name — see [`session_name`] —
    /// and the flag is then left off entirely rather than passed empty: an agent
    /// told to run under no name at all would refuse to start, where one not
    /// told anything picks its own. A backend that takes no session id at all is
    /// told none whatever Verkstead named it — see [`Line::names_the_session`].
    ///
    /// Last of all comes the tail the backend itself needs — see [`Line::tail`]
    /// — which with the two above is the whole of what reads differently for one
    /// agent type than for another.
    ///
    /// **And on one platform the prompt is not on the line at all.** Windows
    /// caps a command line at 32,767 characters and an implementing session's
    /// prompt carries the whole handoff document inlined, so there the prompt is
    /// written into the Conversation's handoff directory and what goes in its
    /// place is one line naming the file — see [`Handoffs::wrote_prompt`].
    /// *Always* there rather than only where it would not fit, so a Windows
    /// session has one shape rather than two and nothing turns on a length
    /// nobody measured; and only there, because nothing on the other two
    /// platforms is the worse for the argument.
    ///
    /// The choice is off [`Platform`] as a value rather than a `cfg!`, for the
    /// reason every other choice here is: the arm this machine will never run
    /// is still an arm a test on it can ask for.
    ///
    /// `None` is a prompt that could not be written down, which is a session
    /// with nothing to be started on — the caller starts none, the way it
    /// starts none for a sandbox it could not build. Nothing else here can fail,
    /// so the platform that keeps its prompt on the line is always `Some`.
    ///
    /// **This blocks on the platform that writes the file**, which is why it is
    /// asked from the same blocking thread the sandbox is built on.
    fn argv(
        &self,
        conversation_id: i64,
        pairing: &store::Pairing,
        prompt: &str,
        session: Option<&str>,
        worktree: Option<&Path>,
    ) -> Option<Vec<String>> {
        let agent_type = pairing.profile.agent_type();
        let line = line(agent_type, worktree);

        let mut argv = match &self.agent {
            Some(standing) => standing.clone(),
            None => vec![binary(agent_type).to_owned()],
        };

        if let Some(model) = pairing.runs_on() {
            argv.push(line.model.to_owned());
            argv.push(model.to_owned());
        }

        if let Some(flag) = line.prompt {
            argv.push(flag.to_owned());
        }

        argv.push(match self.homes.platform() {
            Platform::Windows => self.handoffs.wrote_prompt(
                conversation_id,
                self.homes.for_conversation(conversation_id).handoffs(),
                prompt,
            )?,
            Platform::Linux | Platform::MacOs => prompt.to_owned(),
        });

        if let Some(session) = session.filter(|_| line.names_the_session) {
            argv.push("--session-id".to_owned());
            argv.push(session.to_owned());
        }

        argv.extend(line.tail);

        Some(argv)
    }

    /// What a session of `agent_type` has on its Screen that says whether it has
    /// stopped, and `None` where its idle is the silence itself.
    ///
    /// **One constant per backend**, the same bargain the usage-limit phrase
    /// makes — see [`crate::limits`]: the wording is the backend's and it will
    /// move, so it is kept in one place and costs one edit when it does. What
    /// puts a signature that has drifted in front of the human rather than
    /// leaving a session nothing ever catches is the long-stop behind it — see
    /// [`Judged::Drawing`].
    ///
    /// Claude has none, and none rather than an unknown one: it draws inline
    /// rather than repainting a screen, so there is no frame to read a prompt
    /// off, and three seconds of silence is an answer that works.
    ///
    /// Codex's is [`CODEX_AT_WORK`], Grok Build's is [`GROK_AT_WORK`] and
    /// OpenCode's is [`OPENCODE_AT_WORK`], and all three are the same one of
    /// the two readings — see [`Signature`]. Every backend that draws a screen
    /// has now been measured and every one of them came out that way round, so
    /// the at-the-prompt reading stands unused: what it is there for is the
    /// backend that turns out to differ, and none has. What stands where any of
    /// the three goes instead is whatever the suite handed in, which is the
    /// prompt drawn by the stub it stands where an agent goes: no backend here
    /// draws a prompt of its own, so that reading is proved against a stub
    /// rather than against an account.
    fn signature(&self, agent_type: store::AgentType) -> Option<Signature> {
        match agent_type {
            store::AgentType::Claude => None,
            store::AgentType::Codex => Some(
                self.signature
                    .clone()
                    .unwrap_or_else(|| Signature::AtWork(CODEX_AT_WORK.to_owned())),
            ),
            store::AgentType::Grok => Some(
                self.signature
                    .clone()
                    .unwrap_or_else(|| Signature::AtWork(GROK_AT_WORK.to_owned())),
            ),
            store::AgentType::OpenCode => Some(
                self.signature
                    .clone()
                    .unwrap_or_else(|| Signature::AtWork(OPENCODE_AT_WORK.to_owned())),
            ),
        }
    }

    /// And how a session of that type is judged idle — see [`Judged`].
    fn judged(&self, agent_type: store::AgentType) -> Judged {
        match self.signature(agent_type) {
            Some(signature) => Judged::Drawing {
                signature,
                long_stop: self.pace.long_stop,
            },
            None => Judged::Printing,
        }
    }
}

/// The program a Profile of each agent type is run as.
///
/// The host provides them, as it provides `claude` (ADR-0011): the installer
/// puts each backend's binary on the system profile the sandbox already reads,
/// and a Profile whose binary is missing fails at session start, named in the
/// Capture of the session that could not run.
///
/// A later backend adds one arm here and nothing else, which is the whole
/// reason this is a mapping rather than a name written into the line. The type
/// comes off the Pairing's Profile, so nothing has to be plumbed through to say
/// which agent is being launched.
fn binary(agent_type: store::AgentType) -> &'static str {
    match agent_type {
        store::AgentType::Claude => "claude",
        store::AgentType::Codex => "codex",
        store::AgentType::Grok => "grok",
        store::AgentType::OpenCode => "opencode",
    }
}

/// The rest of a backend's launch line: how it is told its model, how it is
/// given the Brief, whether it takes the session id Verkstead named, and what
/// goes on the end.
///
/// One value rather than a mapping apiece, because they are one fact — what
/// this backend's command line looks like — and a stage that lands a backend
/// writes it once here.
struct Line {
    /// What the model is said with. `--model` reads as claude's own; codex
    /// takes `-m`.
    model: &'static str,

    /// The flag the Brief goes under, or `None` where it is the one positional
    /// argument.
    ///
    /// The positional is what every backend up to this one takes, and opencode
    /// is the first that does not: its positional is the project to start in,
    /// and the prompt is `--prompt`. Said here rather than assumed, so that a
    /// backend taking either shape is one arm of the mapping below rather than
    /// a branch in the builder.
    prompt: Option<&'static str>,

    /// Whether the session Verkstead named is named on the line.
    ///
    /// False for a backend that takes no session id at all, whose log is
    /// therefore found rather than named — see [`crate::transcript`].
    names_the_session: bool,

    /// The flags and configuration overrides that go last, after the prompt.
    ///
    /// Owned rather than borrowed, because the trust pre-seed below names the
    /// Worktree this session is being launched in.
    tail: Vec<String>,
}

/// Which line `agent_type` takes, for a session working in `worktree`.
///
/// **Every backend is launched with its approval bypass on.** Running
/// unattended is what Verkstead promises rather than something the account's
/// own configuration is trusted to have been left holding: a session that
/// stopped to ask for approval would be asking it in front of nobody, with the
/// whole backlog behind it waiting on an answer that is not coming. What stops
/// a session doing harm is the Sandbox, which this does not touch and which is
/// still the boundary — and codex's own sandbox is off rather than nested,
/// because it will not start inside bwrap and bwrap is already the boundary.
///
/// Codex and grok both draw a full-screen TUI, and `--no-alt-screen` keeps each
/// drawing inline instead: the Capture and the Screen are the record of what a
/// session did, and an alternate screen is a record that is thrown away as the
/// program leaves it.
///
/// **opencode has no such flag and takes the alternate screen**, which is a
/// finding rather than an oversight (ADR-0011): its Screen reads either way,
/// since the screen model tracks which buffer is in front, and what its Capture
/// then replays to is the farewell banner opencode leaves on the ordinary
/// buffer as it exits rather than anything the session did. The record this
/// backend is read back from is its session store, which is what the Timeline
/// draws from. `--mini` — the minimal interface, which draws inline and carries
/// the same at-work label — is what to reach for the day the Capture has to be
/// that record instead.
///
/// **Grok Build is the one backend after Claude that takes the session id.** It
/// takes it under the spelling [`Agents::argv`] writes, it insists on a valid
/// UUID and it refuses one it already has a session for — all three of which
/// Verkstead's own names satisfy, being version-4 UUIDs drawn fresh per session.
/// So its log is named at launch rather than found afterwards, which is
/// Claude's shape and the opposite of Codex's. Its own sandbox is off for the
/// reason codex's is, and nothing about its account is said on the line: grok
/// reads its whole configuration out of the home the Profile named.
///
/// **What its account needs is said on the line rather than written into it.**
/// Codex takes `-c key=value` overrides, which is how the home is configured
/// without Verkstead writing into a directory that belongs to the human's
/// account: the credential store file-backed, since there is no keyring inside
/// the sandbox and a login that reached for one would find nothing; and the
/// Worktree pre-seeded as trusted, since some versions still put the trust
/// prompt up despite the bypass and a session stopped at a prompt is a run
/// waiting on nobody.
///
/// **OpenCode's is the shortest line here.** The model as `-m provider/model`
/// — the whole string is what the human typed on the Profile — the Brief under
/// `--prompt`, and `--auto` for the approvals, which is where the other three
/// backends' bypasses sit. `--prompt` submits rather than only prefilling
/// (checked against opencode 1.18.25: the home screen fills the prompt in and
/// sends it as soon as the model store is ready), so nothing is typed into the
/// terminal to start the session working. It has no sandbox of its own to
/// switch off, and it takes no session id at launch: `--session` means
/// *continue this one* and is validated against the store before the TUI
/// starts, so a fresh name would be a session that never starts rather than a
/// session named — which is why its log is found rather than named, as codex's
/// is. Everything else about the account is in the directories the Profile
/// named — see [`crate::sandbox`].
fn line(agent_type: store::AgentType, worktree: Option<&Path>) -> Line {
    match agent_type {
        store::AgentType::Claude => Line {
            model: "--model",
            prompt: None,
            names_the_session: true,
            tail: vec!["--dangerously-skip-permissions".to_owned()],
        },
        store::AgentType::Codex => {
            let mut tail = vec![
                "--dangerously-bypass-approvals-and-sandbox".to_owned(),
                "--no-alt-screen".to_owned(),
                "-c".to_owned(),
                format!("{CODEX_CREDENTIAL_STORE}=\"file\""),
            ];

            // A session with no Worktree is one that never starts — see
            // [`Sessions::start`] — so what this leaves off is the trust of a
            // directory there is none of, rather than a prompt let through.
            //
            // The whole table rather than the one key under it, because codex
            // splits a `-c` key on every dot it finds and does not stop at the
            // quotes: a Worktree named for a Repo or a branch with a dot in it
            // — and either may have one — is a path
            // `projects."…".trust_level` addresses some other way, and the
            // session then sits on the trust prompt for ever. A table on the
            // right-hand side is read as the TOML it is.
            if let Some(worktree) = worktree {
                tail.push("-c".to_owned());
                tail.push(format!(
                    "projects={{\"{}\"={{trust_level=\"trusted\"}}}}",
                    worktree.display()
                ));
            }

            Line {
                model: "-m",
                prompt: None,
                names_the_session: false,
                tail,
            }
        }
        store::AgentType::Grok => Line {
            model: "-m",
            prompt: None,
            names_the_session: true,
            tail: vec![
                "--always-approve".to_owned(),
                "--sandbox".to_owned(),
                "off".to_owned(),
                "--no-alt-screen".to_owned(),
            ],
        },
        store::AgentType::OpenCode => Line {
            model: "-m",
            prompt: Some("--prompt"),
            names_the_session: false,
            tail: vec!["--auto".to_owned()],
        },
    }
}

/// Where codex keeps the credentials a login writes, which inside the sandbox
/// has to be the file beside the configuration rather than a keyring.
///
/// Named because it is somebody else's spelling, the same bargain the
/// usage-limit phrase and the idle signature make: one place to edit when it
/// moves.
const CODEX_CREDENTIAL_STORE: &str = "cli_auth_credentials_store";

/// What codex has on its Screen while it is working, and nothing of what it has
/// there when it is waiting for a human — see [`Signature::AtWork`].
///
/// Read off codex 0.149.0 rather than guessed at, and the reading is the whole
/// reason this backend's answer is an at-work line rather than a prompt: the
/// frame codex leaves when its turn is over and the frame it draws mid-turn are
/// the same screen but for this one line. The composer, its placeholder — `Ask
/// Codex to do anything` — and the bar under it stand in both, so none of them
/// says anything about whether the session has stopped.
///
/// The fragment rather than the whole line, because the rest of it moves while
/// this does not: the spinner glyph in front changes every frame, and the
/// seconds count up.
///
/// Named for the same reason the spelling above is, and it is the same bargain
/// the usage-limit phrase makes: the wording is codex's and it will move, and
/// moving it costs one edit here.
const CODEX_AT_WORK: &str = "esc to interrupt";

/// What grok has on its Screen while it is working, and nothing of what it has
/// there when it is waiting for a human — see [`Signature::AtWork`].
///
/// Read off grok 1.0.13 driven on a hundred-column terminal rather than guessed
/// at, and it comes out where codex came out: the frame grok leaves when its
/// turn is over and the frame it draws mid-turn are the same screen but for the
/// live status line — `⠧ Responding… 5.7s … [stop]` — and this hint on the row
/// under the composer. The composer itself, its `❯`, the `grok-4.6 ·
/// always-approve` label on its border and the `Shift+Tab:mode` and
/// `Ctrl+x:shortcuts` hints beside this one stand in both, so none of them says
/// whether the session has stopped.
///
/// The hint rather than the `[stop]` chip, which goes and comes with it: the
/// hints are the row grok draws at the foot of every frame, where the status
/// line is there only while a turn runs, and a keybinding label is a harder
/// thing to find by accident in what the session printed than a bracketed word
/// is.
///
/// The fragment rather than the whole row, because the rest of it moves while
/// this does not: a turn that has backgrounded something adds `Ctrl+b:send to
/// bg` beside it, and the hints at rest change with what the composer is
/// offering.
///
/// Named for the same reason [`CODEX_AT_WORK`] is, and it is the same bargain
/// the usage-limit phrase makes: the wording is grok's and it will move, and
/// moving it costs one edit here.
const GROK_AT_WORK: &str = "Esc:cancel";

/// What opencode has on its Screen while it is working, and nothing of what it
/// has there when it is waiting for a human — see [`Signature::AtWork`].
///
/// Read off opencode 1.18.25 driven on a hundred-column terminal rather than
/// guessed at, and it comes out where the two before it came out: the frame
/// opencode leaves when its turn is over and the frame it draws mid-turn are
/// the same screen but for the status bar at its foot. Mid-turn that bar is a
/// progress dial and this label — `⬝⬝⬝⬝⬝■■■  esc interrupt` — and at rest it is
/// the project's path instead. The composer above it, the `Build auto ·
/// <model>` label on its border and the `tab agents` and `ctrl+p commands`
/// hints beside this one stand in both, so none of them says whether the
/// session has stopped. Across two turns of one session sampled once a second —
/// a tool call and then a streamed reply, twice — the label was in every
/// working frame and in none of the resting ones.
///
/// The label rather than the dial in front of it, which goes and comes with it:
/// the dial's cells fill and empty every frame where this does not move, and a
/// keybinding label is a harder thing to find by accident in what the session
/// printed than a run of block characters is.
///
/// Two words where codex's is three — opencode writes `esc interrupt` where
/// codex writes `esc to interrupt` — so neither backend's constant reads the
/// other's frame, which is what the tests on this reading turn on.
///
/// **And it is the same label in either interface opencode offers.** The
/// minimal one — `--mini`, which is what draws inline rather than taking the
/// alternate screen — puts it in a status bar of its own, and it goes there
/// when the turn is over exactly as it goes here. So the reading does not turn
/// on which of the two a session was started in, whatever a later stage
/// decides about the Capture (ADR-0011).
///
/// Named for the same reason [`CODEX_AT_WORK`] is, and it is the same bargain
/// the usage-limit phrase makes: the wording is opencode's and it will move,
/// and moving it costs one edit here.
const OPENCODE_AT_WORK: &str = "esc interrupt";

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

    /// And whether a session this platform runs stands outside a Sandbox, which
    /// is a fact about the build rather than about this router — see
    /// [`Sessions::unsandboxed`].
    unsandboxed: bool,

    running: Arc<Mutex<HashMap<i64, Running>>>,

    /// And the backend of the session each Conversation is *launching*, held
    /// from before its process exists until it is on the register above — see
    /// [`Sessions::launching`].
    ///
    /// The register cannot answer for that stretch, and there is one thing that
    /// has to be answered in it: how a session asks. A process is spawned and
    /// then written down, and between the two it is running and already able to
    /// reach the server — so a Set it sends in that window would be read as one
    /// from outside a session and waited on, on a backend whose sessions cannot
    /// wait. See [`Sessions::channel`], which is the whole of what this is for.
    launching: Arc<Mutex<HashMap<i64, store::AgentType>>>,

    /// Whose turn it is in each Conversation's Worktree — see [`Sessions::turn`].
    turns: Arc<Mutex<HashMap<i64, Arc<tokio::sync::Mutex<()>>>>>,
}

/// The Worktree of one Conversation, held for as long as one thing is using it.
///
/// Dropping it is what hands it on, so it is held across the whole of a session
/// rather than taken to start one: what it is protecting is not the launching but
/// the working.
pub(crate) type Turn = tokio::sync::OwnedMutexGuard<()>;

/// The note that a Conversation is launching a session, held for as long as the
/// launch takes — see [`Sessions::launching`].
///
/// A guard rather than a pair of calls, because what it is covering has three
/// ways out: a process that would not start, a Capture that could not be opened,
/// and the one that worked. Only the last of them leaves anything on the
/// register, and none of them should leave this behind.
struct Launching {
    launching: Arc<Mutex<HashMap<i64, store::AgentType>>>,
    conversation_id: i64,
}

impl Drop for Launching {
    fn drop(&mut self) {
        self.launching
            .lock()
            .expect("the launching registry is not poisoned")
            .remove(&self.conversation_id);
    }
}

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

    /// Whether it has stopped, and how long ago — see [`Idle`].
    pub(crate) idle: Idle,

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
    /// gone quiet, the human closed the Conversation or pressed Force stop out
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
    /// session went quiet, so there is nothing to tell. The human closed the
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

/// Whether a session is idle, how long it has been, and whether it has ever
/// said anything at all.
///
/// **One judgement, read by everything that has ever asked**: the mark on the
/// sidebar and on the Conversation, every ender's grace, and Rescue — both the
/// span it waits out and the moment it proves a stir by. What makes it one
/// judgement rather than a rule per caller is that idle means different things
/// on different backends, and a caller keeping its own reading of the clock
/// would be a backend judged one way by the sidebar and another by the thing
/// that ends it. See [`Judged`].
///
/// Shared with the relay, which puts it back to now on everything that arrives.
/// That is what makes a grace period safe to end a session on: a session still
/// working is never one to end, however long it goes on for, and the work a
/// session does after its commit — a message, a summary, a push — runs to
/// completion rather than being cut off mid-sentence.
///
/// The last part is for the driver that has nothing else to read. A session
/// ended on its own quiet is one being taken at its word, and a session that
/// never said a word has given none: see [`Idle::said_anything`].
#[derive(Debug, Clone)]
pub(crate) struct Idle {
    /// How this session's backend is read — the same for the whole of its life,
    /// because it is a fact about which agent is running.
    judged: Judged,

    /// And what the relay has seen of it, which is what the judgement is made
    /// of.
    silence: Arc<Mutex<Silence>>,
}

/// How a session's backend says it has stopped.
///
/// Two readings of the one terminal, and which of them a session gets is its
/// agent type's — see [`Agents::signature`], which is where each backend's
/// answer is kept.
#[derive(Debug, Clone)]
enum Judged {
    /// By what it prints: [`IDLE_AFTER`] with nothing arriving. Claude's, and
    /// the rule every session was read by before there was a second backend.
    Printing,

    /// By what it draws: this backend's signature read off the Screen, with a
    /// long byte-quiet behind it.
    ///
    /// A full-screen interface is never reliably silent — it repaints while it
    /// works and may go on repainting its prompt after it has stopped — so
    /// silence says nothing about one either way, and what does is the frame it
    /// leaves on the terminal. Which of the two things a frame can say is this
    /// backend's — see [`Signature`].
    ///
    /// **The long-stop is what a drifted signature lands in.** The wording is
    /// the backend's and will move, and a signature that no longer matches
    /// reads as a session that never stops: Rescue's precondition is idle,
    /// every ender waits on the same judgement, and no session carries a cap on
    /// its life. So a session that has printed nothing for `long_stop` is idle
    /// whatever its screen says, and what the human gets is the ordinary
    /// would-not-ask stop — one slow round rather than never. See
    /// [`crate::runner::Pace::long_stop`].
    Drawing {
        signature: Signature,
        long_stop: Duration,
    },
}

/// The one line a backend draws that says whether it has stopped, and which of
/// the two things it says.
///
/// A backend is read either way round, and which way is a fact about what it
/// draws rather than a choice:
///
/// - one that draws a prompt of its own when it is waiting says so by that line
///   *standing*, and
/// - one whose waiting frame is indistinguishable from its working frame says so
///   by its at-work line *going*.
///
/// Codex is the second, and it is the reason there are two — see
/// [`CODEX_AT_WORK`], where the frames it draws are set out.
///
/// **The two read the silence differently, and they have to.** A prompt standing
/// is a whole answer: the backend has drawn the thing it draws only when it is
/// waiting, and nothing else has to agree with it. An at-work line *gone* is
/// half of one — the line is missing from the frame before the first frame is
/// drawn, and it is missing again from every frame of a session drawing
/// something Verkstead has never seen — so the ordinary [`IDLE_AFTER`] quiet is
/// asked for beside it. That is what keeps a drifted at-work phrase from
/// stopping a session in the middle of its work: a working TUI repaints, and one
/// that is repainting is never quiet.
#[derive(Debug, Clone)]
pub(crate) enum Signature {
    /// The line this backend draws when it is sitting at its prompt: standing
    /// says the session has stopped, and it is the whole of the judgement.
    AtThePrompt(String),

    /// The line this backend draws while it is working, where it draws nothing
    /// of its own when it is waiting: gone says the session has stopped, once
    /// [`IDLE_AFTER`] of quiet says so too.
    AtWork(String),
}

impl Signature {
    /// Whether the frame on `screen` says the session has stopped.
    fn at_rest(&self, screen: &Live) -> bool {
        match self {
            Signature::AtThePrompt(line) => screen.showing(line),
            Signature::AtWork(line) => !screen.showing(line),
        }
    }

    /// And when a session whose frame has said so since `at_rest` — having last
    /// printed at `printed` — is idle.
    ///
    /// At once for a prompt, which is an answer on its own; [`IDLE_AFTER`] after
    /// the last byte for an at-work line, which is half of one.
    fn settled(&self, at_rest: Instant, printed: Instant) -> Instant {
        match self {
            Signature::AtThePrompt(_) => at_rest,
            Signature::AtWork(_) => printed + IDLE_AFTER,
        }
    }

    /// And the moment such a session stopped, which is what every span of idle
    /// is measured from and what *last seen at work* answers with.
    ///
    /// When its prompt first stood, for a backend that draws one — it has been
    /// sitting there since, whatever else its terminal has done. Its last byte
    /// for a backend read by its at-work line, because a backend that draws only
    /// while it works was working right up to the moment it went quiet.
    fn stopped_at(&self, at_rest: Instant, printed: Instant) -> Instant {
        match self {
            Signature::AtThePrompt(_) => at_rest,
            Signature::AtWork(_) => printed,
        }
    }

    /// Whether a session that has drawn nothing at all is at rest under this
    /// reading.
    ///
    /// It is under an at-work one: a session that has drawn nothing is not
    /// drawing that it is at work, and what says it has stopped is then the
    /// quiet alone — which is Claude's own rule, and the right one for a
    /// launched session that never got going. Under a prompt it is not: nothing
    /// drawn is no prompt drawn.
    fn at_rest_undrawn(&self) -> bool {
        matches!(self, Signature::AtWork(_))
    }
}

/// What the clock holds: when the session last printed anything, whether that
/// was ever it saying anything rather than it starting, and when the judgement
/// last turned to idle.
#[derive(Debug)]
struct Silence {
    /// The moment it was last put back — the session's last word, or the moment
    /// it was launched where it has had none.
    at: Instant,

    /// Whether it has said anything since it started.
    spoke: bool,

    /// When the judgement last said the session had stopped, and `None` while it
    /// says the session is at work.
    ///
    /// Only ever moved by something arriving, because that is the only thing
    /// that changes what is drawn: a prompt redrawn is the same silence going
    /// on rather than a new one, so a signature already standing keeps the
    /// moment it first stood.
    ///
    /// Set from the launch under an at-work reading, where a session that has
    /// drawn nothing is a session not drawing that it is at work — see
    /// [`Signature::at_rest_undrawn`].
    ///
    /// Never set at all under [`Judged::Printing`], where the silence itself is
    /// the judgement and [`Silence::at`] is the whole of it.
    idling_since: Option<Instant>,
}

impl Idle {
    fn started(judged: Judged) -> Idle {
        let now = Instant::now();
        let undrawn = match &judged {
            Judged::Printing => false,
            Judged::Drawing { signature, .. } => signature.at_rest_undrawn(),
        };

        Idle {
            judged,
            silence: Arc::new(Mutex::new(Silence {
                at: now,
                spoke: false,
                idling_since: undrawn.then_some(now),
            })),
        }
    }

    /// The session printed, and `screen` has what it printed on it: put the
    /// clock back, and read the judgement off the frame it left.
    ///
    /// After the Screen has been fed rather than before it, which is the whole
    /// of what makes the reading exact — the frame a backend's prompt appears on
    /// is drawn by the very text this is being told about.
    fn printed(&self, screen: &Live) {
        // Outside the lock, because it takes the Screen's: two locks held at
        // once are two locks that can be taken in two orders.
        let at_rest = match &self.judged {
            Judged::Printing => false,
            Judged::Drawing { signature, .. } => signature.at_rest(screen),
        };

        let mut silence = self.silence();
        let now = Instant::now();

        silence.at = now;
        silence.spoke = true;

        if at_rest {
            silence.idling_since.get_or_insert(now);
        } else {
            silence.idling_since = None;
        }
    }

    /// Whether the session is idle as of now.
    ///
    /// What the sidebar and the Conversation's own row are drawn from, and the
    /// mark the relay announces the crossing of.
    pub(crate) fn idling(&self) -> bool {
        let silence = self.silence();

        match &self.judged {
            Judged::Printing => silence.at.elapsed() >= IDLE_AFTER,
            Judged::Drawing {
                signature,
                long_stop,
            } => {
                let settled = silence
                    .idling_since
                    .is_some_and(|since| signature.settled(since, silence.at) <= Instant::now());

                settled || silence.at.elapsed() >= *long_stop
            }
        }
    }

    /// And how long it has been, which is what every grace is measured against.
    ///
    /// [`Duration::ZERO`] where it is not idle at all: a backend judged on its
    /// screen is at work however long it has been between frames, which is the
    /// point of judging it that way — a TUI that falls silent for a moment
    /// mid-turn would otherwise be reaped out from under its own work.
    ///
    /// Past the long-stop the whole silence counts, rather than the part of it
    /// after the long-stop: the session *was* stopped for all of it, and this is
    /// the moment Verkstead is willing to say so.
    pub(crate) fn for_how_long(&self) -> Duration {
        let silence = self.silence();

        match &self.judged {
            Judged::Printing => silence.at.elapsed(),
            Judged::Drawing {
                signature,
                long_stop,
            } => {
                let drawn = silence
                    .idling_since
                    .filter(|since| signature.settled(*since, silence.at) <= Instant::now())
                    .map(|since| signature.stopped_at(since, silence.at).elapsed())
                    .unwrap_or_default();
                let printed = silence.at.elapsed();

                if printed >= *long_stop {
                    drawn.max(printed)
                } else {
                    drawn
                }
            }
        }
    }

    /// When it will be idle if nothing else arrives, for whoever wants to sleep
    /// until it is rather than to keep asking.
    ///
    /// A moment already past where it is idle now, which is a sleep that is over
    /// before it starts — exactly what a caller waiting for the crossing wants
    /// of a session that has already crossed.
    pub(crate) fn crossing(&self) -> Instant {
        let silence = self.silence();

        match &self.judged {
            Judged::Printing => silence.at + IDLE_AFTER,
            Judged::Drawing {
                signature,
                long_stop,
            } => match silence.idling_since {
                Some(since) => signature.settled(since, silence.at),
                None => silence.at + *long_stop,
            },
        }
    }

    /// Whether the session has said anything at all since it started.
    ///
    /// What tells a session that finished from one that never got going, for the
    /// driver whose only signal is silence — see [`crate::runner`]'s
    /// propose-then-fix rule. A session that reports through the repository has a
    /// commit or an artifact to be read as done; one that reports through nothing
    /// but its own words has said nothing, and *nothing* is not a report.
    ///
    /// Every byte counts here, whatever the judgement makes of it: what this
    /// asks is whether the session ever got going, and a frame drawn is a
    /// session that did.
    pub(crate) fn said_anything(&self) -> bool {
        self.silence().spoke
    }

    /// When it was last seen at work, for whoever wants to know whether that was
    /// *after* something else — an answer handed to the session, a line typed
    /// into it — which is a question about the order of two moments rather than
    /// about a span. See [`crate::rescues::until_it_will_not_ask`], where a
    /// session seen working later than the stir is the proof that the stir
    /// reached it at all.
    ///
    /// The same judgement read as a moment rather than as a span, and it has to
    /// be: a byte is free on a backend that repaints, so a session's last *word*
    /// would prove nothing there.
    ///
    /// The moment it was launched, where it has done nothing yet: a session that
    /// never got going has been stopped since it started, which is exactly what
    /// this reader wants of one.
    pub(crate) fn since(&self) -> Instant {
        let silence = self.silence();

        match &self.judged {
            Judged::Printing => silence.at,
            Judged::Drawing { signature, .. } => silence
                .idling_since
                .filter(|since| signature.settled(*since, silence.at) <= Instant::now())
                .map(|since| signature.stopped_at(since, silence.at))
                .unwrap_or(silence.at),
        }
    }

    fn silence(&self) -> std::sync::MutexGuard<'_, Silence> {
        self.silence
            .lock()
            .expect("a session's idle clock is not poisoned")
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
    idle: Idle,
    ended: watch::Receiver<Option<Ended>>,

    /// Whether the process has gone, set the moment the relay reads
    /// end-of-file — see [`Sessions::alive`], which is the whole of what it is
    /// for.
    gone: Arc<AtomicBool>,

    /// Which backend it is, so that a Set it asks is stored the way that backend
    /// asks — see [`Sessions::channel`].
    agent_type: store::AgentType,
}

/// Whether a session a Verkstead built for `platform` runs stands outside a
/// Sandbox.
///
/// The one place the fact is decided, and the whole of the decision: the two
/// Unixes have a Sandbox each — bubblewrap and seatbelt — and a Windows one has
/// none yet, so its agent runs as an ordinary process with the human's own
/// account's reach. Sessions themselves run everywhere: what stood in the way
/// of a Windows one was a pseudo-terminal, and [`crate::terminal`]'s Windows
/// arm is a pseudoconsole.
///
/// **Not a refusal.** A session that runs unsandboxed is a session, and what
/// this decides is one sentence on the Conversation view rather than a press
/// that will not go — see [`verkstead_render::ConversationView::unsandboxed`],
/// which is where it is read.
///
/// A function of the platform rather than a `cfg!`, for the reason
/// [`Platform`] is a value: the arm this machine will never run is still an arm
/// its tests call. What a running server answers is [`Platform::HERE`]'s answer,
/// and it is [`Sessions::under`] that asks — everything above reads it off the
/// registry rather than off the target it was compiled for.
pub(crate) fn unsandboxed_on(platform: Platform) -> bool {
    match platform {
        Platform::Linux | Platform::MacOs => false,
        Platform::Windows => true,
    }
}

impl Sessions {
    /// A server that can run sessions, under `agents`.
    ///
    /// Which is what the served router is built with, so this is where the
    /// platform's own answer is read: a Windows one runs its sessions outside a
    /// Sandbox, whatever agents it was handed — see [`unsandboxed_on`].
    pub(crate) fn under(agents: Agents) -> Sessions {
        Sessions {
            agents: Some(Arc::new(agents)),
            unsandboxed: unsandboxed_on(Platform::HERE),
            running: Arc::new(Mutex::new(HashMap::new())),
            launching: Arc::new(Mutex::new(HashMap::new())),
            turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// The skills every session this server runs is given, or `None` where it
    /// runs none.
    ///
    /// Here because a prompt names them: what sends a session into a skill is
    /// the path it is told to read, and where that path is depends on where
    /// they were installed — see [`Skills::inside`]. A server with no agents
    /// has no skills to name and nothing to start, so the two are `None`
    /// together.
    pub(crate) fn skills(&self) -> Option<&Skills> {
        self.agents.as_deref().map(|agents| &agents.skills)
    }

    /// How this server runs anything inside a Conversation's Sandbox, or `None`
    /// where it runs nothing at all.
    ///
    /// Here because a session is not the only thing that goes in one: a
    /// Terminal is the human's own shell in the same Sandbox, built by the same
    /// [`Agents::sandboxed`] — see [`crate::terminals`]. Held by the sessions
    /// register rather than beside it because this is what was resolved at
    /// startup and handed in, and two copies of it would be two answers about
    /// one machine.
    pub(crate) fn agents(&self) -> Option<Arc<Agents>> {
        self.agents.clone()
    }

    /// One that cannot: nothing is launched, and everything else about starting
    /// a grilling holds.
    ///
    /// It answers sandboxed whatever machine it was built for, which is the one
    /// place the platform's own answer is not read. Only a test stands one of
    /// these up, and what a test stands it up for is what a press leaves behind
    /// — the branch, the worktree, the record — rather than what platform it is
    /// running on.
    ///
    /// What a Windows build answers is asked of [`Sessions::unsandboxed_here`]
    /// instead, on whichever machine is running the tests.
    pub(crate) fn none() -> Sessions {
        Sessions {
            agents: None,
            unsandboxed: false,
            running: Arc::new(Mutex::new(HashMap::new())),
            launching: Arc::new(Mutex::new(HashMap::new())),
            turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// And one whose sessions run outside a Sandbox — which today is a Windows
    /// build, and which is the whole of what it says.
    ///
    /// The arm a Linux machine will never be, stood up so that its tests can
    /// call it: what the Conversation view says about a session that has no
    /// Sandbox around it is a rule rather than a platform, and it is asked
    /// wherever the suite runs. See [`crate::router_running_unsandboxed`].
    pub(crate) fn unsandboxed_here() -> Sessions {
        Sessions {
            unsandboxed: true,
            ..Sessions::none()
        }
    }

    /// Whether a session started here runs outside a Sandbox — what the panes a
    /// session is started from and watched on say in a line, and nothing else.
    ///
    /// Not [`Sessions::runs_sessions`], which is a fact about this router:
    /// whether it was given agents. This one is a fact about the build, and it
    /// gates nothing at all — a session runs either way, and the difference is
    /// what reach it has.
    pub(crate) fn unsandboxed(&self) -> bool {
        self.unsandboxed
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

    /// How a Conversation's running session asks: the channel its backend's
    /// agent type names — see [`store::AgentType::channel`].
    ///
    /// The register rather than the Conversation's Pairings, because what this
    /// decides is how *this* session's Set is stored and a Conversation runs its
    /// roles under Pairings that need not agree — a wrap-up's review may be on
    /// one backend and the work on another.
    ///
    /// The register **or the launch that has not reached it yet**, because a
    /// session is running before it is written down: [`Sessions::start`] spawns
    /// the process and then opens the Capture it prints into, and an agent that
    /// asks in between would be asking from a Conversation the register says has
    /// nothing running. It is the fixture's stub that does that every time and a
    /// loaded machine that lets a real one, and the answer is wrong either way —
    /// so the backend is written down before there is a process to ask, and taken
    /// off again when the register has it. See [`Sessions::launching`].
    ///
    /// [`store::Channel::Blocking`] where neither has one, which is what a Set
    /// arriving from outside a session is: a router with no agents at all, and
    /// the human's own devices, which never post one here. It is also the safe
    /// way round — a wait opened on a Set nobody will nudge about ends when the
    /// CLI that opened it does, where a Set stored for a session that is not
    /// idling would be one nobody ever comes back for.
    pub(crate) fn channel(&self, conversation_id: i64) -> store::Channel {
        let running = self
            .running
            .lock()
            .expect("the sessions registry is not poisoned")
            .get(&conversation_id)
            .map(|running| running.agent_type);

        running
            .or_else(|| {
                self.launching
                    .lock()
                    .expect("the launching registry is not poisoned")
                    .get(&conversation_id)
                    .copied()
            })
            .map(store::AgentType::channel)
            .unwrap_or(store::Channel::Blocking)
    }

    /// Write down that this Conversation is launching a session on `agent_type`,
    /// and hand back the note to hold while it does.
    ///
    /// Taken before the process is spawned and dropped when [`Sessions::start`]
    /// returns, which is after the session is on the register — so the two
    /// answers meet rather than leaving a gap, and a launch that failed leaves
    /// nothing behind for the next Set to read.
    fn launching(&self, conversation_id: i64, agent_type: store::AgentType) -> Launching {
        self.launching
            .lock()
            .expect("the launching registry is not poisoned")
            .insert(conversation_id, agent_type);

        Launching {
            launching: self.launching.clone(),
            conversation_id,
        }
    }

    /// Whether a Conversation's running session has stopped — its backend's own
    /// judgement of that, see [`Idle`].
    ///
    /// `false` for a Conversation with nothing running, which is the answer
    /// that reads right wherever it is asked: idle is a thing a *running*
    /// session is, and a session that has ended is neither.
    ///
    /// Read at the moment a Conversation is drawn rather than stored, as
    /// [`Sessions::writing`] is and for the same reason — whether a process has
    /// stopped is a fact about a process. The crossing is announced as it
    /// happens too, because a session going idle is exactly when it stops
    /// producing the Nudges an open page re-reads on; see [`relay`].
    pub(crate) fn idling(&self, conversation_id: i64) -> bool {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .get(&conversation_id)
            .is_some_and(|running| running.idle.idling())
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

    /// Whether the session named is a process that is still there.
    ///
    /// Narrower than [`Sessions::screen`], which answers about the register, and
    /// the difference between them is the whole reason this exists: a session
    /// stays on the register until its last sweep of the branch has finished,
    /// which happens well after the process it belongs to has gone. A watcher
    /// reading the final frame of a session that has just exited wants the
    /// register's answer. Anything about to *speak* to a session wants this one
    /// — see [`crate::rescues`], which would otherwise type into a terminal
    /// nothing is reading and count the silence against it.
    pub(crate) fn alive(&self, conversation_id: i64, event_id: i64) -> bool {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .get(&conversation_id)
            .filter(|running| running.event_id == event_id)
            .is_some_and(|running| !running.gone.load(Ordering::Acquire))
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
                idle: running.idle.clone(),
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

    /// And which of those have stopped — [`Sessions::idling`] for the whole
    /// sidebar at once, and one lock rather than one per row for the same reason
    /// [`Sessions::working`] is.
    ///
    /// A subset of [`Sessions::working`] by construction, because both are the
    /// same register read: idle is a thing a running session is, and a
    /// Conversation with nothing in it is in neither set.
    pub(crate) fn idle(&self) -> HashSet<i64> {
        self.running
            .lock()
            .expect("the sessions registry is not poisoned")
            .iter()
            .filter(|(_, running)| running.idle.idling())
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

    /// Whether a session's Rust build would have its *compiling* cached and not
    /// only its downloads — which is whether this server found an sccache to
    /// hand out. See [`BuildCache::caches_compiles`].
    ///
    /// False for a server that runs no sessions at all, which is the same answer
    /// by another route: there is nothing to give a cache to.
    pub(crate) fn caches_compiles(&self) -> bool {
        self.agents
            .as_ref()
            .is_some_and(|agents| agents.cache.caches_compiles())
    }

    /// Run `pairing`'s agent on `prompt`, inside `conversation`'s sandbox, and
    /// put what it prints on the Timeline as it arrives.
    ///
    /// The session that was started, for whoever is driving it — see
    /// [`Session`]. A server with no way to run agents starts none, a server
    /// with no image of its own to equip one with starts none either, and a
    /// sandbox that cannot be built — a Conversation with no worktree, or one
    /// git will not own — is the same answer: there is nothing here to launch.
    /// All three are logged, because each of them means a Conversation that is
    /// grilling with nothing grilling it.
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
        //
        // Either of the two ways there can be no image reaches this: one the
        // server could not find, and one it found and could not run. Which of
        // them it was is in the startup log — see [`Executable::probed`] — and
        // saying it again per session would be repeating at every refusal what
        // does not change between them.
        if agents.verkstead.is_none() {
            tracing::error!(
                conversation_id = conversation.id,
                "Verkstead has no image of its own to equip a session with — the startup log \
                 says which of the two it is — so this session was not started"
            );
            return Ok(None);
        }

        // Decided here rather than read back off the agent afterwards, which is
        // the whole of why the log it writes can be found at all — see
        // [`session_name`].
        let session = session_name();

        // And the companion repos this Conversation was configured with, listed
        // under whatever prompt the caller built. Here rather than in each
        // builder because this is the one place every session is launched from
        // — the grilling one included, which is built nowhere near the rest —
        // so a prompt builder added later cannot forget it.
        let prompt = skills::alongside(prompt, &conversation.branch, &conversation.companions);

        // And, where the branch is still on the name Verkstead invented for it,
        // the instruction to pick a better one. Here for the reason the listing
        // above is here — it is not any one prompt's, and the three starts that
        // can be a Conversation's first session are built in three different
        // modules — and asked of the record rather than of the press, which is
        // what makes it the first session's alone: nothing sets that but the
        // work starting, and the first session to end puts it down. See
        // [`skills::naming`].
        let prompt = skills::naming(&prompt, conversation.naming);

        let conversation_id = conversation.id;

        // The sandbox asks git where the worktree's object database is, and the
        // dev-shell question is a `nix eval` or two. The line itself blocks on
        // the platform that writes the prompt to a file — see [`Agents::argv`].
        // All of it blocks, and all of it is decided before anything is spawned.
        let built = tokio::task::spawn_blocking({
            let agents = agents.clone();
            let conversation = conversation.clone();
            let pairing = pairing.clone();
            let session = session.clone();

            move || {
                let argv = agents.argv(
                    conversation_id,
                    &pairing,
                    &prompt,
                    session.as_deref(),
                    conversation.worktree.as_deref(),
                )?;

                agents.sandboxed(&conversation, &pairing.profile, &argv)
            }
        })
        .await?;

        let Some((sandbox, argv)) = built else {
            tracing::error!(
                conversation_id,
                "there is nothing to run a session on — no sandbox to run it in, or no \
                 prompt it could be started on — so none was started"
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

        // The moment the session started, taken before it starts. A backend
        // whose log is found rather than named is looking for a file that
        // appeared after this — and a moment read afterwards could be later
        // than the log it is meant to be earlier than, which would be a session
        // that never found its own record. See [`crate::transcript`].
        let at_launch = SystemTime::now();

        // And which backend this Conversation is launching on, put down before
        // there is a process that could ask anything. It is off the register
        // that a Set is read for how the session that sent it asks, and the
        // register does not learn of this one until its Capture is open — which
        // is a database write away, and a process that starts talking in the
        // meantime is a session asking as some other backend would. Held until
        // this returns, by which time the register has it or there is no session
        // to have. See [`Sessions::channel`].
        let _launching = self.launching(conversation_id, pairing.profile.agent_type());

        // `argv` inside the sandbox with nothing between the two, and the three
        // streams left to the terminal — which is the whole of what says a
        // session runs on one.
        //
        // And beside it what is left to see to once this session has gone: on
        // the platform that joins the account into a session's profile by hard
        // link, a file the session replaced rather than wrote in place. Held by
        // the relay from here, which is the thing that knows when the session is
        // over. See [`crate::sandbox::Closing`], which is nothing at all on
        // either Unix.
        let (command, afterwards) = sandbox.command(&argv);

        let child = match terminal.spawn(&command) {
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

        // And a keeper beside it where the platform's sandbox has nothing to
        // say about how long what it started lives — see
        // [`crate::sandbox::outliving`]. The terminal it was spawned on made it
        // a session of its own, so its pid is the process group everything it
        // goes on to start will be in. Nothing on Linux, where the flag it was
        // started with is what says this.
        if let Some(running) = child.id() {
            outliving::keep(Platform::HERE, running, std::process::id());
        }

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

        // The Event this session prints into, stamped as it opens with the name
        // Verkstead gave the session and with the Pairing it is being launched
        // under — see [`store::start_capture`]. Both are in hand exactly here
        // and nowhere afterwards.
        let event_id =
            store::start_capture(pool, conversation_id, session.as_deref(), Some(pairing)).await?;

        // The log the agent keeps of itself is followed inside the directory of
        // the Profile it is running under — under the name Verkstead gave the
        // session on a backend that takes one, and by the Worktree it opened in
        // and the moment it started on a backend that does not. A session with
        // no name has no log to look for — see [`crate::transcript`].
        let tail = session.as_deref().map(|session| {
            Tail::of(
                conversation_id,
                &pairing.profile,
                session,
                conversation.worktree.as_deref(),
                at_launch,
            )
        });

        // And the same output watched for the one thing a session says that is
        // about the account rather than about the work: that its window is
        // spent. The Profile is taken now because that is what the stop names,
        // and its agent type because that is what says which sentence to read
        // for — one per backend, as the idle signature is. A Profile renamed
        // while a session runs was not the account this one is on — see
        // [`crate::limits`].
        let limits = crate::limits::Watch::on(
            conversation_id,
            event_id,
            pairing.profile.name.clone(),
            pairing.profile.agent_type(),
        );

        let (stop, stopping) = oneshot::channel();

        // The two halves of what a driver holds a session by: the judgement the
        // relay keeps as it reads, and the word that the relay has finished. A
        // watch rather than a oneshot, because one session may be handed to more
        // than one driver over its life — see [`Sessions::following`].
        //
        // How this one is read is settled here and never again: it is the
        // backend's, and the backend is the Pairing's Profile.
        let idle = Idle::started(agents.judged(pairing.profile.agent_type()));
        let (over, ended) = watch::channel(None);

        // And the third: whether the process itself has gone. Set the moment the
        // relay reads end-of-file and long before `over` is sent — what is
        // between them is one last sweep of the branch, which may take a while
        // and which nothing typed into a terminal could reach. So this is what
        // says whether there is a session there to type into at all; see
        // [`Sessions::alive`] and [`crate::rescues`], which is what asks.
        let gone = Arc::new(AtomicBool::new(false));

        // What the session is about to commit, which is the other half of what
        // it leaves behind. Watched for as long as it runs and once more as it
        // ends — see [`crate::commits`].
        //
        // One branch per repository a commit can land in: the Conversation's
        // own, and one for each read-write companion it was configured with.
        let watched = crate::commits::watched(conversation);

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
                let idle = idle.clone();
                let gone = gone.clone();

                async move {
                    // One watcher per branch, each with the word that stops it.
                    // A record that cannot say what a branch was cut from
                    // contributes none — see [`crate::commits::watched`], which
                    // is also where a Conversation with no companions comes back
                    // as the one watcher it always had.
                    let watching: Vec<_> = watched
                        .into_iter()
                        .map(|branch| {
                            let (stop, stopping) = oneshot::channel();

                            let watcher = tokio::spawn(crate::commits::watch(
                                pool.clone(),
                                nudges.clone(),
                                conversation_id,
                                branch,
                                stopping,
                            ));

                            (stop, watcher)
                        })
                        .collect();

                    let ended = relay(
                        &pool,
                        &nudges,
                        Printing {
                            conversation_id,
                            event_id,
                        },
                        &mut launched,
                        &idle,
                        tail,
                        limits,
                        stopping,
                    )
                    .await;

                    // The process has gone, whatever is left to tidy up after
                    // it. Said here rather than with the rest of the tidying
                    // below, because what reads it is asking whether there is
                    // anything there to speak to — and from the moment the
                    // relay returns there is not, however long the sweep and
                    // the bookkeeping under it take.
                    gone.store(true, Ordering::Release);

                    // And the profile the session was given, seen to now that
                    // the process that had it has been reaped: a file it
                    // replaced rather than wrote in place goes back over the
                    // account's own, and the link is made fresh — see
                    // [`crate::sandbox::Closing`]. Before whoever is driving
                    // hears that the session is over, because the next thing
                    // that happens after that word is the next session being
                    // launched into the same profile. Off the runtime, being a
                    // file copy at worst; nothing at all on either Unix.
                    if let Err(error) =
                        tokio::task::spawn_blocking(move || afterwards.close()).await
                    {
                        tracing::error!(
                            error = ?error,
                            conversation_id,
                            "seeing to what a session wrote to its account ended badly"
                        );
                    }

                    // The session is over, so the branches are finished moving.
                    // Waited on rather than only asked to stop, because what
                    // each does when told is one last sweep: a session's final
                    // act is usually a commit, and it lands a poll after the
                    // process that made it has gone.
                    //
                    // Every one of them, because a session may have been working
                    // in more than one repository and the last commit in each is
                    // the one most worth catching. Told to stop first and awaited
                    // after, so the final sweeps run alongside each other rather
                    // than one repository at a time.
                    let (stops, watchers): (Vec<_>, Vec<_>) = watching.into_iter().unzip();

                    for stop in stops {
                        let _ = stop.send(());
                    }

                    for watcher in watchers {
                        if let Err(error) = watcher.await {
                            tracing::error!(error = ?error, conversation_id, "a branch watcher ended badly");
                        }
                    }

                    // And the branch name is the Conversation's from here,
                    // whatever this session did about it. After the sweeps
                    // rather than before them, because the last of those is
                    // what follows a rename the session made on its way out —
                    // and a settle written over a followed rename would say
                    // nothing anyway. Only the first session of a Conversation
                    // nobody named ever finds anything to put down; see
                    // [`store::settle_naming`].
                    if let Err(error) = store::settle_naming(&pool, conversation_id).await {
                        tracing::error!(error = ?error, conversation_id, "settling for a branch name failed");
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
                    idle: idle.clone(),
                    ended: ended.clone(),
                    gone,
                    agent_type: pairing.profile.agent_type(),
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
            idle,
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
/// `idle` is told about everything read rather than everything written down:
/// what it is judging is whether the session is still working, and a redraw the
/// summariser throws away is a session working.
///
/// The session's Screen is fed the same text, and immediately rather than every
/// [`FLUSH_EVERY`]. The store is a record and half a second is nothing to one;
/// the Screen is a terminal somebody may be watching, and half a second is a
/// long time to watch a terminal not move.
///
/// And the Screen is fed *before* the judgement is told, because on a backend
/// judged by what it draws the two are one act: the frame that says a session is
/// back at its prompt is drawn by the very text that arrived.
///
/// And `limits` is fed the same text a third time, watching for the one thing a
/// session says that is about the account rather than about the work — see
/// [`crate::limits`].
///
/// The one thing this loop announces that is not something it wrote down is the
/// session falling idle, and then waking: a page draws a session that has
/// stopped differently from one getting on with it, and going idle is
/// precisely when a session stops producing the Nudges that would carry the
/// news. So both crossings are announced on the Conversation's own kind, once
/// each — into idle when the judgement says so, and out of it on the first thing
/// read that says it is working again. The waking one is for the sidebar alone:
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
    idle: &Idle,
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

    // Whether the session has already been said to have stopped, so that it is
    // said once per silence rather than every time round the loop.
    let mut announced = false;

    loop {
        let deadline = tokio::time::Instant::from_std(flushed + FLUSH_EVERY);
        let following = tokio::time::Instant::from_std(tailed + FLUSH_EVERY);
        let idling = tokio::time::Instant::from_std(idle.crossing());

        tokio::select! {
            read = terminal.read(&mut buffer) => match read {
                // The far end of the terminal is closed, which is the session
                // gone.
                Ok(0) => break,
                Ok(taken) => {
                    let text = reading.take(&buffer[..taken]);

                    // The grid first and the judgement off it, which on a
                    // backend read by what it draws is one act — see [`Idle`].
                    screen.printed(&text);
                    idle.printed(screen);

                    // Coming back out of the silence is a crossing too, and the
                    // sidebar hears about it on nothing else: what a session
                    // prints is announced on the Screen's kind, which reaches
                    // the Conversation being watched rather than the list of
                    // them. Said once, on the way out — and only where what
                    // arrived was the session going back to work, because a TUI
                    // repainting the prompt it is sitting at has printed
                    // without waking.
                    if announced && !idle.idling() {
                        announced = false;

                        nudges.announce(Nudge::Conversation {
                            conversation: printing.conversation_id,
                        });
                    }

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
                    .look(
                        pool,
                        nudges,
                        &screen.drawn(),
                        tail.as_ref().and_then(Tail::latest),
                    )
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
                        .look(
                            pool,
                            nudges,
                            &screen.drawn(),
                            tail.as_ref().and_then(Tail::latest),
                        )
                        .await
                        && !ending
                    {
                        ending = true;
                        end_the_sandbox(child, event_id);
                    }
                }

                tailed = Instant::now();
            }
            _ = tokio::time::sleep_until(idling), if !announced => {
                announced = true;

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

    /// The Conversation every line below is built for.
    ///
    /// Any id does: what it names is the directory a prompt would be written
    /// into, and that is one platform's business — see [`Agents::argv`].
    const CONVERSATION: i64 = 7;

    impl Agents {
        /// [`Agents::argv`] for [`CONVERSATION`], with the line it always has.
        ///
        /// The `None` that method can answer is a prompt that could not be
        /// written down, which is a thing only the platform that writes one to
        /// a file can do. Every case here but that platform's own keeps its
        /// prompt on the command line, so every one of them has a line.
        fn launched(
            &self,
            pairing: &store::Pairing,
            prompt: &str,
            session: Option<&str>,
            worktree: Option<&Path>,
        ) -> Vec<String> {
            self.argv(CONVERSATION, pairing, prompt, session, worktree)
                .expect("a prompt that stays on the command line is always a line")
        }
    }

    fn profile() -> store::Profile {
        store::Profile {
            id: 1,
            name: "fable".to_owned(),
            account: store::Account::Claude {
                claude_dir: PathBuf::from("/srv/accounts/fable/.claude"),
                config_file: PathBuf::from("/srv/accounts/fable/.claude.json"),
            },
            models: vec!["claude-fable-5".to_owned(), "claude-opus-5".to_owned()],
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

    /// And the same on the second backend: one home, and the model its account
    /// can launch.
    fn codex_pairing() -> store::Pairing {
        store::Pairing {
            profile: store::Profile {
                id: 2,
                name: "work".to_owned(),
                account: store::Account::Codex {
                    home: PathBuf::from("/srv/accounts/work/.codex"),
                },
                models: vec!["gpt-5-codex".to_owned()],
            },
            model: Some("gpt-5-codex".to_owned()),
        }
    }

    /// And on the third, whose account is one home as the second's is.
    fn grok_pairing() -> store::Pairing {
        store::Pairing {
            profile: store::Profile {
                id: 3,
                name: "xai".to_owned(),
                account: store::Account::Grok {
                    home: PathBuf::from("/srv/accounts/work/.grok"),
                },
                models: vec!["grok-4.6".to_owned()],
            },
            model: Some("grok-4.6".to_owned()),
        }
    }

    /// And on the fourth, whose model is the `provider/model` string the human
    /// typed on the Profile and whose home is the directory opencode's XDG
    /// paths resolve inside.
    fn opencode_pairing() -> store::Pairing {
        store::Pairing {
            profile: store::Profile {
                id: 4,
                name: "zen".to_owned(),
                account: store::Account::OpenCode {
                    home: PathBuf::from("/srv/accounts/zen/opencode"),
                },
                models: vec!["opencode/big-pickle".to_owned()],
            },
            model: Some("opencode/big-pickle".to_owned()),
        }
    }

    /// The Worktree a session of either fixture is launched in, which is the
    /// directory codex is told to trust.
    ///
    /// **With a dot in it**, and that is the point: a Worktree is named for its
    /// Repo and the branch it holds, and either may carry one. Codex splits a
    /// `-c` key on every dot and does not stop at the quotes, so a path like
    /// this one is what tells a trust pre-seed that lands from one that leaves
    /// the session sitting on the trust prompt for ever.
    const WORKTREE: &str = "/srv/worktrees/verkstead-rate-limiting-v1.2";

    fn worktree() -> Option<&'static Path> {
        Some(Path::new(WORKTREE))
    }

    /// A server's own: the binary is the Profile's agent type's, with nothing
    /// standing where it goes.
    ///
    /// **Told which platform it is on rather than reading the runner's.** What
    /// nearly every test below asks about is the line a backend takes, and one
    /// of the three platforms does not put the prompt on that line at all — see
    /// [`Agents::argv`]. Read off the machine, the same assertions would be
    /// about two different lines depending on which runner ran them; the arm
    /// that writes the prompt down is asked for by name, by [`on`].
    fn real(state: &std::path::Path) -> Agents {
        Agents::new(
            Homes::on(Platform::Linux, PathBuf::from("/home/verkstead"), state),
            Reachable::at("127.0.0.1:8422".parse().unwrap()),
            SandboxConfig::default(),
            // What the argv is built from is not the sandbox, so this asks for
            // no cache at all rather than making one somewhere.
            BuildCache::none(),
            Skills::installed(Platform::Linux, state).expect("this binary carries skills"),
            // A test harness is its own executable, and what a sandbox does with
            // one is bind it: any file that is really there will do where nothing
            // here runs it.
            Executable::of_the_server(state),
            Handoffs::under(state),
            Settings::in_data_dir(state),
        )
    }

    /// And the same with `agent` standing where every type's binary goes.
    fn agents(agent: Vec<String>, state: &std::path::Path) -> Agents {
        Agents {
            agent: Some(agent),
            ..real(state)
        }
    }

    /// The prompt is what the grilling starts from, and an interactive claude
    /// takes what it is to start on as a positional argument.
    #[test]
    fn a_session_runs_the_pairings_model_on_the_prompt() {
        let state = tempfile::tempdir().unwrap();
        let argv = agents(vec!["claude".to_owned()], state.path()).launched(
            &pairing(),
            "# Rate limiting\n",
            None,
            worktree(),
        );

        assert_eq!(
            argv,
            vec![
                "claude".to_owned(),
                "--model".to_owned(),
                "claude-opus-5".to_owned(),
                "# Rate limiting\n".to_owned(),
                "--dangerously-skip-permissions".to_owned(),
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
        let argv = agents(vec!["claude".to_owned()], state.path()).launched(
            &unpaired,
            "# Rate limiting\n",
            None,
            worktree(),
        );

        assert_eq!(
            argv,
            vec![
                "claude".to_owned(),
                "--model".to_owned(),
                "claude-fable-5".to_owned(),
                "# Rate limiting\n".to_owned(),
                "--dangerously-skip-permissions".to_owned(),
            ]
        );
    }

    /// And a named session says so on the same line, after the prompt — see
    /// [`Agents::argv`] for why the end is where it goes.
    #[test]
    fn a_named_session_is_run_under_the_name_it_was_given() {
        let state = tempfile::tempdir().unwrap();
        let argv = agents(vec!["claude".to_owned()], state.path()).launched(
            &pairing(),
            "# Rate limiting\n",
            Some("d3b07384-d9a0-4c9b-8f2a-1b7c5e6f0a12"),
            worktree(),
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
                "--dangerously-skip-permissions".to_owned(),
            ]
        );
    }

    /// A Profile is launched on the binary its agent type names, so that a
    /// Codex Profile runs codex rather than whatever the first backend was.
    #[test]
    fn a_profile_is_run_on_its_own_agent_types_binary() {
        let state = tempfile::tempdir().unwrap();
        let agents = real(state.path());

        assert_eq!(
            agents
                .launched(&pairing(), "# Rate limiting\n", None, worktree())
                .first()
                .map(String::as_str),
            Some("claude")
        );
        assert_eq!(
            agents
                .launched(&codex_pairing(), "# Rate limiting\n", None, worktree())
                .first()
                .map(String::as_str),
            Some("codex")
        );
        assert_eq!(
            agents
                .launched(&grok_pairing(), "# Rate limiting\n", None, worktree())
                .first()
                .map(String::as_str),
            Some("grok")
        );
        assert_eq!(
            agents
                .launched(&opencode_pairing(), "# Rate limiting\n", None, worktree())
                .first()
                .map(String::as_str),
            Some("opencode")
        );
    }

    /// And the whole of codex's line: the model as `-m`, the prompt as the one
    /// positional, and everything the account and the sandbox need after it.
    ///
    /// No session id, because codex takes none — which is why its log is found
    /// rather than named. The credential store is file-backed because there is
    /// no keyring inside the sandbox, and the Worktree is trusted from the line
    /// rather than from anything written into the Profile's own directory —
    /// trusted as a whole table rather than as a key under one, which is what a
    /// Worktree with a dot in its name needs. See [`WORKTREE`].
    #[test]
    fn a_codex_session_takes_the_line_codex_takes() {
        let state = tempfile::tempdir().unwrap();
        let argv = agents(vec!["codex".to_owned()], state.path()).launched(
            &codex_pairing(),
            "# Rate limiting\n",
            Some("d3b07384-d9a0-4c9b-8f2a-1b7c5e6f0a12"),
            worktree(),
        );

        assert_eq!(
            argv,
            vec![
                "codex".to_owned(),
                "-m".to_owned(),
                "gpt-5-codex".to_owned(),
                "# Rate limiting\n".to_owned(),
                "--dangerously-bypass-approvals-and-sandbox".to_owned(),
                "--no-alt-screen".to_owned(),
                "-c".to_owned(),
                "cli_auth_credentials_store=\"file\"".to_owned(),
                "-c".to_owned(),
                format!("projects={{\"{WORKTREE}\"={{trust_level=\"trusted\"}}}}"),
            ]
        );
    }

    /// A Conversation with no worktree is one no session starts in, so what is
    /// left off is the trust of a directory there is none of — and the rest of
    /// the line, the account's own half included, stands.
    #[test]
    fn a_codex_session_with_no_worktree_trusts_nothing() {
        let state = tempfile::tempdir().unwrap();
        let argv = agents(vec!["codex".to_owned()], state.path()).launched(
            &codex_pairing(),
            "# Rate limiting\n",
            None,
            None,
        );

        assert!(
            !argv.iter().any(|arg| arg.contains("trust_level")),
            "there is no worktree to trust: {argv:?}"
        );
        assert!(
            argv.contains(&"cli_auth_credentials_store=\"file\"".to_owned()),
            "and the account still needs its credential store file-backed: {argv:?}"
        );
    }

    /// And the whole of grok's line: the model as `-m`, the prompt as the one
    /// positional, the session id it is named by after it, and the two bypasses
    /// and the inline screen last.
    ///
    /// The session id because grok is the one backend after Claude that takes
    /// one at launch — which is what makes its log named rather than found —
    /// and it takes it after the positional prompt, which is where this line
    /// builder puts it. Verified against grok 1.0.13, which parsed this line
    /// and got as far as wanting a terminal.
    #[test]
    fn a_grok_session_takes_the_line_grok_takes() {
        let state = tempfile::tempdir().unwrap();
        let argv = agents(vec!["grok".to_owned()], state.path()).launched(
            &grok_pairing(),
            "# Rate limiting\n",
            Some("d3b07384-d9a0-4c9b-8f2a-1b7c5e6f0a12"),
            worktree(),
        );

        assert_eq!(
            argv,
            vec![
                "grok".to_owned(),
                "-m".to_owned(),
                "grok-4.6".to_owned(),
                "# Rate limiting\n".to_owned(),
                "--session-id".to_owned(),
                "d3b07384-d9a0-4c9b-8f2a-1b7c5e6f0a12".to_owned(),
                "--always-approve".to_owned(),
                "--sandbox".to_owned(),
                "off".to_owned(),
                "--no-alt-screen".to_owned(),
            ]
        );
    }

    /// And the whole of opencode's line: the model as `-m provider/model`, the
    /// Brief under `--prompt` rather than as a positional, and `--auto` for the
    /// approvals.
    ///
    /// The flagged prompt is what makes this line a shape of its own — the
    /// positional opencode takes is the project to start in, not the thing to
    /// start on. No session id, because `--session` means *continue this one*
    /// and is validated against the store before the TUI starts, so a fresh
    /// name would be a session that never starts: opencode's log is found
    /// rather than named, as codex's is. Verified against opencode 1.18.25,
    /// which parsed this line on a pseudo-terminal, submitted the Brief without
    /// anything being typed, and answered it.
    #[test]
    fn an_opencode_session_takes_the_line_opencode_takes() {
        let state = tempfile::tempdir().unwrap();
        let argv = agents(vec!["opencode".to_owned()], state.path()).launched(
            &opencode_pairing(),
            "# Rate limiting\n",
            Some("d3b07384-d9a0-4c9b-8f2a-1b7c5e6f0a12"),
            worktree(),
        );

        assert_eq!(
            argv,
            vec![
                "opencode".to_owned(),
                "-m".to_owned(),
                "opencode/big-pickle".to_owned(),
                "--prompt".to_owned(),
                "# Rate limiting\n".to_owned(),
                "--auto".to_owned(),
            ]
        );
    }

    /// A server told it is running on `platform`, which is the whole of what
    /// decides whether a prompt goes on the command line or into a file.
    ///
    /// A value rather than a `cfg!`, so the runner this suite is on asks every
    /// arm — see [`Agents::argv`].
    fn on(platform: Platform, state: &std::path::Path) -> Agents {
        Agents {
            homes: Homes::on(platform, PathBuf::from("/home/verkstead"), state),
            ..agents(vec!["claude".to_owned()], state)
        }
    }

    /// Where the Conversation's prompt file is written, seen from outside the
    /// session — the handoff directory under the Data Directory.
    fn written_at(state: &std::path::Path) -> PathBuf {
        state
            .join("handoffs")
            .join(CONVERSATION.to_string())
            .join("prompt.md")
    }

    /// And where the session opens it: the same directory reached from inside,
    /// which is under the profile Windows gives a Conversation.
    ///
    /// Composed the way the server composes it rather than by joining the names
    /// again here. What a session is told is a path it can *open*, so the
    /// handoff directory inside a HOME is spelled with a forward slash whichever
    /// machine composed it — see [`crate::sandbox::under`] — and a `join` here
    /// would put this machine's own separator where that one is and match
    /// nothing.
    fn opened_at(state: &std::path::Path) -> PathBuf {
        crate::handoffs::inside(
            Platform::Windows,
            &state.join("homes").join(CONVERSATION.to_string()),
        )
        .join("prompt.md")
    }

    /// A prompt with everything a real one carries: what the builders above put
    /// under the Brief, rather than a line invented here.
    fn built_prompt() -> String {
        skills::naming("# Rate limiting\n", true)
    }

    /// Windows caps a command line at 32,767 characters, so a session there is
    /// started on one line naming a file rather than on the prompt itself — and
    /// the file holds the whole of what the builders produced.
    #[test]
    fn a_windows_session_is_started_on_a_line_naming_its_prompt() {
        let state = tempfile::tempdir().unwrap();
        let prompt = built_prompt();

        let argv = on(Platform::Windows, state.path())
            .argv(CONVERSATION, &pairing(), &prompt, None, worktree())
            .expect("a prompt that could be written down");

        assert_eq!(
            argv.len(),
            5,
            "the line is the backend's own, with one argument where the prompt was: {argv:?}",
        );
        assert!(
            !argv
                .iter()
                .any(|argument| argument.contains("Rate limiting")),
            "nothing of the prompt itself is on the line: {argv:?}",
        );

        let started_on = &argv[3];

        assert!(
            !started_on.contains('\n'),
            "what is there instead is one line: {started_on:?}",
        );
        assert!(
            started_on.contains(&opened_at(state.path()).display().to_string()),
            "and it names the file by the path the session opens it at: {started_on:?}",
        );

        assert_eq!(
            std::fs::read_to_string(written_at(state.path())).expect("a prompt file"),
            prompt,
            "and the file holds the whole prompt, the naming instruction included",
        );
    }

    /// And a prompt no Windows command line could have carried starts a session
    /// all the same, which is what writing it down is for. An implementing
    /// session's prompt carries the whole handoff document inlined, and a
    /// grilling that settled a lot settles more than 32,767 characters of it.
    #[test]
    fn a_prompt_too_long_for_a_command_line_starts_a_session_all_the_same() {
        /// What Windows will take, as `CreateProcessW` documents it.
        const LIMIT: usize = 32_767;

        let state = tempfile::tempdir().unwrap();
        let prompt = format!(
            "# What we settled\n\n{}",
            "A paragraph of the handoff document. ".repeat(1_000),
        );
        assert!(prompt.len() > LIMIT, "the prompt is longer than a line");

        let argv = on(Platform::Windows, state.path())
            .argv(CONVERSATION, &pairing(), &prompt, None, worktree())
            .expect("a prompt that could be written down");

        // The line as the machine measures it: every argument, and a space and
        // a pair of quotes around each.
        let measured: usize = argv.iter().map(|argument| argument.len() + 3).sum();

        assert!(
            measured < LIMIT,
            "the line is {measured} characters, which is one Windows would refuse: {argv:?}",
        );
        assert_eq!(
            std::fs::read_to_string(written_at(state.path())).expect("a prompt file"),
            prompt,
            "and the whole of it is in the file",
        );
    }

    /// The two platforms whose command line is long enough keep the prompt on
    /// it, byte for byte as they always have — and write nothing down.
    #[test]
    fn the_platforms_with_a_long_enough_line_keep_the_prompt_on_it() {
        for platform in [Platform::Linux, Platform::MacOs] {
            let state = tempfile::tempdir().unwrap();

            let argv = on(platform, state.path())
                .argv(
                    CONVERSATION,
                    &pairing(),
                    "# Rate limiting\n",
                    None,
                    worktree(),
                )
                .expect("a prompt that stays on the command line");

            assert_eq!(
                argv,
                vec![
                    "claude".to_owned(),
                    "--model".to_owned(),
                    "claude-opus-5".to_owned(),
                    "# Rate limiting\n".to_owned(),
                    "--dangerously-skip-permissions".to_owned(),
                ],
                "{platform:?} runs the prompt itself",
            );
            assert!(
                !state.path().join("handoffs").exists(),
                "and nothing was written down for it on {platform:?}",
            );
        }
    }

    /// What a stand-in agent does with such a line: read the arguments it was
    /// started with, find the one naming a file, and print what is in it.
    ///
    /// Which is the whole of what the stand-in agent in the Windows suite does
    /// with the same line, written here in the shell this suite's machine has.
    #[cfg(unix)]
    const STAND_IN: &str = r#"
for word in "$@"; do
    file=$(printf '%s' "$word" | sed -n 's/.*`\(.*\)`.*/\1/p')
    if [ -n "$file" ]; then
        cat "$file"
        exit 0
    fi
done
exit 1
"#;

    /// And the line works: an agent that reads it finds the file and gets the
    /// Brief out of it.
    ///
    /// The two ends of the path are one directory on Windows because the open
    /// rendering joins the handoff directory into the session's profile by a
    /// directory junction — see [`crate::sandbox::open`]. There are no
    /// junctions on the machine this runs on, so a symbolic link stands where
    /// one goes: what is being asked is whether the *line* names the file, and
    /// a link makes the path it names the file it wrote.
    #[cfg(unix)]
    #[test]
    fn a_stand_in_agent_reads_the_brief_out_of_the_file() {
        let state = tempfile::tempdir().unwrap();
        let prompt = built_prompt();

        let argv = on(Platform::Windows, state.path())
            .argv(CONVERSATION, &pairing(), &prompt, None, worktree())
            .expect("a prompt that could be written down");

        let opened = opened_at(state.path());
        std::fs::create_dir_all(opened.parent().unwrap().parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(
            written_at(state.path()).parent().unwrap(),
            opened.parent().unwrap(),
        )
        .unwrap();

        let read = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(STAND_IN)
            .args(&argv)
            .output()
            .expect("a shell to stand where the agent goes");

        assert!(
            read.status.success(),
            "the stand-in found no file to read in {argv:?}",
        );
        assert_eq!(
            String::from_utf8_lossy(&read.stdout),
            prompt,
            "and what it read is the Brief the session was started on",
        );
    }

    /// The stub the suite stands where an agent goes stands there for every
    /// type, and reads the line it reads today: the model first and the prompt
    /// after it, whichever backend's line that is.
    #[test]
    fn a_stub_stands_where_every_types_binary_goes() {
        let state = tempfile::tempdir().unwrap();
        let stub = vec!["/bin/sh".to_owned(), "-c".to_owned(), "printf x".to_owned()];
        let argv = agents(stub.clone(), state.path()).launched(
            &codex_pairing(),
            "# Rate limiting\n",
            None,
            worktree(),
        );

        assert_eq!(argv[..stub.len()], stub[..]);
        assert_eq!(argv[stub.len() + 1], "gpt-5-codex".to_owned());
        assert_eq!(argv[stub.len() + 2], "# Rate limiting\n".to_owned());
    }

    /// A name every backend that takes a session id will take, which is a
    /// version 4 UUID and nothing else — claude refuses a malformed one and so
    /// does grok, and the session then never starts. Fresh each time, which is
    /// grok's other condition: it refuses an id it already has a session for.
    #[test]
    fn a_session_is_named_something_a_backend_will_accept() {
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

    /// Every platform runs a session, and the one with no Sandbox of its own
    /// runs it outside one — which is Windows, until the Sandbox stage lands.
    ///
    /// Every arm asked on whichever machine is running this, which is the whole
    /// reason it is a function of the platform rather than a `cfg!`: what a
    /// Windows build answers is a thing the Linux runner can check.
    #[test]
    fn the_platform_without_a_sandbox_runs_sessions_outside_one() {
        assert!(
            unsandboxed_on(Platform::Windows),
            "Windows has a terminal now and no Sandbox yet",
        );
        assert!(!unsandboxed_on(Platform::Linux));
        assert!(!unsandboxed_on(Platform::MacOs));
    }

    /// And what a registry says about it: the served router's own is the
    /// platform's answer, and a test's is a build with a Sandbox unless it is
    /// stood up as the one without.
    #[test]
    fn a_registry_says_whether_a_session_here_is_sandboxed() {
        assert!(!Sessions::none().unsandboxed());
        assert!(Sessions::unsandboxed_here().unsandboxed());
        assert!(
            !Sessions::unsandboxed_here().runs_sessions(),
            "and a registry stood up for that question has no agents either way",
        );
    }
}
