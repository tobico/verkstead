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
                    gh_config: self.home.path().join(".config/gh"),
                },
                Reachable::at(LISTENING),
                SandboxConfig::resolve(&[self.spill.path().display().to_string()]).unwrap(),
                Skills::installed(self.state.path()).expect("this binary carries skills"),
                Handoffs::under(self.state.path()),
            )
            .at_pace(BRISKLY),
            gh_stub(gh),
        )
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

    /// The same, for a Set whose questions are not the proposal's one.
    async fn respond(&self, set_id: i64, answers: serde_json::Value) -> Submitted {
        post(
            &self.app,
            &format!("/api/ui/sets/{set_id}/response"),
            &serde_json::json!({ "answers": answers }),
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
    checks: Duration::from_millis(100),
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

/// The same, for a check that is red until `green` is there and passing once it
/// is — which is how a test moves GitHub on between one poll and the next.
fn gh_checking_until(green: &Path) -> String {
    format!(
        "if [ -e {green} ]; then how=SUCCESS; else how=FAILURE; fi\n{}",
        gh_checking("$how"),
        green = quoted(green),
    )
}

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
        home,
        state,
        spill,
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

/// A sandbox that will not start says why where somebody is looking.
///
/// bwrap and `script` talk on the pipe beside the pseudo-terminal, and nothing
/// they say ever reaches it — so a session whose sandbox never came up prints
/// nothing at all. Its Event would otherwise read `0 lines` with the only
/// account of why in a log at a level nobody has turned on, which is a failure
/// that looks exactly like an agent with nothing to say.
///
/// Provoked through a configured bind, because that is the one part of a sandbox
/// that is checked at startup and can go on to be missing: what fails here is
/// bwrap and nothing else, on a Conversation whose worktree and Profile are both
/// fine.
#[tokio::test]
async fn a_sandbox_that_will_not_start_says_why_on_the_transcript() {
    let fixture = grilling(
        r#"
        case "$1" in
        claude-grilling-5)
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

    let grilled = fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    fixture
        .until(|view| {
            outputs(view)
                .iter()
                .all(|output| !output.running)
                .then_some(())
        })
        .await;

    // The bind the server admitted at startup, gone by the time the next session
    // wants it. Taken away only once the grilling has ended, because a directory
    // bound into a sandbox that is still up is one the host cannot remove.
    let missing = fixture.spill.path().to_owned();
    std::fs::remove_dir_all(&missing).unwrap();

    assert_eq!(fixture.direct("inline").await, DirectionChosen::Chosen);

    let refused = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id != grilled && !output.running)
                .map(|output| (output.id, output.lines, output.latest.clone()))
        })
        .await;

    let (event, lines, latest) = refused;
    let said = fixture.transcript(event).await;

    assert!(
        said.contains("[the sandbox, not the agent]"),
        "the plumbing's word goes on the transcript, marked as its own rather \
         than left to read as something the agent said: {said:?}"
    );
    assert!(
        said.contains(&missing.display().to_string()),
        "and it names the bind bwrap could not make: {said:?}"
    );
    assert!(
        lines > 0 && !latest.is_empty(),
        "so the Timeline says what happened rather than `0 lines` and nothing: \
         {lines} lines, latest {latest:?}"
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

/// The roadmap direction end to end: the same Profile and the same worktree
/// again, inside Verkstead's fork of to-roadmap this time — which writes a real
/// `docs/roadmaps/<name>/` and commits it to the branch.
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
async fn choosing_a_roadmap_runs_the_staging_fork_and_commits_a_roadmap() {
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
            grep '^name:' "$HOME/.claude/skills/staging/SKILL.md"
            mkdir -p docs/roadmaps/rate-limiting
            printf '# Rate limiting roadmap\n\n## Stages\n\n- [x] 01: Count the requests — [brief](01-counter.md)\n- [ ] 02: Refuse the rest — [brief](02-refusing.md)\n' > docs/roadmaps/rate-limiting/ROADMAP.md
            printf '# 01. Count the requests\n' > docs/roadmaps/rate-limiting/01-counter.md
            printf '# 02. Refuse the rest\n' > docs/roadmaps/rate-limiting/02-refusing.md
            git add docs
            git commit --quiet -m 'docs: stage the rate-limiting roadmap'
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
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.view().await.state, Lifecycle::Direction);

    assert_eq!(fixture.direct("roadmap").await, DirectionChosen::Chosen);

    let staging = fixture
        .until(|view| {
            outputs(view)
                .into_iter()
                .find(|output| output.id != grilling_output && output.lines > 0)
                .map(|output| output.id)
        })
        .await;

    let said = fixture.transcript(staging).await.replace("\r\n", "\n");

    assert!(
        said.contains("model=claude-implementation-5"),
        "the staging runs under the implementation Profile, as the work it plans does: {said:?}"
    );
    assert!(
        said.contains("~/.claude/skills/staging/SKILL.md"),
        "and inside the bundled fork of to-roadmap: {said:?}"
    );
    assert!(
        said.contains("name: staging"),
        "which is really there to be read: installed under the State Directory and \
         bound into the sandbox exactly as grilling's is: {said:?}"
    );
    assert!(
        !said.contains("~/.claude/skills/breaking-down/SKILL.md")
            && !said.contains("~/.claude/skills/implementing/SKILL.md"),
        "which are the other two directions, and not this one: {said:?}"
    );
    assert!(
        said.contains("A counter per key.") && said.contains(BRIEF),
        "primed with both documents, exactly as the other two are: {said:?}"
    );

    // Writing the roadmap is the work starting rather than a step in front of
    // it, and a roadmap is work like any other work: the fork carries the branch
    // to a pull request the same way a finished backlog does, and the
    // Conversation moves on to wrapping that up.
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
            Lifecycle::Direction,
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
        ],
        "through Implementing on the way, because writing the roadmap is the work",
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
            case "$2" in
            *reviewing/SKILL.md*)
                printf 'I read the whole branch and found nothing worth raising\n'
                exit 0
                ;;
            esac
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

    // The finish step is not the last session a backlog leads to: the pull
    // request it opened is reviewed, which is where the wrap-up starts.
    fixture
        .until(|view| {
            let sessions = outputs(view);
            (sessions.len() == 6 && sessions.iter().all(|output| !output.running)).then_some(())
        })
        .await;

    let view = fixture.view().await;

    assert_eq!(
        outputs(&view).len(),
        6,
        "one Event per session: the grilling, the breakdown, a task each, the finish, \
         and the review of the pull request it opened",
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
        6,
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
claude-grilling-5) printf 'grilling\n'; sleep 300 ;;
*)
    case "$2" in
    *reviewing/SKILL.md*)
        printf 'I read the whole branch and found nothing worth raising\n'
        exit 0
        ;;
    esac
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
        interruptions(&view).is_empty(),
        "nothing stopped: {:?}",
        interruptions(&view),
    );
    assert!(checks_settled(&fixture).await, "and they are still green");
}

/// The whole of what a red check costs: two fix sessions, and then the human.
///
/// The first failure dispatches one fix session inside the bundled addressing
/// skill, under the implementation Profile, given the check as its feedback. It
/// commits, the check is still red, and it gets one more. After that Verkstead
/// stops asking the machine: an Interruption carries which check failed and what
/// the last session said, and nothing further is dispatched for it.
#[tokio::test]
async fn a_check_two_fix_sessions_could_not_fix_stops_and_asks_the_human() {
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

    // Which step it was, and what makes the choice answerable from a phone:
    // which check is red, where its run is, and what the last fix session said.
    assert!(
        stopped.what.contains("checks"),
        "the step is named as what it was: {stopped:?}",
    );
    assert!(
        stopped.how.contains("Rust") && stopped.how.contains("/actions/runs/1/job/2"),
        "and the reason names the check and its run: {stopped:?}",
    );
    assert!(
        stopped.tail.contains("having a go at the check"),
        "with the tail of what the last fix session said: {stopped:?}",
    );

    let view = fixture.view().await;

    assert_eq!(
        view.blocked_on,
        Some(stopped.id),
        "what is waiting is the human, which is what an Interruption is for",
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
        "the run does not go round again while the human is being asked",
    );
    assert_eq!(
        interruptions(&view).len(),
        1,
        "and it is asked once: {:?}",
        interruptions(&view),
    );
}

/// And the remedy that means *have another go*: the human has read which check
/// failed, done whatever they were going to do about it, and asked for the
/// machine to try again.
///
/// Retrying a checks Interruption is the fix sessions starting over from no
/// attempts spent, which is what makes it answerable at all — a count left
/// standing would raise the same Interruption on the next poll without
/// dispatching anything.
#[tokio::test]
async fn retrying_a_red_check_watches_it_again_from_no_attempts_spent() {
    let prompts = tempfile::tempdir().unwrap();
    let written = prompts.path().join("fix-prompts");
    let green = prompts.path().join("green");

    let fixture = grilling_spilling(
        prompts,
        &a_backlog_then_fixes(&written),
        &gh_checking_until(&green),
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert_eq!(fixes(&fixture.view().await), 2, "the two goes it had");

    // Whatever the human went off and did about it, done: the check GitHub is
    // running is green from the next poll onwards.
    std::fs::write(&green, "").unwrap();

    assert_eq!(
        fixture.settle(stopped.id, "Retry", "").await,
        RemedySettled::Settled,
    );

    let deadline = Instant::now() + PATIENCE;
    while !checks_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the retry never looked at the checks again",
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let view = fixture.view().await;

    assert_eq!(
        fixes(&view),
        2,
        "and nothing was dispatched at a check that had gone green on its own",
    );
    assert!(view.blocked_on.is_none(), "nothing is waiting on the human");
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
        interruptions(&view).is_empty(),
        "and nothing was raised about it: a login to renew is not a run that stopped — {:?}",
        interruptions(&view),
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

    assert_eq!(fixture.abort().await, ConversationAborted::Aborted);

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
                gh_config: PathBuf::from("/nonexistent/.config/gh"),
            },
            Reachable::at(LISTENING),
            SandboxConfig::default(),
            Skills::installed(fixture.state.path()).expect("this binary carries skills"),
            Handoffs::under(fixture.state.path()),
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

/// The findings of a review, as the bundled reviewing skill writes them: a
/// Question per finding, and the `review` block that says which Answer to each
/// means *fix it*.
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
review:
  findings:
    - fix: Q1.1
      what: Reset the counter as the window rolls.
    - fix: Q2.1
      what: Collapse the two clocks onto one.
"#;

/// The shortest whole backlog, plus the two sessions a wrap-up dispatches.
///
/// The review writes down the prompt it was given and then idles, which is what
/// a session blocked on `verkstead ask` does; a fix session writes its prompt
/// down and commits, which is what one reports through. Told apart by the skill
/// their prompts name, because that is the fact under it — all three run under
/// the same implementation Profile and differ only in what they were sent to do.
fn a_backlog_then_wraps_up(reviews: &Path, dispatched: &Path, review: &str) -> String {
    format!(
        r#"
case "$2" in
*reviewing/SKILL.md*)
    printf 'model=%s\n%s\n=====\n' "$1" "$2" >> {reviews}
{review}
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
        dispatched = quoted(dispatched),
    )
}

/// A review session that reads the branch and then waits on the human, which is
/// what one blocked on `verkstead ask` looks like from outside.
const REVIEW_THEN_WAIT: &str = "    printf 'reading the branch\\n'\n    sleep 300";

/// One that finds nothing, says so as the last thing it prints, and stops.
const REVIEW_AND_FIND_NOTHING: &str =
    "    printf 'I read the whole branch and found nothing worth raising\\n'";

/// Whether Verkstead has recorded this Conversation's review as done with.
async fn review_settled(fixture: &Grilling) -> bool {
    let pool = open_database(&fixture.database).await.unwrap();
    let settled = verkstead_server::store::wrap_up_settled(&pool, fixture.id)
        .await
        .unwrap();
    pool.close().await;

    settled.contains(&verkstead_server::store::WaitingOn::Review)
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

/// How many sessions wrote to one of those files.
fn prompts(written: &str) -> Vec<&str> {
    written
        .split("=====")
        .filter(|prompt| !prompt.trim().is_empty())
        .collect()
}

/// The whole of the wrap-up self-review: one fresh session reads the branch, its
/// findings arrive as a Question Set, and the ones the human accepts become work.
///
/// The session that reviews is the first thing to see this branch — the ones that
/// wrote it each saw one task — so it runs in a fresh context, under the
/// implementation Profile, inside the bundled reviewing skill. It changes nothing:
/// what it produces is the Set, and what becomes of each finding is the human's.
#[tokio::test]
async fn the_review_puts_its_findings_to_the_human_and_what_they_accept_becomes_work() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_WAIT),
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

    // Verkstead ends it once the findings are in: the Response is not its to wait
    // for, and each finding the human accepts gets a session of its own.
    fixture
        .until(|view| {
            outputs(view)
                .last()
                .filter(|output| !output.running)
                .map(|_| ())
        })
        .await;

    let view = fixture.view().await;

    assert_eq!(fixes(&view), 0, "the review itself changes nothing");
    assert!(
        !review_settled(&fixture).await,
        "and the review is what wrap-up is waiting on until they answer it",
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

    let told = until_written(&dispatched).await;

    assert!(
        told.contains("addressing/SKILL.md"),
        "a finding they accepted goes to the bundled addressing skill: {told}",
    );
    assert!(
        told.contains("Reset the counter as the window rolls"),
        "carrying the finding as the review wrote it for whoever fixes it: {told}",
    );
    assert!(
        told.contains("Keep the signature."),
        "and what they said when they agreed: {told}",
    );

    // Long enough for a second session, had anything been going to dispatch one.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let told = std::fs::read_to_string(&dispatched).unwrap();

    assert_eq!(
        prompts(&told).len(),
        1,
        "one session for the one finding they accepted: {told}",
    );
    assert!(
        !told.contains("Collapse the two clocks"),
        "the finding they declined dispatches nothing at all: {told}",
    );

    assert!(
        review_settled(&fixture).await,
        "and answering it is what settles the review",
    );

    let view = fixture.view().await;

    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        1,
        "nothing reviews the branch a second time",
    );
    assert!(
        interruptions(&view).is_empty(),
        "nothing stopped: {:?}",
        interruptions(&view),
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

/// A review session that dies is not a review that found nothing.
///
/// One is a branch nobody has read and the other is a branch somebody read and
/// had nothing to say about, and reading the first as the second would let a
/// crash pass for a clean bill of health. So the run stops at an Interruption
/// like every other, and retrying it is the review over again.
#[tokio::test]
async fn a_review_session_that_dies_stops_the_run_and_is_run_again_on_a_retry() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");
    let mended = spill.path().join("mended");

    // Falls over the first time, and reads the branch once whatever the human
    // went off and did about it is done.
    let review = format!(
        "    if [ -e {mended} ]; then\n        \
             printf 'I read the whole branch and found nothing worth raising\\n'\n    \
         else\n        \
             printf 'gh: could not read the diff\\n'\n        \
             exit 1\n    \
         fi",
        mended = quoted(&mended),
    );

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, &review),
        PULL_REQUEST,
    )
    .await;

    worked_to_empty(&fixture).await;

    let stopped = fixture.stopped().await;

    assert!(
        stopped.what.contains("review"),
        "the step is named as what it was: {stopped:?}",
    );
    assert!(
        stopped.tail.contains("could not read the diff"),
        "with the tail of what the session said, which is where it says why: {stopped:?}",
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

    std::fs::write(&mended, "").unwrap();

    assert_eq!(
        fixture.settle(stopped.id, "Retry", "").await,
        RemedySettled::Settled,
    );

    let deadline = Instant::now() + PATIENCE;
    while !review_settled(&fixture).await {
        assert!(
            Instant::now() < deadline,
            "the retry never reviewed anything"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert_eq!(
        prompts(&std::fs::read_to_string(&reviews).unwrap()).len(),
        2,
        "the review that failed, and the one the retry ran",
    );
}

/// One agent in one Worktree, which is what the wrap-up's turns are for.
///
/// The checks are watched while the review reads the branch, and starting a
/// session for a Conversation *ends* the one it already has — so a red check
/// dispatching a fix session mid-review would kill the review, and nothing would
/// ever say so. It waits for the Worktree instead, and takes it when the review
/// has put its findings down.
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
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_THEN_WAIT),
        &gh_checking_after(&reviews),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_written(&reviews).await;

    // Long enough for many polls of a suite that is red the whole time.
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        !dispatched.exists(),
        "nothing was dispatched into the Worktree the review is reading: {:?}",
        std::fs::read_to_string(&dispatched).ok(),
    );
    let view = fixture.view().await;

    assert!(
        outputs(&view).last().is_some_and(|output| output.running),
        "and the review session is still the one running: {:?}",
        outputs(&view).last(),
    );

    // The review reports and its session ends, which hands the Worktree on.
    fixture.ask(REVIEW).await;

    let told = until_written(&dispatched).await;

    assert!(
        told.contains("Rust") && told.contains("/actions/runs/1/job/2"),
        "the fix session that was waiting is about the red check: {told}",
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

/// What is said on the pull request reaches a session — from all three places a
/// human writes — and everything said in a minute reaches **one**.
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
async fn new_comments_dispatch_one_session_between_them_rather_than_one_each() {
    let spill = tempfile::tempdir().unwrap();
    let reviews = spill.path().join("review-prompts");
    let dispatched = spill.path().join("fix-prompts");

    let fixture = grilling_spilling(
        spill,
        &a_backlog_then_wraps_up(&reviews, &dispatched, REVIEW_AND_FIND_NOTHING),
        &gh_about(GREEN, THREE_COMMENTS, TWO_ON_THE_DIFF),
    )
    .await;

    worked_to_empty(&fixture).await;
    until_written(&dispatched).await;

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

    let told = std::fs::read_to_string(&dispatched).unwrap();

    assert_eq!(
        prompts(&told).len(),
        1,
        "one session between the five, rather than one each: {told}",
    );
    assert!(
        told.contains("addressing/SKILL.md"),
        "inside the bundled addressing skill: {told}",
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
    let gh = gh_about(CHECKS_UNANSWERABLE, THREE_COMMENTS, "");

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
        interruptions(&view).is_empty(),
        "nothing stopped: {:?}",
        interruptions(&view),
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

/// A roadmap Conversation that stages, wraps up and settles — with a stub that
/// tells the four sessions apart by the skill their prompts name, which is the
/// fact under them: all of them run under the one implementation Profile.
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
    sleep 300
    ;;
*staging/SKILL.md*)
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

/// Take a roadmap Conversation from its Brief to a settled wrap-up: staged,
/// pushed, reviewed, green, done — with nothing pressed but the two the human
/// presses at the start.
async fn staged_and_settled(fixture: &Grilling) {
    fixture
        .until(|view| output(view).filter(|output| output.lines > 0).map(|o| o.id))
        .await;

    let set = fixture.ask(PROPOSING).await;
    assert_eq!(fixture.answer(set).await, Submitted::Accepted);
    assert_eq!(fixture.direct("roadmap").await, DirectionChosen::Chosen);

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
    view.timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Notice(notice) => Some(notice.html.clone()),
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
            .implementation_profile
            .as_ref()
            .map(|profile| profile.name.clone()),
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

    // The annotation itself, which is what says the stage is in flight and on
    // what — and what stops the stage after it being read as this one again.
    let index = std::fs::read_to_string(worktree.join("docs/roadmaps/rate-limiting/ROADMAP.md"))
        .expect("the roadmap is on the stage's branch too");

    assert!(
        index.contains("*(in progress: `counter`)*"),
        "the roadmap says which branch stage 01 is being worked on: {index:?}",
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

/// A roadmap with every stage checked starts nothing, and the Timeline says why.
///
/// Which is where the whole pipeline stops of its own accord: there is no stage
/// left, so there is nothing to carry on and nothing for the human to do.
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

    staged_and_settled(&fixture).await;

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
}
