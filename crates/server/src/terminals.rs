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
//! **Running the shell the machine's own human would get.** The server user's
//! login shell out of passwd, where that is a shell a Sandbox can run, and
//! `/bin/sh` where it is not — see [`shell`]. There is no setting for it: on a
//! packaged install the nix module gives the service user a shell, which is
//! where a machine's shells are said already.
//!
//! **A register of its own.** A Conversation has one session and may have any
//! number of terminals, so these are kept apart from the sessions' map rather
//! than bent into it, keyed by a number this server issues in order and never
//! reuses for that Conversation — a reload comes back to the number it left, and
//! a number that came round again would put it on somebody else's shell.
//!
//! **Four ways one ends, and no others.** The shell exits, which is the human
//! typing `exit`; the tab is closed, which is [`close`]; the Conversation
//! closes, which is [`Terminals::end_every`], before its Worktree is removed;
//! or the server stops, which nothing here does anything about — a terminal's
//! Sandbox is a `bwrap --die-with-parent` child like a session's, with the
//! keeper beside it on the platform whose sandbox has no such flag (see
//! [`outliving`]). No idle reaper and no ending on leaving the pane: the server
//! holds the terminal so that closing the pane, switching devices or losing a
//! connection loses nothing, and a reaper would take back with one hand what
//! that gives with the other.
//!
//! The three this module *does* do are one ending, in [`follow`]: the shell is
//! hung up, killed after [`LINGERING`] where it is still standing, and the
//! terminal comes off the register — after which every watcher's socket closes,
//! which is the one thing a tab is ever told about a terminal ending.
//!
//! **Not a record.** Nothing here writes to the store, puts anything on a
//! Timeline or reaches a Share: a session's bytes are kept because they are the
//! record of what an agent did, and a human's shell is the human doing
//! something. It is memory only, and it goes with the server.
//!
//! **And not a hold on the run**, exactly as typing into a Screen is not: a
//! terminal opened beside a running session is somebody looking, and somebody
//! who means to take the work on by hand presses **Stop** first.

/// Which shell a terminal comes up in: the server user's own, where the machine
/// has given it a usable one — see [`shell`], which is the whole of the
/// choosing.
pub mod shell;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as HttpResponse};
use tokio::sync::oneshot;
use verkstead_render::TerminalOpened;

use crate::AppState;
use crate::capture::Reading;
use crate::platform::Platform;
use crate::sandbox::{Closing, outliving};
use crate::screen::Live;
use crate::sessions::CHUNK;
use crate::store;
use crate::terminal::{Child, Terminal};

/// How that shell is started.
///
/// Interactive, because what is at the other end of it is a human at a keyboard.
/// And not a login shell, because a login shell reads the system's profile,
/// which rebuilds `PATH` — the Sandbox's invariant that the running server's own
/// `verkstead` is first on it has to hold in a terminal as it does in a session.
///
/// That is not the whole of holding it, and the rest is the Sandbox's: on NixOS
/// every shell rebuilds the environment as it starts whether it is a login shell
/// or not, and what stops it is said in the environment — see
/// [`crate::sandbox::Sandbox::shelled`].
const INTERACTIVE: &str = "-i";

/// How long a shell is given to go on its own after it has been hung up, before
/// it is killed where it stands.
///
/// A hangup is what a terminal being taken away sends and what a shell answers
/// by exiting, so what this is measuring is whatever it runs on the way out —
/// milliseconds, in every ordinary case. The deadline is there for the shell
/// that will not take it: a Conversation closing waits for its terminals before
/// the Worktree they are standing in is removed, and a wait with no end on it
/// would be a close nothing could ever finish.
const LINGERING: Duration = Duration::from_secs(2);

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

    /// The ones still running, by that number.
    live: HashMap<i64, Watched>,
}

/// One live terminal, as the register holds it: what it is drawing, and the two
/// words that end it.
///
/// The three travel together for the reason a session's [`Launched`] does: a
/// Screen nothing can end is a shell nobody can close, and a word to end one
/// with nothing to wait on would be a Conversation closing over a Worktree its
/// shell is still standing in.
///
/// [`Launched`]: crate::sessions
struct Watched {
    /// What it is drawing, for anybody who wants to watch — see [`Live`].
    screen: Live,

    /// Word to the relay that this terminal is to end, which it answers with a
    /// hangup and then a kill — see [`follow`].
    ///
    /// Dropped rather than sent where the shell ended by itself, and read as the
    /// same word either way: this is the register's own end of it, so a sender
    /// that has gone is a terminal nothing is holding any more.
    closing: oneshot::Sender<()>,

    /// And the relay's word back that it is over, which is its holding the other
    /// end of this for as long as it runs. Awaited rather than read: what it
    /// says is *when*, and there is nothing in it to say.
    ended: oneshot::Receiver<()>,
}

impl Watched {
    /// Tell the relay this terminal is to end, and hand back what says it has.
    ///
    /// Split from the waiting so that a Conversation with several can hang them
    /// all up and then wait once, rather than waiting out each in turn.
    fn hung_up(self) -> oneshot::Receiver<()> {
        // A relay that has already finished has dropped its end of this, which
        // is the same instruction arriving too late to be needed.
        let _ = self.closing.send(());

        self.ended
    }
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
        Some(
            self.held()
                .get(&conversation_id)?
                .live
                .get(&number)?
                .screen
                .clone(),
        )
    }

    /// Put one on the register under this Conversation's next number, and hand
    /// that number back.
    fn register(&self, conversation_id: i64, watched: Watched) -> i64 {
        let mut open = self.held();
        let held = open.entry(conversation_id).or_default();

        held.issued += 1;
        let number = held.issued;
        held.live.insert(number, watched);

        number
    }

    /// End one of them: the shell hung up and then killed where it lingers, and
    /// the terminal off the register.
    ///
    /// Waited for rather than asked, because both callers have something to do
    /// after it: a tab closing wants the socket under it closed, and a
    /// Conversation closing takes away the Worktree the shell was standing in.
    ///
    /// Nothing is refused for. A number nothing is holding is a terminal that
    /// has already ended, which is this asked for and already done.
    pub(crate) async fn end(&self, conversation_id: i64, number: i64) {
        let taken = self
            .held()
            .get_mut(&conversation_id)
            .and_then(|held| held.live.remove(&number));

        let Some(watched) = taken else {
            return;
        };

        let _ = watched.hung_up().await;

        tracing::info!(conversation_id, number, "a terminal was closed");
    }

    /// And every one this Conversation has, which is what its close does — see
    /// [`crate::conversations`], where it happens before the Worktree goes.
    ///
    /// Hung up together and waited for after, so a Conversation with several
    /// waits out one shell's going rather than each of them in turn. The count
    /// that names them stays where it is: a number is one shell's name for as
    /// long as this server is up, whatever became of the shell.
    pub(crate) async fn end_every(&self, conversation_id: i64) {
        let taken: Vec<(i64, Watched)> = match self.held().get_mut(&conversation_id) {
            Some(held) => held.live.drain().collect(),
            None => return,
        };

        let going: Vec<oneshot::Receiver<()>> = taken
            .into_iter()
            .map(|(number, watched)| {
                tracing::info!(
                    conversation_id,
                    number,
                    "a terminal is ending with its Conversation"
                );

                watched.hung_up()
            })
            .collect();

        for over in going {
            let _ = over.await;
        }
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
    // A build that runs no sessions runs no terminals either, and it says so in
    // front of everything else — the same rule, asked in the same place, that
    // refuses a session there.
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

        move || {
            // What this machine's own human gets at a keyboard, which is what a
            // terminal is for. Asked here rather than above because it is the
            // machine that answers — the passwd database and the filesystem —
            // and this is the thread that is allowed to wait on either.
            let chosen = shell::of_the_server();
            let argv = vec![chosen.clone(), INTERACTIVE.to_owned()];

            // Said twice: it is the command the Sandbox runs, and it is what
            // `SHELL` names inside — so what the human is typing into and what
            // anything they start reads out of the environment are one shell.
            agents
                .sandboxed(&conversation, &profile, &argv)
                .map(|(sandbox, argv)| (sandbox.shelled(&chosen), argv))
        }
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

    // And what is left to see to once this shell has gone, which a terminal
    // holds for the reason a session holds one: it runs in the same profile
    // under the same account, so a file it replaced rather than wrote in place
    // is a file the account should end up with. See [`Closing`], and
    // [`crate::sessions`], where the same value is held by the relay. Named for
    // what it is rather than for its type, `closing` in this module already
    // being the word down that a terminal is to end.
    let (command, afterwards) = sandbox.command(&argv);

    let child = match terminal.spawn(&command) {
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

    // The two words between the register and the relay: one down, saying this
    // terminal is to end, and one back, saying it has — see [`Watched`].
    let (closing, closed) = oneshot::channel();
    let (over, ended) = oneshot::channel();

    // On the register before the relay, so that a browser attaching with the
    // shell's first prompt has a Screen to attach to — and so that a shell that
    // dies on its first line is taken off a register it is already on, rather
    // than put on one it has just been taken off.
    let number = state.terminals.register(
        conversation_id,
        Watched {
            screen: screen.clone(),
            closing,
            ended,
        },
    );

    tokio::spawn(follow(
        state.terminals.clone(),
        conversation_id,
        number,
        terminal,
        child,
        afterwards,
        screen,
        closed,
        over,
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
///
/// **And this is where a terminal is ended from**, whoever asked for it: a tab
/// closed, or the Conversation closing around it. The word arrives on `closing`
/// and is answered the way a terminal going away is answered anywhere — the
/// shell hung up, and killed after [`LINGERING`] where it is still there. `over`
/// is the word back, and it is the holding of it rather than anything said down
/// it: whoever is waiting hears when this returns, which is after the shell has
/// been reaped.
///
/// **And `afterwards` is what the rendering left to see to**, held here for the
/// same reason the terminal is: it is asked once the shell has gone, and the
/// shell is what it is about — a file it replaced rather than wrote in place,
/// in the profile it shared with the Conversation's sessions. See [`Closing`].
#[expect(
    clippy::too_many_arguments,
    reason = "\
    one terminal's whole self: what it runs on, what runs on it, what it leaves \
    to see to, what it draws on, and a word each way about its ending"
)]
async fn follow(
    terminals: Terminals,
    conversation_id: i64,
    number: i64,
    terminal: Arc<Terminal>,
    mut child: Child,
    afterwards: Closing,
    screen: Live,
    mut closing: oneshot::Receiver<()>,
    over: oneshot::Sender<()>,
) {
    let mut reading = Reading::default();
    let mut buffer = vec![0u8; CHUNK];

    // Whether the shell has been hung up, and the moment it has to be gone by —
    // which is nothing at all until it has, the branch that reads it being off
    // until then.
    let mut hung_up = false;
    let mut going_by: Option<tokio::time::Instant> = None;

    loop {
        let killing = going_by.unwrap_or_else(|| tokio::time::Instant::now() + LINGERING);

        tokio::select! {
            read = terminal.read(&mut buffer) => match read {
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
            },

            // The word that this terminal is to end. A sender dropped rather
            // than sent says the same thing by the same hand: the register has
            // let go of this terminal, and nothing is coming back for it.
            _ = &mut closing, if !hung_up => {
                hung_up = true;
                going_by = Some(tokio::time::Instant::now() + LINGERING);

                hang_up(&child, conversation_id, number);
            }

            // And the shell that would not take it, which is a shell with
            // nothing left to be polite about.
            _ = tokio::time::sleep_until(killing), if going_by.is_some() => {
                going_by = None;

                tracing::warn!(
                    conversation_id,
                    number,
                    "a terminal's shell did not go when it was hung up, so it was killed"
                );

                if let Err(error) = child.start_kill() {
                    tracing::error!(
                        error = ?error,
                        conversation_id,
                        number,
                        "a terminal's shell would not be killed"
                    );
                }
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

    // And the profile that shell was in, seen to the way a session's is: a file
    // it replaced rather than wrote in place goes back over the account's own,
    // and the link is made fresh — see [`Closing`]. After the shell has been
    // reaped, because until then there is something that may still be writing
    // it. Off the runtime, being a file copy at worst; nothing at all on either
    // Unix.
    if let Err(error) = tokio::task::spawn_blocking(move || afterwards.close()).await {
        tracing::error!(
            error = ?error,
            conversation_id,
            number,
            "seeing to what a terminal wrote to its account ended badly"
        );
    }

    tracing::info!(conversation_id, number, "a terminal has ended");

    // And whoever asked for this to end hears that it has, which is this going
    // rather than anything said down it — see [`Watched::ended`]. Said here
    // rather than left to the end of the function so that what it promises is
    // plain: the shell has been reaped by the time it goes.
    drop(over);
}

/// Hang the shell up: the signal a terminal being taken away sends, and the one
/// a shell answers by exiting.
///
/// To the whole process group rather than to the process the Sandbox was started
/// as, so that the shell hears it rather than only what is wrapped around it.
/// The group is this terminal's and nobody else's: what was started on it took a
/// session of its own before it ran anything — see
/// [`crate::terminal::Terminal::spawn`] — so it leads a group holding the
/// sandbox, the shell and whatever the shell started.
///
/// Nothing is refused for. A child with no id has been reaped already, and a
/// group nothing is left in is a shell that has gone by itself — both of which
/// are this arriving too late to be needed, which is the same as it having
/// worked.
#[cfg(unix)]
fn hang_up(child: &Child, conversation_id: i64, number: i64) {
    let running = child
        .id()
        .and_then(|running| i32::try_from(running).ok())
        .and_then(rustix::process::Pid::from_raw);

    let Some(running) = running else {
        return;
    };

    if let Err(error) = rustix::process::kill_process_group(running, rustix::process::Signal::HUP) {
        tracing::debug!(
            %error,
            conversation_id,
            number,
            "a terminal's shell could not be hung up, so it is waited out instead"
        );
    }
}

/// And where there are no process groups to hang up — which is Windows, where
/// what a shell started is held by a Job instead — see [`crate::terminal`].
///
/// The kill after [`LINGERING`] is the whole of the ending there, which is what
/// a session gets on every platform.
#[cfg(not(unix))]
fn hang_up(_child: &Child, _conversation_id: i64, _number: i64) {}

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

/// `DELETE /api/ui/conversations/{id}/terminals/{n}` — close one, which is what
/// the row on a tab's menu asks for.
///
/// The shell is hung up and then killed where it lingers, and the terminal comes
/// off the register — after which every watcher's socket closes under them,
/// exactly as a shell that exited by itself closes them. Which is how the tab
/// goes: the pane hears one thing about a terminal ending, whichever end asked
/// for it.
///
/// Answered once it has, rather than once it has been asked for, so that a
/// Conversation closed the moment after a tab was is a Worktree with nothing
/// standing in it.
///
/// Nothing to answer with and nothing to be refused for. A number nothing is
/// holding is a shell that has already ended, which is a close that has already
/// happened — and a Conversation this server never opened one for is the same
/// answer for the same reason.
pub(crate) async fn close(
    State(state): State<AppState>,
    Path((id, number)): Path<(String, String)>,
) -> HttpResponse {
    // Read as permissively as the attach above, and for the same reason: a pair
    // of ids that name no numbers name no terminal.
    let (Ok(id), Ok(number)) = (id.parse::<i64>(), number.parse::<i64>()) else {
        return StatusCode::NO_CONTENT.into_response();
    };

    state.terminals.end(id, number).await;

    StatusCode::NO_CONTENT.into_response()
}
