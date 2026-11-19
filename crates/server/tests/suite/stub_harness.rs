//! **The stub harness plugin**: a harness that is whatever the suite stands
//! where an agent goes, declared by a `plugin.yaml` written here and run by the
//! component `plugins/tests/stub-harness` builds — the sixth implementation of
//! the server's `Harness` trait, reached the way a third-party harness is, by
//! installing a directory off the Plugins page (ADR-0030, *Agent*).
//!
//! Not a suite: the manifests, the install, and the Profile a suite saves
//! under one, for the suites that launch a session on a plugin harness.

use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::{PluginConsent, PluginsView};

/// The component every manifest here names, built by the server's build script
/// with the built-ins' and embedded nowhere.
pub(crate) const COMPONENT: &str = env!("VERKSTEAD_STUB_HARNESS_WASM");

/// What a drawing stub harness has on its Screen while it works, and nothing of
/// what it has there at rest: its own wording, which is no built-in's, so that
/// a session judged by it is judged by the manifest and nothing else.
pub(crate) const AT_WORK: &str = "▣ stub is working · esc to stop";

/// And what it leaves at rest.
pub(crate) const AT_ITS_PROMPT: &str = "▢ stub is waiting for you";

/// Its mark, which the harness list carries as the plugin ships it.
pub(crate) const MARK: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1 1\"><rect width=\"1\" height=\"1\"/></svg>";

/// What a session of it is told in its environment, beyond what every session
/// is.
pub(crate) const SAYS: (&str, &str) = ("STUB_HARNESS_SAYS", "declared");

/// Which of the stub harnesses: how it idles and how its sessions ask. Each is
/// a plugin of its own, under a uuid of its own, so that suites installing two
/// of them in one process never stand one where the other was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StubHarness {
    /// Idle once it has written nothing for a while, and asking by holding
    /// `verkstead ask` open.
    Silent,

    /// At work while [`AT_WORK`] is on its Screen, and asking by storing the
    /// Set and being nudged.
    Drawing,

    /// At work while [`AT_WORK`] is on its Screen, and asking by holding
    /// `verkstead ask` open.
    DrawingBlocking,

    /// Idle once it has written nothing for a while, asking by holding
    /// `verkstead ask` open, and logged in by Verkstead: `auth login` its
    /// login and `auth status` the question of whether it has one. The only
    /// one declaring a login.
    LoggingIn,

    /// As [`StubHarness::Silent`], under a uuid of its own for the suites that
    /// read back the models it declares: the harness of a uuid is whichever
    /// manifest was installed last in this process, and every other variant is
    /// installed with more than one list.
    Offering,

    /// As [`StubHarness::Silent`], declaring the command that installs its
    /// binary: `/bin/sh` running [`INSTALLER`] from the home of whoever runs the
    /// server, named in full because what the line searches is the machine's
    /// `PATH`.
    /// The only one declaring an install.
    Installing,
}

/// The script an installing stub harness's install command runs, said from
/// the home it runs under.
pub(crate) const INSTALLER: &str = "install-stub-harness.sh";

impl StubHarness {
    /// Its plugin's uuid, which is the harness's id.
    pub(crate) fn uuid(self) -> &'static str {
        match self {
            StubHarness::Silent => "5b0c1a2d-3e4f-4a5b-8c6d-7e8f9a0b1c2d",
            StubHarness::Drawing => "6c1d2b3e-4f5a-4b6c-9d7e-8f9a0b1c2d3e",
            StubHarness::DrawingBlocking => "7d2e3c4f-5a6b-4c7d-8e8f-9a0b1c2d3e4f",
            StubHarness::LoggingIn => "8e3f4d5a-6b7c-4d8e-9f0a-1b2c3d4e5f6a",
            StubHarness::Offering => "9f4a5e6b-7c8d-4e9f-8a1b-2c3d4e5f6a7b",
            StubHarness::Installing => "a05b6f7c-8d9e-4fa0-9b1c-3d4e5f6a7b8c",
        }
    }

    fn idle(self) -> String {
        match self {
            StubHarness::Silent
            | StubHarness::LoggingIn
            | StubHarness::Offering
            | StubHarness::Installing => "    idle: silence\n".to_owned(),
            StubHarness::Drawing | StubHarness::DrawingBlocking => {
                format!("    idle:\n      signature: \"{AT_WORK}\"\n")
            }
        }
    }

    fn channel(self) -> &'static str {
        match self {
            StubHarness::Drawing => "store-and-nudge",
            StubHarness::Silent
            | StubHarness::DrawingBlocking
            | StubHarness::LoggingIn
            | StubHarness::Offering
            | StubHarness::Installing => "blocking",
        }
    }

    fn login(self) -> &'static str {
        match self {
            StubHarness::LoggingIn => {
                "    login:\n      line: [auth, login]\n      status: [auth, status]\n"
            }
            StubHarness::Silent
            | StubHarness::Drawing
            | StubHarness::DrawingBlocking
            | StubHarness::Offering
            | StubHarness::Installing => "",
        }
    }

    fn install(self) -> String {
        match self {
            StubHarness::Installing => format!(
                "    install:\n      instructions: https://example.com/stub-harness\n      \
                 commands: ['/bin/sh \"$HOME/{INSTALLER}\"']\n"
            ),
            StubHarness::Silent
            | StubHarness::Drawing
            | StubHarness::DrawingBlocking
            | StubHarness::LoggingIn
            | StubHarness::Offering => String::new(),
        }
    }

    /// Its `plugin.yaml`, offering `models` with the first the default and
    /// the second the grilling role's.
    pub(crate) fn manifest(self, models: [&str; 2]) -> String {
        let [default, grilling] = models;

        format!(
            r#"uuid: {uuid}
name: Stub harness {variant:?}
author:
  name: Ada Lovelace
  email: ada@example.com
version: 0.1.0
interface: 1
mark: stub.svg
component: stub_harness.wasm
provides:
  - category: agent
    binary: stub-harness
    name: Stub Harness
    launch:
      model: [--model, "{{model}}"]
      prompt: ["{{prompt}}"]
      session: [--session-id, "{{session}}"]
      resume:
        flag: [--resume, "{{session}}"]
      tail: [--yes, --worktree, "{{worktree}}", --settings, "{{settings}}"]
      settings:
        quiet: true
    account:
      - key: dir
        label: Stub directory
        purpose: Mounted at `~/.stub`.
        example: /home/you/accounts/work/.stub
        kind: directory
        mounted: .stub
        allowlist: [settings.json]
        credentials: [auth.json]
        memory: [memory]
      - key: config
        label: Stub config file
        kind: file
        mounted: .stub.json
{idle}    ask:
      channel: {channel}
      guide: |
        Run `verkstead ask`.
    models:
      - id: {default}
        name: Stub default
        default: true
      - id: {grilling}
        name: Stub grilling
        for: [grilling]
    tools: stub__<server>__<tool>
    environment:
      {says}: {said}
    instructions: STUB.md
    excluded: [.stub-work/]
{install}{login}"#,
            uuid = self.uuid(),
            variant = self,
            idle = self.idle(),
            channel = self.channel(),
            install = self.install(),
            login = self.login(),
            says = SAYS.0,
            said = SAYS.1,
        )
    }

    /// The plugin's directory under `under`: the manifest, the mark and the
    /// component, by its path.
    pub(crate) fn written(self, under: &Path, models: [&str; 2]) -> PathBuf {
        let dir = under.join(format!("stub-harness-{}", self.uuid()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.yaml"), self.manifest(models)).unwrap();
        std::fs::write(dir.join("stub.svg"), MARK).unwrap();
        std::fs::copy(COMPONENT, dir.join("stub_harness.wasm")).unwrap();
        dir
    }

    /// Install it on `app` from a directory under `under`, the way the
    /// Plugins section does: the consent card read, and **Install** pressed
    /// granting what it showed.
    pub(crate) async fn installed(self, app: &Router, under: &Path, models: [&str; 2]) {
        let dir = self.written(under, models);
        let source = dir.to_str().unwrap();

        let (status, said) = pressing(
            app,
            "/api/ui/plugins/read",
            serde_json::json!({ "source": source }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{said}");
        let consent: PluginConsent = serde_json::from_str(&said).unwrap();

        let (status, said) = pressing(
            app,
            "/api/ui/plugins",
            serde_json::json!({ "source": source, "granted": consent.capabilities }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{said}");

        let view: PluginsView = serde_json::from_str(&said).unwrap();
        let row = view
            .installed
            .iter()
            .find(|row| row.uuid == self.uuid())
            .expect("the stub harness is listed once installed");
        assert_eq!(row.trouble, None, "the stub harness loads");
    }
}

/// An account of a stub harness under `under`: its directory, holding a
/// `settings.json` that names an MCP server of the account's own, and its
/// config file beside it — by the two keys the manifest declares.
pub(crate) fn account(under: &Path) -> serde_json::Value {
    let dir = under.join(".stub");
    std::fs::create_dir_all(dir.join("memory")).unwrap();
    std::fs::write(
        dir.join("settings.json"),
        r#"{"theme":"dark","mcpServers":{"the-accounts-own":{"url":"http://127.0.0.1:1/own"}}}"#,
    )
    .unwrap();

    let config = under.join(".stub.json");
    std::fs::write(&config, r#"{"beside":true}"#).unwrap();

    serde_json::json!({ "dir": dir, "config": config })
}

async fn pressing(app: &Router, path: &str, body: serde_json::Value) -> (StatusCode, String) {
    let answered = app
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
