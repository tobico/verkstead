//! A Profile's login, run by Verkstead in the Profile's own sandbox: the
//! address it prints, the code handed back, and the login landing in the
//! account — proved against a shell script standing where `claude` goes.
//!
//! The script is what Claude Code 2.1.283 does over plain pipes: a line with
//! the address on it, a prompt with no line ending, and a code read off
//! standard input. It writes the login where Claude would, under the HOME the
//! sandbox gives it, so what reaches the account is whatever the write-back a
//! session's ending does carries there.
//!
//! And the Antigravity CLI's the same way, against a script that is what agy
//! 1.4.12 does — see [`agy_stub`]; and a harness a plugin declares, against
//! the stub harness plugin and a script written for its component to read —
//! see [`plugin_stub`].

#![cfg(target_os = "linux")]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    AgentType, Broken, GrillingStarted, HarnessEntry, LoginState, ProfileEntry, ProfileTrouble,
};
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::{Agents, Gh, open_database, router_running_sessions, store};

use crate::stub_harness::{self, StubHarness};

/// The models a stub harness Profile here offers: the first its default.
const PLUGIN_MODELS: [&str; 2] = ["stub-large", "stub-grilling"];

/// How long anything here is waited for before the test says what it saw.
const WAITING: Duration = Duration::from_secs(20);

/// The code the stub takes. Any other is refused, the way Claude Code 2.1.283
/// refuses one — see [`stub`].
const GOOD_CODE: &str = "good#code";

/// What stands where `claude` goes. `$0` and `$1` are the words after it —
/// `auth login` or `auth status` — and the marker is in the script's own text,
/// which is what a process is found by on the host — see [`running`].
///
/// A wrong code is refused the two ways Claude Code 2.1.283 refuses one: a
/// code with no `#` in it is not a code at all, and is asked for again on the
/// same address; one that is shaped right but wrong is a failed login, and the
/// harness exits.
fn stub(marker: &str) -> String {
    format!(
        r#"# {marker}
case "$1" in
login)
  echo started >> ./started
  echo "Opening browser to sign in..."
  echo "If the browser didn't open, visit: https://claude.com/cai/oauth/authorize?code=true&state=$(od -An -N4 -tu4 /dev/urandom | tr -d ' ')"
  printf 'Paste code here if prompted > '
  while read code; do
    case "$code" in
    *#*) break ;;
    *) echo "Invalid code. Please make sure the full code was copied." >&2 ;;
    esac
  done
  [ "$code" = "{GOOD_CODE}" ] || {{ echo "Login failed: Request failed with status code 400" >&2; exit 1; }}
  printf '{{"claudeAiOauth":{{"accessToken":"%s"}}}}' "$code" > "$HOME/.claude/.credentials.json"
  ;;
status)
  [ -f "$HOME/.claude/.credentials.json" ]
  ;;
esac
"#
    )
}

/// What stands where `agy` goes: what agy 1.4.12 does, measured against the
/// real binary. `$0` is the first word after it — `-p` for the login, `models`
/// for the question of whether the account is signed in.
///
/// It offers the address only to a standard input that is a terminal, and
/// says it on standard error with the address on a line of its own. A wrong
/// code is a failed exchange and an exit. Where `./expiring` is in the
/// directory it was started in, the first of it gives up waiting for a code
/// at once, the way agy does after a minute.
fn agy_stub(marker: &str) -> String {
    format!(
        r#"# {marker}
case "$0" in
-p)
  echo started >> ./started
  [ -t 0 ] || {{ echo "Error: authentication required. Run 'agy' to log in, then retry." >&2; exit 1; }}
  echo "Authentication required. Please visit the URL to log in:" >&2
  echo "  https://accounts.google.com/o/oauth2/auth?access_type=offline&state=$(od -An -N4 -tu4 /dev/urandom | tr -d ' ')" >&2
  echo >&2
  echo "Waiting for authentication (timeout 60s)..." >&2
  echo "Or, paste the authorization code here and press Enter:" >&2
  if [ -f ./expiring ] && [ "$(wc -l < ./started)" -eq 1 ]; then
    sleep 1
    echo "Error: authentication timed out." >&2
    echo "error: authentication failed or timed out" >&2
    exit 1
  fi
  read code
  [ "$code" = "{GOOD_CODE}" ] || {{ echo "Error: authentication failed: token exchange failed: oauth2: \"invalid_grant\" \"Malformed auth code.\"" >&2; exit 1; }}
  printf '{{"token":{{"access_token":"%s"}}}}' "$code" > "$HOME/.gemini/antigravity-cli/antigravity-oauth-token"
  printf '/help\tShow available commands and keybindings\n'
  ;;
models)
  echo "Fetching available models..." >&2
  [ -f "$HOME/.gemini/antigravity-cli/antigravity-oauth-token" ] && {{ echo "gemini-pro"; exit 0; }}
  echo "Error: Please sign in to view available models. Launch the CLI without arguments to sign in." >&2
  exit 1
  ;;
esac
"#
    )
}

/// What stands where a plugin harness's binary goes: a login that says what the
/// stub harness's component recognises — see `plugins/tests/stub-harness` —
/// and nothing a built-in says. `$1` is the second of the manifest's words,
/// `auth login` or `auth status`.
///
/// It prints the address on a line, then a prompt with no line ending, and
/// reads a code: one with no `#` in it is too short and asked for again, a
/// wrong one is refused with an exit. Where `./expiring` is in the directory it
/// was started in, the first of it gives up at once. And its status question
/// leaves `./asked` behind, so that a suite can see it was asked.
fn plugin_stub(marker: &str) -> String {
    format!(
        r#"# {marker}
case "$1" in
login)
  echo started >> ./started
  echo "Stub login. Go to: https://stub.example/login?state=$(od -An -N4 -tu4 /dev/urandom | tr -d ' ')"
  if [ -f ./expiring ] && [ "$(wc -l < ./started)" -eq 1 ]; then
    sleep 1
    echo "Gave up." >&2
    exit 1
  fi
  printf 'Code? '
  while read code; do
    case "$code" in
    *#*) break ;;
    *) echo "Too short."; printf 'Code? ' ;;
    esac
  done
  [ "$code" = "{GOOD_CODE}" ] || {{ echo "Wrong code." >&2; exit 1; }}
  printf '{{"token":"%s"}}' "$code" > "$HOME/.stub/auth.json"
  echo "Signed in."
  ;;
status)
  echo asked >> ./asked
  [ -f "$HOME/.stub/auth.json" ]
  ;;
esac
"#
    )
}

/// What a [`Workbench`] stands where every type's binary goes.
enum Standing {
    /// [`stub`].
    Claude,
    /// [`agy_stub`].
    Agy,
    /// [`plugin_stub`].
    Plugin,
    /// This line, whatever it is.
    Line(Vec<String>),
}

struct Workbench {
    app: Router,
    pool: SqlitePool,
    dir: tempfile::TempDir,
    accounts: tempfile::TempDir,
    marker: String,
    _home: tempfile::TempDir,
}

impl Workbench {
    async fn new(limit: Duration) -> Workbench {
        Workbench::harnessed(limit, Standing::Claude).await
    }

    /// The same, with [`agy_stub`] standing where every type's binary goes.
    async fn agy(limit: Duration) -> Workbench {
        Workbench::harnessed(limit, Standing::Agy).await
    }

    /// The same, with [`plugin_stub`] standing there.
    async fn plugin(limit: Duration) -> Workbench {
        Workbench::harnessed(limit, Standing::Plugin).await
    }

    /// The same, with `harness` standing where every type's binary goes.
    async fn harnessed(limit: Duration, harness: Standing) -> Workbench {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let accounts = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let marker = format!(
            "verkstead-login-stub-{}",
            dir.path().file_name().unwrap().to_string_lossy()
        );

        let agents = Agents::running(
            match harness {
                Standing::Claude => vec!["/bin/sh".to_owned(), "-c".to_owned(), stub(&marker)],
                Standing::Agy => vec!["/bin/sh".to_owned(), "-c".to_owned(), agy_stub(&marker)],
                Standing::Plugin => {
                    vec!["/bin/sh".to_owned(), "-c".to_owned(), plugin_stub(&marker)]
                }
                Standing::Line(harness) => harness,
            },
            Homes::on(Platform::HERE, home.path().to_owned(), dir.path()),
            Reachable::at(LISTENING),
            SandboxConfig::default(),
            BuildCache::none(),
            Skills::installed(Platform::HERE, dir.path()).expect("this binary carries skills"),
            Executable::of_the_server(dir.path()),
            Handoffs::under(dir.path()),
            Attachments::under(dir.path()),
            Settings::in_data_dir(dir.path()),
        )
        .logging_in_within(limit)
        .checking_logins();

        // Who a branch Verkstead cuts is committed by, which a press that
        // starts work is refused without.
        std::fs::write(
            dir.path().join("config.yaml"),
            "authors:\n  - id: work\n    name: Verkstead Test\n    email: test@verkstead.invalid\n",
        )
        .unwrap();

        let app = router_running_sessions(
            pool.clone(),
            dir.path().to_owned(),
            agents,
            Gh::running(vec!["/bin/false".to_owned()]),
        );

        Workbench {
            app,
            pool,
            dir,
            accounts,
            marker,
            _home: home,
        }
    }

    /// A Claude Profile with no login yet, saved the way the form saves one.
    async fn claude_profile(&self, name: &str) -> String {
        let (claude_dir, config_file) = self.pair(name);

        let saved = self
            .press(
                "/api/ui/profiles",
                serde_json::json!({
                    "name": name,
                    "account": {
                        "agent_type": "Claude",
                        "paths": {
                            "claude_dir": claude_dir,
                            "config_file": config_file,
                        },
                    },
                    "models": ["claude-opus-5"],
                }),
            )
            .await;

        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);

        store::profiles(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .find(|profile| profile.name.as_deref() == Some(name))
            .expect("the Profile was saved")
            .id
    }

    /// An Antigravity CLI Profile with no login yet, saved the way the form
    /// saves one.
    async fn agy_profile(&self, name: &str) -> String {
        let home = self.accounts.path().join(name).join(".gemini");
        std::fs::create_dir_all(home.join("antigravity-cli")).unwrap();

        let saved = self
            .press(
                "/api/ui/profiles",
                serde_json::json!({
                    "name": name,
                    "account": { "agent_type": "Agy", "paths": { "home": home } },
                    "models": ["gemini-pro"],
                }),
            )
            .await;

        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);

        store::profiles(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .find(|profile| profile.name.as_deref() == Some(name))
            .expect("the Profile was saved")
            .id
    }

    /// A Profile of the stub harness plugin `harness`, installed first, with
    /// no login yet — saved the way the form saves one.
    async fn plugin_profile(&self, harness: StubHarness, name: &str) -> String {
        harness
            .installed(&self.app, self.accounts.path(), PLUGIN_MODELS)
            .await;

        let saved = self
            .press(
                "/api/ui/profiles",
                serde_json::json!({
                    "name": name,
                    "account": {
                        "agent_type": harness.uuid(),
                        "paths": stub_harness::account(&self.accounts.path().join(name)),
                    },
                    "models": PLUGIN_MODELS,
                }),
            )
            .await;

        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);

        store::profiles(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .find(|profile| profile.name.as_deref() == Some(name))
            .expect("the Profile was saved")
            .id
    }

    /// The entry the harness list has for the stub harness plugin `harness`.
    async fn listed(&self, harness: StubHarness) -> HarnessEntry {
        let answered = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/harnesses")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();
        let listed: Vec<HarnessEntry> = serde_json::from_slice(&bytes).unwrap();

        listed
            .into_iter()
            .find(|entry| entry.id.as_str() == harness.uuid())
            .expect("the stub harness is on the list")
    }

    /// The login a stub harness account named `name` holds, if any.
    fn plugin_login_of(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.accounts.path().join(name).join(".stub/auth.json")).ok()
    }

    /// How many times the status of `profile`'s account has been asked of a
    /// plugin harness — see [`plugin_stub`].
    fn asked(&self, profile: &str) -> usize {
        std::fs::read_to_string(self.place(profile).join("asked"))
            .unwrap_or_default()
            .lines()
            .count()
    }

    /// The login an Antigravity CLI account named `name` holds, if any.
    fn agy_login_of(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(
            self.accounts
                .path()
                .join(name)
                .join(".gemini/antigravity-cli/antigravity-oauth-token"),
        )
        .ok()
    }

    /// The directory `profile`'s login is started in, where a stub keeps what
    /// it was asked to do.
    fn place(&self, profile: &str) -> PathBuf {
        self.dir
            .path()
            .join("homes")
            .join(format!("login-{profile}.place"))
    }

    fn pair(&self, name: &str) -> (PathBuf, PathBuf) {
        let home = self.accounts.path().join(name);
        let claude_dir = home.join(".claude");
        let config_file = home.join(".claude.json");

        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(&config_file, "{}\n").unwrap();

        (claude_dir, config_file)
    }

    /// A draft Investigate on a fresh repository, run under `profile`, with
    /// everything settled but whether its account is logged in.
    async fn investigation_under(&self, profile: &str) -> i64 {
        self.investigation_on(profile, "claude-opus-5").await
    }

    /// The same, paired with `model`.
    async fn investigation_on(&self, profile: &str, model: &str) -> i64 {
        let repo = self.accounts.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        for args in [
            &["init", "--initial-branch", "main"][..],
            &["config", "user.email", "test@verkstead.invalid"],
            &["config", "user.name", "Verkstead Test"],
            &["commit", "--allow-empty", "-m", "first"],
        ] {
            let ran = std::process::Command::new("git")
                .args(args)
                .current_dir(&repo)
                .output()
                .unwrap();
            assert!(ran.status.success(), "git {args:?}");
        }

        let (status, said) = self
            .press("/api/ui/repos", serde_json::json!({ "path": repo }))
            .await;
        assert_eq!(status, StatusCode::OK, "{said}");
        let repo_id = store::registered_repos(&self.pool).await.unwrap()[0].id;

        let (status, said) = self
            .press(
                "/api/ui/conversations",
                serde_json::json!({ "repo_id": repo_id }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{said}");
        let started: serde_json::Value = serde_json::from_str(&said).unwrap();
        let id = started["Started"]["id"].as_i64().expect("{said}");

        for (path, body) in [
            (
                "implementation-pairing",
                serde_json::json!({ "profile_id": profile, "model": model }),
            ),
            ("process", serde_json::json!({ "process": "Investigate" })),
            (
                "brief",
                serde_json::json!({ "markdown": "Find out why it is slow." }),
            ),
        ] {
            let (status, said) = self
                .press(&format!("/api/ui/conversations/{id}/{path}"), body)
                .await;
            assert_eq!(status, StatusCode::OK, "{path}: {said}");
        }

        id
    }

    /// Press the start of `conversation`, and hand back what it answered.
    async fn pressed(&self, conversation: i64) -> GrillingStarted {
        let (status, said) = self
            .press(
                &format!("/api/ui/conversations/{conversation}/grill"),
                serde_json::json!({}),
            )
            .await;

        assert_eq!(status, StatusCode::OK, "{said}");
        serde_json::from_str(&said).unwrap()
    }

    /// Why the Profile's row says it cannot be run under, if it does.
    async fn broken(&self, profile: &str) -> Option<Broken> {
        let answered = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/profiles")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = answered.into_body().collect().await.unwrap().to_bytes();
        let rows: Vec<ProfileEntry> = serde_json::from_slice(&bytes).unwrap();

        rows.into_iter()
            .find(|row| row.id == profile)
            .expect("the Profile is on the list")
            .broken
    }

    fn login_of(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(
            self.accounts
                .path()
                .join(name)
                .join(".claude/.credentials.json"),
        )
        .ok()
    }

    /// How many logins the stub has been started as for `profile`.
    fn started(&self, profile: &str) -> usize {
        std::fs::read_to_string(self.place(profile).join("started"))
            .unwrap_or_default()
            .lines()
            .count()
    }

    /// How many processes on this machine are the stub, logging in.
    fn running(&self) -> usize {
        let Ok(processes) = std::fs::read_dir("/proc") else {
            return 0;
        };

        processes
            .flatten()
            .filter(|process| {
                // The shell itself rather than the `bwrap` in front of it,
                // whose own line carries the script as well.
                std::fs::read(process.path().join("cmdline")).is_ok_and(|line| {
                    let words: Vec<_> = line.split(|byte| *byte == 0).collect();

                    words.first() == Some(&&b"/bin/sh"[..])
                        && String::from_utf8_lossy(&line).contains(&self.marker)
                        && (words.contains(&&b"login"[..]) || words.contains(&&b"/help"[..]))
                })
            })
            .count()
    }

    async fn opened(&self, profile: &str, viewer: &str) -> LoginState {
        let (status, said) = self
            .press(
                &format!("/api/ui/profiles/{profile}/login"),
                serde_json::json!({ "viewer": viewer }),
            )
            .await;

        assert_eq!(status, StatusCode::OK, "{said}");
        serde_json::from_str(&said).unwrap()
    }

    async fn closed(&self, profile: &str, viewer: &str) {
        let (status, said) = self
            .press(
                &format!("/api/ui/profiles/{profile}/login/close"),
                serde_json::json!({ "viewer": viewer }),
            )
            .await;

        assert!(status.is_success(), "{said}");
    }

    async fn coded(&self, profile: &str, code: &str) -> (StatusCode, String) {
        self.press(
            &format!("/api/ui/profiles/{profile}/login/code"),
            serde_json::json!({ "code": code }),
        )
        .await
    }

    async fn reading(&self, profile: &str) -> Option<LoginState> {
        let answered = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/ui/profiles/{profile}/login"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(answered.status(), StatusCode::OK);
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();

        serde_json::from_slice(&bytes).unwrap()
    }

    /// The login's reading once `until` holds of it, or a panic saying what it
    /// was instead.
    async fn read_until(
        &self,
        profile: &str,
        until: impl Fn(&Option<LoginState>) -> bool,
    ) -> Option<LoginState> {
        let deadline = Instant::now() + WAITING;

        loop {
            let reading = self.reading(profile).await;

            if until(&reading) {
                return reading;
            }

            assert!(
                Instant::now() < deadline,
                "the login never got there; it reads {reading:?}"
            );

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn waiting(&self, profile: &str) -> String {
        match self
            .read_until(profile, |reading| {
                matches!(reading, Some(LoginState::Waiting { .. }))
            })
            .await
        {
            Some(LoginState::Waiting { url, .. }) => url,
            _ => unreachable!(),
        }
    }

    /// Until nothing of the stub is running, or a panic.
    async fn none_running(&self) {
        let deadline = Instant::now() + WAITING;

        while self.running() > 0 {
            assert!(
                Instant::now() < deadline,
                "the login is still running: {} processes",
                self.running()
            );

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn press(&self, path: &str, body: serde_json::Value) -> (StatusCode, String) {
        let answered = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = answered.status();
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();

        (status, String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// Where the server would be listening. Nothing dials it: a login is told of
/// no server at all.
const LISTENING: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 8422);

#[tokio::test]
async fn a_code_handed_to_the_login_leaves_the_login_in_the_account() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    assert_eq!(
        workbench.opened(&profile, "laptop").await,
        LoginState::Starting
    );

    let url = workbench.waiting(&profile).await;
    assert!(
        url.starts_with("https://claude.com/cai/oauth/authorize?"),
        "{url}"
    );
    assert_eq!(workbench.login_of("work"), None);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    let login = workbench.login_of("work").expect("the account has a login");
    assert!(login.contains(GOOD_CODE), "{login}");

    workbench.closed(&profile, "laptop").await;
    assert_eq!(workbench.reading(&profile).await, None);
}

/// A human takes longer than a few seconds over the page the address opens, and
/// the login is still there when they come back with the code. `bwrap
/// --die-with-parent` kills its child when the *thread* that started it ends,
/// and the runtime's blocking threads end once they have idled for ten
/// seconds — so a login started from one of those died with it.
#[tokio::test]
async fn a_login_outlives_the_thread_its_sandbox_was_built_on() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(&profile, "laptop").await;
    workbench.waiting(&profile).await;

    tokio::time::sleep(Duration::from_secs(15)).await;
    assert!(
        matches!(
            workbench.reading(&profile).await,
            Some(LoginState::Waiting { .. })
        ),
        "{:?}",
        workbench.reading(&profile).await
    );

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;
}

#[tokio::test]
async fn a_refused_code_starts_a_fresh_login_with_a_new_address() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(&profile, "laptop").await;
    let first = workbench.waiting(&profile).await;

    let (status, said) = workbench.coded(&profile, "wrong#code").await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let again = workbench
        .read_until(
            &profile,
            |reading| matches!(reading, Some(LoginState::Waiting { url, .. }) if url != &first),
        )
        .await;

    let Some(LoginState::Waiting { refused, .. }) = again else {
        unreachable!()
    };
    let refused = refused.expect("the modal is told the code was refused");
    assert!(refused.contains("didn't accept"), "{refused}");

    assert_eq!(workbench.started(&profile), 2, "a second login was started");
    assert_eq!(workbench.running(), 1, "and only once the first had gone");
    assert_eq!(workbench.login_of("work"), None);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;
    assert!(workbench.login_of("work").is_some());

    workbench.closed(&profile, "laptop").await;
    workbench.none_running().await;
}

#[tokio::test]
async fn a_code_cut_short_is_asked_for_again_on_the_same_address() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(&profile, "laptop").await;
    let url = workbench.waiting(&profile).await;

    let (status, said) = workbench.coded(&profile, "good").await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let again = workbench
        .read_until(&profile, |reading| {
            matches!(
                reading,
                Some(LoginState::Waiting {
                    refused: Some(_),
                    ..
                })
            )
        })
        .await;

    let Some(LoginState::Waiting { url: same, refused }) = again else {
        unreachable!()
    };
    assert_eq!(same, url);
    let refused = refused.unwrap();
    assert!(refused.contains("whole code"), "{refused}");
    assert_eq!(workbench.started(&profile), 1);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    workbench.closed(&profile, "laptop").await;
}

#[tokio::test]
async fn a_second_device_joins_the_login_already_running() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(&profile, "laptop").await;
    let url = workbench.waiting(&profile).await;

    assert_eq!(
        workbench.opened(&profile, "phone").await,
        LoginState::Waiting {
            url: url.clone(),
            refused: None
        },
        "the phone is shown the address the laptop's login printed"
    );
    assert_eq!(
        workbench.started(&profile),
        1,
        "and no second login was started"
    );
    assert_eq!(workbench.running(), 1);

    // The laptop going leaves the phone's login running.
    workbench.closed(&profile, "laptop").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        workbench.reading(&profile).await,
        Some(LoginState::Waiting { url, refused: None })
    );
    assert_eq!(workbench.running(), 1);

    workbench.closed(&profile, "phone").await;
    workbench.none_running().await;
}

#[tokio::test]
async fn the_last_device_closing_the_modal_kills_the_login() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(&profile, "laptop").await;
    workbench.waiting(&profile).await;
    assert_eq!(workbench.running(), 1);

    workbench.closed(&profile, "laptop").await;

    workbench.none_running().await;
    assert_eq!(workbench.reading(&profile).await, None);
    assert_eq!(workbench.login_of("work"), None);
}

#[tokio::test]
async fn a_login_left_past_its_limit_is_killed() {
    let workbench = Workbench::new(Duration::from_secs(1)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(&profile, "laptop").await;
    workbench.waiting(&profile).await;

    let ended = workbench
        .read_until(&profile, |reading| {
            matches!(reading, Some(LoginState::Failed { .. }))
        })
        .await;

    let Some(LoginState::Failed { reason }) = ended else {
        unreachable!()
    };
    assert!(reason.contains("ran out of time"), "{reason}");
    workbench.none_running().await;

    // And a code arriving after it is refused rather than written to nothing.
    let (status, _) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_profile_bound_on_another_device_is_not_logged_in_here() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let (claude_dir, config_file) = workbench.pair("mirror");

    let id = "4e0f1d2c-0000-4000-8000-000000000004";

    store::profile_arrived(
        &workbench.pool,
        id,
        "0011223344556677889900aabbccddee",
        &store::ProfileFacts {
            name: Some("away".to_owned()),
            account: store::Account::claude(claude_dir, config_file),
            models: vec!["claude-opus-5".to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap();

    let (status, said) = workbench
        .press(
            &format!("/api/ui/profiles/{id}/login"),
            serde_json::json!({ "viewer": "laptop" }),
        )
        .await;

    assert_eq!(status, StatusCode::CONFLICT, "{said}");
    assert!(said.contains("bound on"), "{said}");
    assert_eq!(workbench.running(), 0);
}

#[tokio::test]
async fn a_profile_of_another_harness_is_not_logged_in_here() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let codex = workbench.accounts.path().join("codex/.codex");
    std::fs::create_dir_all(&codex).unwrap();

    let saved = workbench
        .press(
            "/api/ui/profiles",
            serde_json::json!({
                "name": "codex",
                "account": { "agent_type": "Codex", "paths": { "home": codex } },
                "models": ["gpt-5"],
            }),
        )
        .await;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);

    let id = store::profiles(&workbench.pool).await.unwrap()[0]
        .id
        .clone();

    let (status, said) = workbench
        .press(
            &format!("/api/ui/profiles/{id}/login"),
            serde_json::json!({ "viewer": "laptop" }),
        )
        .await;

    assert_eq!(status, StatusCode::CONFLICT, "{said}");
}

/// A press that would start a session under a Claude account with no login is
/// refused naming it, before anything starts; the row says so too; and a login
/// from the card puts both right, so the same press then goes ahead.
#[tokio::test]
async fn a_press_under_a_signed_out_account_is_refused_until_it_logs_in() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;
    let conversation = workbench.investigation_under(&profile).await;

    assert_eq!(
        workbench.broken(&profile).await,
        None,
        "nothing has asked yet, so nothing reads as wrong",
    );

    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::ProfileBroken(ProfileTrouble {
            broken: Broken::SignedOut,
            agent_type: AgentType::CLAUDE,
        }),
    );
    assert_eq!(workbench.broken(&profile).await, Some(Broken::SignedOut));

    let refused = store::load_conversation(&workbench.pool, conversation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        refused.state,
        store::Lifecycle::Draft,
        "nothing was started"
    );
    assert_eq!(refused.worktree, None, "and nothing was checked out");

    workbench.opened(&profile, "laptop").await;
    workbench.waiting(&profile).await;
    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;
    workbench.closed(&profile, "laptop").await;

    assert_eq!(
        workbench.broken(&profile).await,
        None,
        "the login put the row right",
    );
    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::Started
    );
}

/// And a harness that is not on this machine at all is no answer, rather than
/// an account signed out: nothing ran to ask, so the press goes ahead and the
/// row reads as nothing wrong.
#[tokio::test]
async fn a_harness_that_is_not_there_does_not_read_as_signed_out() {
    let workbench = Workbench::harnessed(
        Duration::from_secs(600),
        Standing::Line(vec!["/nonexistent/verkstead-test/claude".to_owned()]),
    )
    .await;
    let profile = workbench.claude_profile("work").await;
    let conversation = workbench.investigation_under(&profile).await;

    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::Started
    );
    assert_eq!(workbench.broken(&profile).await, None);
}

/// An Antigravity CLI Profile logs in the way a Claude one does: the address
/// agy prints, a code handed back, and the token in the account — read off a
/// terminal, since agy offers its address to nothing else.
#[tokio::test]
async fn an_agy_login_leaves_its_token_in_the_account() {
    let workbench = Workbench::agy(Duration::from_secs(600)).await;
    let profile = workbench.agy_profile("work").await;

    workbench.opened(&profile, "laptop").await;

    let url = workbench.waiting(&profile).await;
    assert!(
        url.starts_with("https://accounts.google.com/o/oauth2/auth?"),
        "{url}"
    );
    assert_eq!(workbench.agy_login_of("work"), None);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    let login = workbench
        .agy_login_of("work")
        .expect("the account has a login");
    assert!(login.contains(GOOD_CODE), "{login}");

    workbench.closed(&profile, "laptop").await;
    workbench.none_running().await;
}

/// A code agy refuses is a fresh login with a fresh address, said in its name.
#[tokio::test]
async fn a_code_agy_refuses_starts_a_fresh_login() {
    let workbench = Workbench::agy(Duration::from_secs(600)).await;
    let profile = workbench.agy_profile("work").await;

    workbench.opened(&profile, "laptop").await;
    let first = workbench.waiting(&profile).await;

    let (status, said) = workbench.coded(&profile, "4/0Ab-cut").await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let again = workbench
        .read_until(
            &profile,
            |reading| matches!(reading, Some(LoginState::Waiting { url, .. }) if url != &first),
        )
        .await;

    let Some(LoginState::Waiting { refused, .. }) = again else {
        unreachable!()
    };
    let refused = refused.expect("the modal is told the code was refused");
    assert!(
        refused.starts_with("Antigravity didn't accept"),
        "{refused}"
    );
    assert_eq!(workbench.started(&profile), 2);
    assert_eq!(workbench.agy_login_of("work"), None);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;
    assert!(workbench.agy_login_of("work").is_some());

    workbench.closed(&profile, "laptop").await;
    workbench.none_running().await;
}

/// agy gives up on a code after a minute. While the modal is open that is a
/// fresh address in place of the one that ran out, rather than a failure.
#[tokio::test]
async fn an_agy_address_that_runs_out_is_replaced_by_a_fresh_one() {
    let workbench = Workbench::agy(Duration::from_secs(600)).await;
    let profile = workbench.agy_profile("work").await;

    std::fs::create_dir_all(workbench.place(&profile)).unwrap();
    std::fs::write(workbench.place(&profile).join("expiring"), "").unwrap();

    workbench.opened(&profile, "laptop").await;
    let first = workbench.waiting(&profile).await;

    let again = workbench
        .read_until(
            &profile,
            |reading| matches!(reading, Some(LoginState::Waiting { url, .. }) if url != &first),
        )
        .await;

    let Some(LoginState::Waiting { refused, .. }) = again else {
        unreachable!()
    };
    let refused = refused.expect("the modal is told the address ran out");
    assert!(refused.contains("ran out"), "{refused}");
    assert_eq!(workbench.started(&profile), 2);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    workbench.closed(&profile, "laptop").await;
    workbench.none_running().await;
}

/// A press under an Antigravity CLI account with no login is refused before
/// anything starts, the way a Claude one is, until a login from the card.
#[tokio::test]
async fn a_press_under_a_signed_out_agy_account_is_refused_until_it_logs_in() {
    let workbench = Workbench::agy(Duration::from_secs(600)).await;
    let profile = workbench.agy_profile("work").await;
    let conversation = workbench.investigation_on(&profile, "gemini-pro").await;

    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::ProfileBroken(ProfileTrouble {
            broken: Broken::SignedOut,
            agent_type: AgentType::AGY,
        }),
    );
    assert_eq!(workbench.broken(&profile).await, Some(Broken::SignedOut));

    workbench.opened(&profile, "laptop").await;
    workbench.waiting(&profile).await;
    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;
    workbench.closed(&profile, "laptop").await;

    assert_eq!(workbench.broken(&profile).await, None);
    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::Started
    );
}

/// A harness a plugin declares logs in through its component: the address the
/// `login` export found in what the harness wrote is shown, the code typed in
/// the modal reaches the harness, and the export saying the login is over
/// leaves it in the account.
#[tokio::test]
async fn a_plugin_harness_logs_in_through_its_login_export() {
    let workbench = Workbench::plugin(Duration::from_secs(600)).await;
    let profile = workbench
        .plugin_profile(StubHarness::LoggingIn, "work")
        .await;

    assert_eq!(
        workbench.opened(&profile, "laptop").await,
        LoginState::Starting
    );

    assert_eq!(
        workbench
            .listed(StubHarness::LoggingIn)
            .await
            .login
            .as_deref(),
        Some("Stub Harness"),
        "its list entry offers a Log in, under the name the manifest gives it"
    );

    let url = workbench.waiting(&profile).await;
    assert!(
        url.starts_with("https://stub.example/login?state="),
        "{url}"
    );
    assert_eq!(workbench.plugin_login_of("work"), None);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    let login = workbench
        .plugin_login_of("work")
        .expect("the account has a login");
    assert!(login.contains(GOOD_CODE), "{login}");

    workbench.closed(&profile, "laptop").await;
    workbench.none_running().await;
}

/// A code the harness says is cut short is asked for again on the same
/// address, said as the export words it.
#[tokio::test]
async fn a_plugin_harness_s_cut_code_is_said_as_its_export_words_it() {
    let workbench = Workbench::plugin(Duration::from_secs(600)).await;
    let profile = workbench
        .plugin_profile(StubHarness::LoggingIn, "work")
        .await;

    workbench.opened(&profile, "laptop").await;
    let url = workbench.waiting(&profile).await;

    let (status, said) = workbench.coded(&profile, "good").await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let again = workbench
        .read_until(&profile, |reading| {
            matches!(
                reading,
                Some(LoginState::Waiting {
                    refused: Some(_),
                    ..
                })
            )
        })
        .await;

    assert_eq!(
        again,
        Some(LoginState::Waiting {
            url,
            refused: Some("The stub wants all of the code.".to_owned()),
        })
    );
    assert_eq!(workbench.started(&profile), 1);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    workbench.closed(&profile, "laptop").await;
    workbench.none_running().await;
}

/// A code the harness refuses is a fresh login with a fresh address, said as
/// the export words the refusal.
#[tokio::test]
async fn a_code_a_plugin_harness_refuses_is_said_as_its_export_words_it() {
    let workbench = Workbench::plugin(Duration::from_secs(600)).await;
    let profile = workbench
        .plugin_profile(StubHarness::LoggingIn, "work")
        .await;

    workbench.opened(&profile, "laptop").await;
    let first = workbench.waiting(&profile).await;

    let (status, said) = workbench.coded(&profile, "wrong#code").await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let again = workbench
        .read_until(
            &profile,
            |reading| matches!(reading, Some(LoginState::Waiting { url, .. }) if url != &first),
        )
        .await;

    let Some(LoginState::Waiting { refused, .. }) = again else {
        unreachable!()
    };
    assert_eq!(refused.as_deref(), Some("The stub turned that code down."));
    assert_eq!(workbench.started(&profile), 2);
    assert_eq!(workbench.plugin_login_of("work"), None);

    workbench.closed(&profile, "laptop").await;
    workbench.none_running().await;
}

/// A harness that gives up waiting for a code is a fresh address in its
/// place, said as the export words the giving up.
#[tokio::test]
async fn a_plugin_harness_giving_up_is_said_as_its_export_words_it() {
    let workbench = Workbench::plugin(Duration::from_secs(600)).await;
    let profile = workbench
        .plugin_profile(StubHarness::LoggingIn, "work")
        .await;

    std::fs::create_dir_all(workbench.place(&profile)).unwrap();
    std::fs::write(workbench.place(&profile).join("expiring"), "").unwrap();

    workbench.opened(&profile, "laptop").await;
    let first = workbench.waiting(&profile).await;

    let again = workbench
        .read_until(
            &profile,
            |reading| matches!(reading, Some(LoginState::Waiting { url, .. }) if url != &first),
        )
        .await;

    let Some(LoginState::Waiting { refused, .. }) = again else {
        unreachable!()
    };
    assert_eq!(
        refused.as_deref(),
        Some("The stub stopped waiting for a code.")
    );
    assert_eq!(workbench.started(&profile), 2);

    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    workbench.closed(&profile, "laptop").await;
    workbench.none_running().await;
}

/// A press under a plugin harness's account with no login is refused before
/// anything starts, its status asked of it the way a built-in's is, until a
/// login from the card.
#[tokio::test]
async fn a_press_under_a_signed_out_plugin_account_is_refused_until_it_logs_in() {
    let workbench = Workbench::plugin(Duration::from_secs(600)).await;
    let profile = workbench
        .plugin_profile(StubHarness::LoggingIn, "work")
        .await;
    let conversation = workbench.investigation_on(&profile, PLUGIN_MODELS[0]).await;

    let GrillingStarted::ProfileBroken(trouble) = workbench.pressed(conversation).await else {
        panic!("the press went ahead under an account with no login");
    };
    assert_eq!(trouble.broken, Broken::SignedOut);
    assert_eq!(trouble.agent_type.as_str(), StubHarness::LoggingIn.uuid());
    assert_eq!(workbench.asked(&profile), 1);

    workbench.opened(&profile, "laptop").await;
    workbench.waiting(&profile).await;
    let (status, said) = workbench.coded(&profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    workbench
        .read_until(&profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;
    workbench.closed(&profile, "laptop").await;

    assert_eq!(workbench.broken(&profile).await, None);
    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::Started
    );
}

/// A harness whose manifest declares no login is one Verkstead does not log
/// in: its list entry offers no Log in, a login opened on one of its Profiles
/// is refused, and a press under one goes ahead without asking its status.
#[tokio::test]
async fn a_plugin_harness_declaring_no_login_is_not_logged_in_or_asked() {
    let workbench = Workbench::plugin(Duration::from_secs(600)).await;
    let profile = workbench.plugin_profile(StubHarness::Silent, "work").await;

    assert_eq!(
        workbench.listed(StubHarness::Silent).await.login,
        None,
        "no Log in is offered"
    );

    let (status, said) = workbench
        .press(
            &format!("/api/ui/profiles/{profile}/login"),
            serde_json::json!({ "viewer": "laptop" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{said}");
    assert_eq!(workbench.running(), 0);

    let conversation = workbench.investigation_on(&profile, PLUGIN_MODELS[0]).await;
    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::Started
    );
    assert_eq!(workbench.asked(&profile), 0, "its status was never asked");
    assert_eq!(workbench.broken(&profile).await, None);
}
