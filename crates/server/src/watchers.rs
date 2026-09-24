//! Following the disk: the watcher a Code pane's attachment runs, and the
//! `files` Nudge it announces the Worktree moving as.
//!
//! The disk moves under the pane — the agent writes, a build runs, a terminal
//! tab checks out a branch — and nothing in the store hears any of it
//! ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *Following the disk*).
//! So the Worktrees are watched, and what a watcher has to say is one kind of
//! Nudge: [`Nudge::Files`], carrying the Conversation and nothing else
//! (ADR 0009). The page re-reads the folders it has expanded and the files it
//! has open, and a re-read is cheap because a folder is one listing and a
//! version is one hash.
//!
//! **On demand, which is what the socket is for.** A watcher costs a handle per
//! directory on every platform and an inotify watch per directory on Linux, so
//! one per Conversation whether or not anybody is looking would be a machine's
//! worth of them spent on Conversations nobody has opened. It runs while a Code
//! pane is *attached* and not otherwise — and how a pane says it is attached is
//! a socket it holds open for as long as it is drawn, the way a terminal tab
//! holds its attach: it dies with the tab whatever becomes of the browser, so a
//! laptop shut mid-edit stops the watcher without anybody having to notice.
//! A request renewed while the pane is open was the other shape, and this one
//! needs no clock and no renewal.
//!
//! **A register beside the terminals', for the terminals' reason.** A
//! Conversation may have any number of panes attached — two windows, a laptop
//! and a phone — and the first one starts its watcher and the last one going
//! stops it.
//!
//! **And a swap is not a detach.** The details pane is taken down whenever
//! something else is drawn in it, so opening an Event and coming straight back
//! closes the socket and opens another one a moment later. Tearing the watcher
//! down and building it again for that would be a walk of the Worktree per
//! press, so the stop waits [`GRACE`] out and then asks again whether anybody
//! is attached — see [`Watchers::let_go`].
//!
//! **One Nudge per burst.** A `cargo build` writes thousands of files and the
//! page wants to look once, so what the watcher says is debounced: the first
//! event opens a burst, [`QUIET`] with nothing further closes it, and
//! [`AT_MOST`] closes one that will not go quiet, so a page in front of a long
//! build is kept up to date rather than left until it ends.
//!
//! **Each root's own directory and nothing below it, for now.** What makes this
//! watch the whole Worktree — the ignore-aware walk, and directories added as
//! they appear — is the stage's next task; a recursive watch over a Rust
//! `target/` would exhaust a machine's inotify watches on its own, which is why
//! it is a walk rather than a flag.
//!
//! **Not a record.** Nothing here writes to the store, puts anything on a
//! Timeline or reaches a Share: it is the disk being read, and the Nudge is the
//! word that a read is worth making. It is memory only, and it goes with the
//! server.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response as HttpResponse;
use notify::{RecursiveMode, Watcher};
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use verkstead_schema::Nudge;

use crate::AppState;
use crate::nudge::Nudges;
use crate::store;

/// How long the last pane going is given to come back before its watcher is
/// stopped.
///
/// The details pane is one place and every pane of the workbench is drawn in
/// it, so a human opening an Event off the Timeline and pressing back is two
/// attachments with a gap between them — and that gap is a repaint rather than
/// anything the human would call leaving. Long enough to cover it and short
/// enough that a pane genuinely closed stops its watcher while somebody is
/// still looking at the tab it was in.
const GRACE: Duration = Duration::from_secs(2);

/// How long a burst has to be over for before it is announced.
///
/// What a burst is here is anything that writes more than one file, which is
/// most of what writes any: a `git checkout`, an agent's edit and its formatter
/// after it, a build. A quarter-second of quiet is past the end of every one of
/// those and is nothing a human would notice waiting.
const QUIET: Duration = Duration::from_millis(250);

/// And how long a burst that will not go quiet is allowed to run before it is
/// announced anyway.
///
/// [`QUIET`] alone would leave a page in front of a three-minute build showing
/// the tree as it stood when the build began, because the quiet it is waiting
/// for never comes. This is the other end of that: a build still running is
/// announced every couple of seconds, which is a handful of Nudges for the
/// thousands of files it wrote rather than one at the very end.
const AT_MOST: Duration = Duration::from_secs(2);

/// The Code panes attached to each Conversation, and the watcher the first of
/// them started.
#[derive(Clone, Default)]
pub(crate) struct Watchers {
    held: Arc<Mutex<HashMap<i64, Following>>>,
}

/// One Conversation's attachments, and what is watching its Worktrees for them.
struct Following {
    /// How many Code panes are holding a socket open on it.
    ///
    /// A count rather than a set, there being nothing to tell two panes apart
    /// by and nothing that would want to: what the watcher turns on is whether
    /// anybody at all is drawing this Conversation's files.
    panes: usize,

    /// Word to that watcher that it is over, which is [`Watchers::let_go`]
    /// letting go of this entry.
    ///
    /// Never sent — dropping it says the same thing by the same hand, and is
    /// the one thing that cannot be forgotten when the entry goes. The
    /// terminals' register ends a shell the same way.
    _closing: oneshot::Sender<()>,
}

impl Watchers {
    /// A server watching nothing, which is every server as it starts: a watcher
    /// is memory only and nothing survives a restart.
    pub(crate) fn new() -> Watchers {
        Watchers::default()
    }

    /// Take one pane's attachment to `conversation_id`, starting a watcher over
    /// `roots` where this is the first, and hand back what holds it.
    ///
    /// The roots are the caller's because reading them is a question for the
    /// store and this is not the place to ask one — see [`attach`], which is
    /// where a socket comes in. They are read once, by the attachment that
    /// starts the watcher: a companion added to a Conversation whose pane is
    /// already open is not in this reading, and is a `Repos` Nudge and a pane
    /// that re-reads its roots either way.
    fn attach(&self, conversation_id: i64, roots: Vec<PathBuf>, nudges: Nudges) -> Attachment {
        let mut held = self.held();

        match held.get_mut(&conversation_id) {
            Some(following) => following.panes += 1,
            None => {
                let (closing, closed) = oneshot::channel();

                tracing::debug!(
                    conversation_id,
                    roots = roots.len(),
                    "a Code pane attached, so its Worktrees are being watched"
                );

                tokio::spawn(following(conversation_id, roots, nudges, closed));

                held.insert(
                    conversation_id,
                    Following {
                        panes: 1,
                        _closing: closing,
                    },
                );
            }
        }

        Attachment {
            watchers: self.clone(),
            conversation_id,
        }
    }

    /// One pane's attachment let go of, which is its socket closing.
    ///
    /// The watcher is left running where another pane is still attached, and
    /// where none is it is given [`GRACE`] before it is asked again: a details
    /// pane swapped for an Event and back is a socket closed and another opened
    /// a moment later, and that is a repaint rather than a detach.
    fn let_go(&self, conversation_id: i64) {
        {
            let mut held = self.held();

            let Some(following) = held.get_mut(&conversation_id) else {
                return;
            };

            following.panes = following.panes.saturating_sub(1);

            if following.panes > 0 {
                return;
            }
        }

        // A runtime that has gone is the server stopping, which takes every
        // watcher with it: the grace is asked for on the runtime, and this is
        // reached from a socket's own ending, so the one arrangement where
        // there is none is the process on its way out.
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };

        let watchers = self.clone();

        runtime.spawn(async move {
            tokio::time::sleep(GRACE).await;
            watchers.stop_where_nobody_came_back(conversation_id);
        });
    }

    /// And the grace run out: the watcher stops unless something attached while
    /// it was running.
    ///
    /// Nothing has to be said to it — taking the entry away drops the word that
    /// ends it, which is how the register lets go of anything.
    ///
    /// No generation to check. Two graces may be in flight over one
    /// Conversation, a pane having attached and left again while the first was
    /// running, and the earlier one firing does no harm: what it reads is
    /// whether anybody is attached *now*, and where nobody is, stopping is what
    /// the later grace was going to do anyway.
    fn stop_where_nobody_came_back(&self, conversation_id: i64) {
        let mut held = self.held();

        if held
            .get(&conversation_id)
            .is_none_or(|following| following.panes > 0)
        {
            return;
        }

        held.remove(&conversation_id);

        tracing::debug!(
            conversation_id,
            "the last Code pane on a Conversation closed, so its Worktrees are no longer watched"
        );
    }

    /// The register, locked.
    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<i64, Following>> {
        self.held
            .lock()
            .expect("the watchers register is not poisoned")
    }
}

/// One pane's attachment, held for as long as its socket is.
///
/// A guard rather than a pair of calls, so that every way a socket can end —
/// the tab closed, the laptop shut, the connection dropped, the task
/// cancelled — lets go of exactly one attachment, and none of them has to
/// remember to.
struct Attachment {
    watchers: Watchers,
    conversation_id: i64,
}

impl Drop for Attachment {
    fn drop(&mut self) {
        self.watchers.let_go(self.conversation_id);
    }
}

/// Watch `roots` until the word comes back that nobody is attached any more,
/// saying what moved as one [`Nudge::Files`] per burst.
///
/// The watcher itself is the platform's own — inotify, FSEvents,
/// `ReadDirectoryChangesW` — and it calls back on a thread of its own, so what
/// it says is put on a channel and read here: the debounce is a clock, and a
/// clock belongs on the runtime rather than on somebody else's thread.
///
/// **A root that will not be watched is logged and left out.** A Conversation
/// before grilling or after closing has no Worktree at all, and one whose
/// checkout has been removed under it has a path that is not there — neither is
/// a reason to refuse the attachment, and what either costs is a pane that
/// hears nothing about a root there is nothing to hear about.
async fn following(
    conversation_id: i64,
    roots: Vec<PathBuf>,
    nudges: Nudges,
    mut closing: oneshot::Receiver<()>,
) {
    let (moved, mut moves) = mpsc::unbounded_channel();

    let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Err(error) = &event {
            tracing::debug!(%error, conversation_id, "watching a Worktree had trouble");
        }

        // Said whichever it was. An error here is most often a queue that
        // overflowed, which is a burst this watcher saw the middle of none of —
        // exactly the moment a page should look again rather than the one
        // moment it should not.
        let _ = moved.send(());
    });

    let mut watcher = match watcher {
        Ok(watcher) => watcher,
        Err(error) => {
            tracing::error!(
                %error,
                conversation_id,
                "this machine would not give Verkstead a filesystem watcher, so the Code pane \
                 will not follow the disk"
            );
            return;
        }
    };

    for root in &roots {
        // Each root's own directory and nothing below it, which is the whole of
        // the watch until the walk lands — see the module note.
        if let Err(error) = watcher.watch(root, RecursiveMode::NonRecursive) {
            tracing::info!(
                %error,
                conversation_id,
                root = %root.display(),
                "a Worktree is not being watched"
            );
        }
    }

    // When the burst that is running is to be announced, where one is: the
    // quiet it is waiting for, and the ceiling it is announced at regardless.
    let mut quiet: Option<Instant> = None;
    let mut ceiling: Option<Instant> = None;

    loop {
        let announcing = match (quiet, ceiling) {
            (Some(quiet), Some(ceiling)) => Some(quiet.min(ceiling)),
            _ => None,
        };

        tokio::select! {
            // Nobody is attached any more. A sender dropped rather than sent
            // says it by the same hand: the register has let go of this
            // Conversation, and nothing is coming back for it.
            _ = &mut closing => break,

            said = moves.recv() => {
                if said.is_none() {
                    break;
                }

                let now = Instant::now();
                quiet = Some(now + QUIET);
                ceiling.get_or_insert(now + AT_MOST);
            }

            // The deadline stands in for itself while no burst is running: a
            // disabled branch still has its future built, and is only never
            // polled — so what is wanted here is a value rather than a panic
            // about one.
            _ = tokio::time::sleep_until(announcing.unwrap_or_else(Instant::now)),
                if announcing.is_some() => {
                quiet = None;
                ceiling = None;

                nudges.announce(Nudge::Files { conversation: conversation_id });
            }
        }
    }

    // Off the runtime, the watcher's own thread being joined as it is dropped.
    if let Err(error) = tokio::task::spawn_blocking(move || drop(watcher)).await {
        tracing::error!(%error, conversation_id, "letting a Worktree watcher go ended badly");
    }
}

/// `GET /api/ui/conversations/{id}/files/attach` — a Code pane saying it is
/// drawn, and holding the socket open for as long as it is.
///
/// Nothing travels either way. What the pane hears about the disk comes down
/// the Nudge stream like every other change, and what this socket is for is
/// being *open*: the first one on a Conversation starts the watcher behind that
/// Nudge and the last one closed stops it. It is read all the same, because a
/// socket nobody reads is one whose closing nothing notices.
///
/// A Conversation this server has no record of is refused, the way a terminal's
/// socket refuses a number that is not live. One with no Worktree is not: a
/// Conversation before grilling or after closing has no roots, so the pane
/// attaches and the watcher watches nothing — which is the same answer a
/// checkout removed under an open pane gets, and there is nothing for a human
/// to do about either.
pub(crate) async fn attach(
    State(state): State<AppState>,
    Path(id): Path<String>,
    pane: WebSocketUpgrade,
) -> HttpResponse {
    // Read as permissively as the terminals' own socket: an id that is not a
    // number cannot name a Conversation, and the id comes out of a URL the
    // human may have typed.
    let Ok(id) = id.parse::<i64>() else {
        return crate::ui::no_such_conversation(&id);
    };

    let conversation = match store::load_conversation(&state.pool, id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return crate::ui::no_such_conversation(&id.to_string()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "loading a Conversation failed");
            return crate::ui::unavailable("the Conversation could not be read");
        }
    };

    // The same roots the tree is drawn over, which is the point: what a pane is
    // told has moved should be the Worktrees it is showing and no others.
    let roots = crate::files::roots(&conversation)
        .into_iter()
        .map(|root| PathBuf::from(root.path))
        .collect();

    pane.on_upgrade(move |socket| attached(socket, state, id, roots))
}

/// Hold that attachment for as long as the socket is open.
async fn attached(
    mut socket: WebSocket,
    state: AppState,
    conversation_id: i64,
    roots: Vec<PathBuf>,
) {
    let _attached = state
        .watchers
        .attach(conversation_id, roots, state.nudges.clone());

    // Read rather than answered. Nothing is expected up it and nothing is sent
    // down it; what this loop is waiting for is the end, which is the pane
    // going — and the attachment above is let go of as this returns, whichever
    // way it returned.
    while socket.recv().await.is_some() {}
}
