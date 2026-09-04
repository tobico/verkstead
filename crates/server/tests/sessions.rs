//! A grilling session end to end: started by pressing the button, run inside
//! the Conversation's sandbox on its worktree, and read back off the Timeline
//! while it is still going.
//!
//! Everything here is real except the agent. The repository is a repository,
//! the worktree is one git made, the sandbox is bwrap and the pseudo-terminal is
//! Verkstead's own — what stands in for claude is a shell script, because what
//! these ask is whether a session's output reaches the human, and asking it of
//! the real claude would be a test that needed an account, a network and a
//! model's patience.
//!
//! The stub is handed exactly what the backend it stands where would be: the
//! model flag, the Profile's model, and then the Brief, with the session name
//! and the backend's own flags after it. So `$1` is the model it was told to
//! run, and everything Verkstead adds to that line goes on the end of it.
//!
//! **The Brief is `$2` on three of the four, and one place later on the
//! fourth.** Claude Code, codex and grok each take it as the one positional
//! argument; opencode's positional is the project to start in, so its Brief
//! goes under `--prompt` and `$2` is the flag — see `Line::prompt` in the
//! server's `sessions` module. The two stubs a session of that type runs put
//! the two back into the order the rest of this reads them in, before anything
//! looks at either, so every case below reads `$1` and `$2` whichever backend
//! it is standing in for.
//!
//! Watching a live session is here too, at the end, and is the one thing asked
//! over a socket rather than of the Router: an upgrade is a connection rather
//! than a request — see [`Watcher`]. It belongs in this file all the same,
//! because what makes a Screen live is a session running on a real terminal, and
//! this is where one runs.
//!
//! And typing into one, which is the Hold: a real keystroke down a real socket,
//! into the terminal a real session is reading. What those tests are about is
//! what it costs Verkstead — a run that ends nothing and advances nothing until
//! the keyboard goes back — so they drive whole backlogs, with the stub waiting
//! at a gate the test opens when it wants the step to land.
//!
//! **On the platforms a session runs on.** The sandbox is bwrap and the
//! terminal is a real pseudo-terminal, and a Windows Verkstead has neither: it
//! runs no session at all, and says so above the spawn rather than under it.
#![cfg(unix)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use avt::Vt;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::{Executor, SqlitePool};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;
use verkstead_render::{
    Adopted, AgentOutputEvent, Attached, BriefSaved, Capture, CommitEvent, CommitPane,
    CompanionAdded, CompanionMode, CompanionModeChosen, CompanionView, ConflictResolution,
    ConversationClosed, ConversationSteered, ConversationStopped, ConversationView,
    GrillingStarted, Lifecycle, NoticeEvent, PickedView, PinnedEvent, ProfileSaved,
    PullRequestEvent, Registered, Resolved, Resumed, Shown, Size, StageListReached, Started,
    SteerOpened, Submitted, TaskListEvent, TaskListReached, TimelineEvent, TranscriptView, Turn,
    Watching,
};
use verkstead_schema::{Direction, Nudge};
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::{Agents, Gh, Pace, WatchedPaths, open_database, router_running_sessions};
use verkstead_store::Decision;

/// The Brief every Conversation here is started from, and what the stub agent
/// is primed with.
const BRIEF: &str = "# Rate limiting\n\nThe API has none.\n";

/// Where the server these sessions belong to would be listening.
///
/// Nothing here dials it — a router driven by `oneshot` has no socket — but it is
/// what a session inside the sandbox is told to reach Verkstead at, which is a
/// thing worth reading back.
const LISTENING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8422);

/// How much longer than its own clock this suite is allowed to take, read once
/// from `VERKSTEAD_TEST_PACE`.
///
/// Every one of these fixtures stands a real server up and launches real
/// sessions into real sandboxes, and the runner decides on wall clock. So a
/// machine running the suite at half speed is one where a session gets
/// descheduled past a budget and is ended by the wrong rule — a failure about
/// the machine wearing the clothes of a failure about the code, and a different
/// test every run. There is nothing to observe instead, because on the tests
/// that matter here the budgets *are* the thing under test.
///
/// So the fix is a ruler that stretches: one factor, read from the environment,
/// applied to everything time-shaped in this file at once. Unset is `1.0`,
/// which is what a developer's own machine runs at; CI sets `2`, because a
/// two-core runner building the workspace alongside is the loaded shape these
/// flakes came from.
///
/// **Uniformly, and that is not a detail.** [`BRISKLY`] keeps `grace` under
/// `proposing` deliberately, so that a review ended after the shorter of them
/// is a review ended by the wrong rule — scaling the two by different factors
/// would quietly delete the thing several tests here exist to prove. One
/// multiplier over the whole file keeps every such ordering as it was written.
///
/// Only this suite reads it. A server's own [`Pace::default`] is untouched:
/// this is a test harness deciding how long to wait, not Verkstead deciding
/// how fast to work.
static PACE: LazyLock<f64> = LazyLock::new(|| {
    let Ok(factor) = std::env::var("VERKSTEAD_TEST_PACE") else {
        return 1.0;
    };

    factor
        .parse::<f64>()
        .ok()
        .filter(|factor| factor.is_finite() && *factor > 0.0)
        .unwrap_or_else(|| panic!("VERKSTEAD_TEST_PACE is a positive number, not {factor:?}"))
});

/// A span of this suite's own, stretched by [`PACE`].
///
/// Everything written as a literal duration in this file goes through here.
/// Anything derived from a [`Pace`] — `BRISKLY.grace * 4`, and the others like
/// it — does not, and must not: the Pace it came from was paced already, and
/// pacing it twice would stretch the window past the ordering it was chosen
/// against.
fn paced(span: Duration) -> Duration {
    span.mul_f64(*PACE)
}

/// Wait out a span of this suite's own — see [`paced`].
///
/// Two shapes use one: the gap between two reads in a loop that is waiting for
/// something to happen, and the do-nothing window a negative assertion holds
/// open. Both stretch for the same reason. A window has to stay long against
/// whatever else is running, or it stops being evidence that nothing happened;
/// and a loop that reads less often on a loaded machine is a loop taking less
/// of the machine the thing it is waiting for needs.
async fn pause(span: Duration) {
    tokio::time::sleep(paced(span)).await;
}

/// How long to wait for something a session does. Generously long, because what
/// is being waited on is a process starting: the flush that carries its output
/// is half a second, and a loaded machine can take a while to get bwrap and a
/// shell going.
///
/// And longer again where the machine says so — see [`PACE`].
static PATIENCE: LazyLock<Duration> = LazyLock::new(|| paced(Duration::from_secs(30)));

/// What every sandbox here is equipped with as `verkstead`.
///
/// A test harness is its own executable, and what a sandbox does with one is
/// bind it read-only — so any file that is really there will do where nothing in
/// this file runs it. That a session asks with the *server's* build is the
/// sandbox's own claim, and `tests/sandbox.rs` is where it is put to a session.
fn equipped(data_dir: &Path) -> Option<Executable> {
    Executable::of_the_server(data_dir)
}

/// A Conversation with a session running under a stub agent, and everything
/// holding its directories open.
struct Grilling {
    /// Dropped last, and only these keep the directories alive: a worktree that
    /// vanished mid-session would fail obscurely.
    _watched: tempfile::TempDir,
    home: tempfile::TempDir,
    state: tempfile::TempDir,

    /// A directory every sandbox gets read-write, so that a session can leave
    /// evidence of itself somewhere that outlives its worktree.
    spill: tempfile::TempDir,

    app: Router,
    id: i64,

    /// Where the database is, for the tests that stand a second server up over
    /// it.
    database: PathBuf,

    /// This fixture's place in the suite — see [`ROOM`]. Last, so that it is
    /// handed back only once everything above has been let go of.
    _room: tokio::sync::OwnedSemaphorePermit,
}

impl Grilling {
    /// The registered repository this Conversation is against.
    ///
    /// [`bench_at_pace`] puts it at one place under the watched directory and
    /// the fixture keeps that directory alive, so where it is is a fact about
    /// the bench rather than something to thread through.
    fn repo(&self) -> PathBuf {
        self._watched.path().join("verkstead")
    }

    /// And the home the OpenCode Profile this bench saved keeps its account in.
    ///
    /// [`Bench::on_one_home`] puts that Profile's home at one place under the
    /// same directory, so where opencode's account is is a fact about the bench
    /// rather than something to thread through — the same bargain
    /// [`Grilling::repo`] makes.
    fn opencode_account(&self) -> PathBuf {
        self._watched.path().join("opencode").join(".opencode")
    }

    /// The Conversation as the workbench reads it.
    async fn view(&self) -> ConversationView {
        get(&self.app, &format!("/api/ui/conversations/{}", self.id)).await
    }

    /// And as the sidebar reads it, which is where what is happening to it right
    /// now is drawn: a spinner while a session runs, a dot while something is
    /// waiting on the human.
    ///
    /// The one row, because these fixtures keep one Conversation.
    async fn row(&self) -> verkstead_render::ConversationEntry {
        let sidebar: Vec<verkstead_render::ConversationEntry> =
            get(&self.app, "/api/ui/conversations").await;

        sidebar
            .into_iter()
            .find(|row| row.id == self.id)
            .expect("this Conversation is on the sidebar")
    }

    /// Read the row back until it says something, or give up.
    async fn row_until<T>(
        &self,
        reached: impl Fn(&verkstead_render::ConversationEntry) -> Option<T>,
    ) -> T {
        let deadline = Instant::now() + *PATIENCE;

        loop {
            let row = self.row().await;

            if let Some(reached) = reached(&row) {
                return reached;
            }

            assert!(
                Instant::now() < deadline,
                "the row never got there. It says: {row:?}"
            );

            pause(Duration::from_millis(25)).await;
        }
    }

    /// Read it back until the session has got somewhere, or give up.
    async fn until<T>(&self, reached: impl Fn(&ConversationView) -> Option<T>) -> T {
        let deadline = Instant::now() + *PATIENCE;

        loop {
            let view = self.view().await;

            if let Some(reached) = reached(&view) {
                return reached;
            }

            if Instant::now() >= deadline {
                panic!(
                    "the session never got there. The Timeline says: {}{}",
                    standing(&view),
                    self.said_by_each(&view).await,
                );
            }

            pause(Duration::from_millis(25)).await;
        }
    }

    /// What every session on this Timeline actually put on its terminal, for the
    /// assertion above.
    ///
    /// Escaped rather than printed as it stands, because half of what a failure
    /// here turns on is what the bytes *were*: a display's own glyphs, a
    /// carriage return, an escape sequence — or a shell that printed the source
    /// of one because its `printf` does not know the escape. A Capture pasted
    /// raw into a panic message hides exactly that.
    ///
    /// Read only once the wait has given up, so a passing test pays nothing for
    /// it.
    async fn said_by_each(&self, view: &ConversationView) -> String {
        let mut said = String::new();

        for output in outputs(view) {
            said.push_str(&format!(
                "\n  #{} printed: {:?}",
                output.id,
                self.capture(output.id).await,
            ));
        }

        said
    }

    /// What the session has printed, whole, as the details pane fetches it.
    async fn capture(&self, event: i64) -> String {
        let capture: Capture = get(
            &self.app,
            &format!("/api/ui/conversations/{}/capture/{event}", self.id),
        )
        .await;

        capture.text
    }

    /// Wait until a session has printed something, or give up.
    ///
    /// For the tests whose stub prints a line to say where it has got to, and
    /// which then do something to the workbench before letting it go on. What
    /// the Capture holds is the whole of what a session has said, so a line
    /// that has landed in it is a session that has reached that point.
    async fn printed(&self, event: i64, said: &str) {
        let deadline = Instant::now() + *PATIENCE;

        loop {
            let capture = self.capture(event).await;

            if capture.contains(said) {
                return;
            }

            assert!(
                Instant::now() < deadline,
                "the session never printed {said:?}. It printed: {capture:?}",
            );

            pause(Duration::from_millis(25)).await;
        }
    }

    /// The Transcript of a session, as the store holds it: every line the agent
    /// wrote in its own log, in the order it wrote them.
    ///
    /// Read from the database rather than over the wire, because what is on the
    /// wire is the rendering of these lines and this is about the lines: what
    /// the store keeps is what the agent wrote, verbatim, whether or not
    /// anything knows how to draw it.
    async fn transcript(&self, event: i64) -> Vec<String> {
        let pool = open_database(&self.database).await.unwrap();

        verkstead_store::transcript(&pool, self.id, event)
            .await
            .unwrap()
            .expect("the session's output is on this Conversation's Timeline")
    }

    /// And the same Transcript as the details pane fetches it: read, rendered,
    /// and read back until it holds `turns` of them.
    async fn spoken(&self, event: i64, turns: usize) -> TranscriptView {
        let deadline = Instant::now() + *PATIENCE;

        loop {
            let view: TranscriptView = get(
                &self.app,
                &format!("/api/ui/conversations/{}/transcript/{event}", self.id),
            )
            .await;

            if view.turns.len() >= turns {
                return view;
            }

            assert!(
                Instant::now() < deadline,
                "the session's log was never drawn that far. The pane says: {view:?}"
            );

            pause(Duration::from_millis(25)).await;
        }
    }

    /// Read it back until it has that many lines, or give up.
    async fn transcript_of(&self, event: i64, lines: usize) -> Vec<String> {
        let deadline = Instant::now() + *PATIENCE;

        loop {
            let transcript = self.transcript(event).await;

            if transcript.len() >= lines {
                return transcript;
            }

            assert!(
                Instant::now() < deadline,
                "the session's log was never followed that far. \
                 The Transcript says: {transcript:?}"
            );

            pause(Duration::from_millis(25)).await;
        }
    }

    /// A second server over the same database, sandboxes, home and agent, which
    /// is what a restart is: nothing a wrap-up is watching lives in the process
    /// that started it.
    ///
    /// The caller holds on to what comes back — dropping the Router is the second
    /// server going away again.
    async fn restarted(&self, stub: &str, gh: &str) -> Router {
        router_running_sessions(
            open_database(&self.database).await.unwrap(),
            WatchedPaths::none(),
            self.state.path().to_owned(),
            Agents::running(
                vec!["/bin/sh".to_owned(), "-c".to_owned(), stub.to_owned()],
                Homes::on(
                    Platform::HERE,
                    self.home.path().to_owned(),
                    self.state.path(),
                ),
                Reachable::at(LISTENING),
                SandboxConfig::resolve(&[self.spill.path().display().to_string()]).unwrap(),
                // No shared build cache: what these tests are about runs a stub
                // where claude goes and builds nothing at all.
                BuildCache::none(),
                Skills::installed(self.state.path()).expect("this binary carries skills"),
                equipped(self.state.path()),
                Handoffs::under(self.state.path()),
                Attachments::under(self.state.path()),
                Settings::in_data_dir(self.state.path()),
            )
            .at_pace(*BRISKLY),
            gh_stub(gh),
        )
    }

    /// And one commit — its summary and its diff — as the same pane fetches it.
    async fn commit_pane(&self, event: i64) -> CommitPane {
        get(
            &self.app,
            &format!("/api/ui/conversations/{}/commit/{event}", self.id),
        )
        .await
    }

    async fn close(&self) -> ConversationClosed {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/close", self.id),
            &serde_json::json!({}),
        )
        .await
    }

    /// Put a Question Set to the human the way the session inside would, and
    /// hand back its id.
    ///
    /// Over the agent API rather than through the stub, because what these ask
    /// is what happens once a Set is answered — the stub's job is to be a
    /// session, not to be a CLI.
    ///
    /// Which leaves one thing the stub has to be told: that the Set is up. A real
    /// session asks moments after it starts and is talking until it does, so the
    /// silence a propose-then-fix session is ended on only ever begins with an ask
    /// of its own already open — see [`WHILE_NOBODY_HAS_ASKED`]. A Set posted from
    /// out here arrives whenever the test gets to it, so the marker is what puts
    /// the two back in the order they really happen in.
    async fn ask(&self, yaml: &str) -> i64 {
        let id = self.asking(yaml, "").await;

        self.asked();

        id
    }

    /// The same, deferred: what a session sends when it is not going to wait
    /// for the Answer, which is a query parameter rather than anything in the
    /// Set — see the server's `sets` module.
    async fn ask_deferred(&self, yaml: &str) -> i64 {
        let id = self.asking(yaml, "?deferred=true").await;

        self.asked();

        id
    }

    /// The same again on a backend whose sessions cannot wait: the ask says
    /// nothing about the channel, and the reply says the Set was stored.
    ///
    /// The one assertion the ask itself carries, because it is what the CLI
    /// reads to know it is not to open a wait — see `verkstead_schema`'s
    /// `SetCreated`.
    async fn ask_stored(&self, yaml: &str) -> i64 {
        let created = self.submitting(yaml, "").await;

        assert!(
            created.stored,
            "an ordinary ask on a store-and-nudge backend comes back stored, \
             which is what tells the CLI to return: {created:?}",
        );

        self.asked();

        created.id
    }

    /// What all of them are made of: post the Set over the agent API and read
    /// its id back.
    async fn asking(&self, yaml: &str, how: &str) -> i64 {
        self.submitting(yaml, how).await.id
    }

    /// And the whole of what the server said, for the one caller that reads
    /// more of it than the id.
    async fn submitting(&self, yaml: &str, how: &str) -> verkstead_schema::SetCreated {
        let (status, body) = fetch(
            &self.app,
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{}/api/v1/sets{how}", self.id))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED, "the Set was refused: {body}");

        serde_saphyr::from_str(&body).unwrap()
    }

    /// Tell the stub that the Set it would have sent is up, which is what stops
    /// it talking — see [`WHILE_NOBODY_HAS_ASKED`].
    ///
    /// Written for every ask rather than only where a stub reads it: a marker
    /// nobody is watching for costs nothing.
    fn asked(&self) {
        let directory = handoff_directory(self);

        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("asked"), "").unwrap();
    }

    /// And take it away again, which is what says the Set that was up has been
    /// answered and no other is.
    ///
    /// **The marker is one Conversation's rather than one session's**, the
    /// handoff directory being the Conversation's — so a Set answered early in
    /// a run would otherwise go on telling every session after it that somebody
    /// is asking. What that costs is a session that falls straight through the
    /// loop it was meant to talk through, goes quiet with nothing of its own
    /// open, and is ended before the test has asked it anything: the fixture
    /// racing an ender rather than anything about the work.
    fn asked_nothing(&self) {
        let asked = handoff_directory(self).join("asked");

        if asked.exists() {
            std::fs::remove_file(asked).unwrap();
        }
    }

    /// Answer it the way the human does, from the browser.
    async fn answer(&self, set_id: i64) -> Submitted {
        post(
            &self.app,
            &format!("/api/ui/sets/{set_id}/response"),
            &serde_json::json!({ "answers": [{ "label": "Q9", "selected": 1 }] }),
        )
        .await
    }

    /// The same, ticking **Nothing else** beside the comment box — which is the
    /// human saying a follow-up is over, and no Answer to any Question the agent
    /// asked. See the schema's `Response::nothing_else`.
    async fn answer_ending(&self, set_id: i64) -> Submitted {
        post(
            &self.app,
            &format!("/api/ui/sets/{set_id}/response"),
            &serde_json::json!({
                "answers": [{ "label": "Q9", "selected": 1 }],
                "nothing_else": true,
            }),
        )
        .await
    }

    /// The same, picking a direction on the chooser — which is what accepts a
    /// proposal, and so what sets the picked direction's pipeline going.
    async fn pick(&self, set_id: i64, direction: &str) -> Submitted {
        post(
            &self.app,
            &format!("/api/ui/sets/{set_id}/response"),
            &serde_json::json!({
                "answers": [{ "label": "Q9", "selected": 1 }],
                "direction": direction,
            }),
        )
        .await
    }

    /// The same, for a Set whose questions are not the proposal's one.
    async fn respond(&self, set_id: i64, answers: serde_json::Value) -> Submitted {
        post(
            &self.app,
            &format!("/api/ui/sets/{set_id}/response"),
            &serde_json::json!({ "answers": answers }),
        )
        .await
    }

    /// Wait until driving has stopped, and hand back the Notice saying why.
    ///
    /// The last one on the Timeline: a Conversation collects notices over a long
    /// run — a stage adopted, a roadmap finished — and what a stop writes is the
    /// newest thing Verkstead had to say.
    async fn stopped(&self) -> NoticeEvent {
        self.until(|view| said(view).last().map(|notice| (*notice).clone()))
            .await
    }

    /// Who stopped it, which is the half of a stop the Timeline does not draw.
    /// A restart reads it to decide whether to take the Conversation up, and
    /// the marks read it to decide whether to say anything — so the record is
    /// the only place to ask.
    async fn chosen(&self) -> Decision {
        self.stop_on_the_record().await.decision
    }

    /// The whole of the stop as the record holds it, for the tests that ask
    /// about the halves the Timeline does not draw.
    async fn stop_on_the_record(&self) -> verkstead_store::Stopped {
        let pool = open_database(&self.database).await.unwrap();

        verkstead_store::stopped(&pool, self.id)
            .await
            .unwrap()
            .expect("the Conversation has stopped")
    }

    /// Take the stop away and touch nothing else, which is the half of Resume
    /// the badge hangs on.
    ///
    /// Through the store rather than through the press, for the tests that are
    /// about what the badge is drawn from: Resume also starts a session, and a
    /// session starting is not what those are asking about.
    async fn drive_again(&self) {
        let pool = open_database(&self.database).await.unwrap();

        verkstead_store::clear_stop(&pool, self.id).await.unwrap();
    }

    /// And press Resume, the way the button at the end of a Timeline does.
    ///
    /// Nothing goes with it: what should be running is recomputed from the state
    /// the Conversation is in and what its branch has written.
    async fn resume(&self) -> Resumed {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/resume", self.id),
            &serde_json::json!({}),
        )
        .await
    }

    /// And press **Resolve conflicts**, the way the button on a finished
    /// Conversation's pull request pane does. Nothing goes with it either:
    /// which Conversation it is is the whole of what it says, and which of its
    /// pull requests conflict is the record's to know.
    async fn resolve_conflicts(&self) -> Resolved {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/resolve-conflicts", self.id),
            &serde_json::json!({}),
        )
        .await
    }

    /// And press Stop, which is the same shape of press: nothing goes with it,
    /// and what comes back says whether the run has stopped or is about to.
    async fn stop(&self) -> ConversationStopped {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/stop", self.id),
            &serde_json::json!({}),
        )
        .await
    }

    /// Open the Conversation, as far as the server is concerned: the press the
    /// browser makes when the human walks into one, which takes the news mark
    /// off its sidebar row.
    ///
    /// Answers nothing, and is refused for nothing — it rides every opening of
    /// every Conversation, so there is nothing for it to be wrong about.
    async fn see(&self) {
        let (status, body) = fetch(
            &self.app,
            Request::builder()
                .method("POST")
                .uri(format!("/api/ui/conversations/{}/seen", self.id))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::NO_CONTENT, "the press failed: {body}");
    }

    /// And click Steer, which is the row beside those in the same menu: it
    /// stops the drive so that nothing launches while the human composes, and
    /// says what it found running.
    async fn steer(&self) -> SteerOpened {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/steer", self.id),
            &serde_json::json!({}),
        )
        .await
    }

    /// And submit the modal that opened: where the work goes, and whether to end
    /// what is running where it stands.
    ///
    /// No Pairing, which is the human leaving the picker on what the
    /// Conversation already runs the work under.
    async fn steer_into(&self, target: &str, interrupt: bool) -> ConversationSteered {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/steer/submit", self.id),
            &serde_json::json!({ "target": target, "interrupt": interrupt }),
        )
        .await
    }

    /// And the submit into Grilling, which is the one target that carries a
    /// payload: the Brief of the round it opens where the human wrote one, and
    /// whether the session is primed with everything they have already answered.
    async fn steer_grilling(&self, brief: Option<&str>, digest: bool) -> ConversationSteered {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/steer/submit", self.id),
            &serde_json::json!({
                "target": "Grilling",
                "interrupt": false,
                "brief": brief,
                "digest": digest,
            }),
        )
        .await
    }

    /// And the submit into Implementing with something written, which is the
    /// other payload: what the session it starts is sent off to do.
    async fn steer_instructed(&self, instruction: &str) -> ConversationSteered {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/steer/submit", self.id),
            &serde_json::json!({
                "target": "Implementing",
                "interrupt": false,
                "instruction": instruction,
            }),
        )
        .await
    }

    /// And the submit that opens a companion up as it moves: which of the ones
    /// already there goes to read-write, and what the branch cut in it is
    /// called — empty being *mirroring*, the Conversation's own branch name.
    ///
    /// No mode on the row, because there is one direction: read-only is not
    /// something the modal can ask for.
    async fn steer_opening(
        &self,
        target: &str,
        instruction: &str,
        repo_id: i64,
        branch: &str,
    ) -> ConversationSteered {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/steer/submit", self.id),
            &serde_json::json!({
                "target": target,
                "interrupt": false,
                "instruction": instruction,
                "upgraded": [{ "repo_id": repo_id, "branch": branch }],
            }),
        )
        .await
    }

    /// And the submit into Follow-up, which carries the payload that is always
    /// required: the brief the session it starts opens the follow-up on.
    async fn steer_following_up(&self, brief: &str) -> ConversationSteered {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/steer/submit", self.id),
            &serde_json::json!({
                "target": "FollowUp",
                "interrupt": false,
                "follow_up": brief,
            }),
        )
        .await
    }

    /// What the next session to start printed, waited for from a Timeline that
    /// held `before` of them.
    ///
    /// Two waits rather than one: a session appears on the Timeline as it
    /// starts, and what it was primed with is on its terminal a moment later.
    async fn printed_after(&self, before: usize) -> String {
        let started = self
            .until(|view| {
                let running = outputs(view);
                (running.len() > before).then(|| running[before].id)
            })
            .await;

        let said = self
            .until(|view| {
                outputs(view)
                    .into_iter()
                    .find(|output| output.id == started && output.lines > 1)
                    .map(|output| output.id)
            })
            .await;

        self.capture(said).await.replace("\r\n", "\n")
    }

    /// The same submit with a Pairing picked: what the work runs under from
    /// here, which is recorded as the Conversation's own.
    async fn steer_under(&self, target: &str, profile_id: i64, model: &str) -> ConversationSteered {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/steer/submit", self.id),
            &serde_json::json!({
                "target": target,
                "interrupt": false,
                "pairing": { "profile_id": profile_id, "model": model },
            }),
        )
        .await
    }

    /// The Agent Profile of that name, as the modal's picker reads the list.
    async fn profile(&self, name: &str) -> i64 {
        let profiles: Vec<verkstead_render::ProfileEntry> =
            get(&self.app, "/api/ui/profiles").await;

        profiles
            .into_iter()
            .find(|profile| profile.name == name)
            .expect("the fixture saved a Profile per role")
            .id
    }

    /// And Force stop, which is the same press without the waiting.
    async fn force_stop(&self) -> ConversationStopped {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/force-stop", self.id),
            &serde_json::json!({}),
        )
        .await
    }

    /// Wait until there is a session running, and hand back the Event it is
    /// printing into — which is what a Screen is watched by.
    async fn running(&self) -> i64 {
        self.until(|view| output(view).filter(|output| output.running).map(|o| o.id))
            .await
    }

    /// The same for a Conversation that has run more than one session: the
    /// `nth` of them once it is printing, which is the one there is a Screen to
    /// attach to.
    ///
    /// Counted rather than taken as the latest, because the sessions of a run
    /// overlap: a pick leaves the grilling session running until whatever it
    /// asked for lands, so *the latest that is running* is the session before
    /// this one right up to the moment it is not.
    async fn attachable(&self, nth: usize) -> i64 {
        self.until(|view| {
            outputs(view)
                .get(nth - 1)
                .filter(|output| output.running)
                .map(|output| output.id)
        })
        .await
    }

    /// Put a device on the human's list, the way the page does when
    /// notifications are turned on for it.
    async fn subscribe(&self, device: &Device) {
        let stored: verkstead_render::Subscribed = post(
            &self.app,
            "/api/ui/push/subscribe",
            &serde_json::json!({
                "endpoint": device.endpoint,
                "p256dh": device.p256dh(),
                "auth": device.auth(),
            }),
        )
        .await;

        assert_eq!(stored, verkstead_render::Subscribed::Stored);
    }

    /// The same workbench, on a socket of its own, and where to find it.
    ///
    /// Everything else here asks the Router directly, which is cheaper and is
    /// how every other endpoint is asked. The Screen's socket cannot be asked
    /// that way: an upgrade is a connection rather than a request, so this is
    /// the one thing that needs a server actually listening somewhere.
    ///
    /// The same Router rather than a second one, because it is the same server:
    /// what the socket hands over is the session this fixture started, held in
    /// the register this Router shares.
    async fn listening(&self) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("a port to listen on");

        let at = listener.local_addr().unwrap();
        let app = self.app.clone();

        // Left running for the rest of the test. Nothing takes it down: the
        // process ends when the test does, and a listener held open costs a
        // socket.
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        at
    }

    /// Wait until nothing is running here, which is when the composer is
    /// offered.
    async fn quiet(&self) {
        self.until(|view| (!view.working).then_some(())).await
    }
}

/// A closing Set: the proposal that ends a grilling, which is what puts the
/// direction chooser on the page.
const PROPOSING: &str = r#"
title: Ready to build the rate limiter
questions:
  - label: Q9
    text: Anything still open before we build it?
    options:
      - n: 1
        text: Nothing from me
        recommended: true
      - n: 2
        text: Yes, see below
proposal:
  direction: inline
  rationale: |
    One change, in one file.
"#;

/// And a breakdown's quiz: an ordinary Set, carrying no `proposal` block at all,
/// because the direction it is being asked under was settled before this session
/// started.
const BREAKDOWN_QUIZ: &str = r#"
title: Does this breakdown look right?
questions:
  - label: Q9
    text: Six tasks, or should the migration be its own?
    options:
      - n: 1
        text: Six is right
        recommended: true
      - n: 2
        text: Split the migration out
"#;

/// A round of grilling that came back: what the session asked before it died,
/// and what a fresh one has to be told rather than ask again.
const ASKED_ALREADY: &str = r#"
title: How the limiter counts
questions:
  - label: Q1
    text: Per key or per address?
    options:
      - n: 1
        text: Per key
      - n: 2
        text: Per address
"#;

/// And one that did not: the Set the session was still waiting on when it went,
/// which nothing is reading any more.
const LEFT_HANGING: &str = r#"
title: What happens when it trips
questions:
  - label: Q2
    text: How long should a client be locked out?
    options:
      - n: 1
        text: A minute
      - n: 2
        text: Until the window rolls over
"#;

/// How fast these run the backlog: fast enough that a test spends its time
/// launching sessions rather than sleeping between them.
///
/// The pace a server keeps is [`Pace::default`] — two seconds and five. What is
/// being asked here is whether a session is ended once its step has landed *and*
/// it has gone quiet, and the number of seconds that takes is not part of the
/// answer.
///
/// Every span here goes through [`paced`], so a loaded machine gets budgets it
/// can meet — and all of them by the same factor, which is what keeps the
/// ordering below saying what it was written to say.
static BRISKLY: LazyLock<Pace> = LazyLock::new(|| Pace {
    poll: paced(Duration::from_millis(100)),
    grace: paced(Duration::from_millis(300)),
    checks: paced(Duration::from_millis(100)),
    // Longer than the grace above, as a server's is: the tests that watch a
    // review being ended on quiet want the two apart, so that a session ended
    // after the shorter of them is one ended by the wrong rule.
    proposing: paced(Duration::from_millis(900)),
    // Four times the grace above, where a server's is five times it: what the
    // ceiling on a stir is for is a session that will never speak again, so the
    // tests that watch one being held off want a window between the two they
    // can assert *nothing happened* inside — and one long enough that a loaded
    // machine cannot close it.
    waking: paced(Duration::from_millis(3600)),
    // Longer than any of these run for, so that the sweep for a stalled
    // Conversation is the one thing that never fires by itself here. Every one
    // of these fixtures is a Conversation whose grilling session has printed and
    // exited, which is a stall by every rule there is — so the tests that are
    // about something else say nothing about it, and the ones that are about it
    // keep [`SWEEPING`].
    stalls: paced(Duration::from_secs(600)),
    // And longer than any of these run for again, for the stall sweep's reason
    // one sweep along: every fixture that reaches Done has a pull request
    // nothing has merged, which is exactly what the sweep after Done goes and
    // asks about — so the tests that are about something else say nothing about
    // it, and the ones that are about it keep [`LANDING`].
    merges: paced(Duration::from_secs(600)),
    // And longer again, for the same reason a third time: every fixture here
    // that ends up archived would have its output taken out from under whatever
    // the test was written to read. The ones that are about the cleanup keep
    // [`CLEANING`].
    cleanup: paced(Duration::from_secs(600)),
    // Nothing, which is a server's own: the review takes the Worktree as soon
    // as the wrap-up starts, and the tests that want the window before it hold
    // it open themselves.
    reviewing: Duration::ZERO,
    // Clear of the grace on both sides and clear of the three-second mark a
    // session is idle on once its screen says so, where a server's is five
    // minutes against sixty seconds and three. Nothing here is judged by it
    // unless its stub draws a screen — see [`DRAWING`] — and the fixtures that
    // are want the window: a silence mid-turn has to be able to run past the
    // grace without reaching this, a session caught by this has to be one the
    // grace alone would have caught much sooner, and a backend read by its
    // at-work line has to be able to go quiet for the three seconds that end it
    // without this ending it first.
    long_stop: paced(Duration::from_millis(6000)),
});

/// And the same at a pace that does look, for the tests that are about the
/// looking.
///
/// A server sweeps every minute. What is being asked here is whether a
/// Conversation nothing is driving is noticed while the server runs, and the
/// number of seconds it waits before noticing is not part of the answer.
static SWEEPING: LazyLock<Pace> = LazyLock::new(|| Pace {
    stalls: paced(Duration::from_millis(100)),
    ..*BRISKLY
});

/// And the same at a pace that sweeps the pull requests of what is already
/// finished, for the tests about what happens to one after Done.
///
/// A server sweeps those every fifteen minutes. What is being asked here is
/// whether a pull request that starts conflicting long after the work on it is
/// over is noticed at all, and the number of minutes it waits is not part of the
/// answer.
static LANDING: LazyLock<Pace> = LazyLock::new(|| Pace {
    merges: paced(Duration::from_millis(100)),
    ..*BRISKLY
});

/// And the same at a pace that cleans up after the archivings, for the tests
/// about what becomes of a Conversation the human has finished looking at.
///
/// A server sweeps those every hour, over a clock counted in days. What is
/// being asked here is what one pass does to each kind of archived
/// Conversation, and the hour it waits between passes is not part of the
/// answer.
static CLEANING: LazyLock<Pace> = LazyLock::new(|| Pace {
    cleanup: paced(Duration::from_millis(100)),
    ..*BRISKLY
});

/// What stands where the host's `gh` goes: a branch with a pull request on it,
/// and nothing said on it yet.
///
/// A script rather than the real thing, for the reason the agent is one: what
/// these ask is what Verkstead does with the answer, and asking GitHub itself
/// would be a test that needed a network, an account and a pull request.
///
/// It tells the questions apart by the fields being asked for, because that is
/// what tells them apart on the command line — except the comments left on the
/// lines of the diff, which are `gh api`'s and so are told by `$1`.
///
/// The checks are on the details pane's question and on nothing else here: the
/// watcher asks about them on their own and this answers that with the pull
/// request itself, which reads as a suite of nothing. Which is what makes the
/// pane's own freshening visible — nothing else writes a rollup down.
///
/// The pane's own answer carries the two facts about the pull request itself
/// beside them — whether it merges, and where it has got to — for the same
/// reason and one of its own: opening the pane is the one thing left that asks
/// GitHub about a pull request nothing is watching, so it freshens every fact a
/// card is drawn off rather than only the list it came for. Where it has got to
/// is the one of the three nothing else here writes down, which is what makes
/// that freshening visible.
///
/// That one answer carries the `mergeable` the same watcher reads, because a
/// wrap-up waits on that as much as on the checks: a pull request nothing has
/// said merges is one Verkstead is still waiting to hear about, and every
/// fixture here that reaches Done would sit in Wrapping for ever. The pull
/// request lookup ignores the extra field, as every `gh` answer ignores the
/// fields nobody asked it for.
const PULL_REQUEST: &str = r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$5" in
*commits*)
    printf '{"commits":[{"oid":"c0ffee1","messageHeadline":"feat: count the requests"}],"comments":[{"author":{"login":"tobico"},"body":"Looks **good**.","createdAt":"2026-08-21T09:00:00Z"}],"statusCheckRollup":[{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}],"mergeable":"MERGEABLE","state":"OPEN"}'
    ;;
*comments*)
    printf '{"comments":[],"reviews":[]}'
    ;;
*)
    printf '{"mergeable":"MERGEABLE","number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}'
    ;;
esac
"#;

/// A `gh` whose pull request has one check on it, concluded as `how` — the word
/// GitHub puts in a check run's `conclusion` column. Shell text rather than a
/// literal, so a caller can hand it something that changes mid-test.
///
/// Everything else it answers is [`PULL_REQUEST`]'s, because a wrap-up needs the
/// pull request found before it has any checks to ask about.
fn gh_checking(how: &str) -> String {
    format!(
        r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$5" in
*statusCheckRollup*)
    printf '{{"mergeable":"MERGEABLE","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"%s","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "{how}"
    ;;
*commits*)
    printf '{{"commits":[],"comments":[]}}'
    ;;
*comments*)
    printf '{{"comments":[],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#
    )
}

/// A `gh` whose pull request answers `rollup` about its checks, carries `said`
/// in its conversation and `on_the_diff` on the lines of its diff — each of them
/// the entries of the answer, so a test can hand it however many it needs and
/// none at all.
///
/// The comments on the diff are told by `$1`, because they are `gh api`'s rather
/// than `gh pr view`'s. The rest are told by the fields being asked for, in the
/// order the field lists have to be told apart in: the details pane asks for
/// `commits,comments` and the comment watcher asks for `comments,reviews`, and
/// only the first of those contains `commits`.
fn gh_about(rollup: &str, said: &str, on_the_diff: &str) -> String {
    format!(
        r#"
if [ "$1" = api ]; then printf '[{on_the_diff}]'; exit 0; fi
case "$5" in
*statusCheckRollup*)
{rollup}
    ;;
*commits*)
    printf '{{"commits":[],"comments":[]}}'
    ;;
*comments*)
    printf '{{"comments":[{said}],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#
    )
}

/// The same, with the details pane's own question answered beside the watchers'.
///
/// The pane asks for `commits` and the rollup together, so its question has to be
/// matched before the checks watcher's — the watcher's own field is in both, and
/// a stub that cannot be asked about the checks would otherwise refuse the pane
/// as well. What it answers with is the same comments the watcher is given, which
/// is the point: the pane draws what is on the pull request, whatever a rule has
/// taken out of what an agent is sent about.
fn gh_about_with_a_pane(rollup: &str, said: &str, on_the_diff: &str) -> String {
    format!(
        r#"
if [ "$1" = api ]; then printf '[{on_the_diff}]'; exit 0; fi
case "$5" in
*commits*)
    printf '{{"commits":[],"comments":[{said}],"statusCheckRollup":[],"mergeable":"MERGEABLE","state":"OPEN"}}'
    ;;
*statusCheckRollup*)
{rollup}
    ;;
*comments*)
    printf '{{"comments":[{said}],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#
    )
}

/// The same, except that nothing has been said on the pull request until `after`
/// is there with something in it.
///
/// Which is how a test says *when* a comment landed. Everything already on a pull
/// request when a wrap-up's review starts is the review's to propose about, so a
/// test about what a batch session does has to put the comment there afterwards —
/// and the file the review session writes its prompt into is exactly the moment
/// it started.
fn gh_about_once(rollup: &str, after: &Path, said: &str, on_the_diff: &str) -> String {
    format!(
        r#"
if [ -s {after} ]; then said='{said}'; on_the_diff='{on_the_diff}'; else said=; on_the_diff=; fi
if [ "$1" = api ]; then printf '[%s]' "$on_the_diff"; exit 0; fi
case "$5" in
*statusCheckRollup*)
{rollup}
    ;;
*commits*)
    printf '{{"commits":[],"comments":[]}}'
    ;;
*comments*)
    printf '{{"comments":[%s],"reviews":[]}}' "$said"
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#,
        after = quoted(after),
    )
}

/// A `gh` for the sweep that watches a pull request after the work on it is
/// Done: everything a wrap-up asks answered green and merging, and the sweep's
/// own question answered out of a file the test writes.
///
/// The sweep is told from every other question by the fields it asks for and
/// nothing else — `mergeable,state`, matched exactly rather than by a glob,
/// because the details pane asks for both of those beside its own three. Which
/// is the whole of what makes these tests possible: nothing but the sweep is
/// ever answered out of `landing`.
///
/// Every one of those questions is written down in `asked` as it arrives, so a
/// test can read back not only what Verkstead recorded but whether it went and
/// asked at all — which is what *never asked about again* is read off.
///
/// An empty `landing` is a pull request that is open and merges, which is what
/// every one of these starts as: the file is how a test says GitHub has come to
/// say something else.
fn gh_landing(asked: &Path, landing: &Path) -> String {
    format!(
        r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$5" in
mergeable,state)
    printf '%s\n' "$5" >> {asked}
    if [ -s {landing} ]; then cat {landing}; else printf '{{"mergeable":"MERGEABLE","state":"OPEN"}}'; fi
    ;;
*commits*)
    printf '{{"commits":[],"comments":[],"mergeable":"MERGEABLE","state":"OPEN"}}'
    ;;
*statusCheckRollup*)
{GREEN}
    ;;
*comments*)
    printf '{{"comments":[],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#,
        asked = quoted(asked),
        landing = quoted(landing),
    )
}

/// How many times the sweep has asked GitHub about the pull request — read off
/// the file [`gh_landing`] writes every question into.
fn swept(asked: &Path) -> usize {
    std::fs::read_to_string(asked)
        .unwrap_or_default()
        .lines()
        .count()
}

/// Wait until the sweep has asked at least once, or give up.
///
/// What a test that is about the *second* thing GitHub says has to wait for
/// first: writing the new answer before anything had read the old one would be a
/// test that never saw the change it was about.
async fn until_swept(asked: &Path) {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        if swept(asked) > 0 {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "the sweep never asked about the pull request",
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// A `gh` whose every answer about merging is read out of the same two files, so
/// that a base can move under the branch at the moment a test chooses and move
/// back once a resolution has been pushed.
///
/// One reading behind all three questions, because there is one fact behind
/// them: the wrap-up's own watcher reads whether the branch merges off the
/// rollup's answer, the sweep after Done asks for it on its own, and the details
/// pane asks for it beside the commits. A stub that let the three disagree would
/// be a GitHub that could not exist.
///
/// Neither file being there is a pull request that merges, which is what the
/// work is carried to Done on; `conflicting` is the base moving under it
/// afterwards, and `resolved` — which the resolution session writes as it
/// commits — is the merge that puts it right.
fn gh_conflicting_between(conflicting: &Path, resolved: &Path) -> String {
    format!(
        r#"
if [ -e {conflicting} ] && [ ! -e {resolved} ]; then merges=CONFLICTING; else merges=MERGEABLE; fi
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$5" in
mergeable,state)
    printf '{{"mergeable":"%s","state":"OPEN"}}' "$merges"
    ;;
*statusCheckRollup*)
    printf '{{"mergeable":"%s","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$merges"
    ;;
*commits*)
    printf '{{"commits":[],"comments":[],"mergeable":"%s","state":"OPEN"}}' "$merges"
    ;;
*comments*)
    printf '{{"comments":[],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#,
        conflicting = quoted(conflicting),
        resolved = quoted(resolved),
    )
}

/// A green suite, as [`gh_about`]'s answer about the checks.
const GREEN: &str = r#"    printf '{"mergeable":"MERGEABLE","statusCheckRollup":[{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}]}'"#;

/// And a suite that never finishes, which is how a test holds a wrap-up down to
/// its checks for as long as it cares to look: nothing is red, so nothing is
/// dispatched, and nothing is green either.
///
/// The pull request merges throughout, that being another of the things a
/// wrap-up waits on: one GitHub said nothing about would be a wrap-up waiting on
/// more than its suite, which is not the condition these are about.
const STILL_RUNNING: &str = r#"    printf '{"mergeable":"MERGEABLE","statusCheckRollup":[{"__typename":"CheckRun","name":"Rust","status":"IN_PROGRESS","conclusion":"","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}]}'"#;

/// A rollup that says its suite is still running until `head` is there, and then
/// a green one belonging to whichever commit that file names.
///
/// Which is how a test says *what commit GitHub thinks it is talking about*. The
/// checks and the head come back together because they are one answer, and a
/// wrap-up holds one against the other — see the tests below.
fn green_for(head: &Path) -> String {
    format!(
        r#"    if [ -s {head} ]; then
        printf '{{"mergeable":"MERGEABLE","headRefOid":"%s","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$(cat {head})"
    else
        printf '{{"mergeable":"MERGEABLE","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"IN_PROGRESS","conclusion":"","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}'
    fi"#,
        head = quoted(head),
    )
}

/// A suite that is green until `landed` is there, and a pull request reporting
/// no checks at all afterwards.
///
/// GitHub takes a commit before it creates the runs for it, so a pull request
/// that had a suite a moment ago and has none now is a push whose run has not
/// appeared — the same answer a repository with no CI gives, and not the same
/// fact.
fn green_until_nothing_is_reported(landed: &Path) -> String {
    format!(
        r#"    if [ -e {landed} ]; then
        printf '{{"mergeable":"MERGEABLE","statusCheckRollup":[]}}'
    else
{GREEN}
    fi"#,
        landed = quoted(landed),
    )
}

/// One that has gone back to running once `landed` is there — which is what a
/// commit pushed to the pull request does to it, GitHub starting a whole new run
/// against the new head.
fn green_until(landed: &Path) -> String {
    format!(
        r#"    if [ -e {landed} ]; then how=IN_PROGRESS; else how=COMPLETED; fi
    printf '{{"mergeable":"MERGEABLE","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"%s","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$how""#,
        landed = quoted(landed),
    )
}

/// A green suite on a pull request GitHub will not merge until `resolved` is
/// there, and one it will merge afterwards.
///
/// Which is how a test says *the base has moved under this branch*. The rollup
/// and the merge come back in one answer because they are one answer — the
/// watcher asks for both in the same call — so a suite that is green throughout
/// leaves the conflict as the only thing between the wrap-up and Done.
fn green_but_conflicting_until(resolved: &Path) -> String {
    format!(
        r#"    if [ -e {resolved} ]; then merges=MERGEABLE; else merges=CONFLICTING; fi
    printf '{{"mergeable":"%s","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$merges""#,
        resolved = quoted(resolved),
    )
}

/// A green suite on a pull request GitHub will never merge, which is how a test
/// spends a pull request's goes at a conflict nothing resolves.
const GREEN_BUT_CONFLICTING: &str = r#"    printf '{"mergeable":"CONFLICTING","statusCheckRollup":[{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}]}'"#;

/// And a green suite on a pull request GitHub has not worked out whether it can
/// merge — which is what it says for a while after every push, and is neither a
/// conflict nor a clean merge.
const GREEN_BUT_UNKNOWN: &str = r#"    printf '{"mergeable":"UNKNOWN","statusCheckRollup":[{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}]}'"#;

/// One whose suite is still running until `started` is there and green once it
/// is, which is how a test keeps the checks out of the way until the thing it is
/// about has begun — and then lets them settle, so that what stops the wrap-up
/// finishing is the thing being asked about and nothing else.
fn green_after(started: &Path) -> String {
    format!(
        r#"    if [ -s {started} ]; then status=COMPLETED; how=SUCCESS; else status=IN_PROGRESS; how=; fi
    printf '{{"mergeable":"MERGEABLE","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"%s","conclusion":"%s","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$status" "$how""#,
        started = quoted(started),
    )
}

/// And one that cannot be asked about the checks at all, which is how a test
/// keeps a wrap-up from settling while it watches what the comments do.
const CHECKS_UNANSWERABLE: &str = r#"    printf 'gh: To use GitHub CLI, run: gh auth login\n' >&2
    exit 1"#;

/// Three comments said in the space of a minute in the pull request's
/// conversation, which is one human making one point rather than three.
const THREE_COMMENTS: &str = r#"{"id":"IC_1","author":{"login":"tobico"},"body":"Rename the window field.","createdAt":"2026-08-21T09:00:00Z"},
{"id":"IC_2","author":{"login":"tobico"},"body":"And the test that pins it.","createdAt":"2026-08-21T09:00:20Z"},
{"id":"IC_3","author":{"login":"tobico"},"body":"Otherwise this reads well.","createdAt":"2026-08-21T09:00:40Z"}"#;

/// The comment Share to Pull Request leaves, and a human quote-replying to it.
///
/// Verkstead's own is written by the configured token — the human's own account,
/// as it usually is — so who said it tells the two apart in neither direction.
/// The marker at the start of the share's last line is the whole of it, and the
/// reply carries that same marker inside a quote, where GitHub has put a `>` in
/// front of every line of it.
///
/// The `\n` are doubled so that `printf` writes them rather than a newline: a
/// literal newline inside a JSON string is not JSON.
const A_SHARE_AND_A_REPLY: &str = r#"{"id":"IC_1","author":{"login":"tobico"},"body":"[Read this conversation](https://x/#9f1)\\n\\nA limiter that counts across instances.\\n\\n<!-- verkstead:shared-conversation -->\\n","createdAt":"2026-08-21T09:00:00Z"},
{"id":"IC_2","author":{"login":"tobico"},"body":"> [Read this conversation](https://x/#9f1)\\n>\\n> <!-- verkstead:shared-conversation -->\\n\\nWhich of these is the one to keep?","createdAt":"2026-08-21T09:05:00Z"}"#;

/// A pull request with two bots and a human on it, which is what an ignore rule
/// is written about.
///
/// `coderabbitai` files the same word about billing on every pull request,
/// because the service was set up with no billing information on it, and says
/// something worth reading beside it. `dependabot` says nothing anybody wants a
/// session for. And the human mentions billing themselves, which is the comment
/// a rule about the word alone would swallow.
const TWO_BOTS_AND_A_HUMAN: &str = r#"{"id":"IC_1","author":{"login":"coderabbitai"},"body":"Your billing information is missing.","createdAt":"2026-08-21T09:00:00Z"},
{"id":"IC_2","author":{"login":"coderabbitai"},"body":"This loop reads the vector twice.","createdAt":"2026-08-21T09:00:20Z"},
{"id":"IC_3","author":{"login":"dependabot"},"body":"Bump serde to 1.0.219.","createdAt":"2026-08-21T09:00:40Z"},
{"id":"IC_4","author":{"login":"tobico"},"body":"We should sort the billing out one day.","createdAt":"2026-08-21T09:01:00Z"}"#;

/// And the same bot leaving its billing note on a line of the diff, which is a
/// comment read through the REST endpoint rather than `pr view` and matched by
/// exactly the same rule.
const A_BOT_ON_THE_DIFF: &str = r#"{"node_id":"PRRC_9","user":{"login":"coderabbitai"},"body":"Billing information is still missing.","created_at":"2026-08-21T09:02:00Z","path":"src/window.rs","line":12}"#;

/// The one note on its own, for the test about a rule written after the wrap-up
/// had started watching.
const A_BOTS_BILLING_NOTE: &str = r#"{"id":"IC_5","author":{"login":"coderabbitai"},"body":"Your billing information is missing.","createdAt":"2026-08-21T09:10:00Z"}"#;

/// The rules the human wrote to silence the two of them: the review service by
/// what it says as well as who says it, and the dependency bot by name alone.
///
/// Two rules rather than one, which is what says the list combines with OR — and
/// the first gives both fields, which is what says a rule's own fields combine
/// with AND: the human's own word about billing is not the bot's.
const THE_BOTS_IGNORED: &str = "ignored_comments:\n  - author: coderabbitai\n    body: '(?i)billing'\n  - author: '^dependabot'\n";

/// And two left on the lines of the diff, which is where a review of code
/// mostly happens — the entries of the REST endpoint's answer, spelled its way.
const TWO_ON_THE_DIFF: &str = r#"{"node_id":"PRRC_1","user":{"login":"tobico"},"body":"This is the wrong way round.","created_at":"2026-08-21T09:03:00Z","path":"src/window.rs","line":12},
{"node_id":"PRRC_2","user":{"login":"tobico"},"body":"And this one has no home any more.","created_at":"2026-08-21T09:03:20Z","path":"src/clock.rs","line":null}"#;

/// And one whose suite is still running until `started` is there, and red once
/// it is.
///
/// For the test about what a red check does while something else holds the
/// Worktree. A suite that was red from the first poll would be racing the very
/// thing it is meant to find in there: [`crate::wrapping::watching`] spawns the
/// checks watcher and the review together, the watcher takes its first look at
/// once, and the review reads the Conversation before it asks for the Worktree —
/// so a loaded machine can have the watcher take it first and dispatch a fix
/// session, which is correct behaviour and not the one being asked about.
///
/// Still running is the answer that is neither red nor green: it dispatches
/// nothing, settles nothing, and the watcher simply looks again. So the check
/// turns red only once the file the review session writes its prompt into is
/// there — which is that session inside its sandbox, and so the Worktree already
/// taken.
fn gh_checking_after(started: &Path) -> String {
    format!(
        r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
if [ -s {started} ]; then status=COMPLETED; how=FAILURE; else status=IN_PROGRESS; how=; fi
case "$5" in
*statusCheckRollup*)
    printf '{{"mergeable":"MERGEABLE","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"%s","conclusion":"%s","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$status" "$how"
    ;;
*commits*)
    printf '{{"commits":[],"comments":[]}}'
    ;;
*comments*)
    printf '{{"comments":[],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#,
        started = quoted(started),
    )
}

/// And one that can find the pull request but cannot say anything about its
/// checks — an account whose login has expired, which is the ordinary way this
/// goes wrong on a machine nobody is sitting at.
const CHECKS_UNASKABLE: &str = r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$5" in
*statusCheckRollup*)
    printf 'gh: To use GitHub CLI, run: gh auth login\n' >&2
    exit 1
    ;;
*commits*)
    printf '{"commits":[],"comments":[]}'
    ;;
*comments*)
    printf '{"comments":[],"reviews":[]}'
    ;;
*)
    printf '{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}'
    ;;
esac
"#;

/// And a `gh` that answers for a branch nothing was opened on, in the words the
/// real one uses.
const NO_PULL_REQUEST: &str = r#"
printf 'no pull requests found for branch "%s"\n' "$3" >&2
exit 1
"#;

/// And one that finds none until `opened` is there, and finds one once it is —
/// the human going to GitHub and opening the pull request by hand, which is
/// what a halt over a missing one tells them they can do.
///
/// Everything it answers once there is one is [`PULL_REQUEST`]'s, because
/// finding it is what starts a wrap-up and a wrap-up asks about the rest.
fn gh_opened_by_hand(opened: &Path) -> String {
    format!(
        r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
if [ ! -f {opened} ]; then
    printf 'no pull requests found for branch "%s"\n' "$3" >&2
    exit 1
fi
case "$5" in
*statusCheckRollup*)
    printf '{{"mergeable":"MERGEABLE","statusCheckRollup":[]}}'
    ;;
*commits*)
    printf '{{"commits":[],"comments":[]}}'
    ;;
*comments*)
    printf '{{"comments":[],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#,
        opened = quoted(opened),
    )
}

/// And a `gh` that cannot answer anything at all, in the words the real one
/// uses: an account that was never logged in, which is the ordinary way this
/// goes wrong on a machine nobody is sitting at.
///
/// Different in kind from [`NO_PULL_REQUEST`], and that is the point of it:
/// GitHub has not said there is no pull request, it has not been asked.
const NOTHING_ASKABLE: &str = r#"
printf 'gh: To use GitHub CLI, run: gh auth login\n' >&2
exit 1
"#;

/// A `gh` that answers for two repositories at once: the Conversation's own,
/// and the companion beside it.
///
/// Told apart by the directory the call is made in, which is how the real one
/// tells them apart too — Verkstead runs `gh` in the repository it is asking
/// about, `#41` meaning something else entirely in another one. `companion` is
/// the whole of what it does when it is asked in `askance`.
///
/// The Conversation's own pull request is green and nothing has been said on
/// it, so what a wrap-up is left waiting on is the companion and nothing else.
fn gh_alongside(companion: &str) -> String {
    format!(
        r#"
case "$(pwd -P)" in
*/askance)
{companion}
    ;;
esac
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$5" in
*statusCheckRollup*)
{green}
    ;;
*commits*)
    printf '{{"commits":[],"comments":[]}}'
    ;;
*comments*)
    printf '{{"comments":[],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#,
        green = GREEN,
    )
}

/// What that companion says when the finish opened a pull request in it.
const COMPANION_PULL_REQUEST: &str = r#"    printf '{"mergeable":"MERGEABLE","number":7,"title":"The other half","url":"https://github.com/tobico/askance/pull/7"}'
    exit 0"#;

/// And what it says when the finish left it without one, in the words the real
/// `gh` uses.
const COMPANION_NO_PULL_REQUEST: &str = r#"    printf 'no pull requests found for branch "%s"\n' "$3" >&2
    exit 1"#;

/// One of those scripts as a `gh` the server can run: `sh -c` gives `$0` the
/// program's own name, so what Verkstead passes lands in `$1` onwards.
fn gh_stub(script: &str) -> Gh {
    Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        script.to_owned(),
        "gh".to_owned(),
    ])
}

/// What a fixture that puts no file on the Conversation hands
/// [`grilling_however_started`] — which is all of them but one, a file being
/// the one thing these are otherwise not about.
///
/// Named rather than written as another `&[]`, because the companions beside it
/// are one too and two empty slices in a row say nothing about which is which.
const NOTHING_ATTACHED: &[(&str, &str)] = &[];

/// Stand a workbench up with `stub` where claude goes, and press *start
/// grilling*.
async fn grilling(stub: &str) -> Grilling {
    grilling_spilling(tempfile::tempdir().unwrap(), stub, PULL_REQUEST).await
}

/// The same, grilled under an account of the second agent type — one home
/// rather than Claude's pair.
///
/// The stub stands where that backend's binary goes, as it stands where claude
/// does: what these fixtures are for is what Verkstead does around a session,
/// and a second backend's own program is the one thing they never run.
async fn grilling_on_codex(stub: &str) -> Grilling {
    grilling_however_started(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::GrillingOnCodex,
        Origin::None,
        None,
    )
    .await
}

/// The same with every role on that Profile, spilling what its sessions were
/// told somewhere that outlives their worktrees and with a `gh` of the caller's
/// choosing — which is what a wrap-up on the second backend needs.
async fn grilling_spilling_on_codex(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    grilling_however_started(
        spill,
        stub,
        gh,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::EverythingOnCodex,
        Origin::None,
        None,
    )
    .await
}

/// And the same with every role on that Profile and a stub that draws a full
/// screen where its binary goes, judged idle by the line it draws at its prompt
/// rather than by its silence.
///
/// Every role, because what these are about is what happens *around* a session
/// that repaints — the ender, the rescue, the stop — and those are the run's
/// sessions rather than the grilling one alone.
///
/// The signature is the suite's rather than a backend's: the one backend that
/// ships one draws an at-work line instead — see [`grilling_at_work`] — and the
/// backends that will draw a prompt of their own are not here yet. So what
/// stands where a backend goes is a stub drawing whatever it is told to — see
/// [`AT_THE_PROMPT`] and [`A_PROMPT_THAT_DRIFTED`].
async fn grilling_drawing(stub: &str, signature: &str) -> Grilling {
    grilling_however_started(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::EverythingOnCodex,
        Origin::None,
        Some(signature),
    )
    .await
}

/// And the same with a stub that draws the way codex draws, with nothing handed
/// in to read it by.
///
/// The difference from [`grilling_drawing`] is the whole point: there the suite
/// stands both halves — the stub's prompt and the signature Verkstead looks for
/// — because no backend ships a prompt yet. Here only the stub is the suite's,
/// and what finds its at-work line is the constant the server already carries
/// for this backend.
async fn grilling_at_work(stub: &str) -> Grilling {
    grilling_however_started(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::EverythingOnCodex,
        Origin::None,
        None,
    )
    .await
}

/// And the same again on the third backend, grilled under an account of its
/// type — one home, as the second's is.
///
/// Which is what the tests about grok's own reading of itself stand on, whether
/// that is its at-work hint or the log it keeps: a session run under another
/// type's Profile would be read by that type's constants and prove nothing about
/// either backend.
async fn grilling_on_grok(stub: &str) -> Grilling {
    grilling_however_started(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::EverythingOnGrok,
        Origin::None,
        None,
    )
    .await
}

/// And the same again on the fourth, whose account is one home as well — two
/// directories inside it rather than the directory itself, which is what
/// [`Bench::everything_on_opencode`] makes.
///
/// What the tests about opencode's own reading of itself stand on, for the
/// reason [`grilling_on_grok`] is there: a session run under another type's
/// Profile would be judged by that type's constants, and the whole claim here
/// is that this backend is judged by its own.
async fn grilling_on_opencode(stub: &str) -> Grilling {
    grilling_however_started(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::EverythingOnOpenCode,
        Origin::None,
        None,
    )
    .await
}

/// The same with every role on that Profile, spilling what its sessions were
/// told somewhere that outlives their worktrees — which is what the tests about
/// opencode's own store need, the store being written for a session from
/// outside the sandbox it is running in.
async fn grilling_spilling_on_opencode(spill: tempfile::TempDir, stub: &str) -> Grilling {
    grilling_however_started(
        spill,
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::EverythingOnOpenCode,
        Origin::None,
        None,
    )
    .await
}

/// What Verkstead is told this backend has on its Screen when it is sitting at
/// its prompt — one line, the whole of the coupling to somebody else's display.
const AT_THE_PROMPT: &str = "▌ ready for anything";

/// And what the stub draws where that wording has moved on without Verkstead: a
/// prompt that is not the one being looked for, which is a backend that renamed
/// its prompt in a release and a signature nobody has caught up with yet.
const A_PROMPT_THAT_DRIFTED: &str = "◆ what would you like to do?";

/// The model that Profile lists, which is what its sessions are launched on.
const CODEX_MODEL: &str = "gpt-5-codex";

/// And the one its grilling role runs on where every role is on that Profile.
///
/// A model apiece for the same reason the Claude fixtures give each role a
/// Profile of its own: the stubs tell the session that breaks the work down
/// from the ones that build it by the model they were launched on — see
/// [`A_BACKLOG_OF_ONE`] — and a Conversation whose roles all ran the same model
/// would be one where no stub could tell what it had been sent to do.
const CODEX_GRILLING_MODEL: &str = "gpt-5-codex-grilling";

/// The model a Grok Build Profile lists, and the one its grilling role runs on
/// where every role is on that Profile — the pair [`CODEX_MODEL`] and
/// [`CODEX_GRILLING_MODEL`] are, and for the same reason.
const GROK_MODEL: &str = "grok-4.6";

/// See [`GROK_MODEL`].
const GROK_GRILLING_MODEL: &str = "grok-4.6-grilling";

/// And the pair an OpenCode Profile lists, which are `provider/model` strings
/// because that is the whole of what opencode is told about a model — the
/// provider it comes from is the front half of the same word.
const OPENCODE_MODEL: &str = "opencode/big-pickle";

/// See [`OPENCODE_MODEL`].
const OPENCODE_GRILLING_MODEL: &str = "opencode/big-pickle-grilling";

/// The same, with a second repository registered beside this one and added to
/// the Conversation as a companion before the press — which is a companion in
/// the mode one is added in, read-only.
async fn grilling_alongside(stub: &str, companion: &str) -> Grilling {
    grilling_at_pace(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[(companion, CompanionMode::ReadOnly)],
    )
    .await
}

/// And the same with a file attached to the Conversation before the press,
/// which is the only time one can be put on: attachments freeze with the Brief.
async fn grilling_with_a_file_attached(stub: &str, name: &str, contents: &str) -> Grilling {
    grilling_however_started(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[],
        &[(name, contents)],
        Pickers::UnderEveryPairing,
        Origin::None,
        None,
    )
    .await
}

/// And the same with that companion in the other mode: a branch of its own, a
/// sandbox that may write to it, and a sweep of that branch beside the
/// Conversation's own.
async fn grilling_building_in(stub: &str, companion: &str) -> Grilling {
    grilling_at_pace(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *BRISKLY,
        &[(companion, CompanionMode::ReadWrite)],
    )
    .await
}

/// The same with the companion in the mode one is added in — read-only — and a
/// `gh` of the caller's choosing.
async fn grilling_alongside_asking(stub: &str, companion: &str, gh: &str) -> Grilling {
    grilling_at_pace(
        tempfile::tempdir().unwrap(),
        stub,
        gh,
        *BRISKLY,
        &[(companion, CompanionMode::ReadOnly)],
    )
    .await
}

/// And the same again with something else where `gh` goes, for the tests about
/// what a wrap-up makes of the pull requests a finish opened in the companion.
async fn grilling_building_in_asking(stub: &str, companion: &str, gh: &str) -> Grilling {
    grilling_at_pace(
        tempfile::tempdir().unwrap(),
        stub,
        gh,
        *BRISKLY,
        &[(companion, CompanionMode::ReadWrite)],
    )
    .await
}

/// The same, with something else where `gh` goes — for the tests about what
/// Verkstead does when GitHub cannot be asked.
async fn grilling_asking(stub: &str, gh: &str) -> Grilling {
    grilling_spilling(tempfile::tempdir().unwrap(), stub, gh).await
}

/// The same, on a server that sweeps for a stalled Conversation briskly enough
/// to watch it do so — see [`SWEEPING`].
async fn grilling_swept(stub: &str) -> Grilling {
    grilling_at_pace(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        *SWEEPING,
        &[],
    )
    .await
}

/// The same, on a server that sweeps a Done Conversation's pull requests briskly
/// enough to watch it do so — see [`LANDING`].
async fn grilling_landing(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    grilling_at_pace(spill, stub, gh, *LANDING, &[]).await
}

/// The same, over a directory the caller already has the name of — which is
/// what a stub that has to write somewhere the worktree is not needs, the
/// script naming the path being written before there is a fixture to ask.
async fn grilling_spilling(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    grilling_at_pace(spill, stub, gh, *BRISKLY, &[]).await
}

/// And the same with the Review picker moved off its Pairing and onto the row
/// that runs nothing, which is the Conversation that wraps up without a review.
async fn grilling_unreviewed(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    grilling_however_started(
        spill,
        stub,
        gh,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::Unreviewed,
        Origin::None,
        None,
    )
    .await
}

/// And the same with the *Grilling* picker moved onto its own such row, which is
/// the Conversation whose press starts the work rather than an interview: it
/// lands Implementing with a session on the Brief alone.
async fn building_ungrilled(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    grilling_however_started(
        spill,
        stub,
        gh,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::Ungrilled,
        Origin::None,
        None,
    )
    .await
}

/// How the Conversation a fixture builds was left on the setup card: every
/// picker under a Pairing, one of them moved onto its *no session* row, or the
/// grilling one moved onto an account of the second agent type.
///
/// The one thing the builders below differ over, and all of it is settled while
/// the Brief drafts — which is why it is a parameter of the build rather than
/// something a test does afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pickers {
    /// Every role under a Pairing of its own.
    UnderEveryPairing,

    /// The human picked *No grilling*, so the press starts the work.
    Ungrilled,

    /// The human picked *No review*, so the wrap-up runs none.
    Unreviewed,

    /// The grilling role runs on a Profile whose whole account is one home,
    /// which is every agent type after Claude — see [`Bench::grilling_on_codex`].
    GrillingOnCodex,

    /// And every role on that Profile, which is a Conversation whose whole run
    /// is on the second backend — see [`Bench::everything_on_codex`].
    EverythingOnCodex,

    /// The same on the third backend — see [`Bench::everything_on_grok`].
    EverythingOnGrok,

    /// And on the fourth — see [`Bench::everything_on_opencode`].
    EverythingOnOpenCode,
}

/// The same with a read-write companion beside it, for the tests about a
/// Conversation that ends on a pull request in each: the sessions spill what they
/// were told somewhere that outlives their worktrees, and `gh` answers for both
/// repositories.
async fn grilling_spilling_alongside(
    spill: tempfile::TempDir,
    stub: &str,
    companion: &str,
    gh: &str,
) -> Grilling {
    grilling_at_pace(
        spill,
        stub,
        gh,
        *BRISKLY,
        &[(companion, CompanionMode::ReadWrite)],
    )
    .await
}

/// And the same again at a pace of the caller’s choosing, alongside whatever
/// `companions` names — no repository at all being the ordinary Conversation.
async fn grilling_at_pace(
    spill: tempfile::TempDir,
    stub: &str,
    gh: &str,
    pace: Pace,
    companions: &[(&str, CompanionMode)],
) -> Grilling {
    grilling_however_started(
        spill,
        stub,
        gh,
        pace,
        companions,
        NOTHING_ATTACHED,
        Pickers::UnderEveryPairing,
        Origin::None,
        None,
    )
    .await
}

/// And the same with somewhere to push to, for the tests about a wrap-up holding
/// a rollup against the commit that was pushed.
///
/// Every other fixture here is a checkout with no remote at all, which is one of
/// the two ways [`checks`](verkstead_server) has of not being able to tell — so
/// none of them asks the question these are about.
async fn grilling_pushing(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    grilling_however_started(
        spill,
        stub,
        gh,
        *BRISKLY,
        &[],
        NOTHING_ATTACHED,
        Pickers::UnderEveryPairing,
        Origin::Cloned,
        None,
    )
    .await
}

/// Whether the repository a fixture builds has a remote.
///
/// A bare clone inside the spill directory, so that a session in a sandbox can
/// reach it: the spill is the one path outside the worktrees every sandbox here
/// is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// No remote at all, which is what a checkout made for a test is.
    None,

    /// A bare clone to push to, at `upstream` inside the spill directory.
    Cloned,
}

/// And the whole of it, the pickers included — which is the setup card pressed
/// the way the human presses it, every picker filled and then one of them moved.
#[allow(clippy::too_many_arguments)]
async fn grilling_however_started(
    spill: tempfile::TempDir,
    stub: &str,
    gh: &str,
    pace: Pace,
    companions: &[(&str, CompanionMode)],
    attaching: &[(&str, &str)],
    pickers: Pickers,
    origin: Origin,
    signature: Option<&str>,
) -> Grilling {
    let bench = bench_at_pace(spill, stub, gh, pace, signature).await;
    let app = &bench.app;

    // Before the Conversation is started, so that every worktree cut from this
    // repository has the remote — a worktree shares the repository's `.git`, and
    // what it shares is where a remote lives.
    if origin == Origin::Cloned {
        let upstream = bench.spill.path().join("upstream");

        git(
            bench.spill.path(),
            &[
                "clone",
                "--no-local",
                "--bare",
                "--quiet",
                &bench.repo.to_string_lossy(),
                &upstream.to_string_lossy(),
            ],
        );
        git(
            &bench.repo,
            &["remote", "add", "origin", &upstream.to_string_lossy()],
        );
        git(&bench.repo, &["fetch", "--quiet", "origin"]);
    }

    let started: Started = post(
        app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": bench.repo_id }),
    )
    .await;
    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    bench.under_every_pairing(id).await;

    match pickers {
        Pickers::UnderEveryPairing => {}
        Pickers::Ungrilled => bench.ungrilled(id).await,
        Pickers::Unreviewed => bench.unreviewed(id).await,
        Pickers::GrillingOnCodex => bench.grilling_on_codex(id).await,
        Pickers::EverythingOnCodex => bench.everything_on_codex(id).await,
        Pickers::EverythingOnGrok => bench.everything_on_grok(id).await,
        Pickers::EverythingOnOpenCode => bench.everything_on_opencode(id).await,
    }

    // While it is still drafting, which is the only time a companion can be
    // added or configured — and off the same endpoints the setup card presses.
    for (name, mode) in companions {
        let repo_id = bench.register(name).await;

        let added: CompanionAdded = post(
            app,
            &format!("/api/ui/conversations/{id}/companions"),
            &serde_json::json!({ "repo_id": repo_id }),
        )
        .await;
        assert_eq!(added, CompanionAdded::Added);

        // Only where it is not the mode one is added in, so the read-only case
        // still goes through exactly the presses it always did.
        if *mode != CompanionMode::ReadOnly {
            let chosen: CompanionModeChosen = post(
                app,
                &format!("/api/ui/conversations/{id}/companions/{repo_id}/mode"),
                &serde_json::json!({ "mode": mode }),
            )
            .await;
            assert_eq!(chosen, CompanionModeChosen::Chosen);
        }
    }

    let saved: BriefSaved = post(
        app,
        &format!("/api/ui/conversations/{id}/brief"),
        &serde_json::json!({ "markdown": BRIEF }),
    )
    .await;
    assert_eq!(saved, BriefSaved::Saved);

    // And whatever the human attached alongside it, off the same route the
    // paperclip presses: the raw bytes as the body and the name in the path.
    // Before the press for the companions' reason — attachments freeze with the
    // Brief, so a draft is the only time one can be put on.
    for (name, contents) in attaching {
        let attached: Attached = upload(
            app,
            &format!("/api/ui/conversations/{id}/attachments/{name}"),
            contents,
        )
        .await;

        let Attached::Attached { attachment } = attached else {
            panic!("expected {name} to be attached, got {attached:?}");
        };

        assert_eq!(attachment.name, *name);
    }

    let grilling: GrillingStarted = post(
        app,
        &format!("/api/ui/conversations/{id}/grill"),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(grilling, GrillingStarted::Started);

    bench.holding(id)
}

/// A workbench with a stub where claude goes and a registered repository in it,
/// before any Conversation has been started against it.
///
/// The two ways in start from here and part company at the press: one writes a
/// Brief and grills, and the other adopts a roadmap the repository already
/// holds — see [`adopting`].
struct Bench {
    watched: tempfile::TempDir,
    state: tempfile::TempDir,
    home: tempfile::TempDir,
    spill: tempfile::TempDir,
    app: Router,
    database: PathBuf,

    /// The registered repository: where it is on disk, and the id a Conversation
    /// is started against.
    repo: PathBuf,
    repo_id: i64,

    /// Held from the moment this bench was built and handed on to the
    /// [`Grilling`] it becomes — see [`ROOM`].
    room: tokio::sync::OwnedSemaphorePermit,
}

impl Bench {
    /// Fix every Pairing on a Conversation, which is what every one of these
    /// has to have settled before anything will start in it.
    ///
    /// Each role gets a Profile of its own, paired with the first of the models
    /// that Profile lists — see [`profile`], which lists two so that a pick can
    /// move off this one without moving off the Profile.
    async fn under_every_pairing(&self, id: i64) {
        for role in ["grilling", "implementation", "review"] {
            let profile = profile(&self.app, self.watched.path(), role).await;
            let pairing = serde_json::json!({
                "profile_id": profile,
                "model": format!("claude-{role}-5"),
            });

            // Two of the pickers offer a row that is no account at all, so what
            // they send is which of their rows was picked — see
            // [`Bench::ungrilled`] and [`Bench::unreviewed`].
            let picked = match role {
                "grilling" | "review" => serde_json::json!({ "pairing": pairing }),
                _ => pairing,
            };

            let chosen: verkstead_render::ProfileChosen = post(
                &self.app,
                &format!("/api/ui/conversations/{id}/{role}-pairing"),
                &picked,
            )
            .await;
            assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
        }
    }

    /// And pick the grilling role under an account of the second agent type,
    /// whose whole account is one home rather than Claude's pair.
    ///
    /// Pressed after [`Bench::under_every_pairing`] for the reason
    /// [`Bench::unreviewed`] is: it is the picker moved off what it was filled
    /// with, which is what the human does on the setup card.
    async fn grilling_on_codex(&self, id: i64) {
        self.on_codex(id, &[("grilling", CODEX_MODEL)]).await;
    }

    /// And the same for every role at once, which is what a Conversation whose
    /// whole run is on the second backend looks like.
    ///
    /// What the tests about the store-and-nudge channel want: the sessions that
    /// ask and are ended on quiet are the wrap-up's, so putting only the
    /// grilling role on that backend would leave every ask of theirs blocking.
    ///
    /// The grilling role on a model of its own, because one Profile is running
    /// every role here and the stubs tell the session that breaks the work down
    /// from the ones that build it by the model — see [`CODEX_GRILLING_MODEL`].
    async fn everything_on_codex(&self, id: i64) {
        self.on_codex(
            id,
            &[
                ("grilling", CODEX_GRILLING_MODEL),
                ("implementation", CODEX_MODEL),
                ("review", CODEX_MODEL),
            ],
        )
        .await;
    }

    /// And every role on a Grok Build Profile, which is the same again on the
    /// third backend.
    ///
    /// Every role for the reason [`Bench::everything_on_codex`] is, and the
    /// grilling one on a model of its own for the reason it is there: one
    /// Profile runs the lot, and the stubs tell the session that breaks the
    /// work down from the ones that build it by the model.
    async fn everything_on_grok(&self, id: i64) {
        self.on_one_home(
            id,
            "Grok",
            "grok",
            &[],
            &[GROK_MODEL, GROK_GRILLING_MODEL],
            &[
                ("grilling", GROK_GRILLING_MODEL),
                ("implementation", GROK_MODEL),
                ("review", GROK_MODEL),
            ],
        )
        .await;
    }

    /// And every role on an OpenCode Profile, which is the same again on the
    /// fourth backend.
    ///
    /// The two directories are what makes that home an opencode account: this
    /// type's home is judged by what opencode keeps an account in rather than
    /// by the directory holding them, so a home without them is a Profile the
    /// form refuses to save.
    async fn everything_on_opencode(&self, id: i64) {
        self.on_one_home(
            id,
            "OpenCode",
            "opencode",
            &[".config/opencode", ".local/share/opencode"],
            &[OPENCODE_MODEL, OPENCODE_GRILLING_MODEL],
            &[
                ("grilling", OPENCODE_GRILLING_MODEL),
                ("implementation", OPENCODE_MODEL),
                ("review", OPENCODE_MODEL),
            ],
        )
        .await;
    }

    /// The Profile both of the Codex pickers pick, and the pressing of the
    /// pickers named, each on the model it is paired with.
    async fn on_codex(&self, id: i64, roles: &[(&str, &str)]) {
        self.on_one_home(
            id,
            "Codex",
            "codex",
            &[],
            &[CODEX_MODEL, CODEX_GRILLING_MODEL],
            roles,
        )
        .await;
    }

    /// Saving a Profile of an agent type whose whole account is one home, and
    /// pressing the pickers named onto it.
    ///
    /// One body for every such type — which is every one after Claude — because
    /// what differs between them is the word in the type and the directory the
    /// account keeps, and a second copy of this would be a second place to
    /// forget one of them.
    ///
    /// `inside` is what has to be *in* that home for it to be an account of
    /// this type, and it is the last of the differences between them: none for
    /// the two whose home is the whole of the account, and the two directories
    /// opencode keeps an account in for an OpenCode one.
    async fn on_one_home(
        &self,
        id: i64,
        agent_type: &str,
        name: &str,
        inside: &[&str],
        models: &[&str],
        roles: &[(&str, &str)],
    ) {
        let home = self.watched.path().join(name).join(format!(".{name}"));
        std::fs::create_dir_all(&home).unwrap();

        for directory in inside {
            std::fs::create_dir_all(home.join(directory)).unwrap();
        }

        let saved: ProfileSaved = post(
            &self.app,
            "/api/ui/profiles",
            &serde_json::json!({
                "name": name,
                "account": { "agent_type": agent_type, "home": home },
                "models": models,
            }),
        )
        .await;
        assert_eq!(saved, ProfileSaved::Saved);

        let profiles: Vec<verkstead_render::ProfileEntry> =
            get(&self.app, "/api/ui/profiles").await;
        let profile_id = profiles
            .into_iter()
            .find(|profile| profile.name == name)
            .expect("the Profile just saved should be on the list")
            .id;

        for (role, model) in roles {
            let pairing = serde_json::json!({ "profile_id": profile_id, "model": model });

            // The two pickers that offer a row which is no account at all send
            // which row was picked, exactly as [`Bench::under_every_pairing`]
            // does.
            let picked = match *role {
                "grilling" | "review" => serde_json::json!({ "pairing": pairing }),
                _ => pairing,
            };

            let chosen: verkstead_render::ProfileChosen = post(
                &self.app,
                &format!("/api/ui/conversations/{id}/{role}-pairing"),
                &picked,
            )
            .await;
            assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
        }
    }

    /// And pick the Grilling picker's other row instead: this Conversation is
    /// not to be grilled at all, so the press starts the work rather than an
    /// interview.
    ///
    /// Pressed after [`Bench::under_every_pairing`] for the reason
    /// [`Bench::unreviewed`] is.
    async fn ungrilled(&self, id: i64) {
        let chosen: verkstead_render::ProfileChosen = post(
            &self.app,
            &format!("/api/ui/conversations/{id}/grilling-pairing"),
            &serde_json::json!({ "pairing": null }),
        )
        .await;
        assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
    }

    /// And pick the Review picker's other row instead: this Conversation is not
    /// to be reviewed at all.
    ///
    /// Pressed after [`Bench::under_every_pairing`] rather than instead of it,
    /// which is how the card is used: the picker arrives filled and the human
    /// moves it to the row that runs nothing.
    async fn unreviewed(&self, id: i64) {
        let chosen: verkstead_render::ProfileChosen = post(
            &self.app,
            &format!("/api/ui/conversations/{id}/review-pairing"),
            &serde_json::json!({ "pairing": null }),
        )
        .await;
        assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
    }

    /// Register a second repository under the watched directory, and hand back
    /// the id a Conversation would add it as a companion by.
    ///
    /// A repository of its own rather than a checkout of this one: what a
    /// companion is, is another registered Repo, and two Repos over one
    /// directory is a different thing entirely.
    async fn register(&self, name: &str) -> i64 {
        let path = repository(self.watched.path().join(name));
        let registered: Registered = post(
            &self.app,
            "/api/ui/repos",
            &serde_json::json!({ "path": path }),
        )
        .await;
        assert_eq!(registered, Registered::Added);

        let repos: Vec<verkstead_render::RepoEntry> = get(&self.app, "/api/ui/repos").await;

        repos
            .into_iter()
            .find(|repo| repo.name == name)
            .expect("the repository was just registered")
            .id
    }

    /// The fixture the tests read, once there is a Conversation running in it.
    fn holding(self, id: i64) -> Grilling {
        Grilling {
            _watched: self.watched,
            home: self.home,
            state: self.state,
            spill: self.spill,
            app: self.app,
            id,
            database: self.database,
            _room: self.room,
        }
    }
}

/// How many of these fixtures may be standing at once.
///
/// Each one is a real server, a real repository on disk and a run of real
/// sessions in real sandboxes, and `cargo test` will happily start every test
/// in this file at once — which on this suite is a hundred and more benches
/// competing for two cores. What comes out of that is not a slow run but a
/// wrong one: sessions descheduled past the budgets they are being judged
/// against, and a different test failing every time.
///
/// Twice the cores the machine admits to, and never fewer than four or more
/// than sixteen. These fixtures are mostly waiting — on a process starting, on
/// a poll coming round — so more of them than cores is right; what broke was
/// having no ceiling at all. The floor keeps a single-core machine from running
/// the suite one test at a time, and the ceiling keeps a large machine from
/// putting the load back.
///
/// Taken in [`bench_at_pace`], which every fixture in this file is built
/// through, and held for the fixture's whole life. It bounds how far load can
/// stretch the clock in the first place; [`PACE`] is what copes with the
/// stretch that is left.
static ROOM: LazyLock<Arc<tokio::sync::Semaphore>> = LazyLock::new(|| {
    let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());

    Arc::new(tokio::sync::Semaphore::new((cores * 2).clamp(4, 16)))
});

async fn bench(spill: tempfile::TempDir, stub: &str, gh: &str) -> Bench {
    bench_at_pace(spill, stub, gh, *BRISKLY, None).await
}

/// The same, at a pace of the caller’s choosing — which is what the tests about
/// the stall sweep need, that being the one thing [`BRISKLY`] deliberately keeps
/// slow.
async fn bench_at_pace(
    spill: tempfile::TempDir,
    stub: &str,
    gh: &str,
    pace: Pace,
    signature: Option<&str>,
) -> Bench {
    // Before anything is built, so that a bench queued behind the suite's
    // ceiling costs nothing while it waits — see [`ROOM`].
    let room = ROOM
        .clone()
        .acquire_owned()
        .await
        .expect("the suite's room is never closed");

    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // Who a session commits as: a settings file in the Data Directory, which is
    // where every sandbox is configured out of.
    std::fs::write(state.path().join("config.yaml"), THE_AUTHOR).unwrap();

    let database = state.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();

    let agents = Agents::running(
        vec!["/bin/sh".to_owned(), "-c".to_owned(), stub.to_owned()],
        Homes::on(Platform::HERE, home.path().to_owned(), state.path()),
        Reachable::at(LISTENING),
        SandboxConfig::resolve(&[spill.path().display().to_string()]).unwrap(),
        BuildCache::none(),
        Skills::installed(state.path()).expect("this binary carries skills"),
        equipped(state.path()),
        Handoffs::under(state.path()),
        Attachments::under(state.path()),
        Settings::in_data_dir(state.path()),
    )
    .at_pace(pace);

    // And, where this fixture's stub draws a full screen rather than printing
    // lines, the prompt it draws when its turn is over — which stands where a
    // backend's own signature goes, for the backends that will draw one. A
    // fixture that hands in nothing leaves the server reading its own, which is
    // how the Codex ones are written; see the server's `sessions` module.
    let agents = match signature {
        Some(signature) => agents.drawing(signature),
        None => agents,
    };

    let app = router_running_sessions(
        pool,
        WatchedPaths::resolve(&[watched.path().to_owned()]).unwrap(),
        state.path().to_owned(),
        agents,
        gh_stub(gh),
    );

    let repo = repository(watched.path().join("verkstead"));
    let registered: Registered =
        post(&app, "/api/ui/repos", &serde_json::json!({ "path": repo })).await;
    assert_eq!(registered, Registered::Added);

    let repos: Vec<verkstead_render::RepoEntry> = get(&app, "/api/ui/repos").await;
    let repo_id = repos[0].id;

    Bench {
        watched,
        state,
        home,
        spill,
        app,
        database,
        repo,
        repo_id,
        room,
    }
}

/// The one agent-output Event on a Timeline, where there is one yet.
fn output(view: &ConversationView) -> Option<&AgentOutputEvent> {
    outputs(view).into_iter().next()
}

/// Where a Conversation had got to, for the assertion that gave up waiting for
/// it to get somewhere else.
///
/// Every session it has run and every Notice on it, rather than the first
/// session alone. A run that stops short is nearly always one whose *second*
/// session never started or never said what was expected, and a message naming
/// only the first says the same thing whichever of those it was — which is a
/// failure that has to be reproduced before it can be read, and this suite is
/// slower to reproduce on some machines than on others.
fn standing(view: &ConversationView) -> String {
    let sessions: Vec<String> = outputs(view)
        .iter()
        .map(|output| {
            format!(
                "#{} {:?} ({} lines{})",
                output.id,
                output.latest,
                output.lines,
                if output.running { ", running" } else { "" },
            )
        })
        .collect();

    let notices: Vec<String> = said(view)
        .iter()
        .map(|notice| format!("#{} {:?}", notice.id, notice.html))
        .collect();

    format!(
        "state {:?}, blocked_on {:?}; sessions [{}]; notices [{}]",
        view.state,
        view.blocked_on,
        sessions.join(", "),
        notices.join(", "),
    )
}

/// All of them, in order — a Conversation has one per session it has run, and
/// the inline direction is where it gets its second.
fn outputs(view: &ConversationView) -> Vec<&AgentOutputEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::AgentOutput(output) => Some(output),
            _ => None,
        })
        .collect()
}

/// The commits on a Timeline, in Timeline order — which is the order they
/// landed on the branch.
fn commits(view: &ConversationView) -> Vec<&CommitEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Commit(commit) => Some(commit),
            _ => None,
        })
        .collect()
}

/// The Question Sets on a Timeline, in the order they were asked.
fn sets(view: &ConversationView) -> Vec<&verkstead_render::QuestionSetEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::QuestionSet(asked) => Some(asked),
            _ => None,
        })
        .collect()
}

/// The backlog pinned to a Conversation, where its Worktree holds one.
fn backlog(view: &ConversationView) -> Option<&TaskListEvent> {
    view.pinned.iter().find_map(|pinned| match pinned {
        PinnedEvent::TaskList(list) => Some(list),
        _ => None,
    })
}

/// And the same backlog on the record, at the row that says it landed on the
/// branch.
///
/// The other of the two places a task list is drawn. Its content is the same
/// live reading the pinned copy is drawn from — the server takes it once and
/// hands it over twice — so what this adds is where it landed.
fn backlog_row(view: &ConversationView) -> Option<&TaskListReached> {
    view.timeline.iter().find_map(|event| match event {
        TimelineEvent::TaskList(reached) => Some(reached),
        _ => None,
    })
}

/// And the roadmap on the record, at the row that says it landed.
fn roadmap_row(view: &ConversationView) -> Option<&StageListReached> {
    view.timeline.iter().find_map(|event| match event {
        TimelineEvent::StageList(reached) => Some(reached),
        _ => None,
    })
}

/// And the pull request pinned beside it, once the finish step has opened one.
fn pull_request(view: &ConversationView) -> Option<&PullRequestEvent> {
    view.pinned.iter().find_map(|pinned| match pinned {
        PinnedEvent::PullRequest(opened) => Some(opened),
        _ => None,
    })
}

/// And every pull request pinned, which is one per repository the work was
/// carried to: the Conversation's own, and one for each companion it committed
/// in.
fn pull_requests(view: &ConversationView) -> Vec<&PullRequestEvent> {
    view.pinned
        .iter()
        .filter_map(|pinned| match pinned {
            PinnedEvent::PullRequest(opened) => Some(opened),
            _ => None,
        })
        .collect()
}

/// A Conversation's own directory outside its worktree, as the host sees it —
/// the far side of the `/tmp/verkstead` a session writes its handoff into.
///
/// What the tests put in there is a marker a stub is waiting on: a stub cannot
/// idle on a blocking ask, so this is how one is held at a point the test needs
/// it held at.
fn handoff_directory(fixture: &Grilling) -> PathBuf {
    fixture
        .state
        .path()
        .join("handoffs")
        .join(fixture.id.to_string())
}

/// The handoff on a Timeline, once the grilling has handed one over.
fn handoff(view: &ConversationView) -> Option<&verkstead_render::HandoffEvent> {
    view.timeline.iter().find_map(|event| match event {
        TimelineEvent::Handoff(handoff) => Some(handoff),
        _ => None,
    })
}

/// An Agent Profile saved from a pair inside `watched`, on models that are
/// worth reading back.
///
/// Two of them, `claude-<name>-5` and `claude-<name>-4.8`. The first is what
/// every Conversation here is paired with; the second is there so that a pick
/// can change the *model* under a Profile that does not change, which is a
/// different pick and not the same one made twice.
async fn profile(app: &Router, watched: &Path, name: &str) -> i64 {
    let claude_dir = watched.join(name).join(".claude");
    let config_file = watched.join(name).join(".claude.json");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    let saved: ProfileSaved = post(
        app,
        "/api/ui/profiles",
        &serde_json::json!({
            "name": name,
            "account": {
                "agent_type": "Claude",
                "claude_dir": claude_dir,
                "config_file": config_file,
            },
            "models": [format!("claude-{name}-5"), format!("claude-{name}-4.8")],
        }),
    )
    .await;
    assert_eq!(saved, ProfileSaved::Saved);

    let profiles: Vec<verkstead_render::ProfileEntry> = get(app, "/api/ui/profiles").await;
    profiles
        .into_iter()
        .find(|profile| profile.name == name)
        .expect("the Profile just saved should be on the list")
        .id
}

/// A git repository at `path`, with one commit on `main`.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "test@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Test"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "README.md"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

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

/// A path as one shell word, for the stubs that name a directory.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

async fn get<T: DeserializeOwned>(app: &Router, path: &str) -> T {
    let (status, body) = fetch(
        app,
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "GET {path} failed: {body}");
    read(&body)
}

async fn post<T: DeserializeOwned>(app: &Router, path: &str, body: &serde_json::Value) -> T {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(body).unwrap()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "POST {path} failed: {body}");
    read(&body)
}

/// A file put on a Conversation: the raw bytes as the body, and no content type
/// at all — which is what the composer sends and what the route reads.
async fn upload<T: DeserializeOwned>(app: &Router, path: &str, body: &str) -> T {
    let (status, body) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(path)
            .body(Body::from(body.to_owned()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "POST {path} failed: {body}");
    read(&body)
}

async fn fetch(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn read<T: DeserializeOwned>(body: &str) -> T {
    serde_json::from_str(body).unwrap_or_else(|err| panic!("reading {body:?}: {err}"))
}

/// And what a Conversation with a companion repo tells its grilling session:
/// the same prompt, with the companion named under it.
///
/// The grilling session and not one of the ones that build, because it is the
/// one whose prompt is built nowhere near the rest — if the listing reaches
/// this one, it reaches them by the same line.
#[tokio::test]
async fn a_grilling_session_is_told_about_the_companion_repos_too() {
    let fixture = grilling_alongside(r#"printf 'prompt=%s' "$2""#, "askance").await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains(BRIEF),
        "the Brief is still what the grilling starts from: {said:?}"
    );
    assert!(
        said.contains("# Companion repositories"),
        "and the companion is named under it: {said:?}"
    );
    assert!(
        said.contains("`askance` at `"),
        "with where it was checked out: {said:?}"
    );

    // The commit it is detached at rather than the branch that was picked: the
    // two are the same thing only on the day, and a session told it was on
    // `main` would be told something the next push makes untrue.
    let at = fixture.view().await.companions[0]
        .base_commit
        .clone()
        .expect("a checked-out companion says what its base came to");

    assert!(
        said.contains(&format!("detached at `{at}`, read-only.")),
        "and what it holds and whether it may be written to: {said:?}"
    );
}

/// And what a Conversation with a file attached tells its grilling session: the
/// same prompt, with the file named under it at the path its sandbox puts it at.
///
/// The grilling session for the companions listing's reason — it is the one
/// whose prompt is built nowhere near the rest, so a section that reaches this
/// one reaches the wrap-up's own by the same line.
#[tokio::test]
async fn a_grilling_session_is_told_about_the_attached_files_too() {
    let fixture =
        grilling_with_a_file_attached(r#"printf 'prompt=%s' "$2""#, "rates.csv", "a,b\n1,2\n")
            .await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains(BRIEF),
        "the Brief is still what the grilling starts from: {said:?}"
    );
    assert!(
        said.contains("# Attached files"),
        "and the file is named under it: {said:?}"
    );
    assert!(
        said.contains("Attached to the Brief:"),
        "under what it was attached to: {said:?}"
    );
    assert!(
        said.contains("- `/verkstead/attachments/rates.csv`, 8 bytes."),
        "at the path the sandbox puts it at, with its size: {said:?}"
    );
}

/// The whole of what pressing the button now does: the Profile's agent, on the
/// Profile's model, running in the Conversation's worktree, sent into the
/// bundled grilling skill and primed with the Brief.
#[tokio::test]
async fn a_session_runs_the_grilling_profiles_agent_on_the_brief_in_the_worktree() {
    let fixture = grilling(
        r#"
        printf 'model=%s\n' "$1"
        printf 'where=%s\n' "$(pwd)"
        printf 'prompt=%s' "$2"
        "#,
    )
    .await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let said = fixture.capture(event).await;
    let said = said.replace("\r\n", "\n");

    assert!(
        said.contains("model=claude-grilling-5"),
        "the grilling Profile's model is what the session runs on, not the implementation one's: {said:?}"
    );
    assert!(
        said.contains("/verkstead/skills/grilling/SKILL.md"),
        "the session is sent into the bundled grilling skill by the prompt, there being no \
         global CLAUDE.md inside to say what it is for: {said:?}"
    );
    assert!(
        said.contains(BRIEF),
        "and the Brief is what the grilling starts from: {said:?}"
    );

    let worktree = PathBuf::from(
        fixture
            .view()
            .await
            .worktree
            .expect("a grilling Conversation has a worktree")
            .path,
    );

    assert!(
        said.contains(&format!(
            "where={}\n",
            worktree.canonicalize().unwrap().display()
        )),
        "a session works in its Conversation's worktree and nowhere else: {said:?}"
    );

    // And the Event it printed into says what it ran under, stamped as the
    // Capture was opened: the pairing is on the record rather than read off the
    // Conversation afterwards, so a Profile renamed or repicked later leaves
    // this saying what actually ran.
    let stamped = output(&fixture.view().await)
        .expect("the session printed into an Event")
        .clone();
    assert_eq!(
        (stamped.profile.as_deref(), stamped.model.as_deref()),
        (Some("grilling"), Some("claude-grilling-5")),
        "the grilling Profile and the model it was launched on"
    );
}

/// Running unattended is Verkstead's own doing: the bypass flag is on the line
/// it launches the session with, rather than something the Profile's account
/// was hoped to have been configured to hold.
///
/// The stub reads its whole line back, which is how this is provable without an
/// account: what a real claude does with the flag is claude's business, and what
/// could be wrong here is Verkstead's end of it. It reads `$1` and `$2` in the
/// same breath, because the flag going on the end is the half of the promise
/// that keeps every other stub in this file reading what it reads today.
#[tokio::test]
async fn a_session_is_launched_with_the_bypass_flag_verkstead_passes_it() {
    let fixture = grilling(
        r#"
        printf 'model=%s\n' "$1"
        printf 'prompt=%s\n' "$2"
        for arg in "$@"; do
            if [ "$arg" = --dangerously-skip-permissions ]; then printf 'unattended\n'; fi
        done
        "#,
    )
    .await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains("unattended\n"),
        "the agent should have been launched with the flag that stops it asking \
         approval of nobody: {said:?}"
    );
    assert!(
        said.contains("model=claude-grilling-5\n"),
        "and the model is still the first thing after the program: {said:?}"
    );
    assert!(
        said.contains(BRIEF),
        "and the Brief is still the one after that: {said:?}"
    );
}

/// A Conversation is started on a branch name Verkstead invented, so its first
/// session is told to pick a real one — and the record follows the name it
/// picks, which is what stops it being called a Draft.
///
/// The whole of the loop in one press: the instruction goes out under the Brief,
/// the session renames the branch in its own worktree, and Verkstead reads the
/// rename off the checkout rather than being told about it.
#[tokio::test]
async fn a_first_session_is_told_to_name_the_branch_and_is_followed_to_the_name() {
    let fixture = grilling(
        r#"
        printf 'prompt=%s' "$2"
        git branch -m rate-limiting
        "#,
    )
    .await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains(BRIEF),
        "the Brief is still what the grilling starts from: {said:?}"
    );
    assert!(
        said.contains("# This branch has no name yet"),
        "and under it, the one thing to do before anything lands on the branch: {said:?}"
    );

    let branch = fixture
        .until(|view| (!view.naming).then(|| view.branch.clone()))
        .await;

    assert_eq!(
        branch, "rate-limiting",
        "the record follows the session to the name it picked",
    );
}

/// And a first session that reads the instruction and leaves the name alone
/// settles it: the name it was given is the Conversation's from then on.
///
/// Otherwise the Conversation would read *Draft* for the rest of its life, on
/// the strength of a session that has been and gone.
#[tokio::test]
async fn a_first_session_that_renames_nothing_settles_for_the_name_it_was_given() {
    let fixture = grilling(r#"printf 'prompt=%s' "$2""#).await;

    let prefilled = fixture.view().await.branch;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains("# This branch has no name yet"),
        "the instruction went out all the same: {said:?}"
    );

    let branch = fixture
        .until(|view| (!view.naming).then(|| view.branch.clone()))
        .await;

    assert_eq!(branch, prefilled, "settling for a name is not changing it",);
}

/// The terminal a session is on, asked from inside the sandbox.
///
/// Verkstead opens the pair and starts the sandbox on it — see
/// [`verkstead_server::terminal`] — so all three of a session's streams are the
/// one terminal, it is a hundred columns by thirty, and it is told what kind of
/// terminal that is. Nothing said the last of those before, and what an agent's
/// interface draws depends on what it thinks it has.
///
/// Read back with `stty`, which is a process asking the terminal underneath it
/// rather than reading a variable something set: `COLUMNS` and `LINES` are not
/// exported, deliberately, because a number in the environment is a copy that
/// stops being true the moment a watcher resizes the window.
#[tokio::test]
async fn a_session_runs_on_a_terminal_verkstead_opened_for_it() {
    let fixture = grilling(
        r#"
        printf 'term=%s\n' "$TERM"
        printf 'size=%s\n' "$(stty size)"
        if [ -t 0 ] && [ -t 1 ] && [ -t 2 ]; then
            printf 'streams=all three are the terminal\n'
        else
            printf 'streams=0:%s 1:%s 2:%s\n' "$(test -t 0 && echo tty)" "$(test -t 1 && echo tty)" "$(test -t 2 && echo tty)"
        fi
        "#,
    )
    .await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains("streams=all three are the terminal\n"),
        "a session's stdin, stdout and stderr are the one terminal, so what the \
         sandbox complains about lands where the session printed: {said:?}"
    );
    assert!(
        said.contains("size=30 100\n"),
        "and it is a hundred columns by thirty until somebody watching says \
         otherwise: {said:?}"
    );
    assert!(
        said.contains("term=xterm-256color\n"),
        "and it is told what kind of terminal it is on: {said:?}"
    );
}

/// Verkstead names a session before it starts it, so that the log the agent
/// keeps of its own conversation is a lookup rather than a guess.
///
/// Three things have to be true together, and separately they prove nothing: the
/// name reaches the agent, the same name is written down beside the session's
/// Event, and a file named for it inside the sandbox lands under the Agent
/// Profile's own directory on the host — which is where the log will be looked
/// for. The stub writes that file where claude writes its session log, because
/// what could actually be wrong here is Verkstead's end of it.
#[tokio::test]
async fn a_session_is_named_before_it_starts_and_writes_its_log_under_that_name() {
    let fixture = grilling(
        r#"
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        printf 'named=%s\n' "$name"

        mkdir -p "$HOME/.claude/projects/stub"
        printf '' > "$HOME/.claude/projects/stub/$name.jsonl"
        "#,
    )
    .await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let pool = open_database(&fixture.database).await.unwrap();
    let name = verkstead_store::session_id(&pool, event)
        .await
        .unwrap()
        .expect("Verkstead should have written down what it named the session");

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains(&format!("named={name}\n")),
        "the session should have been run under the name Verkstead recorded for it: {said:?}"
    );

    let log = fixture
        ._watched
        .path()
        .join("grilling/.claude/projects/stub")
        .join(format!("{name}.jsonl"));

    assert!(
        log.is_file(),
        "a log named for the session should land under the grilling Profile's own \
         directory, at {}",
        log.display()
    );
}

/// The log a session keeps of its own conversation is followed while the session
/// runs, and every line of it is kept exactly as it was written.
///
/// The stub writes its log where claude writes one, and writes one of its lines
/// in two halves with a wait in between — which is a poll landing mid-line, and
/// the thing a tailer that stored whatever was there would get wrong. It is all
/// read back while the session is still going, because a Transcript that only
/// turned up once the session was over would be a details pane nobody could
/// watch.
#[tokio::test]
async fn a_sessions_own_log_is_followed_line_by_line_while_it_runs() {
    let fixture = grilling(
        r#"
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        log=$HOME/.claude/projects/verkstead/$name.jsonl
        mkdir -p "$(dirname "$log")"

        printf '{"type":"user","text":"Rate limiting"}\n' > "$log"
        printf 'Reading the brief.\n'

        printf '{"type":"assistant","te' >> "$log"
        sleep 2
        printf 'xt":"Where does the counter live?"}\n' >> "$log"
        printf 'Asking.\n'

        sleep 300
        "#,
    )
    .await;

    let event = fixture.until(|view| output(view).map(|o| o.id)).await;
    let transcript = fixture.transcript_of(event, 2).await;

    assert_eq!(
        transcript,
        vec![
            r#"{"type":"user","text":"Rate limiting"}"#.to_owned(),
            r#"{"type":"assistant","text":"Where does the counter live?"}"#.to_owned(),
        ],
        "the log's lines should be kept exactly as the agent wrote them, and a line \
         caught half-written should wait for the rest of itself"
    );

    let view = fixture.view().await;
    let printed = output(&view).expect("the session is on the Timeline");

    assert!(
        printed.running,
        "the session is still sitting on its `sleep`, so its Transcript arrived while \
         it was running"
    );

    // And the Capture is being written the whole time, which is what a session
    // that leaves no log has instead.
    let said = fixture.capture(event).await.replace("\r\n", "\n");
    assert!(
        said.contains("Reading the brief.\n") && said.contains("Asking.\n"),
        "following the log should not cost the Capture anything: {said:?}"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// A session already running is untouched when the Agent Profile it was
/// launched under is removed.
///
/// Which is the whole of what removing one promises about work in flight. The
/// account is bind-mounted into a sandbox that is already standing, and what
/// says which backend is running is the register the launch wrote rather than
/// the row — so there is nothing left for a removed Profile to interrupt.
///
/// The stub prints, waits on a file the test writes, and prints again, so the
/// removal lands squarely in the middle of the session rather than racing it:
/// the second line is a session that went on running after the Profile it names
/// had gone, and it could not have been printed before the removal.
#[tokio::test]
async fn removing_the_profile_a_running_session_was_launched_under_leaves_it_running() {
    let spill = tempfile::tempdir().unwrap();
    let carry_on = spill.path().join("carry-on");

    let fixture = grilling_spilling(
        spill,
        &format!(
            r#"
            printf 'Reading the brief.\n'

            while [ ! -f {carry_on} ]; do sleep 0.05; done

            printf 'Still here.\n'
            sleep 300
            "#,
            carry_on = carry_on.display(),
        ),
        PULL_REQUEST,
    )
    .await;

    let event = fixture.until(|view| output(view).map(|o| o.id)).await;
    fixture.printed(event, "Reading the brief.").await;

    let saved: Vec<verkstead_render::ProfileEntry> = get(&fixture.app, "/api/ui/profiles").await;
    assert!(
        !saved.is_empty(),
        "the bench saved the Profile it grills on"
    );

    for profile in &saved {
        let removed: verkstead_render::ProfileDeleted = post(
            &fixture.app,
            &format!("/api/ui/profiles/{}/delete", profile.id),
            &serde_json::json!({}),
        )
        .await;

        assert_eq!(
            removed,
            verkstead_render::ProfileDeleted::Removed,
            "a Profile is removed with a session running under it",
        );
    }

    assert_eq!(
        fixture.view().await.grilling_pairing,
        PickedView::Nothing,
        "the Conversation is nulled out of while its session runs",
    );

    std::fs::write(&carry_on, "").unwrap();
    fixture.printed(event, "Still here.").await;

    let view = fixture.view().await;

    assert!(
        output(&view)
            .expect("the session is on the Timeline")
            .running,
        "the session should still be sitting on its `sleep` with the Profile it \
         was launched under gone",
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And the details pane reads that log as a conversation while the session is
/// still having it: the lines are parsed and rendered on the way out, which is
/// what keeps the reading of somebody else's file format to the one crate that
/// has the parsers in it (ADR 0006).
///
/// The stub writes a line of each class — the conversation itself, the
/// backend's own bookkeeping, a whole line of a type nobody has ever heard of,
/// and a turn with a block of one inside it — because what the pane does with
/// them is the whole of what makes a log readable. The last two are where the
/// boundary runs: the line folds away with the bookkeeping and the block stays
/// in the turn it was said in.
#[tokio::test]
async fn a_running_sessions_log_is_read_back_as_a_conversation() {
    let fixture = grilling(
        r#"
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        log=$HOME/.claude/projects/verkstead/$name.jsonl
        mkdir -p "$(dirname "$log")"

        printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Reading the **brief**."}]}}\n' > "$log"
        printf '{"type":"attachment","attachment":{"type":"todos"}}\n' >> "$log"
        printf '{"type":"atis-latch","latched":"a kind from a later version"}\n' >> "$log"
        printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"divination","omen":"a raven"}]}}\n' >> "$log"
        printf 'Reading the brief.\n'

        sleep 300
        "#,
    )
    .await;

    let event = fixture.until(|view| output(view).map(|o| o.id)).await;
    let view = fixture.spoken(event, 2).await;

    assert_eq!(
        view.turns.first(),
        Some(&Turn::Prose(verkstead_render::Prose {
            id: 1,
            html: "<p>Reading the <strong>brief</strong>.</p>\n".to_owned()
        })),
        "the agent's prose should arrive rendered: {:?}",
        view.turns
    );
    assert!(
        matches!(view.turns.get(1), Some(Turn::Unread(_))),
        "a block nobody knows should arrive as itself rather than as nothing: {:?}",
        view.turns
    );
    assert_eq!(
        view.bookkeeping
            .iter()
            .map(|kept| kept.kind.as_str())
            .collect::<Vec<&str>>(),
        ["attachment", "atis-latch"],
        "and the backend's own bookkeeping should be out of the conversation, \
         a type this version has never met among it: {:?}",
        view.bookkeeping
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// A session on the second agent type runs the same way: launched into a
/// sandbox with its Profile's one home bound where that backend looks for it,
/// on the model its Pairing names, told which backend it is.
///
/// And its record is the Capture. This stub writes no rollout, and a Codex
/// session whose log never appears is a session with nothing to follow — which
/// is ADR-0006's rule for a session with no log and not a new one. Nothing is
/// said about it either: a log that has not turned up is the ordinary state of
/// a session's first seconds.
#[tokio::test]
async fn a_session_on_a_second_backend_runs_from_its_home_with_the_capture_as_its_record() {
    let fixture = grilling_on_codex(
        r#"
        printf 'model=%s\n' "$1"
        printf 'agent=%s\n' "${VERKSTEAD_AGENT-unset}"
        printf 'home=%s\n' "$(ls -A "$HOME" | sort | tr '\n' ' ')"
        printf 'prompt=%s' "$2"
        "#,
    )
    .await;

    let summary = fixture
        .until(|view| output(view).filter(|output| !output.running).cloned())
        .await;

    let said = fixture.capture(summary.id).await.replace("\r\n", "\n");

    assert!(
        said.contains(&format!("model={CODEX_MODEL}\n")),
        "the grilling Pairing's model is what the session runs on: {said:?}"
    );
    assert!(
        said.contains("agent=codex\n"),
        "and the session is told which backend it is, which is what tailors the \
         Guide it reads: {said:?}"
    );
    assert!(
        said.contains("home=.codex \n"),
        "and its one home is the whole of what HOME holds, bound where that \
         backend looks for it: {said:?}"
    );
    assert!(
        said.contains(BRIEF),
        "and the Brief is what the grilling starts from, as on any backend: {said:?}"
    );

    assert!(
        fixture.transcript(summary.id).await.is_empty(),
        "a Codex session that wrote no rollout has no Transcript"
    );
    assert_eq!(
        summary.turns, None,
        "and its row shows no metric rather than a count of none"
    );
}

/// And the line it is launched with is codex's own: the model as `-m`, the
/// Brief as the one positional, the approval bypass and the inline screen after
/// them, and no session id at all.
///
/// The stub reads its whole line back, which is how this is provable without an
/// account: what a real codex does with the line is codex's business, and what
/// could be wrong here is Verkstead's end of it.
///
/// **The account is configured from the line rather than from its directory.**
/// The credential store is file-backed because there is no keyring inside the
/// sandbox, and the Worktree is trusted so that no version of codex stops at a
/// trust prompt in front of nobody — and the Profile's own home is left exactly
/// as the account keeps it, which is what the last of these reads.
#[tokio::test]
async fn a_codex_session_is_launched_with_the_line_codex_takes() {
    let fixture = grilling_on_codex(
        r#"
        printf 'flag=%s\n' "$0"
        printf 'model=%s\n' "$1"
        printf 'where=%s\n' "$(pwd)"
        printf 'account=%s\n' "$(ls -A "$HOME/.codex" | tr '\n' ' ')"
        for arg in "$@"; do printf 'arg=%s\n' "$arg"; done
        printf 'prompt=%s' "$2"
        "#,
    )
    .await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains("flag=-m\n") && said.contains(&format!("model={CODEX_MODEL}\n")),
        "codex is told its model with -m: {said:?}"
    );
    assert!(
        said.contains(BRIEF),
        "and the Brief is the one positional after it: {said:?}"
    );

    for flag in [
        "--dangerously-bypass-approvals-and-sandbox",
        "--no-alt-screen",
        "cli_auth_credentials_store=\"file\"",
    ] {
        assert!(
            said.contains(&format!("arg={flag}\n")),
            "codex's line carries {flag}: {said:?}"
        );
    }

    // The directory codex will actually be sitting in, read off the session
    // rather than worked out here: a seed naming any other path would be a trust
    // prompt in front of nobody.
    let where_it_ran = said
        .lines()
        .find_map(|line| line.strip_prefix("where="))
        .expect("the session says where it ran");
    assert!(
        said.contains(&format!(
            "arg=projects={{\"{where_it_ran}\"={{trust_level=\"trusted\"}}}}\n"
        )),
        "and it trusts the Worktree it was launched in — as a whole table, which \
         is what a path with a dot in it needs: {said:?}"
    );

    assert!(
        !said.contains("--session-id"),
        "codex takes no session id, so it is told none: {said:?}"
    );
    assert!(
        said.contains("account=\n"),
        "and Verkstead writes nothing into the Profile's own directory: {said:?}"
    );
}

/// A Codex session's log is found rather than named: codex takes no session id,
/// so what says a rollout is this session's is the Worktree its own first line
/// names and its having appeared after this session was launched.
///
/// The stub writes three logs into the one store its account keeps, which is
/// what a machine running more than one session at a time actually has in
/// there: the session's own rollout, one belonging to a session in another
/// Worktree written a moment before it, and a compressed older one of this very
/// Worktree. Only the first is the record of this session, and the two beside
/// it say so in the only way a test can — by being followed instead if the
/// finder gets it wrong.
///
/// And the following itself is stage 02's, unchanged: the lines reach the
/// Transcript exactly as codex wrote them, in order, with a line caught
/// half-written held until the rest of it arrives.
#[tokio::test]
async fn a_codex_session_follows_the_rollout_that_names_its_own_worktree() {
    let fixture = grilling_on_codex(
        r#"
        day=$HOME/.codex/sessions/$(date +%Y/%m/%d)
        mkdir -p "$day"

        # A session of another Conversation, writing into the same account's
        # store at the same moment.
        printf '{"type":"session_meta","payload":{"cwd":"/srv/worktrees/tables"}}\n' \
            > "$day/rollout-2026-08-30T17-47-00-aaaa.jsonl"

        # And this Worktree's own work from a week ago, compressed where codex
        # leaves the older ones.
        printf '{"type":"session_meta","payload":{"cwd":"%s"}}\n' "$(pwd)" \
            > "$day/rollout-2026-08-30T17-47-01-bbbb.jsonl.zst"

        printf 'where=%s\n' "$(pwd)"

        log=$day/rollout-2026-08-30T17-47-02-cccc.jsonl
        printf '{"type":"session_meta","payload":{"cwd":"%s"}}\n' "$(pwd)" > "$log"
        printf 'Reading the brief.\n'

        printf '{"type":"response_item","payload":{"type":"mess' >> "$log"
        sleep 2
        printf 'age","role":"assistant"}}\n' >> "$log"
        printf 'Asking.\n'

        sleep 300
        "#,
    )
    .await;

    let event = fixture.until(|view| output(view).map(|o| o.id)).await;
    let transcript = fixture.transcript_of(event, 2).await;

    // The directory the session was actually sitting in, read off what it
    // printed rather than worked out here — which is what makes the first line
    // below a match rather than a restatement.
    let said = fixture.capture(event).await.replace("\r\n", "\n");
    let worktree = said
        .lines()
        .find_map(|line| line.strip_prefix("where="))
        .expect("the session says where it ran");

    assert_eq!(
        transcript,
        vec![
            format!(r#"{{"type":"session_meta","payload":{{"cwd":"{worktree}"}}}}"#),
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant"}}"#
                .to_owned(),
        ],
        "the rollout naming this session's own Worktree is the one followed, and its \
         lines should be kept exactly as codex wrote them — a line caught half-written \
         waiting for the rest of itself"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And what the row beside that session shows is read off the same rollout: how
/// many turns its conversation has taken, and the last thing it said.
///
/// Both off the reading the Transcript pane draws, which is what keeps a Codex
/// session's row saying what a Claude session's row says. The lines codex
/// writes twice are what proves it: the turn the screen drew is counted once
/// and quoted once, and the same turn again as the model was sent it is
/// bookkeeping and neither.
#[tokio::test]
async fn a_codex_sessions_row_is_summarised_from_the_rollout_the_pane_draws() {
    let fixture = grilling_on_codex(
        r#"
        day=$HOME/.codex/sessions/$(date +%Y/%m/%d)
        mkdir -p "$day"
        log=$day/rollout-2026-08-30T17-47-02-cccc.jsonl

        printf '{"type":"session_meta","payload":{"cwd":"%s"}}\n' "$(pwd)" > "$log"
        printf '{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"UserMessage","id":"u-1","content":[{"type":"text","text":"Where should the counter live?"}]}}}\n' >> "$log"
        printf '{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"m-1","content":[{"type":"Text","text":"In the store, beside the window it counts over."}]}}}\n' >> "$log"
        printf '{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":1240}}}}\n' >> "$log"
        printf '{"type":"response_item","payload":{"type":"message","id":"m-1","role":"assistant","content":[{"type":"output_text","text":"In the store, beside the window it counts over."}]}}\n' >> "$log"

        sleep 300
        "#,
    )
    .await;

    let summary = fixture
        .until(|view| {
            output(view)
                .filter(|output| !output.latest.is_empty())
                .cloned()
        })
        .await;

    assert_eq!(
        summary.latest, "In the store, beside the window it counts over.",
        "the row quotes what the agent said and not what a tool or the backend did"
    );
    assert_eq!(
        summary.turns,
        Some(2),
        "the turn put to it and the turn it took — and neither the token count \
         nor the same turn over again as the model was sent it"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// A Grok session's log is named rather than found: grok takes the session id at
/// launch, so the conversation it keeps is the file under the name Verkstead
/// gave it.
///
/// What Verkstead does not know is the directory grok grouped that session
/// under. Grok's store is organised by working directory and then by session,
/// and the outer name is grok's own encoding of the path — so the stub writes
/// its log under an encoding of its own, as grok would have, and nothing in
/// Verkstead works that name out. Two things sit beside it that a real store
/// also has in it and that are not this session's conversation: another
/// Conversation's session under its own encoded directory, and the index entry
/// grok keeps next to the log itself.
///
/// And the following is stage 02's, unchanged: the lines reach the Transcript
/// exactly as grok wrote them, in order, with a line caught half-written held
/// until the rest of it arrives. What is added here is the far end of that —
/// the pane over those same lines, drawing the session updates grok wrote as
/// the conversation they record.
#[tokio::test]
async fn a_grok_session_follows_the_log_it_was_named_for() {
    let fixture = grilling_on_grok(
        r#"
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        store=$HOME/.grok/sessions

        # A session of another Conversation, in the same account's store, under
        # the directory grok grouped that Worktree's sessions in.
        elsewhere=$store/%2Fsrv%2Fworktrees%2Ftables/6f8b17c2-not-this-session
        mkdir -p "$elsewhere"
        printf '{"method":"session/update","params":{"sessionId":"6f8b17c2-not-this-session","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Another Conversation."}}}}\n' \
            > "$elsewhere/updates.jsonl"

        # And this session's own, under grok's encoding of the directory it is
        # running in — which the stub stands in for, since nothing here is grok.
        encoded=$(pwd | sed 's|/|%2F|g')
        mine=$store/$encoded/$name
        mkdir -p "$mine"

        printf 'grouped=%s\n' "$encoded"

        # The store's index entry, which sits beside the log and is not it.
        printf '{"title":"Rate limiting","model":"grok-4.6"}\n' > "$mine/summary.json"

        printf 'named=%s\n' "$name"

        log=$mine/updates.jsonl
        printf '{"method":"session/update","params":{"sessionId":"%s","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"Rate limiting"}}}}\n' "$name" > "$log"
        printf 'Reading the brief.\n'

        printf '{"method":"session/update","params":{"sessionId":"%s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"te' "$name" >> "$log"
        sleep 2
        printf 'xt","text":"Where does the counter live?"}}}}\n' >> "$log"
        printf 'Asking.\n'

        sleep 300
        "#,
    )
    .await;

    let event = fixture.until(|view| output(view).map(|o| o.id)).await;
    let transcript = fixture.transcript_of(event, 2).await;

    // The name it was actually run under, read off what it printed: what makes
    // the log above this session's own rather than whatever else was in there,
    // and what grok names the session inside every line of it.
    let said = fixture.capture(event).await.replace("\r\n", "\n");
    let name = said
        .lines()
        .find_map(|line| line.strip_prefix("named="))
        .expect("the session says what it was named");

    assert_eq!(
        transcript,
        [
            r#"{"method":"session/update","params":{"sessionId":"NAMED","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"Rate limiting"}}}}"#,
            r#"{"method":"session/update","params":{"sessionId":"NAMED","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Where does the counter live?"}}}}"#,
        ]
        .map(|line| line.replace("NAMED", name)),
        "the log under the name Verkstead gave this session is the one followed, \
         and its lines should be kept exactly as grok wrote them — a line caught \
         half-written waiting for the rest of itself"
    );

    let pool = open_database(&fixture.database).await.unwrap();

    assert_eq!(
        verkstead_store::session_id(&pool, event).await.unwrap(),
        Some(name.to_owned()),
        "and that name is the one Verkstead wrote down beside the session's Event"
    );

    // And the log really was inside an encoded directory, which is what says the
    // store was walked rather than the path guessed at: a stub that had left the
    // group out would have put its log where a lookup finds it without walking
    // anything, and proved nothing.
    let grouped = said
        .lines()
        .find_map(|line| line.strip_prefix("grouped="))
        .expect("the session says what it grouped its log under");

    assert!(
        grouped.contains("%2F"),
        "the group directory is grok's own encoding of the working directory: {grouped:?}"
    );
    assert!(
        said.contains("Reading the brief.\n"),
        "following the log should not cost the Capture anything: {said:?}"
    );

    let view = fixture.view().await;
    let printed = output(&view).expect("the session is on the Timeline");

    assert!(
        matches!(printed.turns, Some(turns) if turns > 0),
        "and the row shows a Transcript rather than nothing: {:?}",
        printed.turns
    );

    // The details pane over the same lines, which is where the following and
    // the reading meet: the log grok wrote, followed under the name Verkstead
    // gave it, drawn as the conversation it records.
    let drawn = fixture.spoken(event, 2).await;

    assert!(
        matches!(
            &drawn.turns[..],
            [Turn::Put(put), Turn::Prose(prose)]
                if put.html.contains("Rate limiting")
                    && prose.html.contains("Where does the counter live?")
        ),
        "the pane should draw what has been stored: {:?}",
        drawn.turns
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// An OpenCode session's record is found rather than named, and it is not a
/// file: opencode takes no session id, and it keeps its sessions in one database
/// under its account. So what says a session in there is this one is the
/// directory it recorded opening in and its having been created after this
/// session was launched — Codex's rule, against a store of another shape.
///
/// The store is written from outside the sandbox, because there is no `sqlite3`
/// on the system profile for a stub to write one with — and it is written the
/// way a machine running more than one session at a time actually has it: this
/// session's own, one belonging to a session in another Worktree created a
/// moment before it, and an older one of this very Worktree. Only the first is
/// the record of this session, and the two beside it say so in the only way a
/// test can — by being followed instead if the finder gets it wrong.
///
/// What lands is each record whole: opencode's own kind, its place in the
/// session's sequence, and the payload byte for byte — and what the row counts
/// of it is what the pane would draw, which for these two is the turn put to
/// the session and nothing for the session's own row.
#[tokio::test]
async fn an_opencode_session_follows_the_records_of_the_session_it_opened_in_its_worktree() {
    let spill = tempfile::tempdir().unwrap();
    let ran_in = spill.path().join("ran-in");

    let fixture = grilling_spilling_on_opencode(
        spill,
        &format!(
            r#"
            printf '%s' "$(pwd)" > {ran_in}
            printf 'Reading the brief.\n'
            sleep 300
            "#,
            ran_in = ran_in.display(),
        ),
    )
    .await;

    // Where the session is actually sitting, read off what it wrote rather than
    // worked out here — which is what makes the row below a match rather than a
    // restatement.
    let worktree = until_written(&ran_in).await;
    let event = fixture.until(|view| output(view).map(|o| o.id)).await;

    let store = opencode_store(&fixture.opencode_account()).await;

    // A session of another Conversation, in the same account's store, created
    // at the same moment as this one.
    opencode_session(&store, "ses_elsewhere", "/srv/worktrees/tables", 0).await;
    opencode_record(
        &store,
        "ses_elsewhere",
        0,
        "session.created.1",
        r#"{"info":{"title":"Another Conversation."}}"#,
    )
    .await;

    // And this Worktree's own work from an hour ago, which is the same
    // Conversation resumed and not this session.
    opencode_session(&store, "ses_before", &worktree, -3_600_000).await;
    opencode_record(
        &store,
        "ses_before",
        0,
        "session.created.1",
        r#"{"info":{"title":"An hour ago."}}"#,
    )
    .await;

    // And this session's own.
    opencode_session(&store, "ses_mine", &worktree, 0).await;
    opencode_record(
        &store,
        "ses_mine",
        0,
        "session.created.1",
        &format!(r#"{{"info":{{"title":"Rate limiting","directory":"{worktree}"}}}}"#),
    )
    .await;

    let first = fixture.transcript_of(event, 1).await;

    assert_eq!(
        first,
        [
            r#"{"kind":"session.created.1","seq":0,"record":{"info":{"title":"Rate limiting","directory":"WORKTREE"}}}"#,
        ]
        .map(|line| line.replace("WORKTREE", &worktree)),
        "the session in the store that opened in this Worktree after this session \
         started is the one followed, and its record should reach the Transcript \
         whole — opencode's own kind and its place in the sequence around the \
         payload as the store holds it",
    );

    // And a second poll takes what arrived since and only that, which is what a
    // cursor into a store means where a byte offset into a file meant it before.
    opencode_record(
        &store,
        "ses_mine",
        1,
        "message.part.updated.1",
        r#"{"part":{"type":"text","text":"Where does the counter live?"}}"#,
    )
    .await;

    let both = fixture.transcript_of(event, 2).await;

    assert_eq!(
        both,
        [
            first[0].clone(),
            r#"{"kind":"message.part.updated.1","seq":1,"record":{"part":{"type":"text","text":"Where does the counter live?"}}}"#.to_owned(),
        ],
        "and the record that arrived after it should be added rather than the \
         whole of the session read again",
    );

    let said = fixture.capture(event).await.replace("\r\n", "\n");

    assert!(
        said.contains("Reading the brief.\n"),
        "following the store should not cost the Capture anything: {said:?}"
    );

    // Waited for rather than read the once, because the two are written a step
    // apart: the poll that moves the follower puts the records in the store and
    // *then* summarises the row off them, so a Transcript that has arrived is
    // not yet a count of what is on it — see `summarise` in
    // `crates/server/src/sessions.rs`. Read immediately, this asserted against
    // whichever of the two had happened by then, which on a loaded machine is
    // the first.
    let printed = fixture
        .until(|view| {
            output(view)
                .filter(|printed| printed.turns.is_some())
                .cloned()
        })
        .await;

    assert_eq!(
        printed.turns,
        Some(1),
        "and the row counts the reading the pane draws: the text put to the \
         session is the one turn of it, and the session's own row is opencode's \
         bookkeeping",
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And a store this build cannot read leaves the session Capture-only, which is
/// what ADR-0006 gives every session with no record of its own. opencode's
/// layout is its own and moves between releases — a table renamed, a column gone
/// — and none of that may fail a session.
#[tokio::test]
async fn an_opencode_store_this_build_cannot_read_leaves_the_session_capture_only() {
    let spill = tempfile::tempdir().unwrap();
    let ran_in = spill.path().join("ran-in");

    let fixture = grilling_spilling_on_opencode(
        spill,
        &format!(
            r#"
            printf '%s' "$(pwd)" > {ran_in}
            printf 'Reading the brief.\n'
            "#,
            ran_in = ran_in.display(),
        ),
    )
    .await;

    until_written(&ran_in).await;

    // A release that renamed the column the directory is recorded under, which
    // stands for the whole class: the database is there and readable, and the
    // question Verkstead asks of it has no answer.
    let store = opencode_store(&fixture.opencode_account()).await;

    store
        .execute("ALTER TABLE session RENAME COLUMN directory TO cwd")
        .await
        .unwrap();

    let summary = fixture
        .until(|view| output(view).filter(|output| !output.running).cloned())
        .await;

    assert!(
        fixture.transcript(summary.id).await.is_empty(),
        "a session whose store this build cannot read has no Transcript"
    );
    assert_eq!(
        summary.turns, None,
        "and its row shows no metric rather than a count of none"
    );
    assert_eq!(
        fixture.capture(summary.id).await,
        "Reading the brief.\r\n",
        "and what it said is on the Capture, which is a complete record on its own"
    );
}

/// opencode's store, made where opencode would have made it — under the data
/// half of the account, in the file whose name the sandbox pinned.
///
/// Only the columns Verkstead reads, for the reason the reader's own fixtures
/// hold only those: what the rest of that database holds is opencode's business,
/// and a fixture that copied it would be this suite claiming to know more of
/// somebody else's schema than Verkstead reads.
async fn opencode_store(account: &Path) -> SqlitePool {
    let data = account.join(".local/share/opencode");
    std::fs::create_dir_all(&data).unwrap();

    let store = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(data.join("opencode.db"))
            .create_if_missing(true)
            // The mode opencode keeps its own store in, which is what a reader
            // of it has to be able to read.
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal),
    )
    .await
    .unwrap();

    store
        .execute(
            "CREATE TABLE session (
                 id           TEXT PRIMARY KEY,
                 parent_id    TEXT,
                 directory    TEXT NOT NULL,
                 time_created INTEGER NOT NULL
             );
             CREATE TABLE event (
                 id           TEXT PRIMARY KEY,
                 aggregate_id TEXT NOT NULL,
                 seq          INTEGER NOT NULL,
                 type         TEXT NOT NULL,
                 data         TEXT NOT NULL
             );",
        )
        .await
        .unwrap();

    store
}

/// A session in that store, opened in `directory` `when` milliseconds from now.
async fn opencode_session(store: &SqlitePool, session: &str, directory: &str, when: i64) {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + when;

    sqlx::query(
        "INSERT INTO session (id, parent_id, directory, time_created) VALUES (?, NULL, ?, ?)",
    )
    .bind(session)
    .bind(directory)
    .bind(created)
    .execute(store)
    .await
    .unwrap();
}

/// And a record it wrote inside one.
async fn opencode_record(store: &SqlitePool, session: &str, seq: i64, kind: &str, data: &str) {
    sqlx::query("INSERT INTO event (id, aggregate_id, seq, type, data) VALUES (?, ?, ?, ?, ?)")
        .bind(format!("{session}-{seq}"))
        .bind(session)
        .bind(seq)
        .bind(kind)
        .bind(data)
        .execute(store)
        .await
        .unwrap();
}

/// A session that keeps no log of itself leaves no Transcript, and nothing about
/// that is a fault: it is every stub agent the test suite runs, and every
/// backend that keeps no such record. What those sessions said is the Capture,
/// which is a complete record on its own.
#[tokio::test]
async fn a_session_that_keeps_no_log_leaves_no_transcript() {
    let fixture = grilling(r#"printf 'Nothing to say.\n'"#).await;

    let summary = fixture
        .until(|view| output(view).filter(|output| !output.running).cloned())
        .await;
    let event = summary.id;

    assert!(
        fixture.transcript(event).await.is_empty(),
        "a session that wrote no log should have left no Transcript behind"
    );
    assert_eq!(
        summary.turns, None,
        "and its row shows no metric at all rather than a count of none: there is \
         no conversation here to have taken turns"
    );
    assert_eq!(
        fixture.capture(event).await,
        "Nothing to say.\r\n",
        "and what it said should be on the Capture as it always was"
    );
}

/// The Timeline row reads what the agent said, and it keeps up while the
/// session is still saying it.
///
/// The terminal underneath is a display being redrawn — a box, a spinner, a
/// status line saying which key interrupts — and the last line of one at any
/// given moment is whatever the interface happened to be drawing. What the row
/// is for is somebody deciding from one line whether to open the pane, so it
/// reads the agent's own prose off the log beside it.
#[tokio::test]
async fn a_running_sessions_row_reads_the_last_thing_the_agent_said() {
    let fixture = grilling(
        r#"
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        log=$HOME/.claude/projects/verkstead/$name.jsonl
        mkdir -p "$(dirname "$log")"

        printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Reading the brief."}]}}\n' > "$log"
        printf '\033[2m╭──────────────────╮\033[0m\n'
        sleep 1

        printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Where should the counter live?"}]}}\n' >> "$log"
        printf '\033[2m│ esc to interrupt │\033[0m\n'

        sleep 300
        "#,
    )
    .await;

    let summary = fixture
        .until(|view| {
            output(view)
                .filter(|output| output.latest == "Where should the counter live?")
                .cloned()
        })
        .await;

    assert!(
        summary.running,
        "the session is still sitting on its `sleep`, so the row moved on while it ran"
    );

    // What the terminal had on it at the same moment, which is what the row
    // would have read instead.
    let drawn = fixture.capture(summary.id).await;
    assert!(
        drawn.contains("esc to interrupt"),
        "the interface was drawing something else the whole time: {drawn:?}"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And the row's metric is how far that conversation has got: the turns on the
/// Transcript, counted as the log is followed.
///
/// The line count it replaced was a count of newlines off the terminal, and a
/// full-screen interface redraws itself with cursor moves — so it read 0 for
/// every real session. The turns are the Transcript's own, which is where a
/// session's conversation actually is.
///
/// Counted as the pane draws them, so the backend's own bookkeeping — about a
/// third of every log — is none of them. The stub writes one of those beside
/// the three turns to say so.
#[tokio::test]
async fn a_running_sessions_row_counts_the_turns_on_its_transcript() {
    let fixture = grilling(
        r#"
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        log=$HOME/.claude/projects/verkstead/$name.jsonl
        mkdir -p "$(dirname "$log")"

        printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Reading the brief."}]}}\n' > "$log"
        printf '{"type":"attachment","attachment":{"type":"todos"}}\n' >> "$log"
        printf 'Reading the brief.\n'
        sleep 1

        printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]}}\n' >> "$log"
        printf '{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"limiter.md"}]}}\n' >> "$log"
        printf 'Looking.\n'

        sleep 300
        "#,
    )
    .await;

    let counted = fixture
        .until(|view| {
            output(view)
                .filter(|output| output.turns == Some(1))
                .cloned()
        })
        .await;

    assert!(
        counted.running,
        "the session is still sitting on its `sleep`, so the count moved while it ran"
    );

    // And it goes on moving: the tool call and its answer are two more turns,
    // and the bookkeeping line beside them is none.
    let grown = fixture
        .until(|view| {
            output(view)
                .filter(|output| output.turns == Some(3))
                .cloned()
        })
        .await;

    assert_eq!(
        grown.turns,
        Some(3),
        "the prose, the call and the answer — and not the backend's own line"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And beside the metric, whether that session is still talking — which is a
/// different question from whether it is running, and the one the mark on the
/// row draws.
///
/// Idle is the server's judgement rather than the page's, because what it is
/// measuring is a terminal: claude repaints its spinner many times a second
/// while it works, so a session that has printed nothing for three seconds is
/// one that has stopped. The case it exists for is a grilling sitting on a
/// blocking ask for hours with the Timeline saying it is busy.
///
/// And it goes back on speaking, which is the other half: the flag is computed
/// on every read rather than latched, so nothing has to remember to clear it.
#[tokio::test]
async fn a_running_sessions_row_says_when_it_has_stopped_talking() {
    let fixture = grilling(
        r#"
        printf 'Reading the brief.\n'
        sleep 5
        printf 'What should happen when the queue is full?\n'
        sleep 300
        "#,
    )
    .await;

    let talking = fixture
        .until(|view| {
            output(view)
                .filter(|output| !output.latest.is_empty())
                .cloned()
        })
        .await;

    assert!(
        talking.running && !talking.idle,
        "a session that has just printed is working, not idle: {talking:?}"
    );

    // And three seconds later it has said nothing more, which is what the empty
    // circle is for.
    let quiet = fixture
        .until(|view| output(view).filter(|output| output.idle).cloned())
        .await;

    assert!(
        quiet.running,
        "idle is a thing a running session is, so the two travel together"
    );

    // Then it speaks again, and the row says so without anything having had to
    // remember to put it back.
    //
    // Read back until the line is on the row as well, rather than asserting it
    // off the first view that had come out of idle: idle is computed live off
    // the clock the terminal moved, and what a session printed reaches the
    // store on the flush after it — so the crossing is readable a moment
    // before the statement that caused it is.
    let woken = fixture
        .until(|view| {
            output(view)
                .filter(|output| {
                    output.running
                        && !output.idle
                        && output.latest == "What should happen when the queue is full?"
                })
                .cloned()
        })
        .await;

    assert_eq!(
        woken.latest, "What should happen when the queue is full?",
        "the statement that woke it is the one the row now reads"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And while it runs, the same card is pinned above the record — first of
/// everything pinned, because it is what the Conversation is *doing* where the
/// rest is what the work is against.
///
/// One card drawn twice rather than two cards, the arrangement a pull request
/// already has: the pinned copy and the copy on the record are the same Event,
/// made by the same call.
///
/// It comes and goes with the run, which is the whole reason the pin is read off
/// what is being written into rather than off the latest session on the record.
/// A Conversation Verkstead has finished with would otherwise carry the last run
/// it ever made at the head of its pane for good — a card saying *running* about
/// a worktree that has been taken away.
#[tokio::test]
async fn the_session_being_written_into_is_pinned_above_the_record() {
    let fixture = grilling(
        r#"
        printf 'Reading the brief.\n'
        sleep 300
        "#,
    )
    .await;

    let running = fixture.running().await;

    let view = fixture
        .until(|view| pinned_session(view).map(|_| view.clone()))
        .await;

    let card = pinned_session(&view).unwrap();

    assert_eq!(
        card.id, running,
        "the pinned card is the session being written into"
    );
    assert!(
        card.running,
        "and it says so, as the copy on the record does"
    );
    assert_eq!(
        Some(card),
        output(&view),
        "the two copies are the one Event, made by the one call"
    );
    assert!(
        matches!(view.pinned.first(), Some(PinnedEvent::AgentOutput(_))),
        "and it leads what is pinned: {:?}",
        view.pinned,
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);

    // Closing takes the session down, and nothing is being written into
    // anything after that: the record keeps the run, and the pinned block has
    // nothing to say about it.
    let after = fixture
        .until(|view| pinned_session(view).is_none().then(|| view.clone()))
        .await;

    assert!(
        output(&after).is_some(),
        "the run is still on the record, which is where it happened",
    );
}

/// The session card a view has pinned, where it has one.
fn pinned_session(view: &ConversationView) -> Option<&AgentOutputEvent> {
    view.pinned.iter().find_map(|event| match event {
        PinnedEvent::AgentOutput(output) => Some(output),
        _ => None,
    })
}

/// And the crossing is announced, because it is the one change a session makes
/// by doing nothing.
///
/// Every other thing a Timeline says moves because a session printed, and
/// printing is what nudges an open page into reading it back. A session going
/// quiet is exactly when that stops — so an open page would sit on a turning
/// ring until something else happened to the Conversation, which for a grilling
/// on a blocking ask is the human answering it.
#[tokio::test]
async fn a_session_falling_quiet_is_announced_to_the_open_pages() {
    let fixture = grilling(
        r#"
        printf 'Reading the brief.\n'
        sleep 300
        "#,
    )
    .await;

    // Opened over a session that has said its piece and is now sitting there:
    // what it printed has already been flushed and announced, so the next Nudge
    // down this stream is the one this test is about.
    fixture
        .until(|view| {
            output(view)
                .filter(|output| !output.latest.is_empty() && !output.idle)
                .map(|_| ())
        })
        .await;

    let mut page = Listening::open(&fixture.app).await;

    assert_eq!(
        page.nudge().await,
        Nudge::Conversation {
            conversation: fixture.id
        },
        "the session printed nothing more, so the only thing that moved is that \
         it stopped — announced on the Conversation's own kind, which is what \
         reaches the Timeline row and the sidebar card alike"
    );

    let quiet = output(&fixture.view().await)
        .expect("the session's Event is on the Timeline")
        .clone();

    assert!(
        quiet.running && quiet.idle,
        "and the page reading it back on that Nudge finds it idle: {quiet:?}"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And it reads the last of it once the session has gone.
///
/// An agent writes its closing words on the way out, which is after the
/// terminal it was printing to has closed — so the last thing said reaches the
/// Transcript after the last thing printed reaches the Capture, and a row
/// written only by the output would stop one statement short of the point.
#[tokio::test]
async fn a_finished_sessions_row_reads_its_closing_words() {
    let fixture = grilling(
        r#"
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        log=$HOME/.claude/projects/verkstead/$name.jsonl
        mkdir -p "$(dirname "$log")"

        printf '\033[2m│ working │\033[0m\n'
        printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"The limiter is written and the tests pass."}]}}\n' > "$log"
        "#,
    )
    .await;

    let summary = fixture
        .until(|view| output(view).filter(|output| !output.running).cloned())
        .await;

    assert_eq!(
        summary.latest, "The limiter is written and the tests pass.",
        "a session that has ended is summarised by what it said on its way out"
    );
}

/// A grilling session runs for an hour. A Timeline that said nothing until it
/// finished would be a Timeline nobody could watch.
#[tokio::test]
async fn what_a_session_prints_reaches_the_timeline_while_it_is_still_running() {
    let fixture = grilling(
        r#"
        printf 'Reading the brief.\n'
        printf 'What should happen when the queue is full?'
        sleep 300
        "#,
    )
    .await;

    let summary = fixture
        .until(|view| {
            output(view)
                .filter(|output| !output.latest.is_empty())
                .cloned()
        })
        .await;

    assert!(
        summary.running,
        "the session is still sitting on its `sleep`, so the Conversation should say so"
    );
    assert_eq!(
        summary.lines, 1,
        "one finished line, and the question it is waiting on is the other"
    );
    assert_eq!(
        summary.latest, "What should happen when the queue is full?",
        "a session goes quiet mid-line exactly when it has stopped to ask something"
    );

    // The whole of it is there to read while it is still being written.
    let said = fixture.capture(summary.id).await;
    assert!(said.contains("Reading the brief."), "{said:?}");

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// What a terminal was sent is what the session said. Nothing is stripped on
/// the way in — the tidying is for the one line the Timeline shows.
#[tokio::test]
async fn the_details_pane_gets_the_capture_byte_for_byte() {
    let fixture = grilling(r#"printf '\033[1mbold\033[0m\nplain\n'"#).await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    assert_eq!(
        fixture.capture(event).await,
        // The line endings are the pseudo-terminal's own doing, and they are
        // part of what was sent.
        "\u{1b}[1mbold\u{1b}[0m\r\nplain\r\n",
    );
}

/// A session that has ended is a Conversation with a Capture, not one with an
/// agent in it.
///
/// Read on every poll rather than only at the end, because the moment worth
/// asking about is the one where the session stops: a relay leaves the register
/// of what is running only once it has flushed the last of what it printed, so
/// an Event drawn as stopped is one whose output is all in the store. Drawn as
/// stopped with nothing in it, it is that order having come apart, and what the
/// page then says is that the session never spoke — which is the one thing about
/// a session that is hardest to tell from the truth.
#[tokio::test]
async fn a_session_that_exits_leaves_a_conversation_that_says_so() {
    let fixture = grilling("printf 'done\\n'").await;

    let summary = fixture
        .until(|view| {
            let output = output(view)?;

            assert!(
                output.running || output.lines > 0,
                "the session is drawn as stopped having printed nothing, and it \
                 printed: {output:?}",
            );

            (!output.running).then(|| output.clone())
        })
        .await;

    assert_eq!(summary.latest, "done");
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Grilling,
        "where the work has got to is not what the session's ending decides"
    );
}

/// The sidebar's spinner: a Conversation reports that it is working for as long
/// as it has a session in it, and stops the moment that session is over.
///
/// Read off the register of running processes rather than off anything stored,
/// which is what makes the second half of this true at all — nothing writes a
/// row saying the session ended, because there was never one saying it began.
#[tokio::test]
async fn a_conversation_reports_that_it_is_working_while_its_session_runs() {
    let fixture = grilling(
        r#"
        printf 'reading the brief\n'
        sleep 300
        "#,
    )
    .await;

    let row = fixture
        .row_until(|row| row.working.then(|| row.clone()))
        .await;
    assert!(
        !row.waiting,
        "nothing is being asked, so the card turns rather than marks",
    );

    // And then wait for the session to have got as far as printing, which the
    // half above deliberately does not: a session is on the register from the
    // moment it is spawned, so the row turns before the sandbox inside it has a
    // shell in it.
    //
    // The close below is what needs the difference. Ending a session is a
    // signal to the sandbox, and a sandbox signalled while it is still setting
    // its namespace up can leave what it was starting behind — a stub with five
    // minutes of `sleep` left in it, a terminal that therefore never closes, and
    // a close that waits the whole five minutes out. Which is a test that hangs
    // rather than one that fails, and hangs the more reliably the quieter the
    // machine is.
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    assert_eq!(fixture.close().await, ConversationClosed::Closed);

    fixture.row_until(|row| (!row.working).then_some(())).await;
}

/// And the ring on that card goes empty while the session sits there, which is
/// the card saying what the row it opens already says.
///
/// The case it exists for is a grilling on a blocking ask: the session is alive
/// and will be for as long as the human takes, and a spinner turning for an hour
/// says something is happening when nothing is. It goes back the moment the
/// session speaks, off nothing having had to remember to clear it — the flag is
/// the quiet clock read at the moment the sidebar is drawn.
#[tokio::test]
async fn a_conversation_whose_session_has_gone_quiet_says_so_on_its_card() {
    let fixture = grilling(
        r#"
        printf 'reading the brief\n'
        sleep 6
        printf 'what should happen when the queue is full?\n'
        sleep 300
        "#,
    )
    .await;

    // A session that is printing is working rather than idle, which is the
    // ordinary turning ring.
    let talking = fixture
        .row_until(|row| (row.working && !row.idle).then(|| row.clone()))
        .await;
    assert!(
        !talking.waiting,
        "nothing is being asked, so what the card draws is the ring",
    );

    // Then it stops, and three seconds of nothing is what the empty ring says.
    let quiet = fixture.row_until(|row| row.idle.then(|| row.clone())).await;
    assert!(
        quiet.working,
        "idle is a thing a running session is, so the two travel together: \
         {quiet:?}"
    );
    assert!(
        !quiet.waiting,
        "and waiting still outranks both, so a card with nothing to answer is \
         where this can be seen at all",
    );

    // And back to the turning one when it speaks again.
    fixture
        .row_until(|row| (row.working && !row.idle).then_some(()))
        .await;

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And the crossing back out of the silence is announced, which is what the
/// sidebar hears it on.
///
/// The other half of the crossing *into* quiet being announced. What a session
/// prints is announced on the Screen's kind, which is about the Conversation
/// being watched rather than the list of them — so a card left on the empty ring
/// would stay on it until something else happened to the Conversation, which for
/// a grilling on a blocking ask is the human answering it.
#[tokio::test]
async fn a_session_speaking_again_is_announced_to_the_open_pages() {
    let fixture = grilling(
        r#"
        printf 'reading the brief\n'
        sleep 6
        printf 'what should happen when the queue is full?\n'
        sleep 300
        "#,
    )
    .await;

    // Opened well past the crossing into quiet — the announcement of *that* has
    // been and gone, so the next thing down this stream is the one this test is
    // about.
    fixture.row_until(|row| row.idle.then_some(())).await;
    pause(Duration::from_millis(200)).await;

    let mut page = Listening::open(&fixture.app).await;

    assert_eq!(
        page.nudge().await,
        Nudge::Conversation {
            conversation: fixture.id
        },
        "the session speaking again is announced on the Conversation's own kind, \
         before what it printed goes out on the Screen's — because the Screen's \
         is not what a sidebar reads",
    );

    let woken = fixture.row().await;

    assert!(
        woken.working && !woken.idle,
        "and the sidebar reading itself back on that Nudge finds the turning \
         ring: {woken:?}"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// And the sidebar's dot, for the source a Conversation can only get to by
/// really running: a run that has stopped is waiting on the human until they
/// start it going again.
///
/// It is waiting and not working by then — the session that failed is gone — so
/// this is also the plainest case of the two facts being separate things.
#[tokio::test]
async fn a_run_that_halted_is_waiting_on_the_human() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the grilling is running\n'
            sleep 300
            ;;
        *)
            printf 'error: unresolved import crate::window\n'
            exit 1
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    // The grilling session is still sitting on its `sleep`, and its Set is the
    // proposal — so what the row says here is that the human is being asked.
    let set = fixture.ask(PROPOSING).await;
    assert!(fixture.row().await.waiting, "a Set nobody has answered");

    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    // Which the implementation session then fails, and the run stops.
    fixture.stopped().await;

    // Waited on together rather than asserted off one snapshot: the session's
    // relay leaves the register and the runner records the stop at much the same
    // moment, and what matters is where the row settles.
    fixture
        .row_until(|row| (row.waiting && !row.working).then_some(()))
        .await;

    // And the dot is the stop's: taking it away — which is what starting to
    // drive again does — takes the dot with it.
    fixture.drive_again().await;

    fixture.row_until(|row| (!row.waiting).then_some(())).await;
}

/// The record is the record. A server that has been restarted has no sessions
/// at all, and every Capture it holds is of one that is over.
#[tokio::test]
async fn a_capture_survives_the_server_restarting() {
    let fixture = grilling(r#"printf 'first\nsecond\n'"#).await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;
    let said = fixture.capture(event).await;

    // A second server over the database the first one wrote, which is what a
    // restart is from the store's side of things.
    let restarted = router_running_sessions(
        open_database(&fixture.database).await.unwrap(),
        WatchedPaths::none(),
        fixture.state.path().to_owned(),
        Agents::running(
            vec!["/bin/sh".to_owned(), "-c".to_owned(), "true".to_owned()],
            Homes::on(
                Platform::HERE,
                PathBuf::from("/nonexistent"),
                fixture.state.path(),
            ),
            Reachable::at(LISTENING),
            SandboxConfig::default(),
            BuildCache::none(),
            Skills::installed(fixture.state.path()).expect("this binary carries skills"),
            equipped(fixture.state.path()),
            Handoffs::under(fixture.state.path()),
            Attachments::under(fixture.state.path()),
            Settings::in_data_dir(fixture.state.path()),
        ),
        gh_stub(PULL_REQUEST),
    );

    let view: ConversationView =
        get(&restarted, &format!("/api/ui/conversations/{}", fixture.id)).await;
    let summary = output(&view).expect("the session's output is still on the Timeline");

    assert!(
        !summary.running,
        "a restarted server is running no sessions, whatever the record says happened"
    );
    assert_eq!(summary.latest, "second");

    let read_back: Capture = get(
        &restarted,
        &format!(
            "/api/ui/conversations/{}/capture/{}",
            fixture.id, summary.id
        ),
    )
    .await;
    assert_eq!(read_back.text, said);
}

/// The inline direction end to end: the human picks it on the closing Set, the
/// grilling session writes a handoff where the skill says and goes quiet, and
/// that handoff plus quiet is what ends the grilling — after which a fresh
/// session under the *other* Profile builds the work, primed with the handoff and
/// committing without anything to wait on, and carries the branch to a pull
/// request the Conversation then wraps up.
///
/// One stub for both sessions, telling them apart by the model it was run on,
/// because that is the fact under all of it: the two run as different accounts,
/// which is why the grilling cannot simply carry on and why the handoff has to
/// exist at all.
///
/// The stub writes its handoff as it starts rather than when the Response lands,
/// for the reason the task-list and roadmap stubs commit their artifacts early: a
/// stub cannot idle on a blocking ask, and a document already written is watched
/// for exactly as one written a minute later is.
#[tokio::test]
async fn choosing_inline_runs_the_implementation_profile_on_the_handoff() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nAn in-process counter.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            printf 'model=%s\n' "$1"
            printf 'prompt=%s\n' "$2"
            printf 'a limiter\n' > limiter.md
            git add limiter.md
            git commit --quiet -m 'feat: rate limiting'
            ;;
        esac
        "#,
    )
    .await;

    // The grilling has done its half: the handoff is written, and what follows
    // is the human answering.
    let grilling_output = fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let worktree = PathBuf::from(
        fixture
            .view()
            .await
            .worktree
            .expect("a grilling Conversation has a worktree")
            .path,
    );

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    // The handoff plus quiet is what ends the grilling, and the document goes on
    // the Timeline at that moment — the one moment it is certainly finished.
    let handed = fixture
        .until(|view| handoff(view).map(|handoff| handoff.html.clone()))
        .await;

    assert!(
        handed.contains("in-process counter"),
        "the handoff arrives rendered, like every other piece of agent markdown: {handed}",
    );

    // The second session, which is a different Event: the first is the grilling,
    // and it ended when its handoff landed.
    let implementing = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id != grilling_output && !output.running)
                .map(|output| output.id)
        })
        .await;

    let said = fixture.capture(implementing).await.replace("\r\n", "\n");

    assert!(
        said.contains("model=claude-implementation-5"),
        "the work runs under the implementation Profile, not the one that grilled: {said:?}"
    );
    assert!(
        said.contains("/verkstead/skills/implementing/SKILL.md"),
        "and inside the bundled implementation skill: {said:?}"
    );
    assert!(
        said.contains("An in-process counter."),
        "primed with the handoff the grilling wrote: {said:?}"
    );
    assert!(
        said.contains(BRIEF),
        "and with the Brief the work started from: {said:?}"
    );

    // And the ending an inline run has. The session that built the work carried
    // the branch to a pull request on its way out, the way a finished backlog's
    // last step does, so the Conversation moves on to wrapping that up.
    let opened = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping)
                .then(|| pull_request(view).cloned())
                .flatten()
        })
        .await;

    assert_eq!(opened.number, 41);

    assert_eq!(
        fixture
            .view()
            .await
            .timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::Moved(moved) => Some(moved.state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [
            Lifecycle::Grilling,
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
        ],
        "with Implementing on the way, unlike a roadmap: an inline Conversation's \
         own work is the building, and the whole of it was that one session's",
    );

    assert!(
        git(&worktree, &["log", "--oneline"]).contains("feat: rate limiting"),
        "the session commits its work with nothing to ask and nobody to ask",
    );
    assert_eq!(
        git(&worktree, &["status", "--porcelain"]),
        "",
        "and the handoff is not in there to be swept into it",
    );
}

/// And an inline grilling that goes quiet without writing one: the run stops,
/// the way every other step that never landed does.
///
/// The handoff is what the session that builds is primed with, so a session that
/// ended without one has left the work half handed over — and nothing here
/// guesses at whether that was a crash, an agent that stopped short, or one that
/// decided it had nothing to say. Driving stops and the human is told.
#[tokio::test]
async fn an_inline_grilling_that_writes_no_handoff_halts_the_run() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'the grilling is running\n'
            while [ ! -f /tmp/verkstead/go ]; do sleep 0.1; done
            printf 'I have nothing more to add\n'
            exit 0
            ;;
        *)
            printf 'model=%s\n' "$1"
            printf 'prompt=%s\n' "$2"
            printf 'a limiter\n' > limiter.md
            git add limiter.md
            git commit --quiet -m 'feat: rate limiting'
            ;;
        esac
        "#,
    )
    .await;

    let grilled = fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    // The grilling stops where it is, with the pick answered and nothing written
    // in its directory.
    std::fs::write(handoff_directory(&fixture).join("go"), "").unwrap();

    let stopped = fixture.stopped().await;

    assert!(
        stopped
            .html
            .contains("Writing the handoff for the session that builds"),
        "which step failed: the inline tail is the handoff, and it did not land: {:?}",
        stopped.html,
    );
    assert!(
        stopped
            .html
            .contains("the session ended without finishing the step"),
        "and an agent that stops short exits zero, so nothing but this could say \
         it: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "a session that ended short is a brake Verkstead pulled, so a restart \
         leaves it alone",
    );

    let view = fixture.view().await;

    assert_eq!(handoff(&view), None, "there is none to have been taken");
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "and the badge points at the Notice that says so",
    );
    assert_eq!(
        view.state,
        Lifecycle::Grilling,
        "and nothing moved: the artifact is what moves a Conversation",
    );
    assert_eq!(
        outputs(&view)
            .into_iter()
            .filter(|output| output.id != grilled)
            .count(),
        0,
        "with nothing launched behind it",
    );
}

/// A grilling picked on twice: the later pick is the one watched for, and the
/// artifact the earlier one asked for moves nothing.
///
/// Which is the whole of *the pick informs, the artifact moves*. A pick lets the
/// session proceed and never makes it, so between one and the artifact the
/// session may come back with another Set instead — and where that Set carries a
/// proposal of its own, a pick on it supersedes. Exactly one watcher is live from
/// that moment, and it is watching for what the human last asked for.
///
/// The stub writes the superseded artifact anyway, which is what makes this prove
/// anything: a handoff appearing on a Conversation whose pick has moved on is a
/// document nobody asked for, and nothing may act on it.
#[tokio::test]
async fn a_later_pick_moves_the_watcher_onto_the_artifact_it_asked_for() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'the grilling is running\n'
            while [ ! -f /tmp/verkstead/handoff-now ]; do sleep 0.1; done
            printf '# What we settled\n\nAn in-process counter.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            while [ ! -f /tmp/verkstead/backlog-now ]; do sleep 0.1; done
            mkdir -p .tasks
            printf '# Rate limiting\n\n## Tasks\n\n- [ ] 01: count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-counter.md
            git add .tasks
            git commit --quiet -m 'chore: plan rate-limiting tasks'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            printf 'model=%s\n' "$1"
            printf 'prompt=%s\n' "$2"
            sleep 300
            ;;
        esac
        "#,
    )
    .await;

    let grilled = fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    // The first proposal, picked inline: from here the handoff is what this
    // Conversation is waiting on.
    let first = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(first, "inline").await, Submitted::Accepted);

    // The session judged that something was still open and came back with
    // another proposal rather than writing anything, and this time they picked
    // the other way.
    let second = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(second, "task-list").await, Submitted::Accepted);

    assert_eq!(
        fixture.view().await.direction,
        Some(verkstead_schema::Direction::TaskList),
        "the latest pick is the one in force",
    );

    // Now the artifact the superseded pick asked for. The watcher that would have
    // taken it was cancelled when the second pick armed its own.
    std::fs::write(handoff_directory(&fixture).join("handoff-now"), "").unwrap();

    // Written and sitting there, which is what makes the rest of this a test: a
    // handoff nothing takes has to be one there was something to take.
    let written = handoff_directory(&fixture).join("handoff.md");
    let deadline = Instant::now() + *PATIENCE;
    while !written.is_file() {
        assert!(
            Instant::now() < deadline,
            "the stub never wrote the handoff the superseded pick asked for",
        );
        pause(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls than the handoff watcher would have needed:
    // it wakes every 100ms and ends a session on 300ms of quiet.
    pause(Duration::from_millis(1500)).await;

    let view = fixture.view().await;

    assert_eq!(
        handoff(&view),
        None,
        "nothing took it: the pick that asked for one has been superseded",
    );
    assert_eq!(
        view.state,
        Lifecycle::Grilling,
        "so nothing moved, and the grilling is still what is happening",
    );
    assert_eq!(
        outputs(&view)
            .into_iter()
            .filter(|output| output.id != grilled)
            .count(),
        0,
        "with nothing launched behind it",
    );

    // And the artifact the pick in force asked for, which is what does move it.
    std::fs::write(handoff_directory(&fixture).join("backlog-now"), "").unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Implementing).then_some(()))
        .await;

    let worked = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id != grilled && output.lines > 0)
                .map(|output| output.id)
        })
        .await;

    let said = fixture.capture(worked).await.replace("\r\n", "\n");

    assert!(
        said.contains("model=claude-implementation-5")
            && said.contains("/verkstead/skills/next-task/SKILL.md"),
        "the backlog is being worked, which is where a task-list pick leads: {said:?}"
    );
    assert_eq!(
        handoff(&fixture.view().await),
        None,
        "and the handoff was never taken, on a direction that needs none",
    );
}

/// A server restarted between the pick and the artifact grills the work again.
///
/// The pick is a row and survives; the grilling session that would have written
/// the artifact was a process and did not. Nobody decided that, so nothing is
/// raised about it: the restart presses Resume for itself, and what Resume means
/// on a Conversation that is grilling is a fresh grilling — the same Brief, and a
/// digest of what the dead interview already settled.
///
/// Which is the whole of what this replaces. A restart used to leave a stop with
/// the tail named on it and wait for somebody to press a button about a
/// Conversation nothing had chosen to stop.
///
/// A second server over the same database, which is what a restart is here: what
/// a pick armed lives in the process that armed it.
#[tokio::test]
async fn a_restarted_server_grills_the_work_again_rather_than_raising_anything() {
    let fixture = grilling(
        r#"
        printf 'the grilling is running\n'
        sleep 300
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    assert!(
        notices(&fixture.view().await).is_empty(),
        "the first server is watching it happily, which is what makes this prove \
         anything",
    );

    let before = outputs(&fixture.view().await).len();

    let _restarted = fixture
        .restarted(r#"printf 'prompt was: %s\n' "$2""#, PULL_REQUEST)
        .await;

    // The session the second server started, with nobody having pressed
    // anything: a restart takes up everything the last one was driving.
    let grilling_again = fixture
        .until(|view| {
            let sessions = outputs(view);
            (sessions.len() > before).then(|| sessions[before].id)
        })
        .await;

    let said = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id == grilling_again && output.lines > 1)
                .map(|output| output.id)
        })
        .await;

    let printed = fixture.capture(said).await.replace("\r\n", "\n");

    assert!(
        printed.contains("/verkstead/skills/grilling/SKILL.md"),
        "a grilling started again is a grilling: {printed:?}",
    );
    assert!(
        printed.contains("The API has none."),
        "on the Brief it was always about: {printed:?}",
    );

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "and nothing was written about the session the restart killed: {:?}",
        notices(&view),
    );
    assert_eq!(
        view.blocked_on, None,
        "nothing is waiting on the human, because nothing is waiting at all",
    );
    assert_eq!(
        view.state,
        Lifecycle::Grilling,
        "which is where a fresh grilling leaves it: an interview that has to be \
         had again is still the interview",
    );
}

/// A sandbox that will not start says why where somebody is looking.
///
/// bwrap complains on its own stderr, which is the terminal Verkstead started
/// it on — so what it said is in the Capture of the session that failed, where
/// it happened and among whatever else was printed. The Event would otherwise
/// read `0 lines` with the only account of why in a log at a level nobody has
/// turned on, which is a failure that looks exactly like an agent with nothing
/// to say.
///
/// Provoked through a configured bind, because that is the one part of a sandbox
/// that is checked at startup and can go on to be missing: what fails here is
/// bwrap and nothing else, on a Conversation whose worktree and Profile are both
/// fine.
#[tokio::test]
async fn a_sandbox_that_will_not_start_says_why_on_the_capture() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'the grilling is running\n'
            while [ ! -f /tmp/verkstead/go ]; do sleep 0.1; done
            printf '# What we settled\n\nAn in-process counter.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            printf 'this session never gets to run\n'
            ;;
        esac
        "#,
    )
    .await;

    // The grilling holds off writing its handoff until the test says so, because
    // the handoff is what ends it: the bind has to be gone before the session
    // that wants it is launched, and that session follows the grilling ending.
    let grilled = fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    // The bind the server admitted at startup, gone by the time the next session
    // wants it.
    let missing = fixture.spill.path().to_owned();
    std::fs::remove_dir_all(&missing).unwrap();

    std::fs::write(handoff_directory(&fixture).join("go"), "").unwrap();

    let refused = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id != grilled && !output.running)
                .map(|output| (output.id, output.lines, output.latest.clone()))
        })
        .await;

    let (event, lines, latest) = refused;
    let said = fixture.capture(event).await;

    assert!(
        said.contains(&missing.display().to_string()),
        "the sandbox's own complaint goes on the Capture of the session that \
         failed, naming the bind bwrap could not make: {said:?}"
    );
    assert!(
        lines > 0 && !latest.is_empty(),
        "so the Timeline says what happened rather than `0 lines` and nothing: \
         {lines} lines, latest {latest:?}"
    );
}

/// The task-list direction end to end: the grilling session reads on into the
/// bundled fork of to-tasks and commits a real `.tasks/` backlog to the branch,
/// without anything else being launched.
///
/// The stub writes its backlog as it starts rather than when the Response lands,
/// because a stub cannot idle on a blocking ask — nothing in these fixtures dials
/// the router. That costs the test nothing: what Verkstead does with a task-list
/// pick is launch nothing and start watching, and a backlog already committed is
/// watched for exactly as one committed a minute later is.
///
/// Repo files stay the source of truth, so what this asks of the far end is what
/// git says: the backlog is in the worktree and it is committed. Verkstead runs
/// the workflow that writes it and owns none of it.
#[tokio::test]
async fn choosing_a_task_list_breaks_the_work_down_in_the_grilling_session() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'model=%s\n' "$1"
            grep '^name:' "/verkstead/skills/breaking-down/SKILL.md"
            printf '# A document nobody asked for\n' > /tmp/verkstead/handoff.md
            mkdir -p .tasks
            printf '# Rate limiting\n\n## Tasks\n\n- [ ] 01: count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-counter.md
            git add .tasks
            git commit --quiet -m 'chore: plan rate-limiting tasks'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            printf 'model=%s\n' "$1"
            printf 'prompt=%s\n' "$2"
            sleep 300
            ;;
        esac
        "#,
    )
    .await;

    let grilling_output = fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let worktree = PathBuf::from(
        fixture
            .view()
            .await
            .worktree
            .expect("a grilling Conversation has a worktree")
            .path,
    );

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    // The plan commit plus quiet is what ends the grilling, and the move that
    // follows it is what says the work is being built.
    fixture
        .until(|view| (view.state == Lifecycle::Implementing).then_some(()))
        .await;

    let said = fixture.capture(grilling_output).await.replace("\r\n", "\n");

    assert!(
        said.contains("model=claude-grilling-5"),
        "the backlog is written by the session that settled it, under the grilling \
         Profile it has been running as all along: {said:?}"
    );
    assert!(
        said.contains("name: breaking-down"),
        "reading on into the bundled fork of to-tasks, which is really there to be \
         read from a grilling sandbox: {said:?}"
    );

    assert!(
        git(&worktree, &["log", "--oneline"]).contains("chore: plan rate-limiting tasks"),
        "the plan commit is not something the fork asks permission for",
    );
    assert_eq!(
        git(&worktree, &["status", "--porcelain"]),
        "",
        "and the backlog is committed rather than left in the worktree",
    );
    assert!(
        worktree.join(".tasks/TODO.md").is_file()
            && worktree.join(".tasks/01-counter.md").is_file(),
        "a real `.tasks/` backlog: TODO.md and the numbered files beside it",
    );
    assert_eq!(
        git(&worktree, &["branch", "--show-current"]).trim(),
        fixture.view().await.branch,
        "on the branch the Conversation already made — the fork creates none",
    );

    // What follows the backlog is the task run, under the Profile that builds:
    // no second planning session anywhere on the Timeline.
    let worked = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id != grilling_output && output.lines > 0)
                .map(|output| output.id)
        })
        .await;

    let next = fixture.capture(worked).await.replace("\r\n", "\n");

    assert!(
        next.contains("model=claude-implementation-5"),
        "the task run is the implementation Profile's, as the work it does is: {next:?}"
    );
    assert!(
        next.contains("/verkstead/skills/next-task/SKILL.md")
            && !next.contains("/verkstead/skills/breaking-down/SKILL.md"),
        "and it is the first task rather than a second breakdown: {next:?}"
    );

    // And no handoff anywhere in it. The backlog is what the grilling settled,
    // committed to the branch, so a document beside it would be a second record
    // of the plan that nothing downstream reads — which is why the stub wrote one
    // and nothing went looking for it.
    assert_eq!(
        handoff(&fixture.view().await),
        None,
        "nothing was taken onto the Timeline",
    );
    assert!(
        !next.contains("What the grilling settled"),
        "and nothing was folded into the task session's prompt: {next:?}"
    );
}

/// The roadmap direction end to end: the grilling session reads on into the
/// bundled fork of to-roadmap and commits a real `docs/roadmaps/<name>/` to the
/// branch, without anything else being launched.
///
/// The stub writes its roadmap as it starts rather than when the Response lands,
/// for the reason the task-list one does: a stub cannot idle on a blocking ask,
/// and a roadmap already committed is watched for exactly as one committed a
/// minute later is.
///
/// Repo files stay the source of truth here too, so what this asks of the far
/// end is what git says, and what it asks of Verkstead is the reading it draws
/// back off that: the roadmap the branch wrote is pinned as the stage list, with
/// its stages in the roadmap's own order and the boxes as the roadmap wrote
/// them.
///
/// The session idles after its commit, which is what a real interactive one
/// does. So what ends it is Verkstead's own done-signal — a roadmap on the
/// branch that was not there before, committed, and then quiet.
#[tokio::test]
async fn choosing_a_roadmap_stages_the_work_in_the_grilling_session() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'model=%s\n' "$1"
            grep '^name:' "/verkstead/skills/staging/SKILL.md"
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            mkdir -p docs/roadmaps/rate-limiting
            printf '# Rate limiting roadmap\n\n## Stages\n\n- [x] 01: Count the requests — [brief](01-counter.md)\n- [ ] 02: Refuse the rest — [brief](02-refusing.md)\n' > docs/roadmaps/rate-limiting/ROADMAP.md
            printf '# 01. Count the requests\n' > docs/roadmaps/rate-limiting/01-counter.md
            printf '# 02. Refuse the rest\n' > docs/roadmaps/rate-limiting/02-refusing.md
            git add docs
            git commit --quiet -m 'docs: stage the rate-limiting roadmap'
            printf 'the roadmap is written\n'
            sleep 300
            ;;
        *)
            printf 'model=%s\n' "$1"
            printf 'prompt=%s\n' "$2"
            sleep 300
            ;;
        esac
        "#,
    )
    .await;

    let grilling_output = fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let worktree = PathBuf::from(
        fixture
            .view()
            .await
            .worktree
            .expect("a grilling Conversation has a worktree")
            .path,
    );

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "roadmap").await, Submitted::Accepted);

    // A roadmap is work like any other work: the same session carries the branch
    // to a pull request the way a finished backlog does, and the Conversation
    // moves on to wrapping that up.
    let opened = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping)
                .then(|| pull_request(view).cloned())
                .flatten()
        })
        .await;

    assert_eq!(opened.number, 41);

    let said = fixture.capture(grilling_output).await.replace("\r\n", "\n");

    assert!(
        said.contains("model=claude-grilling-5"),
        "the roadmap is written by the session that settled it, under the grilling \
         Profile it has been running as all along: {said:?}"
    );
    assert!(
        said.contains("name: staging"),
        "reading on into the bundled fork of to-roadmap, which is really there to be \
         read from a grilling sandbox: {said:?}"
    );

    assert_eq!(
        outputs(&fixture.view().await)
            .into_iter()
            .filter(|output| output.id != grilling_output && output.lines > 0)
            .count(),
        0,
        "and nothing else was launched: the staging is the grilling carrying on",
    );

    // And no handoff anywhere in it either. A roadmap Conversation crosses into
    // no fresh context that has to be told what was settled — the stage briefs
    // say it, in the repository, and each Stage has a grilling of its own — so
    // the document the stub wrote is one nothing goes looking for.
    assert_eq!(
        handoff(&fixture.view().await),
        None,
        "nothing was taken onto the Timeline",
    );

    assert_eq!(
        fixture
            .view()
            .await
            .timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::Moved(moved) => Some(moved.state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [Lifecycle::Grilling, Lifecycle::Wrapping],
        "with no Implementing on the way: on a roadmap the building belongs to the \
         Stages, and this Conversation's own work is the planning",
    );

    assert!(
        git(&worktree, &["log", "--oneline"]).contains("docs: stage the rate-limiting roadmap"),
        "the roadmap commit is not something the fork asks permission for",
    );
    assert_eq!(
        git(&worktree, &["status", "--porcelain"]),
        "",
        "and the roadmap is committed rather than left in the worktree",
    );
    assert!(
        worktree
            .join("docs/roadmaps/rate-limiting/ROADMAP.md")
            .is_file()
            && worktree
                .join("docs/roadmaps/rate-limiting/01-counter.md")
                .is_file(),
        "a real roadmap: ROADMAP.md and a brief per stage beside it",
    );

    // And what Verkstead makes of it, which is the other half of the direction:
    // the roadmap the branch wrote, read back off the Worktree and pinned.
    let stages = fixture
        .view()
        .await
        .pinned
        .into_iter()
        .find_map(|pinned| match pinned {
            verkstead_render::PinnedEvent::StageList(list) => Some(list),
            _ => None,
        })
        .expect("the roadmap this branch wrote is pinned as the stage list");

    assert_eq!(stages.name, "rate-limiting");
    assert_eq!(stages.title, "Rate limiting roadmap");
    assert_eq!(
        stages
            .stages
            .iter()
            .map(|stage| (stage.number.as_str(), stage.title.as_str(), stage.done))
            .collect::<Vec<_>>(),
        [
            ("01", "Count the requests", true),
            ("02", "Refuse the rest", false),
        ],
        "the roadmap's own order, numbers and titles, and the boxes as it wrote them",
    );

    // And the same roadmap on the record, at the row the landing stamped —
    // before the pull request the same session went on to open, which is the
    // order the two happened in.
    let view = fixture.view().await;
    let reached = roadmap_row(&view).expect("the roadmap landing is on the record");

    assert_eq!(
        reached.roadmaps,
        [stages],
        "the record draws the pinned card itself",
    );

    let at = view
        .timeline
        .iter()
        .position(|event| matches!(event, TimelineEvent::StageList(_)));
    let opened = view
        .timeline
        .iter()
        .position(|event| matches!(event, TimelineEvent::PullRequest(_)));

    assert!(
        at < opened,
        "the roadmap landed before the branch went up for review: {:?}",
        view.timeline,
    );
}

/// The backlog working itself: once `.tasks/` is committed, Verkstead launches
/// a fresh session for the lowest-numbered task, ends it once the task has
/// landed, and launches the next — through to the finish step, with no gate
/// anywhere in it and nobody asked.
///
/// Every session here idles after its commit, which is what a real interactive
/// one does: nothing exits, so what advances the run is the runner ending each
/// session on its done-signal plus quiet. A stub that exited would prove the
/// loop counts to four and nothing about the part that is hard.
///
/// The stub decides what it is by looking at `.tasks/`, exactly as the bundled
/// fork does — every box ticked means the finish step — so what this asserts is
/// that Verkstead and the fork read the same backlog the same way.
#[tokio::test]
async fn a_committed_backlog_works_itself_one_fresh_session_per_task() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'breaking down\n'
            mkdir -p .tasks
            printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
            printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
            printf -- '- [ ] 02: refuse the excess\n' >> .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            printf '# 02. Refuse the excess\n' > .tasks/02-refuse.md
            git add .tasks
            git commit --quiet -m 'chore: plan rate-limiting tasks'
            sleep 300
            ;;
        *)
            case "$2" in
            *reviewing/SKILL.md*)
                printf 'I read the whole branch and found nothing worth raising\n'
                exit 0
                ;;
            esac
            number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
            next=$(ls .tasks | grep -E "^$number-" | head -n 1)
            if [ -n "$next" ]; then
                printf 'working %s\n' "$next"
                printf 'skill=%s\n' "$(grep '^name:' "/verkstead/skills/next-task/SKILL.md")"
                printf 'a limiter\n' >> limiter.md
                sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
                git add -A
                git commit --quiet -m "feat: $next"
            else
                printf 'finishing\n'
                git rm --quiet -r .tasks
                git commit --quiet -m 'chore: finish rate-limiting'
            fi
            sleep 300
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let worktree = PathBuf::from(
        fixture
            .view()
            .await
            .worktree
            .expect("a grilling Conversation has a worktree")
            .path,
    );

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    // The breakdown lands, and the runner is watching it: nothing here presses
    // anything again.
    let landed = fixture
        .until(|view| {
            let landed = commits(view);
            (landed.len() == 4).then(|| {
                landed
                    .iter()
                    .map(|commit| commit.subject.clone())
                    .collect::<Vec<_>>()
            })
        })
        .await;

    assert_eq!(
        landed,
        vec![
            "chore: plan rate-limiting tasks".to_owned(),
            "feat: 01-count.md".to_owned(),
            "feat: 02-refuse.md".to_owned(),
            "chore: finish rate-limiting".to_owned(),
        ],
        "the backlog in order, then the finish step — each by a session of its own",
    );

    // The finish step is not the last session a backlog leads to: the pull
    // request it opened is reviewed, which is where the wrap-up starts.
    fixture
        .until(|view| {
            let sessions = outputs(view);
            (sessions.len() == 5 && sessions.iter().all(|output| !output.running)).then_some(())
        })
        .await;

    let view = fixture.view().await;

    assert_eq!(
        outputs(&view).len(),
        5,
        "one Event per session: the grilling, which broke the work down itself, a task \
         each, the finish, and the review of the pull request it opened",
    );
    assert!(
        outputs(&view).iter().all(|output| !output.running),
        "every one of them was ended once its step landed and it had gone quiet, though \
         each was still sitting on its `sleep`",
    );

    let worked: Vec<String> = outputs(&view)
        .iter()
        .map(|output| output.latest.clone())
        .collect();
    assert!(
        worked.iter().any(|said| said.contains("name: next-task")),
        "the task sessions run inside the bundled fork of next-task: {worked:?}",
    );

    assert!(
        !worktree.join(".tasks").exists(),
        "the finish step took the backlog away, which is what says the feature is done",
    );
    assert_eq!(
        git(&worktree, &["status", "--porcelain"]),
        "",
        "and left nothing uncommitted behind it",
    );

    // Long enough for many more polls of a backlog that has nothing left in it.
    pause(Duration::from_secs(2)).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        5,
        "an empty backlog leaves the runner idle rather than launching sessions at nothing",
    );
}

/// The pinned Event is read off the Worktree rather than remembered, so the
/// backlog the human is watching ticks along as the runner works it — without
/// anything writing the list down twice.
#[tokio::test]
async fn the_pinned_task_list_ticks_along_as_the_runner_works_it() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'grilling\n'
            mkdir -p .tasks
            printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
            printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
            printf -- '- [ ] 02: refuse the excess\n' >> .tasks/TODO.md
            printf '# 01\n' > .tasks/01-count.md
            printf '# 02\n' > .tasks/02-refuse.md
            git add .tasks
            git commit --quiet -m 'chore: plan rate-limiting tasks'
            sleep 300
            ;;
        *)
            number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
            next=$(ls .tasks | grep -E "^$number-" | head -n 1)
            if [ -n "$next" ]; then
                sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
                git add -A
                git commit --quiet -m "feat: $next"
                # Only the first task, so the list is caught half worked
                # through rather than empty.
                sleep 300
            fi
            printf 'stopping\n'
            sleep 300
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    // The backlog as the breakdown wrote it: nothing done yet.
    let written = fixture
        .until(|view| backlog(view).filter(|list| list.tasks.len() == 2).cloned())
        .await;

    assert_eq!(written.feature, "Rate limiting");
    assert_eq!(
        written
            .tasks
            .iter()
            .map(|task| (task.number.as_str(), task.done))
            .collect::<Vec<_>>(),
        [("01", false), ("02", false)],
    );

    // And once the runner has seen the first task out.
    let worked = fixture
        .until(|view| backlog(view).filter(|list| list.tasks[0].done).cloned())
        .await;

    assert_eq!(
        worked
            .tasks
            .iter()
            .map(|task| task.done)
            .collect::<Vec<_>>(),
        [true, false],
        "the task whose entry is ticked is done, and the one still to do is not",
    );

    // And the same list is on the record, at the row the backlog landing
    // stamped. One card in two places: the row fixes where it landed and the
    // card at it is the pinned card's own reading, so it has ticked along with
    // the work exactly as the pinned one has.
    let view = fixture.view().await;
    let reached = backlog_row(&view).expect("the backlog landing is on the record");

    assert_eq!(
        reached.list.as_ref(),
        backlog(&view),
        "the record draws the pinned card itself",
    );

    // Where it landed, which is before the move it was made on the strength of:
    // the plan commit is the end of the planning and the start of the run.
    let at = view
        .timeline
        .iter()
        .position(|event| matches!(event, TimelineEvent::TaskList(_)));
    let moved = view.timeline.iter().position(
        |event| matches!(event, TimelineEvent::Moved(moved) if moved.state == Lifecycle::Implementing),
    );

    assert!(
        at < moved,
        "the backlog is on the record before the move it wrote: {:?}",
        view.timeline,
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
}

/// One task and then the finish, which is the shortest whole backlog there is —
/// and the one the wrap-up tests below drive to its end.
///
/// The wrap-up's review is played too, because every backlog worked to empty
/// gets one: it reads the branch, finds nothing worth raising, and gets out of
/// the way. A stub that fell through to the backlog arm would find `.tasks` gone
/// and write a whole second backlog.
///
/// The finish step here does what the bundled fork tells a session to do:
/// commits the removal of `TODO.md`, and pushes and opens a pull request through
/// its own `gh`. There is no remote to push to in these fixtures, so the stub
/// says it and stops there — what Verkstead does next is ask the *host's* `gh`,
/// which is the half under test.
const A_BACKLOG_OF_ONE: &str = r#"
case "$1" in
claude-grilling-5|gpt-5-codex-grilling)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf 'I read the whole branch and found nothing worth raising\n'
        exit 0
        ;;
    esac
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: count the requests"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
        printf 'pushed, and the pull request is open\n'
    fi
    sleep 300
    ;;
esac
"#;

/// The same backlog again, in a Conversation with a read-write companion — and a
/// finish that carries that companion to a pull request of its own, which is what
/// the bundled forks tell a session to do about every repository it committed in.
///
/// The companion's commit lands after the finish commit and before the session
/// says anything, exactly as a finish sequence worked in order leaves it: the
/// Conversation's own repository first, then each companion in its own worktree.
const A_BACKLOG_ALONGSIDE: &str = r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf 'I read the whole branch and found nothing worth raising\n'
        exit 0
        ;;
    esac
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: count the requests"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
        cd ../askance-*
        printf 'the other half\n' > halves.md
        git add halves.md
        git commit --quiet -m 'feat: the other half'
        printf 'pushed both, and the pull requests are open\n'
    fi
    sleep 300
    ;;
esac
"#;

/// The same backlog, with the session Verkstead sends after a finish that opened
/// nothing doing the one thing it is sent for.
///
/// `opened` is this fixture's stand-in for a pull request appearing on GitHub:
/// the `gh` beside it answers *no pull request* until the file is there and
/// answers #41 once it is, which is the change the session's own push and
/// `gh pr create` would make. See [`gh_opened_by_hand`].
///
/// The finish itself is unchanged — it commits the list away and gets no
/// further, which is what a session that stopped short of its push leaves
/// behind.
fn a_backlog_whose_pull_request_is_opened_when_asked(opened: &Path) -> String {
    format!(
        r#"
case "$2" in
*submitting/SKILL.md*)
    printf 'prompt was: %s\n' "$2"
    printf 'the branch is pushed and the pull request is open\n'
    printf 'https://github.com/tobico/verkstead/pull/41\n' > {opened}
    exit 0
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#,
        opened = quoted(opened),
    )
}

/// And the same again where the session sent for the pull request cannot open one
/// either: it says why and exits, which is what a `gh` nobody logged in looks
/// like from inside a session.
fn a_backlog_that_cannot_open_a_pull_request() -> String {
    format!(
        r#"
case "$2" in
*submitting/SKILL.md*)
    printf 'prompt was: %s\n' "$2"
    printf 'gh is not logged in here, so there is no pull request to open\n'
    exit 1
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#
    )
}

/// And the same again where the first session sent for the pull request cannot
/// open one either, and the second — the one a press of Resume sends — can.
///
/// Which is the shape the button is for: the run has its go, stops on the same
/// missing thing, and the human presses when whatever was in the way is out of
/// it. `tried` is how the stub remembers which of the two it is in.
fn a_backlog_whose_pull_request_takes_two_asks(tried: &Path, opened: &Path) -> String {
    format!(
        r#"
case "$2" in
*submitting/SKILL.md*)
    printf 'prompt was: %s\n' "$2"
    if [ ! -f {tried} ]; then
        printf 'no remote to push to, so there is no pull request to open\n'
        printf 'once\n' > {tried}
        exit 1
    fi
    printf 'the branch is pushed and the pull request is open\n'
    printf 'https://github.com/tobico/verkstead/pull/41\n' > {opened}
    exit 0
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#,
        tried = quoted(tried),
        opened = quoted(opened),
    )
}

/// Take a Conversation from the pick on its closing Set to a worked-through
/// backlog, with nothing pressed on the way: the whole point of the run is that
/// nobody is asked anything between the direction and the pull request.
async fn worked_to_empty(fixture: &Grilling) {
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    // And nobody is asking again from here, which the sessions that come after
    // this one have to be told — see [`Grilling::asked_nothing`]. A wrap-up's
    // review is one of them, and it talks until the test puts its Set up.
    fixture.asked_nothing();

    fixture
        .until(|view| {
            commits(view)
                .iter()
                .any(|commit| commit.subject.starts_with("chore: finish"))
                .then_some(())
        })
        .await;
}

/// The whole of what a finished backlog leaves behind: a pull request open on
/// the branch, the Conversation wrapping it up, and the move on the Timeline —
/// with nothing having asked for approval at any point.
#[tokio::test]
async fn a_backlog_worked_to_empty_leaves_a_pull_request_pinned_and_the_work_wrapping() {
    let fixture = grilling(A_BACKLOG_OF_ONE).await;

    worked_to_empty(&fixture).await;

    // Kept as it was read rather than read again afterwards: what is being
    // asked about is the moment the work starts wrapping, and a wrap-up with
    // nothing outstanding settles itself without waiting for anybody — so a
    // second read is as likely to be of a Conversation that has already
    // finished.
    let view = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping && pull_request(view).is_some())
                .then(|| view.clone())
        })
        .await;

    let opened = pull_request(&view).expect("the wrap-up has its pull request pinned");

    assert_eq!(opened.number, 41);
    assert_eq!(opened.title, "Rate limiting");
    assert_eq!(opened.url, "https://github.com/tobico/verkstead/pull/41");

    // And the same pull request is on the record where it happened: pinned is
    // something an Event is *as well as* being listed, so the sticky block
    // holds it in view and the record keeps the moment the work went up for
    // review. One card in two places, so both are the same Event.
    let listed = view
        .timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::PullRequest(opened) => Some(opened),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        listed,
        [opened],
        "the record carries the pinned card itself"
    );

    // Where it happened, which is before the move it wrote.
    let at = view
        .timeline
        .iter()
        .position(|event| matches!(event, TimelineEvent::PullRequest(_)));
    let moved = view.timeline.iter().rposition(
        |event| matches!(event, TimelineEvent::Moved(moved) if moved.state == Lifecycle::Wrapping),
    );

    assert!(
        at < moved,
        "the PR is on the record before the move it wrote"
    );

    // The move is on the record like every other, and it is the last thing to
    // have happened.
    assert_eq!(
        view.timeline.iter().rev().find_map(|event| match event {
            TimelineEvent::Moved(moved) => Some(moved.state),
            _ => None,
        }),
        Some(Lifecycle::Wrapping),
    );

    // Nothing waited on anybody: the only Set on the Timeline is the proposal
    // that ended the grilling, and nothing is blocked.
    assert_eq!(sets(&view).len(), 1);
    assert!(view.blocked_on.is_none());
    assert!(
        notices(&view).is_empty(),
        "nothing stopped: {:?}",
        notices(&view),
    );

    // Nothing has written down how the checks are: this `gh` answers the
    // watcher's question with a pull request that has no suite on it.
    assert_eq!(check_rollup(&fixture).await, None);

    // Nor where the pull request has got to, which the watcher never asks about:
    // whether anybody has merged the work yet is no business of a Conversation
    // still wrapping it up.
    assert_eq!(recorded_standing(&fixture).await, None);

    // And what is on the PR is fetched when the pane opens it, through the same
    // host `gh` — never written down.
    let carried: verkstead_render::PullRequestDetails = get(
        &fixture.app,
        &format!(
            "/api/ui/conversations/{}/pull-request/{}",
            fixture.id, opened.id
        ),
    )
    .await;

    assert_eq!(carried.commits.len(), 1);
    assert_eq!(carried.commits[0].subject, "feat: count the requests");
    assert_eq!(carried.comments.len(), 1);
    assert!(carried.comments[0].html.contains("<strong>good</strong>"));

    // The checks come back on the same answer, each with the run to follow.
    assert_eq!(
        carried
            .checks
            .iter()
            .map(|check| (check.name.as_str(), check.how, check.link.as_str()))
            .collect::<Vec<_>>(),
        [(
            "Rust",
            verkstead_render::Checked::Passed,
            "https://github.com/tobico/verkstead/actions/runs/1/job/2",
        )],
    );

    // And opening it is what freshens the rollup the card draws: the watcher
    // stops when the wrap-up is over, so on a Conversation nothing is watching
    // the pane is the one thing left that asks GitHub.
    assert_eq!(
        check_rollup(&fixture).await,
        Some(verkstead_server::store::Rollup::Passed),
    );

    // And the two facts about the pull request itself that rode the same answer,
    // which a card is drawn off as much as the icon is: it merges, and it is
    // open. Whatever state the Conversation is in — this one is still Wrapping,
    // and the sweep that keeps them fresh does not start until Done.
    assert_eq!(
        recorded_merging(&fixture).await,
        Some(verkstead_server::store::Merging::Cleanly),
    );
    assert_eq!(
        recorded_standing(&fixture).await,
        Some(verkstead_server::store::Standing::Open),
        "which nothing but the pane could have written down here",
    );
}

/// The same backlog, plus a session that plays a fix: it writes down the prompt
/// it was given — somewhere that outlives the worktree — and commits something,
/// which is what a fix session reports through.
///
/// Told apart from the backlog's sessions by the skill its prompt names rather
/// than by the model, because that is the fact under it: a fix session runs
/// under the *same* implementation Profile as the tasks did and differs only in
/// what it was sent to do.
fn a_backlog_then_fixes(prompts: &Path) -> String {
    format!(
        r#"
case "$2" in
*addressing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {prompts}
    printf 'having a go at the check\n'
    printf 'a fix\n' >> fixes.md
    git add -A
    git commit --quiet -m 'fix: have a go at the failing check'
    sleep 300
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#,
        prompts = quoted(prompts),
    )
}

/// How many fix sessions have run, by the commits they left on the branch.
fn fixes(view: &ConversationView) -> usize {
    commits(view)
        .iter()
        .filter(|commit| commit.subject.starts_with("fix:"))
        .count()
}

/// Whether Verkstead has recorded this Conversation's own pull request's checks
/// as green.
///
/// Read out of the store rather than off the Timeline, because that is where it
/// is: settling is bookkeeping about what wrap-up is still waiting on rather
/// than something that happened, and nothing that is not an Event goes on a
/// Timeline.
///
/// The Conversation's own, there being one suite per pull request: a companion's
/// is [`companion_checks_settled`]'s.
async fn checks_settled(fixture: &Grilling) -> bool {
    settled(
        fixture,
        verkstead_server::store::WaitingOn::Checks(own_repo(fixture).await),
    )
    .await
}

/// And whether Verkstead has recorded that GitHub can merge it, which is the
/// other thing one poll of the checks watcher settles.
async fn merge_settled(fixture: &Grilling) -> bool {
    settled(
        fixture,
        verkstead_server::store::WaitingOn::Mergeable(own_repo(fixture).await),
    )
    .await
}

/// And whether the pull request opened in the companion beside it is green.
async fn companion_checks_settled(fixture: &Grilling) -> bool {
    settled(
        fixture,
        verkstead_server::store::WaitingOn::Checks(companion_repo(fixture).await),
    )
    .await
}

/// And whether that one merges, a conflict in a companion being as much a reason
/// to wait as one in the Conversation's own repository.
async fn companion_merge_settled(fixture: &Grilling) -> bool {
    settled(
        fixture,
        verkstead_server::store::WaitingOn::Mergeable(companion_repo(fixture).await),
    )
    .await
}

/// Whether one of the things this wrap-up waits on is settled.
async fn settled(fixture: &Grilling, waiting_on: verkstead_server::store::WaitingOn) -> bool {
    let pool = open_database(&fixture.database).await.unwrap();
    let settled = verkstead_server::store::wrap_up_settled(&pool, fixture.id)
        .await
        .unwrap();
    pool.close().await;

    settled.contains(&waiting_on)
}

/// The Repo the Conversation's own work is in, which is what its own pull
/// request's checks and fix sessions are keyed by.
async fn own_repo(fixture: &Grilling) -> i64 {
    let pool = open_database(&fixture.database).await.unwrap();
    let conversation = verkstead_server::store::load_conversation(&pool, fixture.id)
        .await
        .unwrap()
        .expect("the Conversation is there");
    pool.close().await;

    conversation.repo.id
}

/// And the one beside it, for the fixtures that are configured with a companion.
async fn companion_repo(fixture: &Grilling) -> i64 {
    let pool = open_database(&fixture.database).await.unwrap();
    let conversation = verkstead_server::store::load_conversation(&pool, fixture.id)
        .await
        .unwrap()
        .expect("the Conversation is there");
    pool.close().await;

    conversation
        .companions
        .first()
        .expect("this Conversation was configured with a companion")
        .repo
        .id
}

/// How Verkstead has written down that this Conversation's checks are, or
/// nothing where nothing has asked.
///
/// The rollup the pull request card draws its icon from, read out of the store
/// because that is where it is: what a poll or an opened details pane learned
/// from GitHub outlives both.
async fn check_rollup(fixture: &Grilling) -> Option<verkstead_server::store::Rollup> {
    let pool = open_database(&fixture.database).await.unwrap();
    let rollup = verkstead_server::store::check_rollup(&pool, fixture.id)
        .await
        .unwrap();
    pool.close().await;

    rollup
}

/// How many fix sessions Verkstead has counted against one of this
/// Conversation's checks, on its own pull request.
///
/// The count that *two attempts, then ask* is kept by, read the way the watcher
/// reads it — per check and per pull request, the same check name red on two of
/// them being two different failures. What a check the review folded into its
/// own session has spent is nothing: an attempt is counted where a fix session
/// is dispatched, and none is dispatched into a Worktree the review is holding.
async fn attempts_spent(fixture: &Grilling, check: &str) -> i64 {
    let repo = own_repo(fixture).await;
    let pool = open_database(&fixture.database).await.unwrap();
    let spent = verkstead_server::store::fix_attempts(&pool, fixture.id, repo, check)
        .await
        .unwrap();
    pool.close().await;

    spent
}

/// Write `config.yaml` over with `more` beside the author every sandbox is
/// configured out of — see [`bench_at_pace`], which is what wrote it in the
/// first place.
///
/// The file is read at the moment it is needed rather than held from startup, so
/// a fixture that writes it after its server is up is a human who went to the
/// settings page while the work was going on. Written over rather than added to,
/// which is what lets a test take a setting away again: `configure(&fixture, "")`
/// is the human deleting what they had just written.
fn configure(fixture: &Grilling, more: &str) {
    let path = fixture.state.path().join("config.yaml");

    std::fs::write(&path, format!("{THE_AUTHOR}{more}")).unwrap();
}

/// Who every session in these tests commits as, which is the whole of what a
/// bench's `config.yaml` says until a test writes something else into it.
const THE_AUTHOR: &str = "git_author:\n  name: Verkstead Test\n  email: test@verkstead.invalid\n";

/// And say how one Repo resolves a conflict, which is the override that wins
/// over that file.
async fn told_to_resolve_by(fixture: &Grilling, repo: i64, resolution: ConflictResolution) {
    let view: verkstead_render::RepoView = post(
        &fixture.app,
        &format!("/api/ui/repos/{repo}/resolution"),
        &serde_json::json!({ "resolution": resolution }),
    )
    .await;

    assert_eq!(view.conflict_resolution, Some(resolution));
}

/// What Verkstead has written down about whether this Conversation's own pull
/// request merges, which is the reading a card is drawn off long after anything
/// stopped watching.
///
/// Read out of the store rather than off the Timeline for [`checks_settled`]'s
/// reason: it is a reading of GitHub rather than something that happened.
async fn recorded_merging(fixture: &Grilling) -> Option<verkstead_server::store::Merging> {
    let repo = own_repo(fixture).await;
    let pool = open_database(&fixture.database).await.unwrap();
    let merging = verkstead_server::store::merging(&pool, fixture.id, repo)
        .await
        .unwrap();
    pool.close().await;

    merging
}

/// And where it had got to — open, merged or closed — which is the reading that
/// ends the sweep after Done.
async fn recorded_standing(fixture: &Grilling) -> Option<verkstead_server::store::Standing> {
    let repo = own_repo(fixture).await;
    let pool = open_database(&fixture.database).await.unwrap();
    let standing = verkstead_server::store::standing(&pool, fixture.id, repo)
        .await
        .unwrap();
    pool.close().await;

    standing
}

/// Read what is written down about the merge back until GitHub's latest word is
/// there, or give up.
async fn until_merging(fixture: &Grilling, said: verkstead_server::store::Merging) {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        let read = recorded_merging(fixture).await;

        if read == Some(said) {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "GitHub said the pull request merges {said:?} and nothing wrote it down. \
             What stands is {read:?}",
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// And the same about where it has got to.
async fn until_standing(fixture: &Grilling, said: verkstead_server::store::Standing) {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        let read = recorded_standing(fixture).await;

        if read == Some(said) {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "GitHub said the pull request stands {said:?} and nothing wrote it down. \
             What stands is {read:?}",
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// And how many resolution sessions it has counted against the conflict on that
/// same pull request.
///
/// Counted per pull request and nothing finer, a branch having one base: the
/// count *two goes, then ask* is kept by on the merging side.
async fn conflict_attempts_spent(fixture: &Grilling) -> i64 {
    let repo = own_repo(fixture).await;
    let pool = open_database(&fixture.database).await.unwrap();
    let spent = verkstead_server::store::conflict_fix_attempts(&pool, fixture.id, repo)
        .await
        .unwrap();
    pool.close().await;

    spent
}

/// The ordinary way a wrap-up starts: the finish step opened a pull request and
/// everything GitHub runs against it is green.
///
/// Nothing to fix, so nothing is dispatched — and the checks stop being one of
/// the things wrap-up is waiting on.
#[tokio::test]
async fn a_green_suite_settles_the_checks_and_dispatches_nothing() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture = grilling_spilling(
        prompts,
        &a_backlog_then_fixes(&written),
        &gh_checking("SUCCESS"),
    )
    .await;

    worked_to_empty(&fixture).await;
    fixture
        .until(|view| (view.state == Lifecycle::Wrapping).then_some(()))
        .await;

    let deadline = Instant::now() + *PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(Instant::now() < deadline, "the checks never settled");
        pause(Duration::from_millis(50)).await;
    }

    // Long enough for many more polls of a suite that has nothing wrong with it.
    pause(Duration::from_millis(500)).await;

    let view = fixture.view().await;

    assert_eq!(fixes(&view), 0, "a green check is nothing to fix");
    assert!(
        !written.exists(),
        "and no session was put inside the addressing skill: {:?}",
        std::fs::read_to_string(&written).ok(),
    );
    assert!(
        notices(&view).is_empty(),
        "nothing stopped: {:?}",
        notices(&view),
    );
    assert!(checks_settled(&fixture).await, "and they are still green");
}

/// The whole of what a red check costs: two fix sessions, and then the human.
///
/// The first failure dispatches one fix session inside the bundled addressing
/// skill, under the implementation Profile, given the check as its feedback. It
/// commits, the check is still red, and it gets one more. After that Verkstead
/// stops asking the machine: the run stops, the Notice carries which check failed and
/// what the last session said, and nothing further is dispatched for it.
#[tokio::test]
async fn a_check_two_fix_sessions_could_not_fix_halts_and_tells_the_human() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture = grilling_spilling(
        prompts,
        &a_backlog_then_fixes(&written),
        &gh_checking("FAILURE"),
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    // Which step it was, and what makes the stop readable from a phone: which
    // check is red, where its run is, and what the last fix session said.
    assert!(
        stopped.html.contains("checks"),
        "the step is named as what it was: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("Rust") && stopped.html.contains("/actions/runs/1/job/2"),
        "and the reason names the check and its run: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("having a go at the check"),
        "with the tail of what the last fix session said: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "every fix session the branch was allowed has been spent, so a restart \
         that started the fixing over would spend them all again",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "what is waiting is the human, which is what the badge is for",
    );
    assert!(
        !checks_settled(&fixture).await,
        "and a red suite settles nothing",
    );

    // Two goes at it and no more — this is the count that *two attempts, then
    // ask* is about.
    assert_eq!(fixes(&view), 2, "the machine had two goes at the check");

    // Each of them inside the bundled addressing skill, told which check was red
    // and where to go and read the real failure.
    let told = std::fs::read_to_string(&written).expect("both fix sessions wrote their prompt");
    let prompts: Vec<&str> = told
        .split("=====")
        .filter(|it| !it.trim().is_empty())
        .collect();

    assert_eq!(prompts.len(), 2, "one prompt per fix session: {told}");
    for prompt in &prompts {
        assert!(
            prompt.contains("addressing/SKILL.md"),
            "the session is put inside the bundled skill: {prompt}",
        );
        assert!(
            prompt.contains("model=claude-implementation-5"),
            "under the implementation Profile, as every session that writes code is: \
             {prompt}",
        );
        assert!(
            prompt.contains("Rust") && prompt.contains("/actions/runs/1/job/2"),
            "and told which check is failing, and where: {prompt}",
        );
        assert!(
            prompt.contains("The API has none."),
            "under the Brief the work started from: {prompt}",
        );
    }

    // Long enough for many more polls, had anything still been dispatching.
    pause(Duration::from_millis(500)).await;

    let view = fixture.view().await;

    assert_eq!(
        fixes(&view),
        2,
        "the run does not go round again once it has stopped",
    );
    assert_eq!(
        notices(&view).len(),
        1,
        "and it stops once: {:?}",
        notices(&view),
    );
}

/// A `gh` that can find the pull request and cannot say anything about its
/// checks. Verkstead does not know how they are, so it concludes nothing: it
/// neither settles them nor dispatches a fix session at a failure it has not
/// seen.
#[tokio::test]
async fn checks_gh_cannot_answer_about_leave_the_wrap_up_waiting() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture =
        grilling_spilling(prompts, &a_backlog_then_fixes(&written), CHECKS_UNASKABLE).await;

    worked_to_empty(&fixture).await;
    fixture
        .until(|view| (view.state == Lifecycle::Wrapping).then_some(()))
        .await;

    // Long enough for many polls, every one of them unable to ask.
    pause(Duration::from_millis(500)).await;

    let view = fixture.view().await;

    assert!(
        !checks_settled(&fixture).await,
        "not knowing is not the same as green",
    );
    assert_eq!(fixes(&view), 0, "and it is not the same as red either");
    assert!(!written.exists(), "so no fix session was dispatched");
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped over it: a login to renew is not a run that stopped — {:?}",
        notices(&view),
    );
    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "the Conversation goes on wrapping up, which is what waiting looks like",
    );
}

/// A wrap-up with everything but its checks settled and nothing running in its
/// Worktree reads as **Waiting on checks**, and says so once.
///
/// A condition of Wrapping rather than a state: the Lifecycle is untouched and
/// nothing new is stored on the Conversation, so what the card and the sidebar
/// draw it from is the wrap-up's own settle facts read alongside the register of
/// what is running. The review read the branch and found nothing to raise,
/// nothing has been said on the pull request, the pull request merges, and here
/// the suite never finishes — which is a wrap-up that will wait for as long as
/// this test cares to look.
///
/// The Notice is written once per narrowing and not once per poll: the settling
/// loop asks on a cadence, and a Timeline that grew a line every half second
/// would be one nobody could read.
#[tokio::test]
async fn a_wrap_up_down_to_its_checks_reads_as_waiting_on_them_and_says_so_once() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(STILL_RUNNING, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    let view = fixture
        .until(|view| view.waiting_on_checks.then(|| view.clone()))
        .await;

    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "which is a condition of Wrapping rather than a state of its own",
    );
    assert!(
        !view.working,
        "and one only a wrap-up with nobody in it is ever in",
    );
    assert!(
        !checks_settled(&fixture).await,
        "the checks are the one thing left, and they have not finished",
    );
    assert!(
        review_settled(&fixture).await,
        "the review read the branch and found nothing to raise",
    );

    assert!(
        fixture.row().await.waiting_on_checks,
        "and the sidebar says the same thing about the same Conversation",
    );

    // Long enough for many more polls of a wrap-up that is still down to its
    // checks, every one of them finding the line already written.
    pause(Duration::from_millis(500)).await;

    let view = fixture.view().await;

    assert_eq!(
        waiting_on_checks(&view).len(),
        1,
        "one line per narrowing, not one per poll: {:?}",
        waiting_on_checks(&view),
    );
    assert!(
        notices(&view).is_empty(),
        "and it is a condition rather than a stop, so nothing stopped: {:?}",
        notices(&view),
    );
    assert!(
        view.waiting_on_checks,
        "and the label goes on standing for as long as the condition does",
    );
}

/// The checks are asked about for as long as the Conversation is wrapping up and
/// no longer.
///
/// The other half of *while it is Wrapping*, and the half nothing else here
/// would catch: a poller that went on asking GitHub about a Conversation that
/// had stopped would be a `gh` call every half minute, for ever, about work
/// nobody is doing.
#[tokio::test]
async fn the_checks_stop_being_asked_about_once_the_conversation_leaves_wrapping() {
    let spill = tempfile::tempdir().unwrap();
    let asked = spill.path().join("asked");
    let written = spill.path().join("fix-prompts");

    // A `gh` that keeps a mark per question about the checks, so a test can see
    // whether anything is still asking.
    let counting = format!(
        "case \"$5\" in *statusCheckRollup*) printf 'x' >> {asked} ;; esac\n{}",
        gh_checking("SUCCESS"),
        asked = quoted(&asked),
    );

    let fixture = grilling_spilling(spill, &a_backlog_then_fixes(&written), &counting).await;

    worked_to_empty(&fixture).await;
    fixture
        .until(|view| (view.state == Lifecycle::Wrapping).then_some(()))
        .await;

    let deadline = Instant::now() + *PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the checks were never asked about"
        );
        pause(Duration::from_millis(50)).await;
    }

    assert_eq!(fixture.close().await, ConversationClosed::Closed);

    // And then wait for the Conversation to have gone quiet all the way
    // through, rather than sleeping a poll's worth and hoping.
    //
    // The watcher holds the Conversation on the drivers register for as long as
    // its task runs, and its task ends *after* the question it was part way
    // through has come back — so an empty register is the mark that question
    // left already being on the file. A sleep here was a guess at how long that
    // took, and a guess taken on a loaded machine is a baseline with a question
    // still in flight behind it, which reads afterwards as polling that never
    // stopped.
    fixture
        .until(|view| (!view.working && !view.driven).then_some(()))
        .await;

    let when_stopped = std::fs::metadata(&asked).map(|it| it.len()).unwrap();

    // Long enough for many more polls, had anything still been polling.
    pause(Duration::from_millis(500)).await;

    assert_eq!(
        std::fs::metadata(&asked).map(|it| it.len()).unwrap(),
        when_stopped,
        "GitHub was still being asked about a Conversation that had stopped",
    );
}

/// A pull request goes on being built while Verkstead is down, so a server that
/// comes back up watches again what it was left watching.
///
/// The first server cannot ask about the checks at all, so it settles nothing
/// however long it runs; the second asks the same question of a `gh` that
/// answers. What settles them is therefore the restart having resumed the
/// watching, and nothing else.
#[tokio::test]
async fn a_restarted_server_watches_the_checks_it_was_left_wrapping_up() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture =
        grilling_spilling(prompts, &a_backlog_then_fixes(&written), CHECKS_UNASKABLE).await;

    worked_to_empty(&fixture).await;
    fixture
        .until(|view| (view.state == Lifecycle::Wrapping).then_some(()))
        .await;

    assert!(
        !checks_settled(&fixture).await,
        "the first server never got an answer, which is what makes this prove anything",
    );

    let _restarted = router_running_sessions(
        open_database(&fixture.database).await.unwrap(),
        WatchedPaths::none(),
        fixture.state.path().to_owned(),
        Agents::running(
            vec!["/bin/sh".to_owned(), "-c".to_owned(), "true".to_owned()],
            Homes::on(
                Platform::HERE,
                PathBuf::from("/nonexistent"),
                fixture.state.path(),
            ),
            Reachable::at(LISTENING),
            SandboxConfig::default(),
            BuildCache::none(),
            Skills::installed(fixture.state.path()).expect("this binary carries skills"),
            equipped(fixture.state.path()),
            Handoffs::under(fixture.state.path()),
            Attachments::under(fixture.state.path()),
            Settings::in_data_dir(fixture.state.path()),
        )
        .at_pace(*BRISKLY),
        gh_stub(&gh_checking("SUCCESS")),
    );

    let deadline = Instant::now() + *PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the restarted server never looked at the checks it was left with",
        );
        pause(Duration::from_millis(50)).await;
    }

    // And it was never called a Conversation nothing was driving, which is the
    // other half of taking one up: the sweep waits for the wrap-ups to be
    // resumed before it judges anything, so a wrap-up under live watchers is
    // exactly as healthy as it looks.
    assert!(
        notices(&fixture.view().await).is_empty(),
        "a wrap-up a restarting server took up again is not one standing still",
    );
}

/// A finish that commits its work and opens no pull request is not the end of the
/// run: one session is sent for the one thing missing, and a branch that comes
/// back on a pull request wraps up as though the finish had done it itself.
///
/// The failure this is about is the finish stopping between its commit and its
/// push. The step lands — `.tasks/` is gone and the commit removing it is on the
/// branch — so everything Verkstead watches for says done, and the work is
/// nonetheless built, committed and unreadable by anybody. It is one push and one
/// `gh pr create` from being finished, which is the cheapest thing there is to
/// ask for and the one thing the run cannot go on without.
///
/// Sent inside the submitting skill rather than the fork that works a backlog:
/// the work is built, and a session told to work the next task would find no
/// backlog and nothing to do.
#[tokio::test]
async fn a_finish_that_opened_no_pull_request_is_sent_back_for_one() {
    let spill = tempfile::tempdir().unwrap();
    let opened = spill.path().join("opened-when-asked");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_whose_pull_request_is_opened_when_asked(&opened),
        &gh_opened_by_hand(&opened),
    )
    .await;

    worked_to_empty(&fixture).await;

    // Read at the moment it moves rather than afterwards: a wrap-up with nothing
    // outstanding settles itself, so a second look is as likely to be of a
    // Conversation that has already finished.
    let view = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping && pull_request(view).is_some())
                .then(|| view.clone())
        })
        .await;

    let found = pull_request(&view).expect("the wrap-up has its pull request pinned");

    assert_eq!(
        found.number, 41,
        "the pull request the session opened is the one it wraps up",
    );
    assert_eq!(
        sessions_on(&fixture, "submitting/SKILL.md").await,
        1,
        "one session, sent for the pull request and nothing else",
    );
    assert_eq!(
        view.blocked_on, None,
        "and nothing is waiting on the human: the run finished the way it was \
         always supposed to",
    );
}

/// And what happens when the session sent for it opens none either — no `gh`, no
/// login, or a branch nothing was opened on. The Conversation stays where it is
/// with the reason on its Timeline, rather than becoming a Wrapping with no pull
/// request under it.
///
/// One go and then the stop: two agents that both stopped short of the same push
/// is something for the human to look at, and a third would be Verkstead spending
/// an account on the same missing thing with nobody watching.
#[tokio::test]
async fn a_finish_whose_pull_request_never_arrives_leaves_the_conversation_where_it_is() {
    let fixture = grilling_asking(
        &a_backlog_that_cannot_open_a_pull_request(),
        NO_PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("pull request"),
        "the step is named as what it was: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("no pull request"),
        "and the reason is `gh`'s, in words: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "what is missing is out here rather than in a driver that went away, so a \
         restart looking again would find the same missing thing",
    );
    assert_eq!(
        sessions_on(&fixture, "submitting/SKILL.md").await,
        1,
        "and the stop is on the far side of the one go rather than instead of it",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "the work is where it was, because nothing about it got any further",
    );
    assert!(pull_request(&view).is_none(), "and nothing is pinned");
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "what is waiting is the human, which is what the badge is for",
    );
}

/// A finish that carried a companion it committed in to a pull request of its
/// own has both of them pinned: the Conversation's own unlabelled, and the
/// companion's named with the repository it was opened in.
///
/// Which is the whole of the split, one repository further out: the push and the
/// pull requests are the session's, and what is Verkstead's is knowing that they
/// happened — asked of the host's `gh` in each repository in turn, `#41` and `#7`
/// being numbers in two different places.
#[tokio::test]
async fn a_companion_the_work_committed_in_is_pinned_beside_the_conversations_own() {
    let fixture = grilling_building_in_asking(
        A_BACKLOG_ALONGSIDE,
        "askance",
        &gh_alongside(COMPANION_PULL_REQUEST),
    )
    .await;

    worked_to_empty(&fixture).await;

    let view = fixture
        .until(|view| (pull_requests(view).len() == 2).then(|| view.clone()))
        .await;

    let pinned = pull_requests(&view);
    let named = |repo: Option<&str>| {
        pinned
            .iter()
            .find(|opened| opened.repo.as_deref() == repo)
            .unwrap_or_else(|| panic!("no pull request from {repo:?} among {pinned:#?}"))
    };

    assert_eq!(
        named(None).number,
        41,
        "the work's own repository draws unlabelled, as its commits do",
    );
    assert_eq!(
        named(Some("askance")).number,
        7,
        "and the companion's says which repository it was opened in",
    );
    assert_eq!(
        named(Some("askance")).url,
        "https://github.com/tobico/askance/pull/7",
        "with the URL of that repository's own pull request rather than a number \
         built onto this one's",
    );

    assert_eq!(view.state, Lifecycle::Wrapping);
    assert_eq!(
        view.blocked_on, None,
        "and nothing is waiting on the human: every repository the work committed \
         in is on a pull request",
    );
}

/// A read-write companion nobody committed in is ignored by the whole of
/// wrap-up: nothing asked of GitHub, nothing recorded, nothing waited on.
///
/// The `gh` this is given would stop the run if it were asked about `askance` at
/// all — it answers there the way one does for a branch nothing was opened on —
/// so a Conversation that reaches a settled wrap-up with one pull request pinned
/// and no Notice is one that never asked.
#[tokio::test]
async fn a_companion_nothing_was_committed_in_is_never_asked_about() {
    let fixture = grilling_building_in_asking(
        A_BACKLOG_OF_ONE,
        "askance",
        &gh_alongside(COMPANION_NO_PULL_REQUEST),
    )
    .await;

    worked_to_empty(&fixture).await;

    // Waited on rather than read straight off: what says nothing was asked is
    // the wrap-up getting past the point where it would have asked.
    let deadline = Instant::now() + *PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the wrap-up never got as far as its checks: {}",
            standing(&fixture.view().await),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let view = fixture.view().await;

    assert_eq!(
        pull_requests(&view).len(),
        1,
        "one pull request, for the one repository the work committed in: {:#?}",
        pull_requests(&view),
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing to say about a companion nobody wrote in: {:?}",
        notices(&view),
    );
    assert_eq!(view.blocked_on, None);
}

/// And a read-only companion is not asked at all, whatever its repository holds.
///
/// Its checkout is detached and bound read-only, so nothing a session did can be
/// on it — which is why the branch given to it here is made from outside, the way
/// the sweep's own read-only test makes one. A wrap-up that asked git about it
/// would find a branch a commit ahead of where that checkout was cut, ask GitHub
/// about it, and stop over the answer.
#[tokio::test]
async fn a_read_only_companion_is_not_asked_about_a_pull_request() {
    let fixture = grilling_alongside_asking(
        A_BACKLOG_OF_ONE,
        "askance",
        &gh_alongside(COMPANION_NO_PULL_REQUEST),
    )
    .await;

    // Put there before anything reaches a wrap-up, so it is in front of the
    // asking rather than behind it.
    let view = fixture.view().await;
    let companion = Path::new(&view.companions[0].repo.path).to_owned();

    git(&companion, &["checkout", "--quiet", "-b", &view.branch]);
    std::fs::write(companion.join("halves.md"), "the other half\n").unwrap();
    git(&companion, &["add", "halves.md"]);
    git(
        &companion,
        &["commit", "--quiet", "-m", "feat: the other half"],
    );

    worked_to_empty(&fixture).await;

    let deadline = Instant::now() + *PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the wrap-up never got as far as its checks: {}",
            standing(&fixture.view().await),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let view = fixture.view().await;

    assert_eq!(
        pull_requests(&view).len(),
        1,
        "one pull request, for the one repository a session could commit in: {:#?}",
        pull_requests(&view),
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing to say about a repository nothing was ever asked about: {:?}",
        notices(&view),
    );
}

/// And a companion the work *did* commit in and left without a pull request stops
/// the run, with a Notice naming the repository.
///
/// A deliberate stop, the shape a missing pull request already had: the work ran
/// and left none, so what is wrong is out here rather than in a driver that went
/// away. What was already found stays found — the Conversation's own pull request
/// is pinned and clickable while the human sorts out the one that is missing.
#[tokio::test]
async fn a_committed_in_companion_without_a_pull_request_stops_the_run_naming_it() {
    let fixture = grilling_building_in_asking(
        A_BACKLOG_ALONGSIDE,
        "askance",
        &gh_alongside(COMPANION_NO_PULL_REQUEST),
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("askance"),
        "the Notice names the repository that was left without one: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("no pull request"),
        "and the reason is `gh`'s, in words: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "looking again would find the same missing thing",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "the work is on a pull request of its own, which is what wrapping up is",
    );
    assert_eq!(
        pull_requests(&view).len(),
        1,
        "and the one that was found stays found: {:#?}",
        pull_requests(&view),
    );
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "what is waiting is the human, which is what the badge is for",
    );

    // And it must not reach Done past its own Notice. A wrap-up that stopped on
    // a companion has nothing unsettled to hold it — nothing was recorded about
    // the missing pull request to be unsettled about — so the recorded one going
    // green, the review answered and nothing outstanding is exactly the state
    // the rule that ends a wrap-up would finish on. Written here rather than
    // waited for, the watchers having stopped along with everything else.
    settle_everything(&fixture).await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "a stopped wrap-up is not finished, however little is left outstanding",
    );
}

/// Everything a wrap-up waits on, recorded as settled without any of it having
/// happened.
///
/// What a green suite, an answered review and a quiet pull request would leave
/// behind, put there directly because the watchers that would have written it
/// stop when the run does — see [`verkstead_server::store::settle_wrap_up`].
async fn settle_everything(fixture: &Grilling) {
    let pool = open_database(&fixture.database).await.unwrap();

    let opened = verkstead_server::store::pull_requests(&pool, fixture.id)
        .await
        .unwrap();

    // The one there is one of per Conversation, and a green suite and a quiet
    // conversation for every pull request the wrap-up found — which is what the
    // rule that ends one asks for.
    let waiting_on =
        verkstead_server::store::WAITED_ON
            .into_iter()
            .chain(opened.into_iter().flat_map(|(repo, _)| {
                [
                    verkstead_server::store::WaitingOn::Checks(repo.id),
                    verkstead_server::store::WaitingOn::Comments(repo.id),
                ]
            }));

    for waiting_on in waiting_on {
        verkstead_server::store::settle_wrap_up(&pool, fixture.id, waiting_on)
            .await
            .unwrap();
    }

    pool.close().await;
}

/// The findings of a review, as the bundled reviewing skill writes them: a
/// Question per finding, each offering a way to fix it beside leaving it alone.
///
/// Put through the agent API by the test rather than by the stub, exactly as the
/// grilling's proposal is: a router driven by `oneshot` has no socket for a
/// session to reach, and what is under test is what Verkstead does once the Set
/// is there.
const REVIEW: &str = r#"
title: Review of the rate limiter branch
preface: |
  Two things worth a decision.
questions:
  - label: Q1
    text: The window counter is never reset between windows.
    options:
      - n: 1
        text: Fix it
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: Two clocks now, and the tests pin both.
    options:
      - n: 1
        text: Fix it
      - n: 2
        text: Leave it
        recommended: true
"#;

/// The shortest whole backlog, plus the sessions a wrap-up runs.
///
/// A batch of comments is answered by a session that finds nothing in them,
/// which is the quietest of the three things one can do — see
/// [`a_backlog_then_answers_comments`] for the tests that are about the other
/// two.
fn a_backlog_then_wraps_up(reviews: &Path, dispatched: &Path, review: &str) -> String {
    wrapping_up(
        reviews,
        dispatched,
        dispatched,
        review,
        RESPOND_AND_FIND_NOTHING,
    )
}

/// The shortest whole backlog, plus a wrap-up whose fix session resolves the
/// conflict it was sent at — once the test lets it.
///
/// It writes the prompt it was given down and then talks to itself until
/// `released` is there, which is what holds a conflicted wrap-up still for as
/// long as a test wants to look at one: the session is in the Worktree, its go
/// is spent, and nothing has moved. Talking rather than sleeping, because a
/// session that fell silent with nothing committed would be ended out from under
/// the test.
///
/// Then it commits, and puts `resolved` there — which is the push, as far as the
/// `gh` beside it is concerned.
fn a_backlog_then_resolves(dispatched: &Path, released: &Path, resolved: &Path) -> String {
    format!(
        r#"
case "$2" in
*reviewing/SKILL.md*)
    printf 'I read the whole branch and found nothing worth raising\n'
    exit 0
    ;;
*addressing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {dispatched}
    while [ ! -e {released} ]; do printf 'merging the base branch in\n'; sleep 0.1; done
    printf 'a merge\n' >> merged.md
    git add -A
    git commit --quiet -m 'fix: merge the base branch in and resolve the conflicts'
    printf 'x' > {resolved}
    sleep 300
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#,
        dispatched = quoted(dispatched),
        released = quoted(released),
        resolved = quoted(resolved),
    )
}

/// The shortest whole backlog, a review that finds nothing, and a fix session
/// that resolves the conflict it is sent at as soon as it arrives.
///
/// Both spills matter. `dispatched` is what says a resolution was sent at all,
/// and `reviews` is what says how many times the branch has been read — which is
/// the whole of what the resolve press is not allowed to do twice.
///
/// [`a_backlog_then_resolves`] is the same session held still by a release file,
/// for the tests that want to look at a conflicted wrap-up standing there. This
/// one has nothing to hold still for: what it is in aid of is the round after
/// Done finishing.
fn a_backlog_then_resolves_at_once(reviews: &Path, dispatched: &Path, resolved: &Path) -> String {
    format!(
        r#"
case "$2" in
*reviewing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {reviews}
    printf 'I read the whole branch and found nothing worth raising\n'
    exit 0
    ;;
*addressing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {dispatched}
    printf 'merging the base branch in\n'
    printf 'a merge\n' >> merged.md
    git add -A
    git commit --quiet -m 'fix: merge the base branch in and resolve the conflicts'
    printf 'x' > {resolved}
    sleep 300
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#,
        reviews = quoted(reviews),
        dispatched = quoted(dispatched),
        resolved = quoted(resolved),
    )
}

/// The same, with the batch sessions' prompts spilled somewhere of their own and
/// doing whatever `responding` says.
fn a_backlog_then_answers_comments(
    reviews: &Path,
    dispatched: &Path,
    batches: &Path,
    responding: &str,
) -> String {
    wrapping_up(
        reviews,
        dispatched,
        batches,
        REVIEW_AND_FIND_NOTHING,
        responding,
    )
}

/// What all of them are made of.
///
/// The review writes down the prompt it was given and then does whatever
/// `review` says; a batch session does the same with `responding`; a fix session
/// writes its prompt down and commits, which is what one reports through. Told
/// apart by the skill their prompts name, because that is the fact under it —
/// all of them run under the same implementation Profile and differ only in what
/// they were sent to do.
fn wrapping_up(
    reviews: &Path,
    dispatched: &Path,
    batches: &Path,
    review: &str,
    responding: &str,
) -> String {
    // Written as a word in the stubs above and spelled out here, because the loop
    // is the same loop in every one of them — see [`WHILE_NOBODY_HAS_ASKED`].
    let review = review.replace("WHILE_NOBODY_HAS_ASKED", WHILE_NOBODY_HAS_ASKED);
    let responding = responding.replace("WHILE_NOBODY_HAS_ASKED", WHILE_NOBODY_HAS_ASKED);

    format!(
        r#"
case "$2" in
*reviewing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {reviews}
{review}
    ;;
*responding/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {batches}
{responding}
    ;;
*addressing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {dispatched}
    printf 'having a go at it\n'
    printf 'a fix\n' >> fixes.md
    git add -A
    git commit --quiet -m 'fix: address what the wrap-up raised'
    sleep 300
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#,
        reviews = quoted(reviews),
        batches = quoted(batches),
        dispatched = quoted(dispatched),
    )
}

/// A session reading, up until the Set the test puts on its behalf is up.
///
/// The two halves of a propose-then-fix session in one line, because the fixture
/// splits them across two processes. A real one reads the branch out loud, asks
/// within moments of starting and only then goes silent — so the quiet it is
/// ended on begins with its ask already open, and the ask is the only thing
/// keeping it alive. Here the reading is a stub and the asking is the test, which
/// posts the Set whenever it gets to it: without this the stub would fall silent
/// with nothing open, and be ended before the test had asked anything.
///
/// The same line over and over, so that the last thing said is the same whether
/// it was said once or fifty times — which is what the Timeline shows of a
/// session. [`Grilling::ask`] is what writes the marker.
const WHILE_NOBODY_HAS_ASKED: &str =
    "while [ ! -f /tmp/verkstead/asked ]; do printf '%s\\n' \"$SAYING\"; sleep 0.1; done";

/// A review session that reads the branch and then waits on the human, which is
/// what one blocked on `verkstead ask` looks like from outside.
const REVIEW_THEN_WAIT: &str = "    SAYING='reading the branch'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     sleep 300";

/// One that is still at work when the answers have come back, and stays that way.
///
/// What a restart has to interrupt to be a restart at all: a session with nothing
/// left to do is ended on quiet rather than left hanging, so a stub that fell
/// silent on being answered would be seen out by the server that started it and
/// there would be nothing for the restart to find.
const REVIEW_THEN_WORK_ON: &str = "    SAYING='reading the branch'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while :; do printf 'acting on the answers\\n'; sleep 0.1; done";

/// One that goes on to fix what the human accepted, which is the rest of what a
/// review session is for.
///
/// A stub cannot idle on a blocking ask and then wake up, so the marker file
/// stands in for the Response arriving: the test writes it once it has answered.
const REVIEW_THEN_FIX: &str = "    SAYING='reading the branch'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'a fix\\n' >> fixes.md\n    \
     git add -A\n    \
     git commit --quiet -m 'fix: reset the counter as the window rolls'\n    \
     printf 'fixed what was accepted and left the rest\\n'";

/// One that fixes what was accepted in both of the repositories the work
/// reached: a commit in the worktree it started in and one in the companion's
/// beside it.
///
/// What a review of work on two pull requests lands. The findings were one Set
/// across the lot of them, and where they landed is wherever the finding was
/// about — so the session commits in each worktree it fixed something in and
/// pushes each of them.
const REVIEW_THEN_FIX_BOTH: &str = "    SAYING='reading the branch'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'a fix\\n' >> fixes.md\n    \
     git add -A\n    \
     git commit --quiet -m 'fix: reset the counter as the window rolls'\n    \
     cd ../askance-*\n    \
     printf 'a fix\\n' >> halves.md\n    \
     git add -A\n    \
     git commit --quiet -m 'fix: take the other half with it'\n    \
     printf 'fixed what was accepted in both, and pushed both\\n'";

/// One that waits for the answers and then goes without landing any of them,
/// which is the failure a session dying between the deciding and the doing
/// leaves behind.
const REVIEW_THEN_VANISH: &str = "    SAYING='reading the branch'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'that is what I would have fixed\\n'";

/// One that waits for the answers and then falls over, which is a session that
/// took what it was going to do about them with it.
const REVIEW_THEN_DIE_ON_THE_ANSWERS: &str = "    SAYING='reading the branch'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'gh: the connection dropped\\n'\n    \
     exit 1";

/// One that finds nothing, says so as the last thing it prints, and stops.
const REVIEW_AND_FIND_NOTHING: &str =
    "    printf 'I read the whole branch and found nothing worth raising\n'";

/// The same, except that it never stops: it says what it found and then idles,
/// which is what an interactive agent does when its work is done and so what
/// every real session does.
///
/// The one above exits when it has finished, which is convenient to write and is
/// a shape no agent has: a stub that sees itself out proves nothing about a
/// session that simply sits there, which is every session there is.
const REVIEW_AND_FIND_NOTHING_THEN_IDLE: &str =
    "    printf 'I read the whole branch and found nothing worth raising\n'\n    sleep 300";

/// One that stores its ask, ends its turn, and reads what is typed into it when
/// the Answers land — which is what a session on a store-and-nudge backend does
/// with the rest of its life.
///
/// **Two reads, with the terminal out of canonical mode for them.** What says
/// the line and its Enter arrived as two keystrokes is a session that took two
/// reads to get them: a burst would come back from the first read whole, which
/// is the paste an agent's interface would have read it as. So `stty` is set
/// before the wait rather than after it — the line may land while the stub is
/// still getting there — and each read writes down what it got.
///
/// Then it carries on, as a session told its Answers are there does: it says
/// what it did and idles, which is what ends the review on quiet.
const REVIEW_THEN_READ_THE_NUDGE: &str = "    SAYING='reading the branch'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     stty -icanon min 1 time 0\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     LINE=$(dd bs=4096 count=1 2>/dev/null)\n    \
     printf '%s\\n' \"$LINE\" >> /tmp/verkstead/nudges\n    \
     ENTER=$(dd bs=4096 count=1 2>/dev/null | od -An -c)\n    \
     printf '%s\\n' \"$ENTER\" >> /tmp/verkstead/nudges\n    \
     printf 'fetched the answers and left the rest\\n'\n    \
     sleep 300";

/// One that reads the branch, waits on the human, does what they accepted — and
/// then idles rather than exiting, as a real one does.
const REVIEW_THEN_FIX_AND_IDLE: &str = "    SAYING='reading the branch'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'fixed what was accepted and left the rest\\n'\n    \
     sleep 300";

/// One that comes up and never says a word — an agent that fell over before its
/// first line, or one that never got as far as reading anything.
///
/// Silence is the whole of what quiet-with-nothing-pending has to read, so this
/// is the shape that would satisfy it having done nothing at all.
const REVIEW_THAT_SAYS_NOTHING: &str = "    sleep 300";

/// And one that never goes quiet at all, which is a session still at work.
const REVIEW_THAT_KEEPS_TALKING: &str = "    printf 'reading the branch\\n'\n    \
     while :; do sleep 0.1; printf 'still reading\\n'; done";

/// A batch session that finds nothing in what was said needing a change, says so
/// as the last thing it prints, and stops.
const RESPOND_AND_FIND_NOTHING: &str =
    "    printf 'I read what was said and none of it needs a change\n'";

/// One that proposes, waits for the answers and then does what was accepted,
/// which is the whole of what a batch session is for.
///
/// The marker file stands in for the Response arriving, as the review's does: a
/// stub cannot idle on a blocking ask and then wake up.
const RESPOND_THEN_FIX: &str = "    SAYING='reading what was said'\n    \
     printf '%s\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'a fix\n' >> fixes.md\n    \
     git add -A\n    \
     git commit --quiet -m 'fix: move the reset above the comparison'\n    \
     printf 'did what was accepted and left the rest\n'";

/// And one that waits for the answers and then goes without landing any of them.
const RESPOND_THEN_VANISH: &str = "    SAYING='reading what was said'\n    \
     printf '%s\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'that is what I would have done\n'";

/// And one that waits for the answers and then falls over, which is a batch
/// session that took what it was going to do about them with it.
const RESPOND_THEN_DIE_ON_THE_ANSWERS: &str = "    SAYING='reading what was said'\n    \
     printf '%s\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'gh: the connection dropped\n'\n    \
     exit 1";

/// One that reads what was said and then waits on the human, which is what a
/// batch session blocked on `verkstead ask` looks like from outside.
const RESPOND_THEN_WAIT: &str = "    SAYING='reading what was said'\n    \
     printf '%s\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     sleep 300";

/// What a batch session puts to the human, as the bundled responding skill
/// writes it: a Question per comment it would do something about, each offering
/// to do it beside leaving it alone.
const ANSWERING_THE_COMMENTS: &str = r#"
title: What was said on the rate limiter's pull request
preface: |
  Two things you asked for, as I read them.
questions:
  - label: Q1
    text: You said the reset is the wrong way round. It is.
    options:
      - n: 1
        text: Do it
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: You asked about the test that pins the old name.
    options:
      - n: 1
        text: Do it
      - n: 2
        text: Leave it
        recommended: true
"#;

/// Whether Verkstead has recorded this Conversation's review as done with.
async fn review_settled(fixture: &Grilling) -> bool {
    let pool = open_database(&fixture.database).await.unwrap();
    let settled = verkstead_server::store::wrap_up_settled(&pool, fixture.id)
        .await
        .unwrap();
    pool.close().await;

    settled.contains(&verkstead_server::store::WaitingOn::Review)
}

/// An open page, listening on the Nudge stream.
///
/// The stream's own tests are in `nudges.rs`, which is where what a Nudge says
/// and when belongs. This is the one thing they cannot ask: a Nudge that is
/// sent because a *session* did nothing, which needs a session to be running.
struct Listening {
    body: Body,

    /// What has been read off the stream and is not a whole frame yet. SSE
    /// frames are not the chunks they arrive in.
    buffered: String,
}

impl Listening {
    /// Open the stream the way a page does. Returns once the response is in
    /// hand, which is after the handler has subscribed — so anything that
    /// happens next is something this page is listening for.
    async fn open(app: &Router) -> Self {
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

        Self {
            body: response.into_body(),
            buffered: String::new(),
        }
    }

    /// The next Nudge, past the keep-alives that are the stream's other
    /// traffic, as the page reads it: the JSON of the `data` line.
    async fn nudge(&mut self) -> verkstead_schema::Nudge {
        let waited_for = tokio::time::timeout(*PATIENCE, async {
            loop {
                let frame = self.frame().await;

                if let Some(data) = frame
                    .starts_with("event: nudge")
                    .then(|| frame.lines().find_map(|line| line.strip_prefix("data: ")))
                    .flatten()
                {
                    return serde_json::from_str(data).unwrap_or_else(|error| {
                        panic!("a Nudge should be readable as one: {data:?} — {error}")
                    });
                }
            }
        });

        waited_for.await.expect("waited for a Nudge in vain")
    }

    /// The next whole frame off the stream, whatever kind it is.
    async fn frame(&mut self) -> String {
        loop {
            if let Some(end) = self.buffered.find("\n\n") {
                return self.buffered.drain(..end + 2).collect();
            }

            let chunk = self
                .body
                .frame()
                .await
                .expect("the stream ended")
                .unwrap()
                .into_data()
                .expect("the stream carries data frames");

            self.buffered.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    }
}

/// Wait until a session has written its prompt down, and hand back what is
/// there.
async fn until_written(path: &Path) -> String {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        if let Ok(written) = std::fs::read_to_string(path) {
            if !written.trim().is_empty() {
                return written;
            }
        }

        assert!(
            Instant::now() < deadline,
            "no session ever wrote to {}",
            path.display(),
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// The same, and then until the session that wrote it is on the Timeline —
/// which is what a test about to ask on its behalf has to wait for.
///
/// A stub writes its prompt down as its first act, and the Event a session
/// prints into is opened a moment *after* the process was spawned. So the file
/// can be there while the Timeline still knows nothing of the session, and a
/// Set posted inside that window lands on an Event of its own ahead of the
/// session's. Which is a Set that belongs to nobody: every Set that landed
/// after a session's Event is that session's, and one that landed before it is
/// some earlier session's — so the session is read as idling on nothing and
/// ended by the very rule that exists to keep one waiting on a human alive.
/// See the runner's `asking`.
///
/// A real session cannot get in front of its own Event: it has a sandbox to
/// come up in and a branch to read before it asks anything, and it is talking
/// the whole time. This is the other half of what [`WHILE_NOBODY_HAS_ASKED`]
/// does — the two of them together putting the test's ask back where a real
/// one happens, after the session it is coming from.
async fn until_asking(fixture: &Grilling, path: &Path) -> String {
    let written = until_written(path).await;

    fixture
        .until(|view| {
            outputs(view)
                .last()
                .filter(|output| output.running)
                .map(|_| ())
        })
        .await;

    written
}

/// The same, waiting until what is there says something in particular — for the
/// tests where an earlier session has already written to the file, so that
/// merely finding it there says nothing about the one being waited on.
async fn until_written_saying(path: &Path, said: &str) -> String {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        if let Ok(written) = std::fs::read_to_string(path) {
            if written.contains(said) {
                return written;
            }
        }

        assert!(
            Instant::now() < deadline,
            "no session ever wrote {said:?} to {}",
            path.display(),
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// The same again, waiting until `sessions` of them have written — for the tests
/// where several sessions write to the one file and finding some of them there
/// says nothing about the rest.
///
/// What a test whose sessions are dispatched by more than one watcher needs. Two
/// watchers reach their last session at their own pace, so anything that has
/// already happened on the Timeline — the first Notice included — is a moment
/// one of them may still be behind, and a read taken there counts the prompts of
/// whichever got there first.
async fn until_written_by(path: &Path, sessions: usize) -> String {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        let written = std::fs::read_to_string(path).unwrap_or_default();

        if prompts(&written).len() >= sessions {
            return written;
        }

        assert!(
            Instant::now() < deadline,
            "only {} of {sessions} sessions ever wrote to {}: {written}",
            prompts(&written).len(),
            path.display(),
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// How many sessions wrote to one of those files.
fn prompts(written: &str) -> Vec<&str> {
    written
        .split("=====")
        .filter(|prompt| !prompt.trim().is_empty())
        .collect()
}

/// The whole of the wrap-up self-review: one fresh session reads the branch, its
/// findings arrive as a Question Set, and the same session fixes the ones the
/// human accepted.
///
/// The session that reviews is the first thing to see this branch — the ones that
/// wrote it each saw one task — so it runs in a fresh context, under the
/// implementation Profile, inside the bundled reviewing skill. It changes nothing
/// before the human has answered, and everything they accepted afterwards: a
/// handful of fixes is not a handful of pieces of work, and nothing is dispatched
/// to do any of it.
#[tokio::test]
async fn the_review_proposes_its_findings_and_then_fixes_what_was_accepted() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;

    let told = until_asking(&fixture, &reviews).await;
    let started = prompts(&told);

    assert_eq!(started.len(), 1, "one review and one only: {told}");
    assert!(
        started[0].contains("reviewing/SKILL.md"),
        "inside the bundled reviewing skill: {told}",
    );
    assert!(
        started[0].contains("model=claude-review-5"),
        "under the review Profile rather than the one that built it, reviewing \
         being a fresh set of eyes: {told}",
    );
    assert!(
        started[0].contains("The API has none."),
        "and told what the work was meant to be: {told}",
    );
    assert!(
        !started[0].contains("The pull requests this work is on"),
        "a Conversation whose work touched nothing else is told what it has always \
         been told, the branch this worktree is on being the whole of it: {told}",
    );

    assert!(
        !review_settled(&fixture).await,
        "a review that has not reported settles nothing",
    );

    // What the review session does through the CLI, played by the test.
    let set = fixture.ask(REVIEW).await;

    // Long enough for the ask to have ended a session, had anything been going to.
    pause(Duration::from_millis(500)).await;

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "the session that asked is the one that fixes, so nothing ends it on the ask: \
         {:?}",
        outputs(&view).last(),
    );
    assert_eq!(fixes(&view), 0, "and it changes nothing until they answer");
    assert!(
        !review_settled(&fixture).await,
        "the review being over is not the same as its findings being put",
    );

    // The human answers from the workbench: fix the first, leave the second.
    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "Keep the signature." },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    // Which is what the review session was waiting on — the marker standing in
    // for the Response its ask returns.
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture.until(|view| (fixes(view) == 1).then_some(())).await;

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the session landed the fix and the review never settled",
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert_eq!(
        fixes(&view),
        1,
        "the accepted finding was fixed by the session that raised it, and the \
         declined one by nobody",
    );
    assert!(
        !dispatched.exists(),
        "with nothing dispatched to fix anything: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        1,
        "and nothing reviews the branch a second time",
    );
    assert!(
        notices(&view).is_empty(),
        "nothing stopped: {:?}",
        notices(&view),
    );
}

/// A review that finds nothing asks nothing.
///
/// A Set with no findings in it would be a row for the human to dismiss, and the
/// point of the phase is to spend their attention only where there is a decision.
/// So it says what it found where they are already looking — the last line a
/// session prints is what its Timeline row shows — and wrap-up carries on.
#[tokio::test]
async fn a_review_that_finds_nothing_raises_no_question_set_and_says_so() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(Instant::now() < deadline, "the review never settled");
        pause(Duration::from_millis(50)).await;
    }

    let view = fixture.view().await;

    assert_eq!(
        sets(&view).len(),
        1,
        "the only Set on the Timeline is the proposal that ended the grilling",
    );
    assert!(
        view.blocked_on.is_none(),
        "nothing is waiting on the human, which is the whole of finding nothing",
    );
    assert!(
        outputs(&view)
            .last()
            .is_some_and(|output| output.latest.contains("nothing worth raising")),
        "and the Timeline says what the review found: {:?}",
        outputs(&view).last(),
    );
    assert!(
        !dispatched.exists(),
        "nothing was dispatched: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
}

/// A review session that finishes and then sits there is ended, and the wrap-up
/// carries on to Done with nobody touching a Screen.
///
/// The bug this closes: every session is an interactive agent, which idles when
/// its work is done rather than exiting, so a review Verkstead waited to see exit
/// waited forever. The wrap-up never settled, a roadmap's next stage never
/// started, and the only way on was quitting the session by hand.
///
/// Green all the way through, so the review is the one thing between this wrap-up
/// and Done — which makes reaching Done the whole proof.
#[tokio::test]
async fn a_review_that_finishes_without_exiting_is_ended_and_the_wrap_up_carries_on() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING_THEN_IDLE),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert!(
        review_settled(&fixture).await,
        "the review settled, which is what reaching Done was waiting on",
    );
    assert!(
        outputs(&view).last().is_some_and(|output| !output.running),
        "and the session that never exited is over: {:?}",
        outputs(&view).last(),
    );
    assert!(
        outputs(&view)
            .last()
            .is_some_and(|output| output.latest.contains("nothing worth raising")),
        "with what it said still the last thing on its row: {:?}",
        outputs(&view).last(),
    );
    assert!(
        notices(&view).is_empty(),
        "ending it is a session finishing rather than one that stopped: {:?}",
        notices(&view),
    );
}

/// A review sitting on a Blocking Ask is left alone however long the human takes,
/// and is ended once they have answered and it has finished.
///
/// The other half of the rule, and the one that makes the first half safe: a
/// session idling on an ask prints nothing for hours, and quiet on its own would
/// reap it mid-question and throw the answers away. So it is quiet *and* nothing
/// of its own left to answer, or it is left where it is.
#[tokio::test]
async fn a_review_waiting_on_its_ask_is_left_alone_until_the_answers_are_in() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX_AND_IDLE),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW).await;

    // Several graces of a session saying nothing at all, which is what waiting on
    // a human looks like from outside.
    tokio::time::sleep(BRISKLY.proposing * 4).await;

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "the session is still there to read what they say: {:?}",
        outputs(&view).last(),
    );
    assert!(
        !review_settled(&fixture).await,
        "and nothing settled a review whose questions are still open",
    );

    // Declined outright, so that what is proven is the ending rather than the
    // fixing: nothing is owed either way.
    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 2 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the answers were in and the session that read them was never ended",
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| !output.running),
        "the session that sat on the ask is over: {:?}",
        outputs(&view).last(),
    );
    assert!(
        notices(&view).is_empty(),
        "nothing stopped: {:?}",
        notices(&view),
    );
}

/// A Deferred Ask holds nothing open: nobody is idling on it, so the session that
/// sent one is ended on quiet like any other.
///
/// Waiting on one would be waiting for the human to answer something nothing was
/// waiting for — its Answers reach a later session by design — and the session
/// would sit there until they got round to it.
///
/// And the wrap-up reads it the same way the ending did: a Deferred Ask left
/// standing is nobody's proposal, so the review settles over the top of one and
/// the question stays open for the human to answer in their own time. Reading it
/// as a proposal with nobody behind it would stop the run over a question that
/// was working, and close it on the human's behalf as it went.
#[tokio::test]
async fn a_deferred_ask_of_a_reviews_own_does_not_hold_its_session_open() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX_AND_IDLE),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask_deferred(REVIEW).await;

    // Which is the whole assertion: it returns only once the session that sent
    // the Set is over, and a Deferred Ask that held it open would hold this open
    // until the deadline instead.
    fixture
        .until(|view| {
            outputs(view)
                .last()
                .and_then(|output| (!output.running).then_some(()))
        })
        .await;

    let view = fixture.view().await;

    assert!(
        !responded(&view, set),
        "and nobody ever answered it: {:?}",
        sets(&view),
    );

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the session was seen out and the review never settled: {:?}",
            notices(&fixture.view().await),
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped over a question nobody was waiting on: {:?}",
        notices(&view),
    );
    assert!(
        matches!(
            where_it_stands(&view, set),
            Some(verkstead_render::Standing::Waiting(_))
        ),
        "and it is still there to be answered in their own time: {:?}",
        where_it_stands(&view, set),
    );
}

/// The same review on a backend whose sessions cannot hold a shell command open
/// for hours: the ask comes back stored the moment it lands, and the session it
/// came from is left standing all the same.
///
/// Which is the whole of the third state. The Set is stored as a Deferred Ask
/// is — nothing is waiting on the wire, so the CLI returns and the session ends
/// its turn — and a session *is* idling on it, waiting for the line Verkstead
/// types when the Response lands. Read as a Deferred Ask it would be ended on
/// quiet and prodded by the rescue before the human had answered, leaving the
/// Response with nothing to nudge; read as a blocking one the CLI would sit
/// there for hours. So it is counted as open by the enders and by the rescue,
/// and stored by the reply.
///
/// Nothing is nudged here — that is the next step's — so the stub waits on the
/// marker the test writes, exactly as the blocking one does.
#[tokio::test]
async fn an_ask_on_a_store_and_nudge_backend_is_stored_and_holds_its_session_open() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling_on_codex(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX_AND_IDLE),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    // The ask itself says nothing about the channel — it is the same `verkstead
    // ask` it would be anywhere — and what comes back says the Set was stored.
    let set = fixture.ask_stored(REVIEW).await;

    // Several graces of a session saying nothing at all, which on this backend
    // is what a session with its turn ended looks like from outside.
    tokio::time::sleep(BRISKLY.proposing * 4).await;

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "the session is still there to be nudged: {:?}",
        outputs(&view).last(),
    );
    assert!(
        !review_settled(&fixture).await,
        "and nothing settled a review whose questions are still open",
    );
    assert!(
        notices(&view).is_empty(),
        "nor was it prodded and stopped over a question the human has not \
         answered: {:?}",
        notices(&view),
    );
    assert!(
        matches!(
            where_it_stands(&view, set),
            Some(verkstead_render::Standing::Waiting(
                verkstead_schema::Liveness::Deferred
            ))
        ),
        "and the human sees a deferred-shaped ask, because nothing is holding a \
         connection open on it: {:?}",
        where_it_stands(&view, set),
    );

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 2 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the answers were in and the session that read them was never ended",
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| !output.running),
        "the session that sat on the ask is over: {:?}",
        outputs(&view).last(),
    );
    assert!(
        notices(&view).is_empty(),
        "nothing stopped: {:?}",
        notices(&view),
    );
}

/// And `--deferred` on that same backend still means an ask nobody is idling on:
/// the session that sent one is ended on quiet like any other.
///
/// The one thing the backend does not decide. `--deferred` is the agent saying
/// it will carry straight on, and a backend that stores every ask does not make
/// that untrue — so the review settles over the top of one and the question
/// stays open for the human to answer in their own time, exactly as on Claude.
#[tokio::test]
async fn a_deferred_ask_on_a_store_and_nudge_backend_still_holds_nothing_open() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling_on_codex(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX_AND_IDLE),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask_deferred(REVIEW).await;

    // Which is the whole assertion: it returns only once the session that sent
    // the Set is over, and an ask read as one somebody was idling on would hold
    // this open until the deadline instead.
    fixture
        .until(|view| {
            outputs(view)
                .last()
                .and_then(|output| (!output.running).then_some(()))
        })
        .await;

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the session was seen out and the review never settled: {:?}",
            notices(&fixture.view().await),
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped over a question nobody was waiting on: {:?}",
        notices(&view),
    );
    assert!(
        matches!(
            where_it_stands(&view, set),
            Some(verkstead_render::Standing::Waiting(_))
        ),
        "and it is still there to be answered in their own time: {:?}",
        where_it_stands(&view, set),
    );
}

/// The far end of a store-and-nudge ask: the Response lands, and the session
/// that stored the Set is told so in its own terminal.
///
/// Which is the one thing that end of the ask cannot do for itself. The session
/// asked, `verkstead ask` came back with the id, and the turn ended there — so
/// there is nothing on the wire to hand a Response to and nothing on that end
/// listening for one. What there is is a terminal, and one line goes into it
/// naming the Set and the command that fetches it, down the channel the rescue
/// already types through.
///
/// **Two keystrokes**, which is what the stub's two reads are for: an agent's
/// interface reads a line and its carriage return arriving together as a paste,
/// and a paste's return is a line break rather than a send. And what the session
/// does with the line is take another turn, which is the whole point of typing
/// one — no marker is written here, unlike every other test of a session being
/// answered, because the line *is* the Response reaching it.
#[tokio::test]
async fn a_response_to_a_store_and_nudge_ask_is_typed_into_the_session_that_stored_it() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling_on_codex(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_READ_THE_NUDGE),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask_stored(REVIEW).await;

    // Long enough for the session to have ended its turn and be sitting at the
    // read, which is what the two keystrokes are told apart by: a line typed at
    // one that had not got there yet would be waiting in the terminal with its
    // Enter behind it, and the first read would take both.
    pause(BRISKLY.proposing * 2).await;

    assert!(
        outputs(&fixture.view().await)
            .last()
            .is_some_and(|output| output.running),
        "the session is still there to be nudged: {}",
        standing(&fixture.view().await),
    );

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 2 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    let read = typed_into(&fixture, "nudges", 2).await;

    assert!(
        read[0].contains(&format!("Question Set {set}")),
        "the line names the Set, which is what an agent that stored more than \
         one has no other way of knowing: {read:?}",
    );
    assert!(
        read[0].contains(&format!("verkstead answers {set}")),
        "and the command that fetches it, so that reading the line is enough \
         without going back to the Guide: {read:?}",
    );
    assert!(
        !read[0].contains('\r') && !read[0].contains('\n'),
        "the first read got the line and nothing that would submit it: {read:?}",
    );
    assert!(
        read[1].trim() == r"\n",
        "and the second got the Enter on its own, a moment behind — one read for \
         each keystroke, where a burst would have arrived as a paste. A newline \
         rather than the carriage return that was typed, because that is what a \
         terminal hands a program reading one: {read:?}",
    );

    // And the session takes another turn on it, which is the whole of what the
    // line is for: it fetches, says what it did and goes quiet, and going quiet
    // with nothing open is what ends a review.
    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the session was told and never carried on: {}",
            standing(&fixture.view().await),
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "and nothing stopped along the way: {:?}",
        notices(&view),
    );
    let said = nudges_about(&fixture, set).await;

    assert_eq!(
        said.len(),
        1,
        "the line is in the session's own Capture and nowhere else — a terminal \
         says what is typed into it, and that is the whole of the account \
         Verkstead gives of having spoken: {said:?}",
    );
}

/// A backlog of one whose step session falls over with its ask still standing,
/// and works the task once the human has pressed Resume — with every session
/// writing down the prompt it was started on.
///
/// What a stored ask whose session dies wants around it: a Set stored by a
/// session that is gone before the Response lands, and a session after it whose
/// prompt can be read. A step's ask is an ordinary one — unlike a review's,
/// which is closed unanswered when the session that was to act on it goes, there
/// being nothing left that could act on it.
///
/// Held up by markers of the test's own: `dropped` is when the session falls
/// over, which is after the test has asked, and `mended` is the human having
/// been round to it, which is what the press finds.
fn a_backlog_of_one_dying_on_its_ask(prompts: &Path) -> String {
    format!(
        r#"
case "$1" in
{CODEX_GRILLING_MODEL})
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks 2>/dev/null | grep -E "^$number-" | head -n 1)
    printf '===== %s\n%s\n' "${{next:-finish}}" "$2" >> {prompts}
    if [ ! -f /tmp/verkstead/mended ]; then
        while [ ! -f /tmp/verkstead/dropped ]; do printf 'reading the task\n'; sleep 0.1; done
        printf 'gh: the connection dropped\n'
        exit 1
    fi
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: $next"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
        printf 'pushed, and the pull request is open\n'
    fi
    sleep 300
    ;;
esac
"#,
        prompts = quoted(prompts),
    )
}

/// A session that has gone before the Response lands is the folding rule's
/// case: nothing is typed anywhere, and its Answers open the next session's
/// prompt.
///
/// The two ends of a stored ask do not overlap, and this is which of them takes
/// a Set whose session died. Nothing about the folding is new — an answered
/// stored ask goes into the prompt of the next session started on its
/// Conversation, under the documents that prompt is built from, and folded once
/// — and the whole of what is under test is that the nudge stays out of it.
#[tokio::test]
async fn a_store_and_nudge_ask_whose_session_died_is_folded_in_and_nothing_is_typed() {
    let spill = tempfile::tempdir().unwrap();
    let written = spill.path().join("task-prompts");

    let fixture = grilling_spilling_on_codex(
        spill,
        &a_backlog_of_one_dying_on_its_ask(&written),
        PULL_REQUEST,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let proposal = fixture.ask_stored(PROPOSING).await;
    assert_eq!(
        fixture.pick(proposal, "task-list").await,
        Submitted::Accepted
    );

    // The step session, started on the task and asking something about it.
    until_written_by(&written, 1).await;

    let set = fixture.ask_stored(DEFERRED).await;

    // And then it goes, with the Set standing and nobody left to fetch it.
    std::fs::write(handoff_directory(&fixture).join("dropped"), "").unwrap();

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("the connection dropped"),
        "the run stopped over the session rather than over the question: {:?}",
        stopped.html,
    );

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([{
                    "label": "Q9",
                    "selected": 1,
                    "free_text": "and say which limit it hit",
                }]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("mended"), "").unwrap();
    assert_eq!(fixture.resume().await, Resumed::Resumed);

    // The step over again, and the finish step behind it — which is what says
    // the Answers went in once rather than into every session after them.
    until_written_by(&written, 3).await;

    let started = prompts_by_step(&written);
    let (_, first) = &started[0];
    let (_, second) = &started[1];
    let (_, third) = &started[2];

    assert!(
        second.contains("# What I have since said about the deferred questions"),
        "the session after the one that asked opens with the Answers: {second:?}",
    );
    assert!(
        second.contains("429 Too Many Requests") && second.contains("and say which limit it hit"),
        "and it is the exchange itself — the Option picked and what the human \
         wrote beside it: {second:?}",
    );
    assert!(
        !first.contains("429 Too Many Requests"),
        "the session that asked was started before it had asked: {first:?}",
    );
    assert!(
        !third.contains("429 Too Many Requests"),
        "and folded once: the step after it is primed with the work rather than \
         with a decision it has already been told: {third:?}",
    );

    assert!(
        nudges_about(&fixture, set).await.is_empty(),
        "and nothing was typed at anything: the session the Answers belonged to \
         had gone, and the one the press started reads them off its prompt",
    );
}

/// A Response to a `--deferred` Set types nothing, on the backend that stores
/// every ask as much as anywhere else.
///
/// The one thing a backend does not decide. `--deferred` is the agent saying
/// nobody is idling on this, and a nudge typed at a session that never stopped
/// working would be Verkstead interrupting it about a question it has already
/// carried on past.
#[tokio::test]
async fn a_response_to_a_deferred_ask_types_nothing_into_the_session_that_stored_it() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    // One that never goes quiet, so that it is still there to be typed at when
    // the Response lands: a session that has carried on is exactly the session
    // this must not speak to.
    let fixture = grilling_spilling_on_codex(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THAT_KEEPS_TALKING),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask_deferred(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 2 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    // A window with the Response settled and the session still printing, which
    // is where a nudge would land if one were coming.
    pause(BRISKLY.proposing * 2).await;

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "the session is still there for a line to have been typed at: {}",
        standing(&view),
    );
    assert!(
        nudges_about(&fixture, set).await.is_empty(),
        "and nothing was typed at it: nobody is idling on a Deferred Ask, \
         whatever the backend it was sent from",
    );
}

/// And a Response to a Blocking Ask types nothing either: the wait is what
/// delivers it.
///
/// Claude behaves exactly as it did. The session is sitting on `verkstead ask`
/// with the Response about to come back down it, and a line typed into that
/// terminal would be Verkstead talking over the answer on its way in.
#[tokio::test]
async fn a_response_to_a_blocking_ask_types_nothing_into_the_session_waiting_on_it() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX_AND_IDLE),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 2 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    // What stands in for the Response arriving down the wait, as it does in
    // every other test of an answered blocking session: a stub cannot idle on
    // one and wake up.
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the answered session was never seen out: {}",
            standing(&fixture.view().await),
        );
        pause(Duration::from_millis(25)).await;
    }

    assert!(
        nudges_about(&fixture, set).await.is_empty(),
        "and nothing was typed at it on the way: what a blocking ask is is a \
         session already holding the connection its Answers come back down",
    );
}

/// Every line Verkstead has typed at one of this Conversation's sessions about
/// this Set, as their own Captures hold it.
///
/// Read off the Capture rather than from inside a session, because a terminal
/// says what is typed into it: the keystrokes are echoed straight back out and
/// land in the session's own account of itself, whatever the session then does
/// about them. Which is what makes *nothing was typed* something that can be
/// asserted from out here at all — a stub that was never spoken to and one that
/// ignored what it was told look alike from everywhere else.
///
/// By the Set, because a Conversation run on this backend has more than one
/// stored ask in it: the proposal every one of these fixtures is directed by is
/// itself an ordinary ask, and its own session is told about it in the same
/// breath. What each of these tests is about is one Set.
async fn nudges_about(fixture: &Grilling, set_id: i64) -> Vec<String> {
    let view = fixture.view().await;
    let fetching = format!("verkstead answers {set_id}`");
    let mut typed = Vec::new();

    for output in outputs(&view) {
        let capture = fixture.capture(output.id).await;

        typed.extend(
            capture
                .lines()
                .filter(|line| line.contains(&fetching))
                .map(str::to_owned),
        );
    }

    typed
}

/// Where this Set of the Conversation's stands, or `None` where the Conversation
/// has no such Set.
fn where_it_stands(view: &ConversationView, set_id: i64) -> Option<verkstead_render::Standing> {
    sets(view)
        .into_iter()
        .find(|asked| asked.set_id == set_id)
        .map(|asked| asked.standing.clone())
}

/// Whether this Set of the Conversation's was answered, as against still open or
/// closed unanswered.
fn responded(view: &ConversationView, set_id: i64) -> bool {
    sets(view)
        .into_iter()
        .find(|asked| asked.set_id == set_id)
        .is_some_and(|asked| matches!(asked.standing, verkstead_render::Standing::Answered(_)))
}

/// A review that is still talking is never cut off, however long it goes on for.
///
/// Anything printed puts the whole grace back on the clock, which is what makes a
/// grace safe to end a session on: the work after a commit — a message, a
/// summary, a push — runs to completion rather than being killed mid-sentence.
#[tokio::test]
async fn a_review_that_keeps_talking_is_never_ended_under_it() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THAT_KEEPS_TALKING),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_written(&reviews).await;

    tokio::time::sleep(BRISKLY.proposing * 4).await;

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "a session that never stops printing is never one this ends: {:?}",
        outputs(&view).last(),
    );
    assert!(
        !review_settled(&fixture).await,
        "and nothing settled a review still being read",
    );
}

/// A review session that never says a word is not a review that found nothing.
///
/// The one place the quiet rule needs a second signal. Every other ending here
/// pairs quiet with something the session produced — a commit, a backlog, a
/// handoff — so a session that came up and did nothing satisfies none of them.
/// This one is satisfied by pure silence, and a review is exactly the session
/// whose whole report is its own words: reading silence as *it found nothing*
/// would settle the review and carry the wrap-up to Done over a branch nobody
/// read, with nothing on the Timeline saying so.
///
/// Green all the way through, so nothing but the review stands between this
/// wrap-up and Done — which is what makes the stop the whole proof.
#[tokio::test]
async fn a_review_that_never_said_anything_stops_the_run_rather_than_settling() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THAT_SAYS_NOTHING),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped
            .html
            .contains("Reviewing the branch the pull request is on"),
        "the step is named as what it was: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("never said anything"),
        "and the reason is that there was no report to read: {:?}",
        stopped.html,
    );
    assert!(
        !review_settled(&fixture).await,
        "a branch nobody said a word about is not a branch that was reviewed",
    );
    assert_ne!(
        fixture.view().await.state,
        Lifecycle::Done,
        "so the wrap-up does not carry on over the top of it",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "what is waiting is the human",
    );
}

/// A review session that dies is not a review that found nothing.
///
/// One is a branch nobody has read and the other is a branch somebody read and
/// had nothing to say about, and reading the first as the second would let a
/// crash pass for a clean bill of health. So the run stops like every other, and
/// what is on the Timeline is the Notice saying the review did not happen.
#[tokio::test]
async fn a_review_session_that_dies_halts_the_run_rather_than_passing_the_branch() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(
            &reviews,
            &dispatched,
            "    printf 'gh: could not read the diff\n'\n    exit 1",
        ),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped
            .html
            .contains("Reviewing the branch the pull request is on"),
        "the step is named as what it was: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("could not read the diff"),
        "with the tail of what the session said, which is where it says why: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "a branch nobody has read is not one a restart may carry on past",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "what is waiting is the human",
    );
    assert!(
        !review_settled(&fixture).await,
        "and a review that did not happen settles nothing",
    );
    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        1,
        "with nothing launched behind it: the run does not have another go at a \
         review of its own accord",
    );
}

/// A review session that dies **after** putting its findings up leaves a Set
/// nobody is behind, and that is not something to wait out.
///
/// The propose-then-fix shape has one session hold the whole of a review, its ask
/// included, so a session that goes between the asking and the answering takes
/// the only reader of that Set with it. Nothing is coming to read what the human
/// writes there, and no other session is ever handed somebody else's ask — so the
/// questions are closed as the run stops, which says on the Timeline that they
/// are off, and the retry is the branch read again rather than a wrap-up sitting
/// on a review that can never finish.
#[tokio::test]
async fn a_review_that_dies_on_its_own_ask_closes_its_questions_and_reads_the_branch_again() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let asked = spill.path().join("asked");
    let mended = spill.path().join("mended");

    // Puts its findings up and then falls over on its own ask — and once whatever
    // the human went off and did about it is done, reads the branch and finds
    // nothing.
    let review = format!(
        "    if [ -e {mended} ]; then\n        \
             printf 'I read the whole branch and found nothing worth raising\n'\n    \
         else\n        \
             printf 'reading the branch\n'\n        \
             while [ ! -e {asked} ]; do sleep 0.1; done\n        \
             printf 'gh: the connection dropped\n'\n        \
             exit 1\n    \
         fi",
        asked = quoted(&asked),
        mended = quoted(&mended),
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, &review),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    // The findings go up, and the session that would have read the answers dies
    // where it stood.
    let set = fixture.ask(REVIEW).await;
    std::fs::write(&asked, "").unwrap();

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("closed unanswered"),
        "the Notice says the questions are off: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("exited with status 1"),
        "beside how the session went: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("the connection dropped"),
        "with the tail of what it last said, which is where it says why: {:?}",
        stopped.html,
    );
    assert!(
        !review_settled(&fixture).await,
        "and a review nobody answered settles nothing",
    );

    let standing = sets(&fixture.view().await)
        .into_iter()
        .find(|asked| asked.set_id == set)
        .expect("the Set the dead session left open is on the Timeline")
        .standing
        .clone();

    assert!(
        matches!(standing, verkstead_render::Standing::LockedUnanswered(_)),
        "with nothing left for the human to answer into: {standing:?}",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "what is waiting is the human",
    );

    std::fs::write(&mended, "").unwrap();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the resume never reviewed anything, so the wrap-up never settled",
        );
        pause(Duration::from_millis(50)).await;
    }

    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        2,
        "the review that died on its ask, and the one the resume ran",
    );
    assert!(
        !dispatched.exists(),
        "with nothing dispatched to fix findings nobody ever decided about: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
}

/// A server that comes back up over a review still sitting on its ask does not
/// leave it there.
///
/// A session lives and dies with the process that started it, so a restart is the
/// same fact a crash is: the findings are up and nothing is behind them. Nothing
/// notices that by itself — the checks, the comments and the settling watcher are
/// all registered as driving the Conversation, so the stall sweep sees a wrap-up
/// being driven and the review's own entry point used to read *already asked* as
/// *nothing to do*. So the restart is what asks, and what it finds unanswered it
/// closes and stops the run over.
#[tokio::test]
async fn a_restart_over_a_review_waiting_on_its_ask_stops_the_run_rather_than_leaving_it() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    // Green all the way through, so that the only thing between this wrap-up and
    // Done is the review — which is what makes leaving it unattended a
    // Conversation that finishes with the human's questions still open.
    let stub = a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_WAIT);
    let gh = gh_about(GREEN, "", "");

    let fixture = grilling_spilling(spill, &stub, &gh).await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW).await;

    // A second server over the same database, which is what a restart is: the
    // session idling on that ask does not exist as far as it is concerned.
    let _restarted = fixture.restarted(&stub, &gh).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("the review"),
        "the stop is named as the half that failed: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("closed unanswered"),
        "and the Notice says the questions are off: {:?}",
        stopped.html,
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "the Conversation did not reach Done over a Set nobody answered",
    );
    assert!(
        !review_settled(&fixture).await,
        "and nothing settled the review it never finished",
    );

    let standing = sets(&view)
        .into_iter()
        .find(|asked| asked.set_id == set)
        .expect("the Set the gone session left open is on the Timeline")
        .standing
        .clone();

    assert!(
        matches!(standing, verkstead_render::Standing::LockedUnanswered(_)),
        "with nothing left for the human to answer into: {standing:?}",
    );
}

/// And one that comes back up over a review whose findings *were* answered stops
/// the run just the same, rather than picking the doing up off what was picked.
///
/// The propose-then-fix shape has one session hold the whole of a review, and a
/// session lives and dies with the process that started it however far through
/// it was. What the answers say is what the human decided; how much of it the
/// lost session had already carried out is beyond asking, and nothing reads a
/// record of the findings to guess. So the run stops, nothing is dispatched, and
/// the press is the branch read again by a session as fresh as the first — which
/// raises whatever is still worth raising and nothing that has since been put
/// right.
#[tokio::test]
async fn a_restart_over_an_answered_review_stops_the_run_rather_than_landing_anything() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let stub = a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_WORK_ON);

    let fixture = grilling_spilling(spill, &stub, PULL_REQUEST).await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "Keep the signature." },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    // And the process that was going to read those answers goes away.
    let _restarted = fixture.restarted(&stub, PULL_REQUEST).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("what the review found"),
        "the stop is named as the half that failed: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("Resuming reads the branch again"),
        "and says what going again means: {:?}",
        stopped.html,
    );
    assert!(
        !review_settled(&fixture).await,
        "a review nothing saw the end of settles nothing",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "what is waiting is the human",
    );
    assert!(
        !dispatched.exists(),
        "with nothing dispatched off what they picked: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        1,
        "and nothing read the branch behind the stop: going again is the human's \
         press, which is what the test below watches",
    );
}

/// A review session that ends having landed none of what the human accepted
/// settles all the same: what it did with the answers is its own to report.
///
/// The record could say otherwise — the findings they accepted are on the Set,
/// and there is no commit on the branch since — and it is deliberately not
/// asked. The session read the branch, put what it found, was answered and ran
/// to the end of what it had to say; a Verkstead that audited the branch against
/// the picks would be second-guessing the only participant that was there, and
/// stopping the run over a fix the session decided against on second look.
#[tokio::test]
async fn a_review_that_landed_nothing_still_settles_on_its_session_ending() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_VANISH),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    // One accepted, one declined, and the session lands neither.
    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "Keep the signature." },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the review session ended and the review never settled",
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped over what the record says was owed: {:?}",
        notices(&view),
    );
    assert_eq!(fixes(&view), 0, "and nothing was committed");
    assert!(
        !dispatched.exists(),
        "with nothing dispatched to land it afterwards: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        1,
        "and nothing read the branch a second time",
    );
}

/// But a review session that *dies* after the answers stops the run, and
/// dispatches nothing to finish what it started.
///
/// The other half of the same rule: what the session did with the answers is its
/// report to make, and one that fell over made none. There is nothing here that
/// knows whether the fixes landed, so the run stops with the tail of what it said
/// as the evidence — and the press is the branch read afresh rather than a
/// session handed decisions off the record. What that fresh reading raises is
/// whatever is still worth raising on the branch as it now stands.
#[tokio::test]
async fn a_review_that_dies_after_the_answers_stops_the_run_and_dispatches_nothing() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let mended = spill.path().join("mended");

    // Falls over on the answers — and once whatever the human went off and did
    // about it is done, reads the branch and finds nothing.
    let review = format!(
        "    if [ -e {mended} ]; then\n        \
             printf 'I read the whole branch and found nothing worth raising\n'\n    \
         else\n{die}\n    \
         fi",
        mended = quoted(&mended),
        die = REVIEW_THEN_DIE_ON_THE_ANSWERS,
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, &review),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "Keep the signature." },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let stopped = fixture.stopped().await;

    assert!(
        stopped
            .html
            .contains("Reviewing the branch the pull request is on"),
        "the step is named as what it was: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("the connection dropped"),
        "with the tail of what the session said, which is where it says why: {:?}",
        stopped.html,
    );
    assert!(
        !review_settled(&fixture).await,
        "a review that did not finish settles nothing",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "what is waiting is the human",
    );
    assert!(
        !dispatched.exists(),
        "and nothing was dispatched to carry out what it was answered: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        1,
        "with nothing launched behind the stop either",
    );

    // And the press is the review over from the start: a session as fresh as the
    // first, reading the branch rather than stopping over the Set again.
    std::fs::write(&mended, "").unwrap();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the press never reviewed anything, so the wrap-up never settled",
        );
        pause(Duration::from_millis(50)).await;
    }

    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        2,
        "the review that died on the answers, and the one the press ran",
    );
    assert!(
        !dispatched.exists(),
        "and still nothing dispatched from the record: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
}

/// A Conversation can wrap up twice, and the second wrap is a whole one.
///
/// The plumbing a split-out backlog lands on: the work goes back to be built,
/// its finish step wraps it up again, and what comes back is a branch nobody has
/// read. So the review runs afresh — *settled once and stays settled* is a rule
/// about one wrap rather than about the Conversation — and the first wrap's own
/// Set is no longer the review it finds asking. Nothing is recorded twice
/// either: the pull request the second finish step opens is the one the first
/// one did.
///
/// The two moves are made here rather than by a session, so that what is under
/// test is the plumbing on its own — the whole path a review takes through it is
/// [`a_review_that_split_a_finding_out_sends_the_work_back_to_be_built`]. What
/// starts the second wrap's watchers is a restart, which takes up every
/// Conversation it finds wrapping.
///
/// The checks cannot be asked about, which is what keeps both wraps going: one
/// that had finished would be a Conversation there was nothing left to review
/// from.
#[tokio::test]
async fn a_conversation_sent_back_to_be_built_wraps_up_and_reviews_again() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let stub = a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX);
    let gh = gh_about(CHECKS_UNANSWERABLE, "", "");

    let fixture = grilling_spilling(spill, &stub, &gh).await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    // The first wrap, whole: findings put, answered, fixed and settled.
    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "Keep the signature." },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the first wrap's review never settled",
        );
        pause(Duration::from_millis(25)).await;
    }

    assert_eq!(
        review_asked(&fixture).await,
        Some(set),
        "which is the Set the first wrap put its findings on",
    );

    // What the session that split its findings out into a backlog leaves behind,
    // and what the finish step that follows the backlog then does.
    let pool = open_database(&fixture.database).await.unwrap();

    assert_eq!(
        verkstead_server::store::implement_again(&pool, fixture.id)
            .await
            .unwrap(),
        verkstead_server::store::Rebuilding::Started,
    );
    let repo = verkstead_server::store::load_conversation(&pool, fixture.id)
        .await
        .unwrap()
        .unwrap()
        .repo
        .id;

    assert_eq!(
        verkstead_server::store::record_pull_request(
            &pool,
            fixture.id,
            repo,
            &verkstead_server::store::PullRequest {
                number: 41,
                title: "Rate limiting".to_owned(),
                url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
                repo: None,
            },
        )
        .await
        .unwrap(),
        verkstead_server::store::Wrapping::Started,
    );

    pool.close().await;

    assert!(
        !review_settled(&fixture).await,
        "leaving Wrapping took the review's settle with it",
    );
    assert_eq!(
        review_asked(&fixture).await,
        None,
        "and the first wrap's Set is not this wrap's review",
    );

    // A second server over the same database, which takes up every Conversation
    // it finds wrapping up — the whole of a wrap-up, its review included.
    let _restarted = fixture.restarted(&stub, &gh).await;

    let deadline = Instant::now() + *PATIENCE;
    let read_again = loop {
        let written = std::fs::read_to_string(&reviews).unwrap_or_default();

        if prompts(&written).len() > 1 {
            break written;
        }

        assert!(
            Instant::now() < deadline,
            "the second wrap never read the branch: {written}",
        );
        pause(Duration::from_millis(25)).await;
    };

    assert_eq!(
        prompts(&read_again).len(),
        2,
        "one review per wrap, and the second wrap ran its own: {read_again}",
    );
    assert!(
        prompts(&read_again)
            .iter()
            .all(|prompt| prompt.contains("model=claude-review-5")),
        "each of them under the review Pairing, the fresh eyes being what a \
         review is however many times the branch is looked at: {read_again}",
    );

    let pool = open_database(&fixture.database).await.unwrap();
    let events = verkstead_server::store::timeline(&pool, fixture.id)
        .await
        .unwrap();
    pool.close().await;

    let requests = events
        .iter()
        .filter(|event| matches!(event.event, verkstead_server::store::Event::PullRequest(_)))
        .count();

    assert_eq!(
        requests, 1,
        "one branch, one pull request, however many times it is wrapped up",
    );
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "and the second wrap is where the Conversation is, its checks still unanswerable",
    );
}

/// Which Set this Conversation's wrap-up has put to the human, as Verkstead
/// reads it — and `None` where the wrap it is in has asked nothing.
async fn review_asked(fixture: &Grilling) -> Option<i64> {
    let pool = open_database(&fixture.database).await.unwrap();
    let asked = verkstead_server::store::last_proposal(&pool, fixture.id)
        .await
        .unwrap();
    pool.close().await;

    asked
}

/// The Set a review writes where one of its findings is too big to fix in the
/// sitting it was found in: a third Option on that Question, offering to spin
/// the work out as a backlog of its own, so all three answers mean something.
const REVIEW_WITH_A_SPLIT: &str = r#"
title: Review of the rate limiter branch
preface: |
  Two things worth a decision, and one of them is bigger than this sitting.
questions:
  - label: Q1
    text: The window counter is never reset between windows.
    options:
      - n: 1
        text: Fix it
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: The clock abstraction wants rebuilding rather than patching.
    options:
      - n: 1
        text: Fix it here
      - n: 2
        text: Split it out as its own work
        recommended: true
      - n: 3
        text: Leave it
"#;

/// A review session that writes what was split out as a `.tasks/` backlog and
/// then ends — once.
///
/// Once, because the same stub runs the second wrap's review: a session that
/// split its findings out every time it read the branch would send the work back
/// for ever, and what a test wants to watch is the round trip finishing. The
/// marker is in the spilling directory, which is the one thing bound writable
/// into every Sandbox of the fixture.
///
/// `also` is whatever it does before writing the backlog, which is how a test
/// says whether anything was accepted to fix here as well.
fn review_then_split(once: &Path, also: &str) -> String {
    format!(
        "    printf 'reading the branch\n'\n    \
         if [ -e {once} ]; then\n        \
         printf 'I read the whole branch and found nothing worth raising\n'\n        \
         exit 0\n    \
         fi\n    \
         : > {once}\n    \
         while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
         {also}mkdir -p .tasks\n    \
         printf '# Rebuilding the clock\n\n## Tasks\n\n' > .tasks/TODO.md\n    \
         printf -- '- [ ] 01: collapse the clocks\n' >> .tasks/TODO.md\n    \
         printf '# 01. Collapse the clocks\n' > .tasks/01-clocks.md\n    \
         git add -A\n    \
         git commit --quiet -m 'chore: plan the clock tasks'\n    \
         printf 'fixed what was accepted and split the rest out\n'",
        once = quoted(once),
    )
}

/// What a session does about a finding the human accepted to fix here, as the
/// half of [`review_then_split`] a mixed pick adds.
const AND_A_FIX: &str = "printf 'a fix\n' >> fixes.md\n    \
     git add -A\n    \
     git commit --quiet -m 'fix: reset the counter as the window rolls'\n    ";

/// How many times this Conversation has been moved into `state`, which is what
/// tells a second wrap from a first.
fn moves_into(view: &ConversationView, state: Lifecycle) -> usize {
    view.timeline
        .iter()
        .filter(|event| matches!(event, TimelineEvent::Moved(moved) if moved.state == state))
        .count()
}

/// The escape hatch, end to end.
///
/// One review, two findings, and the human answers them differently: fix the
/// first here, split the second out. So the session does both — the fix
/// committed and pushed as any accepted finding is, the split written down as a
/// `.tasks/` backlog — and what Verkstead does with a wrap-up that ended holding
/// a backlog is send it back down the ladder. The list is then worked a session
/// at a time like any other, and the finish that follows the last task wraps the
/// work up again on the pull request it already had, read afresh by a review that
/// knows nothing of the first.
///
/// The checks cannot be asked about, which is what keeps both wraps going: one
/// that had finished would be a Conversation there was nothing left to review
/// from.
#[tokio::test]
async fn a_review_that_split_a_finding_out_sends_the_work_back_to_be_built() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let once = spill.path().join("split-written");

    let stub = a_backlog_then_wraps_up(&reviews, &dispatched, &review_then_split(&once, AND_A_FIX));
    let gh = gh_about(CHECKS_UNANSWERABLE, "", "");

    let fixture = grilling_spilling(spill, &stub, &gh).await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW_WITH_A_SPLIT).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1 },
                    { "label": "Q2", "selected": 2, "free_text": "Keep the public signature." },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    // The one move down the ladder: a wrap-up that ended holding a backlog is a
    // Conversation with work to build.
    fixture
        .until(|view| (moves_into(view, Lifecycle::Implementing) == 2).then_some(()))
        .await;

    // Which is then worked and finished, and the finish wraps it up a second
    // time on the pull request it already had.
    fixture
        .until(|view| (moves_into(view, Lifecycle::Wrapping) == 2).then_some(()))
        .await;

    let deadline = Instant::now() + *PATIENCE;
    let read_again = loop {
        let written = std::fs::read_to_string(&reviews).unwrap_or_default();

        if prompts(&written).len() > 1 {
            break written;
        }

        assert!(
            Instant::now() < deadline,
            "the second wrap never read the branch: {written}",
        );
        pause(Duration::from_millis(25)).await;
    };

    assert_eq!(
        prompts(&read_again).len(),
        2,
        "one review per wrap, and the second wrap ran its own: {read_again}",
    );
    assert!(
        prompts(&read_again)
            .iter()
            .all(|prompt| prompt.contains("model=claude-review-5")),
        "each of them under the review Pairing, the fresh eyes being what a \
         review is however many times the branch is looked at: {read_again}",
    );

    let view = fixture.view().await;
    let landed: Vec<&str> = commits(&view)
        .iter()
        .map(|commit| commit.subject.as_str())
        .collect();

    assert!(
        landed
            .iter()
            .any(|subject| subject.starts_with("fix: reset the counter")),
        "the finding they accepted was fixed by the session that raised it: {landed:?}",
    );
    assert!(
        landed
            .iter()
            .any(|subject| subject.starts_with("chore: plan the clock tasks")),
        "and the one they split out was written down rather than built: {landed:?}",
    );
    assert_eq!(
        landed
            .iter()
            .filter(|subject| subject.starts_with("chore: finish"))
            .count(),
        2,
        "which was then worked to empty and finished, like any other backlog: {landed:?}",
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped on the way: {:?}",
        notices(&view),
    );
    assert_eq!(
        sets(&view).len(),
        2,
        "the human was asked twice — the grilling's proposal and the one review \
         that found anything: {:?}",
        sets(&view).len(),
    );
}

/// A Response that accepted nothing to fix here and split one finding out works
/// the same way: there is nothing to commit but the backlog, and committing the
/// backlog is the whole of what the session was answered.
///
/// The half of the rule that would be easy to get wrong — a wrap-up that only
/// went back down the ladder where something had been fixed first would strand
/// the split-out work of every review whose other findings were declined. The
/// list on the branch is the whole signal, and it does not need a commit beside
/// it to count.
#[tokio::test]
async fn a_split_with_nothing_else_accepted_still_sends_the_work_back() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let once = spill.path().join("split-written");

    let stub = a_backlog_then_wraps_up(&reviews, &dispatched, &review_then_split(&once, ""));
    let gh = gh_about(CHECKS_UNANSWERABLE, "", "");

    let fixture = grilling_spilling(spill, &stub, &gh).await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW_WITH_A_SPLIT).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 2 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture
        .until(|view| (moves_into(view, Lifecycle::Implementing) == 2).then_some(()))
        .await;

    let view = fixture.view().await;
    let landed: Vec<&str> = commits(&view)
        .iter()
        .map(|commit| commit.subject.as_str())
        .collect();

    assert!(
        landed
            .iter()
            .any(|subject| subject.starts_with("chore: plan the clock tasks")),
        "the backlog is what it committed: {landed:?}",
    );
    assert!(
        !landed
            .iter()
            .any(|subject| subject.starts_with("fix: reset the counter")),
        "and nothing was fixed, because nothing was accepted to fix: {landed:?}",
    );
    assert!(
        notices(&view).is_empty(),
        "a review that fixed nothing and split something out owes nobody a fix: {:?}",
        notices(&view),
    );
}

/// An inline run whose review splits work out: the backlog that arrives is the
/// first this Conversation has ever carried, and the record says where it
/// landed.
///
/// The landing a Conversation built from a backlog cannot show. There, the row
/// was stamped by the breakdown long before the review, so a split-out backlog
/// that stamped nothing would look exactly right. An inline run has no backlog
/// at all until the review writes one, which is what makes this the case that
/// tells the two apart.
fn an_inline_run_then_splits(reviews: &Path, review: &str) -> String {
    format!(
        r#"
printf 'prompt was: %s\n' "$2"

case "$2" in
*reviewing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {reviews}
{review}
    ;;
*implementing/SKILL.md*)
    printf 'a limiter\n' > limiter.md
    git add limiter.md
    git commit --quiet -m 'feat: rate limiting'
    printf 'pushed, and the pull request is open\n'
    ;;
*next-task/SKILL.md*)
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'one clock\n' >> clocks.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m 'feat: collapse the clocks'
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish the clocks'
        printf 'pushed, and the pull request is open\n'
    fi
    sleep 300
    ;;
*)
    printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
    printf 'the handoff is written\n'
    sleep 300
    ;;
esac
"#,
        reviews = quoted(reviews),
    )
}

#[tokio::test]
async fn a_split_out_backlog_lands_on_the_record_of_a_run_that_never_had_one() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let once = spill.path().join("split-written");

    let stub = an_inline_run_then_splits(&reviews, &review_then_split(&once, ""));
    let gh = gh_about(CHECKS_UNANSWERABLE, "", "");

    let fixture = grilling_spilling(spill, &stub, &gh).await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    until_asking(&fixture, &reviews).await;

    // Nothing has been broken down and nothing ever will be by this run, so
    // there is no landing to have stamped yet.
    let view = fixture.view().await;

    assert!(
        backlog(&view).is_none(),
        "an inline run carries no backlog: {:?}",
        view.pinned,
    );
    assert!(
        backlog_row(&view).is_none(),
        "and nothing has landed one on the record: {:?}",
        view.timeline,
    );

    let set = fixture.ask(REVIEW_WITH_A_SPLIT).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 2 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    // The one move down the ladder, which is the review's backlog being picked
    // up — and the landing that goes on the record with it.
    fixture
        .until(|view| (moves_into(view, Lifecycle::Implementing) == 2).then_some(()))
        .await;

    let reached = fixture.until(|view| backlog_row(view).cloned()).await;

    assert_eq!(
        reached.list.map(|list| list.feature),
        Some("Rebuilding the clock".to_owned()),
        "the row draws the backlog the review wrote",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.timeline
            .iter()
            .filter(|event| matches!(event, TimelineEvent::TaskList(_)))
            .count(),
        1,
        "one row, a list landing once: {:?}",
        view.timeline,
    );
}

/// And a split pick the session wrote no backlog for settles like any other
/// review, because the branch is the whole of what says otherwise.
///
/// The Option the human picked is not consulted and there is nothing else that
/// could be: the session that offered it was the one answered and the one that
/// would have written the list, and it committed a fix and no list. Which is a
/// session that thought better of the spin-off between the ask and the doing,
/// and that is its to think better of.
#[tokio::test]
async fn a_split_no_backlog_was_written_for_settles_like_any_other_review() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW_WITH_A_SPLIT).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the review session ended and the review never settled",
        );
        pause(Duration::from_millis(25)).await;
    }

    // And then let the wrap-up run to its ordinary end, which is the whole of
    // what *like any other review* means: the review is answered, the checks are
    // green and nothing is left unaddressed, so the Conversation is Done.
    //
    // Waited for rather than read. The state a moment after the review settles
    // is a fact about which of two things won a race — this read, or the loop
    // that ends a wrap-up — and *still Wrapping* is the answer only on a machine
    // with nothing else to do. Done is where it comes to rest either way, and
    // reaching it says more than catching it on the way: a run that had gone
    // back down the ladder would never arrive.
    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped over a backlog the record said was owed: {:?}",
        notices(&view),
    );
    assert_eq!(
        moves_into(&view, Lifecycle::Implementing),
        1,
        "and nothing went back down the ladder: there is no backlog to build",
    );
    assert!(
        !dispatched.exists(),
        "with nothing dispatched to write the list instead: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
}

/// One agent in one Worktree, which is what the wrap-up's turns are for.
///
/// The checks are watched while the review runs, and starting a session for a
/// Conversation *ends* the one it already has — so a red check dispatching a fix
/// session mid-review would kill the review, and nothing would ever say so. It
/// waits for the Worktree instead, across the whole of the review: the ask that
/// blocks for hours is a session working rather than a Worktree free, and it is
/// only once that session has fixed what was accepted and gone that the check's
/// own fix gets its turn.
///
/// And it waits having cost the check nothing: an attempt is spent where a fix
/// session is dispatched, so a suite red for the whole of a wait still has both
/// of its goes afterwards. What the reviewing skill sends the woken session to do
/// about the check meanwhile is the fold-in, and no counter here knows about it.
#[tokio::test]
async fn a_red_check_waits_for_the_worktree_rather_than_ending_the_review() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    // Red only once the review has written its prompt, which is the review inside
    // the sandbox and so the Worktree already taken — see [`gh_checking_after`].
    // A suite red from the first poll would be a race with the review for it.
    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX),
        &gh_checking_after(&reviews),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    // The review puts its findings down and the human answers them, which is the
    // hours a red check has to wait through.
    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    // Long enough for many polls of a suite that is red the whole time.
    pause(Duration::from_millis(500)).await;

    assert!(
        !dispatched.exists(),
        "nothing was dispatched into the Worktree the review is working in: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );

    assert_eq!(
        attempts_spent(&fixture, "Rust").await,
        0,
        "and the check has spent none of its two attempts on the wait: an attempt \
         is counted where a fix session is dispatched, and nothing was",
    );

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "and the review session is still the one running: {:?}",
        outputs(&view).last(),
    );

    // Now it lands what was accepted and ends, which hands the Worktree on.
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let told = until_written(&dispatched).await;

    assert!(
        told.contains("Rust") && told.contains("/actions/runs/1/job/2"),
        "the fix session that was waiting is about the red check: {told}",
    );

    assert_eq!(
        attempts_spent(&fixture, "Rust").await,
        1,
        "and it is the first of the two, spent here rather than during the wait: \
         whatever the woken review did about the check cost the counter nothing",
    );
}

/// Whether Verkstead has recorded that nothing said on this Conversation's own
/// pull request is left unaddressed.
///
/// The Conversation's own, there being one conversation per pull request: a
/// companion's is [`companion_comments_settled`]'s.
async fn comments_settled(fixture: &Grilling) -> bool {
    settled(
        fixture,
        verkstead_server::store::WaitingOn::Comments(own_repo(fixture).await),
    )
    .await
}

/// And whether nothing is left unaddressed on the pull request opened in the
/// companion beside it.
async fn companion_comments_settled(fixture: &Grilling) -> bool {
    settled(
        fixture,
        verkstead_server::store::WaitingOn::Comments(companion_repo(fixture).await),
    )
    .await
}

/// What was already said on the pull request when the wrap-up's review starts is
/// part of what that session reads — from all three places a human writes — and
/// nothing is dispatched to act on any of it.
///
/// This is the whole of what stops a comment being acted on ungated. The review
/// is the session that proposes, so what has been said reaches it whole, in the
/// order it was said in and with where each of it was said, and goes into the one
/// Set beside the findings it made itself. Recorded as addressed as it is
/// dispatched, so no batch session is later sent about the same words.
///
/// The checks cannot be asked about, which keeps the Conversation wrapping up long
/// enough to watch nothing happen.
#[tokio::test]
async fn what_was_already_said_reaches_the_review_rather_than_a_session_of_its_own() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(CHECKS_UNANSWERABLE, THREE_COMMENTS, TWO_ON_THE_DIFF),
    )
    .await;

    worked_to_empty(&fixture).await;

    let told = until_written(&reviews).await;

    assert!(
        told.contains("Rename the window field.")
            && told.contains("And the test that pins it.")
            && told.contains("Otherwise this reads well."),
        "everything said in the conversation reached the review: {told}",
    );
    assert!(
        told.contains("This is the wrong way round.")
            && told.contains("And this one has no home any more."),
        "and so did what was said on the lines of the diff: {told}",
    );
    assert!(
        told.contains("`src/window.rs` line 12"),
        "with where it was said, which is half of what it means: {told}",
    );

    // Written down as the review was dispatched, so what is left unaddressed is
    // nothing — which is what says no batch session is owed about them.
    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "what was said to the review was never recorded as addressed",
        );
        pause(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls of a pull request with five comments on it.
    pause(Duration::from_millis(500)).await;

    assert!(
        !dispatched.exists(),
        "and nothing was dispatched to act on any of it: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
}

/// The comment a share leaves on a pull request is Verkstead's own, so nothing is
/// ever dispatched about it and the review is never given it — while a human
/// quote-replying to it is somebody to answer like anybody else.
///
/// Share to Pull Request writes as the configured token, which is usually the
/// human's own account, so no rule about who said it could tell the two apart:
/// the marker at the start of a line is what does. Quote-replying puts a `>` in
/// front of every line of what is quoted, which is why the rule is written about
/// the start of a line and not about the text appearing at all.
///
/// The checks cannot be asked about, which keeps the Conversation wrapping up
/// long enough to watch nothing happen.
#[tokio::test]
async fn the_comment_a_share_left_is_never_addressed_and_a_reply_to_it_is() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    assert!(
        A_SHARE_AND_A_REPLY.contains(verkstead_render::SHARE_MARKER),
        "the fixture is the comment a share actually leaves",
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(CHECKS_UNANSWERABLE, A_SHARE_AND_A_REPLY, ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    let told = until_written(&reviews).await;

    assert!(
        !told
            .lines()
            .any(|line| line.starts_with(verkstead_render::SHARE_MARKER)),
        "what Verkstead said itself was not folded into the review: {told}",
    );
    assert!(
        told.contains("Which of these is the one to keep?"),
        "and the human's reply to it was: {told}",
    );

    // Nothing is left unaddressed: the reply went to the review, and the share's
    // own comment was never anything to address.
    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the pull request never settled, so the share's own comment was read \
             as something to answer",
        );
        pause(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls of a pull request the share comment is
    // still standing on, and will be standing on for ever.
    pause(Duration::from_millis(500)).await;

    assert!(
        !dispatched.exists(),
        "and no session was dispatched about any of it: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
}

/// The comments the human's own ignore rules match are skipped wherever Wrapping
/// reads them: they never reach the review prompt, nothing is dispatched about
/// them, and each is written down as addressed on the way past.
///
/// What the rules are for is a bot nobody can turn off — a review service set up
/// with no billing information on it, filing the same word about billing on every
/// pull request — where the alternative is a session spun up to address it each
/// time. So the test is written about what a rule has to leave alone as much as
/// what it takes away: the same bot saying something worth reading is still
/// somebody to answer, and so is the human using the word the rule is about.
///
/// Both fields of the first rule have to match and the two rules combine with OR,
/// and the bot's note on a line of the diff is matched by the same rule as its
/// note in the conversation — the three places a human writes arrive as one
/// stream of comments, so there is one check across all of them.
///
/// The checks cannot be asked about, which keeps the Conversation wrapping up
/// long enough to watch nothing happen.
#[tokio::test]
async fn the_comments_the_ignore_rules_match_reach_nobody_and_are_written_down() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about_with_a_pane(CHECKS_UNANSWERABLE, TWO_BOTS_AND_A_HUMAN, A_BOT_ON_THE_DIFF),
    )
    .await;

    // Said long before the wrap-up is anywhere near: the file is read on every
    // poll, so what matters is that the rules are there by the time the comments
    // are.
    configure(&fixture, THE_BOTS_IGNORED);

    worked_to_empty(&fixture).await;

    let told = until_written(&reviews).await;

    assert!(
        !told.contains("Your billing information is missing.")
            && !told.contains("Billing information is still missing."),
        "what the first rule matches was not folded into the review, in the \
         conversation or on the diff: {told}",
    );
    assert!(
        !told.contains("Bump serde"),
        "nor what the second one does: {told}",
    );
    assert!(
        told.contains("This loop reads the vector twice."),
        "the same bot saying something worth reading is still somebody to \
         answer, both fields of that rule having to match: {told}",
    );
    assert!(
        told.contains("We should sort the billing out one day."),
        "and so is the human using the word the rule is about: {told}",
    );

    // Written down as they were skipped, which is what makes deleting the rule
    // later change what happens next rather than what happened. The two the
    // review was given are written down beside them, as the review's always are.
    let mut written = addressed(&fixture).await;
    written.sort();

    assert_eq!(
        written,
        ["IC_1", "IC_2", "IC_3", "IC_4", "PRRC_9"],
        "every comment on the pull request is dealt with: three by a rule and \
         two by the review",
    );

    // Long enough for many more polls of a pull request with five comments on it.
    pause(Duration::from_millis(500)).await;

    assert!(
        !dispatched.exists(),
        "and nothing was dispatched about any of it: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );

    // The ignore is about agent work and nothing else. What the pane lists is
    // what is on the pull request, read straight off `gh` as it opens — a
    // comment nobody is being sent to answer is still a comment the human wrote
    // a rule about, and hiding it would leave them no way to see what the rule
    // is doing.
    let opened = pull_request(&fixture.view().await)
        .expect("the wrap-up has its pull request pinned")
        .id;

    let carried: verkstead_render::PullRequestDetails = get(
        &fixture.app,
        &format!(
            "/api/ui/conversations/{}/pull-request/{}",
            fixture.id, opened
        ),
    )
    .await;

    assert!(
        carried
            .comments
            .iter()
            .any(|comment| comment.html.contains("Your billing information is missing")),
        "the details pane still lists what the rules skipped: {:?}",
        carried.comments,
    );
}

/// A rule written while a wrap-up is already watching takes effect on the next
/// poll, and taking it away again brings nothing back.
///
/// Both halves are what the rules are read off the file for rather than held from
/// startup. The human is on a phone reading a pull request a bot has just filed
/// its note on: they write the rule where they are, and the poll after it is the
/// one that skips the comment — no restart, and nothing for them to go back to
/// the machine for.
///
/// And the comment is written down as addressed as it is skipped, which is the
/// other half. A rule silencing months of a bot's nagging would otherwise be a
/// rule nobody could ever delete: the day they did, the whole of it would come
/// back as sessions on the next poll.
///
/// The bot says nothing until the test says so, which is how it says the comment
/// landed after the review — everything standing when the review starts is the
/// review's to propose about. The checks cannot be asked about, which keeps the
/// Conversation wrapping up long enough to watch nothing happen.
#[tokio::test]
async fn a_rule_written_while_the_wrap_up_watches_takes_effect_and_deleting_it_brings_nothing_back()
{
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let landed = spill.path().join("the-bot-has-filed-its-note");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about_once(CHECKS_UNANSWERABLE, &landed, A_BOTS_BILLING_NOTE, ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    until_written(&reviews).await;

    // The review is over and nothing has been said, which is the wrap-up sitting
    // where the human finds it: watching a pull request nobody has written on.
    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the pull request nobody had written on never settled",
        );
        pause(Duration::from_millis(25)).await;
    }

    // Then the human writes the rule, on their phone and with the server they
    // wrote it against still running — and only then does the bot file its note.
    configure(&fixture, THE_BOTS_IGNORED);
    std::fs::write(&landed, "filed").unwrap();

    // The next poll skips it and writes it down, which is the whole of what the
    // rule taking effect without a restart looks like from outside.
    let deadline = Instant::now() + *PATIENCE;
    while addressed(&fixture).await != ["IC_5"] {
        assert!(
            Instant::now() < deadline,
            "the rule written mid-wrap-up never took effect: {:?}",
            addressed(&fixture).await,
        );
        pause(Duration::from_millis(25)).await;
    }

    assert!(
        !dispatched.exists(),
        "and no session was dispatched about it: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );

    // And the human deletes the rule again, the bot's note still standing on the
    // pull request. Nothing comes back: it was written down as addressed the
    // moment it was skipped.
    configure(&fixture, "");

    // Long enough for many more polls of a pull request the note is still on.
    pause(Duration::from_millis(500)).await;

    assert!(
        !dispatched.exists(),
        "deleting the rule resurrected what it had already silenced: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert!(
        comments_settled(&fixture).await,
        "and the pull request is still settled, nothing being left unaddressed \
         on it",
    );
}

/// A comment that lands while the review's Set is in flight is not folded into
/// it, and reaches a batch session once the Worktree is free — everything said in
/// a minute reaching **one**.
///
/// A human writing five times is making one point, and five sessions racing each
/// other in one Worktree is the thing a batch prevents. So the whole batch goes
/// to one session inside the bundled addressing skill, which commits and pushes
/// as that skill says.
///
/// Three of them are in the pull request's conversation and two are on the lines
/// of the diff, which is where a review of code mostly happens: a watcher that
/// read only the conversation would miss the feedback it most needs to act on.
#[tokio::test]
async fn comments_said_while_the_review_runs_reach_one_batch_session_afterwards() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    // Nothing has been said until the review session has written its prompt down,
    // which is that session inside its sandbox and so the Worktree already taken.
    let gh = gh_about_once(GREEN, &reviews, THREE_COMMENTS, TWO_ON_THE_DIFF);

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX),
        &gh,
    )
    .await;

    worked_to_empty(&fixture).await;

    let told = until_asking(&fixture, &reviews).await;

    assert!(
        !told.contains("Rename the window field."),
        "a comment said after the review started is not one it was given: {told}",
    );

    let set = fixture.ask(REVIEW).await;

    // Long enough for many polls of a pull request that now has five comments on
    // it, while the review still holds the Worktree.
    pause(Duration::from_millis(500)).await;

    assert!(
        !dispatched.exists(),
        "nothing is dispatched while the review's Set is in flight: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let told = until_written(&dispatched).await;

    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "what was said was never addressed",
        );
        pause(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls of a pull request whose comments have all
    // been dispatched for.
    pause(Duration::from_millis(500)).await;

    let told = std::fs::read_to_string(&dispatched).unwrap_or(told);

    assert_eq!(
        prompts(&told).len(),
        1,
        "one session between the five, rather than one each: {told}",
    );
    assert!(
        told.contains("responding/SKILL.md"),
        "inside the bundled responding skill: {told}",
    );
    assert!(
        told.contains("Rename the window field.")
            && told.contains("And the test that pins it.")
            && told.contains("Otherwise this reads well."),
        "everything said in the conversation reached it: {told}",
    );
    assert!(
        told.contains("This is the wrong way round.")
            && told.contains("And this one has no home any more."),
        "and so did what was said on the lines of the diff: {told}",
    );
    assert!(
        told.contains("`src/window.rs` line 12"),
        "with where it was said, which is half of what it means: {told}",
    );
}

/// A batch of comments is proposed about before anything is changed, and fixed
/// by the same session on approval.
///
/// The whole of what stops a comment being acted on ungated once the review is
/// over. What somebody wrote on a pull request says what they think is wrong, not
/// what to do about it — so the session reads the batch, puts what it would do to
/// the human as one small Set, changes nothing while it waits, and lands what
/// they accepted itself.
///
/// The checks cannot be asked about, which keeps the Conversation wrapping up
/// long enough to watch the whole of it.
#[tokio::test]
async fn a_batch_of_comments_is_proposed_about_and_then_fixed_in_the_same_session() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let batches = spill.path().join("batch-prompts");

    // Nothing has been said until the review session has written its prompt down:
    // everything standing on the pull request when it starts is the review's own
    // to propose about.
    let gh = gh_about_once(
        CHECKS_UNANSWERABLE,
        &reviews,
        THREE_COMMENTS,
        TWO_ON_THE_DIFF,
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_answers_comments(&reviews, &dispatched, &batches, RESPOND_THEN_FIX),
        &gh,
    )
    .await;

    worked_to_empty(&fixture).await;

    let told = until_asking(&fixture, &batches).await;

    assert_eq!(prompts(&told).len(), 1, "one session for the batch: {told}");
    assert!(
        told.contains("responding/SKILL.md"),
        "inside the bundled responding skill: {told}",
    );
    assert!(
        told.contains("model=claude-implementation-5"),
        "under the implementation Profile, as every session about the code is: {told}",
    );
    assert!(
        told.contains("Rename the window field.") && told.contains("`src/window.rs` line 12"),
        "given what was said and where, whole: {told}",
    );

    // Long enough for a session told to do what it was given to have done it.
    pause(Duration::from_millis(500)).await;

    assert_eq!(
        fixes(&fixture.view().await),
        0,
        "and nothing is changed before the human has answered",
    );

    // What the batch session does through the CLI, played by the test.
    let set = fixture.ask(ANSWERING_THE_COMMENTS).await;

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "the session that asked is the one that fixes, so nothing ends it on the ask: {:?}",
        outputs(&view).last(),
    );

    // The human answers from the workbench: do the first, leave the second.
    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture.until(|view| (fixes(view) == 1).then_some(())).await;

    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the batch landed its fix and what was said never settled",
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped: {:?}",
        notices(&view),
    );
    assert!(
        !dispatched.exists(),
        "and nothing was dispatched to fix anything: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
}

/// A batch with nothing in it worth doing asks nothing at all.
///
/// A question the commits since have answered, or somebody saying this reads
/// well: a Set about that would be a row for the human to dismiss, and the point
/// of asking is to spend their attention only where there is a decision. So the
/// session says what it made of the batch where they are already looking — the
/// last line a session prints is what its Timeline row shows — and the batch
/// settles as addressed.
#[tokio::test]
async fn a_batch_with_nothing_to_do_asks_nothing_and_settles_as_addressed() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let batches = spill.path().join("batch-prompts");

    let gh = gh_about_once(CHECKS_UNANSWERABLE, &reviews, THREE_COMMENTS, "");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_answers_comments(&reviews, &dispatched, &batches, RESPOND_AND_FIND_NOTHING),
        &gh,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_written(&batches).await;

    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the batch was answered and never settled",
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert_eq!(
        sets(&view).len(),
        1,
        "the only Set on the Timeline is the proposal that ended the grilling",
    );
    assert!(
        view.blocked_on.is_none(),
        "nothing is waiting on the human, which is the whole of finding nothing",
    );
    assert!(
        outputs(&view)
            .last()
            .is_some_and(|output| output.latest.contains("none of it needs a change")),
        "and the Timeline says what the session made of what was said: {:?}",
        outputs(&view).last(),
    );
    assert_eq!(fixes(&view), 0, "with nothing changed about the branch");
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped: {:?}",
        notices(&view),
    );
}

/// A comment left while the review runs is answered, on a pull request nobody
/// had written on when the wrap-up started.
///
/// The wrap-up starts its review and its watchers together, so there is a poll
/// before the review has the Worktree — and what it reads is a pull request with
/// nothing on it. Settling on that reading is what this is about: the comment
/// that lands a moment later puts nothing back to waiting, because nothing was
/// waiting to be put back, and the wrap-up reaches Done the moment the review
/// ends. The watcher's next poll then finds a Conversation that is not wrapping
/// up any more and stops, and what the human wrote is never answered at all.
///
/// So the settling waits for the review exactly as the dispatching does, and
/// what this asks is the whole of that rule: the Conversation cannot be Done
/// with a comment nobody was sent to deal with.
///
/// The window is one poll wide and nothing can land in it on purpose, so this
/// holds it open — [`Pace::reviewing`] is a span a server keeps at zero and this
/// is the test it exists for.
#[tokio::test]
async fn a_comment_left_while_the_review_runs_is_answered_rather_than_settled_over() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let batches = spill.path().join("batch-prompts");

    // Long enough that the comments watcher polls a pull request nobody has
    // written on several times over before anything has looked at it.
    let dawdling = Pace {
        reviewing: paced(Duration::from_millis(600)),
        ..*BRISKLY
    };

    // Green, unlike the other batch tests here, because what this is about is a
    // wrap-up that can reach Done: a suite nobody can ask about would hold it in
    // Wrapping whatever the comments did.
    let gh = gh_about_once(GREEN, &reviews, THREE_COMMENTS, "");

    let fixture = grilling_at_pace(
        spill,
        &a_backlog_then_answers_comments(&reviews, &dispatched, &batches, RESPOND_AND_FIND_NOTHING),
        &gh,
        dawdling,
        &[],
    )
    .await;

    worked_to_empty(&fixture).await;

    // Nothing is settled while the review is still waiting for the Worktree,
    // whatever the pull request looks like from outside.
    assert!(
        !comments_settled(&fixture).await,
        "a pull request nothing has looked at yet settles nothing",
    );

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let told = std::fs::read_to_string(&batches)
        .expect("a batch session was dispatched before the wrap-up finished");

    assert!(
        told.contains("Rename the window field."),
        "and it was sent what was written while the review ran: {told}",
    );
}

/// A wrap-up waits for the run the review's own fix pushed, rather than reaching
/// Done on the green it read before it.
///
/// The checks are settled by a poll and the wrap-up is finished by a loop of its
/// own, so what stands between them is which of the two looks first. This holds
/// the ordering that makes it safe: the push lands while the review still has the
/// Worktree, and finishing needs a poll after that in any case, so the watcher
/// always gets its look in — and a change to either cadence that broke it would
/// break this.
#[tokio::test]
async fn a_wrap_up_waits_for_the_run_the_review_pushed() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let landed = spill.path().join("landed");

    let review = format!(
        "{REVIEW_THEN_FIX}\n    printf 'x' > {landed}",
        landed = quoted(&landed),
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, &review),
        &gh_about(&green_until(&landed), "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !landed.exists() {
        assert!(Instant::now() < deadline, "the review never landed its fix");
        pause(Duration::from_millis(25)).await;
    }

    // Long enough for many polls of a wrap-up with everything else settled.
    pause(Duration::from_millis(1500)).await;

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "the wrap-up waits for the run the fix started",
    );
    assert!(
        !checks_settled(&fixture).await,
        "and nothing settled the suite the push replaced",
    );
}

/// The same for the push a batch session makes, which is the other way a commit
/// lands during a wrap-up.
///
/// Its ordering is looser than the review's — what was said settles on a poll of
/// its own after the session ends, so there is a whole interval in there — and it
/// is held here for the same reason: nothing in the code says so, and a cadence
/// that changed would take it away quietly.
#[tokio::test]
async fn a_wrap_up_waits_for_the_run_a_batch_session_pushed() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let batches = spill.path().join("batch-prompts");
    let landed = spill.path().join("landed");

    let responding = format!(
        "    printf 'a fix\\n' >> fixes.md\n    \
         git add -A\n    \
         git commit --quiet -m 'fix: what was asked'\n    \
         printf 'x' > {landed}\n    \
         printf 'did what was accepted\\n'",
        landed = quoted(&landed),
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_answers_comments(&reviews, &dispatched, &batches, &responding),
        &gh_about_once(&green_until(&landed), &reviews, THREE_COMMENTS, ""),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_written(&batches).await;

    let deadline = Instant::now() + *PATIENCE;
    while !landed.exists() {
        assert!(Instant::now() < deadline, "the batch never landed its fix");
        pause(Duration::from_millis(25)).await;
    }

    // Long enough for many polls of a wrap-up with what was said settled.
    pause(Duration::from_millis(1500)).await;

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "the wrap-up waits for the run the batch's fix started",
    );
    assert!(
        !checks_settled(&fixture).await,
        "and nothing settled the suite the push replaced",
    );
}

/// A green rollup about a commit that is not the one origin is holding settles
/// nothing: it is the suite of the commit before the push rather than of the
/// work.
///
/// GitHub answers a pull request as its own record stands, and that record runs
/// behind the branch for a while after a push. So *green* on its own is not a
/// fact about the branch — it is a fact about whichever commit GitHub named
/// beside it, and a wrap-up that took the one for the other would reach Done
/// over work nothing has ever checked. This was watched happening on Verkstead's
/// own pull request on 2026-08-29, three commits behind and green.
#[tokio::test]
async fn a_rollup_about_a_commit_that_is_not_what_was_pushed_settles_nothing() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let head = spill.path().join("reported-head");

    let fixture = grilling_pushing(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(&green_for(&head), "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    let worktree = PathBuf::from(
        fixture
            .view()
            .await
            .worktree
            .expect("the work has a worktree")
            .path,
    );

    // The branch as origin holds it — and then GitHub asked about, and answering
    // for the commit before it. Nothing is said about the checks at all until
    // this is written, so there is no window in which the wrap-up could have
    // settled on something else.
    git(&worktree, &["push", "--quiet", "origin", "HEAD"]);
    std::fs::write(&head, git(&worktree, &["rev-parse", "HEAD~1"])).unwrap();

    // Long enough for many polls of a pull request answering green every time.
    pause(Duration::from_millis(1500)).await;

    assert!(
        !checks_settled(&fixture).await,
        "a suite belonging to another commit is not this branch's suite",
    );
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "so the wrap-up waits for the run for what was pushed",
    );

    // And the moment GitHub catches up, it is the same green suite and it counts.
    std::fs::write(&head, git(&worktree, &["rev-parse", "HEAD"])).unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;
}

/// And a pull request that had a suite and now reports none has a run that has
/// not been created, rather than no CI.
///
/// The gap the head above cannot catch: GitHub takes a commit before it makes
/// the runs for it, and in between it names the new commit and reports nothing
/// against it. *Nothing is running against this* is read as green on purpose —
/// a repository with no CI is nothing for a wrap-up to wait on — so the only
/// thing that tells the two apart is that this one was reporting a suite a
/// moment ago.
#[tokio::test]
async fn checks_that_have_gone_from_a_pull_request_that_had_them_settle_nothing() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let landed = spill.path().join("landed");

    let review = format!(
        "{REVIEW_THEN_FIX}\n    printf 'x' > {landed}",
        landed = quoted(&landed),
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, &review),
        &gh_about(&green_until_nothing_is_reported(&landed), "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    // Which is what the review session was waiting on: it lands the fix, pushes
    // it, and the pull request goes back to reporting nothing.
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !landed.exists() {
        assert!(Instant::now() < deadline, "the review never landed its fix");
        pause(Duration::from_millis(25)).await;
    }

    // Long enough for many polls of a pull request reporting nothing.
    pause(Duration::from_millis(1500)).await;

    assert!(
        !checks_settled(&fixture).await,
        "a pull request that had a suite and now has none has a run still coming",
    );
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "so the wrap-up waits for it rather than finishing over the fix",
    );
}

/// A pull request GitHub cannot merge keeps its Conversation in Wrapping,
/// however green the suite on it is — and the wrap-up finishes the moment the
/// conflict is gone.
///
/// A base that moves under a branch puts it in conflict without anybody touching
/// it, and nothing lands after that. A Conversation carried to Done over one
/// would be work Verkstead had called finished that the human could not merge,
/// so being conflict-free is one of the things a wrap-up waits on — read off the
/// same poll the checks are, GitHub saying both in one answer.
///
/// And it is not **Waiting on checks**. That is the wrap-up down to the one
/// thing nothing here can hurry, and a conflict is not that: what this needs is
/// a resolution rather than a suite, and a card sending the human off to watch
/// GitHub finish would send them to the wrong place.
/// And what resolves it is a session of Verkstead's own: one look that reads
/// CONFLICTING dispatches a fix session told to merge the base branch in.
#[tokio::test]
async fn a_pull_request_that_conflicts_keeps_the_wrap_up_out_of_done() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let released = spill.path().join("released");
    let resolved = spill.path().join("conflict-resolved");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_resolves(&dispatched, &released, &resolved),
        &gh_about(&green_but_conflicting_until(&resolved), "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    // The resolution session in the Worktree with its go spent and nothing
    // committed yet, which is the conflicted wrap-up standing still.
    let told = until_written_by(&dispatched, 1).await;

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "nothing can land a conflicted pull request, so the work is not finished with",
    );
    assert!(
        checks_settled(&fixture).await,
        "the suite itself is green and settled — the two are different facts about \
         the same branch, and a conflict does not make a green suite red",
    );
    assert!(
        !merge_settled(&fixture).await,
        "and the conflict is what the wrap-up is waiting on",
    );
    assert!(
        !view.waiting_on_checks,
        "which is not waiting on checks: the checks came in",
    );
    assert!(
        waiting_on_checks(&view).is_empty(),
        "and nothing said it was on the Timeline either: {:?}",
        waiting_on_checks(&view),
    );
    assert!(
        notices(&view).is_empty(),
        "nothing stopped over it: the pull request has a go left, and this is it: {:?}",
        notices(&view),
    );
    assert_eq!(
        conflict_attempts_spent(&fixture).await,
        1,
        "the go was counted as the session was dispatched, so a restart does not \
         spend it again",
    );

    // What that session was told: which pull request will not merge, where its
    // branch is checked out, and to merge the base in rather than rebase onto it.
    let worktree = view.worktree.clone().expect("the work is checked out").path;
    let prompt = prompts(&told)[0];

    assert!(
        prompt.contains("addressing/SKILL.md"),
        "the session is put inside the bundled skill, as a check's fix is: {prompt}",
    );
    assert!(
        prompt.contains("model=claude-implementation-5"),
        "under the implementation Profile, as every session that writes code is: \
         {prompt}",
    );
    assert!(
        prompt.contains("#41") && prompt.contains("verkstead"),
        "and told which pull request in which repository: {prompt}",
    );
    assert!(
        prompt.contains(&worktree),
        "and the worktree to do the merge in, {worktree} being where that branch \
         is: {prompt}",
    );
    assert!(
        prompt.contains("Merge the pull request's base branch"),
        "and what to do about it: {prompt}",
    );
    assert!(
        prompt.contains("rather than a rebase") && prompt.contains("force-push"),
        "a merge rather than a rebase, nothing here force-pushing: {prompt}",
    );

    // The session lets go, which commits the merge and puts the pull request in
    // front of GitHub again — and this time GitHub says it merges.
    std::fs::write(&released, "x").unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert!(
        merge_settled(&fixture).await,
        "a pull request GitHub says it can merge settles the last of it",
    );
    let told = std::fs::read_to_string(&dispatched).unwrap();

    assert_eq!(
        prompts(&told).len(),
        1,
        "and one session was the whole of it: the conflict is gone, so nothing \
         further was dispatched: {told}",
    );
}

/// Two resolution sessions and then the human, exactly as a red check goes.
///
/// The base stays moved under the branch however many times it is merged, so
/// nothing the machine does lands the pull request. After its two goes Verkstead
/// stops asking: the run stops, the Notice names the pull request that would not
/// merge clean and carries the tail of what the last session said, and nothing
/// further is dispatched.
///
/// And then Resume forgets the count, which is what a press is for: the human
/// has read the Notice and asked for another go, and one that stopped again on
/// its next poll without dispatching anything would be no go at all.
#[tokio::test]
async fn a_conflict_two_sessions_could_not_resolve_halts_and_tells_the_human() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(GREEN_BUT_CONFLICTING, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("conflict"),
        "the step is named as what it was: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("#41") && stopped.html.contains("verkstead"),
        "and the reason names the pull request that would not merge clean: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("having a go at it"),
        "with the tail of what the last session said: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "every resolution session the branch was allowed has been spent, so a \
         restart that started the merging over would spend them all again",
    );
    assert!(
        !merge_settled(&fixture).await,
        "and a pull request that will not merge settles nothing",
    );
    assert_eq!(
        conflict_attempts_spent(&fixture).await,
        2,
        "two goes at it and no more",
    );

    // Long enough for many more polls, had anything still been dispatching.
    pause(Duration::from_millis(500)).await;

    let told = std::fs::read_to_string(&dispatched).unwrap();

    assert_eq!(
        prompts(&told).len(),
        2,
        "the run does not go round again once it has stopped: {told}",
    );

    // The stop ended the session it was written over, so the press finds a
    // Worktree with nothing in it.
    fixture
        .until(|view| {
            outputs(view)
                .iter()
                .all(|session| !session.running)
                .then_some(())
        })
        .await;

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    // A third session, which is only possible on a count that was forgotten.
    until_written_by(&dispatched, 3).await;

    assert_eq!(
        conflict_attempts_spent(&fixture).await,
        1,
        "the count started again from nothing, and that third session is the \
         first of the new two",
    );
}

/// A conflict GitHub has not worked out yet is nothing to conclude and nothing
/// to dispatch at.
///
/// *UNKNOWN* is what GitHub says for a while after every push, and reading it as
/// a conflict would spend a pull request's goes on a merge nobody has said is
/// needed — twice, and then stop a run over a question that was never answered.
#[tokio::test]
async fn a_merge_github_has_not_worked_out_dispatches_nothing() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(GREEN_BUT_UNKNOWN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    // Long enough for many polls of a pull request GitHub will not commit itself
    // about.
    pause(Duration::from_millis(1500)).await;

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "not knowing is not knowing: it settles nothing, so the wrap-up waits",
    );
    assert!(
        !merge_settled(&fixture).await,
        "and it settles nothing either way",
    );
    assert!(
        !dispatched.exists(),
        "nothing was dispatched at a conflict nobody has said is there: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert_eq!(
        conflict_attempts_spent(&fixture).await,
        0,
        "so no go was spent on it",
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped: {:?}",
        notices(&view),
    );
}

/// A pull request that starts conflicting after the work on it is Done is
/// noticed all the same — and nothing at all is done about it.
///
/// A wrap-up's watchers stop the moment the Conversation reaches Done, and the
/// pull request goes on sitting there waiting for the human to merge it. Bases
/// move: a branch that merged cleanly at Done conflicts the next morning without
/// anybody having touched it, and there would be nothing left asking.
///
/// So a sweep of its own asks, every fifteen minutes on a server and every
/// hundred milliseconds here. What it does with the answer is write it down and
/// nothing else: no session, no stop, no Notice, and the Conversation stays
/// exactly where it was. After Done, what to do about a conflict is the human's.
#[tokio::test]
async fn a_conflict_that_arrives_after_done_is_written_down_and_nothing_else() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let asked = spill.path().join("gh-sweeps");
    let landing = spill.path().join("landing");

    let fixture = grilling_landing(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_landing(&asked, &landing),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    // The sweep has it in hand: the pull request is open, which is the reading
    // nothing but this sweep asks for — the wrap-up's own watcher never asks
    // where a pull request has got to.
    until_standing(&fixture, verkstead_server::store::Standing::Open).await;

    assert_eq!(
        recorded_merging(&fixture).await,
        Some(verkstead_server::store::Merging::Cleanly),
        "it merged cleanly the whole way to Done",
    );

    // And then somebody merges something else, and the base moves out from under
    // a branch nobody is working on any more.
    std::fs::write(&landing, r#"{"mergeable":"CONFLICTING","state":"OPEN"}"#).unwrap();

    until_merging(&fixture, verkstead_server::store::Merging::Conflicting).await;

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Done,
        "the work is still finished with: a conflict after Done moves nothing",
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing was said about it on the Timeline: {:?}",
        notices(&view),
    );
    assert!(
        !dispatched.exists(),
        "and nothing was sent at it: after Done, what to do about a conflict is \
         the human's: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert_eq!(
        conflict_attempts_spent(&fixture).await,
        0,
        "so no go was spent on it either",
    );
}

/// And the press that does something about one: **Resolve conflicts** on a Done
/// pull request's details pane, which sends the Conversation back to its wrap-up
/// with the review left settled.
///
/// The whole round trip, because the whole round trip is the point. A branch
/// that merged cleanly all the way to Done starts conflicting, the sweep writes
/// that down and dispatches nothing, the human presses the button — and what
/// happens next is the wrap-up Verkstead already knows how to run: a resolution
/// session sent at the pull request with fresh goes, and Done again once the
/// merge lands and GitHub says it can merge.
///
/// **And no review session anywhere in it.** That is what makes this a press of
/// its own rather than a steer into Wrapping: the work was reviewed, the human
/// read the review, and a base that moved under the branch since is not a reason
/// to read it a second time. The review's settle stands, so the wrap-up finds it
/// settled and runs nothing for it.
///
/// **The checkout has gone by the time they press**, which is the state a Done
/// Conversation is likeliest of any to be in: nothing has worked in it since the
/// work was finished with, and that may be weeks. So the press makes it again
/// from the branch before it moves anything, the way Resume makes one — and the
/// resolution session below is a session that could only have run in a checkout
/// that was there.
#[tokio::test]
async fn pressing_resolve_on_a_conflicted_done_pull_request_gets_it_resolved() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let conflicting = spill.path().join("conflicting");
    let resolved = spill.path().join("conflict-resolved");

    let fixture = grilling_landing(
        spill,
        &a_backlog_then_resolves_at_once(&reviews, &dispatched, &resolved),
        &gh_conflicting_between(&conflicting, &resolved),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let read_once = prompts(&std::fs::read_to_string(&reviews).unwrap()).len();
    assert_eq!(read_once, 1, "the branch was read once on the way to Done");

    // And then somebody merges something else, and the base moves out from under
    // a branch nobody is working on any more. The sweep after Done writes that
    // down and does nothing about it.
    std::fs::write(&conflicting, "x").unwrap();

    until_merging(&fixture, verkstead_server::store::Merging::Conflicting).await;

    assert!(
        !dispatched.exists(),
        "nothing is dispatched after Done: the conflict is the human's to decide \
         about",
    );

    // And in the weeks nothing was working in it, the checkout went. A worktree
    // is derived state and the branch holds everything that was committed, so
    // this is a directory to make again rather than work that is lost — but a
    // session dispatched at the path before it is made again is a session that
    // cannot start.
    let worktree = PathBuf::from(
        fixture
            .view()
            .await
            .worktree
            .expect("the work is checked out")
            .path,
    );
    std::fs::remove_dir_all(&worktree).unwrap();

    // Which they do, on the pull request's own details pane.
    assert_eq!(fixture.resolve_conflicts().await, Resolved::Resolving);

    assert!(
        worktree.join(".git").exists(),
        "the press made the checkout again from the branch before it moved \
         anything, so there is somewhere for the resolution session to work",
    );

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "the press moves it back into the wrap-up itself, rather than leaving \
         something to notice that it should",
    );

    // The resolution session, dispatched by the wrap-up's own watcher against
    // the record's own reading of the conflict.
    let told = until_written_by(&dispatched, 1).await;
    let prompt = prompts(&told)[0];

    assert!(
        prompt.contains("addressing/SKILL.md") && prompt.contains("#41"),
        "sent at the pull request that will not merge: {prompt}",
    );
    assert!(
        prompt.contains("Merge the pull request's base branch"),
        "and told what to do about it, by the strategy this repository resolves \
         conflicts by: {prompt}",
    );

    // And then the merge lands, GitHub says the branch merges again, and the
    // ordinary settling rule carries the work back to Done.
    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert!(
        merge_settled(&fixture).await,
        "a pull request GitHub says it can merge settles the last of it",
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped anywhere along the way: {:?}",
        notices(&view),
    );
    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        read_once,
        "and the branch was never read again: the review it was carried to Done \
         on is the review of this work, and the press left its settle standing",
    );

    // Two moves and the human's own line between them, which is what a Timeline
    // long enough to have forgotten this has to be readable back for — and the
    // line is the press's own rather than a steer's, because the two are
    // different acts. A steer into Wrapping would have read the branch again.
    assert_eq!(
        view.timeline
            .iter()
            .filter_map(|event| match event {
                verkstead_render::TimelineEvent::Moved(moved) =>
                    Some(format!("moved to {:?}", moved.state)),
                verkstead_render::TimelineEvent::Steer(steer) =>
                    Some(format!("steered into {:?}", steer.target)),
                verkstead_render::TimelineEvent::ResolveConflicts(_) =>
                    Some("asked for the conflict to be resolved".to_owned()),
                _ => None,
            })
            .skip_while(|event| event.as_str() != "moved to Done")
            .collect::<Vec<_>>(),
        [
            "moved to Done",
            "asked for the conflict to be resolved",
            "moved to Wrapping",
            "moved to Done",
        ]
        .map(str::to_owned),
        "somebody decided this, the machine's move says what came of it, and no \
         steer was written for a press that reads no branch",
    );
}

/// A pull request somebody has merged is never asked about again, and that is
/// where the sweep ends.
///
/// Merged and closed are both endings — nothing about either moves again — so
/// asking a second time would be a `gh` call every fifteen minutes for the life
/// of the server about a pull request that has stopped existing as a question.
/// It is learned from the same answer the conflict is watched for, which is what
/// makes the ending free.
///
/// And it ends per pull request rather than all at once: what stops the asking is
/// the reading written down about *this* one.
#[tokio::test]
async fn a_pull_request_that_has_been_merged_is_never_asked_about_again() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let asked = spill.path().join("gh-sweeps");
    let landing = spill.path().join("landing");

    let fixture = grilling_landing(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_landing(&asked, &landing),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    until_swept(&asked).await;

    // The human merges it, which is the act this whole pipeline is built around
    // and the one Verkstead never makes itself.
    std::fs::write(&landing, r#"{"mergeable":"MERGEABLE","state":"MERGED"}"#).unwrap();

    until_standing(&fixture, verkstead_server::store::Standing::Merged).await;

    // GitHub would say something else now, and nothing is ever going to hear it.
    std::fs::write(&landing, r#"{"mergeable":"CONFLICTING","state":"OPEN"}"#).unwrap();

    let stopped_at = swept(&asked);

    // Long enough for many sweeps of a pull request there is nothing left to ask
    // about.
    pause(Duration::from_millis(1500)).await;

    assert_eq!(
        swept(&asked),
        stopped_at,
        "the sweep stopped asking the moment the merge was recorded",
    );
    assert_eq!(
        recorded_merging(&fixture).await,
        Some(verkstead_server::store::Merging::Cleanly),
        "so what stands about it is the last thing anybody asked, from before it \
         was merged",
    );
}

/// And a Closed Conversation is never asked about at all.
///
/// Closing is the human finished with the work, whatever state it had got to —
/// so a pull request left open on one is theirs to leave open, and a sweep going
/// on asking about it would be Verkstead watching something nobody is waiting
/// for. Which takes an Archived Conversation with it: archiving is a Closed
/// Conversation off the sidebar rather than a state of its own.
#[tokio::test]
async fn a_closed_conversation_is_never_asked_about() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let asked = spill.path().join("gh-sweeps");
    let landing = spill.path().join("landing");

    let fixture = grilling_landing(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_landing(&asked, &landing),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    until_swept(&asked).await;

    assert_eq!(fixture.close().await, ConversationClosed::Closed);

    // The base moves under it afterwards, and nothing is listening.
    std::fs::write(&landing, r#"{"mergeable":"CONFLICTING","state":"OPEN"}"#).unwrap();

    let stopped_at = swept(&asked);

    // Long enough for many sweeps of a Conversation nobody is waiting on.
    pause(Duration::from_millis(1500)).await;

    assert_eq!(
        swept(&asked),
        stopped_at,
        "closing it stopped the asking, whatever GitHub had left to say",
    );
    assert_eq!(
        recorded_merging(&fixture).await,
        Some(verkstead_server::store::Merging::Cleanly),
        "so the conflict that arrived after the human was finished is not written \
         down anywhere",
    );
}

/// The Cleanup trims what has been archived for longer than its days, and
/// leaves everything else exactly where it was.
///
/// Five Conversations in one pass, because what the sweep *is* is a rule about
/// which ones: archived four days ago, archived a moment ago, closed but never
/// archived, trimmed already since it was last archived, and archived a second
/// time after a trim a life ago. Two of them have their bulk taken and three of
/// them are untouched.
///
/// The bulk is written through the store rather than by running sessions. What
/// a session puts on a Timeline is this suite's subject everywhere else and is
/// not in question here — what is, is which archivings the sweep reaches, and
/// five real runs would be five minutes of waiting to ask it.
#[tokio::test]
async fn the_cleanup_trims_what_was_archived_long_enough_ago() {
    let bench = bench_at_pace(
        tempfile::tempdir().unwrap(),
        PRINTS_AND_STOPS,
        PULL_REQUEST,
        *CLEANING,
        None,
    )
    .await;

    // Listening before there is anything to hear, because a Nudge is sent
    // rather than stored: a page that subscribed after the sweep ran would be
    // the one thing this cannot tell from a sweep that announced nothing.
    let mut page = Listening::open(&bench.app).await;

    let pool = open_database(&bench.database).await.unwrap();
    let repo = bench.repo_id;

    // Archived four days ago, which is past the three the sweep keeps.
    let old = archived_printing(&pool, repo, "rate-limiting").await;
    aged(&pool, "archived_conversations", "archived_at", old.id, 4).await;

    // And archived just now, which is not.
    let fresh = archived_printing(&pool, repo, "usage-limits").await;

    // And closed and left on the sidebar, which has no clock running on it at
    // all.
    let living = archived_printing(&pool, repo, "burst-allowance").await;
    verkstead_store::unarchive_conversation(&pool, living.id)
        .await
        .unwrap();

    // And one trimmed since it was last archived, with a second session's output
    // written after the trim: the mark is what says there is nothing to do, so a
    // sweep that read it wrong would take this too.
    let done = archived_printing(&pool, repo, "window-rollover").await;
    aged(&pool, "archived_conversations", "archived_at", done.id, 4).await;
    verkstead_store::trim_conversation(&pool, done.id)
        .await
        .unwrap();
    let since = printing(&pool, done.id, "window-rollover-again").await;

    // And one steered back to life and put away again, whose trim mark is older
    // than the archiving it is under now — so its new bulk is taken as well as
    // its old.
    let again = archived_printing(&pool, repo, "counter-reset").await;
    aged(&pool, "archived_conversations", "archived_at", again.id, 10).await;
    verkstead_store::trim_conversation(&pool, again.id)
        .await
        .unwrap();
    aged(&pool, "trimmed_conversations", "trimmed_at", again.id, 9).await;
    verkstead_store::unarchive_conversation(&pool, again.id)
        .await
        .unwrap();
    let relived = printing(&pool, again.id, "counter-reset-again").await;
    verkstead_store::archive_conversation(&pool, again.id)
        .await
        .unwrap();
    aged(&pool, "archived_conversations", "archived_at", again.id, 4).await;

    let timeline = verkstead_store::timeline(&pool, old.id).await.unwrap();

    until_trimmed(&pool, old.id, old.event).await;
    until_trimmed(&pool, again.id, relived).await;

    assert_eq!(
        verkstead_store::timeline(&pool, old.id).await.unwrap(),
        timeline,
        "and nothing was written on the Timeline to say so: a cleanup puts \
         nothing in front of the human",
    );

    // What it does say is that this Conversation moved, which is how a page
    // open on it learns to draw the word and to stop drawing the drill-downs
    // that have gone. Told rather than shown: the viewer does not poll.
    until_nudged(
        &mut page,
        Nudge::Conversation {
            conversation: old.id,
        },
    )
    .await;

    // Long enough for many more passes, so that what the others still hold is
    // held against a sweep that has had every chance at them.
    tokio::time::sleep(CLEANING.cleanup * 8).await;

    assert!(
        !held(&pool, fresh.id, fresh.event).await.is_empty(),
        "one archived a moment ago is not old enough to have anything taken",
    );
    assert!(
        !held(&pool, living.id, living.event).await.is_empty(),
        "and one back on the sidebar has no clock running on it at all",
    );
    assert!(
        !held(&pool, done.id, since).await.is_empty(),
        "and one trimmed since it was last archived is left alone, whatever has \
         been printed on it since",
    );
    assert!(
        held(&pool, old.id, old.event).await.is_empty(),
        "while the one archived four days ago has had its output taken",
    );
    assert!(
        held(&pool, again.id, relived).await.is_empty(),
        "and so has the one archived again, its last trim being older than the \
         archiving it is under now",
    );

    pool.close().await;
}

/// And it goes by what the settings say at the moment of the pass: the switch
/// off is a sweep that takes nothing, and the days are what it counts as old.
///
/// One Conversation and one running server, told three different things in
/// turn. Nothing is restarted between them, which is the point: the settings are
/// read off the file on every pass, so a switch flipped from a phone is in force
/// on the next one.
#[tokio::test]
async fn the_cleanup_goes_by_what_the_settings_say_at_the_time() {
    let bench = bench_at_pace(
        tempfile::tempdir().unwrap(),
        PRINTS_AND_STOPS,
        PULL_REQUEST,
        *CLEANING,
        None,
    )
    .await;

    // Switched off before there is anything to take, so that what the first
    // stretch asserts is a sweep that had every chance and declined.
    cleaning_up(&bench, "cleanup:\n  trim:\n    enabled: false\n");

    let pool = open_database(&bench.database).await.unwrap();

    // Archived four days ago, which is past the three a Verkstead nobody has
    // configured keeps.
    let old = archived_printing(&pool, bench.repo_id, "rate-limiting").await;
    aged(&pool, "archived_conversations", "archived_at", old.id, 4).await;

    // Long enough for many passes, so that what it still holds is held against a
    // sweep that has had every chance at it.
    tokio::time::sleep(CLEANING.cleanup * 8).await;

    assert!(
        !held(&pool, old.id, old.event).await.is_empty(),
        "the trim is switched off, so the sweep takes nothing at all",
    );

    // Back on, and told to wait ten days — which this one, at four, has not.
    cleaning_up(
        &bench,
        "cleanup:\n  trim:\n    enabled: true\n    days: 10\n",
    );

    tokio::time::sleep(CLEANING.cleanup * 8).await;

    assert!(
        !held(&pool, old.id, old.event).await.is_empty(),
        "four days is not the ten the settings now say to wait",
    );

    // And down to two, which it is past. No restart anywhere in any of this.
    cleaning_up(
        &bench,
        "cleanup:\n  trim:\n    enabled: true\n    days: 2\n",
    );

    until_trimmed(&pool, old.id, old.event).await;

    pool.close().await;
}

/// And the Cleanup deletes what has been archived for longer than the delete's
/// days — but only once the human has said so.
///
/// Two stretches on one running server, because the switch is the whole subject.
/// In the first nothing is deleted however old it is, and the trim goes on
/// taking bulk beside it: off is a Verkstead that forgets nothing. In the second
/// the switch is on, and what was already past the threshold goes on the next
/// pass — the backlog reading the trim's own rule follows.
///
/// Three Conversations, the other two being what *only the old one* means: one
/// archived a moment ago, and one the human has taken back off the archive.
#[tokio::test]
async fn the_cleanup_deletes_what_was_archived_long_enough_ago() {
    let bench = bench_at_pace(
        tempfile::tempdir().unwrap(),
        PRINTS_AND_STOPS,
        PULL_REQUEST,
        *CLEANING,
        None,
    )
    .await;

    // The branch the work is on, made in the repository itself. A delete is the
    // store's and nothing else's: the branch outlives the record of it, closing
    // having already chosen to keep it.
    git(&bench.repo, &["branch", "rate-limiting"]);

    // And a page open on the sidebar throughout — see the trim's own test for
    // why it subscribes before there is anything to hear.
    let mut page = Listening::open(&bench.app).await;

    let pool = open_database(&bench.database).await.unwrap();
    let repo = bench.repo_id;

    // Archived a month ago, which is past the thirty a delete keeps.
    let old = archived_printing(&pool, repo, "rate-limiting").await;
    aged(&pool, "archived_conversations", "archived_at", old.id, 31).await;

    // And archived just now, which is not.
    let fresh = archived_printing(&pool, repo, "usage-limits").await;

    // And one back on the sidebar, which has no clock running on it at all.
    let living = archived_printing(&pool, repo, "burst-allowance").await;
    aged(
        &pool,
        "archived_conversations",
        "archived_at",
        living.id,
        31,
    )
    .await;
    verkstead_store::unarchive_conversation(&pool, living.id)
        .await
        .unwrap();

    // The trim reaching it is what says the sweep is running and has had this
    // one in its hands.
    until_trimmed(&pool, old.id, old.event).await;

    // Long enough for many more passes at a Conversation a Verkstead nobody has
    // configured will not delete.
    tokio::time::sleep(CLEANING.cleanup * 8).await;

    assert!(
        still_there(&pool, old.id).await,
        "the delete is switched off, so a month-old archive is trimmed and kept",
    );

    // Turned on, and left to say nothing about the days: thirty is what a
    // Verkstead told only that it should delete goes by.
    cleaning_up(&bench, "cleanup:\n  delete:\n    enabled: true\n");

    until_deleted(&pool, old.id).await;

    // And the list itself is announced as having moved, which is what makes the
    // sidebar's own promise true: gone even under Show archived, rather than a
    // row still drawn for something that answers not-found.
    until_nudged(&mut page, Nudge::Conversations).await;

    // Long enough for many more passes, so that what is still there is held
    // against a sweep that has had every chance at it.
    tokio::time::sleep(CLEANING.cleanup * 8).await;

    assert!(
        still_there(&pool, fresh.id).await,
        "one archived a moment ago is not old enough to go",
    );
    assert!(
        still_there(&pool, living.id).await,
        "and one back on the sidebar has no clock running on it at all",
    );

    let (status, body) = fetch(
        &bench.app,
        Request::builder()
            .uri(format!("/api/ui/conversations/{}", old.id))
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "opening what was deleted says there is no such Conversation: {body}",
    );

    assert!(
        git(&bench.repo, &["branch", "--list", "rate-limiting"]).contains("rate-limiting"),
        "and the branch is where it was: no git operation belongs on this path",
    );

    pool.close().await;
}

/// Whether the store still has a Conversation of that id at all.
async fn still_there(pool: &SqlitePool, id: i64) -> bool {
    verkstead_store::load_conversation(pool, id)
        .await
        .unwrap()
        .is_some()
}

/// Wait for the sweep to delete one Conversation, which is the store no longer
/// having it.
async fn until_deleted(pool: &SqlitePool, id: i64) {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        if !still_there(pool, id).await {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "the cleanup never deleted Conversation {id}",
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// Say what the Cleanup is to do, in the Data Directory the running server reads
/// its settings out of.
///
/// Written over the author every sandbox is configured out of rather than beside
/// it, the way [`configure`] does, and through a rename rather than in place: a
/// sweep reading the file half-written would read a Verkstead that had been told
/// nothing, which is precisely the answer these tests are telling it apart from.
fn cleaning_up(bench: &Bench, said: &str) {
    let path = bench.state.path().join("config.yaml");
    let writing = path.with_extension("yaml.writing");

    std::fs::write(&writing, format!("{THE_AUTHOR}{said}")).unwrap();
    std::fs::rename(&writing, &path).unwrap();
}

/// A stub that prints once and stops, for a fixture whose sessions never start:
/// [`bench_at_pace`] wants an agent, and nothing here launches one.
const PRINTS_AND_STOPS: &str = r#"printf 'nothing to do\n'"#;

/// One archived Conversation with a session's worth of bulk on it.
async fn archived_printing(pool: &SqlitePool, repo: i64, branch: &str) -> Archived {
    let id = verkstead_store::start_conversation(pool, repo, branch)
        .await
        .unwrap()
        .expect("the Repo is registered");

    verkstead_store::save_brief(pool, id, "# Rate limiting\n")
        .await
        .unwrap();

    let event = printing(pool, id, branch).await;

    verkstead_store::close_conversation(pool, id).await.unwrap();
    verkstead_store::archive_conversation(pool, id)
        .await
        .unwrap();

    Archived { id, event }
}

/// A Conversation the sweep has an opinion about, and the Event its bulk hangs
/// off.
struct Archived {
    id: i64,
    event: i64,
}

/// One session's worth of what a trim takes: a Capture with something in it, and
/// the log the session kept of itself.
async fn printing(pool: &SqlitePool, id: i64, session: &str) -> i64 {
    let event = verkstead_store::start_capture(pool, id, Some(session), None)
        .await
        .unwrap();

    verkstead_store::append_capture(
        pool,
        event,
        "the session said a great deal\n",
        &verkstead_store::Summary {
            lines: 1,
            turns: Some(2),
            latest: "the session said a great deal".to_owned(),
        },
    )
    .await
    .unwrap();

    verkstead_store::append_transcript(pool, event, &[r#"{"type":"assistant"}"#.to_owned()])
        .await
        .unwrap();

    event
}

/// Put a stamp back in time, which is the only way a test gets to be days old.
///
/// Written straight into the row, because there is nothing that would do it
/// otherwise: an archiving is stamped `now` and the clock the sweep reads is
/// counted in days.
async fn aged(pool: &SqlitePool, table: &str, column: &str, id: i64, days: u32) {
    sqlx::query(&format!(
        "UPDATE {table} SET {column} = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)
         WHERE conversation_id = ?"
    ))
    .bind(format!("-{days} days"))
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

/// What one session's Capture still holds.
async fn held(pool: &SqlitePool, id: i64, event: i64) -> String {
    verkstead_store::capture(pool, id, event)
        .await
        .unwrap()
        .expect("the Event is on that Conversation's Timeline")
}

/// Wait for one Nudge of a particular shape to reach a page that is listening.
///
/// Past whatever else the sweep announced on the way, rather than reading the
/// next one and insisting on it: two Conversations trimmed in one pass are two
/// Nudges, and which of them arrives first is nobody's promise.
async fn until_nudged(page: &mut Listening, wanted: Nudge) {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        if page.nudge().await == wanted {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "the cleanup never announced {wanted:?}",
        );
    }
}

/// Wait for the sweep to reach one Conversation, which is its Capture emptied.
async fn until_trimmed(pool: &SqlitePool, id: i64, event: i64) {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        if held(pool, id, event).await.is_empty() {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "the cleanup never trimmed Conversation {id}",
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// What a server that comes back up owes a batch's proposal nobody is behind: the
/// same thing it owes the review's.
///
/// This is the bug the addressing-as-dispatched trade opens. The comments are
/// written down as dealt with the moment a batch session is dispatched, so that a
/// restart does not dispatch about them twice — which means a batch session lost
/// to a restart leaves a record saying somebody saw to what was said and a Set
/// nobody is behind. Left alone, the watcher finds nothing new, settles the
/// comments, and the wrap-up reaches Done with the human's questions still open.
///
/// So it is asked about instead: the questions are closed, because nothing is
/// coming to read an answer to them, what was said goes back to being unread so
/// that the human's feedback outlives the session that lost it, and the run stops
/// where they can see it.
#[tokio::test]
async fn a_restart_over_a_batch_waiting_on_its_ask_stops_the_run_and_reads_what_was_said_again() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let batches = spill.path().join("batch-prompts");

    // Green all the way through, so that the only thing between this wrap-up and
    // Done is what was said — which is what makes an unattended batch a
    // Conversation that finishes with the questions still open.
    let stub = a_backlog_then_answers_comments(&reviews, &dispatched, &batches, RESPOND_THEN_WAIT);
    let gh = gh_about_once(&green_after(&batches), &reviews, THREE_COMMENTS, "");

    let fixture = grilling_spilling(spill, &stub, &gh).await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &batches).await;

    let set = fixture.ask(ANSWERING_THE_COMMENTS).await;

    // A second server over the same database, which is what a restart is: the
    // session idling on that ask does not exist as far as it is concerned.
    let _restarted = fixture.restarted(&stub, &gh).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("the pull request's comments"),
        "the stop is named as the half that failed: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("closed unanswered"),
        "and the Notice says the questions are off: {:?}",
        stopped.html,
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "the Conversation did not reach Done over a Set nobody answered",
    );
    assert!(
        !comments_settled(&fixture).await,
        "and nothing settled what was said, because nobody dealt with it",
    );

    let standing = sets(&view)
        .into_iter()
        .find(|asked| asked.set_id == set)
        .expect("the Set the gone session left open is on the Timeline")
        .standing
        .clone();

    assert!(
        matches!(standing, verkstead_render::Standing::LockedUnanswered(_)),
        "with nothing left for the human to answer into: {standing:?}",
    );
    assert!(
        addressed(&fixture).await.is_empty(),
        "and what was said is unread again, so a retry is a session about the same \
         words rather than one about nothing",
    );
}

/// Which of this Conversation's own pull request's comments Verkstead has
/// recorded as dealt with.
async fn addressed(fixture: &Grilling) -> Vec<String> {
    addressed_on(fixture, own_repo(fixture).await).await
}

/// The same for the pull request opened in `repo_id`, which is what a companion's
/// is read by.
async fn addressed_on(fixture: &Grilling, repo_id: i64) -> Vec<String> {
    let pool = open_database(&fixture.database).await.unwrap();
    let addressed = verkstead_server::store::addressed_comments(&pool, fixture.id, repo_id)
        .await
        .unwrap();
    pool.close().await;

    addressed
}

/// A batch session that ends having landed none of what the human accepted
/// leaves what was said dealt with all the same: what it did with the answers is
/// its own to report.
///
/// The review's rule one turn later, and for the review's reason. The record
/// could say otherwise — the proposals they accepted are on the Set, and there
/// is no commit on the branch since — and it is deliberately not asked. The
/// session read what was said, put what it would do, was answered and ran to the
/// end of what it had to say; a Verkstead that audited the branch against the
/// picks would be second-guessing the only participant that was there, and
/// stopping the run over a change the session decided against on second look.
#[tokio::test]
async fn a_batch_that_landed_nothing_still_leaves_what_was_said_addressed() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let batches = spill.path().join("batch-prompts");

    let gh = gh_about_once(CHECKS_UNANSWERABLE, &reviews, THREE_COMMENTS, "");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_answers_comments(&reviews, &dispatched, &batches, RESPOND_THEN_VANISH),
        &gh,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &batches).await;

    // One accepted, one declined, and the session lands neither.
    let set = fixture.ask(ANSWERING_THE_COMMENTS).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "Keep the signature." },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the batch session ended and what was said never settled",
        );
        pause(Duration::from_millis(25)).await;
    }

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped over what the record says was owed: {:?}",
        notices(&view),
    );
    assert_eq!(fixes(&view), 0, "and nothing was committed");
    assert!(
        !dispatched.exists(),
        "with nothing dispatched to land it afterwards: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert_eq!(
        prompts(&std::fs::read_to_string(&batches).unwrap()).len(),
        1,
        "and nothing read the comments a second time",
    );
}

/// But a batch session that *dies* after the answers stops the run, and
/// dispatches nothing to finish what it started.
///
/// The other half of the same rule: what the session did with the answers is its
/// report to make, and one that fell over made none. There is nothing here that
/// knows whether the changes landed, so the run stops with the tail of what it
/// said as the evidence — and what was said goes back to being unread, because
/// the press is the batch over again rather than a session handed decisions off
/// the record. What that fresh reading proposes is whatever is still worth
/// proposing about the branch as it now stands.
#[tokio::test]
async fn a_batch_that_dies_after_the_answers_stops_the_run_and_dispatches_nothing() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let batches = spill.path().join("batch-prompts");
    let mended = spill.path().join("mended");

    // Falls over on the answers — and once whatever the human went off and did
    // about it is done, reads what was said and finds nothing left in it.
    let responding = format!(
        "    if [ -e {mended} ]; then\n        \
             printf 'I read what was said and none of it needs a change\n'\n    \
         else\n{die}\n    \
         fi",
        mended = quoted(&mended),
        die = RESPOND_THEN_DIE_ON_THE_ANSWERS,
    );

    let gh = gh_about_once(CHECKS_UNANSWERABLE, &reviews, THREE_COMMENTS, "");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_answers_comments(&reviews, &dispatched, &batches, &responding),
        &gh,
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &batches).await;

    let set = fixture.ask(ANSWERING_THE_COMMENTS).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "Keep the signature." },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let stopped = fixture.stopped().await;

    assert!(
        stopped
            .html
            .contains("Answering what was said on the pull request"),
        "the step is named as what it was: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("the connection dropped"),
        "with the tail of what the session said, which is where it says why: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "what is waiting is the human",
    );
    assert!(
        !comments_settled(&fixture).await,
        "and nothing settled what was said, because nobody saw it through",
    );
    assert!(
        addressed(&fixture).await.is_empty(),
        "which is unread again, so the press is a session about the same words \
         rather than one about nothing",
    );
    assert!(
        !dispatched.exists(),
        "and nothing was dispatched to carry out what it was answered: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    assert_eq!(
        prompts(&std::fs::read_to_string(&batches).unwrap()).len(),
        1,
        "with nothing launched behind the stop either",
    );

    // And the press is the batch over again: a session as fresh as the first,
    // reading what was said rather than being handed what was decided about it.
    std::fs::write(&mended, "").unwrap();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the press never answered what was said, so it never settled",
        );
        pause(Duration::from_millis(50)).await;
    }

    assert_eq!(
        prompts(&std::fs::read_to_string(&batches).unwrap()).len(),
        2,
        "the batch that died on the answers, and the one the press ran",
    );
    assert!(
        !dispatched.exists(),
        "and still nothing dispatched from the record: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
}

/// Which comments have been dispatched for is written down rather than held in
/// the process, so a server that comes back up does not dispatch a session about
/// feedback that was addressed yesterday.
///
/// The checks cannot be asked about here, which is what keeps the Conversation
/// wrapping up long enough for a second server to take it over: a wrap-up that
/// had finished would be one there was nothing left to dispatch from.
#[tokio::test]
async fn comments_already_dispatched_for_are_not_dispatched_for_again_after_a_restart() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let stub = a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING);

    // Said after the review started, so a batch session is what they reach:
    // everything already there when it starts is the review's own to propose
    // about, and there would be nothing dispatched for to write down.
    let gh = gh_about_once(CHECKS_UNANSWERABLE, &reviews, THREE_COMMENTS, "");

    let fixture = grilling_spilling(spill, &stub, &gh).await;

    worked_to_empty(&fixture).await;
    until_written(&dispatched).await;

    pause(Duration::from_millis(500)).await;
    assert_eq!(
        prompts(&std::fs::read_to_string(&dispatched).unwrap()).len(),
        1,
        "the first server dispatched once for the batch",
    );

    // A second server over the same database, sandboxes and agent — which knows
    // nothing about the comments except what was written down.
    let _restarted = fixture.restarted(&stub, &gh).await;

    // Long enough for many polls of both of them.
    pause(Duration::from_millis(800)).await;

    let told = std::fs::read_to_string(&dispatched).unwrap();

    assert_eq!(
        prompts(&told).len(),
        1,
        "a restarted server dispatched about comments that had already been addressed: {told}",
    );
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "and the wrap-up is still going, because its checks were never green",
    );
}

/// The rule that ends a wrap-up: the checks green, the review answered, nothing
/// said left unaddressed and the pull request merging, all four together.
/// Verkstead decides it itself — there is nobody at the workbench to press
/// anything.
///
/// And what it does not wait for is the merge itself. Whether GitHub *can* merge
/// the pull request is asked on every poll, one conflicted being one nothing
/// could land; whether anybody *has* merged it is asked by nothing a wrap-up
/// runs. The pull request is open the whole time here — stages stack on unmerged
/// predecessors, so a Conversation that waited for one to land would hold up
/// every stage behind it.
///
/// The sweep after Done asks it, which is another question at another time: what
/// it is watching for is the moment there is nothing left to watch, rather than
/// anything a wrap-up is waiting on.
#[tokio::test]
async fn a_wrap_up_with_nothing_left_outstanding_finishes_without_waiting_for_a_merge() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let questions = spill.path().join("gh-questions");

    // A `gh` that writes down every question it is asked, so that what was never
    // asked can be read back.
    let recording = format!(
        "printf '%s\\n' \"$5\" >> {questions}\n{}",
        gh_about(GREEN, "", ""),
        questions = quoted(&questions),
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &recording,
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    // The move is on the record like every other, and it is the last thing to
    // have happened.
    assert_eq!(
        view.timeline.iter().rev().find_map(|event| match event {
            TimelineEvent::Moved(moved) => Some(moved.state),
            _ => None,
        }),
        Some(Lifecycle::Done),
    );
    assert!(
        notices(&view).is_empty(),
        "nothing stopped: {:?}",
        notices(&view),
    );
    assert_eq!(
        fixes(&view),
        0,
        "and nothing was dispatched, because nothing was wrong",
    );

    let asked = std::fs::read_to_string(&questions).unwrap();

    let fields = || asked.lines().flat_map(|asked| asked.split(','));

    assert!(
        fields().any(|field| field == "mergeable"),
        "whether it can be merged is asked on every poll: {asked}",
    );
    assert!(
        !fields().any(|field| field == "merged" || field == "mergedAt"),
        "and whether it has been merged is never asked at all: {asked}",
    );
}

/// And the two milestones a whole run passes through on its way there reach the
/// devices: the work landing on a pull request, and the Conversation reaching
/// Done.
///
/// These are the moments the work moved on with nobody watching. Everything
/// between the direction and Done happens unattended by design — that is what
/// the pipeline is *for* — and a human who has to keep opening the sidebar to
/// find out whether it did is one the notifications were never written for.
///
/// The device subscribes after the direction is picked, so what is read back is
/// the milestones alone: a Question Set's push is `push_delivery.rs`'s subject
/// and there is none left to send after this point anyway.
///
/// And the news mark the Done push leaves behind it is here too, because the
/// two are one act: the push is the moment, the mark is what is left of it on
/// every device until somebody opens the Conversation.
#[tokio::test]
async fn a_pull_request_opening_and_a_conversation_finishing_reach_the_devices() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(GREEN, "", ""),
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let (service, taken) = push_service().await;
    let phone = Device::new(&service, "phone");
    fixture.subscribe(&phone).await;

    let before = fixture.view().await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    // In the order they happened: the pull request opens the wrap-up, and Done
    // is what ends it.
    let pushed = pushes(&taken, 2).await;
    let opened = phone.read(&pushed[0]);
    let done = phone.read(&pushed[1]);

    assert_eq!(
        opened["path"],
        format!("/conversations/{}", fixture.id),
        "tapping either opens the Conversation it is about",
    );
    assert_eq!(
        opened["title"],
        format!("{} is on pull request #41", before.branch),
        "and the pull request's says which one, because that is where the human's \
         own part of the work starts",
    );
    assert_eq!(opened["project"], before.repo.name);

    assert_eq!(done["path"], format!("/conversations/{}", fixture.id));
    assert_eq!(
        done["title"],
        format!("{} is done", before.branch),
        "and Done's says so in words nothing else here says",
    );

    // Long enough for many more polls of the settling loop and the checks
    // watcher, either of which could have said the same thing twice.
    tokio::time::sleep(BRISKLY.checks * 3).await;

    assert_eq!(
        taken.lock().unwrap().len(),
        2,
        "one push per milestone, and no reminders about a Conversation that is over",
    );

    // And the sidebar keeps the second of them until the human has looked. The
    // push is a moment and the mark is what is left of it: a notification read
    // on the phone and swiped away would otherwise be the only trace this ever
    // finished.
    assert!(
        fixture.row().await.unseen,
        "the Conversation Verkstead carried to Done has news on its row",
    );

    fixture.see().await;

    assert!(
        !fixture.row().await.unseen,
        "and opening it is what takes the news off — everywhere, the mark being \
         the server's rather than a browser's",
    );

    // Long enough again for the loops that were still running to have said
    // anything they had left to say.
    tokio::time::sleep(BRISKLY.checks * 3).await;

    assert!(
        !fixture.row().await.unseen,
        "and it does not come back: this Done has been read",
    );
    assert_eq!(
        taken.lock().unwrap().len(),
        2,
        "and nothing was pushed a second time either",
    );
}

/// And none of it can hold the work up: a push service that cannot be reached
/// costs a notification and nothing else.
///
/// Every one of these is sent from behind the thing it announces, which is what
/// makes them safe to send at all — the pull request is recorded, the wrap-up
/// settles and the Conversation reaches Done whether or not a phone hears about
/// any of it. Reached through an address nothing is listening on, because that
/// is the shape of it on the real internet: a vendor's push service down, or a
/// tailnet with no way out.
///
/// The device stays on the list, too. Only a `404` or a `410` says a
/// subscription is finished with — see `push_delivery.rs` — and a service that
/// cannot be reached has said nothing at all about the device.
#[tokio::test]
async fn a_push_service_that_cannot_be_reached_costs_a_notification_and_nothing_else() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(GREEN, "", ""),
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let phone = Device::new(&nowhere().await, "phone");
    fixture.subscribe(&phone).await;

    // Which arrives on its own, at the pace it would have without any of this:
    // a run that waited on the push services would never get here at all.
    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;
    let opened = pull_request(&view).expect("the pull request is on the record");

    assert_eq!(opened.number, 41, "the record landed with nobody told");
    assert!(
        !notices(&view)
            .iter()
            .any(|notice| notice.contains("stopped")),
        "and a notification nobody could be sent is not a run that went wrong: {:?}",
        notices(&view),
    );

    let pool = open_database(&fixture.database).await.unwrap();
    let devices = verkstead_server::store::push_subscriptions(&pool)
        .await
        .unwrap();
    pool.close().await;

    assert_eq!(
        devices
            .iter()
            .map(|device| device.endpoint.as_str())
            .collect::<Vec<_>>(),
        [phone.endpoint.as_str()],
        "and the device is still on the list: nothing said it had gone",
    );
}

/// An address nothing is listening on: bound to claim a free port, then dropped.
async fn nowhere() -> String {
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("a port to claim");
    let at = listener.local_addr().unwrap();
    drop(listener);

    format!("http://{at}")
}

/// A commit landing on the pull request is a new run to wait on, so the green the
/// last one left does not stand.
///
/// The commit here is one a fix session pushes; what GitHub does with it is start
/// a run against the new head, and what the wrap-up does is stop being settled
/// until that one is green. A wrap-up that kept yesterday's green would finish a
/// Conversation on a suite that had never seen its last commit.
#[tokio::test]
async fn a_commit_landing_on_the_pull_request_puts_the_checks_back_to_waiting() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let landed = spill.path().join("landed");

    // The review reads the branch and then waits on the human, so it never
    // settles — which is what keeps the Conversation wrapping up while this
    // watches what the checks do.
    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_WAIT),
        &gh_about(&green_until(&landed), "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    let deadline = Instant::now() + *PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(Instant::now() < deadline, "the checks never settled");
        pause(Duration::from_millis(25)).await;
    }

    std::fs::write(&landed, "").unwrap();

    let deadline = Instant::now() + *PATIENCE;
    while checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "a commit landed and the checks stayed settled from the run before it",
        );
        pause(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls of a run that has not finished.
    pause(Duration::from_millis(500)).await;

    assert!(
        !checks_settled(&fixture).await,
        "and they stay waiting until the new run is green",
    );
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "so the wrap-up is not over",
    );
}

/// A breakdown asks its quiz the way every session asks anything: an ordinary
/// Set, with the session idling until the Answers come back. Nothing about it
/// ends or redirects the Conversation — the direction is settled, and what moves
/// the Conversation is the plan commit rather than any Answer.
///
/// Which matters more now that the quiz comes from the grilling session itself:
/// the approval round still gates the commit, and the Conversation stays grilling
/// across it.
#[tokio::test]
async fn a_breakdown_question_reaches_the_human_as_an_ordinary_set() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5) printf 'breaking down\n'; sleep 300 ;;
        *) printf 'this session never gets to run\n'; sleep 300 ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let quiz = fixture.ask(BREAKDOWN_QUIZ).await;

    let view = fixture.view().await;
    let asked = sets(&view);
    assert_eq!(
        asked.len(),
        2,
        "the proposal and the quiz, both on the Timeline they were asked from",
    );
    assert_eq!(
        view.direction,
        Some(verkstead_schema::Direction::TaskList),
        "an ordinary Set carries no proposal, so the pick that was made still stands",
    );

    assert_eq!(fixture.answer(quiz).await, Submitted::Accepted);

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Grilling,
        "answering a breakdown question moves nothing: the direction is settled, and \
         nothing has been committed yet",
    );
    assert_eq!(
        outputs(&fixture.view().await).len(),
        1,
        "and nothing was launched: the session writing the backlog is the one that \
         proposed",
    );
}

/// The same watching, in a read-write companion: a commit landing on the
/// companion's branch reaches the Timeline as a commit on the Conversation's
/// own does, and says which repository it came from.
///
/// The stub commits in the companion's checkout — which is a directory beside
/// the worktree it starts in, on a branch of its own — and then exits, so what
/// this asks is both halves at once: that the companion is being watched at all,
/// and that the last commit a session makes is caught for it too. Nothing tells
/// Verkstead a commit happened in either repository.
#[tokio::test]
async fn what_a_session_commits_in_a_companion_lands_on_the_timeline_labelled() {
    let fixture = grilling_building_in(
        r#"
        printf 'a limiter\n' > limiter.md
        git add limiter.md
        git commit --quiet -m 'feat: rate limiting'

        cd ../askance-*
        printf 'the other half\n' > halves.md
        git add halves.md
        git commit --quiet -m 'feat: the other half'
        "#,
        "askance",
    )
    .await;

    let landed = fixture
        .until(|view| {
            let landed = commits(view);
            (landed.len() == 2).then(|| landed.into_iter().cloned().collect::<Vec<_>>())
        })
        .await;

    // Read by what each commit was called rather than by where it is in the
    // list: two repositories swept at once are two repositories noticed in
    // whichever order the sweeps got there, and the Timeline's order is the
    // order Verkstead saw them.
    let named = |subject: &str| {
        landed
            .iter()
            .find(|commit| commit.subject == subject)
            .unwrap_or_else(|| panic!("no commit called {subject:?} among {landed:#?}"))
    };

    assert_eq!(
        named("feat: rate limiting").repo,
        None,
        "the work's own repository draws unlabelled",
    );
    assert_eq!(
        named("feat: the other half").repo,
        Some("askance".to_owned()),
        "and a companion's commit says which repository it came from",
    );

    // And the pane, which is the other half of what a commit is: the diff read
    // out of the repository it was recorded against rather than the
    // Conversation's own, which knows nothing about it.
    let pane = fixture.commit_pane(named("feat: the other half").id).await;

    let diff = pane.diff.expect("a commit that added a file has a diff");

    assert_eq!(diff.paths, vec!["halves.md".to_owned()]);
    assert!(diff.html.contains("the other half"), "{}", diff.html);
}

/// And a companion a steer opened up is one the session that steer launches may
/// write in: the commit it makes there lands on the Timeline labelled with the
/// Repo's registered name, exactly as one in a companion that came in
/// read-write does.
///
/// The whole of what an upgrade is for, end to end. The grilling session is
/// running against a companion it may only read; the steer ticks it up and
/// starts a session on an instruction; and that session commits in it. Nothing
/// but the upgrade stands between the two — the same repository, the same
/// Conversation, the same sandbox — so a commit landing at all is the sandbox
/// binding the new checkout writable, and the label on it is the sweep having
/// picked the branch up.
#[tokio::test]
async fn a_companion_a_steer_opened_up_is_one_the_next_session_writes_in() {
    let fixture = grilling_alongside(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'the grilling is running\n'
            sleep 300
            ;;
        *)
            cd ../askance-*
            printf 'the other half\n' > halves.md
            git add halves.md
            git commit --quiet -m 'feat: the other half'
            printf 'committed in the companion\n'
            sleep 300
            ;;
        esac
        "#,
        "askance",
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let view = fixture.view().await;
    let askance = view
        .companions
        .first()
        .expect("the fixture added one companion");

    assert_eq!(
        askance.mode,
        CompanionMode::ReadOnly,
        "a companion is added in the mode one is added in",
    );

    let repo_id = askance.repo.id;

    assert_eq!(fixture.steer().await, SteerOpened::Opened { working: true });
    assert_eq!(
        fixture
            .steer_opening("Implementing", "write the other half", repo_id, "")
            .await,
        ConversationSteered::Steered,
    );

    // The commit the session makes in it, which nothing tells Verkstead about:
    // what puts it here is the sweep of the branch the upgrade cut.
    let landed = fixture
        .until(|view| {
            commits(view)
                .into_iter()
                .find(|commit| commit.subject == "feat: the other half")
                .cloned()
        })
        .await;

    assert_eq!(
        landed.repo,
        Some("askance".to_owned()),
        "a companion's commit says which repository it came from",
    );

    let view = fixture.view().await;
    let opened = view
        .companions
        .first()
        .expect("the companion is still the one companion");

    assert_eq!(opened.mode, CompanionMode::ReadWrite);
    assert_eq!(
        opened.branch, "",
        "the field was left empty, which is the row following the Conversation's \
         own branch",
    );
}

/// A Conversation with a read-only companion has one branch being swept, not
/// two: that checkout is detached and bound read-only, so there is nothing there
/// for a commit to land on — and the Conversation's own commits draw exactly as
/// they always did.
///
/// The companion repository is given something a sweep of it would find: a
/// branch named as this Conversation's is, a commit ahead of the base its
/// checkout was made at. That is exactly what a read-write companion's watcher
/// resolves *mirroring* to, so a sweep that reached this one would put that
/// commit on the Timeline labelled `askance`. Nothing this session does can put
/// it there — no session can commit in a read-only checkout, which is why the
/// branch is made from outside — so what the Timeline still holding one commit
/// says is that nothing swept the companion at all.
#[tokio::test]
async fn a_read_only_companion_is_not_swept_and_the_conversations_own_is_unlabelled() {
    let fixture = grilling_alongside(
        r#"
        printf 'a limiter\n' > limiter.md
        git add limiter.md
        git commit --quiet -m 'feat: rate limiting'

        printf 'committed\n'
        sleep 300
        "#,
        "askance",
    )
    .await;

    // Put it there before waiting for anything, so it is in front of every sweep
    // this test gives the watchers rather than only the last few.
    let view = fixture.view().await;
    let companion = Path::new(&view.companions[0].repo.path).to_owned();

    git(&companion, &["checkout", "--quiet", "-b", &view.branch]);
    std::fs::write(companion.join("halves.md"), "the other half\n").unwrap();
    git(&companion, &["add", "halves.md"]);
    git(
        &companion,
        &["commit", "--quiet", "-m", "feat: the other half"],
    );

    // Waited for by name rather than by there being one: what this is about is
    // a commit that must never arrive, so the wait has to be one a commit
    // arriving would not simply time out.
    let landed = fixture
        .until(|view| {
            let landed = commits(view);
            landed
                .iter()
                .any(|commit| commit.subject == "feat: rate limiting")
                .then(|| landed.into_iter().cloned().collect::<Vec<_>>())
        })
        .await;

    let ours = landed
        .iter()
        .find(|commit| commit.subject == "feat: rate limiting")
        .expect("the wait above is for exactly this commit");

    assert_eq!(
        ours.repo, None,
        "an unlabelled card means the work's own repo",
    );

    // Long enough for several more sweeps of both repositories, one of which is
    // not being swept at all.
    tokio::time::sleep(Duration::from_secs(5)).await;

    let drawn = fixture.view().await;
    let subjects: Vec<&str> = commits(&drawn)
        .into_iter()
        .map(|commit| &*commit.subject)
        .collect();

    assert_eq!(
        subjects,
        ["feat: rate limiting"],
        "the companion's branch is nobody's to sweep, so what is on it reaches no Timeline",
    );
}

/// What a session leaves behind besides its output: the commits it lands on the
/// Conversation's branch, each one a Timeline Event with the diff behind it.
///
/// Nothing tells Verkstead a commit happened — the session is launched but not
/// driven — so what this asks is whether watching the branch notices. The stub
/// commits twice and then idles, exactly as a real session does between pieces
/// of work, so both are found by a sweep rather than by the session ending.
#[tokio::test]
async fn what_a_session_commits_lands_on_the_timeline() {
    let fixture = grilling(
        r#"
        printf 'a limiter\n' > limiter.md
        git add limiter.md
        git commit --quiet -m 'feat: rate limiting

```mermaid
flowchart LR
  in --> limiter --> out
```

A bucket per account.

Co-Authored-By: Claude <noreply@anthropic.com>'

        printf 'why\nand how\n' > NOTES.md
        git add NOTES.md
        git commit --quiet -m 'docs: say what it does'

        printf 'committed\n'
        sleep 300
        "#,
    )
    .await;

    let landed = fixture
        .until(|view| {
            let landed = commits(view);
            (landed.len() == 2).then(|| landed.into_iter().cloned().collect::<Vec<_>>())
        })
        .await;

    assert_eq!(
        landed
            .iter()
            .map(|commit| commit.subject.as_str())
            .collect::<Vec<_>>(),
        vec!["feat: rate limiting", "docs: say what it does"],
        "in the order they landed on the branch",
    );

    let notes = &landed[1];
    assert_eq!(notes.files, 1);
    assert_eq!(notes.insertions, 2);
    assert_eq!(notes.deletions, 0);

    // The details pane's half: the commit's own diff, rendered by the same
    // renderer an attached Diff goes through — folds, highlighting and all.
    let pane = fixture.commit_pane(notes.id).await;

    assert_eq!(
        pane.summary, None,
        "that commit's message was a subject and nothing else",
    );
    assert!(
        !pane.diagrams,
        "and a pane with no summary has nothing to draw, so it loads no mermaid",
    );

    let diff = pane.diff.expect("a commit that added a file has a diff");

    assert_eq!(diff.paths, vec!["NOTES.md".to_owned()]);
    assert!(
        diff.html.contains("<details class=\"diffFile\""),
        "the folds the renderer already gives an attached Diff: {}",
        diff.html
    );
    assert!(diff.html.contains("and how"), "{}", diff.html);
    assert!(
        !diff.html.contains("docs: say what it does"),
        "the message is the Event's to say — the diff arrives headerless: {}",
        diff.html
    );

    // And the other half of what the pane draws: what the commit said about
    // itself, rendered above the diff — the Diagram held for the client-side
    // renderer, the prose as prose, and none of the bookkeeping git keeps under
    // it.
    let drawn = fixture.commit_pane(landed[0].id).await;

    assert!(
        drawn.diagrams,
        "a summary with a Diagram in it is what the pane loads the renderer for",
    );

    let summary = drawn.summary.expect("that commit's message had a body");

    assert!(
        summary.contains("<pre class=\"mermaid\">"),
        "the Diagram is held for the renderer in the page: {summary}",
    );
    assert!(summary.contains("limiter"), "{summary}");
    assert!(
        summary.contains("<p>A bucket per account.</p>"),
        "and the prose is rendered markdown: {summary}",
    );
    assert!(
        !summary.contains("Co-Authored-By"),
        "the trailers are not what the agent had to say: {summary}",
    );

    // Long enough for several more sweeps of a branch that has not moved.
    pause(Duration::from_secs(5)).await;

    let view = fixture.view().await;
    assert_eq!(
        commits(&view).len(),
        2,
        "a branch is swept whole every few seconds, and each commit lands once",
    );
    assert_eq!(
        commits(&view)
            .iter()
            .map(|commit| commit.id)
            .collect::<Vec<_>>(),
        landed.iter().map(|commit| commit.id).collect::<Vec<_>>(),
        "and they are the same Events, rather than the same commits recorded again",
    );
}

/// The commit a session makes as its last act, which is the ordinary shape of
/// unattended work: it commits and exits, and there is no next poll to catch it.
#[tokio::test]
async fn a_commit_made_as_the_session_ends_still_lands() {
    let fixture = grilling(
        r#"
        printf 'a limiter\n' > limiter.md
        git add limiter.md
        git commit --quiet -m 'feat: rate limiting'
        "#,
    )
    .await;

    // The session is over — its Event says so — and the commit is on the
    // Timeline all the same, because ending it sweeps the branch once more.
    let landed = fixture
        .until(|view| {
            let ended = output(view).is_some_and(|output| !output.running);
            let landed = commits(view);

            (ended && !landed.is_empty()).then(|| landed[0].clone())
        })
        .await;

    assert_eq!(landed.subject, "feat: rate limiting");
    assert_eq!(landed.files, 1);
    assert_eq!(landed.insertions, 1);
}

/// The one ordering that matters when a Conversation is stopped: the agent is
/// gone before the directory it was working in is.
#[tokio::test]
async fn closing_ends_the_session_before_the_worktree_goes() {
    // Somewhere a session can leave evidence of itself that outlives the
    // worktree about to be removed: a directory Sandbox Configuration binds
    // into every sandbox.
    let spill = tempfile::tempdir().unwrap();
    let ticks = spill.path().join("ticks");

    let fixture = grilling_spilling(
        spill,
        &format!(
            r#"
            while :; do
                printf 'tick\n' >> {ticks}
                printf 'tick\n'
                sleep 0.05
            done
            "#,
            ticks = quoted(&ticks),
        ),
        PULL_REQUEST,
    )
    .await;

    let view = fixture.view().await;
    let worktree = PathBuf::from(view.worktree.expect("a grilling Conversation has one").path);

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    assert_eq!(fixture.close().await, ConversationClosed::Closed);

    let when_stopped = std::fs::metadata(&ticks).map(|it| it.len()).ok();

    assert!(
        when_stopped.is_some_and(|written| written > 0),
        "the session never got as far as ticking, so this proves nothing"
    );

    // Long enough for many more ticks, had anything still been ticking.
    pause(Duration::from_millis(500)).await;

    assert_eq!(
        std::fs::metadata(&ticks).map(|it| it.len()).ok(),
        when_stopped,
        "the session was still writing after the close said it had stopped"
    );
    assert!(
        !worktree.exists(),
        "the worktree should be gone once the Conversation is closed"
    );
    assert!(
        !output(&fixture.view().await)
            .expect("the Capture stays on the Timeline")
            .running,
        "a closed Conversation has no session running"
    );
}

/// A run that stops: an implementation session that goes wrong, and everything
/// the human is handed to decide with.
///
/// The whole reason a stop is written down is that nobody is at the terminal.
/// Verkstead launches the sessions but does not drive them, so a session that
/// falls over is a run that has quietly stopped — and what this asks is whether
/// stopping is *legible*: the Notice says which step failed, how it ended, what
/// git makes of the worktree and what the session last said, and the Conversation
/// says it is blocked on the human.
///
/// The stub exits 1 after saying something worth reading back, which is a crash
/// as far as anything outside it can tell.
#[tokio::test]
async fn a_session_that_exits_badly_halts_the_run_with_a_notice() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            printf 'half a limiter\n' > limiter.md
            printf 'error: unresolved import crate::window\n'
            exit 1
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let worktree = PathBuf::from(
        fixture
            .view()
            .await
            .worktree
            .expect("a grilling Conversation has a worktree")
            .path,
    );

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Implementing the work inline"),
        "which step failed: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("the session exited with status 1"),
        "and how it ended, which is the thing a status can say: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("limiter.md"),
        "what git makes of the worktree, which is where the half-done work is: {:?}",
        stopped.html,
    );
    assert!(
        stopped
            .html
            .contains("error: unresolved import crate::window"),
        "and the tail of what the session last said: {:?}",
        stopped.html,
    );
    assert!(
        !stopped.html.contains('\u{1b}'),
        "tidied of the terminal's own control sequences: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "Verkstead pulled the brake on a session that fell over, so going again \
         is the human's press rather than a restart's to assume",
    );

    let view = fixture.view().await;
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "the Conversation is blocked on the human, and says which Event it is blocked on",
    );
    assert!(
        !view.stopped_by_hand,
        "loudly, nobody having pressed anything: the badge rather than the quiet \
         label",
    );
    assert!(
        fixture.row().await.waiting,
        "and the sidebar says so too, a stop from outside the human being the \
         whole of what is waiting",
    );
    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "blocked on you is a badge on an active state, never a state of its own",
    );
    assert!(
        worktree.join("limiter.md").exists(),
        "and the repo is left exactly as the session left it",
    );
}

/// And where the session kept a log, the evidence is what it said rather than
/// what its terminal was drawing.
///
/// An agent that gives up says why in a sentence. Underneath that sentence is a
/// display of it — the box it was drawn in, the colours, the status line — which
/// says the same thing at ten times the length, and this is read on a phone by
/// somebody deciding whether to resume.
#[tokio::test]
async fn the_evidence_of_a_run_that_stopped_is_what_the_agent_said() {
    let fixture = grilling(
        r#"
        model=$1
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        case "$model" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            log=$HOME/.claude/projects/verkstead/$name.jsonl
            mkdir -p "$(dirname "$log")"

            printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"The window type is not where the brief says it is, so I have stopped."}]}}\n' > "$log"
            printf '\033[2m╭─ esc to interrupt ─╮\033[0m\n'
            exit 1
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert!(
        stopped
            .html
            .contains("The window type is not where the brief says it is, so I have stopped."),
        "the evidence should be the agent's own account of why it stopped: {:?}",
        stopped.html,
    );
    assert!(
        !stopped.html.contains("esc to interrupt"),
        "and not the interface it was drawn inside: {:?}",
        stopped.html,
    );
}

/// A backlog whose task session dies: the run stops at that task rather than
/// going round again, and the Notice says which task it was.
///
/// This is the case that matters most, because a runner is a loop: one that
/// relaunched a step nothing had moved would be a machine spending an account on
/// the same failure over and over, with nobody watching.
#[tokio::test]
async fn a_backlog_halts_at_the_task_whose_session_died() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            mkdir -p .tasks
            printf '# Rate limiting\n\n- [ ] 01: Count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            git add .tasks
            git commit --quiet -m 'chore: plan the rate limiter'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            printf 'this task is beyond me\n'
            exit 1
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let worktree = PathBuf::from(fixture.view().await.worktree.unwrap().path);

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("The task in .tasks/01-count.md"),
        "the Notice names the step that failed, so the human knows what stopped: \
         {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("the session exited with status 1"),
        "{:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("this task is beyond me"),
        "with the tail of what it said on its way out: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "a task nothing has moved is not one a restart may have another go at",
    );

    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "and the run is blocked on the human",
    );

    let sessions = outputs(&fixture.view().await).len();

    // Long enough for several more turns of a runner that was still turning.
    pause(Duration::from_secs(3)).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        sessions,
        "the run does not advance past a stop",
    );
    assert_eq!(
        notices(&fixture.view().await).len(),
        1,
        "and it does not stop twice over the same thing either",
    );

    assert!(
        worktree.join(".tasks/01-count.md").exists(),
        "the task is still there to be worked, because nothing reverted anything",
    );
}

/// A backlog whose next entry names a document nobody wrote: the run stops there
/// rather than putting a session at nothing to work from.
///
/// What a breakdown looks like part way through writing itself, and what a
/// hand-edited `TODO.md` looks like too. The box is what says a task is done, so
/// an entry that is not ticked is work still outstanding — and one with no file
/// beside it is work nothing can be told how to do.
#[tokio::test]
async fn a_backlog_entry_with_no_task_file_stops_the_run() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            mkdir -p .tasks
            printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
            printf -- '- [x] 01: count the requests\n' >> .tasks/TODO.md
            printf -- '- [ ] 02: refuse the excess\n' >> .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            git add .tasks
            git commit --quiet -m 'chore: plan the rate limiter'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            printf 'working a task nobody wrote down\n'
            sleep 300
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Working the backlog"),
        "the Notice says what stopped: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("entry 02 of"),
        "and names the entry there is nothing to work from: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("<code>.tasks/TODO.md</code>"),
        "in the file the human has to go and fix: {:?}",
        stopped.html,
    );

    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "a backlog nothing can be read out of is not one a restart may guess past",
    );

    let sessions = outputs(&fixture.view().await).len();

    // Long enough for several more turns of a runner that was still turning.
    pause(Duration::from_secs(3)).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        sessions,
        "and no session was launched at the entry there is nothing to work from",
    );

    // And the press that follows the Notice they have just read: a Resume
    // before `TODO.md` is fixed asks the same question of the same backlog, so
    // it gets the same answer. A press is the human's leave to try the run
    // again rather than their leave to work a list nothing can be read out of.
    let notices = said(&fixture.view().await).len();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let again = fixture
        .until(|view| said(view).get(notices).map(|notice| (*notice).clone()))
        .await;

    assert!(
        again.html.contains("entry 02 of"),
        "the resumed run stops at the same entry, and says so again: {:?}",
        again.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "and on the same reading, which is not one a restart may guess past either",
    );

    // Long enough again for a launch to have happened behind the stop.
    pause(Duration::from_secs(3)).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        sessions,
        "and the press spent no session on it: what Resume found is what the \
         loop found",
    );
}

/// The backlog every usage-limit test below is worked against: two tasks, and a
/// stub that prints its account is out of window once it has worked the first.
///
/// The sentence is the one claude 2.1.234 draws — the reset time is what the
/// caller varies. Each task session commits its task and exits, so the run
/// reaches the point of launching the next one, which is the moment the stop has
/// to hold it.
///
/// The banner comes *after* the commit, which is what makes these tests say one
/// thing: the account runs out between two steps, so what a Resume picks up is
/// the next task whether or not it ends the session that was waiting.
///
/// **The banner is redrawn, with the glyph in front of it turning**, which is
/// what a display does for as long as the wait lasts. The line is therefore a
/// different string every frame, and every frame of it has to come to one stop:
/// what Verkstead keeps is the line as it was drawn, decoration and all, so a
/// build that told one banner from the next by comparing those strings would
/// read every repaint as a fresh limit — the store refuses the second stop, so
/// what it costs is the reading behind it, twice a second for as long as the
/// wait lasts.
fn out_of_window(sentence: &str) -> String {
    out_of_window_marked("'✻' '✽' '✳' '✢'", sentence)
}

/// And the same for a backend that puts something else in front of its own
/// sentence: `marks` is the shell word list the stub draws the banner with, one
/// repaint apiece and the whole list twice over.
///
/// A list of one is a display that redraws the *same* string, which is the other
/// half of what the latch is for: claude's spinner turns, and a backend that
/// draws a still mark repeats itself exactly. Neither is a second wait.
fn out_of_window_marked(marks: &str, sentence: &str) -> String {
    out_of_window_saying(&format!(
        r#"
                    for pass in 1 2; do
                        for turning in {marks}
                        do
                            printf "$turning {sentence}\r\n"
                            sleep 0.125
                        done
                    done
        "#
    ))
}

/// And the same again for a backend that *draws* it rather than printing it:
/// `banner` is the shell the stub says it with, run where the account runs out.
///
/// Which is what a full-screen display does — a cursor move per row and no
/// newline anywhere — and what the two above come to as well, their printing
/// being the ordinary case of the same thing.
fn out_of_window_saying(banner: &str) -> String {
    format!(
        r#"
        case "$1" in
        claude-grilling-5|gpt-5-codex-grilling|grok-4.6-grilling)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            mkdir -p .tasks
            printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
            printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
            printf -- '- [ ] 02: refuse the excess\n' >> .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            printf '# 02. Refuse the excess\n' > .tasks/02-refuse.md
            git add .tasks
            git commit --quiet -m 'chore: plan the rate limiter'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
            next=$(ls .tasks | grep -E "^$number-" | head -n 1)
            if [ -n "$next" ]; then
                printf 'working %s\n' "$next"
                printf 'a limiter\n' >> limiter.md
                sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
                git add -A
                git commit --quiet -m "feat: $next"
                if [ "$next" = 01-count.md ]; then
                    # The wait itself, in miniature: the task lands, the account
                    # runs out before the next one, and the agent holds with its
                    # banner up. Said twice over — more than the half a second
                    # Verkstead writes down what a session printed on, so the
                    # banner is looked at more than once — with whatever that
                    # backend puts in front of it, which is what makes each
                    # repaint a different line where a mark turns and the same
                    # line where it does not.
                    # The glyphs themselves rather than `\xe2\x9c\xbb` and its
                    # kind. `\xNN` is bash's extension to `printf` and not
                    # POSIX: a `/bin/sh` that is dash — which is what Debian and
                    # Ubuntu have, so it is what CI runs — prints the escape
                    # rather than the character, and a banner opening with a
                    # literal backslash is one [`verkstead_server`] is right to
                    # refuse. ASCII punctuation is not decoration there,
                    # deliberately, so it does not open a status line. This file
                    # is UTF-8 and the shell passes the bytes through, which
                    # needs no escape at either end. `\033` is the one escape
                    # POSIX does give a format, which is what a frame is drawn
                    # with.
                    {banner}
                fi
            else
                printf 'finishing\n'
                git rm --quiet -r .tasks
                git commit --quiet -m 'chore: finish rate-limiting'
            fi
            sleep 300
            ;;
        esac
        "#
    )
}

/// Get such a backlog running, and hand back the fixture once the first task's
/// session has started.
async fn running_out(fixture: &Grilling) {
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);
}

/// An account that runs out of window mid-run: the run stops with a Notice
/// naming the account and the line the session printed, the devices are told,
/// the session that printed it is ended, and nothing else is launched until
/// somebody presses Resume.
///
/// The agent waits too, and that is exactly why the session goes: no stop
/// resumes itself, so an agent left holding would wake when the window came
/// back and work on inside a Conversation that reads as stopped. What Verkstead
/// puts in its place is a wait answerable from a phone instead of a session that
/// has gone quiet for no stated reason.
#[tokio::test]
async fn an_account_out_of_window_stops_the_run_and_tells_the_devices() {
    let fixture = grilling(&out_of_window(
        "Usage limit reached · continuing automatically at 2026-08-24T05:00:00Z · esc to cancel",
    ))
    .await;

    running_out(&fixture).await;

    // Subscribed after the closing Set is answered, so the only push these
    // devices are ever told about is the one this test is about — a Set's own
    // push is `asking.rs`'s subject.
    let (service, taken) = push_service().await;
    let phone = Device::new(&service, "phone");
    fixture.subscribe(&phone).await;

    let notice = fixture.stopped().await;

    assert!(
        notice
            .html
            .contains("<strong>Implementing the work</strong> stopped."),
        "it stopped the way everything else stops, in the same words: {:?}",
        notice.html,
    );
    assert!(
        notice.html.contains(
            "the account <strong>implementation</strong> was being spent is out of window"
        ),
        "naming the account that ran out, which is the implementation Profile the \
         task was being worked under: {:?}",
        notice.html,
    );
    assert!(
        notice.html.contains("✻ Usage limit reached"),
        "with the backend's own sentence kept as it was printed, spinner and all: {:?}",
        notice.html,
    );

    let stop = fixture.stop_on_the_record().await;

    assert_eq!(
        stop.decision,
        Decision::Verkstead,
        "Verkstead pulled the brake, so a restart leaves it waiting",
    );
    assert_eq!(
        stop.resets.as_deref(),
        Some("2026-08-24T05:00:00Z"),
        "and the stop carries when the window comes back, where the sentence said",
    );

    assert_eq!(
        fixture.view().await.blocked_on,
        Some(notice.id),
        "a run that has stopped carries *blocked on you*, whatever stopped it",
    );

    // One per subscribed device, exactly as a Question Set's push is.
    let pushed = pushes(&taken, 1).await;
    let told = phone.read(&pushed[0]);

    assert_eq!(
        told["path"],
        format!("/conversations/{}", fixture.id),
        "tapping it opens the Conversation whose run stopped",
    );
    assert_eq!(
        told["title"], "implementation is out of window until 2026-08-24T05:00:00Z",
        "and it says which account and until when, which is what decides whether the \
         human does anything about it",
    );

    let sessions = outputs(&fixture.view().await).len();

    // Long enough for several more turns of a runner that was still turning.
    pause(Duration::from_secs(3)).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        sessions,
        "while it is stopped the run does not advance: no next Step, no fresh session",
    );
    assert_eq!(
        notices(&fixture.view().await).len(),
        1,
        "and the banner redrawing — eight repaints, a different line each time as \
         the spinner turns — does not stop it twice over the same wait",
    );
    assert_eq!(
        taken.lock().unwrap().len(),
        1,
        "one push for the wait, however long it lasts",
    );

    assert!(
        outputs(&fixture.view().await)
            .iter()
            .all(|session| !session.running),
        "and no session is running behind the stop: the agent would have held its \
         own at the limit and worked on when the window came back, inside a \
         Conversation that reads as stopped",
    );

    let worktree = PathBuf::from(fixture.view().await.worktree.unwrap().path);

    assert_eq!(
        git(&worktree, &["log", "--format=%s", "-1"]),
        "feat: 01-count.md\n",
        "the task the session was working landed, because a stop reverts nothing",
    );
    assert!(
        worktree.join(".tasks/02-refuse.md").exists(),
        "and the task after it is still there to be worked",
    );
    assert_eq!(
        git(&worktree, &["status", "--porcelain"]),
        "",
        "the Worktree is exactly as the session left it",
    );
}

/// The wording is the backend's, so a Codex session is stopped by codex's own
/// sentence — and by that one alone.
///
/// One phrase per backend, matched off the Capture exactly as claude's is: the
/// stop is the ordinary stop, naming the Profile whose account ran out and
/// keeping the line the session drew. What follows codex's phrase is the plan's
/// — an upgrade, credits to buy, an admin to ask — so the sentence here carries
/// decoration the matcher has never seen and the stop lands all the same.
///
/// The mark in front is still rather than turning, which is the other half of
/// what the latch is for: eight repaints of the *same* string are still one
/// wait.
#[tokio::test]
async fn a_codex_account_out_of_window_stops_the_run_on_codexs_own_sentence() {
    let fixture = grilling_spilling_on_codex(
        tempfile::tempdir().unwrap(),
        &out_of_window_marked(
            "'▌'",
            "You've hit your usage limit. Upgrade to Plus to continue using Codex \
             (https://chatgpt.com/explore/plus)",
        ),
        PULL_REQUEST,
    )
    .await;

    running_out(&fixture).await;

    let notice = fixture.stopped().await;

    assert!(
        notice
            .html
            .contains("the account <strong>codex</strong> was being spent is out of window"),
        "naming the Profile whose account ran out, which is the Codex one every \
         role of this run is on: {:?}",
        notice.html,
    );
    assert!(
        notice.html.contains("▌ You've hit your usage limit."),
        "with codex's own sentence kept as it was drawn, the plan's decoration and \
         all: {:?}",
        notice.html,
    );

    let stop = fixture.stop_on_the_record().await;

    assert_eq!(
        stop.decision,
        Decision::Verkstead,
        "Verkstead pulled the brake, exactly as it does on the other backend",
    );
    assert_eq!(
        stop.resets, None,
        "and codex names no reset, so the stop carries none: what the sentence \
         does not say, nothing here invents",
    );

    assert_eq!(
        fixture.view().await.blocked_on,
        Some(notice.id),
        "a run that has stopped carries *blocked on you*, whatever backend it was on",
    );

    let sessions = outputs(&fixture.view().await).len();

    // Long enough for several more turns of a runner that was still turning.
    pause(Duration::from_secs(3)).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        sessions,
        "while it is stopped the run does not advance: no next Step, no fresh session",
    );
    assert_eq!(
        notices(&fixture.view().await).len(),
        1,
        "and the same line redrawn is the same wait, however many times it is drawn",
    );
    assert!(
        outputs(&fixture.view().await)
            .iter()
            .all(|session| !session.running),
        "and no session is running behind the stop",
    );
}

/// And the other record a session leaves behind: a Codex session that says its
/// account is spent in its own rollout and never on its display is stopped off
/// the Transcript.
///
/// Both records are read, on either backend and for the same reason — a backend
/// that says so in one and not the other would otherwise go unnoticed until the
/// terminal happened to speak. Nothing of codex's limit line reaches the
/// Capture here, so the Transcript is the only thing that could have stopped it.
#[tokio::test]
async fn a_codex_session_saying_so_only_in_its_rollout_is_stopped_off_the_transcript() {
    let fixture = grilling_on_codex(
        r#"
        day=$HOME/.codex/sessions/$(date +%Y/%m/%d)
        mkdir -p "$day"
        log=$day/rollout-2026-08-30T17-47-02-cccc.jsonl

        printf '{"type":"session_meta","payload":{"cwd":"%s"}}\n' "$(pwd)" > "$log"
        printf 'reading the brief\n'
        printf '{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"m-1","content":[{"type":"Text","text":"You'"'"'ve hit your usage limit. Upgrade to Plus to continue using Codex."}]}}}\n' >> "$log"

        sleep 300
        "#,
    )
    .await;

    let notice = fixture.stopped().await;

    assert!(
        notice
            .html
            .contains("the account <strong>codex</strong> was being spent is out of window"),
        "the stop is the same stop, named for the same Profile: {:?}",
        notice.html,
    );
    assert!(
        notice
            .html
            .contains("You've hit your usage limit. Upgrade to Plus"),
        "carrying the sentence as the rollout held it: {:?}",
        notice.html,
    );

    let event = fixture
        .until(|view| output(view).map(|output| output.id))
        .await;

    assert!(
        !fixture.capture(event).await.contains("usage limit"),
        "and the terminal never said a word about it, so the Transcript is the \
         only record that could have stopped this run",
    );

    assert_eq!(
        fixture.stop_on_the_record().await.decision,
        Decision::Verkstead,
        "Verkstead pulled the brake",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(notice.id),
        "and the run is blocked on the human until they press Resume",
    );
}

/// And the third backend, which says it by *drawing* it: a Grok session that
/// puts its card up stops the run off the frame that card is on.
///
/// The same claim as the Codex one above and the harder half of it. Grok heads a
/// bordered card with `You hit your free usage limit.` and offers three tiers
/// under it — read off grok 1.0.13 driven until it drew one — and it draws that
/// card the way a full-screen display draws anything: a cursor move to each row
/// and not one newline in the frame. So what the session *printed* is a single
/// line with the spinner it was turning at the front of it and the sentence
/// somewhere in the middle, which is a line that says nothing; what it drew is a
/// grid, and the sentence is a line of that with the card's border in front. The
/// stub says it exactly that way round, so a build that read only the bytes
/// would run this backlog to the end.
///
/// A paid account's card is headed differently and nobody has watched one drawn,
/// so a paid stop stalls instead — the accepted state rather than a gap, and
/// nothing this can stand for.
#[tokio::test]
async fn a_grok_account_out_of_window_stops_the_run_on_the_card_it_draws() {
    let fixture = grilling_on_grok(&out_of_window_saying(
        r#"
                    for pass in 1 2 3 4; do
                        printf '\033[?2026h\033[23;5H⠴ 7s\033[19;3H┃  You hit your free usage limit.\033[?2026l'
                        sleep 0.125
                    done
        "#,
    ))
    .await;

    running_out(&fixture).await;

    let notice = fixture.stopped().await;

    assert!(
        notice
            .html
            .contains("the account <strong>grok</strong> was being spent is out of window"),
        "naming the Profile whose account ran out, which is the Grok Build one \
         every role of this run is on: {:?}",
        notice.html,
    );
    assert!(
        notice.html.contains("You hit your free usage limit."),
        "with the card's own heading kept as it was drawn, the border in front \
         of it and all: {:?}",
        notice.html,
    );

    let stop = fixture.stop_on_the_record().await;

    assert_eq!(
        stop.decision,
        Decision::Verkstead,
        "Verkstead pulled the brake, exactly as it does on the other two backends",
    );
    assert_eq!(
        stop.resets, None,
        "and grok names no reset, so the stop carries none: what the sentence \
         does not say, nothing here invents",
    );

    assert_eq!(
        fixture.view().await.blocked_on,
        Some(notice.id),
        "a run that has stopped carries *blocked on you*, whatever backend it was on",
    );

    let sessions = outputs(&fixture.view().await).len();

    // Long enough for several more turns of a runner that was still turning.
    pause(Duration::from_secs(3)).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        sessions,
        "while it is stopped the run does not advance: no next Step, no fresh session",
    );
    assert_eq!(
        notices(&fixture.view().await).len(),
        1,
        "and the banner turning is the same wait, however many times it is drawn",
    );
    assert!(
        outputs(&fixture.view().await)
            .iter()
            .all(|session| !session.running),
        "and no session is running behind the stop",
    );
}

/// And off the last of the three: a Grok session that says its account is spent
/// in the log it keeps, and neither prints nor draws it, is stopped off the
/// Transcript.
///
/// Every record is read on this backend as on the others — a backend that said
/// so in one and not the rest would otherwise go unnoticed until the terminal
/// happened to speak — and what carries it here is grok's own kind for the prose
/// it streams, a session update being what grok's log is made of.
///
/// **This one is a guard rather than a path grok walks.** Driven to a real
/// refusal, grok 1.0.13 puts nothing about it in its log but a `retry_state`
/// line whose reason is the server's own error string — bookkeeping, which the
/// reader folds under its own name and the summary never reads. So today a Grok
/// limit is caught on the frame and only there; what this holds is the case
/// where a release starts saying it in prose.
#[tokio::test]
async fn a_grok_session_saying_so_only_in_its_log_is_stopped_off_the_transcript() {
    let fixture = grilling_on_grok(
        r#"
        name=
        while [ $# -gt 0 ]; do
            if [ "$1" = --session-id ]; then name=$2; fi
            shift
        done

        mine=$HOME/.grok/sessions/$(pwd | sed 's|/|%2F|g')/$name
        mkdir -p "$mine"
        log=$mine/updates.jsonl

        printf 'reading the brief\n'
        printf '{"method":"session/update","params":{"sessionId":"%s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"You hit your free usage limit."}}}}\n' "$name" > "$log"

        sleep 300
        "#,
    )
    .await;

    let notice = fixture.stopped().await;

    assert!(
        notice
            .html
            .contains("the account <strong>grok</strong> was being spent is out of window"),
        "the stop is the same stop, named for the same Profile: {:?}",
        notice.html,
    );
    assert!(
        notice.html.contains("You hit your free usage limit."),
        "carrying the sentence as the log held it: {:?}",
        notice.html,
    );

    let event = fixture
        .until(|view| output(view).map(|output| output.id))
        .await;

    assert!(
        !fixture.capture(event).await.contains("usage limit"),
        "and the terminal never said a word about it, so the Transcript is the \
         only record that could have stopped this run",
    );

    assert_eq!(
        fixture.stop_on_the_record().await.decision,
        Decision::Verkstead,
        "Verkstead pulled the brake",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(notice.id),
        "and the run is blocked on the human until they press Resume",
    );
}

/// The human presses Resume — the one Resume, the same button every other stop
/// waits on — and the backlog picks up from where it stopped.
#[tokio::test]
async fn the_humans_press_starts_a_stopped_run_again_where_it_stopped() {
    let fixture = grilling(&out_of_window("Usage limit reached")).await;

    running_out(&fixture).await;

    fixture.stopped().await;

    assert_eq!(
        fixture.stop_on_the_record().await.resets,
        None,
        "a sentence with no time in it carries no reset words, which changes \
         nothing about how the stop ends",
    );

    // The stop ended the session that printed the banner — no stop resumes
    // itself, so an agent left holding would have carried on unwatched — and
    // the driver that was seeing it out advanced nothing past the stop. So the
    // press finds a Worktree with nothing running in it, and what it does is
    // recompute the step and launch.
    fixture
        .until(|view| {
            outputs(view)
                .iter()
                .all(|session| !session.running)
                .then_some(())
        })
        .await;

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    // The rest of the backlog, worked by sessions of its own: the run picked up
    // at the step it had reached rather than starting over.
    let subjects = fixture
        .until(|view| {
            let landed = commits(view);
            (landed.len() == 4).then(|| {
                landed
                    .iter()
                    .map(|commit| commit.subject.clone())
                    .collect::<Vec<_>>()
            })
        })
        .await;

    assert_eq!(
        subjects,
        vec![
            "chore: plan the rate limiter".to_owned(),
            "feat: 01-count.md".to_owned(),
            "feat: 02-refuse.md".to_owned(),
            "chore: finish rate-limiting".to_owned(),
        ],
        "the backlog in order and each step once: the run picked up where it stopped \
         rather than starting over",
    );

    let worktree = PathBuf::from(fixture.view().await.worktree.unwrap().path);

    assert_eq!(
        git(&worktree, &["status", "--porcelain"]),
        "",
        "and nothing was reverted, reset or stashed on the way through the stop",
    );

    assert_eq!(
        fixture.view().await.blocked_on,
        None,
        "and nothing is blocked on them any more",
    );
}

/// And nobody presses anything: the reset passing changes nothing at all.
///
/// The sentence names a time that has been and gone, so anything reading it as a
/// moment would find it due the instant the stop was written. Nothing does: the
/// reset is words on the card, the run waits for the press like every other, and
/// there is no clock anywhere for it to wait on instead.
#[tokio::test]
async fn a_reset_that_has_been_and_gone_starts_nothing() {
    let fixture = grilling(&out_of_window(
        "Usage limit reached · continuing automatically at 2020-01-01T00:00:00Z",
    ))
    .await;

    running_out(&fixture).await;

    let notice = fixture.stopped().await;

    assert_eq!(
        fixture.stop_on_the_record().await.resets.as_deref(),
        Some("2020-01-01T00:00:00Z"),
        "the reset is on the stop in the words the session printed it in",
    );

    // The baseline, taken once the run has gone quiet all the way through: no
    // session running and nothing left on the drivers register.
    //
    // The Notice appearing is not that moment. The stop ends the session that
    // printed the banner and the driver seeing it out lets go a little after,
    // and on a loaded machine a commit already in flight lands in between — so
    // a count read at the Notice is a count read before the run had finished
    // stopping, and the window below then reads the tail of the stop as the
    // backlog going on. Read in the same view as the quiet is established, so
    // there is no gap between the two.
    let landed = fixture
        .until(|view| (!view.working && !view.driven).then(|| commits(view).len()))
        .await;

    // Long enough for anything running on a clock to have come round.
    pause(Duration::from_secs(3)).await;

    let view = fixture.view().await;

    assert_eq!(
        commits(&view).len(),
        landed,
        "the backlog did not go on: the task after the one that landed is still \
         waiting to be worked",
    );
    assert_eq!(
        view.blocked_on,
        Some(notice.id),
        "and the Conversation is still blocked on the human, which is what a stop \
         nothing resumes for itself comes to",
    );
    assert_eq!(
        notices(&view).len(),
        1,
        "one Notice for the one stop: a Conversation stopped on a window is never \
         swept up as a stall on top of it",
    );
}

/// An exhausted account is a wait, never a reason to spend a different one.
#[tokio::test]
async fn nothing_moves_a_stopped_conversation_onto_another_profile() {
    let fixture = grilling(&out_of_window("Usage limit reached")).await;

    let before = fixture.view().await;

    running_out(&fixture).await;

    fixture.stopped().await;
    fixture.resume().await;

    fixture
        .until(|view| (commits(view).len() == 4).then_some(()))
        .await;

    let after = fixture.view().await;

    assert_eq!(
        after
            .grilling_pairing
            .pairing()
            .map(|pairing| pairing.profile.id),
        before
            .grilling_pairing
            .pairing()
            .map(|pairing| pairing.profile.id),
    );
    assert_eq!(
        after
            .implementation_pairing
            .map(|pairing| pairing.profile.id),
        before
            .implementation_pairing
            .map(|pairing| pairing.profile.id),
        "the account that ran out is the account the rest of the run is spent on",
    );
}

/// Closing a conversation is not a run that went wrong.
///
/// The close ends the session and takes the worktree away, so every signal the
/// runner reads says the step did not land — the file is gone because the whole
/// directory is. What tells it apart is that Verkstead is what ended the session:
/// stopping here would be telling the human that driving stopped, about the thing
/// they had just stopped themselves.
#[tokio::test]
async fn closing_a_run_is_not_something_to_ask_the_human_about() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *) printf 'working\n'; sleep 300 ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    // Once the breakdown session is up, so there is a run to close mid-step.
    fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.running && output.lines > 0)
                .map(|output| output.id)
        })
        .await;

    assert_eq!(fixture.close().await, ConversationClosed::Closed);

    // Long enough for the driver to have noticed its session go and decided what
    // that meant.
    pause(Duration::from_secs(2)).await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "a run the human stopped has nothing to tell them about: {:?}",
        notices(&view),
    );
    assert_eq!(
        view.blocked_on, None,
        "and a closed Conversation is not blocked on anybody",
    );
    assert_eq!(view.state, Lifecycle::Closed);
}

/// A backlog whose task sessions wait at a gate the test opens, so that a press
/// can arrive while a step is genuinely in flight.
///
/// Two tasks, because what Stop promises is about the *next* one: a backlog with
/// a single task in it would come to rest whether the human pressed anything or
/// not.
fn two_tasks_waiting_at(gate: &Path) -> String {
    format!(
        r#"
case "$1" in
claude-grilling-5)
    printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
    printf 'breaking down\r\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf -- '- [ ] 02: refuse the excess\n' >> .tasks/TODO.md
    printf '# 01. Count the requests\n' > .tasks/01-count.md
    printf '# 02. Refuse the excess\n' > .tasks/02-refuse.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    printf 'the backlog has landed\r\n'
    sleep 300
    ;;
*)
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    printf 'working %s\r\n' "$next"
    while [ ! -f {gate} ]; do sleep 0.05; done
    printf 'a limiter\n' >> limiter.md
    sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
    git add -A
    git commit --quiet -m "feat: $next"
    sleep 300
    ;;
esac
"#,
        gate = quoted(gate),
    )
}

/// Stop pressed while a task is being worked: the session finishes what it was
/// doing, and the run stops before it starts the next one.
///
/// The whole promise of the press, and both halves of it matter. Nothing is cut
/// short — the commit the session was on its way to lands, because a step killed
/// halfway is work the human then has to pick apart — and nothing is launched
/// behind it, because a run that took one more task after being told to stop
/// would be a machine ignoring a button.
///
/// It is the human's own press, so it stays stopped until they say otherwise and
/// their phone is told nothing: they are the one person who already knows.
#[tokio::test]
async fn stop_lets_the_task_finish_and_halts_before_the_next_one() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let fixture = grilling_spilling(spill, &two_tasks_waiting_at(&gate), PULL_REQUEST).await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    // The first task's session, waiting at the gate: a step in flight, which is
    // what this press is about.
    let working = fixture.attachable(2).await;

    let (service, taken) = push_service().await;
    fixture.subscribe(&Device::new(&service, "phone")).await;

    assert_eq!(fixture.stop().await, ConversationStopped::Stopping);

    let view = fixture.view().await;

    assert!(
        outputs(&view)
            .iter()
            .any(|output| output.id == working && output.running),
        "the session working the task is left alone: Stop waits for it",
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing has stopped yet, because the step has not finished: {:?}",
        notices(&view),
    );
    assert!(
        view.stop_asked,
        "the press is recorded, which is what takes Stop off the menu: there is \
         nothing left to ask, and pressing it again would do what the first one did",
    );
    assert!(
        view.ready_to_stop,
        "and there is still a run to stop, which is what keeps Force stop there \
         — the escalation from here, for a human who turns out not to want to \
         wait for the step",
    );

    // What the session was waiting for. From here it commits its task and idles,
    // which is where the run would have launched the next one.
    std::fs::write(&gate, "go").unwrap();

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("you pressed Stop"),
        "the Notice says whose stop it was: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Human,
        "a stop the human asked for is not one a restart may drive past",
    );

    let view = fixture.view().await;

    assert_eq!(
        commits(&view)
            .iter()
            .map(|commit| commit.subject.clone())
            .collect::<Vec<_>>(),
        vec![
            "chore: plan rate-limiting tasks".to_owned(),
            "feat: 01-count.md".to_owned(),
        ],
        "the task the session was on landed: nothing was cut short",
    );
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "and the run is waiting on the human, who is the only one who ends this",
    );

    let worktree = PathBuf::from(view.worktree.clone().unwrap().path);

    // Long enough for the runner to have launched the next task if it were still
    // going to, several times over.
    pause(Duration::from_secs(2)).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        2,
        "the grilling and the one task: nothing was started after the press",
    );
    assert!(
        worktree.join(".tasks/02-refuse.md").exists(),
        "the task nothing was launched for is still there to be worked",
    );
    assert!(
        taken.lock().unwrap().is_empty(),
        "and nobody's phone was told: they pressed it themselves",
    );

    // And the one press that undoes it, which reads the backlog again and works
    // what is left of it.
    assert_eq!(fixture.resume().await, Resumed::Resumed);

    fixture
        .until(|view| (outputs(view).len() == 3).then_some(()))
        .await;

    assert_eq!(
        fixture.view().await.blocked_on,
        None,
        "nothing is waiting on the human once the work is being driven again",
    );
}

/// Force stop pressed while a task is being worked: the session is ended where
/// it stands and the stop is written at once.
///
/// The other half of the same choice. What it costs is the step — the session is
/// killed mid-sentence, and whatever it had not committed stays uncommitted in
/// the Worktree — and what it buys is not having to wait for a session that may
/// be stuck for hours. Nothing is reverted: the repository is left exactly as the
/// session left it, which is what makes taking it on by hand possible.
///
/// Then the two presses on a Conversation that has already stopped, which refuse
/// as such: a second stop is not a thing to record, and Resume is what gets one
/// going again.
#[tokio::test]
async fn force_stop_ends_the_session_where_it_stands_and_halts_at_once() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("never");
    let fixture = grilling_spilling(spill, &two_tasks_waiting_at(&gate), PULL_REQUEST).await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let working = fixture.attachable(2).await;

    let (service, taken) = push_service().await;
    fixture.subscribe(&Device::new(&service, "phone")).await;

    // The gate is never opened, so this session would sit there for the whole
    // five minutes it sleeps for. Which is the case the press is for.
    assert_eq!(fixture.force_stop().await, ConversationStopped::Stopped);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("you pressed Force stop"),
        "the Notice says whose stop it was, and which of the two: {:?}",
        stopped.html,
    );
    assert_eq!(fixture.chosen().await, Decision::Human);

    fixture
        .until(|view| {
            outputs(view)
                .iter()
                .find(|output| output.id == working)
                .filter(|output| !output.running)
                .map(|_| ())
        })
        .await;

    let view = fixture.view().await;
    let worktree = PathBuf::from(view.worktree.clone().unwrap().path);

    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "and the run is waiting on the human",
    );
    assert!(
        worktree.join(".tasks/01-count.md").exists(),
        "the task the session was cut off in the middle of is still there: \
         nothing was reverted and nothing was finished either",
    );

    // Long enough for the driver that was seeing the session out to have decided
    // what its ending meant, and for anything else to have been launched.
    pause(Duration::from_secs(2)).await;

    let view = fixture.view().await;

    assert_eq!(
        outputs(&view).len(),
        2,
        "nothing was started to replace what was ended",
    );
    assert_eq!(
        notices(&view).len(),
        1,
        "and the session Verkstead ended is not a run that went wrong: one stop, \
         one Notice",
    );
    assert!(
        taken.lock().unwrap().is_empty(),
        "and nobody's phone was told about a press they made themselves",
    );

    assert_eq!(
        fixture.stop().await,
        ConversationStopped::AlreadyStopped,
        "a Conversation that has stopped is not one to stop again",
    );
    assert_eq!(
        fixture.force_stop().await,
        ConversationStopped::AlreadyStopped,
        "whichever of the two is pressed",
    );

    assert_eq!(
        fixture.resume().await,
        Resumed::Resumed,
        "and the one press that undoes either of them works on this one too",
    );

    fixture
        .until(|view| (outputs(view).len() == 3).then_some(()))
        .await;
}

/// Force stop pressed as an inline grilling lands its handoff: the run stops
/// there, and nothing is started on the far side of the pick.
///
/// The moment a stop has the least to hold on to. A step that landed is landed,
/// so the driver seeing the grilling out asks the worktree once more before it
/// reads the ending — finds the handoff sitting there, takes it, and carries on
/// to what the pick asked for, which is a fresh session under the implementation
/// Profile. That launch is the only thing left between the press and an agent
/// being spent, and the press left nothing waiting behind it to be found: a Force
/// stop writes its stop outright. So the launch is what has to ask about the
/// stop, and not about the press.
///
/// The stub keeps talking after it writes the handoff, which is what makes the
/// press land in that moment rather than beside it: handoff plus quiet is what
/// would ordinarily end the grilling, so a session that never goes quiet leaves
/// the document on disk and unclaimed until the press ends it.
#[tokio::test]
async fn force_stop_as_the_handoff_lands_starts_nothing_behind_the_halt() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'the grilling is running\n'
            while [ ! -f /tmp/verkstead/go ]; do sleep 0.1; done
            printf '# What we settled\n\nAn in-process counter.\n' > /tmp/verkstead/handoff.md
            while true; do printf 'still talking\n'; sleep 0.05; done
            ;;
        *)
            printf 'model=%s\n' "$1"
            sleep 300
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    std::fs::write(handoff_directory(&fixture).join("go"), "").unwrap();

    // The handoff on disk with the session still talking, which is the state the
    // press has to arrive in: the driver is watching for it and has not taken it.
    let written = handoff_directory(&fixture).join("handoff.md");
    let deadline = Instant::now() + *PATIENCE;
    while !written.is_file() {
        assert!(
            Instant::now() < deadline,
            "the grilling never wrote the handoff its pick asked for",
        );
        pause(Duration::from_millis(25)).await;
    }

    assert_eq!(fixture.force_stop().await, ConversationStopped::Stopped);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("you pressed Force stop"),
        "the run stopped because of the press: {:?}",
        stopped.html,
    );
    assert_eq!(fixture.chosen().await, Decision::Human);

    // Long enough for the driver to have read the ending, taken the handoff and
    // reached the launch on the other side of it.
    pause(Duration::from_secs(2)).await;

    let view = fixture.view().await;

    assert!(
        handoff(&view).is_some(),
        "the driver did get past the ending — the handoff was there and it took \
         it, which is the whole reason there was a launch left to stop",
    );
    assert_eq!(
        outputs(&view).len(),
        1,
        "and then nothing: the grilling is the only session this Conversation \
         ever ran, and no implementation account was spent behind the stop",
    );
    assert_eq!(notices(&view).len(), 1, "one press, one stop, one Notice",);
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "and the Conversation is waiting on the human to start it again",
    );
}

/// Steer clicked while a task is being worked, with **Interrupt current task**
/// ticked on the modal it opened: the session is ended where it stands and the
/// Conversation is Done.
///
/// The click is what stops the drive, and it stops it the way Stop does — the
/// session is left alone and nothing new is launched — so the checkbox is the
/// only thing that ends one. What it costs is the step, exactly as Force stop's
/// does: the task is left however far the session had got, uncommitted and all,
/// and nothing is reverted.
///
/// And nothing at all starts afterwards, which is what steering into Done means:
/// there is nothing to drive in Done, and the stop the click wrote is taken away
/// rather than left as a badge on finished work that no press could answer.
#[tokio::test]
async fn steering_into_done_with_interrupt_ends_the_session_where_it_stands() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("never");
    let fixture = grilling_spilling(spill, &two_tasks_waiting_at(&gate), PULL_REQUEST).await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    // The first task's session, waiting at a gate nothing opens: a step in
    // flight, which is the case the checkbox is for.
    let working = fixture.attachable(2).await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: true },
        "the click found a session running, which is what the checkbox is offered against",
    );

    assert!(
        outputs(&fixture.view().await)
            .iter()
            .any(|output| output.id == working && output.running),
        "and left it alone: the click stops the drive, not the session",
    );

    assert_eq!(
        fixture.steer_into("Done", true).await,
        ConversationSteered::Steered,
    );

    fixture
        .until(|view| {
            outputs(view)
                .iter()
                .find(|output| output.id == working)
                .filter(|output| !output.running)
                .map(|_| ())
        })
        .await;

    let view = fixture.view().await;
    let worktree = PathBuf::from(view.worktree.clone().unwrap().path);

    assert_eq!(view.state, Lifecycle::Done);
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("moved", Lifecycle::Implementing),
            ("steer", Lifecycle::Done),
            ("moved", Lifecycle::Done),
        ],
        "the human's own Event carrying the target, and the plain move under it",
    );
    assert!(
        worktree.join(".tasks/01-count.md").exists(),
        "the task the session was cut off in the middle of is still there: \
         nothing was reverted and nothing was finished either",
    );

    // Long enough for the driver that was seeing the session out to have decided
    // what its ending meant, and for anything else to have been launched.
    pause(Duration::from_secs(2)).await;

    let view = fixture.view().await;

    assert_eq!(
        outputs(&view).len(),
        2,
        "the grilling and the one task: nothing was started to replace what was ended",
    );
    assert_eq!(
        view.blocked_on, None,
        "and nothing is waiting on the human: the stop the click wrote went with the move",
    );
    assert!(
        !view.ready_to_resume && !view.ready_to_stop,
        "there being nothing to drive in Done, and so neither press to offer",
    );
}

/// The same steer with the checkbox left alone: the session finishes what it was
/// doing, and nothing is launched behind it.
///
/// Done is the target this is true of, and it is true because nothing runs
/// there: what ends a session under every other target is the session the steer
/// starts taking the Worktree from it, and here there is none to start. What
/// holds the next launch off is not a stop any more — the steer took that away —
/// but where the work now stands: nothing drives a Conversation Verkstead has
/// finished with, so the driver seeing this session out finds a state with no
/// next step in it. See `stopping::stopped`, which is the one question every
/// launch asks.
#[tokio::test]
async fn steering_into_done_without_interrupt_sees_the_session_out() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let fixture = grilling_spilling(spill, &two_tasks_waiting_at(&gate), PULL_REQUEST).await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let working = fixture.attachable(2).await;

    assert_eq!(fixture.steer().await, SteerOpened::Opened { working: true });
    assert_eq!(
        fixture.steer_into("Done", false).await,
        ConversationSteered::Steered,
    );

    assert!(
        outputs(&fixture.view().await)
            .iter()
            .any(|output| output.id == working && output.running),
        "the session is still working the task it was on: nothing was cut short",
    );

    // What it was waiting for. From here it commits its task and idles, which is
    // where the run would have launched the next one.
    std::fs::write(&gate, "go").unwrap();

    fixture
        .until(|view| (commits(view).len() == 2).then_some(()))
        .await;

    // Long enough for the runner to have launched the next task if it were still
    // going to, several times over.
    pause(Duration::from_secs(2)).await;

    let view = fixture.view().await;
    let worktree = PathBuf::from(view.worktree.clone().unwrap().path);

    assert_eq!(
        commits(&view)
            .iter()
            .map(|commit| commit.subject.clone())
            .collect::<Vec<_>>(),
        vec![
            "chore: plan rate-limiting tasks".to_owned(),
            "feat: 01-count.md".to_owned(),
        ],
        "the task the session was on landed: it was seen out rather than ended",
    );
    assert_eq!(
        outputs(&view).len(),
        2,
        "and nothing was started after it, the work being finished with",
    );
    assert!(
        worktree.join(".tasks/02-refuse.md").exists(),
        "so the task nothing was launched for is still there",
    );
    assert_eq!(view.state, Lifecycle::Done);
    assert_eq!(
        view.blocked_on, None,
        "and no badge came back on the far side of the session that was seen out",
    );
}

/// What a Conversation's Timeline says about where the work went, in order: the
/// states the human steered it into and the states it moved to.
///
/// Both kinds together, because what a steer leaves is the pair — the human's
/// own line and the machine's move under it — and a reading that kept only one
/// of them could not say they stand beside each other.
fn steered(view: &ConversationView) -> Vec<(&'static str, Lifecycle)> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Steer(steer) => Some(("steer", steer.target)),
            TimelineEvent::Moved(moved) => Some(("moved", moved.state)),
            _ => None,
        })
        .collect()
}

/// Stop pressed with nothing running: there is nothing to see out, so it stops
/// where it stands.
///
/// The quiet moments are half of what a run is — between two steps, waiting on a
/// poll, after a session has gone — and a Stop that recorded a wish and did
/// nothing in one of them would be the press the human trusted least. So the
/// same button means the same thing whenever it is pressed: from here on,
/// nothing is driving this.
#[tokio::test]
async fn stop_pressed_with_nothing_running_halts_where_it_stands() {
    let fixture = grilling(r#"printf 'the grilling has nothing to say\n'"#).await;

    fixture.quiet().await;

    assert_eq!(fixture.stop().await, ConversationStopped::Stopped);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("nothing was running to see out"),
        "the Notice says why it stopped now rather than after something: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("Grilling the work"),
        "and what it was that stopped: {:?}",
        stopped.html,
    );
    assert_eq!(fixture.chosen().await, Decision::Human);

    let view = fixture.view().await;

    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "and nothing is driving the Conversation from here, with the Notice \
         saying so where the record kept it",
    );
    assert!(
        view.stopped_by_hand,
        "said quietly, because they are the one who pressed it: the label \
         rather than the badge",
    );
    assert!(
        !fixture.row().await.waiting,
        "and the sidebar's dot stays off, a dot on the work they stopped last \
         being the one that teaches them to ignore the dots",
    );
}

/// A roadmap Conversation that stages, wraps up and settles — with a stub that
/// tells the sessions apart by the skill their prompts name, which is the fact
/// under them: every one after the grilling runs under the one implementation
/// Profile.
///
/// The grilling session is the one that stages, because that is what a roadmap
/// pick does now: it reads on into the staging skill and carries the branch to a
/// pull request without leaving the context that settled the work. It writes the
/// roadmap as it starts rather than when the Response lands, because a stub
/// cannot idle on a blocking ask — nothing in these fixtures dials the router —
/// and a roadmap already committed is watched for exactly as one committed a
/// minute later is.
///
/// `workflow` is what the branch records about how this repository's own work
/// goes for review, written beside the roadmap by the session that stages it.
/// Empty is a repository that records nothing, which is one of the two answers
/// about where the next stage's branch goes.
fn a_roadmap_then_wraps_up(planning: &Path, worked: &Path, stages: &str, workflow: &str) -> String {
    format!(
        r#"
case "$2" in
*grilling/SKILL.md*)
    printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
    printf 'the handoff is written\n'
    mkdir -p docs/roadmaps/rate-limiting docs/agents
{workflow}
    printf '# Rate limiting roadmap\n\n## Stages\n\n{stages}' > docs/roadmaps/rate-limiting/ROADMAP.md
    printf '# 01. Count the requests\n\n## Goal\n\nA counter per key, and nothing else.\n' > docs/roadmaps/rate-limiting/01-counter.md
    printf '# 02. Refuse the rest\n' > docs/roadmaps/rate-limiting/02-refusing.md
    git add -A
    git commit --quiet -m 'docs: stage the rate-limiting roadmap'
    printf 'pushed, and the pull request is open\n'
    sleep 300
    ;;
*reviewing/SKILL.md*)
    printf 'I read the whole branch and found nothing worth raising\n'
    exit 0
    ;;
*next-stage/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {planning}
    printf 'planning the stage\n'
    mkdir -p .tasks
    printf '# Count the requests\n\nRoadmap stage: [01: Count the requests](docs/roadmaps/rate-limiting/01-counter.md)\n\n## Tasks\n\n- [ ] 01: count them — [details](01-count.md)\n' > .tasks/TODO.md
    printf '# 01. count them\n' > .tasks/01-count.md
    sed -i 's|\[brief\](01-counter.md)|[brief](01-counter.md) *(in progress: `rate-limiting/01-counter`)*|' docs/roadmaps/rate-limiting/ROADMAP.md 2>/dev/null || true
    git add -A
    git commit --quiet -m 'chore: plan counter tasks'
    sleep 300
    ;;
*next-task/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {worked}
    printf 'working the stage backlog\n'
    sleep 300
    ;;
*)
    sleep 300
    ;;
esac
"#,
        planning = quoted(planning),
        worked = quoted(worked),
    )
}

/// What a repository that records a way to stack a roadmap stage says, as the
/// staging session writes it onto the branch.
const RECORDS_STACKING: &str = r#"    printf '# Git workflow\n\n## Review process\n\n### Finish sequence\n\nPush it, open a draft PR.\n\n### Stacking roadmap stages\n\nAdopt the predecessor with `gh stack init <predecessor> <new>`.\n' > docs/agents/git-workflow.md"#;

/// Both stages open, which is a roadmap with something to start.
const TWO_STAGES: &str = r#"- [ ] 01: Count the requests — [brief](01-counter.md)\n- [ ] 02: Refuse the rest — [brief](02-refusing.md)\n"#;

/// The two presses a roadmap Conversation ever takes: start grilling, and pick
/// the roadmap direction on the Set that ends it.
///
/// Everything after this happens with nobody watching, which is what makes the
/// far side of it worth a notification — so it is a step of its own, for the
/// tests that subscribe a device in between.
async fn staged(fixture: &Grilling) {
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "roadmap").await, Submitted::Accepted);
}

/// Take a roadmap Conversation from its Brief to a settled wrap-up: staged,
/// pushed, reviewed, green, done — with nothing pressed but the two the human
/// presses at the start.
async fn staged_and_settled(fixture: &Grilling) {
    staged(fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;
}

/// Every Conversation the workbench knows about, newest first.
async fn conversations(app: &Router) -> Vec<verkstead_render::ConversationEntry> {
    get(app, "/api/ui/conversations").await
}

/// The Conversation the stage was started as, once there is one — or a panic
/// saying nothing ever started.
/// Waited for as a Conversation that is *working* rather than as one that
/// exists: it is a record before it is a branch, a worktree and a notice, and a
/// test that read it the moment the row appeared would be reading a half-made
/// one.
async fn stage_of(fixture: &Grilling) -> ConversationView {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        let started = conversations(&fixture.app)
            .await
            .into_iter()
            .find(|entry| entry.id != fixture.id);

        if let Some(entry) = started {
            let view: ConversationView =
                get(&fixture.app, &format!("/api/ui/conversations/{}", entry.id)).await;

            if view.state == Lifecycle::Implementing && !notices(&view).is_empty() {
                return view;
            }
        }

        assert!(
            Instant::now() < deadline,
            "no stage was ever started. The Timeline says: {:?}",
            notices(&fixture.view().await),
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// What a Conversation has said on its own account, once it has said anything.
///
/// A notice is written after the thing it is about has happened, so a test that
/// read the Timeline the moment the work finished would be reading it too early.
async fn said_by(fixture: &Grilling) -> String {
    fixture
        .until(|view| {
            let said = notices(view);

            (!said.is_empty()).then(|| said.join("\n"))
        })
        .await
}

/// What Verkstead has said on a Timeline on its own account.
fn notices(view: &ConversationView) -> Vec<String> {
    said(view)
        .into_iter()
        .map(|notice| notice.html.clone())
        .collect()
}

/// The same, as the Events they are — which is what a badge points at.
///
/// Every Notice but the one a wrap-up narrowing to its checks writes, which is
/// [`waiting_on_checks`]'s to read. That line is a label on a condition rather
/// than anything Verkstead did about the run: it is written on a wrap-up with
/// nothing wrong with it, and it lands last, so the tests that ask what a
/// Conversation had to say about its own run would otherwise be reading it.
fn said(view: &ConversationView) -> Vec<&NoticeEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Notice(notice) if !narrowing(notice) => Some(notice),
            _ => None,
        })
        .collect()
}

/// And the lines a wrap-up down to its checks writes, which is the one kind
/// [`said`] passes over.
fn waiting_on_checks(view: &ConversationView) -> Vec<&NoticeEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Notice(notice) if narrowing(notice) => Some(notice),
            _ => None,
        })
        .collect()
}

/// Which of the two a Notice is, by what it opens with — the rendered markdown
/// the settling loop writes.
fn narrowing(notice: &NoticeEvent) -> bool {
    notice.html.contains("Waiting on checks")
}

/// The whole of stage auto-continue: a settled wrap-up on a roadmap Conversation
/// starts the next stage, with nobody asked.
///
/// The stage is a Conversation of its own — one Repo, one branch, one Worktree —
/// against the same Repo, under the same Profiles, primed with the stage brief as
/// its Brief and going straight to Implementing. Its branch stacks on the
/// predecessor because this repository records how, and the session it starts is
/// the bundled fork of next-stage, told which of the two happened.
#[tokio::test]
async fn a_settled_wrap_up_starts_the_next_stage_on_a_conversation_of_its_own() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, RECORDS_STACKING),
        &gh_about(GREEN, "", ""),
    )
    .await;

    let roadmap_branch = fixture.view().await.branch;

    staged_and_settled(&fixture).await;

    let stage = stage_of(&fixture).await;

    assert_eq!(
        stage.branch, "rate-limiting/01-counter",
        "the branch is the stage brief's own name, under the roadmap it belongs to",
    );
    assert_eq!(
        stage.state,
        Lifecycle::Implementing,
        "straight to Implementing: the grilling that would have settled this wrote the brief",
    );
    assert_eq!(
        stage.repo.path,
        fixture.view().await.repo.path,
        "and against the same Repo, a stage being the same work one branch further on",
    );
    assert_eq!(
        stage
            .implementation_pairing
            .as_ref()
            .map(|pairing| pairing.profile.name.clone()),
        Some("implementation".to_owned()),
        "under the same Profiles, there being nobody to choose them again",
    );

    let brief = stage
        .timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Brief(brief) => Some(brief.markdown.clone()),
            _ => None,
        })
        .expect("a Conversation's first Event is its Brief");

    assert!(
        brief.contains("A counter per key, and nothing else."),
        "primed with the stage brief itself: {brief:?}",
    );

    // What the runner is watching, and what says the stage went straight to
    // work: no move through Grilling or Direction, because neither happened.
    assert_eq!(
        stage
            .timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::Moved(moved) => Some(moved.state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [Lifecycle::Implementing],
    );

    let said = notices(&stage).join("\n");

    assert!(
        said.contains("Stage 01") && said.contains("rate-limiting"),
        "the stage says which stage of which roadmap it is: {said:?}",
    );
    assert!(
        said.contains(&format!("<code>{roadmap_branch}</code>")),
        "and that its branch stacks on the one the stage before it was worked on: {said:?}",
    );

    // The branch really is on top of the predecessor's work, which is what
    // stacking is: the roadmap commit is in this branch's history.
    let worktree = PathBuf::from(stage.worktree.expect("a stage has a Worktree").path);

    assert!(
        git(&worktree, &["log", "--oneline"]).contains("docs: stage the rate-limiting roadmap"),
        "the stage's branch stands on the predecessor's unmerged work",
    );

    // And the Conversation that settled says what became of it, where the human
    // was watching when it happened.
    let carried_on = said_by(&fixture).await;

    assert!(
        carried_on.contains("Stage 01")
            && carried_on.contains("<code>rate-limiting/01-counter</code>"),
        "the settled Conversation says which stage started and on what: {carried_on:?}",
    );

    // The session the stage is working in: the bundled fork of next-stage, under
    // the implementation Profile, primed with the brief and told where its branch
    // came from.
    let prompt = until_written(&planning).await;

    assert!(
        prompt.contains("model=claude-implementation-5"),
        "the stage plans under the implementation Profile: {prompt:?}",
    );
    assert!(
        prompt.contains("/verkstead/skills/next-stage/SKILL.md"),
        "inside the bundled fork of next-stage: {prompt:?}",
    );
    assert!(
        prompt.contains("A counter per key, and nothing else."),
        "primed with the stage brief: {prompt:?}",
    );
    assert!(
        prompt.contains(&format!("stacks on `{roadmap_branch}`")),
        "and told what its branch stands on, which it cannot read anywhere: {prompt:?}",
    );

    // And what the plan commit hands over to: the runner works the backlog the
    // fork wrote, one fresh session per task, with nothing pressed in between.
    let working = until_written(&worked).await;

    assert!(
        working.contains("/verkstead/skills/next-task/SKILL.md"),
        "the stage's backlog is worked by the same runner a feature's is: {working:?}",
    );

    // The roadmap keeps its own score on the branch that earned it, and the
    // pinned stage list follows: it is read off the Worktree as it stands.
    let stage =
        get::<ConversationView>(&fixture.app, &format!("/api/ui/conversations/{}", stage.id)).await;

    let stages = stage
        .pinned
        .iter()
        .find_map(|pinned| match pinned {
            PinnedEvent::StageList(list) => Some(list),
            _ => None,
        })
        .expect("the roadmap this branch has written to is pinned");

    assert_eq!(
        stages
            .stages
            .iter()
            .map(|stage| (stage.number.as_str(), stage.title.as_str(), stage.done))
            .collect::<Vec<_>>(),
        [
            ("01", "Count the requests", false),
            ("02", "Refuse the rest", false),
        ],
        "stage 01 is under way rather than done: what ticks it is the stage after it",
    );

    assert!(
        backlog(&stage).is_some(),
        "and the backlog it wrote is pinned beside it, as a feature's is",
    );

    // And on the record where it landed, as a feature's is: a stage's first step
    // is the one that writes its backlog, and landing that is the same moment.
    assert_eq!(
        backlog_row(&stage).map(|reached| reached.list.as_ref()),
        Some(backlog(&stage)),
        "the stage's backlog is on the record too, drawn from the pinned reading",
    );

    // The annotation itself, which is what says the stage is in flight and on
    // what — and what stops the stage after it being read as this one again.
    let index = std::fs::read_to_string(worktree.join("docs/roadmaps/rate-limiting/ROADMAP.md"))
        .expect("the roadmap is on the stage's branch too");

    assert!(
        index.contains("*(in progress: `rate-limiting/01-counter`)*"),
        "the roadmap says which branch stage 01 is being worked on: {index:?}",
    );
}

/// And the roadmap moving on is the third milestone: the devices are told which
/// stage started, and tapping it opens the Conversation the work is on now.
///
/// This is the moment the pipeline is *for* — a stage complete, its successor
/// already running, and nobody asked about any of it. What the human would
/// otherwise do is open the sidebar to find out whether it happened.
///
/// The Conversation that settled is announced too, in its own words: they are
/// two facts about two Conversations, and a phone told only that something was
/// done would say nothing about the thing that had started.
#[tokio::test]
async fn a_roadmap_moving_on_tells_the_devices_which_stage_started() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, RECORDS_STACKING),
        &gh_about(GREEN, "", ""),
    )
    .await;

    staged(&fixture).await;

    let (service, taken) = push_service().await;
    let phone = Device::new(&service, "phone");
    fixture.subscribe(&phone).await;

    let roadmap = fixture.view().await;
    let stage = stage_of(&fixture).await;

    // Three: the pull request the staging session opened, the roadmap
    // Conversation reaching Done, and the stage that started behind it.
    let told: Vec<serde_json::Value> = pushes(&taken, 3)
        .await
        .iter()
        .map(|push| phone.read(push))
        .collect();

    let titles: Vec<&str> = told
        .iter()
        .map(|notice| notice["title"].as_str().unwrap())
        .collect();

    let started = told
        .iter()
        .find(|notice| notice["title"] == "Stage 01 of the `rate-limiting` roadmap has started")
        .unwrap_or_else(|| panic!("nothing said a stage had started: {titles:?}"));

    assert_eq!(
        started["path"],
        format!("/conversations/{}", stage.id),
        "tapping it opens the stage, which is where the work is now — not the \
         Conversation it was started from",
    );
    assert_eq!(started["project"], roadmap.repo.name);

    assert!(
        titles.contains(&format!("{} is done", roadmap.branch).as_str()),
        "and the Conversation that settled says so in words of its own: {titles:?}",
    );
    assert!(
        titles.contains(&format!("{} is on pull request #41", roadmap.branch).as_str()),
        "as does the pull request it opened on the way: {titles:?}",
    );

    // Long enough for the stage's own run to have got going and said something
    // else, if starting one were worth announcing twice.
    tokio::time::sleep(BRISKLY.checks * 3).await;

    assert_eq!(
        taken.lock().unwrap().len(),
        3,
        "one push per thing that happened, and nothing about the stage's own work",
    );
}

/// A repository that records no way to stack a roadmap stage gets a branch off
/// its default branch — and the Timeline says so plainly rather than Verkstead
/// inventing a convention the repository never agreed to.
#[tokio::test]
async fn a_repository_with_no_stacking_recorded_gets_a_stage_off_the_default_branch() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, ""),
        &gh_about(GREEN, "", ""),
    )
    .await;

    staged_and_settled(&fixture).await;

    let stage = stage_of(&fixture).await;
    let said = notices(&stage).join("\n");

    assert!(
        said.contains("no way to stack"),
        "what was missing is said rather than worked around: {said:?}",
    );
    assert!(
        said.contains("<code>main</code>"),
        "and where the branch went instead: {said:?}",
    );

    let worktree = PathBuf::from(stage.worktree.expect("a stage has a Worktree").path);

    assert!(
        !git(&worktree, &["log", "--oneline"]).contains("docs: stage the rate-limiting roadmap"),
        "the branch came off the default branch, so the roadmap is not under it",
    );

    let prompt = until_written(&planning).await;

    assert!(
        prompt.contains("not stacked on anything"),
        "and the session is told that rather than left to guess: {prompt:?}",
    );
}

/// Give `repo` an origin it is behind: an upstream holding everything it holds
/// plus one commit more, which this checkout has heard nothing about.
///
/// The upstream is a working clone rather than a bare one so that a commit can
/// be made straight on its default branch, and the extra commit is made there
/// rather than pushed from `repo` — a push would move this checkout's own copy
/// of `origin/main`, and being out of date about origin is the whole state
/// these are for.
///
/// The caller keeps the directory the upstream is in alive: the fetch this
/// sets up is against a path rather than a server, and a tempdir that had gone
/// would look exactly like being offline.
///
/// `--no-local` because `repo` is not sitting still while this runs. A clone
/// from a path copies the object store file by file, and git's own manual says
/// that races with anything writing to it — which is precisely what the session
/// running in this Conversation's worktree does, its commits landing as loose
/// objects in the very directory being copied. The clone dies on the temp file
/// that was renamed out from under it. Going through the git transport instead
/// asks the source what it holds and takes a pack of it, so a write arriving
/// mid-clone is simply not in the answer.
fn behind_an_origin(repo: &Path, upstream: &Path) {
    git(
        upstream.parent().unwrap(),
        &[
            "clone",
            "--no-local",
            &repo.to_string_lossy(),
            &upstream.to_string_lossy(),
        ],
    );
    git(
        upstream,
        &["config", "user.email", "test@verkstead.invalid"],
    );
    git(upstream, &["config", "user.name", "Verkstead Test"]);

    git(
        repo,
        &["remote", "add", "origin", &upstream.to_string_lossy()],
    );
    git(repo, &["fetch", "--quiet", "origin"]);

    std::fs::write(upstream.join("ahead.md"), "# origin moved on\n").unwrap();
    git(upstream, &["add", "ahead.md"]);
    git(upstream, &["commit", "-m", "docs: origin moves on"]);
}

/// An unstacked stage's branch comes off what origin is holding, not off
/// wherever this checkout's copy of the default branch was last left.
///
/// The rule a grilling starts by, at the other end of the pipeline: a machine
/// that has not pulled for a week would otherwise start every stage of a
/// roadmap a week behind the work, with nobody at a button to notice.
#[tokio::test]
async fn an_unstacked_stage_comes_off_origins_tip_rather_than_the_local_branch() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, ""),
        &gh_about(GREEN, "", ""),
    )
    .await;

    // After the grilling started, so that this Conversation came off the local
    // branch and the stage is the only thing origin's tip decides.
    let elsewhere = tempfile::tempdir().unwrap();
    let repo = fixture.repo();
    behind_an_origin(&repo, &elsewhere.path().join("upstream"));

    let behind = git(&repo, &["rev-parse", "main"]).trim().to_owned();

    staged_and_settled(&fixture).await;

    let stage = stage_of(&fixture).await;
    let worktree = PathBuf::from(stage.worktree.expect("a stage has a Worktree").path);

    assert!(
        worktree.join("ahead.md").exists(),
        "the stage should have come off what origin is holding now",
    );

    // The fetch moved the remote-tracking ref and nothing else: the human's own
    // branch is exactly where they left it.
    assert_eq!(git(&repo, &["rev-parse", "main"]).trim(), behind);
}

/// Nobody is at a button when a stage starts, so a fetch git would not make
/// halts it with a notice naming the fetch — and starts nothing at all.
///
/// Halted rather than carried on with off whatever was last fetched: a stage
/// branched from the wrong place is a whole stage of work to unpick, where a
/// notice is a thing the human can go and fix.
#[tokio::test]
async fn a_stage_whose_fetch_fails_halts_with_a_notice_and_starts_nothing() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, ""),
        &gh_about(GREEN, "", ""),
    )
    .await;

    let gone = tempfile::tempdir().unwrap();
    let nowhere = gone.path().join("no-such-remote");
    git(
        &fixture.repo(),
        &["remote", "add", "origin", &nowhere.to_string_lossy()],
    );

    staged_and_settled(&fixture).await;

    let said = said_by(&fixture).await;

    assert!(
        said.contains("would not fetch"),
        "the fetch is what is named, that being what the human can go and fix: {said:?}",
    );

    assert_eq!(
        conversations(&fixture.app).await.len(),
        1,
        "and nothing was started",
    );
    assert!(
        !planning.exists(),
        "so no session was launched inside the next-stage fork either",
    );
}

/// One companion of a Conversation, by the name of the Repo it is a checkout of.
fn alongside<'a>(view: &'a ConversationView, name: &str) -> &'a CompanionView {
    view.companions
        .iter()
        .find(|companion| companion.repo.name == name)
        .unwrap_or_else(|| {
            panic!(
                "`{name}` should be a companion of this Conversation, which has {:?}",
                view.companions
                    .iter()
                    .map(|companion| companion.repo.name.as_str())
                    .collect::<Vec<_>>(),
            )
        })
}

/// Where a companion was checked out, and what git says that directory is
/// holding: the branch it is on, or `HEAD` where it is detached.
fn holding(companion: &CompanionView) -> (PathBuf, String) {
    let path = PathBuf::from(
        &companion
            .worktree
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "`{}` should be checked out somewhere",
                    companion.repo.name.as_str()
                )
            })
            .path,
    );

    let head = git(&path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_owned();

    (path, head)
}

/// Move a companion repository's default branch on, and hand back the commit it
/// now stands at.
///
/// What tells the two base rules apart. A checkout cut from the configured base
/// holds this commit, and one cut from the predecessor stage's companion branch
/// — which was made before it — does not.
fn moved_on(repo: &Path) -> String {
    std::fs::write(repo.join("moved-on.md"), "# the companion moved on\n").unwrap();
    git(repo, &["add", "moved-on.md"]);
    git(repo, &["commit", "-m", "docs: the companion moves on"]);

    git(repo, &["rev-parse", "HEAD"]).trim().to_owned()
}

/// A roadmap grilled with companions builds with them: the stage a settled
/// wrap-up starts carries its parent Conversation's whole companion set across,
/// and has every one of them checked out before its first session runs.
///
/// A stage has no draft moment of its own, so the inheritance funnel is the only
/// place the set could come from — without it a roadmap grilled against two
/// repositories would build against one.
///
/// Read-only comes across as it is and is detached at whatever its base resolves
/// to *for this stage*; read-write cuts a branch of its own named after the
/// stage's own branch, whatever the roadmap Conversation's row was called. This
/// one does not stack, so both come off the configured base as it stands at the
/// moment the stage starts.
#[tokio::test]
async fn a_stage_inherits_the_companion_set_its_roadmap_was_grilled_with() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_at_pace(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, ""),
        &gh_about(GREEN, "", ""),
        *BRISKLY,
        &[
            ("askance", CompanionMode::ReadWrite),
            ("chronicle", CompanionMode::ReadOnly),
        ],
    )
    .await;

    let roadmap = fixture.view().await;
    let written = alongside(&roadmap, "askance").clone();
    let read = alongside(&roadmap, "chronicle").clone();

    // What mirroring came to for the roadmap Conversation itself, which is the
    // branch no stage of that roadmap may share.
    let (_, cut_for_the_roadmap) = holding(&written);
    assert_eq!(cut_for_the_roadmap, roadmap.branch);

    // Both companion repositories move on after this Conversation was checked
    // out, which is what tells the base rules apart.
    let ahead = [
        moved_on(Path::new(&written.repo.path)),
        moved_on(Path::new(&read.repo.path)),
    ];

    staged_and_settled(&fixture).await;

    let stage = stage_of(&fixture).await;
    let inherited = alongside(&stage, "askance").clone();
    let detached = alongside(&stage, "chronicle").clone();

    assert_eq!(
        stage
            .companions
            .iter()
            .map(|companion| (companion.repo.name.as_str(), companion.mode))
            .collect::<Vec<_>>(),
        [
            ("askance", CompanionMode::ReadWrite),
            ("chronicle", CompanionMode::ReadOnly),
        ],
        "every companion of the parent, in the mode it was in",
    );

    let (built_in, on) = holding(&inherited);

    assert_eq!(
        on, stage.branch,
        "a read-write companion's branch is named after the stage's own",
    );
    assert_ne!(
        on, roadmap.branch,
        "so no two stages of one roadmap can share a companion branch",
    );

    let (looked_in, head) = holding(&detached);

    assert_eq!(
        head, "HEAD",
        "and a read-only one is checked out detached, having nothing to commit",
    );

    // Both off the configured base as it stands now: this stage does not stack,
    // so what its checkouts come off is each repository's default branch,
    // resolved at the moment the stage started rather than when the roadmap was.
    assert_eq!(git(&built_in, &["rev-parse", "HEAD"]).trim(), ahead[0]);
    assert_eq!(git(&looked_in, &["rev-parse", "HEAD"]).trim(), ahead[1]);

    assert_eq!(
        detached.base_commit.as_deref(),
        Some(ahead[1].as_str()),
        "and the record says which commit that was, nothing else being able to",
    );

    // And the session the stage starts in is told about both of them, which is
    // how the agent finds out either is there at all.
    let prompt = until_written(&planning).await;

    assert!(
        prompt.contains(&format!(
            "- `askance` at `{}`, on branch `{}`, read-write.",
            built_in.display(),
            stage.branch,
        )),
        "the stage's first session is told where it may build: {prompt:?}",
    );
    assert!(
        prompt.contains(&format!(
            "- `chronicle` at `{}`, detached at `{}`, read-only.",
            looked_in.display(),
            ahead[1],
        )),
        "and where it may only read: {prompt:?}",
    );
}

/// Where the stage's own branch stacks, its companion branches stack too: a
/// read-write companion is in exactly the position the stage is, the predecessor
/// having committed in it with a pull request there unmerged for just as long.
///
/// So the branch is cut from the predecessor stage's companion branch in that
/// repository rather than from the companion's configured base — which is what
/// the companion repository moving on afterwards is here to tell apart.
///
/// A read-only companion has no branch to stand on anything, so a stacked stage
/// reads it exactly as an unstacked one does: detached at whatever its base
/// resolves to now.
#[tokio::test]
async fn a_stacked_stage_cuts_its_companion_branch_from_the_predecessors() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_at_pace(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, RECORDS_STACKING),
        &gh_about(GREEN, "", ""),
        *BRISKLY,
        &[
            ("askance", CompanionMode::ReadWrite),
            ("chronicle", CompanionMode::ReadOnly),
        ],
    )
    .await;

    let roadmap = fixture.view().await;
    let written = alongside(&roadmap, "askance").clone();
    let read = alongside(&roadmap, "chronicle").clone();
    let (predecessor, cut_for_the_roadmap) = holding(&written);

    let stood_on = git(&predecessor, &["rev-parse", "HEAD"]).trim().to_owned();
    let ahead = [
        moved_on(Path::new(&written.repo.path)),
        moved_on(Path::new(&read.repo.path)),
    ];

    staged_and_settled(&fixture).await;

    let stage = stage_of(&fixture).await;
    let (built_in, on) = holding(alongside(&stage, "askance"));

    assert_eq!(on, stage.branch);
    assert_ne!(on, cut_for_the_roadmap);

    assert_eq!(
        git(&built_in, &["rev-parse", "HEAD"]).trim(),
        stood_on,
        "the companion branch stands on the predecessor stage's, which is where \
         the work it builds on is",
    );
    assert_ne!(
        git(&built_in, &["rev-parse", "HEAD"]).trim(),
        ahead[0],
        "rather than on the companion's configured base, which has moved since",
    );

    let (looked_in, head) = holding(alongside(&stage, "chronicle"));

    assert_eq!(head, "HEAD");
    assert_eq!(
        git(&looked_in, &["rev-parse", "HEAD"]).trim(),
        ahead[1],
        "and a read-only companion is that repository as it stands now, stacking \
         being about branches and it having none",
    );
}

/// A companion that cannot be delivered starts no stage at all: nobody is at a
/// button to refuse, so what halts it is a notice naming the repository and what
/// git would not do.
///
/// Halted rather than built without: a stage that quietly went ahead without a
/// repository the roadmap was grilled against is a worse outcome than a stage
/// that waited. And nothing is left behind — no half-made Conversation, and no
/// branch or directory in either repository.
#[tokio::test]
async fn a_stage_whose_companion_cannot_be_delivered_starts_nothing() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_at_pace(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, ""),
        &gh_about(GREEN, "", ""),
        *BRISKLY,
        &[("askance", CompanionMode::ReadWrite)],
    )
    .await;

    // Somebody else's branch, by the name this stage's companion branch would
    // take: `rate-limiting/01-counter` is the first stage's own name, which is
    // what the stage's branch and so its companion branch are called.
    let companion = PathBuf::from(&alongside(&fixture.view().await, "askance").repo.path);
    git(&companion, &["branch", "rate-limiting/01-counter"]);

    staged_and_settled(&fixture).await;

    let said = said_by(&fixture).await;

    assert!(
        said.contains("<code>askance</code>"),
        "the repository that stopped it is named: {said:?}",
    );
    assert!(
        said.contains("already a branch of that repository"),
        "and what git would not do about it: {said:?}",
    );

    // The record the stage got as far as is closed rather than left drafting:
    // drafting is a Conversation waiting for a human to write a Brief and press
    // something, and this is a stage nobody is going to start by hand.
    let half_made = conversations(&fixture.app)
        .await
        .into_iter()
        .find(|entry| entry.id != fixture.id)
        .expect("the stage got as far as a record before git was asked anything");

    assert_eq!(
        half_made.state,
        Lifecycle::Closed,
        "no half-made stage Conversation is left running",
    );

    let closed: ConversationView = get(
        &fixture.app,
        &format!("/api/ui/conversations/{}", half_made.id),
    )
    .await;

    // Its rows say what it would have worked alongside, as any closed
    // Conversation's do — and none of them says a directory, nothing having
    // been checked out anywhere.
    assert!(
        closed.worktree.is_none()
            && closed
                .companions
                .iter()
                .all(|companion| companion.worktree.is_none()),
        "with nothing checked out anywhere: {:?}",
        (closed.worktree, closed.companions),
    );

    assert!(
        !planning.exists(),
        "so no session was launched inside the next-stage fork either",
    );
    assert!(
        !git(
            &fixture.repo(),
            &["branch", "--list", "rate-limiting/01-counter"]
        )
        .trim()
        .contains("counter"),
        "and the stage's own branch was never cut, every question being asked \
         before any of them is answered",
    );
    assert!(
        !git(&companion, &["worktree", "list"]).contains("counter"),
        "nor was anything checked out in the companion",
    );
}

/// A roadmap with every stage checked starts nothing, and the Timeline says why
/// — and the devices are told the roadmap is finished.
///
/// Which is where the whole pipeline stops of its own accord: there is no stage
/// left, so there is nothing to carry on and nothing for the human to do.
///
/// The last stage of a roadmap completing has no stage after it to be announced
/// by, so this is what says it happened. Told about the Conversation that
/// settled, because with nothing started there is no other one to open.
#[tokio::test]
async fn a_roadmap_with_every_stage_checked_starts_nothing_and_says_it_is_complete() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_roadmap_then_wraps_up(
            &planning,
            &worked,
            r#"- [x] 01: Count the requests — [brief](01-counter.md)\n- [x] 02: Refuse the rest — [brief](02-refusing.md)\n"#,
            RECORDS_STACKING,
        ),
        &gh_about(GREEN, "", ""),
    )
    .await;

    staged(&fixture).await;

    let (service, taken) = push_service().await;
    let phone = Device::new(&service, "phone");
    fixture.subscribe(&phone).await;

    let before = fixture.view().await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let said = said_by(&fixture).await;

    assert!(
        said.contains("complete"),
        "the roadmap being finished is a thing to say: {said:?}",
    );

    assert_eq!(
        conversations(&fixture.app).await.len(),
        1,
        "and nothing was started: there is no stage left to start",
    );
    assert!(
        !planning.exists(),
        "so no session was launched inside the next-stage fork either",
    );

    // The pull request, the Conversation reaching Done, and the roadmap running
    // out of stages behind it.
    let told: Vec<serde_json::Value> = pushes(&taken, 3)
        .await
        .iter()
        .map(|push| phone.read(push))
        .collect();

    let finished = told
        .iter()
        .find(|notice| notice["title"] == "The `rate-limiting` roadmap is complete")
        .unwrap_or_else(|| {
            panic!(
                "nothing said the roadmap was finished: {:?}",
                told.iter()
                    .map(|notice| &notice["title"])
                    .collect::<Vec<_>>(),
            )
        });

    assert_eq!(
        finished["path"],
        format!("/conversations/{}", fixture.id),
        "and it opens the Conversation that settled, there being no stage to open",
    );
    assert_eq!(finished["project"], before.repo.name);
}

/// A roadmap committed on the repository's default branch, as the old tools or
/// a human left it: two stages, neither of them ticked, and a brief for each.
///
/// Committed rather than merely written, because that is the whole difference
/// adoption is about — a roadmap no branch Verkstead knows ever touched, so
/// nothing it reads on its own would ever start it.
fn a_roadmap_already_committed(repo: &Path) {
    let directory = repo.join("docs/roadmaps/rate-limiting");
    std::fs::create_dir_all(&directory).unwrap();

    std::fs::write(
        directory.join("ROADMAP.md"),
        "# Rate limiting roadmap\n\n## Stages\n\n\
         - [ ] 01: Count the requests — [brief](01-counter.md)\n\
         - [ ] 02: Refuse the rest — [brief](02-refusing.md)\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("01-counter.md"),
        "# 01. Count the requests\n\n## Goal\n\nA counter per key, and nothing else.\n",
    )
    .unwrap();
    std::fs::write(directory.join("02-refusing.md"), "# 02. Refuse the rest\n").unwrap();

    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-m", "docs: the roadmap as it stands"]);
}

/// A stub that does nothing but say which session it was and where it ran, for
/// the tests about a stage that was adopted rather than staged.
///
/// Nothing here writes a plan: what is being asked is whether the press starts
/// the planning session at all, and what it is primed with.
fn plans_whatever_it_is_given(planning: &Path) -> String {
    format!(
        r#"
printf 'model=%s\nwhere=%s\n%s\n' "$1" "$(pwd)" "$2" >> {planning}
sleep 300
"#,
        planning = quoted(planning),
    )
}

/// Stand a workbench up with a roadmap the repository already holds, and press
/// Adopt on it.
///
/// The other way into the pipeline, and the shorter one: there is no Brief to
/// write and no grilling to run, so what the human settles is the two Profiles
/// and then presses once.
async fn adopting(spill: tempfile::TempDir, stub: &str) -> Grilling {
    adopting_asking(spill, stub, PULL_REQUEST).await
}

/// The same, with something else where `gh` goes — for the test that carries an
/// adopted stage the whole way to a settled wrap-up.
async fn adopting_asking(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    let bench = bench(spill, stub, gh).await;

    a_roadmap_already_committed(&bench.repo);

    let started: Started = post(
        &bench.app,
        "/api/ui/adoptions",
        &serde_json::json!({ "repo_id": bench.repo_id, "roadmap": "rate-limiting" }),
    )
    .await;
    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    bench.under_every_pairing(id).await;

    let adopted: Adopted = post(
        &bench.app,
        &format!("/api/ui/conversations/{id}/adopt"),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(adopted, Adopted::Adopted);

    bench.holding(id)
}

/// The whole of adopting: one press on a roadmap the repository already held,
/// and the stage is running as a Conversation on its own branch — with the
/// planning session the unattended path would have started, in the worktree the
/// press made.
///
/// Nothing about that session is adoption's own. It is the same fork of
/// next-stage a settling predecessor launches, under the same Profile, primed
/// with the same document — which is the point: adoption is an entry into the
/// pipeline rather than a pipeline of its own.
#[tokio::test]
async fn adopting_a_roadmap_starts_its_next_stage_with_a_planning_session() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");

    let fixture = adopting(spill, &plans_whatever_it_is_given(&planning)).await;
    let view = fixture.view().await;

    assert_eq!(
        view.branch, "rate-limiting/01-counter",
        "the branch is the stage brief's own name, under the roadmap it belongs to",
    );
    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "straight to Implementing: the human's press stands in for the stage before it",
    );

    let brief = view
        .timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Brief(brief) => Some(brief.markdown.clone()),
            _ => None,
        })
        .expect("a Conversation's first Event is its Brief");

    assert!(
        brief.contains("A counter per key, and nothing else."),
        "primed with the stage brief itself: {brief:?}",
    );

    let said = notices(&view).join("\n");

    assert!(
        said.contains("Stage 01") && said.contains("<code>rate-limiting</code>"),
        "the Timeline says which stage of which roadmap was adopted: {said:?}",
    );
    assert!(
        said.contains("<code>main</code>"),
        "and where its branch came off: {said:?}",
    );

    // The worktree git made, which is where the session is running.
    let worktree = PathBuf::from(view.worktree.expect("an adopted stage has a Worktree").path);

    assert!(
        worktree
            .join("docs/roadmaps/rate-limiting/01-counter.md")
            .exists(),
        "the branch is checked out with the roadmap it was adopted from",
    );

    // The session itself: the bundled fork of next-stage, under the
    // implementation Profile, in that worktree.
    let prompt = until_written(&planning).await;

    assert!(
        prompt.contains("model=claude-implementation-5"),
        "an adopted stage plans under the implementation Profile: {prompt:?}",
    );
    assert!(
        prompt.contains("/verkstead/skills/next-stage/SKILL.md"),
        "inside the bundled fork of next-stage: {prompt:?}",
    );
    assert!(
        prompt.contains("A counter per key, and nothing else."),
        "primed with the stage brief: {prompt:?}",
    );
    assert!(
        prompt.contains("not stacked on anything"),
        "and told its branch stands on nothing: adoption never stacks: {prompt:?}",
    );
}

/// A stub whose planning session says one thing and goes, and plans the stage
/// the second time it is run — the death that leaves a stage with no backlog,
/// and then the session Resume starts.
///
/// Which time it is is remembered outside the worktree, because what the first
/// one leaves behind is a worktree with nothing in it to remember by. Every
/// session after the planning says which skill it was and stays, which is how
/// the test tells the run carrying on from the planning being run a third time.
fn plans_nothing_then_plans(planning: &Path, worked: &Path, remembered: &Path) -> String {
    format!(
        r#"
case "$2" in
*next-stage/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {planning}

    if [ ! -f {remembered} ]; then
        printf 'once\n' > {remembered}
        printf 'the planning has nothing to say\n'
        exit 0
    fi

    printf 'planning the stage\n'
    mkdir -p .tasks
    printf '# Count the requests\n\n## Tasks\n\n- [ ] 01: count them — [details](01-count.md)\n' > .tasks/TODO.md
    printf '# 01. count them\n' > .tasks/01-count.md
    git add -A
    git commit --quiet -m 'chore: plan counter tasks'
    sleep 300
    ;;
*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {worked}
    printf 'working the stage backlog\n'
    sleep 300
    ;;
esac
"#,
        planning = quoted(planning),
        worked = quoted(worked),
        remembered = quoted(remembered),
    )
}

/// Resume on a stage whose planning session died before it committed runs the
/// planning again, rather than refusing on the backlog it never wrote.
///
/// The Conversation this half of the feature was written for. A stage's first
/// step is its own: the fork of next-stage writes the `.tasks/` everything after
/// it works through, and it is launched once, by the press or the settling that
/// made the stage. So a planning session that dies before its commit leaves a
/// Conversation implementing a backlog that does not exist, and nothing in
/// Verkstead would ever write one — the run stops on the step, and the button
/// that is supposed to unstick it used to answer that there was no backlog left
/// to work. Which was true, and the reason it was true was the one thing the
/// press could have put right.
///
/// `gh` finds no pull request here, which is what makes the reading the point:
/// an empty backlog with no pull request behind it is exactly the shape Resume
/// refuses on, so a stage that never planned has to be told apart from it before
/// GitHub is asked at all.
#[tokio::test]
async fn resuming_a_stage_that_never_planned_runs_the_planning_again() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");
    let remembered = spill.path().join("tried");

    let fixture = adopting_asking(
        spill,
        &plans_nothing_then_plans(&planning, &worked, &remembered),
        NO_PULL_REQUEST,
    )
    .await;

    // The planning ran and came to nothing, so the run stopped on the step it
    // could not see land. Waited for as the Notice the Conversation is blocked
    // on rather than as the last one written: an adopted stage says what it was
    // adopted from before anything has run in it at all.
    let stopped = fixture
        .until(|view| {
            said(view)
                .into_iter()
                .find(|notice| Some(notice.id) == view.blocked_on)
                .cloned()
        })
        .await;

    assert!(
        stopped
            .html
            .contains("Planning the roadmap stage into a backlog"),
        "the stop says which step died: {:?}",
        stopped.html,
    );

    let view = fixture.view().await;
    let worktree = PathBuf::from(view.worktree.expect("a stage has a Worktree").path);

    assert!(
        !worktree.join(".tasks").exists(),
        "and it died before it wrote a backlog, which is the whole condition",
    );
    assert!(
        view.ready_to_resume,
        "so the Conversation is standing still with the button on offer",
    );

    assert_eq!(
        fixture.resume().await,
        Resumed::Resumed,
        "and the press starts the planning again rather than refusing on the backlog",
    );

    // The proof that it is the *planning* that ran again rather than anything
    // read off an empty `.tasks/`: a second session in the fork of next-stage,
    // told what the first one was told.
    let started = until_written_saying(&worked, "next-task").await;
    let planned = std::fs::read_to_string(&planning).unwrap();

    assert_eq!(
        prompts(&planned).len(),
        2,
        "the planning ran twice and no more: {planned:?}",
    );
    assert!(
        prompts(&planned)[1].contains("/verkstead/skills/next-stage/SKILL.md"),
        "the second one is the fork of next-stage as well: {planned:?}",
    );
    assert!(
        prompts(&planned)[1].contains("not stacked on anything"),
        "and told where its branch came from, exactly as the first was: {planned:?}",
    );

    // And the run carried on from what the planning committed, which is what a
    // stage's first step landing means: the backlog is worked from here.
    assert!(
        started.contains("/verkstead/skills/next-task/SKILL.md"),
        "the backlog it wrote is being worked: {started:?}",
    );

    let view = fixture.view().await;

    assert_eq!(view.state, Lifecycle::Implementing);
    assert!(
        view.blocked_on.is_none(),
        "and nothing is waiting on the human any more: the press took the stop away",
    );
}

/// A stub that carries a roadmap stage the whole way on its own: plans it, works
/// the one task the plan wrote, finishes it, and reads the branch back finding
/// nothing worth raising.
///
/// The plan commit is the piece the chain turns on, and it is written the way
/// `/next-stage` writes one: `.tasks/`, plus the in-progress annotation naming
/// the branch it is on. That annotation is what touches the roadmap — so the
/// branch has written to `docs/roadmaps/` by the time the wrap-up settles, which
/// is the only thing the carry-on path ever looks for.
///
/// Which stage it is planning it reads off the branch it is standing on, because
/// that is the fact it has: a stage's branch is its brief's name, so the entry to
/// annotate is the one whose link names it.
fn a_stage_planned_and_worked_to_a_finish(planning: &Path) -> String {
    format!(
        r#"
case "$2" in
*next-stage/SKILL.md*)
    branch=$(git rev-parse --abbrev-ref HEAD)
    printf 'planned=%s\n' "$branch" >> {planning}
    printf 'planning the stage\n'
    mkdir -p .tasks
    printf '# The stage\n\n## Tasks\n\n- [ ] 01: do the work — [details](01-do-the-work.md)\n' > .tasks/TODO.md
    printf '# 01. do the work\n' > .tasks/01-do-the-work.md
    stage=$(basename "$branch")
    sed -i "/($stage.md)/s|\$| *(in progress: \`$branch\`)*|" docs/roadmaps/rate-limiting/ROADMAP.md
    git add -A
    git commit --quiet -m "chore: plan the $branch stage"
    sleep 300
    ;;
*next-task/SKILL.md*)
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'working %s\n' "$next"
        printf 'a counter\n' >> counter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m 'feat: count the requests'
    else
        printf 'finishing\n'
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish the stage'
        printf 'pushed, and the pull request is open\n'
    fi
    sleep 300
    ;;
*reviewing/SKILL.md*)
    printf 'I read the whole branch and found nothing worth raising\n'
    exit 0
    ;;
*)
    sleep 300
    ;;
esac
"#,
        planning = quoted(planning),
    )
}

/// The join adoption rests on: an adopted stage that settles starts the stage
/// after it, down the path a staged roadmap has always gone down.
///
/// Nothing was changed to make that happen, and that is the whole claim. Adopting
/// starts *one* stage; that stage's own plan commit writes the roadmap's
/// annotation onto its branch, which is what the carry-on path reads when the
/// wrap-up settles — so from the first stage onwards an adopted roadmap is a
/// staged one, and the entry point is all adoption ever had to be.
#[tokio::test]
async fn an_adopted_stage_that_settles_starts_the_stage_after_it() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");

    let fixture = adopting_asking(
        spill,
        &a_stage_planned_and_worked_to_a_finish(&planning),
        &gh_about(GREEN, "", ""),
    )
    .await;

    // The adopted stage runs itself the rest of the way with nothing pressed:
    // planned, worked, finished, reviewed, green.
    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert!(
        until_written_saying(&planning, "planned=rate-limiting/01-counter")
            .await
            .contains("planned=rate-limiting/01-counter"),
        "the adopted stage is the one that was planned",
    );

    let next = stage_of(&fixture).await;

    assert_eq!(
        next.branch, "rate-limiting/02-refusing",
        "the stage after the adopted one, on a branch of its own",
    );
    assert_eq!(
        next.state,
        Lifecycle::Implementing,
        "started the ordinary way, by the stage before it settling",
    );
    assert_eq!(
        next.repo.path,
        fixture.view().await.repo.path,
        "against the same Repo the roadmap was adopted out of",
    );

    let brief = next
        .timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Brief(brief) => Some(brief.markdown.clone()),
            _ => None,
        })
        .expect("a Conversation's first Event is its Brief");

    assert!(
        brief.contains("Refuse the rest"),
        "primed with the brief of the stage after the adopted one: {brief:?}",
    );

    // And the notice is the unattended path's own wording rather than adoption's:
    // nobody pressed anything for this one.
    let said = notices(&next).join("\n");

    assert!(
        said.contains("Stage 02") && said.contains("<code>rate-limiting</code>"),
        "the stage says which stage of which roadmap it is: {said:?}",
    );
    assert!(
        said.contains("with nobody asked"),
        "and that nothing was pressed for it, unlike the stage before it: {said:?}",
    );

    // The Conversation that settled says what became of it, where the human who
    // pressed Adopt would be looking.
    let carried_on = fixture
        .until(|view| {
            let said = notices(view).join("\n");

            said.contains("Stage 02").then_some(said)
        })
        .await;

    assert!(
        carried_on.contains("<code>rate-limiting/02-refusing</code>"),
        "the adopted Conversation says which stage started and on what: {carried_on:?}",
    );

    // And the session it started is the same fork of next-stage the adopted stage
    // itself was planned by, this time with nobody at the workbench at all.
    let planned = until_written_saying(&planning, "planned=rate-limiting/02-refusing").await;

    assert_eq!(
        planned.matches("planned=").count(),
        2,
        "one planning session for the adopted stage and one for the stage after \
         it: {planned:?}",
    );
}

/// A browser watching one live session's Screen: the socket it is attached
/// over, and the terminal it is painting on.
///
/// A terminal of its own rather than a list of the messages that arrived,
/// because that is what a browser *is* here — xterm.js fed a repaint and then
/// what the session printed after it. So what these assertions read is the grid,
/// which is the only claim the socket makes: a watcher ends up showing what the
/// session is showing.
struct Watcher {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    vt: Vt,
}

impl Watcher {
    /// Attach to a session's Screen the way the workbench does, and take the
    /// repaint it opens with.
    async fn attach(at: SocketAddr, conversation: i64, event: i64) -> Watcher {
        let (socket, _) = tokio_tungstenite::connect_async(format!(
            "ws://{at}/api/ui/conversations/{conversation}/screen/{event}/attach"
        ))
        .await
        .expect("the session's Screen to be attachable");

        let mut watcher = Watcher {
            socket,
            // Replaced by the repaint's own size below. A watcher does not know
            // how big the Screen is until it is told, which is why the size
            // comes with the repaint.
            vt: Vt::new(1, 1),
        };

        match watcher.shown().await {
            Shown::Painted(painted) => watcher.paint(&painted),
            shown => panic!("a socket should open with a repaint, and it said: {shown:?}"),
        }

        watcher
    }

    /// The next thing the server says, or a panic if it says nothing for long
    /// enough.
    async fn shown(&mut self) -> Shown {
        let said = tokio::time::timeout(*PATIENCE, self.socket.next())
            .await
            .expect("the server to say something")
            .expect("the socket to still be open")
            .expect("the socket to be readable");

        match said {
            Message::Text(said) => read(&said),
            said => panic!("a Screen's socket speaks JSON, and it said: {said:?}"),
        }
    }

    /// Start again on a grid the repaint's size, and paint it — which is what a
    /// terminal handed one does, a repaint saying what the whole grid is rather
    /// than what has changed about it.
    fn paint(&mut self, painted: &verkstead_render::Screen) {
        self.vt = Vt::new(usize::from(painted.columns), usize::from(painted.rows));
        self.vt.feed_str(&painted.repaint);
    }

    /// Follow the socket until the grid says something, and hand back what it is
    /// showing.
    async fn until(&mut self, enough: impl Fn(&str) -> bool) -> Vec<String> {
        let deadline = Instant::now() + *PATIENCE;

        loop {
            let showing = self.showing();

            if enough(&showing.join("\n")) {
                return showing;
            }

            assert!(
                Instant::now() < deadline,
                "the Screen never showed it. It is showing: {showing:?}"
            );

            match self.shown().await {
                Shown::Painted(painted) => self.paint(&painted),
                Shown::Printed(printed) => {
                    self.vt.feed_str(&printed);
                }
            }
        }
    }

    /// Type into it.
    ///
    /// The bytes a terminal would have made of the keys, because that is what
    /// the browser sends: xterm.js turns a keypress into what a session expects
    /// before anything crosses the socket, so a carriage return here is what a
    /// press of Enter is there.
    async fn types(&mut self, keys: &str) {
        self.puts_in(keys, "the socket to take a keystroke").await;
    }

    /// Move the mouse over it, which goes up the socket as the same kind of
    /// thing.
    ///
    /// The report a terminal makes of a move, a click or a scroll, because that
    /// is what the browser sends: a session whose interface tracks the mouse is
    /// sent one of these per movement over its Screen.
    async fn mouses(&mut self, report: &str) {
        self.puts_in(report, "the socket to take a mouse report")
            .await;
    }

    /// Either of those: one kind of watcher input, on its way to the session's
    /// own terminal.
    async fn puts_in(&mut self, input: &str, took_it: &str) {
        let said = serde_json::to_string(&Watching::PutIn(input.to_owned())).unwrap();

        self.socket
            .send(Message::Text(said.into()))
            .await
            .expect(took_it);
    }

    /// This watcher's window is now that big — which is the Screen's size from
    /// here on, for everybody watching it.
    async fn resize(&mut self, columns: u16, rows: u16) {
        let said = serde_json::to_string(&Watching::Resized(Size { columns, rows })).unwrap();

        self.socket
            .send(Message::Text(said.into()))
            .await
            .expect("the socket to take a resize");
    }

    /// The grid it is showing, row by row, with the blank rows at the bottom
    /// left off.
    fn showing(&self) -> Vec<String> {
        let mut rows: Vec<String> = self
            .vt
            .view()
            .map(|line| line.text().trim_end().to_owned())
            .collect();

        while rows.last().is_some_and(|row| row.is_empty()) {
            rows.pop();
        }

        rows
    }
}

/// Everybody watching one session is watching the one Screen — and a watcher
/// that goes away and comes back is put where the others already are.
///
/// The whole of what a repaint is for. A session prints for an hour and a
/// browser opened at the end of it has seen none of that hour, so what it is
/// handed on connect is not what happens next but what the grid *is*.
#[tokio::test]
async fn everybody_watching_one_session_sees_the_one_screen() {
    let fixture = grilling(
        r#"
        printf 'reading the brief\r\n'
        printf 'thinking\rthinking… done\r\n'
        while :; do sleep 0.05; done
        "#,
    )
    .await;

    let event = fixture.running().await;
    let at = fixture.listening().await;

    let mut one = Watcher::attach(at, fixture.id, event).await;
    let mut two = Watcher::attach(at, fixture.id, event).await;

    let showing = one.until(|grid| grid.contains("thinking… done")).await;

    assert_eq!(
        two.until(|grid| grid.contains("thinking… done")).await,
        showing,
        "two browsers on one session should be showing the one grid",
    );

    // The watcher goes — a closed tab, a phone that slept, a connection that
    // dropped — and comes back to find the session exactly where it was.
    drop(one);

    let again = Watcher::attach(at, fixture.id, event).await;

    assert_eq!(
        again.showing(),
        showing,
        "a watcher that reconnects should be repainted with what the others are showing",
    );
}

/// The size reaches the session's own terminal, so its interface redraws to fit.
///
/// Read back off the session rather than off the Screen, because that is the
/// claim: a grid made wider on the server and nowhere else would be a Screen the
/// session never heard about and went on drawing a hundred columns into. The
/// stub asks the terminal underneath it how big it is, which is what a terminal
/// application does when it is told the window changed.
#[tokio::test]
async fn resizing_a_watchers_window_resizes_the_session() {
    let fixture = grilling(
        r#"
        trap 'stty size' WINCH
        stty size
        while :; do sleep 0.05; done
        "#,
    )
    .await;

    let event = fixture.running().await;
    let at = fixture.listening().await;

    let mut watcher = Watcher::attach(at, fixture.id, event).await;

    // The size it started on, which is the terminal Verkstead opened for it.
    watcher.until(|grid| grid.contains("30 100")).await;

    watcher.resize(132, 43).await;

    let showing = watcher.until(|grid| grid.contains("43 132")).await;

    assert!(
        showing.iter().any(|row| row.contains("43 132")),
        "the session should have been told its window is now 132 by 43, \
         and the Screen is showing: {showing:?}",
    );
}

/// Watching commits the human to nothing: no Event, no move, and nothing about
/// the run different for somebody having looked.
///
/// The Timeline records the work rather than the watching — the carve-out is
/// written into its glossary entry — and this is the whole of what that means in
/// code. A watcher attaches, sees the session, and goes; what is left behind is
/// the Timeline that was there before.
#[tokio::test]
async fn watching_a_session_leaves_nothing_behind() {
    let fixture = grilling(
        r#"
        printf 'reading the brief\r\n'
        while :; do sleep 0.05; done
        "#,
    )
    .await;

    let event = fixture.running().await;
    let before = fixture.view().await;
    let at = fixture.listening().await;

    {
        let mut watcher = Watcher::attach(at, fixture.id, event).await;
        watcher
            .until(|grid| grid.contains("reading the brief"))
            .await;
    }

    // Read after the socket has gone rather than while it is open, because the
    // claim is about closing one as well as opening it.
    let after = fixture.view().await;

    assert_eq!(after.state, before.state, "watching moved the Conversation");
    assert_eq!(
        after.timeline, before.timeline,
        "watching left something on the Timeline",
    );
}

/// A session that has ended has no socket to attach to.
///
/// Its Screen is the one the details pane fetches — the screen it last stood on,
/// read-only — and there is nowhere for a keystroke to go, because there is no
/// terminal at the other end of one. Refused rather than opened and left silent:
/// a socket that connected to nothing would be a browser waiting for a session
/// that had already gone.
#[tokio::test]
async fn a_session_that_has_ended_has_no_screen_to_attach_to() {
    let fixture = grilling("printf 'and out\\r\\n'").await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    let at = fixture.listening().await;

    let refused = tokio_tungstenite::connect_async(format!(
        "ws://{at}/api/ui/conversations/{}/screen/{event}/attach",
        fixture.id
    ))
    .await;

    let Err(tokio_tungstenite::tungstenite::Error::Http(response)) = refused else {
        panic!("attaching to a session that has ended should be refused, and it was not");
    };

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// The gate a stub waits at, so that a test can type into a session before it
/// does the thing being watched for.
///
/// A file in the spill directory, which every sandbox here gets read-write:
/// there is no other way to make a session pause where a test wants it, and the
/// timing this is standing in for is the whole of what these are about. Nothing
/// waits at it that the test does not open.
fn a_backlog_held_at(gate: &Path, then: &str) -> String {
    format!(
        r#"
case "$1" in
claude-grilling-5)
    printf 'breaking down\r\n'
    while [ ! -f {gate} ]; do sleep 0.05; done
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01. Count the requests\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    printf 'the backlog has landed\r\n'
    {then}
    ;;
*)
    printf 'working the task\r\n'
    printf 'a limiter\n' >> limiter.md
    sed -i "s/- \[ \] 01:/- [x] 01:/" .tasks/TODO.md
    git add -A
    git commit --quiet -m 'feat: count the requests'
    sleep 300
    ;;
esac
"#,
        gate = quoted(gate),
        then = then,
    )
}

/// Take a Conversation as far as its grilling session breaking the work down at
/// `gate`, with somebody typing into its Screen: the session has not committed
/// anything yet, and a keystroke has reached it.
///
/// The breakdown is the grilling session's own — a task-list pick leaves the
/// session that proposed to write the backlog, and the run picks up from the
/// backlog landing. So there is one session here to attach to, and it is the
/// first.
///
/// The two tests below each start here and part company at what the session
/// does once the gate opens.
async fn typed_into_at_the_breakdown(fixture: &Grilling, at: SocketAddr) -> (Watcher, i64) {
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let event = fixture.attachable(1).await;
    let mut watcher = Watcher::attach(at, fixture.id, event).await;

    watcher.until(|grid| grid.contains("breaking down")).await;
    watcher.types("\r").await;

    (watcher, event)
}

/// What a watcher puts into a live Screen reaches the session, whichever hand it
/// came from, and commits Verkstead to nothing.
///
/// Keystrokes and mouse reports go up the one socket as the one kind of thing:
/// by the time either is on the wire it is bytes a terminal is being sent, and
/// bytes are what arrive. A session whose interface tracks the mouse is sent a
/// report of every move over its Screen down the path a keystroke takes, and an
/// interface that draws a cursor and never sees one is broken.
///
/// Read back off the session rather than off the Screen, for the reason a resize
/// is: the claim is that what was put in arrives at the terminal the agent is
/// running on, and a Screen that had drawn the typing itself would show the same
/// thing having reached nobody. So the stub reads its terminal a byte at a time
/// and prints what it read with the escapes made visible.
#[tokio::test]
async fn what_a_watcher_puts_in_reaches_the_session_and_commits_nothing() {
    let fixture = grilling(
        r#"
        stty -icanon -echo min 1 time 0
        printf 'ready
'
        cat -v
        "#,
    )
    .await;

    let event = fixture.running().await;
    let at = fixture.listening().await;

    let mut watcher = Watcher::attach(at, fixture.id, event).await;
    watcher.until(|grid| grid.contains("ready")).await;

    let before = fixture.view().await;
    assert!(
        before.blocked_on.is_none(),
        "nothing is waiting on the human yet",
    );

    watcher.types("hello\r").await;

    let showing = watcher.until(|grid| grid.contains("hello")).await;
    assert!(
        showing.iter().any(|row| row.contains("hello")),
        "the session should have read what was typed: {showing:?}",
    );

    // And a cursor crossing the pane, as a terminal reports one.
    watcher.mouses("\u{1b}[<35;12;24M").await;

    let showing = watcher.until(|grid| grid.contains("^[[<35;12;24M")).await;
    assert!(
        showing.iter().any(|row| row.contains("^[[<35;12;24M")),
        "the session should have read the mouse report: {showing:?}",
    );

    // Long enough for something to have been registered if anything were going
    // to be.
    pause(Duration::from_secs(1)).await;

    let after = fixture.view().await;
    assert!(
        after.blocked_on.is_none(),
        "putting something into a Screen blocked the Conversation on the human",
    );
    assert_eq!(
        after.state, before.state,
        "and it moved the Conversation as well",
    );
    assert_eq!(
        after.timeline.len(),
        before.timeline.len(),
        "and left something on the Timeline: the Timeline records the work \
         rather than the watching",
    );
}

/// Typing into a driven session's Screen changes nothing about when it ends.
///
/// The step lands while somebody is typing into it, which is exactly the moment
/// the runner ends the session and launches the next one — and it does both,
/// because a keystroke commits Verkstead to nothing. Somebody who wants the run
/// held off presses Stop first, and the stop is what holds it.
#[tokio::test]
async fn typing_into_a_driven_session_changes_nothing_about_when_it_ends() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");

    let fixture =
        grilling_spilling(spill, &a_backlog_held_at(&gate, "sleep 300"), PULL_REQUEST).await;

    let at = fixture.listening().await;
    let (_watcher, event) = typed_into_at_the_breakdown(&fixture, at).await;

    // The step lands, and the session goes quiet sitting on its `sleep`.
    std::fs::write(&gate, "go").unwrap();

    // And the run picks up behind it, with nothing pressed.
    //
    // The second commit rather than the second of exactly two. The run does not
    // stop where the task does: the finish step launches behind it and this
    // stub commits in that one too, so a Timeline holding exactly two is a
    // moment between the two sessions rather than anything the run settles at.
    // A machine slow enough to poll straight past that moment would never see
    // the count it was waiting for, and the commit it is really waiting on is
    // there in either read.
    let landed = fixture
        .until(|view| {
            let landed = commits(view);
            (landed.len() >= 2).then(|| landed[1].subject.clone())
        })
        .await;

    assert_eq!(landed, "feat: count the requests");

    let view = fixture.view().await;
    assert!(
        outputs(&view)
            .iter()
            .any(|output| output.id == event && !output.running),
        "the session that was typed into was ended by the ordinary rules: {:?}",
        outputs(&view),
    );
}

/// And the other half of the same rule: a session typed into that lands nothing
/// stops the run, at once and with nothing pressed.
///
/// An inline implementation, because that is the shortest run with an
/// end-of-session judgement in it: what says it did anything is what it
/// committed and what is on the branch, and this one leaves neither — hence the
/// `gh` that finds no pull request, without which a session that committed
/// nothing would still have carried the work to one.
#[tokio::test]
async fn typing_into_a_session_that_lands_nothing_does_not_hold_the_halt_off() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");

    let fixture = grilling_spilling(
        spill,
        &format!(
            r#"
case "$1" in
claude-grilling-5)
    printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
    printf 'the handoff is written\r\n'
    sleep 300
    ;;
*)
    printf 'implementing\r\n'
    while [ ! -f {gate} ]; do sleep 0.05; done
    printf 'and out with nothing to show\r\n'
    ;;
esac
"#,
            gate = quoted(&gate),
        ),
        NO_PULL_REQUEST,
    )
    .await;

    let at = fixture.listening().await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    let event = fixture.attachable(2).await;
    let mut watcher = Watcher::attach(at, fixture.id, event).await;

    watcher.until(|grid| grid.contains("implementing")).await;
    watcher.types("\r").await;

    std::fs::write(&gate, "go").unwrap();

    let stopped = fixture.stopped().await;
    assert!(
        stopped.html.contains("without committing anything"),
        "the ordinary rules judged what was left, unasked: {:?}",
        stopped.html,
    );
}

/// And nothing a watcher does reaches a device.
///
/// Watching a Screen and putting something into one are the same to Verkstead:
/// neither stops the run, so neither is news. What the devices hear about is a
/// stop, and a keystroke is not one.
#[tokio::test]
async fn nothing_a_watcher_puts_in_tells_the_devices() {
    let fixture = grilling(
        r#"
        printf 'reading the brief\r\n'
        while :; do sleep 0.05; done
        "#,
    )
    .await;

    let event = fixture.running().await;
    let at = fixture.listening().await;

    let (service, taken) = push_service().await;
    fixture.subscribe(&Device::new(&service, "phone")).await;

    let mut watcher = Watcher::attach(at, fixture.id, event).await;
    watcher
        .until(|grid| grid.contains("reading the brief"))
        .await;

    watcher.types("\r").await;
    watcher.mouses("\u{1b}[<35;12;24M").await;

    // Long enough for a notification to have gone out if anything here sent
    // one, and for the socket dropping to have been noticed as well.
    drop(watcher);
    pause(Duration::from_secs(3)).await;

    assert!(
        taken.lock().unwrap().is_empty(),
        "a device was told about somebody typing into a Screen",
    );
}

/// A stop Verkstead decided on reaches the human's devices, once.
///
/// The reason a stop is pushed at all: nobody is at the terminal, the run will
/// not start again until Resume is pressed, and a stop nobody is told about is
/// one found days late. So the phone gets what the Notice opens with — which
/// step stopped, on which branch, in which Repo — and tapping it opens the
/// Conversation the Notice is on.
///
/// Then a restart, which sweeps every Conversation as it comes up: what it
/// finds here is one that has already stopped, and it leaves it alone. That is
/// the second half of this — a Conversation stops once, and one push is what
/// one stop is worth however many times something notices it standing still.
#[tokio::test]
async fn a_halt_verkstead_decided_on_tells_the_devices_once() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            printf 'half a limiter\n' > limiter.md
            printf 'error: unresolved import crate::window\n'
            exit 1
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let (service, taken) = push_service().await;
    let phone = Device::new(&service, "phone");
    let laptop = Device::new(&service, "laptop");
    fixture.subscribe(&phone).await;
    fixture.subscribe(&laptop).await;

    let before = fixture.view().await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "Verkstead pulled the brake, which is the kind of stop worth a phone",
    );

    // Four: the proposal Set arriving is a push of its own to each device, and
    // the stop is what follows it, so the stop's is the second of each pair.
    let pushed = pushes(&taken, 4).await;

    for device in [&phone, &laptop] {
        let push = pushed
            .iter()
            .filter(|push| push.device == device.name)
            .next_back()
            .unwrap_or_else(|| panic!("{} was not told the run had stopped", device.name));

        // Decrypted with the device's own keys, which is what says it was
        // encrypted for that device rather than merely addressed to it.
        let notice = device.read(push);

        assert_eq!(
            notice["path"],
            format!("/conversations/{}", fixture.id),
            "a stop's push has to open the Conversation that stopped",
        );
        assert_eq!(
            notice["title"],
            format!("Implementing the work inline stopped on {}", before.branch),
            "and say what stopped and which piece of work it stopped in — the \
             words the Notice opens with",
        );
        assert_eq!(notice["project"], before.repo.name);
    }

    assert!(
        stopped.html.contains("Implementing the work inline"),
        "the Notice and the push name the same stop: {:?}",
        stopped.html,
    );

    // A server coming back sweeps every Conversation before it does anything
    // else, and this one has already stopped.
    let _restarted = fixture.restarted("true", PULL_REQUEST).await;

    // Long enough for that sweep to have looked, and to have said something if
    // it were going to.
    pause(Duration::from_secs(3)).await;

    assert_eq!(
        notices(&fixture.view().await).len(),
        1,
        "the first Notice is the one that explains the stop",
    );
    assert_eq!(
        taken.lock().unwrap().len(),
        4,
        "the two the Set was worth and the two the stop was: one push per device \
         per stop, and a Conversation stops once",
    );
}

/// And a stop nobody chose tells nobody.
///
/// A stall is a driver that went away rather than a decision anybody took, so a
/// Verkstead coming back is free to start the work again unasked — and waking a
/// phone about a run that a restart will carry on by itself is a notification
/// that asks for nothing. The Notice is still written, because the Timeline is
/// the record either way.
///
/// The stall is arranged rather than waited for — see [`wrapping_unwatched`] —
/// so that the device is on the list before there is anything to push about it.
#[tokio::test]
async fn a_stop_nobody_chose_tells_nobody() {
    let fixture = grilling_swept(
        r#"
        printf 'the grilling is thinking it over\n'
        sleep 300
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let (service, taken) = push_service().await;
    fixture.subscribe(&Device::new(&service, "phone")).await;

    assert!(
        notices(&fixture.view().await).is_empty(),
        "the grilling session is running, so nothing is standing still yet",
    );

    // A wrap-up is driven by its watchers alone, so a Conversation moved into
    // one with none started is standing still the moment it arrives — which the
    // sweep this server runs finds on its next look.
    wrapping_unwatched(&fixture).await;

    fixture.stopped().await;

    assert_eq!(
        fixture.chosen().await,
        Decision::Circumstance,
        "nobody decided this run should stop, which is the whole reason it is \
         not worth a phone",
    );

    // Long enough for the push to have gone out if a stop nobody chose were one
    // to send.
    pause(Duration::from_secs(3)).await;

    assert!(
        taken.lock().unwrap().is_empty(),
        "a run a restart will pick up unasked is not one to wake anybody about",
    );
}

/// One push, as the push service received it.
#[derive(Debug, Clone)]
struct Push {
    /// The path it was posted to, which is the device it was meant for.
    device: String,
    body: Vec<u8>,
}

/// A push service on a loopback port, and the pushes it has taken.
///
/// It takes everything it is sent. What the vendors' services answer, and what
/// Verkstead makes of each answer, is `push_delivery.rs`'s: here the question is
/// only whether a Hold left standing is one they hear about.
async fn push_service() -> (String, Arc<Mutex<Vec<Push>>>) {
    let taken: Arc<Mutex<Vec<Push>>> = Arc::new(Mutex::new(Vec::new()));

    let app = Router::new().route(
        "/{device}",
        axum::routing::post({
            let taken = taken.clone();
            move |axum::extract::Path(device): axum::extract::Path<String>,
                  body: axum::body::Bytes| {
                let taken = taken.clone();
                async move {
                    taken.lock().unwrap().push(Push {
                        device,
                        body: body.to_vec(),
                    });
                    StatusCode::CREATED
                }
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("a port to listen on");
    let at = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (format!("http://{at}"), taken)
}

/// A device as its browser would have described it, plus the private half the
/// browser would have kept — which is what lets a test read a push back.
struct Device {
    name: String,
    endpoint: String,
    secret: p256::SecretKey,
    auth: Vec<u8>,
}

impl Device {
    fn new(service: &str, name: &str) -> Device {
        // Deterministic rather than random, so a failure names the same device
        // twice running. Sixteen bytes because that is what the auth secret is.
        let auth: Vec<u8> = name.bytes().cycle().take(16).collect();

        Device {
            name: name.to_owned(),
            endpoint: format!("{service}/{name}"),
            secret: p256::SecretKey::random(&mut p256::elliptic_curve::rand_core::OsRng),
            auth,
        }
    }

    /// The public half, in the encoding `PushManager.subscribe` hands back.
    fn p256dh(&self) -> String {
        use p256::elliptic_curve::sec1::ToEncodedPoint;

        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(self.secret.public_key().to_encoded_point(false).as_bytes())
    }

    fn auth(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.auth)
    }

    /// Read a push meant for this device, as its service worker would.
    fn read(&self, push: &Push) -> serde_json::Value {
        let plain = web_push_native::decrypt(
            push.body.clone(),
            &self.secret,
            &web_push_native::Auth::clone_from_slice(&self.auth),
        )
        .expect("a push for this device has to decrypt with this device's keys");

        serde_json::from_slice(&plain).expect("the notice has to be JSON")
    }
}

/// Wait until the push service has taken `count` pushes, then hand them over.
async fn pushes(taken: &Arc<Mutex<Vec<Push>>>, count: usize) -> Vec<Push> {
    let deadline = Instant::now() + *PATIENCE;

    loop {
        {
            let pushed = taken.lock().unwrap();
            if pushed.len() >= count {
                return pushed.clone();
            }
            assert!(
                Instant::now() < deadline,
                "waited for {count} pushes and {} arrived",
                pushed.len(),
            );
        }

        pause(Duration::from_millis(20)).await;
    }
}

/// A Conversation in a driven state with nothing driving it is **Stalled**, and
/// the sweep is what says so.
///
/// The grilling session here has printed and exited, which is exactly the shape
/// of the bug this is for: the Conversation still says it is being grilled,
/// nothing is grilling it, and nothing on the page offers the human anything.
/// What the sweep records is a stop, and what it says about it is an ordinary
/// Notice — so the badge has something to point at and the human has something
/// to read.
#[tokio::test]
async fn a_conversation_nothing_is_driving_is_halted_while_the_server_runs() {
    let fixture = grilling_swept(r#"printf 'the grilling has nothing to say\n'"#).await;

    fixture.quiet().await;

    let stalled = fixture.stopped().await;

    assert!(
        stalled.html.contains("Grilling the work"),
        "what ought to have been happening, which for a stall is what the state \
         says: {:?}",
        stalled.html,
    );
    assert!(
        stalled.html.contains(
            "nothing is driving it: no session is running, and nothing is left to start one"
        ),
        "and nothing failed, because nothing was ever launched: {:?}",
        stalled.html,
    );
    let view = fixture.view().await;

    assert_eq!(
        view.blocked_on,
        Some(stalled.id),
        "so the Conversation is blocked on the human, which is the whole point: a \
         stall is precisely the condition that had no badge on it",
    );
    assert_eq!(
        view.state,
        Lifecycle::Grilling,
        "and it is still where it was — a stall is a condition an active state is \
         in rather than a state of its own",
    );

    // Long enough for many more sweeps. A Conversation is stopped once, and a
    // stall that goes on standing still is the same stall.
    tokio::time::sleep(SWEEPING.stalls * 8).await;

    assert_eq!(
        notices(&fixture.view().await).len(),
        1,
        "a Conversation standing still for a while is one thing to read, not one \
         a minute",
    );
}

/// And a Conversation left mid-run by a server that stopped is not one the sweep
/// ever gets to.
///
/// The case the sweep would matter most for, if the restart did not answer it
/// first. No driver survives the process, so a restarted server holds no
/// registrations at all — which is the truth about it, and would make every
/// Conversation it was left driving a stall. What puts that right is the
/// restart's own resume, and the sweep waits for it: it runs as soon as that is
/// done, and finds every Conversation it took up being driven.
///
/// So there is nothing to read afterwards, which is the assertion. A Notice here
/// would be a Conversation told it had stopped a moment after being started
/// again.
#[tokio::test]
async fn a_conversation_a_stopped_server_left_mid_run_is_driven_again_before_the_sweep_looks() {
    let fixture = grilling(r#"printf 'the grilling has nothing to say\n'"#).await;

    fixture.quiet().await;

    assert!(
        notices(&fixture.view().await).is_empty(),
        "the server it started on sweeps too slowly to have looked, which is what \
         makes the restart the thing being tested",
    );

    let before = outputs(&fixture.view().await).len();

    let _restarted = fixture
        .restarted(
            r#"
            printf 'the grilling is running again\n'
            sleep 300
            "#,
            PULL_REQUEST,
        )
        .await;

    // The sweep the second server runs the moment its resume is done has looked
    // by the time there is a session, that being the slower of the two.
    fixture
        .until(|view| (outputs(view).len() > before).then_some(()))
        .await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing was written about a Conversation that is being worked on: {:?}",
        notices(&view),
    );
    assert_eq!(
        view.blocked_on, None,
        "and nothing is waiting on the human, because nothing had to ask them",
    );
}

/// A state nothing is supposed to be driving is never one standing still.
///
/// Draft and Direction are waiting on the human, Done is finished and Closed is
/// stopped. A sweep that stopped those would be telling the human about every
/// Conversation they have ever had.
#[tokio::test]
async fn a_conversation_nothing_is_supposed_to_be_driving_is_never_halted() {
    let fixture = grilling_swept(r#"printf 'the grilling has nothing to say\n'"#).await;

    fixture.quiet().await;
    fixture.stopped().await;

    assert_eq!(fixture.close().await, ConversationClosed::Closed);

    let said = notices(&fixture.view().await).len();

    // Long enough for many sweeps over a Conversation that has stopped.
    tokio::time::sleep(SWEEPING.stalls * 8).await;

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Closed,
        "the work stopped where it was, which is not the same as standing still in it",
    );
    assert_eq!(
        notices(&view).len(),
        said,
        "so nothing more was said about it: {:?}",
        notices(&view),
    );
}

/// An inline run that lands its work and leaves no pull request is sent back for
/// one, and halts where that leaves none either — with the reason `gh` gave and
/// the Worktree it stopped in on the Timeline.
///
/// The inline half of what a finish step's missing pull request does — see
/// [`a_finish_that_opened_no_pull_request_is_sent_back_for_one`]. The session
/// committed and exited without ever pushing, which is precisely the ending
/// nothing used to notice: the run went quiet in Implementing and stayed there.
/// Every run here ends on a pull request the same way, so every one of them gets
/// the same go at the one thing missing, and a Conversation that still cannot be
/// moved on is one the human is told about.
///
/// The evidence is both halves of it: what git makes of the Worktree, and the
/// tail of what the last session said — which is where the reason there is still
/// no pull request is written down, and is why each session's own Timeline Event
/// goes to the wrap-up with it. The last session is the one sent for the pull
/// request, so what the Notice carries is its account rather than the builder's:
/// the run's question by then is why the push did not happen, and that session is
/// the one that tried it.
#[tokio::test]
async fn an_inline_run_that_opened_no_pull_request_leaves_the_conversation_where_it_is() {
    let fixture = grilling_asking(
        r#"
        case "$2" in
        *submitting/SKILL.md*)
            printf 'prompt was: %s\n' "$2"
            printf 'gh is not logged in here, so there is no pull request to open\n'
            exit 1
            ;;
        esac

        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            printf 'the limiter is in, the middleware is not\n'
            printf 'a limiter\n' >> limiter.md
            git add limiter.md
            git commit --quiet -m 'feat: rate limiting'
            printf 'and a note to self\n' > notes.md
            ;;
        esac
        "#,
        NO_PULL_REQUEST,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("pull request"),
        "the step is named as what it was: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("no pull request"),
        "and the reason is `gh`'s, in words: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("notes.md"),
        "with what git makes of the Worktree it stopped in: {:?}",
        stopped.html,
    );
    assert_eq!(
        sessions_on(&fixture, "submitting/SKILL.md").await,
        1,
        "and the stop is on the far side of the one go rather than instead of it",
    );
    assert!(
        stopped
            .html
            .contains("gh is not logged in here, so there is no pull request to open"),
        "and the tail of what the last session said, which is the one sent for the \
         pull request and the one that knows why there is none: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "what is missing is out here rather than in a driver that went away, so a \
         restart looking again would find the same missing thing",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "the work is where it was, because nothing about it got any further",
    );
    assert!(pull_request(&view).is_none(), "and nothing is pinned");
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "what is waiting is the human, which is what the badge is for",
    );
}

/// And a wrap-up nothing is watching stops as one, in the words wrapping up
/// uses.
///
/// The third of the driven states, and the one that has nothing to read a
/// session's last words off: a wrap-up starts no session of its own, so what the
/// human is told is the state it stopped in and the Worktree it stopped over.
///
/// The condition is arranged rather than provoked. Every route into Wrapping
/// starts the watchers, so the only way to have a wrap-up nothing is watching is
/// to make the move the way the store makes it and leave them unstarted — which
/// is precisely what a wrap-up whose watchers have all died looks like.
///
/// The grilling session runs on through it, which is what keeps the Conversation
/// off the sweep until the move: a grilling is driven by its session, and a
/// wrap-up is driven by watchers that were never started.
#[tokio::test]
async fn a_wrap_up_nothing_is_watching_halts_as_one() {
    let fixture = grilling_swept(
        r#"
        printf 'the grilling is thinking it over\n'
        sleep 300
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    assert!(
        notices(&fixture.view().await).is_empty(),
        "the grilling session is running, so nothing is standing still yet",
    );

    wrapping_unwatched(&fixture).await;

    let stalled = fixture.stopped().await;

    assert!(
        stalled.html.contains("Wrapping the work up"),
        "the Conversation says it is wrapping up and nothing is watching it: {:?}",
        stalled.html,
    );

    let view = fixture.view().await;

    assert_eq!(view.blocked_on, Some(stalled.id));
}

/// Stop a Conversation the way a stall does, with the ordinary Notice on its
/// Timeline saying what nothing was doing — and stopped by whichever of the
/// four words is being asked about, rather than by waiting for a sweep to find
/// it.
///
/// The record is written rather than provoked, for [`wrapping_unwatched`]'s
/// reason: what these tests are about is what the *next* server makes of a
/// stored word, and a sweep left running would write a second stop over the top
/// of what it was watching. It is also the only way to have a `deliberate` one
/// at all — nothing writes that word any more, and what these tests are asking
/// is what a database written before it stopped being written still does.
async fn halted_by(fixture: &Grilling, decision: Decision) {
    let pool = open_database(&fixture.database).await.unwrap();

    let written = verkstead_store::stop(
        &pool,
        fixture.id,
        decision,
        "**Grilling the work** stopped.\n\nnothing is driving it: no session is \
         running, and nothing is left to start one\n\n### The worktree\n\nGit had \
         nothing pending, or the repository would not answer.\n\n### What the last \
         session said\n\n    the grilling has nothing to say\n",
        None,
    )
    .await
    .unwrap();

    pool.close().await;

    assert!(
        written.is_some(),
        "the Conversation had not stopped already, which is what makes this the \
         one stop on it",
    );
}

/// Move a Conversation into Wrapping the way its finish step's pull request
/// does, and start none of the watchers that ordinarily follow.
///
/// The store's own move, which is the half [`crate::wrapping::opened`] does
/// before it starts watching anything — so what this leaves behind is a
/// Conversation wrapping up with nothing watching it, which is what a wrap-up
/// whose watchers have all died is.
async fn wrapping_unwatched(fixture: &Grilling) {
    let pool = open_database(&fixture.database).await.unwrap();

    let repo = verkstead_server::store::load_conversation(&pool, fixture.id)
        .await
        .unwrap()
        .unwrap()
        .repo
        .id;

    let recorded = verkstead_server::store::record_pull_request(
        &pool,
        fixture.id,
        repo,
        &verkstead_server::store::PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();
    pool.close().await;

    assert_eq!(recorded, verkstead_server::store::Wrapping::Started);
}

/// Resume on a stalled backlog run picks the backlog up again, reading what is
/// next off `.tasks/` exactly as the runner always does.
///
/// What is next is the repository's to say and has not changed on account of
/// nothing having been running: the task whose session died is still there, so
/// that is the task the fresh session is started on. Nothing here reverts
/// anything, and nothing reads the step off whatever stopped — a stop is
/// answered whenever the human gets to it, and the branch is what has the
/// answer by then.
#[tokio::test]
async fn resuming_a_stalled_backlog_run_takes_the_next_task_off_the_repository() {
    let fixture = grilling_swept(
        r#"
        case "$1" in
        claude-grilling-5)
            mkdir -p .tasks
            printf '# Rate limiting\n\n- [ ] 01: Count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            git add .tasks
            git commit --quiet -m 'chore: plan the rate limiter'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            if [ ! -f TRIED ]; then
                printf 'once\n' > TRIED
                printf 'this task is beyond me\n'
                exit 1
            else
                printf 'prompt was: %s\n' "$2"
                sleep 300
            fi
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains(".tasks/01-count.md"),
        "the run stopped at the task whose session died: {:?}",
        stopped.html,
    );

    let before = outputs(&fixture.view().await).len();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let relaunched = fixture
        .until(|view| {
            let running = outputs(view);
            (running.len() > before).then(|| running[before].id)
        })
        .await;

    let said = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id == relaunched && output.lines > 1)
                .map(|output| output.id)
        })
        .await;

    let printed = fixture.capture(said).await.replace("\r\n", "\n");

    assert!(
        printed.contains("/verkstead/skills/next-task/SKILL.md"),
        "the run picks the backlog up again, which is the fork that reads it: {printed:?}",
    );
    assert!(
        printed.contains("The API has none."),
        "primed with the Brief the work started from: {printed:?}",
    );
    assert!(
        !printed.contains("What I said when I asked you to try this again"),
        "and told nothing beside it: Resume carries no note, steering being what \
         carries one: {printed:?}",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.blocked_on, None,
        "nothing is waiting on the human once the Conversation is being driven again",
    );
    assert!(
        !view.ready_to_resume,
        "and there is nothing left to resume, because something is driving it",
    );
    assert_eq!(
        notices(&view).len(),
        1,
        "while the Notice stays where it is: it is a stop that really happened",
    );

    let worktree = PathBuf::from(view.worktree.unwrap().path);

    assert!(
        worktree.join(".tasks/01-count.md").exists(),
        "and it is the same task, because nothing reverted anything",
    );
}

/// The inline stub the three Resume tests below run on: a grilling that writes
/// its handoff and waits, a session that builds the work and exits, and a review
/// that reads the branch and stays up.
///
/// Every branch prints the prompt it was run on, which is what makes *how many
/// sessions were spent* a question the Timeline can answer: a session is an
/// implementing one if its prompt sends it into the implementing skill.
///
/// The review stays up on purpose. What the first of the three asks about is a
/// Conversation that has reached Wrapping, and a review that finished would let
/// the wrap-up settle out from under the assertion.
const AN_INLINE_RUN: &str = r#"
printf 'prompt was: %s\n' "$2"

case "$2" in
*reviewing/SKILL.md*)
    printf 'reading the whole branch\n'
    sleep 300
    ;;
*implementing/SKILL.md*)
    printf 'a limiter\n' > limiter.md
    git add limiter.md
    git commit --quiet -m 'feat: rate limiting'
    printf 'the limiter is in, the middleware is not\n'
    ;;
*)
    printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
    printf 'the handoff is written\n'
    sleep 300
    ;;
esac
"#;

/// How many of a Conversation's sessions were implementing ones, read off what
/// each of them was run on.
///
/// The count the Resume tests are really about: what a pull request already on
/// the branch saves is an account, so *no session spent* has to be asserted as a
/// number rather than as a state that happens to look right.
async fn implementing_sessions(fixture: &Grilling) -> usize {
    sessions_on(fixture, "implementing/SKILL.md").await
}

/// The same count for a backlog's working sessions, which run on the other
/// skill: a task list spends an account per task rather than one on the whole of
/// the work.
async fn working_sessions(fixture: &Grilling) -> usize {
    sessions_on(fixture, "next-task/SKILL.md").await
}

/// How many of a Conversation's sessions were started on `skill`.
///
/// Counted by what each was run on rather than by how many there are, because a
/// Conversation's sessions are not all the work's: a wrap-up reads the branch
/// and answers comments in sessions of its own, and those are not what a *no
/// session spent* assertion is about.
async fn sessions_on(fixture: &Grilling, skill: &str) -> usize {
    let running: Vec<i64> = outputs(&fixture.view().await)
        .into_iter()
        .map(|output| output.id)
        .collect();

    let mut spent = 0;

    for event in running {
        if fixture.capture(event).await.contains(skill) {
            spent += 1;
        }
    }

    spent
}

/// Resume on an inline run whose branch is already on a pull request wraps that
/// up, without spending a session on work that is already done.
///
/// The ending the halt over a missing pull request advises: open it by hand, and
/// resume. Nothing else in Verkstead ever looks again, so before this the run
/// stayed in Implementing however many times the human pressed the button, and
/// each press cost an account a session that had nothing left to build.
///
/// Asked of GitHub rather than of the branch, which is why the `gh` here changes
/// its answer mid-test while nothing about the repository does: the pull request
/// was opened in a browser, and a branch cannot say that it was.
#[tokio::test]
async fn resuming_an_inline_run_whose_branch_has_a_pull_request_wraps_it_up_unspent() {
    let spill = tempfile::tempdir().unwrap();
    let opened = spill.path().join("opened-by-hand");

    let fixture = grilling_spilling(spill, AN_INLINE_RUN, &gh_opened_by_hand(&opened)).await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    // The run stops where an inline run stops: the work is committed and there
    // is no pull request on it.
    let missing = fixture.stopped().await;

    assert!(
        missing.html.contains("no pull request"),
        "the run stopped on the pull request nothing opened: {:?}",
        missing.html,
    );
    assert_eq!(
        implementing_sessions(&fixture).await,
        1,
        "one session so far"
    );

    // And the human opens one from their phone, which is the whole of what they
    // were told to do.
    std::fs::write(&opened, "https://github.com/tobico/verkstead/pull/41\n").unwrap();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let found = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping)
                .then(|| pull_request(view).cloned())
                .flatten()
        })
        .await;

    assert_eq!(
        found.number, 41,
        "the pull request the human opened is the one the Conversation wraps up",
    );
    assert_eq!(
        implementing_sessions(&fixture).await,
        1,
        "and no second one was spent building work that was already on it",
    );
    assert_eq!(
        commits(&fixture.view().await).len(),
        1,
        "which the branch says too: nothing built it again",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        None,
        "and nothing is waiting on the human any more",
    );
}

/// Resume where GitHub has no pull request for the branch builds the work again,
/// exactly as it always did.
///
/// The ordinary case, and the one the reading in front of it must not change:
/// *no pull request* is GitHub answering that the work is unfinished, so a fresh
/// implementing session is started on the handoff the run has always had.
#[tokio::test]
async fn resuming_an_inline_run_with_no_pull_request_builds_the_work_again() {
    let fixture = grilling_asking(
        r#"
        printf 'prompt was: %s\n' "$2"

        case "$2" in
        *implementing/SKILL.md*)
            if [ -f TRIED ]; then
                sleep 300
            else
                printf 'once\n' > TRIED
                printf 'a limiter\n' > limiter.md
                git add limiter.md
                git commit --quiet -m 'feat: rate limiting'
                printf 'the limiter is in, the middleware is not\n'
            fi
            ;;
        *)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        esac
        "#,
        NO_PULL_REQUEST,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    fixture.stopped().await;

    let before = outputs(&fixture.view().await).len();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let relaunched = fixture
        .until(|view| {
            let running = outputs(view);
            (running.len() > before).then(|| running[before].id)
        })
        .await;

    let said = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id == relaunched && output.lines > 1)
                .map(|output| output.id)
        })
        .await;

    let printed = fixture.capture(said).await.replace("\r\n", "\n");

    assert!(
        printed.contains("/verkstead/skills/implementing/SKILL.md"),
        "an inline run picked up again is an inline run: {printed:?}",
    );
    assert!(
        printed.contains("A counter per key."),
        "primed with the handoff the grilling wrote: {printed:?}",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "and the work is being built, which is where it stopped",
    );
    assert_eq!(
        view.blocked_on, None,
        "with nothing waiting on the human while something is driving it",
    );
}

/// And that second session ends the run by opening the pull request, whether or
/// not it had anything left to commit.
///
/// The ending the skill sends it to: the work was already built and pushed by
/// nobody, so what it has to do is check it over and carry it to a pull request
/// — and a branch it finds nothing wrong with is a branch it commits nothing to.
/// Landing no commit is what an empty session looks like too, so the two are
/// told apart by asking GitHub rather than by counting commits: a pull request
/// on the branch is the session having done the whole of what it was sent for.
#[tokio::test]
async fn a_second_inline_session_that_only_opens_the_pull_request_wraps_the_run_up() {
    let spill = tempfile::tempdir().unwrap();
    let opened = spill.path().join("opened-by-the-session");

    // The second session's `gh pr create`, standing where the real one would:
    // the file it writes is what the server's own `gh` finds a pull request by.
    let stub = format!(
        r#"
printf 'prompt was: %s\n' "$2"

case "$2" in
*reviewing/SKILL.md*)
    printf 'reading the whole branch\n'
    sleep 300
    ;;
*implementing/SKILL.md*)
    if [ -f TRIED ]; then
        printf 'the work was already here, so all it wanted was a pull request\n'
        printf 'https://github.com/tobico/verkstead/pull/41\n' > {opened}
    else
        printf 'once\n' > TRIED
        printf 'a limiter\n' > limiter.md
        git add limiter.md
        git commit --quiet -m 'feat: rate limiting'
        printf 'the limiter is in, and nothing pushed it\n'
    fi
    ;;
*)
    printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
    printf 'the handoff is written\n'
    sleep 300
    ;;
esac
"#,
        opened = quoted(&opened),
    );

    let fixture = grilling_spilling(spill, &stub, &gh_opened_by_hand(&opened)).await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    // The first session builds the work and goes without pushing, which is the
    // ending this whole run is about.
    let missing = fixture.stopped().await;

    assert!(
        missing.html.contains("no pull request"),
        "the run stopped on the pull request nothing opened: {:?}",
        missing.html,
    );

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let found = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping)
                .then(|| pull_request(view).cloned())
                .flatten()
        })
        .await;

    assert_eq!(
        found.number, 41,
        "the pull request the second session opened is the one the Conversation wraps up",
    );
    assert_eq!(
        implementing_sessions(&fixture).await,
        2,
        "which took a second session, the first having left the branch unpushed",
    );
    assert_eq!(
        commits(&fixture.view().await).len(),
        1,
        "and it built nothing again: there was nothing left to build",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        None,
        "so nothing is waiting on the human any more",
    );
}

/// And Resume where `gh` cannot answer at all halts saying so, without spending
/// a session on it.
///
/// The third answer, and the one that is neither of the other two: GitHub has
/// not said there is no pull request, it has not been asked. A session launched
/// into that would build whatever was left, reach the push its skill ends on,
/// and dead-end on the same `gh` — so what happens instead is the halt, which is
/// the one thing that reaches the human on their phone.
#[tokio::test]
async fn resuming_an_inline_run_github_cannot_be_asked_about_halts_unspent() {
    let fixture = grilling_asking(AN_INLINE_RUN, NOTHING_ASKABLE).await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "inline").await, Submitted::Accepted);

    let first = fixture.stopped().await;

    assert!(
        first.html.contains("not logged in"),
        "the run stopped because nothing could be asked: {:?}",
        first.html,
    );

    let before = outputs(&fixture.view().await).len();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let again = fixture
        .until(|view| {
            said(view)
                .into_iter()
                .find(|notice| notice.id != first.id)
                .map(|notice| (*notice).clone())
        })
        .await;

    assert!(
        again.html.contains("not logged in"),
        "and it stops on the same thing, named again: {:?}",
        again.html,
    );
    assert!(
        again.html.contains("pull request"),
        "with the step it stopped in front of: {:?}",
        again.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "what is missing is out here, so a restart looking again would find it",
    );

    let view = fixture.view().await;

    assert_eq!(
        outputs(&view).len(),
        before,
        "and nothing was launched into it: a session could only reach the same `gh`",
    );
    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "the work is where it was, because nothing about it got any further",
    );
    assert_eq!(
        view.blocked_on,
        Some(again.id),
        "and the human is what it is waiting on",
    );
}

/// Resume on a stalled grilling means a fresh grilling, because there is nothing
/// else it could mean: an interview lives in the session having it, and that
/// session is gone.
///
/// What survives it is what the human already answered, which is on the Timeline
/// — so the fresh session is primed with the Brief it always had and a digest of
/// every Set that came back, and does not open by asking again what was settled
/// yesterday.
///
/// And the Set the dead session left open is locked on the way past. Nothing
/// is waiting on that Answer any more, so leaving it open would be the human
/// answering into nothing.
#[tokio::test]
async fn resuming_a_stalled_grilling_starts_a_fresh_one_told_what_was_already_settled() {
    let fixture = grilling_swept(
        r#"
        printf 'prompt was: %s\n' "$2"

        # The first session dies where it stands, which is the stall there is to
        # resume from. The one Resume starts stays up, the way a grilling that is
        # really grilling does — a second session that exited would be a second
        # stall, and the sweep looking every tenth of a second here would find it
        # while the assertions were still being read.
        if [ -f GRILLED ]; then
            sleep 300
        else
            printf 'once\n' > GRILLED
        fi
        "#,
    )
    .await;

    fixture.quiet().await;

    // What the dead session got through before it went: one Set answered, and
    // one still hanging with nobody left to read the Answer.
    let answered = fixture.ask(ASKED_ALREADY).await;

    assert_eq!(
        fixture
            .respond(
                answered,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "and burst on top of it" }
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    let orphan = fixture.ask(LEFT_HANGING).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Grilling the work"),
        "the Conversation says it is being grilled and nothing is: {:?}",
        stopped.html,
    );

    let before = outputs(&fixture.view().await).len();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let relaunched = fixture
        .until(|view| {
            let running = outputs(view);
            (running.len() > before).then(|| running[before].id)
        })
        .await;

    let said = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id == relaunched && output.lines > 1)
                .map(|output| output.id)
        })
        .await;

    let printed = fixture.capture(said).await.replace("\r\n", "\n");

    assert!(
        printed.contains("/verkstead/skills/grilling/SKILL.md"),
        "a grilling started again is a grilling: {printed:?}",
    );
    assert!(
        printed.contains("The API has none."),
        "on the Brief it was always about: {printed:?}",
    );
    assert!(
        printed.contains("Per key — and burst on top of it"),
        "and told what the human already settled, so it does not ask again: {printed:?}",
    );
    assert!(
        !printed.contains("How long should a client be locked out"),
        "the Set nobody answered said nothing, so nothing of it is quoted: {printed:?}",
    );

    let hanging = sets(&fixture.view().await)
        .into_iter()
        .find(|asked| asked.set_id == orphan)
        .expect("the Set the dead session left open is on the Timeline")
        .standing
        .clone();

    assert!(
        matches!(hanging, verkstead_render::Standing::LockedUnanswered(_)),
        "and nothing is left for the human to answer into: {hanging:?}",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        None,
        "and the stop is gone, so nothing is waiting on the human",
    );
}

/// And the Deferred Ask the dead session left behind is *not* locked, which is
/// the same rule read the other way.
///
/// Nothing was ever waiting on one, so a session dying takes nothing away from
/// it: the human answers it in their own time and the Answers are folded into
/// whichever session builds next. Locking it here would close a question they
/// were meant to answer, on the grounds that nobody would read the answer — the
/// one thing that is not true of a Deferred Ask.
#[tokio::test]
async fn resuming_a_stalled_grilling_leaves_its_deferred_asks_open() {
    let fixture = grilling_swept(
        r#"
        if [ -f GRILLED ]; then
            sleep 300
        else
            printf 'once\n' > GRILLED
        fi
        "#,
    )
    .await;

    fixture.quiet().await;

    // One of each, left behind by the session that went: a Blocking Ask with
    // nobody reading the Answer, and a Deferred Ask that never had anybody.
    let blocking = fixture.ask(LEFT_HANGING).await;
    let deferred = fixture.ask_deferred(DEFERRED).await;

    fixture.stopped().await;
    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let standing = |view: &ConversationView, wanted: i64| {
        sets(view)
            .into_iter()
            .find(|asked| asked.set_id == wanted)
            .expect("the Set is on the Timeline")
            .standing
            .clone()
    };

    // Waited for on the blocking one, because that is the locking the relaunch
    // does: once it has happened, the deferred one has been walked past too.
    let view = fixture
        .until(|view| {
            matches!(
                standing(view, blocking),
                verkstead_render::Standing::LockedUnanswered(_)
            )
            .then(|| view.clone())
        })
        .await;

    assert!(
        matches!(
            standing(&view, deferred),
            verkstead_render::Standing::Waiting(verkstead_schema::Liveness::Deferred),
        ),
        "the Deferred Ask is still the human's to answer: {:?}",
        standing(&view, deferred),
    );

    // And answering it still reaches a session, which is the whole of what
    // leaving it open was for.
    assert_eq!(
        fixture
            .respond(
                deferred,
                serde_json::json!([{ "label": "Q9", "selected": 1 }]),
            )
            .await,
        Submitted::Accepted,
    );
}

/// Resume on a wrap-up that stopped at a red check watches it again from no
/// attempts spent.
///
/// The fix counters go first: the human has
/// read what stopped and asked for another go, and a count left standing would
/// be a watcher that stopped again on its next poll without dispatching anything.
/// A third fix session is what says they were forgotten — two is every one the
/// branch was allowed.
#[tokio::test]
async fn resuming_a_halted_wrap_up_watches_the_checks_again_from_no_attempts_spent() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture = grilling_spilling(
        prompts,
        &a_backlog_then_fixes(&written),
        &gh_checking("FAILURE"),
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Rust"),
        "the run stopped on the red check: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "and the human is what it is waiting on",
    );
    assert_eq!(fixes(&fixture.view().await), 2, "having had both its goes");

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    // A third fix session says both halves of it at once. Nothing advances past
    // a stop, so one dispatching at all is the stop gone; and two attempts is
    // every one the branch was allowed, so a third is the count forgotten.
    fixture.until(|view| (fixes(view) > 2).then_some(())).await;
}

/// Steering a stalled backlog run into Implementing carries it on: the next
/// task is read off the branch and worked, and the stop the click wrote goes.
///
/// The same recompute rather than one of its own. What is next is the backlog's
/// own answer, asked of `.tasks/` exactly as every other turn of the run asks
/// it — so what the steer starts is what Resume starts, reused rather than
/// forked. The task whose session died is still there, so that is the task the
/// fresh session is started on: nothing here reverts anything.
///
/// And the target is offered because something stands. A branch with a backlog
/// left in it is the whole of what a continue steer needs, which is what the
/// modal draws the row by and what the submit is refused by where there is
/// none.
#[tokio::test]
async fn steering_a_stalled_backlog_run_into_implementing_works_the_next_task() {
    let fixture = grilling_swept(
        r#"
        case "$1" in
        claude-grilling-5)
            mkdir -p .tasks
            printf '# Rate limiting\n\n- [ ] 01: Count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            git add .tasks
            git commit --quiet -m 'chore: plan the rate limiter'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            if [ ! -f TRIED ]; then
                printf 'once\n' > TRIED
                printf 'this task is beyond me\n'
                exit 1
            else
                printf 'prompt was: %s\n' "$2"
                sleep 300
            fi
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains(".tasks/01-count.md"),
        "the run stopped at the task whose session died: {:?}",
        stopped.html,
    );

    let view = fixture.view().await;

    assert!(
        view.ready_to_continue,
        "the branch holds a backlog with work left in it, so the modal offers \
         the target",
    );

    let before = outputs(&view).len();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture.steer_into("Implementing", false).await,
        ConversationSteered::Steered,
    );

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("/verkstead/skills/next-task/SKILL.md"),
        "the run picks the backlog up again, which is the fork that reads it: \
         {printed:?}",
    );

    let view = fixture.view().await;

    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("moved", Lifecycle::Implementing),
            ("steer", Lifecycle::Implementing),
            ("moved", Lifecycle::Implementing),
        ],
        "the human's own line, and the plain move under it — into the state it \
         was already in, because that is what they said",
    );
    assert_eq!(
        view.blocked_on, None,
        "and the stop the click wrote is gone: nothing advances past one, so a \
         session starting at all is the stop taken away",
    );
    assert_eq!(
        view.direction,
        Some(Direction::TaskList),
        "with the direction left exactly as the steer found it: what says how \
         the work is being built is the Conversation's own pick",
    );

    let worktree = PathBuf::from(view.worktree.unwrap().path);

    assert!(
        worktree.join(".tasks/01-count.md").exists(),
        "and it is the same task, because nothing reverted anything",
    );
}

/// A backlog, then a task session that dies the first time it is run and stays
/// the second — the stall, and then the session the steer starts.
///
/// Which time it is is remembered outside the worktree, because this is the
/// test that takes the worktree away between the two.
fn a_backlog_then_once_then_staying(remembered: &Path) -> String {
    format!(
        r#"
case "$1" in
claude-grilling-5)
    mkdir -p .tasks
    printf '# Rate limiting\n\n- [ ] 01: Count the requests\n' > .tasks/TODO.md
    printf '# 01. Count the requests\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan the rate limiter'
    printf 'the backlog is written\n'
    sleep 300
    ;;
*)
    printf 'prompt was: %s\n' "$2"
    if [ ! -f {remembered} ]; then
        printf 'once\n' > {remembered}
        printf 'this task is beyond me\n'
        exit 1
    fi
    sleep 300
    ;;
esac
"#,
        remembered = quoted(remembered),
    )
}

/// And the same steer where the Worktree has gone: the backlog is still on the
/// branch, so it is still something to carry on and nothing has to be written.
///
/// The Conversation this button was written for, steered into the one target
/// that reads a directory to decide what it offers. That reading is of the
/// Worktree as it stands — a directory that has gone holds no `.tasks/` — so a
/// rule taking *cannot tell* for *nothing there* would refuse a steer that was
/// about to check the branch out again and find the whole backlog on it, and
/// would say there was nothing on the branch to carry on while the branch was
/// holding it.
///
/// So the modal offers carrying on, the submit is not refused, and what follows
/// is the next task read off the directory the steer made.
#[tokio::test]
async fn steering_into_implementing_carries_on_a_backlog_whose_worktree_has_gone() {
    let spill = tempfile::tempdir().unwrap();
    let remembered = spill.path().join("tried");

    let fixture = grilling_at_pace(
        spill,
        &a_backlog_then_once_then_staying(&remembered),
        PULL_REQUEST,
        *SWEEPING,
        &[],
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    fixture.stopped().await;

    let view = fixture.view().await;
    let worktree = PathBuf::from(view.worktree.expect("the run has one").path);

    assert!(
        worktree.join(".tasks/01-count.md").exists(),
        "the backlog is there before the directory goes",
    );

    // The registration in the repository outlives the directory, which is what
    // leaves git refusing to check the branch out anywhere.
    std::fs::remove_dir_all(&worktree).unwrap();

    let view = fixture.view().await;

    assert!(
        view.ready_to_continue,
        "the branch still holds the backlog, so carrying on is still worth \
         offering: a directory that has gone is nothing read rather than \
         nothing standing",
    );

    let before = outputs(&view).len();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture.steer_into("Implementing", false).await,
        ConversationSteered::Steered,
        "and nothing is asked for in writing: what is next is the backlog's own \
         answer, as it always was",
    );

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("/verkstead/skills/next-task/SKILL.md"),
        "the run picks the backlog up again, which is the fork that reads it: \
         {printed:?}",
    );

    assert!(
        worktree.join(".tasks/01-count.md").is_file(),
        "in a directory made again from the branch, with the backlog on it",
    );

    let view = fixture.view().await;

    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(
        view.blocked_on, None,
        "and the stop the click wrote is gone"
    );
}

/// Steering a halted wrap-up back into Wrapping does what that press does: the
/// checks are watched again from no attempts spent, and the stop goes.
///
/// The same recompute rather than one of its own. A steer into Wrapping carries
/// no payload — the wrap-up's four watchers work out for themselves what is left
/// to do — so what it starts is what Resume starts, reused rather than forked.
/// A third fix session is what says the counters were forgotten: two is every
/// one the branch was allowed.
///
/// And the stop the click wrote has to be gone for any of it to happen. Nothing
/// advances past a stop, so a watcher dispatching at all is the stop taken away.
#[tokio::test]
async fn steering_a_halted_wrap_up_into_wrapping_watches_the_checks_afresh() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture = grilling_spilling(
        prompts,
        &a_backlog_then_fixes(&written),
        &gh_checking("FAILURE"),
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Rust"),
        "the run stopped on the red check: {:?}",
        stopped.html,
    );
    assert_eq!(fixes(&fixture.view().await), 2, "having had both its goes");

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false },
        "the run had already stopped, so the click found nothing to interrupt",
    );
    assert_eq!(
        fixture.steer_into("Wrapping", false).await,
        ConversationSteered::Steered,
    );

    let view = fixture.view().await;

    assert_eq!(view.state, Lifecycle::Wrapping);
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("moved", Lifecycle::Implementing),
            ("moved", Lifecycle::Wrapping),
            ("steer", Lifecycle::Wrapping),
            ("moved", Lifecycle::Wrapping),
        ],
        "the human's own line, and the plain move under it — into the state it \
         was already in, because that is what they said",
    );
    assert_eq!(
        view.blocked_on, None,
        "and the stop the click wrote is gone",
    );

    // A third fix session says the rest of it at once: nothing advances past a
    // stop, so one dispatching at all is the stop gone; and two attempts is
    // every one the branch was allowed, so a third is the count forgotten.
    fixture.until(|view| (fixes(view) > 2).then_some(())).await;
}

/// The same backlog and wrap-up, plus a session that plays the instruction a
/// steer sends: it prints what it was given and commits, which is what one
/// reports through.
///
/// Told apart from every other session here by the skill its prompt names,
/// because that is the fact under it: an instruction session runs under the same
/// implementation Profile the tasks did and differs only in what it was sent to
/// do — and in which skill it was sent there inside.
fn a_backlog_then_an_instruction(reviews: &Path) -> String {
    format!(
        r#"
case "$2" in
*reviewing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {reviews}
{REVIEW_AND_FIND_NOTHING}
    ;;
*responding/SKILL.md*)
{RESPOND_AND_FIND_NOTHING}
    ;;
*instruction/SKILL.md*)
    printf 'prompt was: %s\n' "$2"
    printf 'and the burst is unbounded\n' >> notes.md
    git add -A
    git commit --quiet -m 'docs: note what the limiter still does not do'
    sleep 300
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#,
        reviews = quoted(reviews),
    )
}

/// Steering a Conversation Verkstead has finished with into Implementing with an
/// instruction hands the pipeline on: the work is done, and the branch's pull
/// request is wrapped up again.
///
/// The exception this whole control was built for. The work went to a pull
/// request, the wrap-up settled and the Conversation finished — and then there
/// is one more thing to do on the branch. Nothing on it stands to be carried on,
/// the backlog having been worked to empty and taken away, so the instruction is
/// the whole of what the target can be, and what the human writes is what the
/// session is sent off with.
///
/// **And then the pipeline, rather than a Conversation left where the session
/// stopped.** What follows a clean finish is read off the branch — no backlog
/// here, and a pull request — so the wrap-up starts again over what was just
/// committed.
#[tokio::test]
async fn an_instruction_session_that_commits_wraps_the_pull_request_up_again() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_an_instruction(&reviews),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert!(
        !view.ready_to_continue,
        "the backlog was worked to empty and taken away, so nothing on the \
         branch stands to be carried on — which is what makes the instruction \
         the whole of this target",
    );

    let before = outputs(&view).len();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false },
        "everything had finished, so the click found nothing to interrupt",
    );
    assert_eq!(
        fixture
            .steer_instructed("Note what the limiter still does not do.\n")
            .await,
        ConversationSteered::Steered,
    );

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("/verkstead/skills/instruction/SKILL.md"),
        "the session is put inside the instruction skill, which is the one that \
         says the pipeline carries on after it: {printed:?}",
    );
    assert!(
        printed.contains("Note what the limiter still does not do."),
        "and it is started on what the human wrote: {printed:?}",
    );

    // The second move into wrapping is the whole assertion: the pull request is
    // the one the finish step opened, so what says it is being wrapped up again
    // is the Conversation arriving there a second time. Waited for rather than
    // read once — a wrap-up with nothing outstanding settles itself and goes
    // straight on to Done, so the state a moment later is as likely to be that.
    let view = fixture
        .until(|view| (moves_into(view, Lifecycle::Wrapping) > 1).then(|| view.clone()))
        .await;

    assert!(
        commits(&view)
            .iter()
            .any(|commit| commit.subject.starts_with("docs: note what the limiter")),
        "with what the instruction committed under it: {:?}",
        commits(&view),
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped anywhere in it: {:?}",
        notices(&view),
    );

    let steer = view
        .timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Steer(steer) => steer.html.clone(),
            _ => None,
        })
        .expect("the steer carries what was written on it");

    assert!(
        steer.contains("Note what the limiter still does not do."),
        "and the instruction is on the record as the Steer's own body, which is \
         what says what this session was for: {steer:?}",
    );
}

/// The same steer where the pull request is one the record never got: the
/// instruction is done, and GitHub's answer is what carries the Conversation on
/// to wrapping it up.
///
/// The other half of
/// [`resuming_an_emptied_backlog_whose_branch_has_a_pull_request_wraps_it_up_unspent`],
/// and the same failed ending seen from the third door. Its finish step pushed
/// and opened the pull request and the recording of it did not happen, so the
/// Conversation is implementing an empty backlog with the work out on GitHub —
/// and a human who steers an instruction in to get it moving is doing the
/// obvious thing.
///
/// What follows the instruction is read off GitHub rather than off the record,
/// which is the whole of what this asks. The record is what Verkstead wrote down
/// and the pull request is GitHub's fact; where they disagree it is because
/// writing it down is what failed, so asking the record would tell the run what
/// it already believes and stop the Conversation a second time over a pull
/// request that is sitting there open.
#[tokio::test]
async fn an_instruction_session_wraps_up_a_pull_request_the_record_never_got() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let opened = spill.path().join("opened-by-hand");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_an_instruction(&reviews),
        &gh_opened_by_hand(&opened),
    )
    .await;

    worked_to_empty(&fixture).await;

    // The ending that did not finish: the backlog is worked through and taken
    // away, and nothing recorded a pull request.
    let missing = fixture.stopped().await;

    assert!(
        missing.html.contains("no pull request"),
        "the run stopped where the ending failed: {:?}",
        missing.html,
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "and the Conversation is left implementing a backlog with nothing in it",
    );
    assert!(
        pull_request(&view).is_none(),
        "with no pull request on the record at all, which is the state this is about",
    );

    // And the pull request is there all along, which is what the record is wrong
    // about.
    std::fs::write(&opened, "https://github.com/tobico/verkstead/pull/41\n").unwrap();

    let before = outputs(&view).len();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false },
        "nothing is running: the run stopped where its ending failed",
    );
    assert_eq!(
        fixture
            .steer_instructed("Note what the limiter still does not do.\n")
            .await,
        ConversationSteered::Steered,
    );

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("Note what the limiter still does not do."),
        "the session is started on what the human wrote: {printed:?}",
    );

    let found = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping)
                .then(|| pull_request(view).cloned())
                .flatten()
        })
        .await;

    assert_eq!(
        found.number, 41,
        "and what follows it is the pull request GitHub had all along, wrapped up \
         rather than stopped over a second time",
    );

    let view = fixture.view().await;

    assert!(
        commits(&view)
            .iter()
            .any(|commit| commit.subject.starts_with("docs: note what the limiter")),
        "with what the instruction committed under it: {:?}",
        commits(&view),
    );
    assert_eq!(
        view.blocked_on, None,
        "and nothing is waiting on the human any more",
    );
}

/// The same backlog and wrap-up, plus a session that plays a follow-up: it does
/// whatever `following_up` says, which is what each of these tests differs by.
fn a_backlog_then_a_follow_up(reviews: &Path, following_up: &str) -> String {
    // Written as a word in the stubs below and spelled out here, exactly as the
    // wrap-up's own stubs write it — see [`WHILE_NOBODY_HAS_ASKED`].
    let following_up = following_up.replace("WHILE_NOBODY_HAS_ASKED", WHILE_NOBODY_HAS_ASKED);

    format!(
        r#"
case "$2" in
*reviewing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {reviews}
{REVIEW_AND_FIND_NOTHING}
    ;;
*responding/SKILL.md*)
{RESPOND_AND_FIND_NOTHING}
    ;;
*following-up/SKILL.md*)
    printf 'prompt was: %s\n' "$2"
{following_up}
    ;;
*)
{A_BACKLOG_OF_ONE}
    ;;
esac
"#,
        reviews = quoted(reviews),
    )
}

/// A follow-up session that does its round and then stays there, which is what
/// one waiting on the human looks like: it has asked, and it is holding the
/// Worktree until they answer.
const A_ROUND_THEN_WAITING: &str = r#"    printf 'it counts the 429s it sends\n' >> notes.md
    git add -A
    git commit --quiet -m 'docs: say what the limiter counts'
    sleep 300"#;

/// One that does a round of work, waits to be answered, says its piece and then
/// idles — which is every follow-up session between rounds, an interactive agent
/// having nothing to do until it is spoken to.
///
/// The commit is what makes this follow-up one that pushed, which is what puts
/// the wrap-up's checks back to waiting when it lands.
const A_ROUND_THEN_IDLE: &str = "    printf 'it counts the 429s it sends\\n' >> notes.md\n    \
     git add -A\n    \
     git commit --quiet -m 'docs: say what the limiter counts'\n    \
     SAYING='following it up'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'nothing else then\\n'\n    \
     sleep 300";

/// The same, committing nothing at all: a follow-up that was a question and an
/// answer and no work, which is half of what the state is for.
const A_QUESTION_THEN_IDLE: &str = "    SAYING='following it up'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'it counts them, yes\\n'\n    \
     sleep 300";

/// And one that goes round twice: it is answered, asks again, and idles once
/// that second round has been answered too.
///
/// The two markers are the two rounds. A stub cannot idle on a blocking ask and
/// wake up, so the test writes `answered` when it has answered the first Set and
/// `again` when it has answered the second.
const TWO_ROUNDS_THEN_IDLE: &str = "    SAYING='following it up'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     rm -f /tmp/verkstead/asked\n    \
     SAYING='one more thing then'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/again ]; do sleep 0.1; done\n    \
     printf 'nothing else then\\n'\n    \
     sleep 300";

/// One that goes idle having asked nothing at all — a turn that ended with
/// nothing to show for it, which is a Conversation the human can neither answer
/// nor end — and takes the rescue: it reads the line typed into its terminal,
/// writes it down where the test can read it, and asks.
///
/// `read` is where a stub is idle without being asleep: it prints nothing and
/// waits on the one thing a rescue arrives through, which is the session's own
/// terminal.
const IDLE_UNTIL_TOLD: &str = "    printf 'reading the branch\\n'\n    \
     read -r TOLD\n    \
     printf '%s\\n' \"$TOLD\" >> /tmp/verkstead/rescues\n    \
     SAYING='asking now'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'nothing else then\\n'\n    \
     sleep 300";

/// And one that will not ask whatever it is told: it writes down every line
/// typed into it and puts nothing to anybody, for as long as it is left there.
const IDLE_WHATEVER_IT_IS_TOLD: &str = "    printf 'reading the branch\\n'\n    \
     while read -r TOLD; do printf '%s\\n' \"$TOLD\" >> /tmp/verkstead/rescues; done\n    \
     sleep 300";

/// One that is answered, works on for longer than the grace, and only then
/// finishes: a session that is gone because it had nothing left to do.
///
/// The talking is what makes this a test of the ending rather than of the
/// quiet. Anything a session prints puts the whole grace back on the clock, so
/// a stub that goes on printing past [`BRISKLY`]'s `proposing` cannot be ended
/// on quiet — which leaves the session going first as the only way this
/// follow-up can end at all.
const A_MARKED_ROUND_THEN_GONE: &str = "    SAYING='following it up'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     LEFT=20\n    \
     while [ $LEFT -gt 0 ]; do \
     printf 'still tidying up\\n'; sleep 0.1; LEFT=$((LEFT - 1)); done\n    \
     printf 'that is that, then\\n'";

/// A follow-up that does its round, is answered, and then finishes: a session
/// that is gone with the human never having said there was nothing else, which
/// is the stop a follow-up is picked up again from.
const A_ROUND_THEN_GONE: &str = "    SAYING='following it up'\n    \
     printf '%s\\n' \"$SAYING\"\n    \
     WHILE_NOBODY_HAS_ASKED\n    \
     while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done\n    \
     printf 'that is that, then\\n'";

/// What a follow-up round puts to the human: an ordinary Set, because that is
/// all a follow-up's rounds ever are. What ends the follow-up is the viewer's
/// own **Nothing else** option beside the comment box, which is no Question of
/// the agent's — see the schema's `Response::nothing_else`.
const A_FOLLOW_UP_ROUND: &str = r#"
title: About the 429s
questions:
  - label: Q9
    text: It counts them against the same window. Is that what you meant?
    options:
      - n: 1
        text: Yes
        recommended: true
      - n: 2
        text: No, see below
"#;

/// Steering a Conversation Verkstead has finished with into Follow-up starts a
/// session inside the follow-up skill, on the brief the human wrote — and the
/// Conversation is driven for as long as that session runs.
///
/// The whole of what the state is: the work is on a pull request, the human has
/// read it, and what they want now is to ask about it and have things done about
/// it. So the brief is the whole of what the session is sent off with, and the
/// commits it makes land on the Timeline the way every other session's do.
///
/// **And it is never swept as stalled.** The sweep here runs every tenth of a
/// second — see [`SWEEPING`] — so a follow-up session sitting on an answer with
/// nothing on the drivers register would be stopped out from under itself within
/// a moment of the steer. Which is what a follow-up is: a Conversation waiting on
/// a human who is on a phone.
#[tokio::test]
async fn steering_into_follow_up_runs_the_skill_on_the_brief_and_is_never_swept() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_at_pace(
        spill,
        &a_backlog_then_a_follow_up(&reviews, A_ROUND_THEN_WAITING),
        &gh_about(GREEN, "", ""),
        *SWEEPING,
        &[],
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;
    let before = outputs(&view).len();
    let said = notices(&view).len();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false },
        "everything had finished, so the click found nothing to interrupt",
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    // The round it will put, up from the moment the session starts: a session
    // waiting on the human is one with an ask of its own open, and that is what
    // keeps the rescue off it as much as the sweep — see
    // [`a_follow_up_that_goes_idle_without_asking_is_told_to_put_it_to_the_human`].
    fixture.ask(A_FOLLOW_UP_ROUND).await;

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("/verkstead/skills/following-up/SKILL.md"),
        "the session is put inside the follow-up skill, which is the one that \
         says to keep asking until the human is finished: {printed:?}",
    );
    assert!(
        printed.contains("Does it count the 429s it sends?"),
        "and it is started on what the human wrote: {printed:?}",
    );

    let view = fixture
        .until(|view| {
            commits(view)
                .iter()
                .any(|commit| commit.subject.starts_with("docs: say what the limiter"))
                .then(|| view.clone())
        })
        .await;

    assert_eq!(
        view.state,
        Lifecycle::FollowUp,
        "the Conversation is where the steer put it, with what the follow-up \
         committed on its Timeline",
    );

    // Long enough for many sweeps. The session is sitting there waiting on the
    // human, which is what a follow-up spends its time doing.
    tokio::time::sleep(SWEEPING.stalls * 8).await;

    let view = fixture.view().await;

    assert_eq!(view.state, Lifecycle::FollowUp);
    assert_eq!(
        notices(&view).len(),
        said,
        "and nothing stopped it: a follow-up session is registered as driving, \
         so the sweep leaves it alone: {:?}",
        notices(&view),
    );
}

/// A follow-up session that is gone stops the Conversation, with the ordinary
/// Notice saying what it was doing.
///
/// The responding rule: a follow-up ends when the human says there is nothing
/// else, and nobody is ever dispatched to finish somebody else's follow-up. So a
/// session that has finished with the human not having said it leaves a
/// Conversation with nobody following anything up, which is a stop like any
/// other — the human is told, and the Steer button is what they have.
#[tokio::test]
async fn a_follow_up_session_that_is_gone_stops_the_conversation() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_a_follow_up(&reviews, "    printf 'nothing more to say\\n'"),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Following the work up"),
        "what was being done, said in the words the state is judged by: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("the follow-up session finished"),
        "and what came of it, which is nothing worse than an ending: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("nobody is left to ask you anything"),
        "and why that is a stop: nothing is dispatched to finish somebody \
         else's follow-up: {:?}",
        stopped.html,
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::FollowUp,
        "stopped where it stood: a stop is a condition an active state is in \
         rather than a state of its own",
    );
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "so the Conversation is blocked on the human, with the Notice to read",
    );
}

/// And one that finishes on a round the human had already marked is a follow-up
/// that is *over* rather than one nobody is left to have.
///
/// The mark is read where the session ends as well as where it idles, and that
/// is the whole of what tells the two endings apart. **Finish your turn** is
/// what the skill tells a session with nothing left to ask, and an interactive
/// agent that decides there is nothing to do exits zero — so a session going
/// before the grace beside it has run out is the ordinary shape of a follow-up
/// ending rather than one that fell over. Read on the quiet alone, it would put
/// a stop on the Timeline of a Conversation the human had finished with, and
/// cost them a press to get back what they had already said.
///
/// The stub talks past the grace after it is answered, so nothing here can be
/// ended on quiet: the session going is the only way this one lands anywhere.
#[tokio::test]
async fn a_follow_up_session_that_finishes_on_the_mark_lands_in_the_wrap_up() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_a_follow_up(&reviews, A_MARKED_ROUND_THEN_GONE),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    let set = fixture.ask(A_FOLLOW_UP_ROUND).await;

    assert_eq!(fixture.answer_ending(set).await, Submitted::Accepted);
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped: a session that finished on the human's own mark is a \
         follow-up that ended rather than one that is gone: {:?}",
        notices(&view),
    );
    assert!(!view.working, "and nothing is left holding the Worktree",);
}

/// A follow-up that pushed ends on the human's mark and lands back in the
/// wrap-up, which waits on the new checks before it says Done again.
///
/// The three things that end one, together: the newest round they answered
/// carries **Nothing else**, nothing is left open on the Conversation, and the
/// session has gone quiet. Then it is ended where it stands and the Conversation
/// goes back to Wrapping over the pull request it was opened about — with the
/// checks put back to waiting, because the follow-up committed and GitHub has a
/// new run to make up its mind about. *Back to Done* is the wrap-up's own
/// settling rule and nothing the follow-up decides.
///
/// Which is what the suite here says: still running for as long as the mark the
/// follow-up's commit left is there, and green once the test takes it away.
/// Without the unsettle the wrap-up would land with yesterday's green standing
/// and could reach Done before the checks watcher had looked once.
#[tokio::test]
async fn a_follow_up_ends_on_the_mark_and_lands_back_in_the_wrap_up() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let running = spill.path().join("checks-running");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_a_follow_up(&reviews, A_ROUND_THEN_IDLE),
        &gh_about(&green_until(&running), "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let said = waiting_on_checks(&fixture.view().await).len();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    // What the follow-up pushed, and the new run GitHub is making up its mind
    // about because of it: the stub commits, and this is that commit's suite.
    fixture
        .until(|view| {
            commits(view)
                .iter()
                .any(|commit| commit.subject.starts_with("docs: say what the limiter"))
                .then_some(())
        })
        .await;

    std::fs::write(&running, "").unwrap();

    let set = fixture.ask(A_FOLLOW_UP_ROUND).await;

    // Long enough for the grace several times over. The round is open, so
    // nothing here ends anything however quiet the session goes.
    tokio::time::sleep(BRISKLY.proposing * 3).await;

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::FollowUp,
        "a session idling on a Blocking Ask is one working rather than one over",
    );

    assert_eq!(fixture.answer_ending(set).await, Submitted::Accepted);
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let view = fixture
        .until(|view| (view.state == Lifecycle::Wrapping).then(|| view.clone()))
        .await;

    assert!(
        !view.working,
        "the session was ended as the follow-up ended, so the Worktree is the \
         wrap-up's again",
    );
    assert!(
        !checks_settled(&fixture).await,
        "and the checks are back to waiting: the follow-up pushed, so the green \
         standing over them was the run before it",
    );

    let view = fixture
        .until(|view| view.waiting_on_checks.then(|| view.clone()))
        .await;

    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "which is the wrap-up down to the one thing nothing here can hurry",
    );
    assert!(
        waiting_on_checks(&view).len() > said,
        "and it said so afresh, this being a narrowing of its own: {:?}",
        waiting_on_checks(&view),
    );

    // The suite finishes, and the wrap-up settles the ordinary way.
    std::fs::remove_file(&running).unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert!(
        notices(&fixture.view().await).is_empty(),
        "and nothing stopped anywhere along it: a follow-up that ends is not a \
         run that stopped: {:?}",
        notices(&fixture.view().await),
    );
}

/// A follow-up that was questions and answers alone lands with everything
/// settled and passes straight through to Done.
///
/// The other half of the landing. Nothing was pushed, so GitHub has nothing new
/// to say and the wrap-up's settle facts stand exactly as the follow-up found
/// them — checks green, review answered, comments dealt with. Which is the whole
/// of the settling rule, so the Conversation is Done the moment it arrives.
#[tokio::test]
async fn a_follow_up_that_pushed_nothing_goes_straight_back_to_done() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_a_follow_up(&reviews, A_QUESTION_THEN_IDLE),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let landed = commits(&fixture.view().await).len();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    let set = fixture.ask(A_FOLLOW_UP_ROUND).await;

    assert_eq!(fixture.answer_ending(set).await, Submitted::Accepted);
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert_eq!(
        commits(&view).len(),
        landed,
        "the follow-up was talk and nothing else, so the branch is where it was",
    );
    assert!(
        checks_settled(&fixture).await,
        "and nothing put the checks back to waiting, there being no new run to \
         wait on",
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped: {:?}",
        notices(&view),
    );
}

/// A Set asked after an end-marked Response keeps the follow-up open, and that
/// newer Set's own Response is what decides.
///
/// The mark is never sticky. Picking **Nothing else** and writing *one more
/// thing* in the comment beside it is the human doing exactly what the control
/// is for: the agent reads the comment, does the thing, and asks again — and a
/// follow-up that had already landed on the first mark would have taken the
/// Worktree away from it mid-sentence. So what the rule reads is the newest
/// Response of the round, every time round.
#[tokio::test]
async fn a_set_asked_after_the_mark_keeps_the_follow_up_open() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_a_follow_up(&reviews, TWO_ROUNDS_THEN_IDLE),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    let first = fixture.ask(A_FOLLOW_UP_ROUND).await;

    assert_eq!(fixture.answer_ending(first).await, Submitted::Accepted);
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    // The second round, which is the session reading the *one more thing* the
    // human wrote beside their tick and coming back about it.
    let second = fixture.ask(A_FOLLOW_UP_ROUND).await;

    // Long enough for the grace several times over. The first Response is marked
    // and the session is quiet between its lines, so a rule that read the newest
    // *mark* rather than the newest *Response* would have landed by now.
    tokio::time::sleep(BRISKLY.proposing * 3).await;

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::FollowUp,
        "the follow-up is open again, and what it is waiting on is the answer to \
         the round it has just asked",
    );
    assert!(
        !responded(&view, second),
        "which is still there to be answered: {:?}",
        where_it_stands(&view, second),
    );

    // And answered without the mark, which is the human saying there is more:
    // the follow-up goes on running rather than landing on the mark before it.
    assert_eq!(fixture.answer(second).await, Submitted::Accepted);
    std::fs::write(handoff_directory(&fixture).join("again"), "").unwrap();

    tokio::time::sleep(BRISKLY.proposing * 3).await;

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::FollowUp,
        "the newest Response decides, and it carried no mark",
    );
}

/// A follow-up session that goes with a question of its own still up stops the
/// Conversation, and the question goes off with it.
///
/// The failure a Set left standing would be: the session that asked it is gone,
/// no other is ever handed somebody else's ask, and a card that stayed blocked
/// on the human over a question nobody is behind is one they could answer for
/// ever without anything reading it. So Verkstead reaches for the lock on their
/// behalf, exactly as a wrap-up does for its own gone sessions.
#[tokio::test]
async fn a_gone_follow_up_session_takes_the_question_it_left_with_it() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_a_follow_up(
            &reviews,
            "    printf 'that is what I would have asked\\n'\n    \
             WHILE_NOBODY_HAS_ASKED\n    \
             printf 'gh: the connection dropped\\n'\n    \
             exit 1",
        ),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    let set = fixture.ask(A_FOLLOW_UP_ROUND).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped
            .html
            .contains("any question it had put to you has been closed unanswered"),
        "the stop says what became of what it left: {:?}",
        stopped.html,
    );

    let view = fixture
        .until(|view| {
            matches!(
                where_it_stands(view, set),
                Some(verkstead_render::Standing::LockedUnanswered(_))
            )
            .then(|| view.clone())
        })
        .await;

    assert_eq!(
        view.state,
        Lifecycle::FollowUp,
        "stopped where it stood, as every stop is",
    );
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "and what the human is blocked on is the Notice rather than a question \
         nobody is behind",
    );
}

/// What a rescued session was told, waited for from the file the stub writes
/// each line typed into it to.
///
/// The one thing a stub can report that is neither printed nor committed: a
/// rescue arrives at the session's own terminal, and what proves it arrived is
/// the session having read it.
async fn told(fixture: &Grilling, times: usize) -> Vec<String> {
    typed_into(fixture, "rescues", times).await
}

/// The same for anything else typed into a session: `file` is what that stub
/// writes each read down to, in the handoff directory every sandbox has.
///
/// One loop for the two things Verkstead types at a session — the rescue and the
/// nudge — because from out here they are the same claim: a line went into a
/// terminal, and the session read it.
async fn typed_into(fixture: &Grilling, file: &str, times: usize) -> Vec<String> {
    let written = handoff_directory(fixture).join(file);
    let deadline = Instant::now() + *PATIENCE;

    loop {
        let said = std::fs::read_to_string(&written).unwrap_or_default();
        let lines: Vec<String> = said.lines().map(str::to_owned).collect();

        if lines.len() >= times {
            return lines;
        }

        assert!(
            Instant::now() < deadline,
            "the session read {} of the {times} lines typed into it: {lines:?}",
            lines.len(),
        );

        pause(Duration::from_millis(25)).await;
    }
}

/// A follow-up session that goes idle without asking anything is typed a line
/// telling it to ask, and asks.
///
/// The condition nothing else here can see: the session is alive, it has nothing
/// open, and the human's newest answer — if there is one at all — carries no
/// mark. Which is a Conversation they can neither answer nor end, because
/// nothing has been put to them. So Verkstead says so, through the one channel
/// into a running session there is, and what it says is the thing an agent
/// cannot find out from inside its own session: that the screen it is printing
/// to has nobody in front of it.
#[tokio::test]
async fn a_follow_up_that_goes_idle_without_asking_is_told_to_put_it_to_the_human() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_a_follow_up(&reviews, IDLE_UNTIL_TOLD),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    let said = told(&fixture, 1).await;

    assert!(
        said[0].contains("verkstead ask"),
        "it is told what the human sees, which is a Question Set and nothing \
         else: {said:?}",
    );
    assert!(
        said[0].contains("carry on with it now"),
        "and told first to get on with its next step, where it has one — a line \
         typed on a guess about a session is one that has to be safe to be \
         wrong about: {said:?}",
    );
    assert!(
        said[0].contains("blocked or waiting on me"),
        "with the ask put as the other case rather than as the instruction, so \
         that a session that was never stuck does not manufacture a Set: \
         {said:?}",
    );
    assert!(
        said[0].contains("summarize your status"),
        "and asked for where it had got to, in the words somebody watching \
         would have typed: {said:?}",
    );

    // Which is what the rescue is for: the session takes another turn and puts
    // the round it was sitting on to the human.
    let set = fixture.ask(A_FOLLOW_UP_ROUND).await;

    assert_eq!(fixture.answer_ending(set).await, Submitted::Accepted);
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "and the follow-up ended the ordinary way: a session that was talked \
         into asking is not one anything stopped: {:?}",
        notices(&view),
    );
    assert_eq!(
        told(&fixture, 1).await.len(),
        1,
        "and it was told once, the ask being what said it had been heard",
    );
}

/// A follow-up session that will not ask, whatever it is told, stops the
/// Conversation after the second rescue — with a Notice saying so.
///
/// Twice at most, because the second failure is evidence rather than bad luck.
/// What is left is a Conversation with nobody putting anything to the human, and
/// that is a stop like any other: they read what happened and press Resume,
/// which starts a fresh session on the same brief.
#[tokio::test]
async fn a_follow_up_session_that_will_not_ask_is_stopped_after_two_rescues() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_a_follow_up(&reviews, IDLE_WHATEVER_IT_IS_TOLD),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Following the work up"),
        "what was being done, said in the words the state is judged by: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("without asking you anything"),
        "and why it is a stop: nothing was ever put to the human: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("being told twice"),
        "with the rescue named, so that the Notice is not about a session that \
         was never spoken to: {:?}",
        stopped.html,
    );

    assert_eq!(
        told(&fixture, 2).await.len(),
        2,
        "twice and no more: the third time round is the stop rather than \
         another line",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::FollowUp,
        "stopped where it stood, as every stop is",
    );
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "with the human blocked on the Notice, which is the one thing there is \
         to read",
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "and Verkstead decided it, so a restart leaves it exactly here",
    );
}

/// A grilling session that has been given its direction and then goes idle —
/// nothing asked, nothing written — with every line typed into it written down
/// where the test can read it.
///
/// The pick is what puts a watcher on a grilling session at all: from that
/// moment there is an artifact it is supposed to be writing, and *not writing
/// it and not asking about it either* is the shape nothing else here can see.
fn a_grilling_that_never_writes_the_backlog() -> String {
    format!(
        "    SAYING='grilling'\n    \
         printf '%s\\n' \"$SAYING\"\n    \
         {WHILE_NOBODY_HAS_ASKED}\n    \
         while read -r TOLD; do printf '%s\\n' \"$TOLD\" >> /tmp/verkstead/rescues; done\n    \
         sleep 300"
    )
}

/// An inline pick whose implementation session comes up, says a word and then
/// goes idle with nothing committed.
///
/// The grilling half writes its handoff and waits, which is what ends a grilling
/// on an inline pick; what is being asked about is the session on the other side
/// of it.
const AN_INLINE_RUN_THAT_GOES_IDLE: &str = r#"
case "$1" in
claude-grilling-5)
    printf '# What we settled\n\nAn in-process counter.\n' > /tmp/verkstead/handoff.md
    printf 'the handoff is written\n'
    sleep 300
    ;;
*)
    printf 'reading the handoff\n'
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#;

/// The same, except that the implementation session commits its work and then
/// idles rather than exiting — which is what an interactive agent with nothing
/// left to do actually does.
const AN_INLINE_RUN_THAT_COMMITS_AND_IDLES: &str = r#"
case "$1" in
claude-grilling-5)
    printf '# What we settled\n\nAn in-process counter.\n' > /tmp/verkstead/handoff.md
    printf 'the handoff is written\n'
    sleep 300
    ;;
*)
    printf 'reading the handoff\n'
    printf 'a limiter\n' > limiter.md
    git add limiter.md
    git commit --quiet -m 'feat: rate limiting'
    printf 'pushed, and the pull request is open\n'
    sleep 300
    ;;
esac
"#;

/// A backlog of one whose step session comes up, says a word and then goes idle
/// with its entry exactly as unticked as it found it.
///
/// The grilling half is [`A_BACKLOG_OF_ONE`]'s: what is being asked about here is
/// the step, so the list it works has to land the ordinary way first.
const A_BACKLOG_THEN_AN_IDLE_STEP: &str = r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    printf 'reading the task\n'
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#;

/// The same, except that the step session asks before it falls silent: a session
/// sitting on a Blocking Ask, which is one doing exactly what it should.
const A_BACKLOG_THEN_A_STEP_THAT_ASKS: &str = r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    printf 'reading the task\n'
    while [ ! -f /tmp/verkstead/asked ]; do sleep 0.1; done
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#;

/// And the same again with a step session that never stops talking, which is one
/// still at work however long it takes.
///
/// It reads its terminal beside the talking rather than instead of it, so that a
/// line typed into it would be written down: a stub that could not have recorded
/// a rescue would prove nothing about not having been sent one.
const A_BACKLOG_THEN_A_STEP_THAT_KEEPS_TALKING: &str = r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    printf 'reading the task\n'
    while :; do printf 'still at it\n'; sleep 0.1; done &
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#;

/// And one that says a word the moment its answer arrives, and then falls
/// silent: a session woken by the answer, which is what one that is working does
/// with it.
///
/// The word is the whole of the fixture. Verkstead cannot see an answer reach a
/// session — what carries it is a chain of hops it has no view of — so what it
/// waits for is the first thing the session says afterwards, and this is a stub
/// that says it.
const A_BACKLOG_THEN_A_STEP_THAT_SPEAKS_WHEN_ANSWERED: &str = r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    printf 'reading the task\n'
    while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done
    printf 'right, on it\n'
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#;

/// And one that never says anything at all: a step session that comes up and is
/// silent from its first moment, which is what a cold start looks like from
/// outside and what a session that died on the way up looks like too.
const A_BACKLOG_THEN_A_STEP_THAT_NEVER_SPEAKS: &str = r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#;

/// What a step session would ask, where it has something to ask about.
const A_STEP_QUESTION: &str = r#"
title: About the counter
questions:
  - label: Q1
    text: Per key or per address?
    options:
      - n: 1
        text: Per key
        recommended: true
      - n: 2
        text: Per address
"#;

/// Whatever the session has been told so far, which for these is nothing.
///
/// [`told`] read the other way round: that one waits for a line to arrive, and
/// this one is what the tests about a session being left alone assert the
/// absence of.
fn anything_told(fixture: &Grilling) -> Vec<String> {
    std::fs::read_to_string(handoff_directory(fixture).join("rescues"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Take a grilling to the moment its direction has been picked, which is where
/// every session Verkstead watches for an artifact starts.
async fn picked(fixture: &Grilling, direction: &str) {
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;

    assert_eq!(fixture.pick(set, direction).await, Submitted::Accepted);
}

/// A grilling session that goes idle without writing what the pick asked for is
/// told to ask, and stopped after the second time it will not.
///
/// The rescue is one mechanism over every state, and this is the same condition
/// a follow-up's is read against with a different done-indicator under it: a
/// grilling is finished when its artifact has landed, and one that is idle, has
/// nothing open and has written nothing is a run nobody can move. Today that sat
/// there indefinitely with nothing saying so.
#[tokio::test]
async fn a_grilling_that_goes_idle_without_its_artifact_is_told_and_then_stopped() {
    let fixture = grilling(&a_grilling_that_never_writes_the_backlog()).await;

    picked(&fixture, "task-list").await;

    let said = told(&fixture, 1).await;

    assert!(
        said[0].contains("verkstead ask"),
        "it is told what the human sees, which is a Question Set and nothing \
         else: {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("reaking the work down"),
        "what was being done, said in the words the step is judged by: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("without asking you anything"),
        "and why it is a stop: nothing was ever put to the human: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("being told twice"),
        "with the rescue named, so that the Notice is not about a session that \
         was never spoken to: {:?}",
        stopped.html,
    );
    assert_eq!(
        told(&fixture, 2).await.len(),
        2,
        "twice and no more: the third time round is the stop rather than \
         another line",
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "and Verkstead decided it, so a restart leaves it exactly here",
    );
}

/// And a backlog step that goes quiet without the commit that finishes it is
/// told and stopped the same way.
///
/// The same loop with the same bound, over the done-indicator a step is judged
/// by: the entry ticked off in the Worktree's `TODO.md` and git holding nothing
/// pending for it. A hung step used to hold the whole run open with the human never told.
#[tokio::test]
async fn a_step_that_goes_quiet_without_its_commit_is_told_and_then_stopped() {
    let fixture = grilling(A_BACKLOG_THEN_AN_IDLE_STEP).await;

    picked(&fixture, "task-list").await;

    let said = told(&fixture, 1).await;

    assert!(
        said[0].contains("summarize your status"),
        "the same line, in the words somebody watching would have typed: \
         {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("01-count.md"),
        "the Notice names the step rather than the state, a human wanting to \
         know which task: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("without asking you anything"),
        "and says why it stopped: {:?}",
        stopped.html,
    );
    assert_eq!(told(&fixture, 2).await.len(), 2, "twice and no more");
}

/// A backlog of one worked by sessions that draw a full screen rather than
/// printing lines, which is what every backend after Claude does.
///
/// None of these is ever silent for long: a frame goes out every twentieth of a
/// second, on the alternate screen an interface takes over, so the byte clock
/// alone would never call one idle — nothing would end it, rescue it or mark it,
/// and a run of them would sit there for ever. What says a turn is over here is
/// `prompt` standing on the Screen.
///
/// And each of them leaves a silence in the middle of its turn that is longer
/// than the grace a printing session is ended on, which is the other half of the
/// same claim: a TUI that stops to think is not a TUI that has finished.
///
/// `frames` is how many times the prompt is drawn before the session falls
/// silent, and `-1` is for ever — a backend that goes on repainting the prompt
/// it is sitting at, which is the case the signature exists for. A bounded one
/// says so in the handoff directory as it stops, so a test waiting for the
/// silence to start has something to wait for.
///
/// `commits` is whether the step does what its task asked. One that does is
/// ended on its landing and its judgement together; one that does not is a run
/// nobody can move, which is the rescue's.
fn a_backlog_drawing(prompt: &str, frames: i32, commits: bool) -> String {
    let working = if commits {
        r#"
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: count the requests"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
    fi"#
    } else {
        ""
    };

    // A span of the suite's own, and one that has to sit between two of the
    // Pace's: past the grace, so that a session ended inside it would be one
    // ended by the byte clock, and well short of the long-stop, so that it is
    // not the long-stop catching it either.
    let thinking = (BRISKLY.proposing * 3 / 2).as_secs_f64();

    format!(
        r#"
frame() {{ printf '\033[2J\033[H%s\n' "$1"; }}
drawing() {{
    printf '\033[?1049h'
    LEFT={frames}
    while [ "$LEFT" -ne 0 ]; do
        frame '{prompt}'
        sleep 0.05
        if [ "$LEFT" -gt 0 ]; then LEFT=$((LEFT - 1)); fi
    done
    printf 'silent\n' > "/tmp/verkstead/silent-$1"
    sleep 300
}}
case "$1" in
gpt-5-codex-grilling)
    frame 'breaking the work down'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    drawing grilling
    ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf 'I read the whole branch and found nothing worth raising\n'
        exit 0
        ;;
    esac
    frame 'reading the task'
    sleep {thinking:.2}
    frame 'still reading the task'{working}
    drawing step &
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#
    )
}

/// A backlog worked by sessions that never fall silent is worked to the end, on
/// the prompt each of them draws when its turn is over.
///
/// The judgement moved off the byte clock and onto the Screen, and this is the
/// whole of what that buys: every session here repaints for as long as it lives,
/// so under the three-second mark not one of them would ever be idle and the run
/// would stop at its first step for ever. What ends each of them is the frame it
/// leaves standing.
///
/// **And the silence each leaves mid-turn ends nothing.** It is longer than the
/// grace a printing session is ended on and longer than the one the rescue waits
/// out, and on this backend it is not idle at all — a TUI that stops to think
/// would otherwise be prodded, or reaped, in the middle of its work.
#[tokio::test]
async fn sessions_that_repaint_are_ended_on_the_prompt_they_draw_rather_than_on_silence() {
    let fixture =
        grilling_drawing(&a_backlog_drawing(AT_THE_PROMPT, -1, true), AT_THE_PROMPT).await;

    worked_to_empty(&fixture).await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped: every session was ended where it stood, on its own \
         prompt: {:?}",
        notices(&view),
    );
    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "and nothing was typed into any of them: the silence each left in the \
         middle of its turn is longer than the grace, and on this backend a \
         session that has stopped printing has not stopped working",
    );
}

/// And one that draws its prompt without doing what it was sent for is told and
/// then stopped, on the same judgement.
///
/// The rescue's precondition is idle, so a backend judged only on its silence
/// would be one the rescue never reached: this session repaints for ever, and
/// what says it is sitting there with nothing to do is the prompt it is
/// repainting.
#[tokio::test]
async fn a_step_that_draws_its_prompt_without_committing_is_told_and_then_stopped() {
    let fixture =
        grilling_drawing(&a_backlog_drawing(AT_THE_PROMPT, -1, false), AT_THE_PROMPT).await;

    picked(&fixture, "task-list").await;

    let said = told(&fixture, 1).await;

    assert!(
        said[0].contains("summarize your status"),
        "the same line as anywhere else, in the words somebody watching would \
         have typed: {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("01-count.md"),
        "the Notice names the step, a human wanting to know which task: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("without asking you anything"),
        "and says why it stopped: {:?}",
        stopped.html,
    );
    assert_eq!(told(&fixture, 2).await.len(), 2, "twice and no more");
}

/// And a prompt the signature no longer matches is caught by the long-stop,
/// which is the whole reason there is one.
///
/// A signature drifts — the wording is the backend's and it will move in a
/// release — and a session drawing a prompt Verkstead does not know reads as one
/// that never stops working. Nothing else here would catch it: the rescue's
/// precondition is idle, every ender waits on the same judgement, and no session
/// carries a cap on its life. So the byte clock stays behind it as a long-stop,
/// and what the human gets is the ordinary would-not-ask stop — one slow round
/// rather than never.
///
/// **And it is slow**, deliberately: the session draws its unknown prompt for
/// longer than the grace and nothing is typed into it, because on this backend a
/// session that is printing is a session at work whatever it is printing. Only
/// once it has stopped printing altogether does the long-stop start, and only
/// once that is out is it idle.
#[tokio::test]
async fn a_prompt_the_signature_does_not_know_is_caught_by_the_long_stop() {
    let fixture = grilling_drawing(
        &a_backlog_drawing(A_PROMPT_THAT_DRIFTED, 40, false),
        AT_THE_PROMPT,
    )
    .await;

    picked(&fixture, "task-list").await;

    // The step session has drawn its prompt for a window longer than the grace,
    // and has now stopped printing altogether — which is where the long-stop
    // starts.
    until_written(&handoff_directory(&fixture).join("silent-step")).await;
    let fell_silent = Instant::now();

    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "nothing was typed into it while it was drawing: a prompt Verkstead \
         does not know is a session still at work, however long it sits there",
    );

    let said = told(&fixture, 1).await;

    assert!(
        fell_silent.elapsed() >= BRISKLY.proposing * 2,
        "and what caught it was the long-stop rather than the grace, which \
         would have had it in under half the time",
    );
    assert!(
        said[0].contains("summarize your status"),
        "the ordinary line, this being the ordinary rules arriving late: \
         {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("without asking you anything"),
        "and the ordinary stop under them: {:?}",
        stopped.html,
    );
    assert_eq!(told(&fixture, 2).await.len(), 2, "twice and no more");
}

/// What the real codex has on its Screen while it is working, which is the whole
/// of what says a Codex session has not stopped — see the server's `sessions`
/// module, where Verkstead's own copy of this wording is kept.
///
/// Written out here rather than reached for out of the server, and deliberately:
/// what these prove is that Verkstead already knows codex's line, so the stub
/// draws what codex draws and nothing is handed in to meet it.
const AT_WORK: &str = "◦ Working (12s • esc to interrupt)";

/// And the frame it leaves when its turn is over: the composer, which is drawn
/// exactly the same while it works.
///
/// That is the reason a Codex session is read the other way round from a session
/// that draws a prompt of its own — there is nothing in this line to tell the two
/// states apart, and the line above is the only thing that changes.
const AT_ITS_PROMPT: &str = "› Ask Codex to do anything";

/// And what a stub draws where that wording has moved on without Verkstead: an
/// at-work line that says the same thing in words Verkstead has never seen.
const AT_WORK_IN_OTHER_WORDS: &str = "◦ Thinking (12s • press escape to stop)";

/// A backlog of one worked by sessions that draw the way codex draws: an at-work
/// line while they work, the composer when they are waiting, and — this being
/// what makes codex codex — not one byte once they are.
///
/// The mirror of [`a_backlog_drawing`], and the shape is the same but for which
/// frame says the turn is over. There the prompt appearing says it; here the
/// at-work line *going* says it, and the quiet behind it is the other half.
///
/// Each of these leaves a silence in the middle of its turn that is longer than
/// the grace a printing session is ended on, with its at-work line standing
/// through it: a TUI that stops to think is not a TUI that has finished, and on
/// this reading the line standing is what says so.
///
/// `at_work` is what it draws while it works — the backend's own where the test
/// is about Verkstead knowing it, and something else where it is about a wording
/// that has moved on. `resting` is the frame it leaves when its turn is over.
///
/// `grilling_model` is the model the session that breaks the work down is
/// launched on, which is how these stubs tell it from the ones that build it —
/// so it is the caller's, one backend's Profile listing different models from
/// another's.
///
/// `commits` is whether the step does what its task asked, as it is there: one
/// that does is ended on its landing and its judgement together, and one that
/// does not is a run nobody can move, which is the rescue's.
///
/// **The Brief is read where the backend it is launched under puts it.** Three
/// of the four take it as the one positional argument and opencode takes it
/// under `--prompt`, so the two are put back into the order the rest of this
/// reads them in before anything looks at either.
fn a_backlog_at_work(grilling_model: &str, at_work: &str, resting: &str, commits: bool) -> String {
    let working = if commits {
        r#"
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: count the requests"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
    fi"#
    } else {
        ""
    };

    // The same span [`a_backlog_drawing`] thinks for, and between the same two
    // of the Pace's: past the grace, so that a session ended inside it would be
    // one ended by the byte clock, and well short of the long-stop.
    let thinking = (BRISKLY.proposing * 3 / 2).as_secs_f64();

    format!(
        r#"
frame() {{ printf '\033[2J\033[H%s\n' "$1"; }}
working() {{
    LEFT=$1
    while [ "$LEFT" -ne 0 ]; do
        frame '{at_work}'
        sleep 0.05
        LEFT=$((LEFT - 1))
    done
}}
resting() {{
    frame '{resting}'
    printf 'silent\n' > "/tmp/verkstead/silent-$1"
    sleep 300
}}
[ "$2" = --prompt ] && set -- "$1" "$3"
case "$1" in
{grilling_model})
    working 10
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    resting grilling
    ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf 'I read the whole branch and found nothing worth raising\n'
        exit 0
        ;;
    esac
    working 10
    sleep {thinking:.2}
    working 10{working}
    resting step &
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#
    )
}

/// A backlog worked by sessions that draw codex's at-work line is worked to the
/// end, on that line going rather than on any frame they leave standing.
///
/// Nothing is handed in here: the stub draws what codex draws, and what finds it
/// is Verkstead's own constant for this backend. That is the whole of what this
/// covers — a Codex session judged by the line the real codex draws.
///
/// **And the silence each leaves mid-turn ends nothing.** It is longer than the
/// grace a printing session is ended on, and the at-work line stands through it:
/// a TUI that stops to think has not stopped working, whatever its terminal is
/// doing.
#[tokio::test]
async fn codex_sessions_are_ended_on_the_at_work_line_going_rather_than_on_the_frame_they_leave() {
    let fixture = grilling_at_work(&a_backlog_at_work(
        CODEX_GRILLING_MODEL,
        AT_WORK,
        AT_ITS_PROMPT,
        true,
    ))
    .await;

    worked_to_empty(&fixture).await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped: every session was ended where it stood, its at-work \
         line gone and its terminal quiet: {:?}",
        notices(&view),
    );
    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "and nothing was typed into any of them: the silence each left in the \
         middle of its turn is longer than the grace, and its at-work line was \
         standing through the whole of it",
    );
}

/// And a step that draws its composer without doing what it was sent for is told
/// and then stopped, on the same judgement.
///
/// The rescue's precondition is idle, so this is the other half of the reading
/// being right: a session that has stopped has to *reach* the rescue, and what
/// says this one has stopped is the at-work line no longer on its Screen.
#[tokio::test]
async fn a_codex_step_that_stops_without_committing_is_told_and_then_stopped() {
    let fixture = grilling_at_work(&a_backlog_at_work(
        CODEX_GRILLING_MODEL,
        AT_WORK,
        AT_ITS_PROMPT,
        false,
    ))
    .await;

    picked(&fixture, "task-list").await;

    let said = told(&fixture, 1).await;

    assert!(
        said[0].contains("summarize your status"),
        "the same line as anywhere else, in the words somebody watching would \
         have typed: {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("01-count.md"),
        "the Notice names the step, a human wanting to know which task: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("without asking you anything"),
        "and says why it stopped: {:?}",
        stopped.html,
    );
    assert_eq!(told(&fixture, 2).await.len(), 2, "twice and no more");
}

/// An at-work line that has moved on without Verkstead costs the run nothing:
/// the quiet behind it is what says the session has stopped, and it says it
/// alone.
///
/// This is the direction a wording moves in most often — a release renames the
/// line, and Verkstead is left looking for words nothing draws. A session
/// drawing them reads as one that has stopped *by the screen*, from its very
/// first frame, and nothing goes wrong: the [`IDLE_AFTER`]-length quiet asked
/// for beside the screen is not there while it works, because a TUI at work
/// repaints. So the backlog is worked to the end, on the byte clock alone —
/// which is Claude's own rule, and the right one to fall back to.
#[tokio::test]
async fn an_at_work_line_that_has_moved_on_leaves_the_run_on_the_byte_clock() {
    let fixture = grilling_at_work(&a_backlog_at_work(
        CODEX_GRILLING_MODEL,
        AT_WORK_IN_OTHER_WORDS,
        AT_ITS_PROMPT,
        true,
    ))
    .await;

    worked_to_empty(&fixture).await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped: a wording Verkstead does not know is a session read \
         on its quiet, not a session nothing ever ends: {:?}",
        notices(&view),
    );
}

/// And an at-work line that never goes is caught by the long-stop, which is the
/// whole reason there is one.
///
/// The other way a signature drifts, and the dangerous one: the line stops
/// telling the two states apart — a release draws it at the prompt as well, or
/// the frame Verkstead reads is not the frame it thought — and a session then
/// reads as one that never stops working. Nothing else here would catch it: the
/// rescue's precondition is idle, every ender waits on the same judgement, and
/// no session carries a cap on its life. So the byte clock stays behind it as a
/// long-stop, and what the human gets is the ordinary would-not-ask stop — one
/// slow round rather than never.
///
/// **And it is slow**, deliberately: the step draws its at-work line for longer
/// than the grace and nothing is typed into it, because a session showing that
/// it is at work is at work whatever else its terminal is doing. Only once it
/// has stopped printing altogether does the long-stop start, and only once that
/// is out is it idle.
#[tokio::test]
async fn an_at_work_line_that_never_goes_is_caught_by_the_long_stop() {
    let fixture = grilling_at_work(&a_backlog_at_work(
        CODEX_GRILLING_MODEL,
        AT_WORK,
        AT_WORK,
        false,
    ))
    .await;

    picked(&fixture, "task-list").await;

    // The step session has drawn its at-work line for a window longer than the
    // grace, and has now stopped printing altogether — which is where the
    // long-stop starts.
    until_written(&handoff_directory(&fixture).join("silent-step")).await;
    let fell_silent = Instant::now();

    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "nothing was typed into it while it was drawing: a session drawing that \
         it is at work is at work, however long it sits there saying so",
    );

    let said = told(&fixture, 1).await;

    assert!(
        fell_silent.elapsed() >= BRISKLY.proposing * 2,
        "and what caught it was the long-stop rather than the grace, which \
         would have had it in under half the time",
    );
    assert!(
        said[0].contains("summarize your status"),
        "the ordinary line, this being the ordinary rules arriving late: \
         {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("without asking you anything"),
        "and the ordinary stop under them: {:?}",
        stopped.html,
    );
}

/// What the real grok has at the foot of every frame while it is working, which
/// is the whole of what says a Grok session has not stopped — see the server's
/// `sessions` module, where Verkstead's own copy of this wording is kept.
///
/// Written out here rather than reached for out of the server, and deliberately,
/// the way codex's is: what these prove is that Verkstead already knows grok's
/// row, so the stub draws what grok draws and nothing is handed in to meet it.
const GROK_AT_WORK: &str = "Shift+Tab:mode  │  Esc:cancel  │  Ctrl+x:shortcuts";

/// And the row it leaves when its turn is over: the same hints but for the one
/// that says the turn can be cancelled.
///
/// That is the reason a Grok session is read the same way round as a Codex one:
/// grok's composer, the model on its border and the rest of these hints are
/// drawn exactly the same while it works, and this row is where the two states
/// differ.
const GROK_AT_ITS_PROMPT: &str = "Shift+Tab:mode  │  Ctrl+x:shortcuts";

/// A backlog worked by sessions that draw grok's at-work hint is worked to the
/// end, on that hint going.
///
/// The same claim as the Codex one above, on the other backend's own wording and
/// with nothing handed in to read it by. What it covers is the shape — a Grok
/// session ended where it stood, and not one prodded through the silence it
/// leaves mid-turn with the hint standing. What pins the wording itself is the
/// test below: a hint Verkstead does not know costs a run nothing here, because
/// the quiet behind the screen carries it either way.
#[tokio::test]
async fn grok_sessions_are_ended_on_their_own_at_work_hint_rather_than_on_codexs() {
    let fixture = grilling_on_grok(&a_backlog_at_work(
        GROK_GRILLING_MODEL,
        GROK_AT_WORK,
        GROK_AT_ITS_PROMPT,
        true,
    ))
    .await;

    worked_to_empty(&fixture).await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped: every session was ended where it stood, its at-work \
         hint gone and its terminal quiet: {:?}",
        notices(&view),
    );
    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "and nothing was typed into any of them: the silence each left in the \
         middle of its turn is longer than the grace, and its at-work hint was \
         standing through the whole of it",
    );
}

/// And a grok hint that never goes is caught by the long-stop, as codex's is.
///
/// The dangerous drift on this reading, and the one worth proving per backend
/// rather than once: a release draws the hint at the prompt as well, or the row
/// Verkstead reads is not the row it thought, and the session then reads as one
/// that never stops working. Nothing else here would catch it — the rescue's
/// precondition is idle and every ender waits on the same judgement — so the
/// byte clock stays behind it, and what the human gets is the ordinary
/// would-not-ask stop.
#[tokio::test]
async fn a_grok_at_work_hint_that_never_goes_is_caught_by_the_long_stop() {
    let fixture = grilling_on_grok(&a_backlog_at_work(
        GROK_GRILLING_MODEL,
        GROK_AT_WORK,
        GROK_AT_WORK,
        false,
    ))
    .await;

    picked(&fixture, "task-list").await;

    // The step session has drawn its at-work hint for a window longer than the
    // grace, and has now stopped printing altogether — which is where the
    // long-stop starts.
    until_written(&handoff_directory(&fixture).join("silent-step")).await;
    let fell_silent = Instant::now();

    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "nothing was typed into it while it was drawing: a Grok session drawing \
         that it is at work is at work, however long it sits there saying so",
    );

    let said = told(&fixture, 1).await;

    assert!(
        fell_silent.elapsed() >= BRISKLY.long_stop,
        "and what caught it was the long-stop rather than the grace or the \
         three seconds behind the screen — which is the whole of what says \
         Verkstead is reading grok's own hint here: a hint it did not know \
         would have had this session in half the time",
    );
    assert!(
        said[0].contains("summarize your status"),
        "the ordinary line, this being the ordinary rules arriving late: \
         {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("without asking you anything"),
        "and the ordinary stop under them: {:?}",
        stopped.html,
    );
}

/// What the real opencode has in the status bar at the foot of every frame
/// while it is working, which is the whole of what says an OpenCode session has
/// not stopped — see the server's `sessions` module, where Verkstead's own copy
/// of this wording is kept.
///
/// Written out here rather than reached for out of the server, and deliberately,
/// the way codex's and grok's are: what these prove is that Verkstead already
/// knows opencode's label, so the stub draws what opencode draws and nothing is
/// handed in to meet it.
const OPENCODE_AT_WORK: &str = "  ⬝⬝⬝⬝⬝■■■  esc interrupt          tab agents  ctrl+p commands";

/// And the bar it leaves when its turn is over: the project's path where the
/// dial and the label were, and the same two hints on the right of it.
///
/// That is the reason an OpenCode session is read the same way round as the two
/// before it: opencode's composer, the `Build auto` label on its border and
/// these hints are drawn exactly the same while it works, and this bar is where
/// the two states differ.
const OPENCODE_AT_ITS_PROMPT: &str = "  /work/verkstead                tab agents  ctrl+p commands";

/// A backlog worked by sessions that draw opencode's at-work label is worked to
/// the end, on that label going.
///
/// The same claim as the two above, on the fourth backend's own wording and with
/// nothing handed in to read it by. It is worth making once per backend rather
/// than once: this label is two words where codex's is three — `esc interrupt`
/// against `esc to interrupt` — so a Verkstead that reached for codex's constant
/// here would find nothing in any frame and read every session as stopped from
/// its first one.
#[tokio::test]
async fn opencode_sessions_are_ended_on_their_own_at_work_label_rather_than_on_codexs() {
    let fixture = grilling_on_opencode(&a_backlog_at_work(
        OPENCODE_GRILLING_MODEL,
        OPENCODE_AT_WORK,
        OPENCODE_AT_ITS_PROMPT,
        true,
    ))
    .await;

    worked_to_empty(&fixture).await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped: every session was ended where it stood, its at-work \
         label gone and its terminal quiet: {:?}",
        notices(&view),
    );
    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "and nothing was typed into any of them: the silence each left in the \
         middle of its turn is longer than the grace, and its at-work label was \
         standing through the whole of it",
    );
}

/// And an opencode label that never goes is caught by the long-stop, as the two
/// before it are.
///
/// The dangerous drift on this reading, and the one worth proving per backend
/// rather than once: a release draws the label at the prompt as well, or the bar
/// Verkstead reads is not the bar it thought, and the session then reads as one
/// that never stops working. Nothing else here would catch it — the rescue's
/// precondition is idle and every ender waits on the same judgement — so the
/// byte clock stays behind it, and what the human gets is the ordinary
/// would-not-ask stop.
#[tokio::test]
async fn an_opencode_at_work_label_that_never_goes_is_caught_by_the_long_stop() {
    let fixture = grilling_on_opencode(&a_backlog_at_work(
        OPENCODE_GRILLING_MODEL,
        OPENCODE_AT_WORK,
        OPENCODE_AT_WORK,
        false,
    ))
    .await;

    picked(&fixture, "task-list").await;

    // The step session has drawn its at-work label for a window longer than the
    // grace, and has now stopped printing altogether — which is where the
    // long-stop starts.
    until_written(&handoff_directory(&fixture).join("silent-step")).await;
    let fell_silent = Instant::now();

    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "nothing was typed into it while it was drawing: an OpenCode session \
         drawing that it is at work is at work, however long it sits there \
         saying so",
    );

    let said = told(&fixture, 1).await;

    assert!(
        fell_silent.elapsed() >= BRISKLY.long_stop,
        "and what caught it was the long-stop rather than the grace or the \
         three seconds behind the screen — which is the whole of what says \
         Verkstead is reading opencode's own label here: a label it did not \
         know would have had this session in half the time",
    );
    assert!(
        said[0].contains("summarize your status"),
        "the ordinary line, this being the ordinary rules arriving late: \
         {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("without asking you anything"),
        "and the ordinary stop under them: {:?}",
        stopped.html,
    );
}

/// A Set of the step session's own, which is what a session blocked on
/// `verkstead ask` is waiting for. Q9, because that is the Question
/// [`Grilling::answer`] answers.
const A_STEPS_QUESTION: &str = r#"
title: Which clock does the window roll on?
questions:
  - label: Q9
    text: The request's own timestamp, or the server's?
    options:
      - n: 1
        text: The server's
        recommended: true
      - n: 2
        text: The request's
"#;

/// A backlog of one whose step session holds a blocking ask: at work until the
/// Set is up, then not a byte and no at-work label until it is answered, and
/// then the work it was sent to do.
///
/// **Which is the strictest shape the wait can take from out here, and not the
/// shape the real opencode wears.** A held ask under the real thing is a
/// session at work: the shell tool runs the command inside the model's own
/// turn, and opencode animates the dial beside its `esc interrupt` label the
/// whole time it does — so it draws its at-work label and is never byte-quiet.
/// What this stub draws instead is what an OpenCode session would look like the
/// day that stops being true: a label that has moved on, or a renderer that
/// settles while a tool call runs. The session is then idle by every reading
/// Verkstead has, the long-stop included, and the unanswered Set of its own is
/// the whole of what stands between the human's answer and a session ended
/// before it could read it.
///
/// `hold` is the marker the test writes once the Set is up, which is what puts
/// the ask and the silence into the order they happen in on a real session —
/// [`WHILE_NOBODY_HAS_ASKED`] is the same trick for a stub that prints. A
/// marker of its own rather than the one [`Grilling::ask`] writes, because the
/// pick that starts the backlog is an ask as well.
fn a_step_that_holds_an_ask(grilling_model: &str, at_work: &str, resting: &str) -> String {
    format!(
        r#"
frame() {{ printf '\033[2J\033[H%s\n' "$1"; }}
working() {{
    LEFT=$1
    while [ "$LEFT" -ne 0 ]; do
        frame '{at_work}'
        sleep 0.05
        LEFT=$((LEFT - 1))
    done
}}
[ "$2" = --prompt ] && set -- "$1" "$3"
case "$1" in
{grilling_model})
    working 10
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    frame '{resting}'
    sleep 300
    ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf 'I read the whole branch and found nothing worth raising\n'
        exit 0
        ;;
    esac
    working 10
    while [ ! -f /tmp/verkstead/hold ]; do
        frame '{at_work}'
        sleep 0.05
    done
    frame '{resting}'
    printf 'holding\n' > /tmp/verkstead/holding
    while [ ! -f /tmp/verkstead/answered ]; do sleep 0.1; done
    working 10
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: count the requests"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
    fi
    frame '{resting}'
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
esac
"#
    )
}

/// An OpenCode session holding a blocking ask is left where it stands however
/// long the human takes, and ended by the ordinary rules once they have
/// answered and its work is done.
///
/// **The half of the blocking ask this backend does not bring with it.** The
/// asking is the CLI's and is the same everywhere; what is this backend's own
/// is that its sessions are judged by what they draw, and a session waiting on
/// the human draws nothing new. Every ender waits on that judgement and the
/// rescue's precondition is idle, so a wait that read as silence would be
/// reaped mid-question — and the byte-quiet long-stop behind the screen would
/// have it whatever its frame said. What holds it is the unanswered Set of its
/// own, which every one of them reads.
///
/// So the stub here is quiet and shows no at-work label for longer than the
/// long-stop, which is the worst case rather than the real one — see
/// [`a_step_that_holds_an_ask`]. Nothing ends it, nothing is typed into it, and
/// the Answers are in front of the session when it goes on.
#[tokio::test]
async fn an_opencode_session_holding_a_blocking_ask_is_neither_ended_nor_prodded() {
    let fixture = grilling_on_opencode(&a_step_that_holds_an_ask(
        OPENCODE_GRILLING_MODEL,
        OPENCODE_AT_WORK,
        OPENCODE_AT_ITS_PROMPT,
    ))
    .await;

    picked(&fixture, "task-list").await;

    // The step session's own Set, and then the marker that tells the stub it is
    // up — which is what puts the ask before the silence, as it is on a real
    // session.
    let set = fixture.ask(A_STEPS_QUESTION).await;
    std::fs::write(handoff_directory(&fixture).join("hold"), "").unwrap();

    until_written(&handoff_directory(&fixture).join("holding")).await;
    let holding = Instant::now();

    // Longer than the long-stop, which is the longest clock in here: the grace
    // is spent several times over, and the byte quiet that catches a signature
    // nobody has caught up with has run past its own mark.
    tokio::time::sleep(BRISKLY.long_stop + BRISKLY.proposing * 2).await;

    assert!(
        holding.elapsed() >= BRISKLY.long_stop,
        "the session was quiet past the one clock that ends a drawing session \
         whatever its screen says",
    );

    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "the session is still there to read what they say: {:?}",
        outputs(&view).last(),
    );
    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "and nothing was typed into it: a session with a question standing in \
         front of the human is not one to prod, however quiet it is",
    );
    assert!(
        notices(&view).is_empty(),
        "nor was it stopped over a question the human has not answered: {:?}",
        notices(&view),
    );

    assert_eq!(fixture.answer(set).await, Submitted::Accepted);

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    // And then the ordinary rules: the step lands, the backlog empties, and the
    // sessions are ended where they stand.
    fixture
        .until(|view| {
            commits(view)
                .iter()
                .any(|commit| commit.subject.starts_with("chore: finish"))
                .then_some(())
        })
        .await;

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "nothing stopped: every session was ended where it stood: {:?}",
        notices(&view),
    );
    assert!(
        !handoff_directory(&fixture).join("rescues").exists(),
        "and nothing was ever typed into any of them",
    );
}

/// And an inline implementation that goes quiet without committing anything is
/// told and stopped the same way.
///
/// The one driver the sweep had been left out of, and the worst place to leave
/// it: an inline run is the whole of a Conversation's work in one session, so
/// one sitting there with its turn over held the Worktree and the registration
/// that says the Conversation is being driven — for ever, with nothing swept
/// because it *was* driven and nothing said because it never spoke.
#[tokio::test]
async fn an_inline_session_that_goes_quiet_without_committing_is_told_and_then_stopped() {
    let fixture = grilling(AN_INLINE_RUN_THAT_GOES_IDLE).await;

    picked(&fixture, "inline").await;

    let said = told(&fixture, 1).await;

    assert!(
        said[0].contains("summarize your status"),
        "the same line, in the words somebody watching would have typed: \
         {said:?}",
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Implementing the work inline"),
        "the Notice names what was being done: {:?}",
        stopped.html,
    );
    assert!(
        stopped.html.contains("without asking you anything"),
        "and says why it stopped: {:?}",
        stopped.html,
    );
    assert_eq!(told(&fixture, 2).await.len(), 2, "twice and no more");
}

/// And one that commits and then idles is ended on that, rather than waited out.
///
/// Every session here is an interactive agent that idles when its work is done
/// rather than exiting, so a run that waited to see one exit was waiting for
/// something that need never come. What says an inline implementation did its
/// work is what it committed, and the grace after the commit is what lets the
/// push and the pull request come after it.
#[tokio::test]
async fn an_inline_session_that_commits_and_idles_is_ended_on_that_and_wraps_up() {
    let fixture = grilling(AN_INLINE_RUN_THAT_COMMITS_AND_IDLES).await;

    picked(&fixture, "inline").await;

    // Both facts off the one view, read at the moment it got there: what the
    // wrap-up goes on to dispatch is not this test's, and a second read would be
    // asking about a Worktree the review had by then.
    let view = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping && pull_request(view).is_some())
                .then(|| view.clone())
        })
        .await;

    // Which is proof enough that the session was ended rather than waited out:
    // the Worktree is handed on only once it is, and the stub is still sitting
    // in its `sleep` — so nothing here could have reached a pull request by
    // waiting for the process to go.
    assert_eq!(
        pull_request(&view)
            .expect("a wrapping Conversation has a pull request")
            .number,
        41,
        "the branch went for review the way a landed inline run always has",
    );

    assert!(
        notices(&view).is_empty(),
        "with nothing stopped on the way: a session that committed and went \
         quiet is one that did its work: {:?}",
        notices(&view),
    );
}

/// A session sitting on a Blocking Ask is never spoken to, however long it sits
/// there.
///
/// The middle third of the condition, and the one that costs the most to get
/// wrong: the ask blocks for as long as the human takes, which may be the next
/// morning, and a line typed in over the top of it would be Verkstead telling a
/// session to ask about the thing it is already asking about.
#[tokio::test]
async fn a_step_waiting_on_its_ask_is_never_told_to_ask() {
    let fixture = grilling(A_BACKLOG_THEN_A_STEP_THAT_ASKS).await;

    picked(&fixture, "task-list").await;

    // Once the step's own session is running, so that the Set it is about to be
    // handed is one asked *by* it: what a session has open is read from its own
    // Event onwards.
    fixture
        .until(|view| (outputs(view).len() > 1).then_some(()))
        .await;

    fixture.ask(A_STEP_QUESTION).await;

    // Several graces of a session saying nothing at all, which is what waiting
    // on a human looks like from outside.
    tokio::time::sleep(BRISKLY.proposing * 4).await;

    assert!(
        anything_told(&fixture).is_empty(),
        "nothing was typed into a session that is waiting on the human: {:?}",
        anything_told(&fixture),
    );

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "and nothing stopped over it: {:?}",
        notices(&view),
    );
}

/// And a session that is still printing is never spoken to either, whatever it
/// has landed.
///
/// Quiet is the first third of the condition and the cheap one, so it is asked
/// first and puts the whole grace back on the clock every time the session says
/// anything. A step that talks its way through an hour of work is one at work.
#[tokio::test]
async fn a_step_that_keeps_talking_is_never_told_to_ask() {
    let fixture = grilling(A_BACKLOG_THEN_A_STEP_THAT_KEEPS_TALKING).await;

    picked(&fixture, "task-list").await;

    fixture
        .until(|view| (outputs(view).len() > 1).then_some(()))
        .await;

    tokio::time::sleep(BRISKLY.proposing * 4).await;

    assert!(
        anything_told(&fixture).is_empty(),
        "nothing was typed into a session that has not stopped talking: {:?}",
        anything_told(&fixture),
    );

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "and nothing stopped over it: {:?}",
        notices(&view),
    );
}

/// A session that has just been answered is left alone until it says something,
/// however long the answer takes to reach it.
///
/// **The near half of the condition, and the half that was wrong.** What carries
/// an answer into a session is a chain Verkstead can see no hop of — the CLI's
/// long poll returning, the harness noticing its background command exited, the
/// model beginning its turn, the first bytes drawn — and a chain slower than the
/// grace is a session that was working perfectly well being told it had gone
/// quiet. It happened: the line landed on a grilling moments after the human's
/// pick, and the session dutifully sent them a Set nobody needed. So what is
/// waited for now is the session's own first word, which is the one thing from
/// out here that says the answer arrived at all.
#[tokio::test]
async fn a_step_that_has_just_been_answered_is_left_alone_until_it_speaks() {
    let fixture = grilling(A_BACKLOG_THEN_A_STEP_THAT_ASKS).await;

    picked(&fixture, "task-list").await;

    fixture
        .until(|view| (outputs(view).len() > 1).then_some(()))
        .await;

    let set = fixture.ask(A_STEP_QUESTION).await;

    // Answered well inside the grace, which is what most picks are: a human on
    // their phone taps the recommended option and puts it down again. Which is
    // the case a loop that looked at the store only once the grace was out
    // would miss entirely — the Set would have come and gone between two looks,
    // and the session would read as one that had never been handed anything.
    tokio::time::sleep(BRISKLY.poll * 4).await;

    assert_eq!(
        fixture
            .respond(set, serde_json::json!([{ "label": "Q1", "selected": 1 }]))
            .await,
        Submitted::Accepted,
    );

    // Twice over the grace a session gets, and this one has said nothing since
    // the answer. Which used to be a rescue and half of the next one.
    tokio::time::sleep(BRISKLY.proposing * 2).await;

    assert!(
        anything_told(&fixture).is_empty(),
        "nothing was typed into a session whose answer may still be on its way \
         to it: {:?}",
        anything_told(&fixture),
    );

    // And the ceiling is where the waiting stops. A session that says nothing
    // at all after its answer is one that died waiting for it, and it is
    // rescued having never spoken — which is a session nobody could move
    // otherwise.
    let said = told(&fixture, 1).await;

    assert!(
        said[0].contains("verkstead ask"),
        "the ordinary line, once the ceiling on the waiting has passed: \
         {said:?}",
    );
}

/// And a word after the answer puts the ordinary grace back in charge, counted
/// from that word.
///
/// The other side of the same rule: the wait is on the session speaking rather
/// than on a longer clock, so a session that speaks and then goes quiet is
/// judged the way it always was. A rule that waited out the ceiling every time
/// would be a rescue arriving five minutes late at every session that genuinely
/// is stuck.
#[tokio::test]
async fn a_step_that_speaks_after_its_answer_is_told_on_the_ordinary_grace() {
    let fixture = grilling(A_BACKLOG_THEN_A_STEP_THAT_SPEAKS_WHEN_ANSWERED).await;

    picked(&fixture, "task-list").await;

    fixture
        .until(|view| (outputs(view).len() > 1).then_some(()))
        .await;

    let set = fixture.ask(A_STEP_QUESTION).await;

    assert_eq!(
        fixture
            .respond(set, serde_json::json!([{ "label": "Q1", "selected": 1 }]))
            .await,
        Submitted::Accepted,
    );

    let answered = Instant::now();

    // What the answer reaching the session looks like here: the stub is waiting
    // on the marker rather than on the Set, the two halves of an answer being
    // split across two processes in this fixture — see [`WHILE_NOBODY_HAS_ASKED`].
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let said = told(&fixture, 1).await;
    let waited = answered.elapsed();

    assert!(
        said[0].contains("verkstead ask"),
        "it was told the ordinary line: {said:?}",
    );
    assert!(
        waited < BRISKLY.waking,
        "and told on the grace after its word rather than on the ceiling, which \
         is what the word is for: {waited:?}",
    );
}

/// A session that has not said a word since it was launched is left alone too:
/// coming up is a stir like any other.
///
/// A cold start is minutes on a loaded machine — the sandbox, the agent, the
/// model's first token — and every one of those minutes looked from here exactly
/// like a session that had gone quiet without asking. So the launch is the first
/// stir a session gets, and nothing is typed into one that has yet to draw its
/// first byte. The ceiling is what catches the one that died coming up.
#[tokio::test]
async fn a_step_that_has_not_spoken_since_launch_is_left_alone_until_the_ceiling() {
    let fixture = grilling(A_BACKLOG_THEN_A_STEP_THAT_NEVER_SPEAKS).await;

    picked(&fixture, "task-list").await;

    // The session's own Capture, which is written when it is launched rather
    // than when it first prints — so this is a session that is up and has said
    // nothing, which is the whole of the fixture.
    fixture
        .until(|view| (outputs(view).len() > 1).then_some(()))
        .await;

    tokio::time::sleep(BRISKLY.proposing * 2).await;

    assert!(
        anything_told(&fixture).is_empty(),
        "nothing was typed into a session that may still be starting up: {:?}",
        anything_told(&fixture),
    );

    assert_eq!(
        told(&fixture, 1).await.len(),
        1,
        "and then it was told, the ceiling being what a session that never \
         speaks at all is caught by",
    );
}

/// And the line this loop types itself is a stir like any other: the second
/// rescue waits on a word too.
///
/// Otherwise a slow turn after the first line would burn the second and the stop
/// with it — a session told twice inside one turn it was in the middle of
/// taking, and stopped for a silence that was one wait rather than two. What
/// ends a Conversation here is meant to be the same evidence twice over.
#[tokio::test]
async fn the_second_rescue_waits_on_a_word_as_the_first_did() {
    let fixture = grilling(A_BACKLOG_THEN_A_STEP_THAT_NEVER_SPEAKS).await;

    picked(&fixture, "task-list").await;

    assert_eq!(told(&fixture, 1).await.len(), 1);

    tokio::time::sleep(BRISKLY.proposing * 2).await;

    assert_eq!(
        anything_told(&fixture).len(),
        1,
        "the second line waits on the word the first one did, the session \
         having said nothing since it was spoken to: {:?}",
        anything_told(&fixture),
    );

    assert_eq!(
        told(&fixture, 2).await.len(),
        2,
        "and the ceiling brings it, so a session that will not speak at all is \
         still stopped rather than left",
    );
}

/// Resume on a stopped follow-up starts a fresh session on the brief it was
/// opened with and the rounds it has already been through — and a restart leaves
/// that stop alone, somebody having decided it.
///
/// A follow-up is a conversation rather than a step, so there is nothing on the
/// branch to read what it had got to off. What outlives the session having it is
/// the Timeline: the brief as the Steer's own body, and the Sets under it. Which
/// is exactly what a relaunched grilling is primed with, and for the same reason
/// — a relaunch that opened by asking again what the human answered an hour ago
/// would cost them the follow-up twice.
#[tokio::test]
async fn resume_follows_the_work_up_again_on_the_brief_and_the_rounds_answered() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let stub = a_backlog_then_a_follow_up(&reviews, A_ROUND_THEN_GONE);

    let fixture = grilling_spilling(spill, &stub, &gh_about(GREEN, "", "")).await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    // One round, answered without the mark: the human has more to say, and the
    // session goes away before they get to say it.
    let set = fixture.ask(A_FOLLOW_UP_ROUND).await;

    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture.stopped().await;

    let before = outputs(&fixture.view().await).len();

    // A second server over the same database, which is what a restart is. This
    // stop is one Verkstead decided, so it waits for the press rather than being
    // taken up unasked.
    let restarted = fixture.restarted(&stub, &gh_about(GREEN, "", "")).await;

    tokio::time::sleep(BRISKLY.proposing * 2).await;

    assert_eq!(
        outputs(&fixture.view().await).len(),
        before,
        "a restart starts nothing over a stop somebody decided",
    );

    drop(restarted);

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("/verkstead/skills/following-up/SKILL.md"),
        "the press starts a follow-up rather than anything else: {printed:?}",
    );
    assert!(
        printed.contains("Does it count the 429s it sends?"),
        "on the brief the steer opened it with, which is what it is still \
         about: {printed:?}",
    );
    assert!(
        printed.contains("What you have already asked, and what I said"),
        "with what has already been said under it: {printed:?}",
    );
    assert!(
        printed.contains("About the 429s") && printed.contains("It counts them against"),
        "which is the round it asked: {printed:?}",
    );
}

/// And a restart takes a follow-up nobody stopped up again, with nobody asked.
///
/// The ordinary case of a server coming back: the session was a process and did
/// not survive it, and nothing decided that. So the restart presses Resume for
/// itself — a fresh session on the same brief, primed with the rounds already
/// answered — rather than leaving a Conversation standing still with a badge
/// nobody put there.
///
/// The session going and the stop being taken off the record are the server
/// going away, said in the two things a test can reach: what a restart finds is
/// a Conversation in Follow-up with no session, nothing driving it, and nobody
/// having decided any of that.
#[tokio::test]
async fn a_restart_follows_the_work_up_again_rather_than_raising_anything() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let stub = a_backlog_then_a_follow_up(&reviews, A_ROUND_THEN_GONE);

    let fixture = grilling_spilling(spill, &stub, &gh_about(GREEN, "", "")).await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_following_up("Does it count the 429s it sends?\n")
            .await,
        ConversationSteered::Steered,
    );

    // One round, answered without the mark: the human has more to say, and the
    // server goes away before they get to say it.
    let set = fixture.ask(A_FOLLOW_UP_ROUND).await;

    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture.stopped().await;
    fixture.drive_again().await;

    let before = outputs(&fixture.view().await).len();

    let _restarted = fixture.restarted(&stub, &gh_about(GREEN, "", "")).await;

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("/verkstead/skills/following-up/SKILL.md"),
        "the restart follows the work up again, with nobody having pressed \
         anything: {printed:?}",
    );
    assert!(
        printed.contains("Does it count the 429s it sends?"),
        "on the brief it was always about: {printed:?}",
    );
    assert!(
        printed.contains("About the 429s") && printed.contains("It counts them against"),
        "and primed with the round it had already been through: {printed:?}",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::FollowUp,
        "the Conversation is where it was, and being followed up again",
    );
    assert!(
        notices(&view)
            .iter()
            .all(|notice| !notice.contains("as the server came back up")),
        "and nothing refused it: a follow-up is something a restart knows how \
         to start again: {:?}",
        notices(&view),
    );
}

/// The same steer over a backlog that still has work in it: the instruction is
/// done first, and then the next task is worked.
///
/// Two things at once, and both are what makes an instruction session a driver
/// rather than an errand beside the work. **The pipeline carries on from it** —
/// what is next is read off `.tasks/` the moment the session goes quiet, exactly
/// as it is after every other turn of the run. And **it is registered as driving
/// while it runs**, which is what the brisk stall sweep here asks: a Conversation
/// with a session in it and nothing on the register is one the sweep stops, so an
/// instruction session that ran unregistered would be stopped out from under
/// itself.
///
/// The direction is left exactly as it found it, too. What says how the work is
/// being built is the human's own pick, and a Conversation working a backlog goes
/// on working one.
#[tokio::test]
async fn an_instruction_session_over_a_backlog_hands_on_to_the_next_task() {
    let fixture = grilling_swept(
        r#"
        case "$2" in
        *instruction/SKILL.md*)
            printf 'prompt was: %s\n' "$2"
            printf 'a note\n' >> notes.md
            git add -A
            git commit --quiet -m 'docs: note the window it counts against'
            sleep 300
            ;;
        *)
            case "$1" in
            claude-grilling-5)
                mkdir -p .tasks
                printf '# Rate limiting\n\n- [ ] 01: Count the requests\n' > .tasks/TODO.md
                printf '# 01. Count the requests\n' > .tasks/01-count.md
                git add .tasks
                git commit --quiet -m 'chore: plan the rate limiter'
                printf 'the backlog is written\n'
                sleep 300
                ;;
            *)
                if [ ! -f TRIED ]; then
                    printf 'once\n' > TRIED
                    printf 'this task is beyond me\n'
                    exit 1
                else
                    printf 'prompt was: %s\n' "$2"
                    sleep 300
                fi
                ;;
            esac
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    fixture.stopped().await;

    let view = fixture.view().await;

    assert!(
        view.ready_to_continue,
        "the branch holds a backlog with work left in it, so carrying on is \
         offered — and the instruction stands beside it rather than in its place",
    );

    let before = outputs(&view).len();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture
            .steer_instructed("Note the window the count is against.\n")
            .await,
        ConversationSteered::Steered,
    );

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("/verkstead/skills/instruction/SKILL.md"),
        "what the human wrote goes first, whatever the branch holds: {printed:?}",
    );

    let printed = fixture.printed_after(before + 1).await;

    assert!(
        printed.contains("/verkstead/skills/next-task/SKILL.md"),
        "and then the run carries on where it stood, which is the fork that \
         reads the backlog: {printed:?}",
    );

    let view = fixture.view().await;

    assert_eq!(view.state, Lifecycle::Implementing);
    assert_eq!(
        view.direction,
        Some(Direction::TaskList),
        "with the direction left exactly as the steer found it: a Conversation \
         that has said how its work is built has said",
    );
    assert_eq!(
        notices(&view).len(),
        1,
        "and nothing stopped after the one the steer answered — an instruction \
         session is registered as driving, so the sweep leaves it alone: {:?}",
        notices(&view),
    );
}

/// An instruction session that ends badly stops the Conversation, with the
/// ordinary Notice saying what it was doing.
///
/// Judged by the ordinary end-of-session rules, which is the other half of being
/// a driver rather than an errand: the human wrote the instruction and walked
/// away, so being told is the only thing that reaches them — and nothing carries
/// the pipeline on over a session that did not finish.
#[tokio::test]
async fn an_instruction_session_that_ends_badly_stops_the_conversation() {
    let fixture = grilling(
        r#"
        case "$2" in
        *instruction/SKILL.md*)
            printf 'I cannot do that\n'
            exit 1
            ;;
        *)
            printf 'grilling\n'
            sleep 300
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    assert_eq!(fixture.steer().await, SteerOpened::Opened { working: true });
    assert_eq!(
        fixture.steer_instructed("Rebase this onto `main`.\n").await,
        ConversationSteered::Steered,
    );

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("Doing what the instruction said"),
        "the Notice says what it was doing when it stopped: {:?}",
        stopped.html,
    );

    let view = fixture.view().await;

    assert!(
        view.blocked_on.is_some() && view.ready_to_resume,
        "and the Conversation is stopped with Resume on offer, which is what a \
         session that ended badly leaves anywhere",
    );
}

/// A steer into Grilling starts the interview again on the round's own Brief,
/// and primes it with everything already answered only where the human asked
/// for that.
///
/// Two steers, because the choice is the point. The first writes a brief and
/// leaves the digest off, which is the ordinary steer: a fresh brief is what the
/// press is usually for, and priming it with the whole of the last interview
/// would be steering into the argument that has just been left behind. The
/// second writes none and asks for the digest, so what it starts on is the Brief
/// the first one wrote — the round's own, whichever round that is — with what
/// the human settled under it.
///
/// **Interrupt current task** is unticked both times, and the session running is
/// ended all the same: one Worktree holds one agent, so the grilling a steer
/// starts takes the Worktree from whatever was in it. The checkbox is for the
/// session a launch cannot displace and for the target that launches nothing —
/// see [`steering_into_done_without_interrupt_sees_the_session_out`].
#[tokio::test]
async fn steering_into_grilling_primes_the_digest_only_where_it_was_asked_for() {
    let fixture = grilling(
        r#"
        printf 'prompt was: %s\n' "$2"
        sleep 300
        "#,
    )
    .await;

    // What the interview being left behind got through: one Set, answered.
    let answered = fixture.ask(ASKED_ALREADY).await;

    assert_eq!(
        fixture
            .respond(
                answered,
                serde_json::json!([
                    { "label": "Q1", "selected": 1, "free_text": "and burst on top of it" }
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    let before = outputs(&fixture.view().await).len();

    assert_eq!(fixture.steer().await, SteerOpened::Opened { working: true });
    assert_eq!(
        fixture
            .steer_grilling(Some("# Retries\n\nThe backoff is wrong.\n"), false)
            .await,
        ConversationSteered::Steered,
    );

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("/verkstead/skills/grilling/SKILL.md"),
        "a grilling steered into is a grilling: {printed:?}",
    );
    assert!(
        printed.contains("The backoff is wrong."),
        "on the Brief the modal has just written: {printed:?}",
    );
    assert!(
        !printed.contains("The API has none."),
        "which is the round's own rather than the one before it: {printed:?}",
    );
    assert!(
        !printed.contains("Per key — and burst on top of it"),
        "and told nothing of the interview it was steered out of: {printed:?}",
    );

    let view = fixture.view().await;

    assert_eq!(view.state, Lifecycle::Grilling);
    assert_eq!(
        steered(&view),
        [
            ("moved", Lifecycle::Grilling),
            ("steer", Lifecycle::Grilling),
            ("moved", Lifecycle::Grilling),
        ],
    );

    let before = outputs(&view).len();

    assert_eq!(fixture.steer().await, SteerOpened::Opened { working: true });
    assert_eq!(
        fixture.steer_grilling(None, true).await,
        ConversationSteered::Steered,
    );

    let printed = fixture.printed_after(before).await;

    assert!(
        printed.contains("The backoff is wrong."),
        "the round's own Brief again, which is the one the steer before it \
         wrote: {printed:?}",
    );
    assert!(
        printed.contains("Per key — and burst on top of it"),
        "and this time everything already answered, because this time they \
         asked for it: {printed:?}",
    );
}

/// A closed Conversation is a source like any other: its Worktree was deleted
/// and its branch kept, so the steer checks the branch out again into one and
/// carries on.
///
/// The furthest a Worktree can be from a running one on a Conversation that has
/// been worked — closing takes the directory away *and* takes it off the record
/// — and it is the one steering has to make from nothing but the branch. Where
/// the record still names a directory the steer rebuilds what it names; here
/// there is nothing to name, so the path is chosen the way a first grilling
/// chooses one.
///
/// Wrapping, because closing leaves the pull request on the record: what is
/// steered into is a wrap-up that has everything under it but somewhere to work.
#[tokio::test]
async fn steering_a_closed_conversation_checks_its_branch_out_again() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture = grilling_spilling(
        prompts,
        &a_backlog_then_fixes(&written),
        &gh_checking("FAILURE"),
    )
    .await;

    worked_to_empty(&fixture).await;
    fixture.stopped().await;

    let branch = fixture.view().await.branch.clone();

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
    assert!(
        fixture.view().await.worktree.is_none(),
        "closing forgets the Worktree and leaves the pull request on the record",
    );

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture.steer_into("Wrapping", false).await,
        ConversationSteered::Steered,
    );

    let view = fixture.view().await;

    assert_eq!(view.state, Lifecycle::Wrapping);

    let worktree = PathBuf::from(
        view.worktree
            .expect("the steer made one and recorded it")
            .path,
    );

    assert!(
        worktree.join("README.md").exists(),
        "it is back, with everything the branch was holding",
    );
    assert_eq!(
        git(&worktree, &["symbolic-ref", "HEAD"]).trim(),
        format!("refs/heads/{branch}"),
        "on the branch closing kept rather than on one cut afresh",
    );

    // And the wrap-up going on in it, which is the reading that says the
    // directory is a worktree rather than a copy of one: a sandbox is given the
    // git directory the checkout points back into.
    fixture.until(|view| (fixes(view) > 2).then_some(())).await;
}

/// The Pairing picked in the modal is recorded as the *Conversation's*, and it
/// is what the sessions after the steer run under.
///
/// Steering re-settles what runs the work rather than picking for one session,
/// which is a thing about the record and a thing about the world at once — so
/// both are read: the Conversation says it runs under the new Pairing, and the
/// fix session dispatched after the steer was launched on that Profile's model.
///
/// The grilling Profile is what it is re-settled to, that being the other
/// Profile this fixture has. Nothing is odd about it: what the two roles run
/// under is the human's, and a steer is the human saying so again.
#[tokio::test]
async fn steering_records_the_pairing_it_was_submitted_with() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture = grilling_spilling(
        prompts,
        &a_backlog_then_fixes(&written),
        &gh_checking("FAILURE"),
    )
    .await;

    worked_to_empty(&fixture).await;
    fixture.stopped().await;

    let picked = fixture.profile("grilling").await;

    assert_ne!(
        Some(picked),
        fixture
            .view()
            .await
            .implementation_pairing
            .map(|pairing| pairing.profile.id),
        "the work is running under the other one, so this is a change to make",
    );

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );

    // A pick judged the way the drafting pickers judge theirs: a Profile that
    // has gone and a model it does not list are both a list that was edited
    // between the read and the pick, and both are refused before anything moves.
    assert_eq!(
        fixture
            .steer_under("Wrapping", 404, "claude-grilling-5")
            .await,
        ConversationSteered::NoSuchProfile,
    );
    assert_eq!(
        fixture
            .steer_under("Wrapping", picked, "claude-nothing-5")
            .await,
        ConversationSteered::NoSuchModel,
    );

    assert_eq!(
        fixture
            .steer_under("Wrapping", picked, "claude-grilling-5")
            .await,
        ConversationSteered::Steered,
    );

    let paired = fixture
        .view()
        .await
        .implementation_pairing
        .expect("the steer settled one");

    assert_eq!(paired.profile.id, picked);
    assert_eq!(
        paired.model.as_deref(),
        Some("claude-grilling-5"),
        "both halves of it: either alone is not something to launch a session with",
    );

    // And the world agrees with the record: the fix session the steer set going
    // was launched on the model the Pairing names.
    fixture.until(|view| (fixes(view) > 2).then_some(())).await;

    let dispatched = std::fs::read_to_string(&written).unwrap();

    assert!(
        dispatched
            .lines()
            .filter(|line| line.starts_with("model="))
            .next_back()
            == Some("model=claude-grilling-5"),
        "the last fix session ran under what the steer settled: {dispatched:?}",
    );

    // And a second steer under the *same* Profile on another of its models,
    // which is a different pick rather than the same one made twice: the picker
    // offers one row per Profile-and-model, so a steer judged by the Profile
    // alone would answer Steered to this and change nothing.
    let ran = fixes(&fixture.view().await);

    assert!(matches!(fixture.steer().await, SteerOpened::Opened { .. }));
    assert_eq!(
        fixture
            .steer_under("Wrapping", picked, "claude-grilling-4.8")
            .await,
        ConversationSteered::Steered,
    );

    assert_eq!(
        fixture
            .view()
            .await
            .implementation_pairing
            .and_then(|pairing| pairing.model),
        Some("claude-grilling-4.8".to_owned()),
        "the model moved under a Profile that did not",
    );

    fixture
        .until(|view| (fixes(view) > ran).then_some(()))
        .await;

    let dispatched = std::fs::read_to_string(&written).unwrap();

    assert!(
        dispatched
            .lines()
            .filter(|line| line.starts_with("model="))
            .next_back()
            == Some("model=claude-grilling-4.8"),
        "and the fix session after it ran on the model the steer settled rather \
         than on the one the Profile was already paired with: {dispatched:?}",
    );
}

/// And a steer whose Worktree has gone makes it again from the branch before any
/// of that.
///
/// The same recreation Resume does, and for the same reason: a directory
/// deleted, hollowed out or dropped from the repository's list of worktrees is a
/// Conversation stuck for good, every session launched into it failing the same
/// way. A worktree is derived state — the branch holds everything that was
/// committed — so it is made again rather than refused on.
#[tokio::test]
async fn steering_a_conversation_whose_worktree_has_gone_makes_it_again() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");

    let fixture = grilling_spilling(
        prompts,
        &a_backlog_then_fixes(&written),
        &gh_checking("FAILURE"),
    )
    .await;

    worked_to_empty(&fixture).await;
    fixture.stopped().await;

    let view = fixture.view().await;
    let branch = view.branch.clone();
    let worktree = PathBuf::from(view.worktree.unwrap().path);

    // The registration in the repository outlives the directory, which is what
    // leaves git refusing to check the branch out anywhere — and what nothing
    // before this knew to clear.
    std::fs::remove_dir_all(&worktree).unwrap();

    assert_eq!(
        fixture.steer().await,
        SteerOpened::Opened { working: false }
    );
    assert_eq!(
        fixture.steer_into("Wrapping", false).await,
        ConversationSteered::Steered,
        "a worktree that has gone is one to make again, not a reason to refuse",
    );

    assert!(
        worktree.join("README.md").exists(),
        "it is back, with everything the branch was holding",
    );
    assert_eq!(
        git(&worktree, &["symbolic-ref", "HEAD"]).trim(),
        format!("refs/heads/{branch}"),
        "checked out on the Conversation's own branch",
    );

    // And the point of all of it: the wrap-up going on in it. Which is the
    // reading that would fail if the worktree were merely there — a sandbox is
    // given the git directory the checkout points back into.
    fixture.until(|view| (fixes(view) > 2).then_some(())).await;
}

/// A grilling that says one thing and goes, and stays the second time it is
/// run — the stall, and then the session Resume starts.
///
/// Which time it is is remembered outside the worktree, because these are the
/// tests that take the worktree away between the two.
fn once_then_staying(remembered: &Path) -> String {
    format!(
        r#"
if [ ! -f {remembered} ]; then
    printf 'once\n' > {remembered}
    printf 'the grilling has nothing to say\n'
else
    printf 'the grilling is running again\n'
    sleep 300
fi
"#,
        remembered = quoted(remembered),
    )
}

/// Resume on a Conversation whose worktree has gone makes it again from the
/// branch, and drives on.
///
/// The Conversation this whole feature was written for: a worktree deleted,
/// hollowed out or dropped from the repository's list of worktrees is a
/// Conversation stuck for good — every session launched into it fails the same
/// way, and the button that is supposed to unstick it would only be pressing
/// that same failure again. So Resume checks before it recomputes, and a
/// worktree that is not one any more is derived state to make again: the branch
/// holds everything that was committed.
#[tokio::test]
async fn resuming_a_conversation_whose_worktree_has_gone_makes_it_again() {
    let spill = tempfile::tempdir().unwrap();
    let remembered = spill.path().join("tried");

    let fixture = grilling_at_pace(
        spill,
        &once_then_staying(&remembered),
        PULL_REQUEST,
        *SWEEPING,
        &[],
    )
    .await;

    fixture.stopped().await;

    let view = fixture.view().await;
    let branch = view.branch.clone();
    let worktree = PathBuf::from(view.worktree.unwrap().path);

    // The registration in the repository outlives the directory, which is what
    // leaves git refusing to check the branch out anywhere — and what nothing
    // before this knew to clear.
    std::fs::remove_dir_all(&worktree).unwrap();

    assert_eq!(
        fixture.resume().await,
        Resumed::Resumed,
        "a worktree that has gone is one to make again, not a reason to refuse",
    );

    assert!(
        worktree.join("README.md").exists(),
        "it is back, with everything the branch was holding",
    );
    assert_eq!(
        git(&worktree, &["symbolic-ref", "HEAD"]).trim(),
        format!("refs/heads/{branch}"),
        "checked out on the Conversation's own branch",
    );

    // And the point of all of it: a session running in it. Which is the reading
    // that would fail if the worktree were merely there — a sandbox is given the
    // git directory the checkout points back into.
    fixture.until(|view| view.working.then_some(())).await;
}

/// A worktree git can still answer about is left exactly as it stands,
/// uncommitted changes and all.
///
/// Validation is there to find the worktrees that have stopped being ones, and
/// a session that died mid-edit has not stopped being anything: what it had
/// written is in the directory and nowhere else, and a rebuild would be Resume
/// throwing away the only copy of it.
#[tokio::test]
async fn resuming_leaves_a_worktree_that_is_still_one_alone() {
    let spill = tempfile::tempdir().unwrap();
    let remembered = spill.path().join("tried");

    let fixture = grilling_at_pace(
        spill,
        &once_then_staying(&remembered),
        PULL_REQUEST,
        *SWEEPING,
        &[],
    )
    .await;

    fixture.stopped().await;

    let worktree = PathBuf::from(fixture.view().await.worktree.unwrap().path);
    let half_written = worktree.join("half-written.rs");

    std::fs::write(&half_written, "// as far as the session got\n").unwrap();

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    assert_eq!(
        std::fs::read_to_string(&half_written).unwrap(),
        "// as far as the session got\n",
        "the work in progress is still there, because nothing was rebuilt",
    );

    fixture.until(|view| view.working.then_some(())).await;
}

/// And a worktree that could not be made again refuses by name.
///
/// The one refusal here with nothing for the human to correct on the workbench,
/// so what it has to do is be a sentence rather than a line in a log: a press
/// that stopped silently on this is exactly the failure the whole feature
/// replaces.
///
/// Something that is not a directory at all sitting where the worktree goes is
/// the plainest way to have one — it cannot be removed as a worktree, it is not
/// a directory to take away, and git will not check a branch out over it.
#[tokio::test]
async fn a_worktree_that_cannot_be_made_again_refuses_by_name() {
    let spill = tempfile::tempdir().unwrap();
    let remembered = spill.path().join("tried");

    let fixture = grilling_at_pace(
        spill,
        &once_then_staying(&remembered),
        PULL_REQUEST,
        *SWEEPING,
        &[],
    )
    .await;

    let stopped = fixture.stopped().await;

    let worktree = PathBuf::from(fixture.view().await.worktree.unwrap().path);

    std::fs::remove_dir_all(&worktree).unwrap();
    std::fs::write(&worktree, "not a worktree\n").unwrap();

    let before = outputs(&fixture.view().await).len();

    assert_eq!(
        fixture.resume().await,
        Resumed::WorktreeRefused,
        "there is nowhere to work and no making one, and the human is told so",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "the stop stands: a refusal changes nothing",
    );
    assert!(
        view.ready_to_resume,
        "and the button is still there, because nothing is driving it yet",
    );
    assert_eq!(
        outputs(&view).len(),
        before,
        "and nothing was launched into a worktree that is not there",
    );
}

/// A Resume pressed on a run that stopped at its push sends for the pull request
/// again, rather than refusing because `.tasks/` has nothing left in it.
///
/// The press that has to be worth pressing. A backlog worked to empty whose
/// branch is on no pull request is the state a finish that stopped between its
/// commit and its push leaves behind — the work is built, the list is gone, and
/// the one thing missing is out on GitHub. Refusing that by name would be the
/// button turning down the one Conversation it could finish, and turning it down
/// every time it was pressed: nothing about the situation changes on its own, so
/// the human would be as stuck as the run was.
///
/// So the branch is read rather than the tree: it has written a backlog since it
/// came off its base, which is a run that got as far as its push, and what is
/// left of it is one session's worth of asking. The stub here cannot open one the
/// first time and can the second, which is the shape of whatever was in the way
/// being taken out of it.
#[tokio::test]
async fn resuming_an_emptied_backlog_with_no_pull_request_sends_for_one_again() {
    let spill = tempfile::tempdir().unwrap();
    let tried = spill.path().join("tried-once");
    let opened = spill.path().join("opened-when-asked");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_whose_pull_request_takes_two_asks(&tried, &opened),
        &gh_opened_by_hand(&opened),
    )
    .await;

    worked_to_empty(&fixture).await;

    // The run has its own go first, and it comes to nothing: the session says it
    // cannot push and GitHub still has nothing on the branch.
    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("no pull request"),
        "the run stopped on the pull request nothing could find: {:?}",
        stopped.html,
    );

    assert_eq!(
        fixture.resume().await,
        Resumed::Resumed,
        "an empty backlog is not the whole answer: the work is built and the push \
         is what is left",
    );

    let view = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping && pull_request(view).is_some())
                .then(|| view.clone())
        })
        .await;

    assert_eq!(
        pull_request(&view)
            .expect("the wrap-up has its pull request pinned")
            .number,
        41,
        "the pull request the press paid for is the one it wraps up",
    );
    assert_eq!(
        sessions_on(&fixture, "submitting/SKILL.md").await,
        2,
        "one session for the run's own go and one for the press, and neither of \
         them sent to build anything",
    );
    assert_eq!(
        view.blocked_on, None,
        "and nothing is waiting on the human any more",
    );
}

/// Resume on a backlog worked to empty whose branch is already on a pull request
/// wraps that up, rather than refusing because `.tasks/` has nothing in it.
///
/// The refusal above with the other answer behind it, and the reason the two are
/// worth telling apart. An empty backlog is a breakdown that never landed *or* a
/// feature that is finished with, and the second one has had its finish step —
/// which pushes and opens the pull request. So an empty backlog is the state a
/// **failed ending** leaves behind: the work is out on GitHub and the record
/// does not know, because recording it is what went wrong.
///
/// Before this, every way back in refused exactly that Conversation. Resume read
/// the bare `.tasks/` and said there was nothing to work; a steer into Wrapping
/// wanted the pull request the record did not have; an instruction steered in
/// asked the record too and stopped a second time. The work sat on a pull request
/// nothing would ever wrap up.
///
/// Asked of GitHub rather than of the branch, for
/// [`resuming_an_inline_run_whose_branch_has_a_pull_request_wraps_it_up_unspent`]'s
/// reason: a pull request is GitHub's fact and a branch cannot say there is one.
/// Which is why the `gh` here changes its answer mid-test while nothing about the
/// repository does.
#[tokio::test]
async fn resuming_an_emptied_backlog_whose_branch_has_a_pull_request_wraps_it_up_unspent() {
    let spill = tempfile::tempdir().unwrap();
    let opened = spill.path().join("opened-by-hand");

    let fixture = grilling_spilling(spill, A_BACKLOG_OF_ONE, &gh_opened_by_hand(&opened)).await;

    worked_to_empty(&fixture).await;

    // The run stops where a finish step with no findable pull request stops: the
    // backlog is worked through and taken away, and GitHub says there is nothing
    // on the branch.
    let missing = fixture.stopped().await;

    assert!(
        missing.html.contains("no pull request"),
        "the run stopped on the pull request nothing could find: {:?}",
        missing.html,
    );

    let spent = working_sessions(&fixture).await;

    // And then there is one — the ending that half happened, or the human
    // opening it by hand off the halt's own advice. Nothing on the branch
    // changes, and nothing needs to.
    std::fs::write(&opened, "https://github.com/tobico/verkstead/pull/41\n").unwrap();

    assert_eq!(
        fixture.resume().await,
        Resumed::Resumed,
        "the empty backlog is no longer the whole answer: GitHub has one",
    );

    let found = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping)
                .then(|| pull_request(view).cloned())
                .flatten()
        })
        .await;

    assert_eq!(
        found.number, 41,
        "the pull request the branch was already on is the one it wraps up",
    );
    assert_eq!(
        working_sessions(&fixture).await,
        spent,
        "and no session was spent working a backlog that has nothing in it",
    );
    assert_eq!(
        fixture.view().await.blocked_on,
        None,
        "and nothing is waiting on the human any more",
    );
}

/// And a Resume pressed twice is the first press arriving again.
///
/// The second one finds something driving the Conversation — which is what the
/// first one started — and refuses as such. Starting a second driver would be
/// two agents in one Worktree, which is the one thing every gate here exists to
/// stop.
#[tokio::test]
async fn a_resume_pressed_twice_is_refused_as_already_driven() {
    // The first session says its piece and goes, which is the stall; the one
    // Resume starts stays, which is what the second press has to find.
    let fixture = grilling_swept(
        r#"
        if [ ! -f TRIED ]; then
            printf 'once\n' > TRIED
            printf 'the grilling has nothing to say\n'
        else
            printf 'the grilling is running\n'
            sleep 300
        fi
        "#,
    )
    .await;

    fixture.quiet().await;
    fixture.stopped().await;

    assert_eq!(fixture.resume().await, Resumed::Resumed);

    // Waited for rather than pressed straight away: what the second press has to
    // find is the session the first one started, and starting one is the slow
    // part of a resume.
    fixture.until(|view| view.working.then_some(())).await;

    assert_eq!(fixture.resume().await, Resumed::AlreadyDriven);
}

/// Resume is offered exactly where the Conversation is in a driven state and
/// nothing is driving it.
///
/// Which is a question about the running server rather than about the record —
/// what drives a Conversation is a session or a task of Verkstead's own, and
/// neither leaves a row behind. So the page is told, rather than being left to
/// work it out from a state and a Timeline that cannot say.
#[tokio::test]
async fn resume_is_offered_exactly_where_nothing_is_driving() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");

    let fixture = grilling_spilling(
        spill,
        &format!(
            r#"
printf 'the grilling is running\n'
while [ ! -f {gate} ]; do sleep 0.05; done
"#,
            gate = quoted(&gate),
        ),
        PULL_REQUEST,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    assert!(
        !fixture.view().await.ready_to_resume,
        "a grilling is driven by its session, and this one is still talking",
    );

    // The session goes, and with it the only thing that was driving it.
    std::fs::write(&gate, "go").unwrap();

    fixture
        .until(|view| view.ready_to_resume.then_some(()))
        .await;

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Grilling,
        "which is a state something ought to be driving, and is what makes a \
         Conversation with nothing driving it worth offering the press on",
    );
}

/// A backlog run a restart interrupted carries on, with nobody having pressed
/// anything.
///
/// The Conversation is mid-task when the server goes: the step is not landed, the
/// session working it dies with the process, and nothing decided any of that. So
/// the second server reads `.tasks/` exactly as the run itself does and starts a
/// fresh session on the task that is still there — which is what a human pressing
/// Resume would have got, arrived at without the human.
///
/// The first server never stops it, which is what makes this about the restart:
/// its session goes on printing, so the run it is driving is perfectly healthy
/// right up to the moment the process would have gone.
#[tokio::test]
async fn a_restarted_server_works_the_backlog_it_was_left_implementing() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            mkdir -p .tasks
            printf '# Rate limiting\n\n- [ ] 01: Count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            git add .tasks
            git commit --quiet -m 'chore: plan the rate limiter'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            while :; do
                printf 'still working on the task\n'
                sleep 0.2
            done
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    // The task's own session, which is the run this restart interrupts.
    fixture
        .until(|view| (view.state == Lifecycle::Implementing).then_some(()))
        .await;

    let before = fixture
        .until(|view| (outputs(view).len() > 1).then(|| outputs(view).len()))
        .await;

    assert!(
        notices(&fixture.view().await).is_empty(),
        "nothing has stopped: the run is being driven, which is what makes this \
         about the restart and not about a stop",
    );

    // Which stays, as the session it replaces did: what is being asked about is
    // the restart taking the run up, and a session that printed and left would
    // be a step nothing landed — an ordinary stop, and a different question.
    let _restarted = fixture
        .restarted(
            r#"
            printf 'prompt was: %s\n' "$2"
            sleep 300
            "#,
            PULL_REQUEST,
        )
        .await;

    let taken_up = fixture
        .until(|view| {
            let sessions = outputs(view);
            (sessions.len() > before).then(|| sessions[before].id)
        })
        .await;

    let said = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id == taken_up && output.lines > 1)
                .map(|output| output.id)
        })
        .await;

    let printed = fixture.capture(said).await.replace("\r\n", "\n");

    assert!(
        printed.contains("/verkstead/skills/next-task/SKILL.md"),
        "the backlog is picked up again, which is the fork that reads it: {printed:?}",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "and the run is where it was: a restart puts things back rather than \
         moving anything on",
    );
    assert_eq!(
        view.blocked_on, None,
        "with nothing waiting on the human, because nobody had to be asked",
    );
}

/// A Conversation somebody decided to stop stays stopped across a restart.
///
/// The one thing the startup resume leaves alone, and the whole reason a stop
/// records whether anybody chose it. A step whose session ended without landing
/// it is Verkstead pulling the brake: it does not spend an account on the same
/// failure again unasked, and a server coming back up is no reason to think
/// differently about that.
///
/// So the badge is still there and the Notice is still the one that explained it —
/// and no session was launched, which is the half a badge cannot say.
#[tokio::test]
async fn a_deliberate_halt_survives_a_restart_with_its_badge_intact() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            mkdir -p .tasks
            printf '# Rate limiting\n\n- [ ] 01: Count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            git add .tasks
            git commit --quiet -m 'chore: plan the rate limiter'
            printf 'the backlog is written\n'
            sleep 300
            ;;
        *)
            printf 'this task is beyond me\n'
            exit 1
            ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    let stopped = fixture.stopped().await;

    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "a step whose session ended without landing it is Verkstead pulling the \
         brake, which is the kind of stop a restart may not overrule",
    );

    let before = outputs(&fixture.view().await).len();

    let _restarted = fixture.restarted("true", PULL_REQUEST).await;

    // Long enough for the second server to have taken up everything it was going
    // to, and for the sweep that follows it to have looked as well.
    tokio::time::sleep(BRISKLY.grace * 4).await;

    let view = fixture.view().await;

    assert_eq!(
        outputs(&view).len(),
        before,
        "no session was launched: a deliberate stop is waiting for a press, and \
         a restart is not one",
    );
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "the badge is intact, and still points at the Notice that explained it",
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "and the stop is the same stop, not one written over the top of it",
    );
    assert_eq!(
        notices(&view).len(),
        1,
        "with nothing added beside it: {:?}",
        notices(&view),
    );
}

/// A stop nobody chose is taken up by the next server unasked, badge and all.
///
/// The other half of the pair: a stall is a driver that went away rather than a
/// decision, so a server coming back is free to start the work again — and the
/// stop goes with it, because nothing is stopped any more. The Notice stays where
/// it is: it is a stop that really happened.
///
/// The stop is written rather than waited for — see [`halted_by`] —
/// so that this server writes exactly one: a sweep that went on looking would
/// stop the Conversation again while the next server was driving it.
#[tokio::test]
async fn a_halt_nobody_chose_is_driven_again_by_the_next_server() {
    let fixture = grilling(r#"printf 'the grilling has nothing to say\n'"#).await;

    fixture.quiet().await;

    halted_by(&fixture, Decision::Circumstance).await;

    let stalled = fixture.stopped().await;

    assert_eq!(
        fixture.chosen().await,
        Decision::Circumstance,
        "nobody decided this run should stop, which is what makes it a restart's \
         to pick up",
    );
    assert_eq!(fixture.view().await.blocked_on, Some(stalled.id));

    let before = outputs(&fixture.view().await).len();

    let _restarted = fixture
        .restarted(
            r#"
            printf 'prompt was: %s\n' "$2"
            sleep 300
            "#,
            PULL_REQUEST,
        )
        .await;

    let taken_up = fixture
        .until(|view| {
            let sessions = outputs(view);
            (sessions.len() > before).then(|| sessions[before].id)
        })
        .await;

    let said = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id == taken_up && output.lines > 1)
                .map(|output| output.id)
        })
        .await;

    let printed = fixture.capture(said).await.replace("\r\n", "\n");

    assert!(
        printed.contains("/verkstead/skills/grilling/SKILL.md")
            && printed.contains("The API has none."),
        "a fresh grilling on the Brief, which is what the button would have \
         started: {printed:?}",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.blocked_on, None,
        "the stop is gone with the press that was never needed: nothing is waiting \
         on the human once the Conversation is being driven again",
    );
    assert_eq!(
        notices(&view).len(),
        1,
        "and the Notice stays where it is: {:?}",
        notices(&view),
    );
}

/// And the two words in between are left alone as well: a stop the human
/// pressed, and one stored before their press and Verkstead's brake were told
/// apart.
///
/// The marks tell those two from the brake above — neither draws a dot or a
/// badge, because there is nobody to tell that they did not already know — but a
/// restart does not, and that is the point of asking here. Whoever decided, it
/// was decided, and the press that undoes it is the human's whichever mark the
/// page happens to be drawing.
///
/// The stored word is put on the record directly, which is the only way to have
/// a `deliberate` one at all: nothing writes that word any more, and what this
/// is asking is what a database written before it stopped being written still
/// does.
#[tokio::test]
async fn a_stop_the_human_pressed_survives_a_restart_quietly() {
    for decision in [Decision::Human, Decision::Deliberate] {
        let fixture = grilling(r#"printf 'the grilling has nothing to say\n'"#).await;

        fixture.quiet().await;

        halted_by(&fixture, decision).await;

        let stopped = fixture.stopped().await;
        let before = outputs(&fixture.view().await).len();

        let _restarted = fixture
            .restarted(
                r#"
                printf 'prompt was: %s\n' "$2"
                sleep 300
                "#,
                PULL_REQUEST,
            )
            .await;

        // Long enough for the second server to have taken up everything it was
        // going to, and for the sweep that follows it to have looked as well.
        tokio::time::sleep(BRISKLY.grace * 4).await;

        let view = fixture.view().await;

        assert_eq!(
            outputs(&view).len(),
            before,
            "no session was launched over a {decision:?} stop: somebody decided \
             it, and a restart is not the press that undoes one",
        );
        assert_eq!(
            view.blocked_on,
            Some(stopped.id),
            "the mark is intact, and still points at the Notice that explained it",
        );
        assert!(
            view.stopped_by_hand,
            "drawn quietly, this one being the human's own: {decision:?}",
        );
        assert!(
            !fixture.row().await.waiting,
            "and the sidebar stays quiet about it too",
        );
        assert_eq!(
            fixture.chosen().await,
            decision,
            "with the stored word exactly as it was, not rewritten by the server \
             that read it",
        );
    }
}

/// A restart that can start nothing for a Conversation says so on the Timeline
/// and stops there.
///
/// The refusal has nobody in front of it — a press answers the browser holding
/// the request open, and a server coming up answers nothing at all — so it goes
/// where the human will find it, in the words the button would have used. A
/// Conversation nothing can be started for is exactly the one somebody has to
/// look at, and *nothing is driving it* from the sweep a minute later is the same
/// Conversation described by something that knows less.
///
/// A Worktree that is not one any more and cannot be made again is the plainest
/// way to have one: a run that is implementing has everywhere to start and
/// nowhere to do it, and it is the one refusal with nothing the workbench can
/// correct. The stop the run left is taken away first, which is the human having
/// pressed Resume on it before the restart — what is being asked about here is
/// the restart's own refusal rather than a stop it would have left alone.
///
/// An emptied backlog is deliberately not the example any more: a branch that
/// wrote a backlog and worked it to empty has work on it and one push to go, so a
/// restart sends for the pull request rather than refusing — see
/// [`resuming_an_emptied_backlog_with_no_pull_request_sends_for_one_again`].
#[tokio::test]
async fn a_restart_that_can_start_nothing_halts_with_the_refusal_on_the_timeline() {
    let fixture = grilling_asking(
        &a_backlog_that_cannot_open_a_pull_request(),
        NO_PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture.stopped().await;

    let before = notices(&fixture.view().await).len();

    fixture.drive_again().await;

    // Something that is not a directory at all sitting where the Worktree goes:
    // it cannot be removed as a worktree, it is not a directory to take away, and
    // git will not check the branch out over it.
    let worktree = PathBuf::from(fixture.view().await.worktree.unwrap().path);

    std::fs::remove_dir_all(&worktree).unwrap();
    std::fs::write(&worktree, "not a worktree\n").unwrap();

    let _restarted = fixture.restarted("true", NO_PULL_REQUEST).await;

    let refused = fixture
        .until(|view| {
            let said = notices(view);
            (said.len() > before).then(|| said[before].clone())
        })
        .await;

    assert!(
        refused.contains("Implementing the work"),
        "what nobody could be started for is named as the run it was: {refused:?}",
    );
    assert!(
        refused.contains("nothing could be started for it as the server came back up"),
        "and the restart is what could not start it: {refused:?}",
    );
    assert!(
        refused.contains("git would not make it again from the branch"),
        "in the words the press refuses in, rather than the sweep's: {refused:?}",
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Verkstead,
        "Verkstead looked and decided nothing could be started, and nothing but \
         the human can change that — so the next restart leaves it alone",
    );

    // Long enough for the sweep the second server runs to have looked as well.
    tokio::time::sleep(BRISKLY.grace * 4).await;

    let view = fixture.view().await;

    assert_eq!(
        notices(&view).len(),
        before + 1,
        "one stop and one Notice, rather than a sweep writing its own over the \
         top of it: {:?}",
        notices(&view),
    );
    assert!(
        view.blocked_on.is_some(),
        "and the human is what it is waiting on",
    );
}

/// A Set the session asked without waiting for it: two Questions, so what comes
/// back is a decision and something the human wrote in their own words.
///
/// Nothing about the Set says it was deferred — that is how it was sent — so
/// this is an ordinary Set, and the Answers to it are what a later session is
/// owed.
const DEFERRED: &str = r#"
title: The wording of the rate-limit error
questions:
  - label: Q9
    text: Which status should a throttled request get?
    options:
      - n: 1
        text: 429 Too Many Requests
        recommended: true
      - n: 2
        text: 503 Service Unavailable
"#;

/// A two-task backlog whose sessions write down the prompt they were started on,
/// somewhere that outlives the worktree.
///
/// The prompts are what these tests are about, and a capture would not do: a
/// prompt is a document, and what is being asked is whether one part of it
/// reached one session and not the next.
fn a_backlog_of_two_writing_prompts(prompts: &Path) -> String {
    format!(
        r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf -- '- [ ] 02: refuse the excess\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    printf '# 02\n' > .tasks/02-refuse.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf 'I read the whole branch and found nothing worth raising\n'
        exit 0
        ;;
    esac
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    printf '===== %s\n%s\n' "${{next:-finish}}" "$2" >> {prompts}
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: $next"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
        printf 'pushed, and the pull request is open\n'
    fi
    sleep 300
    ;;
esac
"#,
        prompts = quoted(prompts),
    )
}

/// What each session was started on, as the stub above wrote them down: the step
/// it was working against the whole prompt.
fn prompts_by_step(written: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(written)
        .unwrap_or_default()
        .split("===== ")
        .filter(|block| !block.trim().is_empty())
        .map(|block| {
            let (step, prompt) = block.split_once('\n').expect("a step names its prompt");
            (step.trim().to_owned(), prompt.to_owned())
        })
        .collect()
}

/// The far end of a Deferred Ask: the session that asked it never saw the
/// Answer, and the next session started on the Conversation opens with it.
///
/// Asked during the grilling and answered before the direction is picked, which
/// is the ordinary shape of one — the Questions the work does not turn on are
/// exactly the ones a grilling can leave with the human while it gets on. What
/// this asks is that the Answers reach the first session that builds, and reach
/// nothing after it: folding is recorded, so the second task session is primed
/// with the work and not with a decision it has already been told.
#[tokio::test]
async fn an_answered_deferred_set_is_folded_into_the_next_session_and_no_later_one() {
    let spill = tempfile::tempdir().unwrap();
    let written = spill.path().join("task-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_of_two_writing_prompts(&written),
        PULL_REQUEST,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    // The grilling asks something the work does not turn on and carries on
    // without it, and the human answers it in their own time — here, before the
    // direction is picked, so that what follows is deterministic.
    let deferred = fixture.ask_deferred(DEFERRED).await;
    assert_eq!(
        fixture
            .respond(
                deferred,
                serde_json::json!([{
                    "label": "Q9",
                    "selected": 1,
                    "free_text": "and say which limit it hit",
                }])
            )
            .await,
        Submitted::Accepted,
    );

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.pick(set, "task-list").await, Submitted::Accepted);

    fixture
        .until(|view| {
            commits(view)
                .iter()
                .any(|commit| commit.subject.starts_with("chore: finish"))
                .then_some(())
        })
        .await;

    let started = prompts_by_step(&written);
    let steps: Vec<&str> = started.iter().map(|(step, _)| step.as_str()).collect();

    assert_eq!(
        steps,
        ["01-count.md", "02-refuse.md", "finish"],
        "a session per step, in order: {started:?}",
    );

    let (_, first) = &started[0];

    assert!(
        first.contains("# What I have since said about the deferred questions"),
        "the first session started after the Answers came back is the one they \
         belong to: {first:?}",
    );
    assert!(
        first.contains("429 Too Many Requests") && first.contains("and say which limit it hit"),
        "and it is the exchange itself — the Option picked and what the human \
         wrote beside it: {first:?}",
    );
    assert!(
        first.contains("# The Brief this started from"),
        "under the documents the prompt is built from, where the newest and \
         least general thing said goes: {first:?}",
    );

    for (step, prompt) in &started[1..] {
        assert!(
            !prompt.contains("deferred questions"),
            "each Answer is folded once: {step} was told again: {prompt:?}",
        );
    }
}

/// A `gh` that answers for two repositories at once with a suite of its own in
/// each: `own` is what it says about the checks where the work's own repository
/// is, and `companion` what it says in `askance`.
///
/// [`gh_alongside`] one step further out. That one is about finding the pull
/// requests; this is about what is happening to them afterwards, which is a
/// different answer per repository — a suite runs against a branch in a
/// repository, and the two have nothing to do with each other.
fn gh_alongside_checking(own: &str, companion: &str) -> String {
    format!(
        r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$(pwd -P)" in
*/askance*)
    case "$5" in
    *statusCheckRollup*)
{companion}
        ;;
    *commits*)
        printf '{{"commits":[],"comments":[]}}'
        ;;
    *comments*)
        printf '{{"comments":[],"reviews":[]}}'
        ;;
    *)
        printf '{{"number":7,"title":"The other half","url":"https://github.com/tobico/askance/pull/7"}}'
        ;;
    esac
    exit 0
    ;;
esac
case "$5" in
*statusCheckRollup*)
{own}
    ;;
*commits*)
    printf '{{"commits":[],"comments":[]}}'
    ;;
*comments*)
    printf '{{"comments":[],"reviews":[]}}'
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#
    )
}

/// A red suite, as an answer about the checks — whichever repository it is asked
/// in.
///
/// The same check name in both, which is the whole point of asking twice: `Rust`
/// red on two pull requests is two different failures, and neither of them
/// spends the other's attempts.
const RED: &str = r#"    printf '{"mergeable":"MERGEABLE","statusCheckRollup":[{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"FAILURE","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}]}'"#;

/// One that is red until `fixed` is there and green once it is, which is what a
/// fix session that reached the right pull request does to it.
fn red_until(fixed: &Path) -> String {
    format!(
        r#"    if [ -s {fixed} ]; then how=SUCCESS; else how=FAILURE; fi
    printf '{{"mergeable":"MERGEABLE","statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"%s","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$how""#,
        fixed = quoted(fixed),
    )
}

/// The backlog worked alongside a companion and carried to two pull requests,
/// plus the sessions a wrap-up runs: a review that finds nothing worth raising,
/// and a fix session that writes down the prompt it was given.
///
/// The fix session says which pull request it was sent at by leaving a marker
/// named for it — `#7` is the companion's and anything else is the work's own —
/// so a `gh` beside it can turn *that* repository's suite green, and a fix that
/// reached the wrong one is visible from outside.
///
/// And it says whether it was ever in there beside another: `busy` exists for as
/// long as one is working, so a second session started over the top of one
/// writes down that they collided.
fn a_backlog_alongside_then_fixes(
    dispatched: &Path,
    busy: &Path,
    own: &Path,
    companion: &Path,
) -> String {
    format!(
        r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf 'I read the whole branch and found nothing worth raising\n'
        exit 0
        ;;
    *addressing/SKILL.md*)
        if [ -e {busy} ]; then printf 'COLLIDED\n' >> {dispatched}; fi
        printf 'x' > {busy}
        printf '%s\n=====\n' "$2" >> {dispatched}
        case "$2" in
        *"pull request #7"*) printf 'x' > {companion} ;;
        *) printf 'x' > {own} ;;
        esac
        printf 'having a go at the check\n'
        printf 'a fix\n' >> fixes.md
        git add -A
        git commit --quiet -m 'fix: have a go at the failing check'
        rm -f {busy}
        sleep 300
        ;;
    esac
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: count the requests"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
        cd ../askance-*
        printf 'the other half\n' > halves.md
        git add halves.md
        git commit --quiet -m 'feat: the other half'
        printf 'pushed both, and the pull requests are open\n'
    fi
    sleep 300
    ;;
esac
"#,
        busy = quoted(busy),
        dispatched = quoted(dispatched),
        own = quoted(own),
        companion = quoted(companion),
    )
}

/// A red check on a companion's pull request is fixed in that companion's
/// worktree, and the run stops naming the pull request that would not go green.
///
/// Which is the whole of what a fix session has to be told once a Conversation
/// holds more than one pull request. It starts in the Conversation's own
/// worktree and `gh` reads its repository from wherever it runs, so a session
/// sent at `#7` and left to work where it landed would ask `verkstead` how
/// `askance`'s checks were getting on — and be told about a suite that is green.
///
/// Two goes and then the human, per pull request: the work's own is green
/// throughout and has nothing dispatched about it, and the two the companion
/// spends are its own.
#[tokio::test]
async fn a_red_check_on_a_companions_pull_request_is_fixed_in_that_companions_worktree() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let busy = spill.path().join("busy");
    let own = spill.path().join("fixed-own");
    let companion = spill.path().join("fixed-companion");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_fixes(&dispatched, &busy, &own, &companion),
        "askance",
        &gh_alongside_checking(GREEN, RED),
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;
    let worktree = fixture.view().await.companions[0]
        .worktree
        .clone()
        .expect("the companion is checked out")
        .path;

    let told = std::fs::read_to_string(&dispatched).expect("both fix sessions wrote their prompt");
    let prompts: Vec<&str> = told
        .split("=====")
        .filter(|it| !it.trim().is_empty())
        .collect();

    assert_eq!(
        prompts.len(),
        2,
        "two goes at the companion's check and no more: {told}",
    );

    for prompt in &prompts {
        assert!(
            prompt.contains("#7") && prompt.contains("askance"),
            "the session is told which pull request in which repository: {prompt}",
        );
        assert!(
            prompt.contains(&worktree),
            "and the worktree to work in, {worktree} being where that repository's \
             branch is: {prompt}",
        );
    }

    assert!(
        stopped.html.contains("#7") && stopped.html.contains("askance"),
        "the Notice says which pull request would not go green: {:?}",
        stopped.html,
    );

    assert!(
        !own.exists(),
        "nothing was ever dispatched about the work's own pull request, it having \
         been green from the first poll",
    );
    assert_eq!(
        attempts_spent(&fixture, "Rust").await,
        0,
        "and the work's own `Rust` spent nothing: it was green the whole time, and \
         the companion's two are the companion's",
    );
}

/// Two red pull requests queue rather than collide: one fix session at a time,
/// and the second dispatched once the Worktree is free.
///
/// Both of them are red from the moment the wrap-up starts and each goes green on
/// its own fix, so each gets exactly one session — and the two sessions are the
/// two watchers taking the Conversation's Turn in turn. A watcher that dispatched
/// without it would put two agents in one sandbox, which is what the marker file
/// here would catch.
///
/// And then the wrap-up ends, which it could only do with both suites green.
#[tokio::test]
async fn two_red_pull_requests_are_fixed_one_session_at_a_time() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let busy = spill.path().join("busy");
    let own = spill.path().join("fixed-own");
    let companion = spill.path().join("fixed-companion");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_fixes(&dispatched, &busy, &own, &companion),
        "askance",
        &gh_alongside_checking(&red_until(&own), &red_until(&companion)),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let told = std::fs::read_to_string(&dispatched).expect("both pull requests were fixed");

    assert!(
        !told.contains("COLLIDED"),
        "two fix sessions were in the Worktree at once: {told}",
    );

    let prompts: Vec<&str> = told
        .split("=====")
        .filter(|it| !it.trim().is_empty())
        .collect();

    assert!(
        prompts
            .iter()
            .any(|prompt| prompt.contains("#41") && prompt.contains("verkstead")),
        "the work's own pull request had a session of its own: {told}",
    );
    assert!(
        prompts
            .iter()
            .any(|prompt| prompt.contains("#7") && prompt.contains("askance")),
        "and so did the companion's: {told}",
    );

    let view = fixture.view().await;

    assert!(
        notices(&view).is_empty(),
        "and nothing stopped: both suites went green on their fix — {:?}",
        notices(&view),
    );
}

/// A wrap-up is over when *every* pull request's checks are green, and not
/// before.
///
/// The work's own is green from the first poll and the companion's suite is still
/// running, which is neither red nor green: nothing is dispatched about it and
/// nothing is settled either. A rule that waited on the Conversation's own alone
/// would have finished here — with a pull request nobody had heard back about.
#[tokio::test]
async fn a_wrap_up_waits_for_every_pull_requests_checks() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let busy = spill.path().join("busy");
    let own = spill.path().join("fixed-own");
    let running = spill.path().join("companion-finished");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_fixes(&dispatched, &busy, &own, &running),
        "askance",
        &gh_alongside_checking(GREEN, &green_after(&running)),
    )
    .await;

    worked_to_empty(&fixture).await;

    let deadline = Instant::now() + *PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the work's own checks never settled: {}",
            standing(&fixture.view().await),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Long enough for many polls of a wrap-up with one suite green and one still
    // running.
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        !companion_checks_settled(&fixture).await,
        "a suite that has not finished settles nothing",
    );
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "so the Conversation waits, whatever its own pull request is doing",
    );
    assert!(
        !dispatched.exists(),
        "and nothing was dispatched about either of them: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );

    // And now the companion's finishes, green.
    std::fs::write(&running, "x").unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;
}

/// And it waits on every pull request *merging* the same way, a companion's
/// conflict being as much a reason to wait as the Conversation's own.
///
/// Both suites are green from the first poll and the companion's pull request is
/// the one its base has moved under, so the only thing between this wrap-up and
/// Done is a conflict in a repository the Conversation is not even in. A rule
/// that read the Conversation's own pull request alone would have finished here,
/// over work half of which nobody could land.
#[tokio::test]
async fn a_wrap_up_waits_for_every_pull_request_to_merge() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let busy = spill.path().join("busy");
    let own = spill.path().join("fixed-own");
    let companion = spill.path().join("fixed-companion");
    let resolved = spill.path().join("conflict-resolved");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_fixes(&dispatched, &busy, &own, &companion),
        "askance",
        &gh_alongside_checking(GREEN, &green_but_conflicting_until(&resolved)),
    )
    .await;

    worked_to_empty(&fixture).await;

    let deadline = Instant::now() + *PATIENCE;
    while !companion_checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the companion's checks never settled: {}",
            standing(&fixture.view().await),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Long enough for many polls of two green suites, one of them on a pull
    // request that will not merge.
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        merge_settled(&fixture).await,
        "the Conversation's own pull request merges, and settles",
    );
    assert!(
        !companion_merge_settled(&fixture).await,
        "and the companion's is the conflict the wrap-up is waiting on",
    );
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "so the Conversation waits, green as its own pull request is",
    );
    assert!(
        !fixture.view().await.waiting_on_checks,
        "and it is not waiting on checks: both suites came in",
    );

    // And now somebody resolves it.
    std::fs::write(&resolved, "x").unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;
}

/// A conflict on a companion's pull request is merged away in that companion's
/// worktree, and the Conversation's own is left alone.
///
/// Which is the whole of what a resolution session has to be told once a
/// Conversation holds more than one pull request. It starts in the
/// Conversation's own worktree and `git` reads its repository from wherever it
/// runs, so a session sent at `#7` and left where it landed would merge
/// `verkstead`'s base into `verkstead`'s branch — a change to work nobody asked
/// about, and the conflict still there afterwards.
#[tokio::test]
async fn a_conflicted_companion_pull_request_is_merged_in_that_companions_worktree() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let busy = spill.path().join("busy");
    let own = spill.path().join("fixed-own");
    let companion = spill.path().join("fixed-companion");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_fixes(&dispatched, &busy, &own, &companion),
        "askance",
        &gh_alongside_checking(GREEN, &green_but_conflicting_until(&companion)),
    )
    .await;

    worked_to_empty(&fixture).await;

    // The session went to the companion, so `gh` there now says the pull
    // request merges — and that is the last thing the wrap-up was waiting on.
    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let worktree = fixture.view().await.companions[0]
        .worktree
        .clone()
        .expect("the companion is checked out")
        .path;

    let told =
        std::fs::read_to_string(&dispatched).expect("the resolution session wrote its prompt");
    let told = prompts(&told);

    assert_eq!(told.len(), 1, "one conflict, one session: {told:?}",);
    assert!(
        told[0].contains("#7") && told[0].contains("askance"),
        "told which pull request in which repository: {:?}",
        told[0],
    );
    assert!(
        told[0].contains(&worktree),
        "and the worktree to do the merge in, {worktree} being where that \
         repository's branch is: {:?}",
        told[0],
    );
    assert!(
        told[0].contains("Merge the pull request's base branch"),
        "and what to do about it: {:?}",
        told[0],
    );
    assert!(
        !own.exists(),
        "nothing was ever dispatched about the work's own pull request, which was \
         green and merging from the first poll",
    );
}

/// Two conflicted pull requests queue rather than collide: one resolution
/// session at a time, and the second dispatched once the Worktree is free.
///
/// Both of them have had their base move under them and each merges once its own
/// session has been, so each gets exactly one — and the two sessions are the two
/// watchers taking the Conversation's Turn in turn. A watcher that dispatched
/// without it would put two agents in one sandbox, which is what the marker file
/// here would catch.
///
/// And then the wrap-up ends, which it could only do with both pull requests
/// merging.
#[tokio::test]
async fn two_conflicted_pull_requests_are_merged_one_session_at_a_time() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let busy = spill.path().join("busy");
    let own = spill.path().join("fixed-own");
    let companion = spill.path().join("fixed-companion");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_fixes(&dispatched, &busy, &own, &companion),
        "askance",
        &gh_alongside_checking(
            &green_but_conflicting_until(&own),
            &green_but_conflicting_until(&companion),
        ),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let told = std::fs::read_to_string(&dispatched).expect("both conflicts were merged away");

    assert!(
        !told.contains("COLLIDED"),
        "two resolution sessions were in the Worktree at once: {told}",
    );

    let told = prompts(&told);

    assert!(
        told.iter()
            .any(|prompt| prompt.contains("#41") && prompt.contains("verkstead")),
        "the work's own pull request had a session of its own: {told:?}",
    );
    assert!(
        told.iter()
            .any(|prompt| prompt.contains("#7") && prompt.contains("askance")),
        "and so did the companion's: {told:?}",
    );
    assert_eq!(
        told.len(),
        2,
        "one each and no more, each conflict going away on its own session: {told:?}",
    );
}

/// What each resolution session is told to *do* is the strategy configured for
/// the repository its pull request is in: the setting every Repo shares, unless
/// that Repo has been given one of its own.
///
/// Two conflicts in one wrap-up, the settings file asking for a rebase and the
/// companion Repo overriding it back to a merge. So the two prompts have to
/// differ — one telling its session to rebase and force-push with a lease, the
/// other to merge and never force-push — which is the whole of the setting doing
/// anything: a strategy that never reached the session would be a picker that
/// wrote a word in a file.
///
/// The stub sessions merge either way, this being about what they are told
/// rather than about git. What a rebase costs is said on the settings page,
/// beside the choice, and is why the merge is what nobody choosing anything
/// gets.
#[tokio::test]
async fn each_resolution_session_is_told_the_strategy_its_repo_resolves_by() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let busy = spill.path().join("busy");
    let own = spill.path().join("fixed-own");
    let companion = spill.path().join("fixed-companion");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_fixes(&dispatched, &busy, &own, &companion),
        "askance",
        &gh_alongside_checking(
            &green_but_conflicting_until(&own),
            &green_but_conflicting_until(&companion),
        ),
    )
    .await;

    // Both said before the wrap-up is anywhere near: the settings file is read
    // as each session is dispatched, so what matters is that they are there by
    // then.
    configure(&fixture, "conflict_resolution: rebase\n");
    told_to_resolve_by(
        &fixture,
        companion_repo(&fixture).await,
        ConflictResolution::Merge,
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let told = std::fs::read_to_string(&dispatched).expect("both conflicts were resolved");
    let told = prompts(&told);

    let own = told
        .iter()
        .find(|prompt| prompt.contains("#41") && prompt.contains("verkstead"))
        .expect("the work's own pull request had a session of its own");

    assert!(
        own.contains("Rebase the branch") && own.contains("--force-with-lease"),
        "the Repo that overrides nothing resolves the way the settings file says, \
         which here is a rebase: {own}",
    );

    let companion = told
        .iter()
        .find(|prompt| prompt.contains("#7") && prompt.contains("askance"))
        .expect("and so did the companion's");

    assert!(
        companion.contains("Merge the pull request's base branch")
            && companion.contains("rather than a rebase"),
        "and the Repo that was given one of its own resolves by that, whatever \
         every other Repo does: {companion}",
    );
}

/// The same check name red on two pull requests gets two fix sessions each: two
/// different failures, and neither of them spending the other's attempts.
///
/// `Rust` is red on both from the first poll and stays red, so this is *two
/// attempts, then ask the human* running twice over — which a count kept per
/// check alone would have cut to two sessions altogether, with one of the two
/// pull requests never looked at.
///
/// All four go, whatever order they go in. Which watcher takes the Turn next is
/// nobody's to say — both poll on the same interval, and the one whose session
/// has just ended is as likely to take it again as the one that has been waiting
/// — so what makes each pull request spend its own two is the stop waiting for
/// the other: a watcher out of goes does not stop a run that still has somewhere
/// to go.
#[tokio::test]
async fn the_same_check_red_on_two_pull_requests_gets_two_goes_each() {
    let spill = tempfile::tempdir().unwrap();
    let dispatched = spill.path().join("fix-prompts");
    let busy = spill.path().join("busy");
    let own = spill.path().join("fixed-own");
    let companion = spill.path().join("fixed-companion");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_fixes(&dispatched, &busy, &own, &companion),
        "askance",
        &gh_alongside_checking(RED, RED),
    )
    .await;

    worked_to_empty(&fixture).await;

    // Waited for by the count rather than read at the first Notice. The stop is
    // written once both pull requests have run out of goes, so a Notice is
    // evidence that all four were dispatched — but a session that has been
    // dispatched is not yet a session that has written anything down, and the
    // last of them is still starting up as the stop lands.
    let told = until_written_by(&dispatched, 4).await;
    let stopped = fixture.stopped().await;
    let prompts = prompts(&told);

    let about = |number: &str| {
        prompts
            .iter()
            .filter(|prompt| prompt.contains(number))
            .count()
    };

    assert_eq!(
        (about("#41"), about("#7")),
        (2, 2),
        "two goes at each pull request, and no more: {told}",
    );

    assert!(
        stopped.html.contains("pull request #"),
        "and the Notice says which one would not go green: {:?}",
        stopped.html,
    );
    assert_eq!(
        attempts_spent(&fixture, "Rust").await,
        2,
        "the work's own `Rust` spent its own two, whatever the companion's spent",
    );
}

/// A `gh` that answers for two repositories at once with a conversation of its
/// own in each: `own` is what has been said where the work's own repository is,
/// and `companion` what has been said in `askance`.
///
/// [`gh_alongside_checking`]'s other half. A human writes on the pull request
/// they are reading and a Conversation now ends on one per repository it was
/// worked in, so what has been said is a different answer per repository — and
/// `#7` is a number in one of them and nothing at all in the other.
///
/// The checks are green in both, so that what holds a wrap-up up in these tests
/// is what was said and nothing else.
fn gh_alongside_saying(own: &str, companion: &str) -> String {
    format!(
        r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$(pwd -P)" in
*/askance*)
    case "$5" in
    *statusCheckRollup*)
{GREEN}
        ;;
    *commits*)
        printf '{{"commits":[],"comments":[]}}'
        ;;
    *comments*)
{companion}
        ;;
    *)
        printf '{{"number":7,"title":"The other half","url":"https://github.com/tobico/askance/pull/7"}}'
        ;;
    esac
    exit 0
    ;;
esac
case "$5" in
*statusCheckRollup*)
{GREEN}
    ;;
*commits*)
    printf '{{"commits":[],"comments":[]}}'
    ;;
*comments*)
{own}
    ;;
*)
    printf '{{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}}'
    ;;
esac
"#
    )
}

/// Nothing said, as an answer about one pull request's conversation.
const NOTHING_SAID: &str = r#"    printf '{"comments":[],"reviews":[]}'"#;

/// One comment said in it, `id` being what GitHub calls the comment — which is
/// what tells one pull request's from another's in the record.
fn saying(id: &str, body: &str) -> String {
    format!(
        r#"    printf '{{"comments":[{{"id":"{id}","author":{{"login":"tobico"}},"body":"{body}","createdAt":"2026-08-21T09:00:00Z"}}],"reviews":[]}}'"#
    )
}

/// The same, said only once `after` is there with something in it — which is how
/// a test says a comment landed after the review had started, and so is a batch
/// session's rather than the review's.
fn saying_once(after: &Path, id: &str, body: &str) -> String {
    format!(
        r#"    if [ -s {after} ]; then
{said}
    else
{nothing}
    fi"#,
        after = quoted(after),
        said = saying(id, body),
        nothing = NOTHING_SAID,
    )
}

/// And one that cannot be asked what was said at all until `logged_in` is there,
/// and answers *nothing was said* once it is.
///
/// An account whose login has expired, which is the ordinary way this goes wrong
/// on a machine nobody is sitting at — and the one answer that is neither
/// *something was said* nor *nothing was*.
fn nothing_said_once_asked(logged_in: &Path) -> String {
    format!(
        r#"    if [ -s {logged_in} ]; then
{nothing}
    else
        printf 'gh: To use GitHub CLI, run: gh auth login\n' >&2
        exit 1
    fi"#,
        logged_in = quoted(logged_in),
        nothing = NOTHING_SAID,
    )
}

/// The backlog worked alongside a companion and carried to two pull requests,
/// plus the two sessions a wrap-up's reading runs: a review that writes down the
/// prompt it was given and finds nothing worth raising, and a batch session that
/// writes down its own and then does whatever `responding` says.
///
/// [`a_backlog_alongside_then_fixes`]'s other half, and the same backlog: the
/// finish commits in the companion beside the work's own repository, which is
/// what makes the wrap-up cover two pull requests rather than one.
fn a_backlog_alongside_then_answers(reviews: &Path, batches: &Path, responding: &str) -> String {
    a_backlog_alongside_then_reviews(reviews, batches, REVIEW_AND_FIND_NOTHING, responding)
}

/// The same again, with a review of the caller's choosing.
///
/// What the tests about one review across two pull requests need: reading the
/// prompt it was given is one half of what a review of work in two repositories
/// does, and what it lands in each of their worktrees is the other.
fn a_backlog_alongside_then_reviews(
    reviews: &Path,
    batches: &Path,
    review: &str,
    responding: &str,
) -> String {
    let review = review.replace("WHILE_NOBODY_HAS_ASKED", WHILE_NOBODY_HAS_ASKED);
    let responding = responding.replace("WHILE_NOBODY_HAS_ASKED", WHILE_NOBODY_HAS_ASKED);

    format!(
        r#"
case "$1" in
claude-grilling-5)
    printf 'grilling\n'
    mkdir -p .tasks
    printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
    printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
    printf '# 01\n' > .tasks/01-count.md
    git add .tasks
    git commit --quiet -m 'chore: plan rate-limiting tasks'
    sleep 300
    ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf '%s\n=====\n' "$2" >> {reviews}
{review}
        exit 0
        ;;
    *responding/SKILL.md*)
        printf '%s\n=====\n' "$2" >> {batches}
{responding}
        exit 0
        ;;
    esac
    number=$(sed -n 's/^- \[ \] \([0-9]*\):.*/\1/p' .tasks/TODO.md | head -n 1)
    next=$(ls .tasks | grep -E "^$number-" | head -n 1)
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
        git add -A
        git commit --quiet -m "feat: count the requests"
    else
        git rm --quiet -r .tasks
        git commit --quiet -m 'chore: finish rate-limiting'
        cd ../askance-*
        printf 'the other half\n' > halves.md
        git add halves.md
        git commit --quiet -m 'feat: the other half'
        printf 'pushed both, and the pull requests are open\n'
    fi
    sleep 300
    ;;
esac
"#,
        reviews = quoted(reviews),
        batches = quoted(batches),
    )
}

/// A comment left on a companion's pull request is answered by a session that is
/// told which repository, which pull request and which worktree it is answering
/// in.
///
/// Which is the whole of what a batch session has to be told once a Conversation
/// holds more than one pull request. It starts in the Conversation's own worktree
/// and both `git` and `gh` read their repository from wherever they run, so a
/// session sent at `#7` and left to work where it landed would read `verkstead`'s
/// diff and push its answer onto `verkstead`'s branch.
///
/// The comment lands after the review has started, so it is a batch's rather than
/// the review's — everything standing when the review starts is the review's to
/// propose about.
#[tokio::test]
async fn a_comment_on_a_companions_pull_request_is_answered_in_that_companions_worktree() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let batches = spill.path().join("batch-prompts");

    let said = saying_once(&reviews, "IC_7", "This is the wrong way round.");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_answers(&reviews, &batches, RESPOND_AND_FIND_NOTHING),
        "askance",
        &gh_alongside_saying(NOTHING_SAID, &said),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let told = std::fs::read_to_string(&batches).expect("the batch session wrote its prompt");
    let worktree = fixture.view().await.companions[0]
        .worktree
        .clone()
        .expect("the companion is checked out")
        .path;

    assert!(
        told.contains("#7") && told.contains("askance"),
        "the session is told which pull request in which repository was commented \
         on: {told}",
    );
    assert!(
        told.contains(&worktree),
        "and the worktree to work in, {worktree} being where that repository's \
         branch is: {told}",
    );
    assert!(
        told.contains("This is the wrong way round."),
        "with what was actually said under it: {told}",
    );

    assert_eq!(
        addressed_on(&fixture, companion_repo(&fixture).await).await,
        vec!["IC_7".to_owned()],
        "and the comment is written down as dealt with against the pull request it \
         was left on",
    );
    assert!(
        addressed(&fixture).await.is_empty(),
        "rather than against the Conversation's own, which nobody said anything on",
    );
}

/// A rule covers a companion's pull request exactly as it covers the work's own:
/// the comment is skipped, no session is sent about it, and it is written down as
/// addressed against the pull request it was left on.
///
/// One rule list for the whole workbench rather than one per repository. A
/// Conversation ends on a pull request per repository it was worked in, and every
/// one of them is read through the same reader — so a bot the human has silenced
/// is silenced wherever it writes, which is what they meant by writing the rule
/// once.
///
/// The comment lands after the review has started, so it is a batch's rather than
/// the review's: everything standing when the review starts is the review's to
/// propose about.
#[tokio::test]
async fn a_rule_covers_a_companions_pull_request_as_much_as_the_works_own() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let batches = spill.path().join("batch-prompts");

    let said = saying_once(&reviews, "IC_7", "nit: this reads oddly.");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_answers(&reviews, &batches, RESPOND_AND_FIND_NOTHING),
        "askance",
        &gh_alongside_saying(NOTHING_SAID, &said),
    )
    .await;

    // A rule about what was said rather than who said it, which is the half of a
    // rule that has nothing to do with which repository it was said in.
    configure(&fixture, "ignored_comments:\n  - body: '^nit:'\n");

    worked_to_empty(&fixture).await;

    until_written(&reviews).await;

    let companion = companion_repo(&fixture).await;
    let deadline = Instant::now() + *PATIENCE;

    while addressed_on(&fixture, companion).await != ["IC_7"] {
        assert!(
            Instant::now() < deadline,
            "the rule never reached the companion's pull request: {:?}",
            addressed_on(&fixture, companion).await,
        );
        pause(Duration::from_millis(25)).await;
    }

    assert!(
        !batches.exists(),
        "and no session was sent about it: {:?}",
        std::fs::read_to_string(&batches).ok(),
    );

    // Which is a wrap-up with nothing left outstanding on either pull request,
    // so it finishes on its own.
    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;
}

/// One review reads the whole of the work, so it is told where the whole of it
/// is: every pull request the Conversation holds, each with its number, the
/// repository it was opened in, its URL and the worktree to read it in.
///
/// Which is what a session that starts in the Conversation's own worktree cannot
/// work out for itself. Both `git` and `gh` read their repository from wherever
/// they are run, so a review left where it landed would read `verkstead`'s diff
/// twice and `askance`'s never — and the seam between the two halves of the work
/// is exactly what one session reading both is for.
#[tokio::test]
async fn the_review_is_told_every_pull_request_and_where_to_read_it() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let batches = spill.path().join("batch-prompts");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_answers(&reviews, &batches, RESPOND_AND_FIND_NOTHING),
        "askance",
        &gh_alongside_saying(NOTHING_SAID, NOTHING_SAID),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let told = std::fs::read_to_string(&reviews).expect("the review session wrote its prompt");
    let view = fixture.view().await;
    let own = view.worktree.clone().expect("the work has a worktree").path;
    let companion = view.companions[0]
        .worktree
        .clone()
        .expect("the companion is checked out")
        .path;

    assert!(
        told.contains("#41") && told.contains("#7") && told.contains("askance"),
        "each pull request by its number and the repository it was opened in: {told}",
    );
    assert!(
        told.contains("https://github.com/tobico/verkstead/pull/41")
            && told.contains("https://github.com/tobico/askance/pull/7"),
        "with the URL of each, rather than a number built onto one repository's: \
         {told}",
    );
    assert!(
        told.contains(&own) && told.contains(&companion),
        "and the worktree to read each of them in, {companion} being where the \
         companion's branch is: {told}",
    );

    assert_eq!(
        prompts(&told).len(),
        1,
        "one review across the lot of them, and one only: {told}",
    );
}

/// A review that splits a finding out still sends the work back to be built, and
/// the second wrap's review is told every pull request again.
///
/// The one move down the ladder, with a companion beside it: the list is worked
/// like any other, the finish that follows the last task wraps the work up again
/// on the pull requests it already had, and the review that reads it afresh knows
/// nothing of the first — including where the other half of the work is, which is
/// why it is told again rather than remembered.
#[tokio::test]
async fn the_second_wrap_of_a_split_out_backlog_reviews_every_pull_request_again() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let batches = spill.path().join("batch-prompts");
    let once = spill.path().join("split-written");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_reviews(
            &reviews,
            &batches,
            &review_then_split(&once, ""),
            RESPOND_AND_FIND_NOTHING,
        ),
        "askance",
        &gh_alongside_saying(NOTHING_SAID, NOTHING_SAID),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    let set = fixture.ask(REVIEW_WITH_A_SPLIT).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 2 },
                    { "label": "Q2", "selected": 2 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    fixture
        .until(|view| (moves_into(view, Lifecycle::Implementing) == 2).then_some(()))
        .await;
    fixture
        .until(|view| (moves_into(view, Lifecycle::Wrapping) == 2).then_some(()))
        .await;

    let deadline = Instant::now() + *PATIENCE;
    let read_again = loop {
        let written = std::fs::read_to_string(&reviews).unwrap_or_default();

        if prompts(&written).len() > 1 {
            break written;
        }

        assert!(
            Instant::now() < deadline,
            "the second wrap never read the work: {written}",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    let read_again = prompts(&read_again);
    let companion = fixture.view().await.companions[0]
        .worktree
        .clone()
        .expect("the companion is checked out")
        .path;

    assert_eq!(read_again.len(), 2, "one review per wrap: {read_again:?}");
    assert!(
        read_again[1].contains("#41")
            && read_again[1].contains("#7")
            && read_again[1].contains(&companion),
        "and the second is told every pull request the work is on and where to \
         read each, exactly as the first was: {:?}",
        read_again[1],
    );
}

/// A review that accepts findings in two repositories lands them in both, and
/// the wrap-up settles over the lot of it.
///
/// One review, one Set, and what it was answered carried out wherever each
/// finding was about: the session commits in the worktree it started in and in
/// the companion's beside it, and both commits reach the Timeline saying which
/// repository they came from. Nothing is dispatched to fix anything and nothing
/// reviews anything a second time — the session that raised the findings is the
/// one that lands them, however many repositories they were spread across.
#[tokio::test]
async fn a_review_that_accepts_findings_in_two_repositories_lands_them_in_both() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let batches = spill.path().join("batch-prompts");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_reviews(
            &reviews,
            &batches,
            REVIEW_THEN_FIX_BOTH,
            RESPOND_AND_FIND_NOTHING,
        ),
        "askance",
        &gh_alongside_saying(NOTHING_SAID, NOTHING_SAID),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_asking(&fixture, &reviews).await;

    // The review is up and waiting on the human, which is the Set the test puts
    // on its behalf and the answer it writes the marker for. Put once the session
    // is reading, because a proposal standing before there is a session behind it
    // is exactly what a wrap-up stops over.
    let set = fixture.ask(REVIEW).await;

    assert_eq!(
        fixture
            .respond(
                set,
                serde_json::json!([
                    { "label": "Q1", "selected": 1 },
                    { "label": "Q2", "selected": 1 },
                ]),
            )
            .await,
        Submitted::Accepted,
    );

    std::fs::write(handoff_directory(&fixture).join("answered"), "").unwrap();

    let view = fixture
        .until(|view| (fixes(view) == 2).then(|| view.clone()))
        .await;

    let landed = commits(&view);
    let named = |subject: &str| {
        landed
            .iter()
            .find(|commit| commit.subject == subject)
            .unwrap_or_else(|| panic!("no commit called {subject:?} among {landed:#?}"))
    };

    assert_eq!(
        named("fix: reset the counter as the window rolls").repo,
        None,
        "the fix made in the work's own repository draws unlabelled",
    );
    assert_eq!(
        named("fix: take the other half with it").repo,
        Some("askance".to_owned()),
        "and the one made in the companion says which repository it landed in",
    );

    let deadline = Instant::now() + *PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the session landed both fixes and the review never settled",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        1,
        "one review across both of them, and nothing reads either a second time",
    );
    assert!(
        notices(&view).is_empty(),
        "nothing stopped: {:?}",
        notices(&view),
    );
}

/// What was already said on *every* pull request when the review starts is part
/// of what that session reads, and nothing is dispatched to act on any of it.
///
/// One review across the whole of the work, so a review given the Conversation's
/// own pull request alone would leave the companion's comments for a batch
/// session to be sent about ungated — the thing the review folding them in exists
/// to prevent.
///
/// And each comment says which pull request it was left on, because that is what
/// it means: *this is the wrong way round* is an instruction with the repository
/// and a riddle without it.
#[tokio::test]
async fn the_review_is_given_what_was_said_on_every_pull_request() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let batches = spill.path().join("batch-prompts");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_answers(&reviews, &batches, RESPOND_AND_FIND_NOTHING),
        "askance",
        &gh_alongside_saying(
            &saying("IC_41", "Rename the window field."),
            &saying("IC_7", "This is the wrong way round."),
        ),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let told = std::fs::read_to_string(&reviews).expect("the review session wrote its prompt");

    assert!(
        told.contains("Rename the window field.") && told.contains("This is the wrong way round."),
        "the review is given what was said on both of them: {told}",
    );
    assert!(
        told.contains("#41") && told.contains("#7") && told.contains("askance"),
        "each of it saying which pull request it was left on: {told}",
    );

    assert!(
        !batches.exists(),
        "and nothing was dispatched about any of it: {:?}",
        std::fs::read_to_string(&batches).ok(),
    );

    assert_eq!(
        addressed(&fixture).await,
        vec!["IC_41".to_owned()],
        "both are written down as dealt with, against the pull request each was \
         left on",
    );
    assert_eq!(
        addressed_on(&fixture, companion_repo(&fixture).await).await,
        vec!["IC_7".to_owned()],
    );
}

/// A wrap-up is over when nothing is left unaddressed on *every* pull request,
/// and not before.
///
/// The work's own pull request is quiet from the first poll and the companion's
/// cannot be asked what has been said on it at all — which is neither *something
/// was said* nor *nothing was*, so nothing is settled about it and nothing is
/// dispatched. A rule that settled *the* comments would have finished here, with
/// a pull request Verkstead had never managed to read.
#[tokio::test]
async fn a_wrap_up_waits_for_every_pull_requests_comments() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let batches = spill.path().join("batch-prompts");
    let logged_in = spill.path().join("logged-in");

    let fixture = grilling_spilling_alongside(
        spill,
        &a_backlog_alongside_then_answers(&reviews, &batches, RESPOND_AND_FIND_NOTHING),
        "askance",
        &gh_alongside_saying(NOTHING_SAID, &nothing_said_once_asked(&logged_in)),
    )
    .await;

    worked_to_empty(&fixture).await;

    let deadline = Instant::now() + *PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the work's own pull request never went quiet: {}",
            standing(&fixture.view().await),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Long enough for many polls of a wrap-up with one pull request quiet and one
    // that cannot be read at all.
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        !companion_comments_settled(&fixture).await,
        "a pull request nobody could ask about settles nothing",
    );
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Wrapping,
        "so the Conversation waits, whatever its own pull request is doing",
    );

    // And now `gh` can be asked, and finds nothing said.
    std::fs::write(&logged_in, "x").unwrap();

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    assert!(
        !batches.exists(),
        "nothing was ever dispatched: nobody said anything on either of them — {:?}",
        std::fs::read_to_string(&batches).ok(),
    );
}

/// A Conversation the human picked *No review* for wraps up without a review
/// session and goes Done on its checks alone.
///
/// The review is not skipped over, it is settled: what the rest of a wrap-up
/// waits on is *the review is over*, and a review that was never to happen is
/// over the moment the wrap-up looks. Which is what leaves everything else
/// exactly as it is — nothing further down has to know.
///
/// Green all the way through and nothing said, so a review session is the one
/// thing that could stand between this wrap-up and Done. It never runs, and it
/// still gets there.
#[tokio::test]
async fn a_conversation_with_no_review_wraps_up_without_one_and_reaches_done() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_unreviewed(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(GREEN, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    fixture
        .until(|view| (view.state == Lifecycle::Done).then_some(()))
        .await;

    let view = fixture.view().await;

    assert!(
        !reviews.exists(),
        "no session was ever put inside the reviewing skill: {:?}",
        std::fs::read_to_string(&reviews).ok(),
    );
    assert!(
        review_settled(&fixture).await,
        "and the review is settled all the same, which is what carries the rest",
    );
    assert_eq!(
        sets(&view).len(),
        1,
        "the only Set on the Timeline is the proposal that ended the grilling",
    );
    assert_eq!(fixes(&view), 0, "nothing was dispatched to fix anything");
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped: {:?}",
        notices(&view),
    );
}

/// And what is said on its pull request is still answered, by the batch session
/// that always answers it.
///
/// With no review there is nothing to fold the comments into, so every one of
/// them is a batch's from the moment it lands. The wrap-up then narrows to
/// waiting on its checks, which here never finish — the same condition a
/// reviewed wrap-up down to its suite is in.
#[tokio::test]
async fn a_wrap_up_with_no_review_answers_what_was_said_and_narrows_to_its_checks() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let batches = spill.path().join("batch-prompts");

    let fixture = grilling_unreviewed(
        spill,
        &a_backlog_then_answers_comments(&reviews, &dispatched, &batches, RESPOND_AND_FIND_NOTHING),
        &gh_about(STILL_RUNNING, THREE_COMMENTS, ""),
    )
    .await;

    worked_to_empty(&fixture).await;

    let said = until_written(&batches).await;

    assert!(
        said.contains("Rename the window field."),
        "the batch session was sent what was written on the pull request: {said}",
    );
    assert!(
        !reviews.exists(),
        "and no review read it first: {:?}",
        std::fs::read_to_string(&reviews).ok(),
    );

    let view = fixture
        .until(|view| view.waiting_on_checks.then(|| view.clone()))
        .await;

    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "which is a condition of Wrapping rather than a state of its own",
    );
    assert!(
        comments_settled(&fixture).await,
        "nothing said is left for anybody to be sent about",
    );
    assert!(
        !checks_settled(&fixture).await,
        "and the checks are the one thing left, which have not finished",
    );
}

/// And a red check costs it exactly what it costs any other wrap-up: a fix
/// session, dispatched under the Implementation Pairing.
///
/// Reviewing is a fresh set of eyes and fixing is building, so picking the eyes
/// away leaves the building where it was.
#[tokio::test]
async fn a_wrap_up_with_no_review_still_dispatches_a_fix_session_at_a_red_check() {
    let spill = tempfile::tempdir().unwrap();
    let written = spill.path().join("fix-prompts");

    let fixture = grilling_unreviewed(
        spill,
        &a_backlog_then_fixes(&written),
        &gh_checking("FAILURE"),
    )
    .await;

    worked_to_empty(&fixture).await;

    let sent = until_written(&written).await;

    assert!(
        sent.contains("addressing/SKILL.md"),
        "a fix session, inside the addressing skill as ever: {sent}",
    );
    assert!(
        sent.contains("model=claude-implementation-5"),
        "and under the Implementation Pairing, which is what builds: {sent}",
    );
}

/// And a stage inherits *No review* the way it inherits the Pairings beside it:
/// through the one act that gives it all three.
///
/// A stage has no draft moment of its own, so the inheritance funnel is the only
/// place the pick could come from — and one that arrived reviewed would be a
/// roadmap the human turned reviewing off for reviewing every stage of it.
#[tokio::test]
async fn a_stage_inherits_the_no_review_its_roadmap_was_grilled_with() {
    let spill = tempfile::tempdir().unwrap();
    let planning = spill.path().join("stage-prompts");
    let worked = spill.path().join("task-prompts");

    let fixture = grilling_unreviewed(
        spill,
        &a_roadmap_then_wraps_up(&planning, &worked, TWO_STAGES, ""),
        &gh_about(GREEN, "", ""),
    )
    .await;

    staged_and_settled(&fixture).await;

    let stage = stage_of(&fixture).await;

    assert_eq!(
        stage.review_pairing,
        PickedView::Skipped,
        "the row its roadmap was grilled on, and so the row its stages run on",
    );
    assert_eq!(
        stage
            .implementation_pairing
            .as_ref()
            .map(|pairing| pairing.profile.name.clone()),
        Some("implementation".to_owned()),
        "with the roles beside it inherited as they always were",
    );
}

/// The stub a Conversation started with *No grilling* runs: one session, on the
/// implementation skill, which writes down what it was told and does the work.
///
/// Cased on the skill for the sake of what comes after it — the wrap-up's review
/// session runs on this stub too, and a second `git commit` with nothing to
/// commit would be a failure inside the thing under test.
fn an_ungrilled_run(prompts: &Path) -> String {
    format!(
        r#"
case "$2" in
*implementing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {prompts}
    printf 'building it\n'
    printf 'a limiter\n' > limiter.md
    git add limiter.md
    git commit --quiet -m 'feat: rate limiting'
    ;;
*)
    printf 'nothing to do\n'
    sleep 300
    ;;
esac
"#,
        prompts = quoted(prompts),
    )
}

/// A Conversation whose human picked *No grilling*, end to end: the press makes
/// the branch and the worktree as it always does and lands the Conversation
/// Implementing, and what runs is one session under the Implementation Pairing,
/// inside the implementation skill, primed with the Brief and told there was no
/// interview.
///
/// The run from there is an inline implementation and nothing else: the session
/// commits, carries the branch to a pull request on its way out, and the
/// Conversation wraps that up exactly as a run the human picked *inline* on at
/// the end of a grilling does.
#[tokio::test]
async fn no_grilling_builds_from_the_brief_alone_and_carries_it_to_a_pull_request() {
    let spill = tempfile::tempdir().unwrap();
    let prompts = spill.path().join("implementing-prompts");

    let fixture = building_ungrilled(spill, &an_ungrilled_run(&prompts), PULL_REQUEST).await;

    let worktree = PathBuf::from(fixture.until(|view| view.worktree.clone()).await.path);

    let sent = until_written(&prompts).await;

    assert!(
        sent.contains("model=claude-implementation-5"),
        "the work runs under the Implementation Pairing, there being no other: {sent}",
    );
    assert!(
        sent.contains("implementing/SKILL.md"),
        "and inside the bundled implementation skill: {sent}",
    );
    assert!(
        sent.contains(BRIEF),
        "primed with the Brief, which is the whole of the plan: {sent}",
    );
    assert!(
        sent.contains("Nothing was grilled"),
        "and told so, rather than left to infer it from a handoff that is not \
         there: {sent}",
    );
    assert!(
        sent.contains("ordinary ask"),
        "with what to do about what the Brief leaves open: {sent}",
    );

    let opened = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping)
                .then(|| pull_request(view).cloned())
                .flatten()
        })
        .await;

    assert_eq!(opened.number, 41);

    let view = fixture.view().await;

    assert_eq!(
        view.timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::Moved(moved) => Some(moved.state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [Lifecycle::Implementing, Lifecycle::Wrapping],
        "with no Grilling on the way at all: the press that would have started \
         an interview started the work",
    );
    assert!(
        git(&worktree, &["log", "--oneline"]).contains("feat: rate limiting"),
        "which committed what it built",
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped on the way: {:?}",
        notices(&view),
    );
}

/// The stub for the ask: a session that builds, then waits on the human the way
/// one holding a Blocking Ask does.
fn an_ungrilled_run_that_asks(prompts: &Path) -> String {
    format!(
        r#"
case "$2" in
*implementing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {prompts}
    printf 'reading the brief\n'
    while [ ! -f /tmp/verkstead/asked ]; do sleep 0.1; done
    while read -r TOLD; do printf '%s\n' "$TOLD" >> /tmp/verkstead/rescues; done
    sleep 300
    ;;
*)
    sleep 300
    ;;
esac
"#,
        prompts = quoted(prompts),
    )
}

/// And a Blocking Ask works from such a session, which is what the prompt tells
/// it to do with a decision the Brief left open.
///
/// The Set lands on this Conversation's Timeline and waits there, and the
/// session holding it is left alone for as long as it takes — the same condition
/// a step session's ask puts a run in, which is the point: nothing downstream of
/// the press knows the interview was skipped.
#[tokio::test]
async fn a_blocking_ask_from_an_ungrilled_session_waits_on_the_human() {
    let spill = tempfile::tempdir().unwrap();
    let prompts = spill.path().join("implementing-prompts");

    let fixture =
        building_ungrilled(spill, &an_ungrilled_run_that_asks(&prompts), PULL_REQUEST).await;

    until_written(&prompts).await;

    let set = fixture.ask(A_STEP_QUESTION).await;

    fixture
        .until(|view| (!sets(view).is_empty()).then_some(()))
        .await;

    // Several graces of silence, which is what waiting on a human looks like
    // from outside — and the session is not ended on it.
    tokio::time::sleep(BRISKLY.proposing * 4).await;

    assert!(
        anything_told(&fixture).is_empty(),
        "nothing was typed into a session waiting on the human: {:?}",
        anything_told(&fixture),
    );

    let view = fixture.view().await;

    assert_eq!(
        view.state,
        Lifecycle::Implementing,
        "the run is where the press left it, with the ask open on it",
    );
    assert!(
        notices(&view).is_empty(),
        "and nothing stopped over it: {:?}",
        notices(&view),
    );

    assert_eq!(
        fixture
            .respond(set, serde_json::json!([{ "label": "Q1", "selected": 1 }]))
            .await,
        Submitted::Accepted,
        "and the human answers it the way they answer any other",
    );
}
