//! A Profile's login, run by Verkstead in the Profile's own sandbox: the
//! address it prints, the code handed back, and the login landing in the
//! account — proved against a shell script standing where `claude` goes.
//!
//! The script is what Claude Code 2.1.283 does over plain pipes: a line with
//! the address on it, a prompt with no line ending, and a code read off
//! standard input. It writes the login where Claude would, under the HOME the
//! sandbox gives it, so what reaches the account is whatever the write-back a
//! session's ending does carries there.

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
    AgentType, Broken, GrillingStarted, LoginState, ProfileEntry, ProfileTrouble,
};
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::{Agents, Gh, open_database, router_running_sessions, store};

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
        Workbench::harnessed(limit, None).await
    }

    /// The same, with `harness` standing where every type's binary goes in
    /// place of the stub.
    async fn harnessed(limit: Duration, harness: Option<Vec<String>>) -> Workbench {
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
            harness.unwrap_or_else(|| vec!["/bin/sh".to_owned(), "-c".to_owned(), stub(&marker)]),
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
            "git_author:\n  name: Verkstead Test\n  email: test@verkstead.invalid\n",
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
    async fn claude_profile(&self, name: &str) -> i64 {
        let (claude_dir, config_file) = self.pair(name);

        let saved = self
            .press(
                "/api/ui/profiles",
                serde_json::json!({
                    "name": name,
                    "account": {
                        "agent_type": "Claude",
                        "claude_dir": claude_dir,
                        "config_file": config_file,
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
    async fn investigation_under(&self, profile: i64) -> i64 {
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
                serde_json::json!({ "profile_id": profile, "model": "claude-opus-5" }),
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
    async fn broken(&self, profile: i64) -> Option<Broken> {
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
    fn started(&self, profile: i64) -> usize {
        std::fs::read_to_string(
            self.dir
                .path()
                .join("homes")
                .join(format!("login-{profile}.place"))
                .join("started"),
        )
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
                        && words.contains(&&b"login"[..])
                })
            })
            .count()
    }

    async fn opened(&self, profile: i64, viewer: &str) -> LoginState {
        let (status, said) = self
            .press(
                &format!("/api/ui/profiles/{profile}/login"),
                serde_json::json!({ "viewer": viewer }),
            )
            .await;

        assert_eq!(status, StatusCode::OK, "{said}");
        serde_json::from_str(&said).unwrap()
    }

    async fn closed(&self, profile: i64, viewer: &str) {
        let (status, said) = self
            .press(
                &format!("/api/ui/profiles/{profile}/login/close"),
                serde_json::json!({ "viewer": viewer }),
            )
            .await;

        assert!(status.is_success(), "{said}");
    }

    async fn coded(&self, profile: i64, code: &str) -> (StatusCode, String) {
        self.press(
            &format!("/api/ui/profiles/{profile}/login/code"),
            serde_json::json!({ "code": code }),
        )
        .await
    }

    async fn reading(&self, profile: i64) -> Option<LoginState> {
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
        profile: i64,
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

    async fn waiting(&self, profile: i64) -> String {
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
        workbench.opened(profile, "laptop").await,
        LoginState::Starting
    );

    let url = workbench.waiting(profile).await;
    assert!(
        url.starts_with("https://claude.com/cai/oauth/authorize?"),
        "{url}"
    );
    assert_eq!(workbench.login_of("work"), None);

    let (status, said) = workbench.coded(profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    let login = workbench.login_of("work").expect("the account has a login");
    assert!(login.contains(GOOD_CODE), "{login}");

    workbench.closed(profile, "laptop").await;
    assert_eq!(workbench.reading(profile).await, None);
}

#[tokio::test]
async fn a_refused_code_starts_a_fresh_login_with_a_new_address() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(profile, "laptop").await;
    let first = workbench.waiting(profile).await;

    let (status, said) = workbench.coded(profile, "wrong#code").await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let again = workbench
        .read_until(
            profile,
            |reading| matches!(reading, Some(LoginState::Waiting { url, .. }) if url != &first),
        )
        .await;

    let Some(LoginState::Waiting { refused, .. }) = again else {
        unreachable!()
    };
    let refused = refused.expect("the modal is told the code was refused");
    assert!(refused.contains("did not accept"), "{refused}");

    assert_eq!(workbench.started(profile), 2, "a second login was started");
    assert_eq!(workbench.running(), 1, "and only once the first had gone");
    assert_eq!(workbench.login_of("work"), None);

    let (status, said) = workbench.coded(profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;
    assert!(workbench.login_of("work").is_some());

    workbench.closed(profile, "laptop").await;
    workbench.none_running().await;
}

#[tokio::test]
async fn a_code_cut_short_is_asked_for_again_on_the_same_address() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(profile, "laptop").await;
    let url = workbench.waiting(profile).await;

    let (status, said) = workbench.coded(profile, "good").await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let again = workbench
        .read_until(profile, |reading| {
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
    assert_eq!(workbench.started(profile), 1);

    let (status, said) = workbench.coded(profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");

    workbench
        .read_until(profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;

    workbench.closed(profile, "laptop").await;
}

#[tokio::test]
async fn a_second_device_joins_the_login_already_running() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(profile, "laptop").await;
    let url = workbench.waiting(profile).await;

    assert_eq!(
        workbench.opened(profile, "phone").await,
        LoginState::Waiting {
            url: url.clone(),
            refused: None
        },
        "the phone is shown the address the laptop's login printed"
    );
    assert_eq!(
        workbench.started(profile),
        1,
        "and no second login was started"
    );
    assert_eq!(workbench.running(), 1);

    // The laptop going leaves the phone's login running.
    workbench.closed(profile, "laptop").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        workbench.reading(profile).await,
        Some(LoginState::Waiting { url, refused: None })
    );
    assert_eq!(workbench.running(), 1);

    workbench.closed(profile, "phone").await;
    workbench.none_running().await;
}

#[tokio::test]
async fn the_last_device_closing_the_modal_kills_the_login() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(profile, "laptop").await;
    workbench.waiting(profile).await;
    assert_eq!(workbench.running(), 1);

    workbench.closed(profile, "laptop").await;

    workbench.none_running().await;
    assert_eq!(workbench.reading(profile).await, None);
    assert_eq!(workbench.login_of("work"), None);
}

#[tokio::test]
async fn a_login_left_past_its_limit_is_killed() {
    let workbench = Workbench::new(Duration::from_secs(1)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(profile, "laptop").await;
    workbench.waiting(profile).await;

    let ended = workbench
        .read_until(profile, |reading| {
            matches!(reading, Some(LoginState::Failed { .. }))
        })
        .await;

    let Some(LoginState::Failed { reason }) = ended else {
        unreachable!()
    };
    assert!(reason.contains("ran out of time"), "{reason}");
    workbench.none_running().await;

    // And a code arriving after it is refused rather than written to nothing.
    let (status, _) = workbench.coded(profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_mirrored_profile_is_not_logged_in_here() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let (claude_dir, config_file) = workbench.pair("mirror");

    let id = store::record_mirror(
        &workbench.pool,
        &store::Mirror {
            device: "0011223344556677889900aabbccddee".to_owned(),
            id: 4,
            login: true,
        },
        &store::ProfileFacts {
            name: Some("away".to_owned()),
            account: store::Account::Claude {
                claude_dir,
                config_file,
            },
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
    assert!(said.contains("at home on"), "{said}");
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
                "account": { "agent_type": "Codex", "home": codex },
                "models": ["gpt-5"],
            }),
        )
        .await;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);

    let id = store::profiles(&workbench.pool).await.unwrap()[0].id;

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
    let conversation = workbench.investigation_under(profile).await;

    assert_eq!(
        workbench.broken(profile).await,
        None,
        "nothing has asked yet, so nothing reads as wrong",
    );

    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::ProfileBroken(ProfileTrouble {
            broken: Broken::SignedOut,
            agent_type: AgentType::Claude,
            device: None,
        }),
    );
    assert_eq!(workbench.broken(profile).await, Some(Broken::SignedOut));

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

    workbench.opened(profile, "laptop").await;
    workbench.waiting(profile).await;
    let (status, said) = workbench.coded(profile, GOOD_CODE).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    workbench
        .read_until(profile, |reading| reading == &Some(LoginState::LoggedIn))
        .await;
    workbench.closed(profile, "laptop").await;

    assert_eq!(
        workbench.broken(profile).await,
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
        Some(vec!["/nonexistent/verkstead-test/claude".to_owned()]),
    )
    .await;
    let profile = workbench.claude_profile("work").await;
    let conversation = workbench.investigation_under(profile).await;

    assert_eq!(
        workbench.pressed(conversation).await,
        GrillingStarted::Started
    );
    assert_eq!(workbench.broken(profile).await, None);
}
