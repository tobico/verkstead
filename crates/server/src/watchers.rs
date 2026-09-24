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
//! **The whole of each Worktree, walked ignore-aware.** Every non-ignored
//! directory of every root is watched, by the walk the tree's own ignore rules
//! answer — see [`crate::files::watchable`] — because a recursive watch over a
//! Rust `target/` would exhaust a machine's inotify watches on its own, which is
//! why it is a walk rather than a flag. The roots themselves are watched before
//! anything is walked, so a pane just attached is already following the top of
//! its Worktree while the walk is still going down.
//!
//! **And directories are added as they appear**, so a folder made after the
//! watcher started is watched without anything being restarted: the paths a burst
//! made, renamed or took away are measured against the disk when it is announced,
//! and what is there is walked from while what has gone is forgotten — see
//! [`Watching::caught_up`]. And where a burst named no paths at all — a watcher
//! that lost its place, a burst too wide to remember them — the whole of every
//! root is walked instead, and that walk is the account of what is there:
//! anything it does not name is let go of, which is the only word about a
//! removal such a burst ever gives. On Windows it names everything this watcher
//! holds whatever became of it, a watch being what keeps a deleted directory in
//! its parent's listing — see [`Held::spread_whole`], where that falls out.
//!
//! **And a burst about nothing the pane draws is not announced.** What is
//! watched is what the tree has rows for, so a path a burst names that is a
//! directory *not* watched is a directory the page cannot see — a build's own
//! `target/`, and whatever a platform reports against it afterwards, which
//! Windows does: a non-recursive watch there says the child changed when
//! something deep under it is written. So a `cargo build` is not a Nudge every
//! couple of seconds over a tree that never moved — see [`Watching::drawn`].
//!
//! **And each root's git `index` and `HEAD`, and nothing else under a
//! repository's insides.** The commit's own writes are the Worktree moving too,
//! and they are what the marks read: a commit rewrites the index without moving
//! HEAD, and a checkout moves HEAD. A Worktree's `.git` is a *file* pointing at
//! the repository's `worktrees/<name>` directory, so the two are found by asking
//! git where its git directory is rather than by joining `.git` onto the root —
//! see [`insides`].
//!
//! **Not a record.** Nothing here writes to the store, puts anything on a
//! Timeline or reaches a Share: it is the disk being read, and the Nudge is the
//! word that a read is worth making. It is memory only, and it goes with the
//! server.

use std::collections::{HashMap, HashSet};
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
/// [`QUIET`] alone would leave a page in front of a three-minute checkout
/// showing the tree as it stood when it began, because the quiet it is waiting
/// for never comes. This is the other end of that: something still writing is
/// looked at every couple of seconds, which is a handful of Nudges for the
/// thousands of files it wrote rather than one at the very end.
///
/// A burst that turns out to be about nothing the pane draws is still not
/// announced, so this is a ceiling on the Nudges a *build* costs rather than the
/// rate of them — see [`Watching::drawn`].
const AT_MOST: Duration = Duration::from_secs(2);

/// How many directories of one Conversation's Worktrees are watched.
///
/// The watcher takes a handle per directory on every platform and an inotify
/// watch per directory on Linux, where what a machine allows is the number in
/// `/proc/sys/fs/inotify/max_user_watches` — tens of thousands of them on the
/// machines this runs on, shared with everything else on the machine that is
/// watching anything. Ten thousand is over the whole of every ordinary checkout
/// with its ignores taken out — Verkstead's own is under a hundred — so the cap
/// is a bound on the monorepo rather than something a human meets, and what it
/// costs there is a corner of a checkout a pane is not told about.
pub(crate) const MAX_WATCHED: usize = 10_000;

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

/// Watch `roots` and everything of them worth watching until the word comes
/// back that nobody is attached any more, saying what moved as one
/// [`Nudge::Files`] per burst.
///
/// The clock of it is here and the disk of it is not: what a burst is, when it
/// has gone quiet and when it has run long enough to be announced regardless are
/// this loop's, and the walk a burst asks for goes off the runtime — see
/// [`aside`]. The watcher itself is [`Watching`], which is handed across and
/// back rather than shared.
///
/// **A directory that will not be watched is logged and left out** — see
/// [`Held::hold`]. A Conversation before grilling or after closing has no
/// Worktree at all, and one whose checkout has been removed under it has a path
/// that is not there: neither is a reason to refuse the attachment.
async fn following(
    conversation_id: i64,
    roots: Vec<PathBuf>,
    nudges: Nudges,
    mut closing: oneshot::Receiver<()>,
) {
    let (moved, mut moves) = mpsc::unbounded_channel();

    let Some(watching) = Watching::new(conversation_id, roots, moved) else {
        return;
    };

    // And the walk down from the roots, which is the first burst this task
    // catches up with: the roots themselves are watched above, before anything
    // is awaited, so the top of a Worktree is being followed while the walk is
    // still on its way down.
    // Announced about by nothing, whatever it found: the page opened this socket
    // and read the world it is drawing in the same breath, and the tree reads
    // every folder it has open the moment it is drawn.
    let Some((mut watching, _)) = aside(conversation_id, watching, Burst::whole()).await else {
        return;
    };

    // What the burst that is running has said, and when it is to be announced:
    // the quiet it is waiting for, and the ceiling it is announced at
    // regardless.
    let mut burst = Burst::default();
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
                let Some(said) = said else {
                    break;
                };

                burst.took(said);

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

                // Caught up with before it is announced, so that the folder the
                // burst made is being watched by the time the page has read it —
                // a file written in it a moment later is a Nudge rather than
                // nothing. And the catch-up is what says whether there is
                // anything to announce: see [`Watching::caught_up`].
                let Some((caught, saying)) = aside(
                    conversation_id,
                    watching,
                    std::mem::take(&mut burst),
                ).await else {
                    return;
                };

                watching = caught;

                if saying {
                    nudges.announce(Nudge::Files { conversation: conversation_id });
                }
            }
        }
    }

    // Off the runtime, the watcher's own thread being joined as it is dropped.
    if let Err(error) = tokio::task::spawn_blocking(move || drop(watching)).await {
        tracing::error!(%error, conversation_id, "letting a Worktree watcher go ended badly");
    }
}

/// Off the runtime with the watching while it catches up with `burst`, and back
/// with it and with whether the burst is worth announcing.
///
/// The walk opens a directory per directory and runs a git per level, which is
/// blocking work of the kind the rest of this server puts on a thread that may
/// block — so the watcher goes across with it and comes back, rather than being
/// shared with a lock nothing else would ever take. Whether to announce comes
/// back with it for the same reason: reading what is at a path is the disk being
/// asked, and the disk is asked over here.
///
/// `None` is that thread having ended badly, which takes the watcher with it:
/// there is nothing left to watch with and nothing to be done about it, so the
/// task says so and stops.
async fn aside(
    conversation_id: i64,
    mut watching: Watching,
    burst: Burst,
) -> Option<(Watching, bool)> {
    let caught = tokio::task::spawn_blocking(move || {
        let saying = watching.caught_up(burst);
        (watching, saying)
    })
    .await;

    match caught {
        Ok(caught) => Some(caught),
        Err(error) => {
            tracing::error!(
                %error,
                conversation_id,
                "walking a Conversation's Worktrees ended badly, so the Code pane will no longer \
                 follow the disk"
            );
            None
        }
    }
}

/// What one watcher is watching: the Worktrees it was given, the directories of
/// them it has found, and the two files of each root's insides.
struct Watching {
    /// The Worktrees, as [`attach`] read them off the Conversation.
    roots: Vec<PathBuf>,

    /// Each root's git `index` and `HEAD`, read afresh with every whole walk —
    /// see [`insides`].
    insides: Vec<PathBuf>,

    /// And the watcher itself, with everything it holds.
    held: Held,
}

impl Watching {
    /// A watcher over `roots`, with each root's own directory watched before
    /// anything else is.
    ///
    /// The watcher is the platform's own — inotify, FSEvents,
    /// `ReadDirectoryChangesW` — and it calls back on a thread of its own, so
    /// what it says is put on `moved` and read by [`following`]: the debounce is
    /// a clock, and a clock belongs on the runtime rather than on somebody
    /// else's thread.
    ///
    /// `None` is a machine that would not give this server a watcher at all,
    /// which is nothing a pane can be told and nothing to keep a task for.
    fn new(
        conversation_id: i64,
        roots: Vec<PathBuf>,
        moved: mpsc::UnboundedSender<Said>,
    ) -> Option<Watching> {
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            // Whatever it was, read on the watcher's own thread and said in a
            // word — or nothing said at all, a read of the Worktree not being
            // the Worktree moving. See [`said`].
            if let Some(said) = said(conversation_id, event) {
                let _ = moved.send(said);
            }
        });

        let watcher = match watcher {
            Ok(watcher) => watcher,
            Err(error) => {
                tracing::error!(
                    %error,
                    conversation_id,
                    "this machine would not give Verkstead a filesystem watcher, so the Code pane \
                     will not follow the disk"
                );
                return None;
            }
        };

        let mut watching = Watching {
            roots,
            insides: Vec::new(),
            held: Held {
                conversation_id,
                watcher,
                watched: HashSet::new(),
            },
        };

        // Before the walk and before anything is awaited, because a walk is a
        // moment and the page is drawn already: what this costs is a syscall a
        // root, and what it buys is the top of a Worktree followed from the
        // instant the pane attached.
        for root in &watching.roots {
            watching.held.hold(root.clone());
        }

        Some(watching)
    }

    /// Catch up with what `burst` said moved: a directory it put there is
    /// watched, one it took away is forgotten, and the insides are taken hold of
    /// again.
    ///
    /// A burst that named no paths — one the watcher lost its place in, or one
    /// too wide to have remembered them — is the whole of every root instead, and
    /// there the walk is the whole account of what is there: a directory it does
    /// not name is forgotten, that being the only word about a removal such a
    /// burst ever gives. See [`Held::spread_whole`].
    ///
    /// And hand back whether the burst is worth telling the page about, which is
    /// whether any of it is about something the pane draws — see [`Watching::drawn`].
    ///
    /// Blocking: the walk is [`crate::files::watchable`], which reads
    /// directories and runs git.
    fn caught_up(&mut self, burst: Burst) -> bool {
        match burst.whole {
            // The whole of every root, which is the first walk and any burst too
            // wide to have remembered what it made.
            true => {
                self.insides = self.roots.iter().flat_map(|root| insides(root)).collect();

                self.held.spread_whole(&self.roots);
            }

            // Or the paths this burst named, each measured against the disk: a
            // directory that has appeared is walked from where it appeared — a
            // `mkdir` being one and a tree moved in being any number under the one
            // path — and one that has gone is forgotten.
            false => {
                for moved in &burst.moved {
                    let Some(root) = self.roots.iter().find(|root| moved.starts_with(root)) else {
                        continue;
                    };

                    // Of the entry rather than of what it points at: a link to a
                    // directory is either a directory this walk has under its own
                    // name or one outside the root, and following it would be a
                    // way out of the Worktree.
                    let directory =
                        matches!(std::fs::symlink_metadata(moved), Ok(it) if it.is_dir());

                    match (directory, self.held.already(moved)) {
                        // Watched already, so walked already — which is most of
                        // what a burst names: a build's own directories are made
                        // once and written in for minutes.
                        (true, true) => {}

                        // Newly there, so walked from here.
                        (true, false) => self.held.spread(root, moved),

                        // And gone, so forgotten: the kernel dropped the watch
                        // with the directory, and a record of watching it would be
                        // a directory made again at the same name and never
                        // watched again.
                        (false, true) => self.held.forget(moved),

                        (false, false) => {}
                    }
                }
            }
        }

        // And the insides taken hold of again, whether or not they were held a
        // moment ago. A commit does not write the index: it writes a new one and
        // renames it over the old, so the file the watch was on is a file that
        // has gone and the watch went with it.
        for at in &self.insides {
            self.held.again(at);
        }

        // And whether any of it is worth a word, asked now that the walk has
        // caught up: a directory the burst made is watched by this point, and one
        // it took away is forgotten.
        //
        // A burst that named nothing is, and is most of them — a write says the
        // file it wrote and nothing about where a directory could have appeared,
        // so the paths it carries are none. So is a whole walk, which is a
        // watcher that lost its place and knows nothing about what it missed.
        burst.whole || burst.moved.is_empty() || burst.moved.iter().any(|moved| self.drawn(moved))
    }

    /// Whether `at` is something the pane draws, which is what a burst is
    /// measured by.
    ///
    /// **Anything that is not a directory is.** A file is a row of the tree or a
    /// tab of the pane, and a path that is not there at all is what a removal
    /// names — the row that has gone. The two git files the insides are watched
    /// through are files too, which is how a commit reaches the marks.
    ///
    /// **A directory is only where it is watched**, and what is not watched is
    /// what the walk left out: what git ignores, and `.git`. So a build's own
    /// `target/` appearing is a burst about no row there is, and so is whatever a
    /// platform reports against that directory afterwards — which Windows does,
    /// a non-recursive watch there saying the *child* changed when something deep
    /// under it is written. Left unasked, a `cargo build` would be a Nudge every
    /// couple of seconds and every one of them would have the page re-read every
    /// open folder, every open file and a `git status` per root, to be told
    /// nothing moved.
    fn drawn(&self, at: &std::path::Path) -> bool {
        !matches!(std::fs::symlink_metadata(at), Ok(it) if it.is_dir()) || self.held.already(at)
    }
}

/// The watcher, and every directory it has been told to watch.
struct Held {
    conversation_id: i64,
    watcher: notify::RecommendedWatcher,

    /// What is watched, so that a directory is walked once rather than once per
    /// burst that writes in it.
    ///
    /// Added to by the walk and taken from by a removal, and squared with the
    /// disk whenever the whole of a root is walked — see [`Held::spread_whole`],
    /// which is where the reason a record of watching what is not there costs
    /// anything is.
    ///
    /// The insides are not in here: they are watched afresh every burst, and
    /// what this set is for is knowing what has been walked.
    watched: HashSet<PathBuf>,
}

impl Held {
    /// Watch `at` and every non-ignored directory of `root` under it, as far as
    /// [`MAX_WATCHED`] allows.
    ///
    /// What is left of the allowance is what the walk is given, so the bound is
    /// over everything this watcher holds rather than over one walk of it: a
    /// monorepo is cut once, and not cut again per folder somebody makes in it.
    fn spread(&mut self, root: &std::path::Path, at: &std::path::Path) {
        let left = MAX_WATCHED.saturating_sub(self.watched.len());

        for each in crate::files::watchable(root, at, left) {
            self.hold(each);
        }
    }

    /// And every non-ignored directory of every one of `roots`, with whatever is
    /// no longer among them forgotten.
    ///
    /// **The one place a directory is forgotten without a word about it having
    /// gone**, and the reason this is not [`Held::spread`] per root. A whole walk
    /// is the whole account of what is there, and it is asked for by exactly the
    /// two things that throw a burst's own paths away: a watcher that lost its
    /// place, and a burst too wide to remember. So a removal inside one of those
    /// is a removal nothing else will ever mention — a `git checkout` across a
    /// branch that moves more than [`MOST_MOVED`] files is one, and it is the
    /// ordinary way this arises.
    ///
    /// Left in, a directory that has gone would cost twice: a watch of the
    /// allowance spent on nothing, so that a long-lived pane creeps up to
    /// [`MAX_WATCHED`] and quietly stops following anything new; and a name
    /// [`Held::already`] answers *yes* about, so that a directory made again at it
    /// is one nothing ever watches and a file written straight into it is a Nudge
    /// nobody gets.
    ///
    /// **And the watcher is told**, unlike [`Held::forget`]: that one is a
    /// removal the kernel had already dropped the watch for, and this is a walk
    /// noticing a directory has gone with nobody having said so. Windows keeps a
    /// deleted directory in its parent's listing for as long as anything holds a
    /// handle on it, and a watch *is* such a handle — so letting go is both what
    /// frees the allowance and what lets the directory finally go.
    ///
    /// **Which is a walk on Windows that finds nothing to let go of**, and that
    /// is the whole of what this does there. A directory this watcher still holds
    /// is a directory still in its parent's listing, so the walk names it and the
    /// reconciliation keeps it — and nothing is lost by that, because the name is
    /// not free for anything to be made at either, and a removal somebody *did*
    /// hear about came through [`Held::forget`] instead. Where a removal is
    /// walkable — Linux, macOS — this is what answers the bursts that never named
    /// one.
    fn spread_whole(&mut self, roots: &[PathBuf]) {
        let mut found = HashSet::new();

        for root in roots {
            let left = MAX_WATCHED.saturating_sub(found.len());

            found.extend(crate::files::watchable(root, root, left));
        }

        for at in self
            .watched
            .difference(&found)
            .cloned()
            .collect::<Vec<PathBuf>>()
        {
            self.let_go_of(&at);
        }

        for at in found {
            self.hold(at);
        }
    }

    /// Watch one directory, where it is not watched already.
    ///
    /// **A directory that will not be watched is logged and left out.** A
    /// Conversation before grilling or after closing has no Worktree at all, one
    /// whose checkout has been removed under it has a path that is not there, and
    /// a machine out of inotify watches has nothing left to give — none of them
    /// is a reason to refuse the attachment, and what each costs is a pane that
    /// hears nothing about a corner there is nothing to hear about. Forgotten
    /// rather than remembered as watched, so that a directory made again is
    /// walked again.
    fn hold(&mut self, at: PathBuf) {
        if !self.watched.insert(at.clone()) {
            return;
        }

        if let Err(error) = self.watcher.watch(&at, RecursiveMode::NonRecursive) {
            tracing::info!(
                %error,
                conversation_id = self.conversation_id,
                at = %at.display(),
                "a directory of a Worktree is not being watched"
            );

            self.watched.remove(&at);
        }
    }

    /// And take hold of one of a repository's insides again, watched or not.
    ///
    /// Debug rather than info, because the ordinary case for a failure here is a
    /// repository with nothing staged yet and so no `index` at all, which is
    /// nothing anybody has to read about once a burst.
    fn again(&mut self, at: &std::path::Path) {
        if let Err(error) = self.watcher.watch(at, RecursiveMode::NonRecursive) {
            tracing::debug!(
                %error,
                conversation_id = self.conversation_id,
                at = %at.display(),
                "a file of a Worktree's git directory is not being watched"
            );
        }
    }

    /// Whether `at` is watched, which is whether it has been walked.
    fn already(&self, at: &std::path::Path) -> bool {
        self.watched.contains(at)
    }

    /// And forget `at` and everything under it, which is a directory that has
    /// gone.
    ///
    /// Nothing is said to the watcher: the kernel drops a watch with the
    /// directory it was on, and what is left to do is stop believing it is held —
    /// otherwise a directory made again at the same name is one [`Held::already`]
    /// says is watched and nothing ever watches.
    ///
    /// Everything under it as well, because a directory goes with its own: one
    /// `rm -r` is one word about the top of it and nothing about the rest.
    fn forget(&mut self, at: &std::path::Path) {
        self.watched.retain(|held| !held.starts_with(at));
    }

    /// And let go of one outright, which is a walk having found it gone.
    ///
    /// The watcher is told here, where [`Held::forget`] says nothing: that one
    /// answers a removal the kernel had already dropped the watch for, and this
    /// answers a directory nobody said anything about — so the watch may well
    /// still be held, and on Windows holding it is what keeps the directory in
    /// its parent's listing at all. A refusal is nothing to say: a watch the
    /// platform has already dropped is one there was nothing to give back.
    fn let_go_of(&mut self, at: &std::path::Path) {
        let _ = self.watcher.unwatch(at);
        self.watched.remove(at);
    }
}

/// The two files of `root`'s git directory a watcher watches, and nothing else
/// under it.
///
/// **Asked of git rather than joined onto the root**, because a Worktree's
/// `.git` is a *file* pointing at the repository's `worktrees/<name>` directory
/// — and it is that directory's `index` and `HEAD` a commit in the Worktree
/// writes, not the repository's own.
///
/// **Those two and no others.** The commit's own writes are the Worktree moving
/// too, and they are what the marks are read from: a commit made on a branch
/// rewrites the index without moving HEAD, which is why the index is the one
/// that catches a commit, and HEAD is what catches a checkout. Everything else
/// under a repository's insides — its objects, its logs, the message of the
/// commit being written — is git's own business and none of the page's.
///
/// A root git will not answer about has none, which is a Worktree that has gone
/// or a directory that was never a checkout.
fn insides(root: &std::path::Path) -> Vec<PathBuf> {
    // `--absolute-git-dir` rather than `--git-dir` and a join, the second being
    // relative to the directory git was run in and this having to be a path.
    let Some(answer) = crate::repos::git(root, &["rev-parse", "--absolute-git-dir"]) else {
        return Vec::new();
    };

    let at = std::path::Path::new(answer.trim());

    vec![at.join("index"), at.join("HEAD")]
}

/// What the watcher's own thread says a burst is made of.
enum Said {
    /// Something moved, and these are the paths of it a directory could have
    /// appeared at or gone from.
    Moved(Vec<PathBuf>),

    /// Or something moved that is past telling, so every root wants walking
    /// again.
    Again,
}

/// One event read as one of those, or as nothing at all.
///
/// **A watcher in trouble asks for a walk.** An error here is most often a queue
/// that overflowed, and a rescan is the platform saying the same thing outright:
/// either way this is a burst the watcher saw the middle of none of, so a
/// directory may have been made without a word — exactly the moment to look at
/// everything again rather than the one moment not to.
///
/// **A read is not the Worktree moving, and this server reads plenty of it.** A
/// watched directory says so when a file in it is so much as opened, and a walk
/// runs a git per level that opens the `.gitignore` of every directory it
/// passes — so a walk left to count as movement would be a burst of its own,
/// that burst would be walked, and a Code pane on a Worktree nobody was touching
/// would announce itself forever. The human's own reads are the same thing more
/// quietly: a file opened in the editor is a file opened on the disk. A write
/// *finishing* is the one access that is a move, and the write it finishes has
/// been said already.
///
/// **And the paths are a create's, a rename's and a removal's alone.** A write is
/// most of what a watcher ever says and none of it can put a directory anywhere
/// or take one away, so a build's thousands of writes are thousands of paths
/// nobody has to remember. Read off the kind rather than off the disk, this being
/// the watcher's own thread: what is at a path is asked once, when the burst is
/// announced, and it is the disk's answer that tells a directory that appeared
/// from one that has gone. A watcher that will not say what it saw has everything
/// it named looked at.
fn said(conversation_id: i64, event: notify::Result<notify::Event>) -> Option<Said> {
    use notify::EventKind::{Access, Any, Create, Modify, Remove};
    use notify::event::{AccessKind, AccessMode, ModifyKind};

    let event = match event {
        Ok(event) => event,
        Err(error) => {
            tracing::debug!(%error, conversation_id, "watching a Worktree had trouble");
            return Some(Said::Again);
        }
    };

    if event.need_rescan() {
        return Some(Said::Again);
    }

    match event.kind {
        Access(AccessKind::Close(AccessMode::Write)) => Some(Said::Moved(Vec::new())),
        Access(_) => None,
        Any | Create(_) | Remove(_) | Modify(ModifyKind::Any | ModifyKind::Name(_)) => {
            Some(Said::Moved(event.paths))
        }
        _ => Some(Said::Moved(Vec::new())),
    }
}

/// One burst, as the task announcing it has heard it so far.
#[derive(Default)]
struct Burst {
    /// Whether every root is to be walked again from the top.
    whole: bool,

    /// Or the paths of it a directory could have appeared at or gone from,
    /// deduplicated: a `git checkout` names the same directory once per file it
    /// writes in it.
    moved: HashSet<PathBuf>,
}

impl Burst {
    /// The burst a walk from the top of every root is the whole of, which is the
    /// one a watcher starts with.
    fn whole() -> Burst {
        Burst {
            whole: true,
            ..Burst::default()
        }
    }

    /// And what the watcher said, taken into it.
    ///
    /// **A burst too wide to remember is a walk.** A checkout across a branch
    /// that moved a hundred thousand files names a path apiece, and holding them
    /// to walk from would be the checkout's own size in memory for an answer one
    /// walk gives: past [`MOST_MOVED`] the paths go and the whole of every root is
    /// walked instead.
    fn took(&mut self, said: Said) {
        if self.whole {
            return;
        }

        match said {
            Said::Again => {
                self.whole = true;
                self.moved = HashSet::new();
            }
            Said::Moved(paths) => {
                self.moved.extend(paths);

                if self.moved.len() > MOST_MOVED {
                    self.whole = true;
                    self.moved = HashSet::new();
                }
            }
        }
    }
}

/// How many paths of one burst are remembered before the whole of every root is
/// walked instead.
///
/// Over anything a human does and over the ordinary checkout, and a few hundred
/// kilobytes of paths at the very most — which is the point of a bound here
/// rather than a number anybody meets.
const MOST_MOVED: usize = 4096;

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

/// What the register a walk keeps does with a directory that has gone.
///
/// **Not on Windows**, and the one test in here says why: a deleted directory is
/// still listed, and still answers that it is a directory, while anything holds a
/// handle on it — so a removal nobody heard about is a removal nothing on that
/// platform can find. Everything else about this module is asked over a real
/// watcher and a real socket in `tests/watching.rs`, which runs everywhere.
#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    /// The register a walk keeps, over a watcher that is asked about nothing:
    /// what is under test here is which directories are *believed* watched, and
    /// the events are the socket's own tests' subject — see `tests/watching.rs`.
    fn register() -> Held {
        Held {
            conversation_id: 1,
            watcher: notify::recommended_watcher(|_: notify::Result<notify::Event>| {})
                .expect("this machine gives a watcher"),
            watched: HashSet::new(),
        }
    }

    /// A whole walk is the whole account of what is there, so a directory that
    /// has gone is forgotten by it.
    ///
    /// Which is the only word about a removal the two bursts that ask for a whole
    /// walk ever give: a watcher that lost its place, and one too wide to have
    /// remembered its paths. Left in, the name would be one [`Held::already`]
    /// answers *yes* about — so a directory made again at it would be watched by
    /// nothing, and a file written straight into it would be a Nudge nobody gets.
    ///
    /// **Not asked on Windows, which is what this module is gated on.** A deleted
    /// directory stays in its parent's listing, and answers that it is a
    /// directory, for as long as anything holds a handle on it — and a watch is
    /// such a handle, so the very thing this would be measuring is what keeps the
    /// directory there to be found. Nothing is lost by that: the name is not free
    /// for anything to be made at either, so neither half of the cost above can
    /// arise until the watch goes. A removal somebody *did* hear about is
    /// [`Held::forget`]'s, on every platform, and `tests/watching.rs` asks about
    /// that one over a real watcher.
    #[test]
    fn a_whole_walk_forgets_a_directory_that_has_gone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_owned();

        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::create_dir(root.join("docs")).unwrap();

        let mut held = register();
        held.spread_whole(std::slice::from_ref(&root));

        assert!(held.already(&root));
        assert!(held.already(&root.join("src")));
        assert!(held.already(&root.join("docs")));

        // `rm -r docs` inside a burst whose paths were thrown away, so nothing
        // ever said this directory had gone.
        std::fs::remove_dir(root.join("docs")).unwrap();
        held.spread_whole(std::slice::from_ref(&root));

        assert!(held.already(&root.join("src")));
        assert!(
            !held.already(&root.join("docs")),
            "a directory that is not there is still believed watched"
        );

        // And one made again at the same name is walked again rather than taken
        // for one that already is.
        std::fs::create_dir(root.join("docs")).unwrap();
        held.spread_whole(std::slice::from_ref(&root));

        assert!(held.already(&root.join("docs")));
    }
}
