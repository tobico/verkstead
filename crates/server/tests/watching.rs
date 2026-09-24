//! Following the disk: the watcher a Code pane's attachment runs over the
//! Worktrees it is drawn on, and the `files` Nudge it announces them moving as
//! (ADR 0019, *Following the disk*).
//!
//! Asked of a real router over a real Conversation with a real checkout, and
//! read off the wire at both ends: the attachment is a socket a browser dials,
//! and the Nudge comes down the stream an open page listens on. Nothing here
//! looks at the register — what has to hold is that a file written by something
//! that is not Verkstead reaches a page that is looking, and that nothing is
//! being watched for a page that is not.
//!
//! So what writes the files is `std::fs` rather than the files API: a terminal
//! tab, an agent and a build are all *outside* this server, and a write it made
//! itself would be a watcher proving something about its own caller.
//!
//! The clock is the machine's throughout. The debounce and the grace are
//! fractions of a second and a second or two, and every wait here is real
//! because the events they are about come off the kernel on a thread of their
//! own — a paused runtime would run the clock out before the disk had said
//! anything.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_schema::Nudge;
use verkstead_server::{open_database, router, store};

/// How long a test will wait for a Nudge it expects: generous, because it is
/// only ever paid when the assertion is about to fail. The stream's own tests
/// wait the same.
const PATIENCE: Duration = Duration::from_secs(5);

/// And how long it listens on after everything expected has arrived, to catch a
/// Nudge that should not have been sent at all.
///
/// Past the debounce the watcher announces a burst after, and past the ceiling
/// it announces one that will not go quiet at — so a second Nudge about one
/// write would be heard inside this rather than after it.
const SETTLING: Duration = Duration::from_secs(3);

/// And how long a watcher that should have stopped is given to stop.
///
/// The grace a detach waits out before the watcher goes, with room over it: the
/// whole claim is that a pane closed stops the watcher within a moment, so what
/// is waited here is that moment and not a length that would pass whatever the
/// grace was.
const STOPPING: Duration = Duration::from_secs(4);

/// A router over a fresh database, served on a port of its own — an attachment
/// is a socket rather than a request, so it is the one thing a `oneshot` over
/// the Router cannot ask for.
///
/// The Router itself comes back beside the address: the Nudge stream is an
/// ordinary response, and a clone of the Router shares the state the served one
/// is holding, so a page listening through this hears what a socket dialled
/// over there started.
async fn fresh_app() -> (tempfile::TempDir, SqlitePool, Router, SocketAddr) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let app = router(pool.clone());

    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let at = listener.local_addr().unwrap();

    tokio::spawn({
        let app = app.clone();
        async move {
            let _ = axum::serve(listener, app).await;
        }
    });

    (dir, pool, app, at)
}

/// A Conversation grilling in a Worktree of its own, and where that Worktree is.
///
/// The shape a grill start really makes, cut down to the one root this file is
/// about — `tests/files.rs` is where the companions are.
async fn grilling(pool: &SqlitePool, dir: &Path) -> (i64, PathBuf) {
    let repo = repository(dir.join("verkstead"));
    let registered = store::register_repo(pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(pool, registered.id, "following")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    let worktree = dir.join("worktrees/verkstead-following");
    let base = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "following",
            &worktree.to_string_lossy(),
            &base,
        ],
    );

    store::start_grilling(pool, conversation, &base, &worktree, &[])
        .await
        .unwrap();

    (conversation, worktree)
}

/// A Conversation that has never been grilled, so it has no Worktree at all.
async fn drafting(pool: &SqlitePool, dir: &Path) -> i64 {
    let repo = repository(dir.join("unstarted"));
    let registered = store::register_repo(pool, &repo, "unstarted", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    store::start_conversation(pool, registered.id, "following")
        .await
        .unwrap()
        .expect("the Repo was just registered")
}

/// A git repository with one commit in it. `tests/files.rs`'s, which is where
/// the same checkout is made for the reading half of this pane.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "tests@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Tests"]);
    git(&path, &["config", "commit.gpgsign", "false"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "-A"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

/// Run git in `dir`, insisting it worked. Scaffolding rather than the code under
/// test, so a failure here is a broken test.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("git should be on the PATH for these tests");

    assert!(
        output.status.success(),
        "git {args:?} failed in {}",
        dir.display()
    );

    String::from_utf8(output.stdout).unwrap()
}

/// A Code pane's attachment, as the browser holds one: the socket dialled, and
/// dropped when the pane goes.
struct Pane(
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
);

impl Pane {
    /// Draw a Code pane on this Conversation, which is the attachment being
    /// taken.
    async fn drawn(at: SocketAddr, conversation: i64) -> Pane {
        let url = format!("ws://{at}/api/ui/conversations/{conversation}/files/attach");

        let (socket, _) = tokio_tungstenite::connect_async(&url)
            .await
            .unwrap_or_else(|error| panic!("{url} to be attachable: {error}"));

        Pane(socket)
    }

    /// And take it down, which is the socket closing under the server.
    async fn closed(mut self) {
        self.0.close(None).await.unwrap();
    }
}

/// An open page, listening on the Nudge stream. `tests/nudges.rs`'s, cut down to
/// the reading this file makes: how many Nudges of a kind arrive in a while.
struct Listening {
    body: Body,
    buffered: String,
}

impl Listening {
    /// Open the stream the way a page does. Returns once the response is in
    /// hand, which is after the handler has subscribed — so anything the test
    /// does next is something this page is listening for.
    async fn open(app: &Router) -> Listening {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/nudges")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|kind| kind.to_str().ok()),
            Some("text/event-stream"),
        );

        Listening {
            body: response.into_body(),
            buffered: String::new(),
        }
    }

    /// The next whole frame off the stream, whatever kind it is.
    async fn frame(&mut self) -> String {
        loop {
            if let Some(end) = self.buffered.find("\n\n") {
                let frame = self.buffered[..end].to_owned();
                self.buffered = self.buffered[end + 2..].to_owned();
                return frame;
            }

            let chunk = self
                .body
                .frame()
                .await
                .expect("the stream should still be open")
                .unwrap();

            self.buffered
                .push_str(std::str::from_utf8(&chunk.into_data().unwrap()).unwrap());
        }
    }

    /// Every Nudge that lands in `over`, keep-alives left out.
    ///
    /// Read to the end of the window rather than stopping at the first, which is
    /// the whole point of it: what most of this file is about is how *many*
    /// arrived.
    async fn heard(&mut self, over: Duration) -> Vec<Nudge> {
        let mut heard = Vec::new();
        let until = tokio::time::Instant::now() + over;

        loop {
            let left = until.saturating_duration_since(tokio::time::Instant::now());

            let Ok(frame) = tokio::time::timeout(left, self.frame()).await else {
                return heard;
            };

            // A keep-alive is a comment line, which is the one frame down this
            // stream that is not a Nudge.
            if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                heard.push(serde_json::from_str(data).unwrap());
            }
        }
    }

    /// And the one Nudge expected, waited for rather than counted: the timeout
    /// is the assertion failing rather than the assertion.
    async fn nudge(&mut self) -> Nudge {
        let frame = tokio::time::timeout(PATIENCE, async {
            loop {
                let frame = self.frame().await;

                if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                    return data.to_owned();
                }
            }
        })
        .await
        .expect("a Nudge should have arrived");

        serde_json::from_str(&frame).unwrap()
    }
}

/// A file written at the top of a Worktree, the way a terminal tab writes one:
/// outside this server entirely.
fn wrote(worktree: &Path, name: &str) {
    std::fs::write(worktree.join(name), "something\n").unwrap();
}

/// A Code pane drawn on a Conversation starts a watcher for it, and a file
/// written at the top of its Worktree reaches the open page as one `files`
/// Nudge.
#[tokio::test]
async fn a_file_written_under_an_attached_pane_reaches_the_page() {
    let (dir, pool, app, at) = fresh_app().await;
    let (conversation, worktree) = grilling(&pool, dir.path()).await;

    let pane = Pane::drawn(at, conversation).await;
    let mut page = Listening::open(&app).await;

    wrote(&worktree, "notes.md");

    assert_eq!(page.nudge().await, Nudge::Files { conversation });

    drop(pane);
}

/// And a burst is one Nudge, not one per file: a build writes thousands and the
/// page wants to look once.
#[tokio::test]
async fn a_burst_of_writes_is_one_nudge() {
    let (dir, pool, app, at) = fresh_app().await;
    let (conversation, worktree) = grilling(&pool, dir.path()).await;

    let pane = Pane::drawn(at, conversation).await;
    let mut page = Listening::open(&app).await;

    for each in 0..200 {
        wrote(&worktree, &format!("written-{each}"));
    }

    // Listened to well past the debounce and past the ceiling a burst that will
    // not go quiet is announced at, so a second Nudge about this one write would
    // be in here.
    let heard = page.heard(SETTLING).await;

    assert_eq!(
        heard,
        vec![Nudge::Files { conversation }],
        "two hundred files written at once should be one Nudge"
    );

    drop(pane);
}

/// And nothing at all where no pane is attached: the watcher is on demand, and a
/// Conversation nobody is looking at costs this server no watch.
#[tokio::test]
async fn nothing_is_watched_for_a_conversation_with_no_pane_on_it() {
    let (dir, pool, app, _at) = fresh_app().await;
    let (_conversation, worktree) = grilling(&pool, dir.path()).await;

    let mut page = Listening::open(&app).await;

    wrote(&worktree, "notes.md");

    assert_eq!(page.heard(SETTLING).await, Vec::new());
}

/// The last pane closed stops the watcher within a moment.
#[tokio::test]
async fn the_last_pane_closed_stops_the_watcher() {
    let (dir, pool, app, at) = fresh_app().await;
    let (conversation, worktree) = grilling(&pool, dir.path()).await;

    let pane = Pane::drawn(at, conversation).await;
    let mut page = Listening::open(&app).await;

    // Watching to begin with, which is what makes the silence below mean
    // something: a test that only asserted quiet would pass against a watcher
    // that never started.
    wrote(&worktree, "before");
    assert_eq!(page.nudge().await, Nudge::Files { conversation });

    pane.closed().await;
    tokio::time::sleep(STOPPING).await;

    // Anything the closing itself was still announcing, drained before the
    // write this is about.
    let _ = page.heard(Duration::from_millis(100)).await;

    wrote(&worktree, "after");

    assert_eq!(
        page.heard(SETTLING).await,
        Vec::new(),
        "a Worktree should not be watched for a pane that has closed"
    );
}

/// And a swap to an Event and straight back leaves it running: the details pane
/// is taken down whenever something else is drawn in it, so a press on a
/// Timeline row and the back arrow is a socket closed and another opened a
/// moment later.
///
/// Read off a write made *in the gap*, which is the one reading that tells a
/// watcher left running from one torn down and built again: a rebuilt watcher
/// would have missed it.
#[tokio::test]
async fn a_swap_to_an_event_and_back_leaves_the_watcher_running() {
    let (dir, pool, app, at) = fresh_app().await;
    let (conversation, worktree) = grilling(&pool, dir.path()).await;

    let pane = Pane::drawn(at, conversation).await;
    let mut page = Listening::open(&app).await;

    pane.closed().await;

    // The Event drawn and the back arrow pressed, which is well inside the
    // grace: nothing on a page takes seconds to repaint.
    wrote(&worktree, "written-in-the-gap");

    assert_eq!(page.nudge().await, Nudge::Files { conversation });

    let again = Pane::drawn(at, conversation).await;
    drop(again);
}

/// A Conversation whose Worktree is not on disk attaches without failing, and
/// watches nothing.
///
/// A draft before grilling has no Worktree at all, which is the ordinary state
/// rather than a missing record — and the pane draws on it, so the attachment
/// has to be one it can take.
#[tokio::test]
async fn a_conversation_with_no_worktree_attaches_and_watches_nothing() {
    let (dir, pool, app, at) = fresh_app().await;
    let conversation = drafting(&pool, dir.path()).await;

    let pane = Pane::drawn(at, conversation).await;
    let mut page = Listening::open(&app).await;

    assert_eq!(page.heard(SETTLING).await, Vec::new());

    drop(pane);
}

/// And a Conversation this server has no record of is refused, the way a
/// terminal's socket refuses a number that is not live.
#[tokio::test]
async fn a_conversation_that_is_not_there_is_refused() {
    let (_dir, _pool, _app, at) = fresh_app().await;

    let dialled = tokio_tungstenite::connect_async(format!(
        "ws://{at}/api/ui/conversations/404/files/attach"
    ))
    .await;

    assert!(
        dialled.is_err(),
        "a Conversation that is not there should have no attachment to take"
    );
}
