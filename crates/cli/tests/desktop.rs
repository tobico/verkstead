//! `verkstead desktop`: the tray app as a verb of the binary an agent asks
//! with.
//!
//! What is judged here is the seam rather than the app. The app itself is
//! `crates/desktop`'s own suite, which runs it end to end; these are the things
//! that are true of it only because it is a verb of *this* binary — that the
//! verb serves and opens the viewer at all, that the image serving is the image
//! asking, that what stops the app reaches the log file from a crate whose
//! events the app's own filter had no reason to admit before, and that a startup
//! registration written from inside the verb names the verb.
//!
//! **The browser is kept shut with `--no-open`** on every test but the one that
//! is about opening it, which puts a stand-in `xdg-open` in front of it — see
//! [`Opener`]. A test of anything else written without the flag opens a real
//! browser on whoever is running it.

#![cfg(feature = "desktop")]

mod support;

use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use support::repo_with_a_commit;
use verkstead_schema::Response;

/// How long a test waits on the app it just started before calling it dead.
const PATIENCE: Duration = Duration::from_secs(30);

/// A Set small enough to be answered by the Response below.
const SET: &str = "
title: Does the desktop verb serve the binary that asks?
questions:
  - label: Q1
    text: Did the Response come back?
    options:
      - n: 1
        text: It did
        recommended: true
";

const ANSWER: &str = "
answers:
  - label: Q1
    selected: 1
";

/// A port nothing is listening on. Bound and released, so the app that takes it
/// is the next thing to claim it — the alternative is a test fighting whatever
/// is already on the default port.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// `verkstead desktop` as an icon starts it: a child process with a Data
/// Directory and a home of its own, killed when the test is done with it.
struct App {
    child: Option<Child>,
    url: String,
}

impl App {
    /// Start the verb with `args` and the browser shut, in a session whose home
    /// is `home`, and block until it answers.
    fn start(port: u16, home: &Path, args: &[&str]) -> App {
        App::starting(port, home, args, &["--no-open"], None)
    }

    /// And the same with the browser left to open, watched by `opener` — whose
    /// directory is the whole of the `PATH` such a run is given, so that what
    /// is written down there is what a browser would have been handed and
    /// nothing else could have answered for it. See [`Opener`].
    #[cfg(unix)]
    fn opening(port: u16, home: &Path, args: &[&str], opener: &Opener) -> App {
        App::starting(port, home, args, &[], Some(&opener.bin))
    }

    fn starting(port: u16, home: &Path, args: &[&str], shut: &[&str], path: Option<&Path>) -> App {
        let mut command = Command::new(env!("CARGO_BIN_EXE_verkstead"));
        command
            .arg("desktop")
            .args(shut)
            .args(args)
            .env_remove("RUST_LOG")
            .env_remove("VERKSTEAD_LISTEN")
            .env_remove("VERKSTEAD_DATA_DIR")
            .env_remove("VERKSTEAD_WATCHED_PATHS")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        profile(&mut command, home);
        if let Some(path) = path {
            command.env("PATH", path);
        }

        let app = App {
            child: Some(
                command
                    .spawn()
                    .expect("the verkstead binary should be built for its own tests"),
            ),
            url: format!("http://127.0.0.1:{port}"),
        };
        app.await_health();
        app
    }

    fn await_health(&self) {
        let deadline = Instant::now() + PATIENCE;
        while ureq::get(format!("{}/api/v1/health", self.url))
            .call()
            .is_err()
        {
            assert!(
                Instant::now() < deadline,
                "`verkstead desktop` never answered on {}",
                self.url
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// A Conversation to ask from, made the way the workbench makes one: a Repo
    /// registered from inside the Watched Path, and a Conversation against it.
    fn asking_from(&self, repo: &Path) -> i64 {
        let registered: serde_json::Value = self.through_the_viewer(
            "/api/ui/repos",
            &serde_json::json!({ "path": repo.to_str().unwrap() }),
        );
        assert_eq!(registered, serde_json::json!("Added"));

        let listed: serde_json::Value = serde_json::from_str(&self.read("/api/ui/repos")).unwrap();
        let repo_id = listed[0]["id"]
            .as_i64()
            .expect("the Repo was just registered");

        let started: serde_json::Value = self.through_the_viewer(
            "/api/ui/conversations",
            &serde_json::json!({ "repo_id": repo_id }),
        );

        started["Started"]["id"]
            .as_i64()
            .unwrap_or_else(|| panic!("the Conversation should have started: {started}"))
    }

    /// Tell the viewer's namespace something, in the JSON a browser would send.
    fn through_the_viewer(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        let mut reply = ureq::post(format!("{}{path}", self.url))
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .unwrap_or_else(|error| panic!("POST {path}: {error}"));

        assert_eq!(reply.status().as_u16(), 200);

        serde_json::from_str(&reply.body_mut().read_to_string().unwrap()).unwrap()
    }

    fn read(&self, path: &str) -> String {
        ureq::get(format!("{}{path}", self.url))
            .call()
            .unwrap_or_else(|error| panic!("GET {path}: {error}"))
            .body_mut()
            .read_to_string()
            .unwrap()
    }

    /// What a session on `conversation` is given as `VERKSTEAD_SERVER`.
    fn asking_url(&self, conversation: i64) -> String {
        format!("{}/conversations/{conversation}", self.url)
    }

    /// Answer Set `id` the way the human's device does, retrying until the Set
    /// the CLI is submitting has landed.
    fn await_answer(&self, conversation: i64, id: i64, yaml: &str) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let submitted = ureq::post(format!(
                "{}/api/v1/sets/{id}/response",
                self.asking_url(conversation)
            ))
            .header("Content-Type", "application/yaml")
            .send(yaml)
            .is_ok_and(|reply| reply.status().as_u16() == 201);
            if submitted {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the CLI never submitted Question Set {id}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.stop();
    }
}

/// What the app finds when it goes looking for a browser: an `xdg-open` that
/// writes down what it was asked to open and exits.
///
/// A Unix machine's, and the reason the app's own suite gives: the opener there
/// is a program on the `PATH` a test can put a stand-in in front of, while on
/// Windows it is `powershell.exe` and `explorer.exe` by name, which Windows
/// finds in its own system directory whatever the `PATH` says.
#[cfg(unix)]
struct Opener {
    bin: std::path::PathBuf,
    opened: std::path::PathBuf,
}

#[cfg(unix)]
impl Opener {
    fn in_dir(dir: &Path) -> Opener {
        use std::os::unix::fs::PermissionsExt;

        let bin = dir.join("bin");
        let opened = dir.join("opened");
        std::fs::create_dir_all(&bin).unwrap();

        let stand_in = bin.join("xdg-open");
        std::fs::write(
            &stand_in,
            format!("#!/bin/sh\nprintf '%s' \"$1\" > {}\n", opened.display()),
        )
        .unwrap();
        std::fs::set_permissions(&stand_in, std::fs::Permissions::from_mode(0o755)).unwrap();

        Opener { bin, opened }
    }

    /// What the browser was handed, once it has been: the opening is a fork of
    /// its own, so it lands a moment after the server answers.
    fn await_asked_for(&self, url: &str) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if std::fs::read_to_string(&self.opened).is_ok_and(|opened| opened == url) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the browser should have been handed {url}, got {:?}",
                std::fs::read_to_string(&self.opened).ok()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// The home directory the app is pointed at, said in the variables this
/// platform keeps its parts in — and everything of this machine's that would
/// otherwise be read instead, taken away.
///
/// The same profile `crates/desktop`'s own suite gives the app, and for the
/// same reason: a run of these tests reads and writes a directory of the test's
/// own, and a machine with Verkstead configured for real is not what a test
/// reads. The registration is the half that matters most — a test that left
/// `XDG_CONFIG_HOME` alone would be one that wrote into the autostart directory
/// of whoever ran it.
#[cfg(unix)]
fn profile(command: &mut Command, home: &Path) {
    command
        .env("HOME", home)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("XDG_DATA_HOME")
        .env_remove("XDG_STATE_HOME")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("APPIMAGE");
}

#[cfg(windows)]
fn profile(command: &mut Command, home: &Path) {
    command
        .env("USERPROFILE", home)
        .env("APPDATA", home.join("AppData").join("Roaming"))
        .env("LOCALAPPDATA", home.join("AppData").join("Local"));
}

/// The flags a test starts the app with: its own address, its own Data
/// Directory and its own Watched Path, which is everything that would otherwise
/// be this machine's.
fn flags(port: u16, data_dir: &Path, watched: &Path) -> [String; 6] {
    [
        "--listen".into(),
        format!("127.0.0.1:{port}"),
        "--data-dir".into(),
        data_dir.to_str().unwrap().into(),
        "--watched-path".into(),
        watched.to_str().unwrap().into(),
    ]
}

fn as_args(flags: &[String; 6]) -> Vec<&str> {
    flags.iter().map(String::as_str).collect()
}

/// The whole of what making the tray app a verb was for: the process serving
/// the workbench is the process an agent asks, out of one file. A session
/// handed this image gets a `verkstead` that answers `ask` against the very
/// server that spawned it, which is the skew ADR-0012 was amended to close.
#[test]
fn the_verb_serves_the_server_the_same_binary_asks() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let watched = tmp.path().join("watched");
    let data_dir = tmp.path().join("data");
    std::fs::create_dir_all(&watched).unwrap();
    let port = free_port();

    let flags = flags(port, &data_dir, &watched);
    let mut app = App::start(port, &home, &as_args(&flags));

    let conversation = app.asking_from(&repo_with_a_commit(&watched));

    let mut asking = Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .arg("ask")
        .env("VERKSTEAD_SERVER", app.asking_url(conversation))
        .current_dir(&watched)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the verkstead binary should be built for its own tests");
    asking
        .stdin
        .take()
        .unwrap()
        .write_all(SET.as_bytes())
        .unwrap();

    app.await_answer(conversation, 1, ANSWER);

    let asked = asking.wait_with_output().unwrap();
    assert!(
        asked.status.success(),
        "the ask should have been answered, got {:?}\n{}",
        asked.status,
        String::from_utf8_lossy(&asked.stderr)
    );

    let response: Response = serde_saphyr::from_str(&String::from_utf8(asked.stdout).unwrap())
        .expect("the ask should have printed a Response");
    assert_eq!(response.answers.len(), 1);

    app.stop();
}

/// Started through the verb with nothing said about opening, the app serves the
/// viewer and hands it to the browser — the whole of what double-clicking an
/// icon is for, and the one thing `--no-open` is the absence of. Every other
/// test here says it the other way round.
///
/// A Unix machine's, for the browser — see [`Opener`].
#[cfg(unix)]
#[test]
fn the_verb_opens_the_viewer_it_is_serving() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let watched = tmp.path().join("watched");
    let data_dir = tmp.path().join("data");
    std::fs::create_dir_all(&watched).unwrap();
    let opener = Opener::in_dir(tmp.path());
    let port = free_port();

    let flags = flags(port, &data_dir, &watched);
    let mut app = App::opening(port, &home, &as_args(&flags), &opener);

    opener.await_asked_for(&format!("http://127.0.0.1:{port}/"));

    app.stop();
}

/// Launch on Startup registers the invocation that is running rather than the
/// executable alone: this image has verbs that are not the app — `verkstead
/// ask` is the same file — so a registration naming the path and nothing else
/// would come up printing the Guide at the next login.
///
/// A Unix machine's, because the registration is a file under a home there. It
/// is a value in this user's own registry on Windows, which no variable
/// redirects, so a test of it would repoint whatever the human running it had
/// registered.
#[cfg(unix)]
#[test]
fn a_launch_through_the_verb_registers_the_verb_beside_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let watched = tmp.path().join("watched");
    let data_dir = tmp.path().join("data");
    std::fs::create_dir_all(&watched).unwrap();
    let port = free_port();

    // Registered already, naming a Verkstead that has since moved: an
    // unregistered machine stays unregistered, so a launch has nothing to
    // rewrite unless somebody had asked for one.
    let entry = registration(&home);
    std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
    std::fs::write(&entry, naming_somewhere_else()).unwrap();

    let flags = flags(port, &data_dir, &watched);
    let mut app = App::start(port, &home, &as_args(&flags));

    let registered = std::fs::read_to_string(&entry).unwrap();
    assert!(
        registered.contains(&through_the_verb(env!("CARGO_BIN_EXE_verkstead"))),
        "the registration should start the running executable through its verb, \
         got:\n{registered}"
    );
    assert!(
        !registered.contains("/somewhere/it/used/to/be/"),
        "and not the one it was written for, got:\n{registered}"
    );

    app.stop();
}

/// The command a registration has to carry, as this platform words one: the
/// executable that is running, the verb that is the app, and the flag that
/// keeps a browser out of the human's way at login.
#[cfg(target_os = "linux")]
fn through_the_verb(exe: &str) -> String {
    format!("\"{exe}\" desktop --no-open")
}

#[cfg(target_os = "macos")]
fn through_the_verb(exe: &str) -> String {
    format!(
        "\t\t<string>{exe}</string>\n\
         \t\t<string>desktop</string>\n\
         \t\t<string>--no-open</string>\n"
    )
}

/// Where the startup registration goes on a machine that says nothing but where
/// its home is, which is the platform's own default and the case every desktop
/// actually has.
#[cfg(target_os = "linux")]
fn registration(home: &Path) -> std::path::PathBuf {
    home.join(".config/autostart/net.tobico.Verkstead.desktop")
}

#[cfg(target_os = "macos")]
fn registration(home: &Path) -> std::path::PathBuf {
    home.join("Library/LaunchAgents/net.tobico.Verkstead.plist")
}

/// A registration naming a Verkstead that is not there, written the way this
/// platform keeps one — what a binary somebody has since moved left behind.
#[cfg(target_os = "linux")]
fn naming_somewhere_else() -> &'static str {
    "[Desktop Entry]\nType=Application\nName=Verkstead\n\
     Exec=\"/somewhere/it/used/to/be/verkstead\" desktop --no-open\n"
}

#[cfg(target_os = "macos")]
fn naming_somewhere_else() -> &'static str {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
     <plist version=\"1.0\">\n<dict>\n\
     \t<key>Label</key>\n\t<string>net.tobico.Verkstead</string>\n\
     \t<key>ProgramArguments</key>\n\t<array>\n\
     \t\t<string>/somewhere/it/used/to/be/verkstead</string>\n\
     \t\t<string>desktop</string>\n\
     \t\t<string>--no-open</string>\n\t</array>\n\
     \t<key>RunAtLoad</key>\n\t<true/>\n</dict>\n</plist>\n"
}

/// A verb of a binary a shell starts still has no terminal anybody is watching
/// when an icon started it, so what stopped the app is written to the log file
/// as well — which is a thing about the *verb*, its events carrying a target of
/// their own that the app's log filter has to admit.
///
/// A Watched Path that is not there is the shortest of the failures that happen
/// after the address is taken, and it stands here for the Data Directory that
/// cannot be written and the machine with no `HOME`. The dialog beside it is the
/// part no test here can see, wanting a screen these deliberately do not have —
/// which is what makes this a Unix machine's, a Windows session having one.
#[cfg(unix)]
#[test]
fn a_startup_that_fails_after_the_address_says_so_in_the_log() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let data_dir = tmp.path().join("data");
    let missing = tmp.path().join("not-a-directory");

    let flags = flags(free_port(), &data_dir, &missing);
    let mut command = Command::new(env!("CARGO_BIN_EXE_verkstead"));
    command
        .arg("desktop")
        .arg("--no-open")
        .args(as_args(&flags))
        .env_remove("RUST_LOG")
        .env_remove("VERKSTEAD_LISTEN")
        .env_remove("VERKSTEAD_DATA_DIR")
        .env_remove("VERKSTEAD_WATCHED_PATHS");
    profile(&mut command, &home);

    let refused = command
        .output()
        .expect("the verkstead binary should be built for its own tests");

    assert!(
        !refused.status.success(),
        "the app should have given up rather than served, got {:?}",
        refused.status
    );

    let log = log_file(&home);
    let logged = std::fs::read_to_string(&log)
        .unwrap_or_else(|why| panic!("the app should have written to {} ({why})", log.display()));

    assert!(
        logged.contains(missing.to_str().unwrap()),
        "the log should carry the failure that stopped the app, got:\n{logged}"
    );
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains(missing.to_str().unwrap()),
        "and so should the standard error, got:\n{}",
        String::from_utf8_lossy(&refused.stderr)
    );
}

/// Where the log file goes on a machine that says nothing but where its home
/// is, which is each platform's own default and the case every desktop has.
#[cfg(target_os = "linux")]
fn log_file(home: &Path) -> std::path::PathBuf {
    home.join(".local/state/verkstead/verkstead.log")
}

#[cfg(target_os = "macos")]
fn log_file(home: &Path) -> std::path::PathBuf {
    home.join("Library/Logs/Verkstead/verkstead.log")
}

/// One binary with every verb on it, which is what the feature being on means:
/// the tray app stands beside the three an agent runs and the one an operator
/// does, rather than in a file of its own.
#[test]
fn the_help_names_the_desktop_verb_beside_the_others() {
    let help = run(&["--help"]);
    let help = String::from_utf8(help.stdout).unwrap();

    for verb in ["ask", "answers", "serve", "desktop", "guide"] {
        assert!(
            help.contains(verb),
            "`verkstead --help` should name {verb:?}, got:\n{help}"
        );
    }
}

/// And the verb changes nothing about what the binary is when nobody says one:
/// an agent that runs it to see what it is still gets the Guide.
#[test]
fn a_build_with_the_verb_still_prints_the_guide_when_nothing_is_said() {
    let printed = run(&[]);

    assert!(printed.status.success(), "got {:?}", printed.status);
    assert!(
        String::from_utf8(printed.stdout).unwrap().contains("Guide"),
        "bare `verkstead` should print the Guide"
    );
}

/// Run the binary to completion with `args` and hand back what it wrote.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .args(args)
        .output()
        .expect("the verkstead binary should be built for its own tests")
}
