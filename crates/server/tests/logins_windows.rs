//! A Profile's login on Windows, run as the session account the way a session
//! is: the address it prints, the code handed back, the login landing in the
//! account through the write-back, and the process killed when it is given up
//! on — proved against a PowerShell script standing where `claude` goes.
//!
//! The Windows arm of `tests/logins.rs`, which says what the script is: what
//! Claude Code 2.1.283 does over plain pipes. What is different here is how it
//! is started — by Verkstead's own logon as the session account, with its input
//! held open for the code (see the server's `sandbox::starting::held_open`) —
//! and that it runs behind a boundary of its own, taken back as it ends.
//!
//! **The script is written into the login's own directory**, which is the
//! directory it is started in. A login is given nothing the installation binds
//! (see the server's `Sandbox::for_login`), so a script anywhere else is a file
//! the session account cannot read; that one directory is the login's to write.
//!
//! **And it needs the session account**, for `tests/sessions_windows.rs`'s
//! reason: making one is an administrator's call, so a machine where
//! `verkstead session-account create` has never been run fails here with the
//! line that names it.

#![cfg(windows)]

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::LoginState;
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::account::Logon;
use verkstead_server::sandbox::account::machine::Account;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::{Agents, Gh, open_database, router_running_sessions, store};

/// How long anything here is waited for before the test says what it saw.
///
/// Longer than the Unix arm's: a login here is a logon and a boundary written
/// before PowerShell has said a word, and a first `powershell.exe` on a runner
/// is slow on its own.
const WAITING: Duration = Duration::from_secs(120);

/// The code the stub takes. Any other is refused, the way Claude Code 2.1.283
/// refuses one that is shaped right but wrong.
const GOOD_CODE: &str = "good#code";

/// What stands where `claude` goes, started in the login's own directory.
const POWERSHELL: [&str; 6] = [
    "powershell.exe",
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    STUB,
];

/// The stub's name in that directory.
const STUB: &str = "agent.ps1";

/// What the stub is. `$args[1]` is `login` or `status`. A login says it
/// started and which process it is, prints the address and the prompt, and
/// takes one line for the code; it writes the login where Claude would, under
/// the HOME the sandbox gives it.
const SCRIPT: &str = r#"
$credentials = Join-Path $env:HOME '.claude\.credentials.json'

switch ($args[1]) {
  'login' {
    Add-Content -Path '.\started' -Value 'started'
    Add-Content -Path '.\pids' -Value $PID
    [Console]::Out.WriteLine('Opening browser to sign in...')
    [Console]::Out.WriteLine("If the browser didn't open, visit: https://claude.com/cai/oauth/authorize?code=true&state=$(Get-Random)")
    [Console]::Out.Write('Paste code here if prompted > ')
    [Console]::Out.Flush()
    $code = [Console]::In.ReadLine()
    if ($code -ne 'good#code') {
      [Console]::Error.WriteLine('Login failed: Request failed with status code 400')
      exit 1
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $credentials) | Out-Null
    Set-Content -Path $credentials -Value "{`"claudeAiOauth`":{`"accessToken`":`"$code`"}}" -NoNewline
    exit 0
  }
  'status' {
    if (Test-Path $credentials) { exit 0 } else { exit 1 }
  }
}
"#;

/// Where every directory this suite makes goes: the machine's temporary
/// directory, spelled the way the filesystem holds it — see
/// `tests/sessions_windows.rs`, whose `SOMEWHERE` says why a session cannot be
/// handed the runner's 8.3 spelling of it.
static SOMEWHERE: LazyLock<PathBuf> = LazyLock::new(|| {
    let temporary = std::env::temp_dir();
    let resolved = std::fs::canonicalize(&temporary).unwrap_or(temporary);
    let spelled = resolved.display().to_string();

    PathBuf::from(spelled.strip_prefix(r"\\?\").unwrap_or(&spelled))
});

fn somewhere() -> tempfile::TempDir {
    tempfile::Builder::new()
        .tempdir_in(&*SOMEWHERE)
        .expect("a directory under the machine's own temporary one")
}

struct Workbench {
    app: Router,
    pool: SqlitePool,
    dir: tempfile::TempDir,
    accounts: tempfile::TempDir,
    _home: tempfile::TempDir,
}

impl Workbench {
    async fn new(limit: Duration) -> Workbench {
        let dir = somewhere();
        let home = somewhere();
        let accounts = somewhere();

        // The password of the account a login runs as, in this Data
        // Directory's own secrets, and its name on the `Homes` below.
        let running_as = the_machines_account();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(
                &settings
                    .secrets()
                    .with_session_account_password(Some(running_as.password().to_owned())),
            )
            .expect("a secrets file to be writable under a temporary Data Directory");

        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let agents = Agents::running(
            POWERSHELL.iter().map(|word| (*word).to_owned()).collect(),
            Homes::on(Platform::HERE, home.path().to_owned(), dir.path())
                .running_sessions_as(running_as.name()),
            Reachable::at(LISTENING),
            SandboxConfig::default(),
            BuildCache::none(),
            Skills::installed(Platform::HERE, dir.path()).expect("this binary carries skills"),
            Some(the_verkstead_binary(dir.path())),
            Handoffs::under(dir.path()),
            Attachments::under(dir.path()),
            Settings::in_data_dir(dir.path()),
        )
        .logging_in_within(limit);

        let app = router_running_sessions(
            pool.clone(),
            dir.path().to_owned(),
            agents,
            Gh::running(vec![
                "cmd.exe".to_owned(),
                "/c".to_owned(),
                "exit 1".to_owned(),
            ]),
        );

        Workbench {
            app,
            pool,
            dir,
            accounts,
            _home: home,
        }
    }

    /// A Claude Profile with no login yet, saved the way the form saves one,
    /// with the stub in the directory its login is started in.
    async fn claude_profile(&self, name: &str) -> i64 {
        let home = self.accounts.path().join(name);
        let claude_dir = home.join(".claude");
        let config_file = home.join(".claude.json");

        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(&config_file, "{}\n").unwrap();

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

        let id = store::profiles(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .find(|profile| profile.name.as_deref() == Some(name))
            .expect("the Profile was saved")
            .id;

        let place = self.place(id);
        std::fs::create_dir_all(&place).unwrap();
        std::fs::write(place.join(STUB), SCRIPT).unwrap();

        id
    }

    /// The directory `profile`'s login is started in.
    fn place(&self, profile: i64) -> PathBuf {
        self.dir
            .path()
            .join("homes")
            .join(format!("login-{profile}.place"))
    }

    fn login_of(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(
            self.accounts
                .path()
                .join(name)
                .join(".claude")
                .join(".credentials.json"),
        )
        .ok()
    }

    /// The processes the stub has been started as for `profile`.
    fn started(&self, profile: i64) -> Vec<u32> {
        std::fs::read_to_string(self.place(profile).join("pids"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    }

    /// Whether the boundary `profile`'s login ran behind is still written down.
    fn boundary_kept(&self, profile: i64) -> bool {
        self.dir
            .path()
            .join("containers")
            .join((-profile).to_string())
            .exists()
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

            tokio::time::sleep(Duration::from_millis(100)).await;
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

    /// Until none of `profile`'s stubs is running, or a panic.
    async fn none_running(&self, profile: i64) {
        let deadline = Instant::now() + WAITING;

        loop {
            let alive: Vec<u32> = self
                .started(profile)
                .into_iter()
                .filter(|pid| running(*pid))
                .collect();

            if alive.is_empty() {
                return;
            }

            assert!(
                Instant::now() < deadline,
                "the login is still running: {alive:?}"
            );

            tokio::time::sleep(Duration::from_millis(100)).await;
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

/// Whether the process `pid` is on this machine, asked of `tasklist`.
fn running(pid: u32) -> bool {
    let listed = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .expect("tasklist answers");

    String::from_utf8_lossy(&listed.stdout).contains(&format!("\"{pid}\""))
}

/// The local account this machine's Verkstead runs its sessions as — see
/// `tests/sessions_windows.rs`, whose own this is.
fn the_machines_account() -> Logon {
    let data_dir =
        platform::data_dir(None).expect("this machine has somewhere for a Data Directory");
    let settings = Settings::in_data_dir(&data_dir);

    match Account::on_this_machine(&data_dir, &settings.secrets()) {
        Ok(account) => Logon::of(account.name(), account.password()),
        Err(missing) => panic!(
            "this suite runs logins as the session account and there is not one: {missing}\n\
             \n\
             The Data Directory it asked about is {}.",
            data_dir.display(),
        ),
    }
}

/// The workspace's own `verkstead.exe`, which a sandbox is equipped with.
fn the_verkstead_binary(data_dir: &Path) -> Executable {
    let built = std::env::current_exe()
        .expect("a test binary knows what it is running")
        .parent()
        .and_then(Path::parent)
        .expect("a test binary is in `deps` under the profile directory")
        .join("verkstead.exe");

    Executable::at(Platform::HERE, built.clone(), data_dir).unwrap_or_else(|| {
        panic!(
            "a login's sandbox is equipped with Verkstead's own binary, and there is no such \
             binary at {}. Build the workspace first: `cargo build --workspace --all-targets`.",
            built.display(),
        )
    })
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

    // And the boundary it ran behind is gone with it.
    assert!(!workbench.boundary_kept(profile));

    workbench.closed(profile, "laptop").await;
    assert_eq!(workbench.reading(profile).await, None);
}

#[tokio::test]
async fn the_last_device_closing_the_modal_kills_the_login() {
    let workbench = Workbench::new(Duration::from_secs(600)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(profile, "laptop").await;
    workbench.waiting(profile).await;
    assert_eq!(workbench.started(profile).len(), 1);

    workbench.closed(profile, "laptop").await;

    workbench.none_running(profile).await;
    assert_eq!(workbench.login_of("work"), None);
}

#[tokio::test]
async fn a_login_left_past_its_limit_is_killed() {
    let workbench = Workbench::new(Duration::from_secs(5)).await;
    let profile = workbench.claude_profile("work").await;

    workbench.opened(profile, "laptop").await;
    workbench.waiting(profile).await;

    let ended = workbench
        .read_until(profile, |reading| {
            matches!(reading, Some(LoginState::Failed { .. }))
        })
        .await;

    match ended {
        Some(LoginState::Failed { reason }) => {
            assert!(reason.contains("ran out of time"), "{reason}");
        }
        _ => unreachable!(),
    }

    workbench.none_running(profile).await;
}
