//! The terminals a Conversation holds of its own: a human's shell inside its
//! Sandbox, with its Worktree as the working directory.
//!
//! For the moment the agent's work is done and somebody wants to try it, make a
//! small change or work with git — without leaving the workbench, and without
//! the run noticing
//! ([ADR 0013](../../../docs/adr/0013-conversation-terminals.md)).
//!
//! **The Screen's own machinery pointed at a shell.** A terminal here is the
//! same pseudo-terminal Verkstead opens for a session, feeding the same
//! server-held virtual terminal, watched over the same socket by the same xterm
//! in the browser — see [`crate::terminal`] and [`crate::screen`]. What runs on
//! it is a shell rather than an agent, and that is the whole of the difference
//! at this layer.
//!
//! **Inside the Sandbox, with everything a session gets.** It is built by
//! [`Agents::sandboxed`], which is the one builder both come through: the
//! Worktree, the git directory, the handoff directory, the build cache, the
//! Sandbox Configuration binds, the GitHub token, the git author and a
//! `VERKSTEAD_SERVER` scoped to this Conversation, wrapped in the worktree's dev
//! shell where its flake has one. Under the implementation Pairing's Profile —
//! the grilling Pairing's where the implementation role has none — because a
//! terminal has no role of its own and that is the account the work is done
//! under. Running one outside the Sandbox was never on the table: the filesystem
//! boundary is what makes a shell in a Conversation safe to offer at all.
//!
//! **A register of its own.** A Conversation has one session and may have any
//! number of terminals, so these are kept apart from the sessions' map rather
//! than bent into it, keyed by a number this server issues in order and never
//! reuses for that Conversation — a reload comes back to the number it left, and
//! a number that came round again would put it on somebody else's shell.
//!
//! **Not a record.** Nothing here writes to the store, puts anything on a
//! Timeline or reaches a Share: a session's bytes are kept because they are the
//! record of what an agent did, and a human's shell is the human doing
//! something. It is memory only, and it goes with the server.
//!
//! **And not a hold on the run**, exactly as typing into a Screen is not: a
//! terminal opened beside a running session is somebody looking, and somebody
//! who means to take the work on by hand presses **Stop** first.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response as HttpResponse;
use tokio::process::{Child, Command};
use verkstead_render::TerminalOpened;

use crate::AppState;
use crate::capture::Reading;
use crate::platform::Platform;
use crate::sandbox::outliving;
use crate::screen::Live;
use crate::sessions::CHUNK;
use crate::store;
use crate::terminal::Terminal;

/// What a Terminal runs.
///
/// The shell every machine Verkstead runs on has, at the one path the Sandbox's
/// own surface is certain to have one at, and interactive because what is at the
/// other end of it is a human at a keyboard. Not a login shell: one reads the
/// system profile, which rebuilds `PATH`, and the Sandbox's invariant that the
/// running server's own `verkstead` is first on it has to hold here too.
///
/// The server user's own login shell is what a human should get, and a later
/// task reads it out of passwd; this is what stands there until it does, and
/// what a machine with no usable one falls back to either way.
const SHELL: &str = "/bin/sh";

/// The terminals this server is holding, by the Conversation each belongs to.
#[derive(Clone, Default)]
pub(crate) struct Terminals {
    open: Arc<Mutex<HashMap<i64, Held>>>,
}

/// One Conversation's terminals, and the count that names them.
#[derive(Default)]
struct Held {
    /// How many have been opened here, which is what the next one is called.
    ///
    /// Counting up and never going back, so a number is one shell's name for as
    /// long as this server is up: a tab reattaches by number after a reload, and
    /// a number handed out twice would reattach it to a stranger.
    issued: i64,

    /// The ones still running, by that number — each a Screen over the shell's
    /// own pseudo-terminal, which is the whole of what anything outside here
    /// needs of one.
    live: HashMap<i64, Live>,
}

impl Terminals {
    /// A server holding none, which is every server as it starts: a terminal is
    /// memory only and nothing survives a restart.
    pub(crate) fn new() -> Terminals {
        Terminals::default()
    }

    /// The numbers of the terminals still live on this Conversation, oldest
    /// first — which is the order they were opened in.
    pub(crate) fn live(&self, conversation_id: i64) -> Vec<i64> {
        let open = self.held();

        let Some(held) = open.get(&conversation_id) else {
            return Vec::new();
        };

        let mut live: Vec<i64> = held.live.keys().copied().collect();
        live.sort_unstable();
        live
    }

    /// The Screen of one of them, or `None` where it is not live.
    ///
    /// What [`crate::screen::follow`] asks for every message it carries, which
    /// is why it is a lookup rather than a handle: a shell exits while somebody
    /// is watching, and the register is what knows.
    pub(crate) fn screen(&self, conversation_id: i64, number: i64) -> Option<Live> {
        self.held()
            .get(&conversation_id)?
            .live
            .get(&number)
            .cloned()
    }

    /// Put one on the register under this Conversation's next number, and hand
    /// that number back.
    fn register(&self, conversation_id: i64, screen: Live) -> i64 {
        let mut open = self.held();
        let held = open.entry(conversation_id).or_default();

        held.issued += 1;
        let number = held.issued;
        held.live.insert(number, screen);

        number
    }

    /// And take it off, its shell having ended.
    ///
    /// The Conversation's own entry goes with the last of them, but its count
    /// does not come back: a Conversation whose terminals have all ended goes on
    /// issuing numbers from where it left off, because the tabs that held those
    /// numbers may still be open.
    fn forget(&self, conversation_id: i64, number: i64) {
        let mut open = self.held();

        let Some(held) = open.get_mut(&conversation_id) else {
            return;
        };

        held.live.remove(&number);
    }

    /// The register, locked.
    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<i64, Held>> {
        self.open
            .lock()
            .expect("the terminals register is not poisoned")
    }
}

/// Open a terminal on `conversation`: a shell running in its Sandbox, with its
/// Worktree as the working directory.
///
/// The Sandbox is built and the flake is asked about on a blocking thread, the
/// way a session's is; the pseudo-terminal is opened here, the shell is started
/// on it, and a relay follows what it prints until it exits.
///
/// What comes back is the number the terminal answers to, or the named reason
/// there is none — see [`TerminalOpened`], whose refusals are the ones a
/// session's start refuses by, asked about a shell.
pub(crate) async fn open(state: &AppState, conversation_id: i64) -> anyhow::Result<TerminalOpened> {
    // A build with no pseudo-terminal to open has nothing to run a shell on, and
    // it says so in front of everything else — the same rule, asked in the same
    // place, that refuses a session there.
    if state.sessions.here().absent() {
        return Ok(TerminalOpened::NotOnWindowsYet);
    }

    let Some(agents) = state.sessions.agents() else {
        tracing::warn!(
            conversation_id,
            "this server has no way to run anything in a Sandbox, so no terminal was opened"
        );
        return Ok(TerminalOpened::Refused);
    };

    let Some(conversation) = store::load_conversation(&state.pool, conversation_id).await? else {
        return Ok(TerminalOpened::NoSuchConversation);
    };

    if conversation.worktree.is_none() {
        return Ok(TerminalOpened::NoWorktree);
    }

    // A terminal has no role of its own: it is the human working where the agent
    // worked, so it runs under the account the work is done under. The grilling
    // Pairing's stands in where the implementation role has none, which is the
    // only way a Conversation with a Worktree can have got this far.
    let paired = conversation
        .implementation_pairing
        .clone()
        .or_else(|| conversation.grilling_pairing.pairing().cloned());

    let Some(pairing) = paired else {
        return Ok(TerminalOpened::NoProfile);
    };

    let built = tokio::task::spawn_blocking({
        let agents = agents.clone();
        let conversation = conversation.clone();
        let profile = pairing.profile.clone();
        let argv = vec![SHELL.to_owned(), "-i".to_owned()];

        move || agents.sandboxed(&conversation, &profile, &argv)
    })
    .await?;

    let Some((sandbox, argv)) = built else {
        tracing::error!(
            conversation_id,
            "there is no sandbox to open a terminal in, so none was opened"
        );
        return Ok(TerminalOpened::Refused);
    };

    // The terminal before the shell, because the shell is started *on* it.
    let mut terminal = match Terminal::open() {
        Ok(terminal) => terminal,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "a terminal's pseudo-terminal could not be opened, so none was opened"
            );
            return Ok(TerminalOpened::Refused);
        }
    };

    let mut command = Command::from(sandbox.command(&argv));

    // A shell left behind by a panicking task is one nothing would ever reap.
    command.kill_on_drop(true);

    let child = match terminal.spawn(&mut command) {
        Ok(child) => child,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "a terminal's shell could not be started"
            );
            return Ok(TerminalOpened::Refused);
        }
    };

    // And a keeper beside it where the platform's sandbox has nothing to say
    // about how long what it started lives — see [`crate::sandbox::outliving`],
    // which is the same thing a session gets and for the same reason.
    if let Some(running) = child.id() {
        outliving::keep(Platform::HERE, running, std::process::id());
    }

    let terminal = Arc::new(terminal);
    let screen = Live::on(terminal.clone());

    // On the register before the relay, so that a browser attaching with the
    // shell's first prompt has a Screen to attach to.
    let number = state.terminals.register(conversation_id, screen.clone());

    tokio::spawn(follow(
        state.terminals.clone(),
        conversation_id,
        number,
        terminal,
        child,
        screen,
    ));

    tracing::info!(conversation_id, number, "a terminal is running");

    Ok(TerminalOpened::Opened { number })
}

/// Follow one terminal's shell until it exits, putting what it prints on its
/// Screen as it arrives.
///
/// Straight onto the Screen and nowhere else: there is no Capture to write, no
/// Timeline to nudge and no summary to keep, which is the whole of what *not a
/// record* costs here.
///
/// The terminal is held for as long as this runs, because the last thing a shell
/// says is said on its way out.
async fn follow(
    terminals: Terminals,
    conversation_id: i64,
    number: i64,
    terminal: Arc<Terminal>,
    mut child: Child,
    screen: Live,
) {
    let mut reading = Reading::default();
    let mut buffer = vec![0u8; CHUNK];

    loop {
        match terminal.read(&mut buffer).await {
            // The far end of the terminal is closed, which is the shell gone.
            Ok(0) => break,
            Ok(taken) => screen.printed(&reading.take(&buffer[..taken])),
            Err(error) => {
                tracing::error!(
                    error = ?error,
                    conversation_id,
                    number,
                    "reading a terminal's output failed"
                );
                break;
            }
        }
    }

    // Whatever was left of a character that never finished arriving, the shell
    // having gone.
    screen.printed(&reading.finish());

    // Off the register before it is reaped, so that a page reading the list back
    // reads a terminal that has ended — and so that every watcher hears the
    // channel close rather than waiting on a Screen nothing will feed again.
    terminals.forget(conversation_id, number);

    if let Err(error) = child.wait().await {
        tracing::error!(
            error = ?error,
            conversation_id,
            number,
            "a terminal's shell could not be reaped"
        );
    }

    tracing::info!(conversation_id, number, "a terminal has ended");
}

/// `GET /api/ui/conversations/{id}/terminals/{n}/attach` — one of a
/// Conversation's terminals, watched as it is drawn.
///
/// The Screen's own socket pointed at a shell: a repaint on connect, what the
/// shell prints after it, and a resize or what was typed coming back up — see
/// [`crate::screen::follow`], which is the whole of both directions.
///
/// A number that is not live is refused as a session's Screen is: there is
/// nothing to relay, and no read-only grid to fall back to either, a terminal
/// being memory only.
pub(crate) async fn attach(
    State(state): State<AppState>,
    Path((id, number)): Path<(String, String)>,
    watcher: WebSocketUpgrade,
) -> HttpResponse {
    // Read as permissively as every other pair of ids here: neither of them
    // naming a number cannot name a terminal.
    let (Ok(id), Ok(number)) = (id.parse::<i64>(), number.parse::<i64>()) else {
        return crate::ui::no_such_terminal();
    };

    if state.terminals.screen(id, number).is_none() {
        return crate::ui::no_such_terminal();
    }

    watcher.on_upgrade(move |socket: WebSocket| {
        crate::screen::follow(
            socket,
            move || state.terminals.screen(id, number),
            format!("conversation {id} terminal #{number}"),
        )
    })
}
