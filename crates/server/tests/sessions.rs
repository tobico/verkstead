//! A grilling session end to end: started by pressing the button, run inside
//! the Conversation's sandbox on its worktree, and read back off the Timeline
//! while it is still going.
//!
//! Everything here is real except the agent. The repository is a repository,
//! the worktree is one git made, the sandbox is bwrap and the pseudo-terminal is
//! `script`'s — what stands in for claude is a shell script, because what these
//! ask is whether a session's output reaches the human, and asking it of the
//! real claude would be a test that needed an account, a network and a model's
//! patience.
//!
//! The stub is handed exactly what claude would be: `--model`, the Profile's
//! model, and then the Brief. So `$1` is the model it was told to run and `$2`
//! is the Brief it was primed with — which is how these read them back.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use verkstead_render::{
    AgentOutputEvent, BriefSaved, CommitDiff, CommitEvent, ConversationAborted, ConversationView,
    DirectionChosen, GrillingStarted, InterruptionEvent, Lifecycle, PinnedEvent, ProfileSaved,
    PullRequestEvent, Registered, Remedy, RemedySettled, Started, Submitted, TaskListEvent,
    TimelineEvent, Transcript,
};
use verkstead_server::handoffs::Handoffs;
use verkstead_server::sandbox::{Home, Reachable, SandboxConfig};
use verkstead_server::skills::Skills;
use verkstead_server::{Agents, Gh, Pace, WatchedPaths, open_database, router_running_sessions};

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

/// A Conversation with a session running under a stub agent, and everything
/// holding its directories open.
struct Grilling {
    /// Dropped last, and only these keep the directories alive: a worktree that
    /// vanished mid-session would fail obscurely.
    _watched: tempfile::TempDir,
    _home: tempfile::TempDir,
    state: tempfile::TempDir,

    /// A directory every sandbox gets read-write, so that a session can leave
    /// evidence of itself somewhere that outlives its worktree.
    _spill: tempfile::TempDir,

    app: Router,
    id: i64,

    /// Where the database is, for the tests that stand a second server up over
    /// it.
    database: PathBuf,
}

impl Grilling {
    /// The Conversation as the workbench reads it.
    async fn view(&self) -> ConversationView {
        get(&self.app, &format!("/api/ui/conversations/{}", self.id)).await
    }

    /// Read it back until the session has got somewhere, or give up.
    async fn until<T>(&self, reached: impl Fn(&ConversationView) -> Option<T>) -> T {
        let deadline = Instant::now() + PATIENCE;

        loop {
            let view = self.view().await;

            if let Some(reached) = reached(&view) {
                return reached;
            }

            assert!(
                Instant::now() < deadline,
                "the session never got there. The Timeline says: {:?}",
                output(&view)
            );

            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// What the session has printed, whole, as the details pane fetches it.
    async fn transcript(&self, event: i64) -> String {
        let transcript: Transcript = get(
            &self.app,
            &format!("/api/ui/conversations/{}/transcript/{event}", self.id),
        )
        .await;

        transcript.text
    }

    /// And one commit's diff, as the same pane fetches that.
    async fn commit_diff(&self, event: i64) -> CommitDiff {
        get(
            &self.app,
            &format!("/api/ui/conversations/{}/commit/{event}", self.id),
        )
        .await
    }

    async fn abort(&self) -> ConversationAborted {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/abort", self.id),
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
    async fn ask(&self, yaml: &str) -> i64 {
        let (status, body) = fetch(
            &self.app,
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{}/api/v1/sets", self.id))
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_owned()))
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED, "the Set was refused: {body}");

        let created: verkstead_schema::SetCreated = serde_saphyr::from_str(&body).unwrap();
        created.id
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

    /// And choose how the work gets built.
    async fn direct(&self, direction: &str) -> DirectionChosen {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/direction", self.id),
            &serde_json::json!({ "direction": direction }),
        )
        .await
    }

    /// Answer a run that stopped, the way the human does from the Timeline.
    async fn settle(&self, event: i64, remedy: &str, note: &str) -> RemedySettled {
        post(
            &self.app,
            &format!("/api/ui/conversations/{}/interruption/{event}", self.id),
            &serde_json::json!({ "remedy": remedy, "note": note }),
        )
        .await
    }

    /// Wait until a run has stopped, and hand back what it stopped at.
    async fn stopped(&self) -> InterruptionEvent {
        self.until(|view| interruptions(view).last().map(|it| (*it).clone()))
            .await
    }
}

/// A closing Set: the proposal that ends a grilling, and the Option that means
/// go ahead.
const PROPOSING: &str = r#"
title: Ready to build the rate limiter
questions:
  - label: Q9
    text: Ready to build it this way?
    options:
      - n: 1
        text: Yes, go ahead
        recommended: true
      - n: 2
        text: Not yet — more to work through
proposal:
  direction: inline
  accepted_by: Q9.1
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
};

/// What stands where the host's `gh` goes: a branch with a pull request on it,
/// and nothing said on it yet.
///
/// A script rather than the real thing, for the reason the agent is one: what
/// these ask is what Verkstead does with the answer, and asking GitHub itself
/// would be a test that needed a network, an account and a pull request.
///
/// It tells the two questions apart by the fields being asked for, because that
/// is what tells them apart on the command line.
const PULL_REQUEST: &str = r#"
case "$5" in
*commits*)
    printf '{"commits":[{"oid":"c0ffee1","messageHeadline":"feat: count the requests"}],"comments":[{"author":{"login":"tobico"},"body":"Looks **good**.","createdAt":"2026-08-21T09:00:00Z"}]}'
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

/// The same, with something else where `gh` goes — for the tests about what
/// Verkstead does when GitHub cannot be asked.
async fn grilling_asking(stub: &str, gh: &str) -> Grilling {
    grilling_spilling(tempfile::tempdir().unwrap(), stub, gh).await
}

/// The same, over a directory the caller already has the name of — which is
/// what a stub that has to write somewhere the worktree is not needs, the
/// script naming the path being written before there is a fixture to ask.
async fn grilling_spilling(spill: tempfile::TempDir, stub: &str, gh: &str) -> Grilling {
    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // Who a session commits as, which every sandbox reads out of the home of
    // whoever runs the server.
    std::fs::write(
        home.path().join(".gitconfig"),
        "[user]\n\tname = Verkstead Test\n\temail = test@verkstead.invalid\n",
    )
    .unwrap();

    let database = state.path().join("verkstead.db");
    let pool = open_database(&database).await.unwrap();

    let agents = Agents::running(
        vec!["/bin/sh".to_owned(), "-c".to_owned(), stub.to_owned()],
        Home {
            path: home.path().to_owned(),
            gh_config: home.path().join(".config/gh"),
        },
        Reachable::at(LISTENING),
        SandboxConfig::resolve(&[spill.path().display().to_string()]).unwrap(),
        Skills::installed(state.path()).expect("this binary carries skills"),
        Handoffs::under(state.path()),
    )
    .at_pace(BRISKLY);

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

    let started: Started = post(
        &app,
        "/api/ui/conversations",
        &serde_json::json!({ "repo_id": repo_id }),
    )
    .await;
    let Started::Started { id } = started else {
        panic!("expected the Conversation to start, got {started:?}");
    };

    for role in ["grilling", "implementation"] {
        let profile = profile(&app, watched.path(), role).await;
        let chosen: verkstead_render::ProfileChosen = post(
            &app,
            &format!("/api/ui/conversations/{id}/{role}-profile"),
            &serde_json::json!({ "profile_id": profile }),
        )
        .await;
        assert_eq!(chosen, verkstead_render::ProfileChosen::Chosen);
    }

    let saved: BriefSaved = post(
        &app,
        &format!("/api/ui/conversations/{id}/brief"),
        &serde_json::json!({ "markdown": BRIEF }),
    )
    .await;
    assert_eq!(saved, BriefSaved::Saved);

    let grilling: GrillingStarted = post(
        &app,
        &format!("/api/ui/conversations/{id}/grill"),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(grilling, GrillingStarted::Started);

    Grilling {
        _watched: watched,
        _home: home,
        state,
        _spill: spill,
        app,
        id,
        database,
    }
}

/// The one agent-output Event on a Timeline, where there is one yet.
fn output(view: &ConversationView) -> Option<&AgentOutputEvent> {
    outputs(view).into_iter().next()
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

/// The Interruptions on a Timeline, in the order the runs stopped.
fn interruptions(view: &ConversationView) -> Vec<&InterruptionEvent> {
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Interruption(stopped) => Some(stopped),
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

/// And the pull request pinned beside it, once the finish step has opened one.
fn pull_request(view: &ConversationView) -> Option<&PullRequestEvent> {
    view.pinned.iter().find_map(|pinned| match pinned {
        PinnedEvent::PullRequest(opened) => Some(opened),
        _ => None,
    })
}

/// The handoff on a Timeline, once the grilling has handed one over.
fn handoff(view: &ConversationView) -> Option<&verkstead_render::HandoffEvent> {
    view.timeline.iter().find_map(|event| match event {
        TimelineEvent::Handoff(handoff) => Some(handoff),
        _ => None,
    })
}

/// An Agent Profile saved from a pair inside `watched`, on a model that is
/// worth reading back.
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
            "model": format!("claude-{name}-5"),
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

    let said = fixture.transcript(event).await;
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
    let said = fixture.transcript(summary.id).await;
    assert!(said.contains("Reading the brief."), "{said:?}");

    assert_eq!(fixture.abort().await, ConversationAborted::Aborted);
}

/// What a terminal was sent is what the session said. Nothing is stripped on
/// the way in — the tidying is for the one line the Timeline shows.
#[tokio::test]
async fn the_details_pane_gets_the_transcript_byte_for_byte() {
    let fixture = grilling(r#"printf '\033[1mbold\033[0m\nplain\n'"#).await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;

    assert_eq!(
        fixture.transcript(event).await,
        // The line endings are the pseudo-terminal's own doing, and they are
        // part of what was sent.
        "\u{1b}[1mbold\u{1b}[0m\r\nplain\r\n",
    );
}

/// A session that has ended is a Conversation with a transcript, not one with an
/// agent in it.
#[tokio::test]
async fn a_session_that_exits_leaves_a_conversation_that_says_so() {
    let fixture = grilling("printf 'done\\n'").await;

    let summary = fixture
        .until(|view| output(view).filter(|output| !output.running).cloned())
        .await;

    assert_eq!(summary.latest, "done");
    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Grilling,
        "where the work has got to is not what the session's ending decides"
    );
}

/// The record is the record. A server that has been restarted has no sessions
/// at all, and every transcript it holds is of one that is over.
#[tokio::test]
async fn a_transcript_survives_the_server_restarting() {
    let fixture = grilling(r#"printf 'first\nsecond\n'"#).await;

    let event = fixture
        .until(|view| output(view).filter(|output| !output.running).map(|o| o.id))
        .await;
    let said = fixture.transcript(event).await;

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
                gh_config: PathBuf::from("/nonexistent/.config/gh"),
            },
            Reachable::at(LISTENING),
            SandboxConfig::default(),
            Skills::installed(fixture.state.path()).expect("this binary carries skills"),
            Handoffs::under(fixture.state.path()),
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

    let read_back: Transcript = get(
        &restarted,
        &format!(
            "/api/ui/conversations/{}/transcript/{}",
            fixture.id, summary.id
        ),
    )
    .await;
    assert_eq!(read_back.text, said);
}

/// The inline direction end to end: the grilling writes a handoff where the
/// skill says, the human accepts its proposal, and choosing *implement inline*
/// runs a fresh session under the *other* Profile — primed with the handoff, and
/// committing without anything to wait on.
///
/// One stub for both sessions, telling them apart by the model it was run on,
/// because that is the fact under all of it: the two run as different accounts,
/// which is why the grilling cannot simply carry on and why the handoff has to
/// exist at all.
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);

    let view = fixture.view().await;
    assert_eq!(view.state, Lifecycle::Direction);
    assert!(
        handoff(&view).is_some_and(|handoff| handoff.html.contains("in-process counter")),
        "the handoff is taken onto the Timeline as the proposal is accepted",
    );
    assert!(
        outputs(&view).iter().all(|output| !output.running),
        "the grilling ended with its proposal: it has its Response and nothing left to do",
    );

    assert_eq!(fixture.direct("inline").await, DirectionChosen::Chosen);

    // The second session, which is a different Event: the first is the grilling,
    // and it ended when its proposal was accepted.
    let implementing = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id != grilling_output && !output.running)
                .map(|output| output.id)
        })
        .await;

    let said = fixture.transcript(implementing).await.replace("\r\n", "\n");

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

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Implementing,
        "the Conversation is building the work",
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

/// The task-list direction end to end: the same Profile and the same worktree as
/// inline, inside Verkstead's fork of to-tasks instead — which writes a real
/// `.tasks/` backlog and commits it to the branch.
///
/// Repo files stay the source of truth, so what this asks of the far end is what
/// git says: the backlog is in the worktree and it is committed. Verkstead runs
/// the workflow that writes it and owns none of it.
#[tokio::test]
async fn choosing_a_task_list_runs_the_breakdown_fork_and_commits_a_backlog() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            printf 'model=%s\n' "$1"
            printf 'prompt=%s\n' "$2"
            grep '^name:' "$HOME/.claude/skills/breaking-down/SKILL.md"
            mkdir -p .tasks
            printf '# Rate limiting\n\n## Tasks\n\n- [ ] 01: count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-counter.md
            git add .tasks
            git commit --quiet -m 'chore: plan rate-limiting tasks'
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.view().await.state, Lifecycle::Direction);

    assert_eq!(fixture.direct("task-list").await, DirectionChosen::Chosen);

    let breaking_down = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id != grilling_output && !output.running)
                .map(|output| output.id)
        })
        .await;

    let said = fixture
        .transcript(breaking_down)
        .await
        .replace("\r\n", "\n");

    assert!(
        said.contains("model=claude-implementation-5"),
        "the breakdown runs under the implementation Profile, as the work it plans does: {said:?}"
    );
    assert!(
        said.contains("~/.claude/skills/breaking-down/SKILL.md"),
        "and inside the bundled fork of to-tasks: {said:?}"
    );
    assert!(
        said.contains("name: breaking-down"),
        "which is really there to be read: installed under the State Directory and \
         bound into the sandbox exactly as grilling's is: {said:?}"
    );
    assert!(
        !said.contains("~/.claude/skills/implementing/SKILL.md"),
        "which is the other direction, and not this one: {said:?}"
    );
    assert!(
        said.contains("A counter per key.") && said.contains(BRIEF),
        "primed with both documents, exactly as an inline session is: {said:?}"
    );

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Implementing,
        "writing the backlog is the work starting rather than a step in front of it",
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
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            if [ ! -d .tasks ]; then
                printf 'breaking down\n'
                mkdir -p .tasks
                printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
                printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
                printf -- '- [ ] 02: refuse the excess\n' >> .tasks/TODO.md
                printf '# 01. Count the requests\n' > .tasks/01-count.md
                printf '# 02. Refuse the excess\n' > .tasks/02-refuse.md
                git add .tasks
                git commit --quiet -m 'chore: plan rate-limiting tasks'
            else
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("task-list").await, DirectionChosen::Chosen);

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

    let view = fixture.view().await;

    assert_eq!(
        outputs(&view).len(),
        5,
        "one Event per session: the grilling, the breakdown, a task each, and the finish",
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
        claude-grilling-5) printf 'grilling\n'; sleep 300 ;;
        *)
            if [ ! -d .tasks ]; then
                mkdir -p .tasks
                printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
                printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
                printf -- '- [ ] 02: refuse the excess\n' >> .tasks/TODO.md
                printf '# 01\n' > .tasks/01-count.md
                printf '# 02\n' > .tasks/02-refuse.md
                git add .tasks
                git commit --quiet -m 'chore: plan rate-limiting tasks'
            else
                next=$(ls .tasks | grep -E '^[0-9]+-' | sort | head -n 1)
                if [ -n "$next" ]; then
                    rm ".tasks/$next"
                    git add -A
                    git commit --quiet -m "feat: $next"
                    # Only the first task, so the list is caught half worked
                    # through rather than empty.
                    sleep 300
                fi
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("task-list").await, DirectionChosen::Chosen);

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

    assert_eq!(fixture.abort().await, ConversationAborted::Aborted);
}

/// One task and then the finish, which is the shortest whole backlog there is —
/// and the one the wrap-up tests below drive to its end.
///
/// The finish step here does what the bundled fork tells a session to do:
/// commits the removal of `TODO.md`, and pushes and opens a pull request through
/// its own `gh`. There is no remote to push to in these fixtures, so the stub
/// says it and stops there — what Verkstead does next is ask the *host's* `gh`,
/// which is the half under test.
const A_BACKLOG_OF_ONE: &str = r#"
case "$1" in
claude-grilling-5) printf 'grilling\n'; sleep 300 ;;
*)
    if [ ! -d .tasks ]; then
        mkdir -p .tasks
        printf '# Rate limiting\n\n## Tasks\n\n' > .tasks/TODO.md
        printf -- '- [ ] 01: count the requests\n' >> .tasks/TODO.md
        printf '# 01\n' > .tasks/01-count.md
        git add .tasks
        git commit --quiet -m 'chore: plan rate-limiting tasks'
    else
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
    fi
    sleep 300
    ;;
esac
"#;

/// Take a Conversation from the direction chooser to a worked-through backlog,
/// with nothing pressed on the way: the whole point of the run is that nobody is
/// asked anything between the direction and the pull request.
async fn worked_to_empty(fixture: &Grilling) {
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("task-list").await, DirectionChosen::Chosen);

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

    let opened = fixture
        .until(|view| {
            (view.state == Lifecycle::Wrapping)
                .then(|| pull_request(view).cloned())
                .flatten()
        })
        .await;

    assert_eq!(opened.number, 41);
    assert_eq!(opened.title, "Rate limiting");
    assert_eq!(opened.url, "https://github.com/tobico/verkstead/pull/41");

    let view = fixture.view().await;

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
        interruptions(&view).is_empty(),
        "nothing stopped: {:?}",
        interruptions(&view),
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

/// And what happens when `gh` cannot answer — no `gh`, no login, or a branch
/// nothing was opened on. The Conversation stays where it is with the reason on
/// its Timeline, rather than becoming a Wrapping with no pull request under it.
#[tokio::test]
async fn a_finish_that_opened_no_pull_request_leaves_the_conversation_where_it_is() {
    let fixture = grilling_asking(A_BACKLOG_OF_ONE, NO_PULL_REQUEST).await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.what.contains("pull request"),
        "the step is named as what it was: {stopped:?}",
    );
    assert!(
        stopped.how.contains("no pull request"),
        "and the reason is `gh`'s, in words: {stopped:?}",
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
        "what is waiting is the human, which is what an Interruption is for",
    );
}

/// A breakdown asks its quiz the way every session asks anything: an ordinary
/// Set, with the session idling until the Answers come back. Nothing about it
/// ends or redirects the Conversation — the direction was settled before this
/// session started.
#[tokio::test]
async fn a_breakdown_question_reaches_the_human_as_an_ordinary_set() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5) printf 'grilling\n'; sleep 300 ;;
        *) printf 'breaking down\n'; sleep 300 ;;
        esac
        "#,
    )
    .await;

    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("task-list").await, DirectionChosen::Chosen);

    let quiz = fixture.ask(BREAKDOWN_QUIZ).await;

    let view = fixture.view().await;
    let asked = sets(&view);
    assert_eq!(
        asked.len(),
        2,
        "the proposal and the quiz, both on the Timeline they were asked from",
    );
    assert_eq!(
        view.proposal.map(|proposal| proposal.direction),
        Some(verkstead_schema::Direction::Inline),
        "an ordinary Set carries no proposal, so the accepted one still stands",
    );

    assert_eq!(fixture.answer(quiz).await, Submitted::Accepted);

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Implementing,
        "answering a breakdown question moves nothing: the direction is settled",
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
        git commit --quiet -m 'feat: rate limiting'

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
    let diff = fixture
        .commit_diff(notes.id)
        .await
        .diff
        .expect("a commit that added a file has a diff");

    assert_eq!(diff.paths, vec!["NOTES.md".to_owned()]);
    assert!(
        diff.html.contains("<details class=\"diff-file\""),
        "the folds the renderer already gives an attached Diff: {}",
        diff.html
    );
    assert!(diff.html.contains("and how"), "{}", diff.html);
    assert!(
        !diff.html.contains("docs: say what it does"),
        "the message is the Event's to say — the diff arrives headerless: {}",
        diff.html
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
async fn aborting_ends_the_session_before_the_worktree_goes() {
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

    assert_eq!(fixture.abort().await, ConversationAborted::Aborted);

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
        "the session was still writing after the abort said it had stopped"
    );
    assert!(
        !worktree.exists(),
        "the worktree should be gone once the Conversation is aborted"
    );
    assert!(
        !output(&fixture.view().await)
            .expect("the transcript stays on the Timeline")
            .running,
        "an aborted Conversation has no session running"
    );
}

/// A run that stops: an implementation session that goes wrong, and everything
/// the human is handed to decide with.
///
/// The whole reason Interruptions exist is that nobody is at the terminal.
/// Verkstead launches the sessions but does not drive them, so a session that
/// falls over is a run that has quietly stopped — and what this asks is whether
/// stopping is *legible*: the Timeline says which step failed, how it ended, what
/// git makes of the worktree and what the session last said, and the Conversation
/// says it is blocked on the human.
///
/// The stub exits 1 after saying something worth reading back, which is a crash
/// as far as anything outside it can tell.
#[tokio::test]
async fn a_session_that_exits_badly_stops_the_run_at_an_interruption() {
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("inline").await, DirectionChosen::Chosen);

    let stopped = fixture.stopped().await;

    assert_eq!(
        stopped.what, "implementing the work inline",
        "which step failed",
    );
    assert_eq!(
        stopped.how, "the session exited with status 1",
        "and how it ended, which is the thing a status can say",
    );
    assert!(
        stopped.git_status.contains("limiter.md"),
        "what git makes of the worktree, which is where the half-done work is: {:?}",
        stopped.git_status,
    );
    assert!(
        stopped
            .tail
            .contains("error: unresolved import crate::window"),
        "and the tail of what the session last said: {:?}",
        stopped.tail,
    );
    assert!(
        !stopped.tail.contains('\u{1b}'),
        "tidied of the terminal's own control sequences: {:?}",
        stopped.tail,
    );
    assert_eq!(
        stopped.settled, None,
        "nobody has answered it, which is what stops the run",
    );

    let view = fixture.view().await;
    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "the Conversation is blocked on the human, and says which Event it is blocked on",
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

/// Retry: the step runs again in a fresh session, told whatever the human wrote
/// alongside.
///
/// The note is the whole point of the remedy taking one — "try again but leave
/// that one alone" is only worth writing if it reaches the agent that can act on
/// it — so what this reads back is the retried session's own prompt.
#[tokio::test]
async fn retrying_runs_the_step_again_in_a_session_told_what_the_human_wrote() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            if [ -f RETRIED ]; then
                printf 'prompt was: %s\n' "$2"
                printf 'a limiter\n' > limiter.md
                git add limiter.md
                git commit --quiet -m 'feat: rate limiting'
                printf 'committed\n'
                sleep 300
            else
                printf 'first go\n' > RETRIED
                exit 1
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("inline").await, DirectionChosen::Chosen);

    let stopped = fixture.stopped().await;
    let before = outputs(&fixture.view().await).len();

    assert_eq!(
        fixture
            .settle(stopped.id, "Retry", "leave the migration alone")
            .await,
        RemedySettled::Settled,
    );

    // A fresh session rather than the old one carrying on, because the old one
    // is gone — that is what an Interruption is.
    let retried = fixture
        .until(|view| {
            let running = outputs(view);
            (running.len() > before).then(|| running[before].id)
        })
        .await;

    let said = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id == retried && output.lines > 1)
                .map(|output| output.id)
        })
        .await;

    let printed = fixture.transcript(said).await.replace("\r\n", "\n");

    assert!(
        printed.contains("leave the migration alone"),
        "what the human wrote reaches the agent that can act on it: {printed:?}",
    );
    assert!(
        printed.contains("~/.claude/skills/implementing/SKILL.md"),
        "and it is the same step, run again: {printed:?}",
    );
    assert!(
        printed.contains(BRIEF),
        "still primed with the documents the work is described by: {printed:?}",
    );

    let settled = fixture
        .view()
        .await
        .timeline
        .iter()
        .find_map(|event| match event {
            TimelineEvent::Interruption(it) if it.id == stopped.id => it.settled.clone(),
            _ => None,
        })
        .expect("the Interruption is settled");

    assert_eq!(settled.remedy, Remedy::Retry);
    assert_eq!(settled.note, "leave the migration alone");

    assert_eq!(
        fixture.view().await.blocked_on,
        None,
        "and nothing is blocked on the human any more",
    );
}

/// The other two remedies, which launch nothing.
///
/// *Take over manually* stops Verkstead driving and leaves the Conversation where
/// it is, because the human is about to work in that worktree themselves. *Abort*
/// ends the run. Both leave the repository exactly as the session left it —
/// which is what makes the first of them a remedy at all, and is why aborting
/// here is not the same thing as aborting a Conversation from its menu.
#[tokio::test]
async fn taking_over_and_aborting_both_leave_the_repo_as_the_session_left_it() {
    for (remedy, expected) in [
        ("TakeOver", Lifecycle::Implementing),
        ("Abort", Lifecycle::Aborted),
    ] {
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
                exit 3
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
        assert_eq!(fixture.answer(set).await, Submitted::Accepted);
        assert_eq!(fixture.direct("inline").await, DirectionChosen::Chosen);

        let stopped = fixture.stopped().await;
        let sessions = outputs(&fixture.view().await).len();

        assert_eq!(
            fixture.settle(stopped.id, remedy, "").await,
            RemedySettled::Settled
        );

        let view = fixture.view().await;

        assert_eq!(
            view.state, expected,
            "{remedy} leaves the Conversation here"
        );
        assert_eq!(
            view.blocked_on, None,
            "{remedy} closes the Interruption, so nothing is waiting on the human",
        );

        assert!(
            worktree.join("limiter.md").exists(),
            "{remedy} leaves the repo as the session left it: none of the three \
             reverts, resets or stashes anything",
        );
        assert!(
            worktree.exists(),
            "{remedy} keeps the worktree, unlike aborting a Conversation from its menu — \
             the human is being handed the wreckage on purpose",
        );

        // Long enough for a runner that was going to launch something to have
        // done it.
        tokio::time::sleep(Duration::from_secs(2)).await;

        assert_eq!(
            outputs(&fixture.view().await).len(),
            sessions,
            "{remedy} launches nothing: Verkstead has stopped driving",
        );
    }
}

/// The second press of a button is the first choice arriving again, not a new
/// decision — the human answers from whichever device is to hand.
#[tokio::test]
async fn a_remedy_pressed_twice_is_the_first_choice_arriving_again() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf 'grilling\n'
            sleep 300
            ;;
        *)
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("inline").await, DirectionChosen::Chosen);

    let stopped = fixture.stopped().await;

    assert_eq!(
        fixture.settle(stopped.id, "TakeOver", "").await,
        RemedySettled::Settled,
    );
    assert_eq!(
        fixture.settle(stopped.id, "Abort", "").await,
        RemedySettled::AlreadySettled,
        "and the second choice is not acted on",
    );

    assert_eq!(
        fixture.view().await.state,
        Lifecycle::Implementing,
        "the first choice stands",
    );

    assert_eq!(
        fixture.settle(9999, "Retry", "").await,
        RemedySettled::NoSuchInterruption,
        "an Event that is not an Interruption of this Conversation names nothing",
    );
}

/// A backlog whose task session dies: the run stops at that task rather than
/// going round again, and the Interruption says which task it was.
///
/// This is the case that matters most, because a runner is a loop: one that
/// relaunched a step nothing had moved would be a machine spending an account on
/// the same failure over and over, with nobody watching.
#[tokio::test]
async fn a_backlog_stops_at_the_task_whose_session_died() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
            printf '# What we settled\n\nA counter per key.\n' > /tmp/verkstead/handoff.md
            printf 'the handoff is written\n'
            sleep 300
            ;;
        *)
            if [ -f .tasks/TODO.md ]; then
                printf 'this task is beyond me\n'
                exit 1
            fi

            mkdir -p .tasks
            printf '# Rate limiting\n\n- [ ] 01: Count the requests\n' > .tasks/TODO.md
            printf '# 01. Count the requests\n' > .tasks/01-count.md
            git add .tasks
            git commit --quiet -m 'chore: plan the rate limiter'
            printf 'the backlog is written\n'
            sleep 300
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("task-list").await, DirectionChosen::Chosen);

    let stopped = fixture.stopped().await;

    assert_eq!(
        stopped.what, "the task in .tasks/01-count.md",
        "the Interruption names the step that failed, so the human knows what to \
         decide about",
    );
    assert_eq!(stopped.how, "the session exited with status 1");
    assert!(
        stopped.tail.contains("this task is beyond me"),
        "with the tail of what it said on its way out: {:?}",
        stopped.tail,
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
        "the run does not advance past an Interruption",
    );
    assert_eq!(
        interruptions(&fixture.view().await).len(),
        1,
        "and it does not stop twice over the same thing either",
    );

    assert!(
        worktree.join(".tasks/01-count.md").exists(),
        "the task is still there to be worked, because nothing reverted anything",
    );
}

/// Aborting a conversation is not a run that went wrong.
///
/// The abort ends the session and takes the worktree away, so every signal the
/// runner reads says the step did not land — the file is gone because the whole
/// directory is. What tells it apart is that Verkstead is what ended the session:
/// raising an Interruption here would be asking the human what to do about the
/// thing they had just done.
#[tokio::test]
async fn aborting_a_run_is_not_something_to_ask_the_human_about() {
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("task-list").await, DirectionChosen::Chosen);

    // Once the breakdown session is up, so there is a run to abort mid-step.
    fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.running && output.lines > 0)
                .map(|output| output.id)
        })
        .await;

    assert_eq!(fixture.abort().await, ConversationAborted::Aborted);

    // Long enough for the driver to have noticed its session go and decided what
    // that meant.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let view = fixture.view().await;

    assert_eq!(
        interruptions(&view),
        Vec::<&InterruptionEvent>::new(),
        "a run the human stopped has nothing to ask them about",
    );
    assert_eq!(
        view.blocked_on, None,
        "and an aborted Conversation is not blocked on anybody",
    );
    assert_eq!(view.state, Lifecycle::Aborted);
}
