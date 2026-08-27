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
//! The stub is handed exactly what claude would be: `--model`, the Profile's
//! model, and then the Brief. So `$1` is the model it was told to run and `$2`
//! is the Brief it was primed with — which is how these read them back.
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

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use avt::Vt;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;
use verkstead_render::{
    Adopted, AgentOutputEvent, BriefSaved, Capture, CommitEvent, CommitPane, CompanionAdded,
    CompanionMode, CompanionModeChosen, ConversationClosed, ConversationSteered,
    ConversationStopped, ConversationView, GrillingStarted, Lifecycle, NoticeEvent, PinnedEvent,
    ProfileSaved, PullRequestEvent, Registered, Resumed, Shown, Size, StageListReached, Started,
    SteerOpened, Submitted, TaskListEvent, TaskListReached, TimelineEvent, TranscriptView, Turn,
    Watching,
};
use verkstead_schema::{Direction, Nudge};
use verkstead_server::handoffs::Handoffs;
use verkstead_server::sandbox::{Executable, Home, Reachable, SandboxConfig};
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

/// How long to wait for something a session does. Generously long, because what
/// is being waited on is a process starting: the flush that carries its output
/// is half a second, and a loaded machine can take a while to get bwrap and a
/// shell going.
const PATIENCE: Duration = Duration::from_secs(30);

/// What every sandbox here is equipped with as `verkstead`.
///
/// A test harness is its own executable, and what a sandbox does with one is
/// bind it read-only — so any file that is really there will do where nothing in
/// this file runs it. That a session asks with the *server's* build is the
/// sandbox's own claim, and `tests/sandbox.rs` is where it is put to a session.
fn equipped() -> Option<Executable> {
    Executable::of_the_server()
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
        let deadline = Instant::now() + PATIENCE;

        loop {
            let row = self.row().await;

            if let Some(reached) = reached(&row) {
                return reached;
            }

            assert!(
                Instant::now() < deadline,
                "the row never got there. It says: {row:?}"
            );

            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Read it back until the session has got somewhere, or give up.
    async fn until<T>(&self, reached: impl Fn(&ConversationView) -> Option<T>) -> T {
        let deadline = Instant::now() + PATIENCE;

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

            tokio::time::sleep(Duration::from_millis(25)).await;
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
        let deadline = Instant::now() + PATIENCE;

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

            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Read it back until it has that many lines, or give up.
    async fn transcript_of(&self, event: i64, lines: usize) -> Vec<String> {
        let deadline = Instant::now() + PATIENCE;

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

            tokio::time::sleep(Duration::from_millis(25)).await;
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
                Home {
                    path: self.home.path().to_owned(),
                },
                Reachable::at(LISTENING),
                SandboxConfig::resolve(&[self.spill.path().display().to_string()]).unwrap(),
                Skills::installed(self.state.path()).expect("this binary carries skills"),
                equipped(),
                Handoffs::under(self.state.path()),
                Settings::in_data_dir(self.state.path()),
            )
            .at_pace(BRISKLY),
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

    /// What both of them are made of: post the Set over the agent API and read
    /// its id back.
    async fn asking(&self, yaml: &str, how: &str) -> i64 {
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

        let created: verkstead_schema::SetCreated = serde_saphyr::from_str(&body).unwrap();
        created.id
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

    /// Answer it the way the human does, from the browser.
    async fn answer(&self, set_id: i64) -> Submitted {
        post(
            &self.app,
            &format!("/api/ui/sets/{set_id}/response"),
            &serde_json::json!({ "answers": [{ "label": "Q9", "selected": 1 }] }),
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

    /// Whether anybody chose to stop, which is the half of a stop the Timeline
    /// does not draw: a restart reads it rather than a human, so the record is
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
const BRISKLY: Pace = Pace {
    poll: Duration::from_millis(100),
    grace: Duration::from_millis(300),
    checks: Duration::from_millis(100),
    // Longer than the grace above, as a server's is: the tests that watch a
    // review being ended on quiet want the two apart, so that a session ended
    // after the shorter of them is one ended by the wrong rule.
    proposing: Duration::from_millis(900),
    // Longer than any of these run for, so that the sweep for a stalled
    // Conversation is the one thing that never fires by itself here. Every one
    // of these fixtures is a Conversation whose grilling session has printed and
    // exited, which is a stall by every rule there is — so the tests that are
    // about something else say nothing about it, and the ones that are about it
    // keep [`SWEEPING`].
    stalls: Duration::from_secs(600),
};

/// And the same at a pace that does look, for the tests that are about the
/// looking.
///
/// A server sweeps every minute. What is being asked here is whether a
/// Conversation nothing is driving is noticed while the server runs, and the
/// number of seconds it waits before noticing is not part of the answer.
const SWEEPING: Pace = Pace {
    stalls: Duration::from_millis(100),
    ..BRISKLY
};

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
const PULL_REQUEST: &str = r#"
if [ "$1" = api ]; then printf '[]'; exit 0; fi
case "$5" in
*commits*)
    printf '{"commits":[{"oid":"c0ffee1","messageHeadline":"feat: count the requests"}],"comments":[{"author":{"login":"tobico"},"body":"Looks **good**.","createdAt":"2026-08-21T09:00:00Z"}]}'
    ;;
*comments*)
    printf '{"comments":[],"reviews":[]}'
    ;;
*)
    printf '{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}'
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
    printf '{{"statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"%s","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "{how}"
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

/// A green suite, as [`gh_about`]'s answer about the checks.
const GREEN: &str = r#"    printf '{"statusCheckRollup":[{"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}]}'"#;

/// One that has gone back to running once `landed` is there — which is what a
/// commit pushed to the pull request does to it, GitHub starting a whole new run
/// against the new head.
fn green_until(landed: &Path) -> String {
    format!(
        r#"    if [ -e {landed} ]; then how=IN_PROGRESS; else how=COMPLETED; fi
    printf '{{"statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"%s","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$how""#,
        landed = quoted(landed),
    )
}

/// One whose suite is still running until `started` is there and green once it
/// is, which is how a test keeps the checks out of the way until the thing it is
/// about has begun — and then lets them settle, so that what stops the wrap-up
/// finishing is the thing being asked about and nothing else.
fn green_after(started: &Path) -> String {
    format!(
        r#"    if [ -s {started} ]; then status=COMPLETED; how=SUCCESS; else status=IN_PROGRESS; how=; fi
    printf '{{"statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"%s","conclusion":"%s","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$status" "$how""#,
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
    printf '{{"statusCheckRollup":[{{"__typename":"CheckRun","name":"Rust","status":"%s","conclusion":"%s","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2"}}]}}' "$status" "$how"
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
    printf '{{"statusCheckRollup":[]}}'
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

/// Stand a workbench up with `stub` where claude goes, and press *start
/// grilling*.
async fn grilling(stub: &str) -> Grilling {
    grilling_spilling(tempfile::tempdir().unwrap(), stub, PULL_REQUEST).await
}

/// The same, with a second repository registered beside this one and added to
/// the Conversation as a companion before the press — which is a companion in
/// the mode one is added in, read-only.
async fn grilling_alongside(stub: &str, companion: &str) -> Grilling {
    grilling_at_pace(
        tempfile::tempdir().unwrap(),
        stub,
        PULL_REQUEST,
        BRISKLY,
        &[(companion, CompanionMode::ReadOnly)],
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
        BRISKLY,
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
        SWEEPING,
        &[],
    )
    .await
}

/// The same, over a directory the caller already has the name of — which is
/// what a stub that has to write somewhere the worktree is not needs, the
/// script naming the path being written before there is a fixture to ask.
async fn grilling_spilling(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    grilling_at_pace(spill, stub, gh, BRISKLY, &[]).await
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
    let bench = bench_at_pace(spill, stub, gh, pace).await;
    let app = &bench.app;

    let started: Started = post(
        app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": bench.repo_id }),
    )
    .await;
    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    bench.under_both_pairings(id).await;

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
}

impl Bench {
    /// Fix both Pairings on a Conversation, which is what every one of these
    /// has to have settled before anything will start in it.
    ///
    /// Each role gets a Profile of its own, paired with the first of the models
    /// that Profile lists — see [`profile`], which lists two so that a pick can
    /// move off this one without moving off the Profile.
    async fn under_both_pairings(&self, id: i64) {
        for role in ["grilling", "implementation"] {
            let profile = profile(&self.app, self.watched.path(), role).await;
            let chosen: verkstead_render::ProfileChosen = post(
                &self.app,
                &format!("/api/ui/conversations/{id}/{role}-pairing"),
                &serde_json::json!({
                    "profile_id": profile,
                    "model": format!("claude-{role}-5"),
                }),
            )
            .await;
            assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
        }
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
        }
    }
}

async fn bench(spill: tempfile::TempDir, stub: &str, gh: &str) -> Bench {
    bench_at_pace(spill, stub, gh, BRISKLY).await
}

/// The same, at a pace of the caller’s choosing — which is what the tests about
/// the stall sweep need, that being the one thing [`BRISKLY`] deliberately keeps
/// slow.
async fn bench_at_pace(spill: tempfile::TempDir, stub: &str, gh: &str, pace: Pace) -> Bench {
    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // Who a session commits as: a settings file in the Data Directory, which is
    // where every sandbox is configured out of.
    std::fs::write(
        state.path().join("config.yaml"),
        "git_author:\n  name: Verkstead Test\n  email: test@verkstead.invalid\n",
    )
    .unwrap();

    let database = state.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();

    let agents = Agents::running(
        vec!["/bin/sh".to_owned(), "-c".to_owned(), stub.to_owned()],
        Home {
            path: home.path().to_owned(),
        },
        Reachable::at(LISTENING),
        SandboxConfig::resolve(&[spill.path().display().to_string()]).unwrap(),
        Skills::installed(state.path()).expect("this binary carries skills"),
        equipped(),
        Handoffs::under(state.path()),
        Settings::in_data_dir(state.path()),
    )
    .at_pace(pace);

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
            "claude_dir": claude_dir,
            "config_file": config_file,
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
        said.contains("~/.claude/skills/grilling/SKILL.md"),
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

/// And the details pane reads that log as a conversation while the session is
/// still having it: the lines are parsed and rendered on the way out, which is
/// what keeps the reading of somebody else's file format to the one crate that
/// has the parsers in it (ADR 0006).
///
/// The stub writes a line of each of the three classes — the conversation
/// itself, the backend's own bookkeeping, and a kind nobody has ever heard of —
/// because what the pane does with the three is the whole of what makes a log
/// readable.
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
        printf '{"type":"divination","omen":"a raven"}\n' >> "$log"
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
        "a kind nobody knows should arrive as itself rather than as nothing: {:?}",
        view.turns
    );
    assert_eq!(
        view.bookkeeping.len(),
        1,
        "and the backend's own bookkeeping should be out of the conversation: {:?}",
        view.bookkeeping
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
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
    let woken = fixture
        .until(|view| {
            output(view)
                .filter(|output| output.running && !output.idle)
                .cloned()
        })
        .await;

    assert_eq!(
        woken.latest, "What should happen when the queue is full?",
        "the statement that woke it is the one the row now reads"
    );

    assert_eq!(fixture.close().await, ConversationClosed::Closed);
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
    tokio::time::sleep(Duration::from_millis(200)).await;

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
            Home {
                path: PathBuf::from("/nonexistent"),
            },
            Reachable::at(LISTENING),
            SandboxConfig::default(),
            Skills::installed(fixture.state.path()).expect("this binary carries skills"),
            equipped(),
            Handoffs::under(fixture.state.path()),
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
        said.contains("~/.claude/skills/implementing/SKILL.md"),
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
        Decision::Deliberate,
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
    let deadline = Instant::now() + PATIENCE;
    while !written.is_file() {
        assert!(
            Instant::now() < deadline,
            "the stub never wrote the handoff the superseded pick asked for",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls than the handoff watcher would have needed:
    // it wakes every 100ms and ends a session on 300ms of quiet.
    tokio::time::sleep(Duration::from_millis(1500)).await;

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
            && said.contains("~/.claude/skills/next-task/SKILL.md"),
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
        printed.contains("~/.claude/skills/grilling/SKILL.md"),
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
            grep '^name:' "$HOME/.claude/skills/breaking-down/SKILL.md"
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
        next.contains("~/.claude/skills/next-task/SKILL.md")
            && !next.contains("~/.claude/skills/breaking-down/SKILL.md"),
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
            grep '^name:' "$HOME/.claude/skills/staging/SKILL.md"
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
/// fork does — no task file left means the finish step — so what this asserts is
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
            next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
            if [ -n "$next" ]; then
                printf 'working %s\n' "$next"
                printf 'skill=%s\n' "$(grep '^name:' "$HOME/.claude/skills/next-task/SKILL.md")"
                number=${next%%-*}
                printf 'a limiter\n' >> limiter.md
                rm ".tasks/$next"
                sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
                git add -A
                git commit --quiet -m "feat: $next"
            else
                printf 'finishing\n'
                git rm --quiet .tasks/TODO.md
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
    tokio::time::sleep(Duration::from_secs(2)).await;

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
            next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
            if [ -n "$next" ]; then
                rm ".tasks/$next"
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
        "the task whose file has gone is done, and the one still to do is not",
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
    next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        rm ".tasks/$next"
        git add -A
        git commit --quiet -m "feat: count the requests"
    else
        git rm --quiet .tasks/TODO.md
        git commit --quiet -m 'chore: finish rate-limiting'
        printf 'pushed, and the pull request is open\n'
    fi
    sleep 300
    ;;
esac
"#;

/// Take a Conversation from the pick on its closing Set to a worked-through
/// backlog, with nothing pressed on the way: the whole point of the run is that
/// nobody is asked anything between the direction and the pull request.
async fn worked_to_empty(fixture: &Grilling) {
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

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

/// Whether Verkstead has recorded this Conversation's checks as green.
///
/// Read out of the store rather than off the Timeline, because that is where it
/// is: settling is bookkeeping about what wrap-up is still waiting on rather
/// than something that happened, and nothing that is not an Event goes on a
/// Timeline.
async fn checks_settled(fixture: &Grilling) -> bool {
    let pool = open_database(&fixture.database).await.unwrap();
    let settled = verkstead_server::store::wrap_up_settled(&pool, fixture.id)
        .await
        .unwrap();
    pool.close().await;

    settled.contains(&verkstead_server::store::WaitingOn::Checks)
}

/// How many fix sessions Verkstead has counted against one of this
/// Conversation's checks.
///
/// The count that *two attempts, then ask* is kept by, read the way the watcher
/// reads it. What a check the review folded into its own session has spent is
/// nothing: an attempt is counted where a fix session is dispatched, and none is
/// dispatched into a Worktree the review is holding.
async fn attempts_spent(fixture: &Grilling, check: &str) -> i64 {
    let pool = open_database(&fixture.database).await.unwrap();
    let spent = verkstead_server::store::fix_attempts(&pool, fixture.id, check)
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

    let deadline = Instant::now() + PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(Instant::now() < deadline, "the checks never settled");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Long enough for many more polls of a suite that has nothing wrong with it.
    tokio::time::sleep(Duration::from_millis(500)).await;

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
        Decision::Deliberate,
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
    tokio::time::sleep(Duration::from_millis(500)).await;

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
    tokio::time::sleep(Duration::from_millis(500)).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the checks were never asked about"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert_eq!(fixture.close().await, ConversationClosed::Closed);

    // One more poll's worth, so that a watcher part way through a question has
    // finished asking it before the count is taken.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let when_stopped = std::fs::metadata(&asked).map(|it| it.len()).unwrap();

    // Long enough for many more polls, had anything still been polling.
    tokio::time::sleep(Duration::from_millis(500)).await;

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
            Home {
                path: PathBuf::from("/nonexistent"),
            },
            Reachable::at(LISTENING),
            SandboxConfig::default(),
            Skills::installed(fixture.state.path()).expect("this binary carries skills"),
            equipped(),
            Handoffs::under(fixture.state.path()),
            Settings::in_data_dir(fixture.state.path()),
        )
        .at_pace(BRISKLY),
        gh_stub(&gh_checking("SUCCESS")),
    );

    let deadline = Instant::now() + PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the restarted server never looked at the checks it was left with",
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
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

/// And what happens when `gh` cannot answer — no `gh`, no login, or a branch
/// nothing was opened on. The Conversation stays where it is with the reason on
/// its Timeline, rather than becoming a Wrapping with no pull request under it.
#[tokio::test]
async fn a_finish_that_opened_no_pull_request_leaves_the_conversation_where_it_is() {
    let fixture = grilling_asking(A_BACKLOG_OF_ONE, NO_PULL_REQUEST).await;

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
        Decision::Deliberate,
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
        let waited_for = tokio::time::timeout(PATIENCE, async {
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
    let deadline = Instant::now() + PATIENCE;

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

        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// The same, waiting until what is there says something in particular — for the
/// tests where an earlier session has already written to the file, so that
/// merely finding it there says nothing about the one being waited on.
async fn until_written_saying(path: &Path, said: &str) -> String {
    let deadline = Instant::now() + PATIENCE;

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

        tokio::time::sleep(Duration::from_millis(25)).await;
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

    let told = until_written(&reviews).await;
    let started = prompts(&told);

    assert_eq!(started.len(), 1, "one review and one only: {told}");
    assert!(
        started[0].contains("reviewing/SKILL.md"),
        "inside the bundled reviewing skill: {told}",
    );
    assert!(
        started[0].contains("model=claude-implementation-5"),
        "under the implementation Profile, as every session about the code is: {told}",
    );
    assert!(
        started[0].contains("The API has none."),
        "and told what the work was meant to be: {told}",
    );

    assert!(
        !review_settled(&fixture).await,
        "a review that has not reported settles nothing",
    );

    // What the review session does through the CLI, played by the test.
    let set = fixture.ask(REVIEW).await;

    // Long enough for the ask to have ended a session, had anything been going to.
    tokio::time::sleep(Duration::from_millis(500)).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the session landed the fix and the review never settled",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(Instant::now() < deadline, "the review never settled");
        tokio::time::sleep(Duration::from_millis(50)).await;
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
    until_written(&reviews).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the answers were in and the session that read them was never ended",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
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
    until_written(&reviews).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the session was seen out and the review never settled: {:?}",
            notices(&fixture.view().await),
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
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
        Decision::Deliberate,
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
    until_written(&reviews).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the resume never reviewed anything, so the wrap-up never settled",
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
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
    until_written(&reviews).await;

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
    until_written(&reviews).await;

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
    until_written(&reviews).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the review session ended and the review never settled",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
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
    until_written(&reviews).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the press never reviewed anything, so the wrap-up never settled",
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
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
    until_written(&reviews).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the first wrap's review never settled",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
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

    let deadline = Instant::now() + PATIENCE;
    let read_again = loop {
        let written = std::fs::read_to_string(&reviews).unwrap_or_default();

        if prompts(&written).len() > 1 {
            break written;
        }

        assert!(
            Instant::now() < deadline,
            "the second wrap never read the branch: {written}",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    assert_eq!(
        prompts(&read_again).len(),
        2,
        "one review per wrap, and the second wrap ran its own: {read_again}",
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
    until_written(&reviews).await;

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

    let deadline = Instant::now() + PATIENCE;
    let read_again = loop {
        let written = std::fs::read_to_string(&reviews).unwrap_or_default();

        if prompts(&written).len() > 1 {
            break written;
        }

        assert!(
            Instant::now() < deadline,
            "the second wrap never read the branch: {written}",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    assert_eq!(
        prompts(&read_again).len(),
        2,
        "one review per wrap, and the second wrap ran its own: {read_again}",
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
    until_written(&reviews).await;

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
    next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
    if [ -n "$next" ]; then
        printf 'one clock\n' >> clocks.md
        rm ".tasks/$next"
        git add -A
        git commit --quiet -m 'feat: collapse the clocks'
    else
        git rm --quiet .tasks/TODO.md
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

    until_written(&reviews).await;

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
///
/// The checks cannot be asked about, which is what leaves the review the only
/// thing this wrap-up is still waiting on. A `gh` that answered them green would
/// settle the last of the three the moment the review settled, and *where the
/// wrap-up left it* would be a state that held for one poll of the settling loop
/// rather than a fact — read after that poll on a loaded machine, it reads Done.
#[tokio::test]
async fn a_split_no_backlog_was_written_for_settles_like_any_other_review() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_FIX),
        &gh_about(CHECKS_UNANSWERABLE, "", ""),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_written(&reviews).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the review session ended and the review never settled",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

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
    assert_eq!(
        view.state,
        Lifecycle::Wrapping,
        "the Conversation is where the wrap-up left it",
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
    until_written(&reviews).await;

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
    tokio::time::sleep(Duration::from_millis(500)).await;

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

/// Whether Verkstead has recorded that nothing said on this pull request is left
/// unaddressed.
async fn comments_settled(fixture: &Grilling) -> bool {
    let pool = open_database(&fixture.database).await.unwrap();
    let settled = verkstead_server::store::wrap_up_settled(&pool, fixture.id)
        .await
        .unwrap();
    pool.close().await;

    settled.contains(&verkstead_server::store::WaitingOn::Comments)
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
    let deadline = Instant::now() + PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "what was said to the review was never recorded as addressed",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls of a pull request with five comments on it.
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        !dispatched.exists(),
        "and nothing was dispatched to act on any of it: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
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

    let told = until_written(&reviews).await;

    assert!(
        !told.contains("Rename the window field."),
        "a comment said after the review started is not one it was given: {told}",
    );

    let set = fixture.ask(REVIEW).await;

    // Long enough for many polls of a pull request that now has five comments on
    // it, while the review still holds the Worktree.
    tokio::time::sleep(Duration::from_millis(500)).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "what was said was never addressed",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls of a pull request whose comments have all
    // been dispatched for.
    tokio::time::sleep(Duration::from_millis(500)).await;

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

    let told = until_written(&batches).await;

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
    tokio::time::sleep(Duration::from_millis(500)).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the batch landed its fix and what was said never settled",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
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

    let deadline = Instant::now() + PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the batch was answered and never settled",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
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
    until_written(&batches).await;

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

/// Which of a pull request's comments Verkstead has recorded as dealt with.
async fn addressed(fixture: &Grilling) -> Vec<String> {
    let pool = open_database(&fixture.database).await.unwrap();
    let addressed = verkstead_server::store::addressed_comments(&pool, fixture.id)
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
    until_written(&batches).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the batch session ended and what was said never settled",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
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
    until_written(&batches).await;

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

    let deadline = Instant::now() + PATIENCE;
    while !comments_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the press never answered what was said, so it never settled",
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
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

    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        prompts(&std::fs::read_to_string(&dispatched).unwrap()).len(),
        1,
        "the first server dispatched once for the batch",
    );

    // A second server over the same database, sandboxes and agent — which knows
    // nothing about the comments except what was written down.
    let _restarted = fixture.restarted(&stub, &gh).await;

    // Long enough for many polls of both of them.
    tokio::time::sleep(Duration::from_millis(800)).await;

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

/// The rule that ends a wrap-up: the checks green, the review answered and
/// nothing said left unaddressed, all three together. Verkstead decides it
/// itself — there is nobody at the workbench to press anything.
///
/// And what it does not wait for is the merge. The pull request is open the whole
/// time and nothing here ever asks GitHub whether it is: stages stack on unmerged
/// predecessors, so a Conversation that waited would hold up every stage behind
/// it.
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

    assert!(
        !asked.contains("merge"),
        "nothing ever asked GitHub whether the pull request had been merged: {asked}",
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

    let deadline = Instant::now() + PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(Instant::now() < deadline, "the checks never settled");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    std::fs::write(&landed, "").unwrap();

    let deadline = Instant::now() + PATIENCE;
    while checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "a commit landed and the checks stayed settled from the run before it",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    // Long enough for many more polls of a run that has not finished.
    tokio::time::sleep(Duration::from_millis(500)).await;

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
    tokio::time::sleep(Duration::from_secs(5)).await;

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
    tokio::time::sleep(Duration::from_millis(500)).await;

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
        Decision::Deliberate,
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
        fixture.row().await.waiting,
        "and the sidebar says so too, a stop being the whole of what is waiting",
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
        Decision::Deliberate,
        "a task nothing has moved is not one a restart may have another go at",
    );

    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "and the run is blocked on the human",
    );

    let sessions = outputs(&fixture.view().await).len();

    // Long enough for several more turns of a runner that was still turning.
    tokio::time::sleep(Duration::from_secs(3)).await;

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
    format!(
        r#"
        case "$1" in
        claude-grilling-5)
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
            next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
            if [ -n "$next" ]; then
                printf 'working %s\n' "$next"
                number=${{next%%-*}}
                printf 'a limiter\n' >> limiter.md
                rm ".tasks/$next"
                sed -i "s/- \[ \] $number:/- [x] $number:/" .tasks/TODO.md
                git add -A
                git commit --quiet -m "feat: $next"
                if [ "$next" = 01-count.md ]; then
                    # The wait itself, in miniature: the task lands, the account
                    # runs out before the next one, and the agent holds with its
                    # banner up. Redrawn eight times over a second — more than
                    # the half a second Verkstead writes down what a session
                    # printed on, so the banner is looked at more than once —
                    # with claude's own spinner turning in front of it, which is
                    # what makes each repaint a different line.
                    # The glyphs themselves rather than `\xe2\x9c\xbb` and its
                    # kind. `\xNN` is bash's extension to `printf` and not
                    # POSIX: a `/bin/sh` that is dash — which is what Debian and
                    # Ubuntu have, so it is what CI runs — prints the escape
                    # rather than the character, and a banner opening with a
                    # literal backslash is one [`verkstead_server`] is right to
                    # refuse. ASCII punctuation is not decoration there,
                    # deliberately, so it does not open a status line. This file
                    # is UTF-8 and the shell passes the bytes through, which
                    # needs no escape at either end.
                    for pass in 1 2; do
                        for turning in '✻' '✽' '✳' '✢'
                        do
                            printf "$turning {sentence}\r\n"
                            sleep 0.125
                        done
                    done
                fi
            else
                printf 'finishing\n'
                git rm --quiet .tasks/TODO.md
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
        Decision::Deliberate,
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
    tokio::time::sleep(Duration::from_secs(3)).await;

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

    let landed = commits(&fixture.view().await).len();

    // Long enough for anything running on a clock to have come round.
    tokio::time::sleep(Duration::from_secs(3)).await;

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
        after.grilling_pairing.map(|pairing| pairing.profile.id),
        before.grilling_pairing.map(|pairing| pairing.profile.id),
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
    tokio::time::sleep(Duration::from_secs(2)).await;

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
    next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
    printf 'working %s\r\n' "$next"
    while [ ! -f {gate} ]; do sleep 0.05; done
    number=$(printf '%s' "$next" | cut -d- -f1)
    printf 'a limiter\n' >> limiter.md
    rm ".tasks/$next"
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
        Decision::Deliberate,
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
    tokio::time::sleep(Duration::from_secs(2)).await;

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
    assert_eq!(fixture.chosen().await, Decision::Deliberate);

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
    tokio::time::sleep(Duration::from_secs(2)).await;

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
    let deadline = Instant::now() + PATIENCE;
    while !written.is_file() {
        assert!(
            Instant::now() < deadline,
            "the grilling never wrote the handoff its pick asked for",
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    assert_eq!(fixture.force_stop().await, ConversationStopped::Stopped);

    let stopped = fixture.stopped().await;

    assert!(
        stopped.html.contains("you pressed Force stop"),
        "the run stopped because of the press: {:?}",
        stopped.html,
    );
    assert_eq!(fixture.chosen().await, Decision::Deliberate);

    // Long enough for the driver to have read the ending, taken the handoff and
    // reached the launch on the other side of it.
    tokio::time::sleep(Duration::from_secs(2)).await;

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
    tokio::time::sleep(Duration::from_secs(2)).await;

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
    tokio::time::sleep(Duration::from_secs(2)).await;

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
    assert_eq!(fixture.chosen().await, Decision::Deliberate);

    assert_eq!(
        fixture.view().await.blocked_on,
        Some(stopped.id),
        "and the Conversation is waiting on the human from here",
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
    sed -i 's|\[brief\](01-counter.md)|[brief](01-counter.md) *(in progress: `counter`)*|' docs/roadmaps/rate-limiting/ROADMAP.md 2>/dev/null || true
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
    let deadline = Instant::now() + PATIENCE;

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

        tokio::time::sleep(Duration::from_millis(25)).await;
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
fn said(view: &ConversationView) -> Vec<&NoticeEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Notice(notice) => Some(notice),
            _ => None,
        })
        .collect()
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
        stage.branch, "counter",
        "the branch is the stage brief's own name, without its number",
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
        carried_on.contains("Stage 01") && carried_on.contains("<code>counter</code>"),
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
        prompt.contains("~/.claude/skills/next-stage/SKILL.md"),
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
        working.contains("~/.claude/skills/next-task/SKILL.md"),
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
        index.contains("*(in progress: `counter`)*"),
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

    bench.under_both_pairings(id).await;

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
        view.branch, "counter",
        "the branch is the stage brief's own name, without its number",
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
        prompt.contains("~/.claude/skills/next-stage/SKILL.md"),
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
    sed -i "/-$branch.md)/s|\$| *(in progress: \`$branch\`)*|" docs/roadmaps/rate-limiting/ROADMAP.md
    git add -A
    git commit --quiet -m "chore: plan the $branch stage"
    sleep 300
    ;;
*next-task/SKILL.md*)
    next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
    if [ -n "$next" ]; then
        printf 'working %s\n' "$next"
        printf 'a counter\n' >> counter.md
        rm ".tasks/$next"
        git add -A
        git commit --quiet -m 'feat: count the requests'
    else
        printf 'finishing\n'
        git rm --quiet .tasks/TODO.md
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
        until_written_saying(&planning, "planned=counter")
            .await
            .contains("planned=counter"),
        "the adopted stage is the one that was planned",
    );

    let next = stage_of(&fixture).await;

    assert_eq!(
        next.branch, "refusing",
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
        carried_on.contains("<code>refusing</code>"),
        "the adopted Conversation says which stage started and on what: {carried_on:?}",
    );

    // And the session it started is the same fork of next-stage the adopted stage
    // itself was planned by, this time with nobody at the workbench at all.
    let planned = until_written_saying(&planning, "planned=refusing").await;

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
        let said = tokio::time::timeout(PATIENCE, self.socket.next())
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
        let deadline = Instant::now() + PATIENCE;

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
    rm -f .tasks/01-count.md
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
    tokio::time::sleep(Duration::from_secs(1)).await;

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
    let landed = fixture
        .until(|view| {
            let landed = commits(view);
            (landed.len() == 2).then(|| landed[1].subject.clone())
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
    tokio::time::sleep(Duration::from_secs(3)).await;

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
        Decision::Deliberate,
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
    tokio::time::sleep(Duration::from_secs(3)).await;

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
    tokio::time::sleep(Duration::from_secs(3)).await;

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
    let deadline = Instant::now() + PATIENCE;

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

        tokio::time::sleep(Duration::from_millis(20)).await;
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

/// An inline run that lands its work and leaves no pull request halts, with the
/// reason `gh` gave and the Worktree it stopped in on the Timeline.
///
/// The inline half of what a finish step's missing pull request does — see
/// [`a_finish_that_opened_no_pull_request_leaves_the_conversation_where_it_is`].
/// The session committed and exited without ever pushing, which is precisely the
/// ending nothing used to notice: the run went quiet in Implementing and stayed
/// there. Now it is asked about, and a Conversation that cannot be moved on is
/// one the human is told about.
///
/// The evidence is both halves of it: what git makes of the Worktree, and the
/// tail of what the session last said — which is where the reason it opened
/// nothing is usually written down, and is why the session's own Timeline Event
/// goes to the wrap-up with it.
#[tokio::test]
async fn an_inline_run_that_opened_no_pull_request_leaves_the_conversation_where_it_is() {
    let fixture = grilling_asking(
        r#"
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
    assert!(
        stopped
            .html
            .contains("the limiter is in, the middleware is not"),
        "and the tail of what the session last said, which is the Event the \
         wrap-up was handed: {:?}",
        stopped.html,
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Deliberate,
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

/// Stop a Conversation the way a stall does — nobody's decision, and the
/// ordinary Notice on its Timeline saying what nothing was doing — without
/// waiting for a sweep to find it.
///
/// The record is written rather than provoked, for [`wrapping_unwatched`]'s
/// reason: what the test is about is what the *next* server makes of a stop
/// nobody chose, and a sweep left running would write a second one over the top
/// of what it was watching.
async fn halted_by_circumstance(fixture: &Grilling) {
    let pool = open_database(&fixture.database).await.unwrap();

    let written = verkstead_store::stop(
        &pool,
        fixture.id,
        Decision::Circumstance,
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
        printed.contains("~/.claude/skills/next-task/SKILL.md"),
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
    let running: Vec<i64> = outputs(&fixture.view().await)
        .into_iter()
        .map(|output| output.id)
        .collect();

    let mut spent = 0;

    for event in running {
        if fixture
            .capture(event)
            .await
            .contains("implementing/SKILL.md")
        {
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
        printed.contains("~/.claude/skills/implementing/SKILL.md"),
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
        Decision::Deliberate,
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
        printed.contains("~/.claude/skills/grilling/SKILL.md"),
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
        printed.contains("~/.claude/skills/next-task/SKILL.md"),
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
        SWEEPING,
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
        printed.contains("~/.claude/skills/next-task/SKILL.md"),
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
        printed.contains("~/.claude/skills/instruction/SKILL.md"),
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
        printed.contains("~/.claude/skills/instruction/SKILL.md"),
        "what the human wrote goes first, whatever the branch holds: {printed:?}",
    );

    let printed = fixture.printed_after(before + 1).await;

    assert!(
        printed.contains("~/.claude/skills/next-task/SKILL.md"),
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
        printed.contains("~/.claude/skills/grilling/SKILL.md"),
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
        SWEEPING,
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
        SWEEPING,
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
        SWEEPING,
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

/// A Resume with nothing to start refuses by name, and leaves the Conversation
/// exactly as it found it.
///
/// The whole of what this replaces: a press that quietly decided there was
/// nothing to do left the human as stuck as they were, with no more to go on
/// than a line in a log they cannot see. So the reason comes back to the page
/// that asked, and the stop it was pressed on stands.
///
/// A finish step whose pull request `gh` could not find is the plainest way to
/// have one: the backlog is worked through and taken away, the Conversation is
/// still implementing, and there is nothing left in `.tasks/` to read a step
/// off. What is missing is out on GitHub, which is not something a session can
/// be launched at.
#[tokio::test]
async fn resuming_with_nothing_to_start_refuses_by_name_and_changes_nothing() {
    let fixture = grilling_asking(A_BACKLOG_OF_ONE, NO_PULL_REQUEST).await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert_eq!(
        fixture.resume().await,
        Resumed::NothingToWork,
        "there is no backlog to read a step off, and saying so is the whole job",
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
        view.state,
        Lifecycle::Implementing,
        "and the Conversation is where it was",
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
        printed.contains("~/.claude/skills/next-task/SKILL.md"),
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
        Decision::Deliberate,
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
        Decision::Deliberate,
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
/// The stop is written rather than waited for — see [`halted_by_circumstance`] —
/// so that this server writes exactly one: a sweep that went on looking would
/// stop the Conversation again while the next server was driving it.
#[tokio::test]
async fn a_halt_nobody_chose_is_driven_again_by_the_next_server() {
    let fixture = grilling(r#"printf 'the grilling has nothing to say\n'"#).await;

    fixture.quiet().await;

    halted_by_circumstance(&fixture).await;

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
        printed.contains("~/.claude/skills/grilling/SKILL.md")
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
/// A finish step whose pull request `gh` could not find is the plainest way to
/// have one: the backlog is worked through and taken away, the Conversation is
/// still implementing, and there is nothing left in `.tasks/` to read a step off.
/// The stop it left is taken away first, which is the human having pressed Resume
/// on it before the restart — what is being asked about here is the restart's own
/// refusal rather than a stop it would have left alone.
#[tokio::test]
async fn a_restart_that_can_start_nothing_halts_with_the_refusal_on_the_timeline() {
    let fixture = grilling_asking(A_BACKLOG_OF_ONE, NO_PULL_REQUEST).await;

    worked_to_empty(&fixture).await;

    fixture.stopped().await;

    let before = notices(&fixture.view().await).len();

    fixture.drive_again().await;

    let _restarted = fixture.restarted("true", PULL_REQUEST).await;

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
        refused.contains("there is nothing left in <code>.tasks/</code> to work"),
        "in the words the press refuses in, rather than the sweep's: {refused:?}",
    );
    assert_eq!(
        fixture.chosen().await,
        Decision::Deliberate,
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
    next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
    printf '===== %s\n%s\n' "${{next:-finish}}" "$2" >> {prompts}
    if [ -n "$next" ]; then
        printf 'a limiter\n' >> limiter.md
        rm ".tasks/$next"
        git add -A
        git commit --quiet -m "feat: $next"
    else
        git rm --quiet .tasks/TODO.md
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
