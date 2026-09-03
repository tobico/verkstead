//! `verkstead desktop`: Verkstead as a human starts it from an icon, and the
//! tray app as a verb of the binary an agent asks with.
//!
//! One suite for one binary, which is what unifying the two of them came to:
//! what the app does before anything else exists — it serves the viewer, it
//! puts the viewer in front of the human unless told not to, it writes its log
//! where a human can find it, it puts right a startup registration naming a
//! Verkstead that has moved, and it refuses an address somebody else is already
//! listening on without having made anything on the way — and beside it the
//! things that are true of the app only because it is a *verb*: that the image
//! serving is the image asking, that what stops the app reaches the log file
//! from a crate whose events the app's own filter had no reason to admit
//! before, and that a startup registration written from inside the verb names
//! the verb.
//!
//! **Half of it is a Unix machine's**, and the half that is says so at each test
//! rather than here. Two things are what divide them. The browser is started
//! through the desktop's own opener, and on a Unix that is a program on the
//! `PATH` a test can put a stand-in in front of, while on Windows it is
//! `powershell.exe` and `explorer.exe` by name — which Windows finds in its own
//! system directory whatever the `PATH` says, so there is nothing to stand in
//! front of and nothing there can watch what would have been opened. **What
//! keeps a browser shut on Windows is `--no-open`**, on every test that runs
//! there, and a test written for that machine without it opens a real browser
//! on whoever is running it. And the dialogs: a Verkstead that could not take
//! its address draws one, which a session with no screen never reaches — the
//! session every test here has on a Unix, and not what a Windows machine with
//! somebody signed into it is.
//!
//! **The registration is the machine's own on Windows**, which is the one thing
//! these cannot keep to themselves there: it is a value in this user's
//! registry, and no variable redirects that the way `HOME` redirects a file. A
//! run of these tests on a machine whose box is ticked would repoint that value
//! at a test binary, so the tests that write one are a Unix machine's too. The
//! registration's own behaviour is unit-tested against a key of the suite's own
//! instead; see `crates/desktop/src/startup/run_key.rs`.

#![cfg(feature = "desktop")]

mod support;

use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
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

/// What the app finds when it goes looking for a browser: on a Unix an
/// `xdg-open` that writes down what it was asked to open and exits, and on
/// Windows a directory with nothing in it at all.
///
/// It is the whole of the `PATH` an app started with one is given either way,
/// and on a Unix that is what makes it work: the opener tries `xdg-open` first,
/// so what is written here is what a browser would have been shown and an empty
/// file is a browser that was never asked.
///
/// **It buys nothing on Windows**, and the directory is there to keep the two
/// arms one shape rather than to stop anything. The Windows opener starts
/// `powershell.exe` and then `explorer.exe`, and Windows finds both in its own
/// system directory before it ever reads the `PATH` — so a stand-in cannot be
/// put in front of them and an empty directory does not keep them from running.
/// What keeps a browser shut there is `--no-open`; see this file's own docs.
struct Opener {
    bin: PathBuf,
    #[cfg(unix)]
    opened: PathBuf,
}

impl Opener {
    #[cfg(unix)]
    fn in_dir(dir: &Path) -> Opener {
        use std::os::unix::fs::PermissionsExt;

        let bin = dir.join("bin");
        let opened = dir.join("opened");
        std::fs::create_dir_all(&bin).unwrap();

        let script = bin.join("xdg-open");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$1\" >> {}\n",
                opened.to_str().unwrap()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        Opener { bin, opened }
    }

    #[cfg(not(unix))]
    fn in_dir(dir: &Path) -> Opener {
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();

        Opener { bin }
    }

    /// What the app asked for, or nothing where it has asked for nothing.
    #[cfg(unix)]
    fn asked_for(&self) -> Option<String> {
        std::fs::read_to_string(&self.opened).ok()
    }

    /// Wait for the app to ask for `url`, which it does as it comes up.
    #[cfg(unix)]
    fn await_asked_for(&self, url: &str) {
        let deadline = Instant::now() + PATIENCE;
        while !self.asked_for().is_some_and(|asked| asked.contains(url)) {
            assert!(
                Instant::now() < deadline,
                "the viewer should have been opened at {url}, and what was opened was {:?}",
                self.asked_for()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// `verkstead desktop` as an icon starts it: a child process with a Data
/// Directory and a home of its own, killed when the test is done with it.
struct App {
    child: Option<Child>,
    url: String,
}

impl App {
    /// Start the verb with `args` in a session whose home is `home`, and block
    /// until it answers.
    ///
    /// `opener` is the stand-in browser, where the test is about what a browser
    /// was or was not handed — and the whole of the `PATH` such a run is given,
    /// so that what is written down there is what a browser would have got and
    /// nothing else could have answered for it. A test that is about something
    /// else gives `None` and keeps the machine's own `PATH`, which is what the
    /// server needs to reach `git`.
    ///
    /// `env` is what a test puts back on top of [`profile`], which is only ever
    /// what it is about.
    fn start(
        port: u16,
        opener: Option<&Opener>,
        home: &Path,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> App {
        let child = command(opener, home, env)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the verkstead binary should be built for its own tests");

        let app = App {
            child: Some(child),
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

    /// Stop the app and hand back everything it said on stderr.
    ///
    /// Read once it has gone rather than while it runs: the pipe is where it
    /// writes, and reading to the end of one is what says there is no more.
    ///
    /// The one test that reads stderr from a running app is a Unix machine's,
    /// so this is too rather than being carried unused.
    #[cfg(unix)]
    fn stop_saying(&mut self) -> String {
        let Some(mut child) = self.child.take() else {
            return String::new();
        };

        let _ = child.kill();
        let said = child
            .wait_with_output()
            .expect("the app should be waited on once it has been stopped");

        String::from_utf8_lossy(&said.stderr).into_owned()
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

/// The binary at its `desktop` verb, with the environment every test gives it
/// and whatever `env` the test itself is about.
fn command(opener: Option<&Opener>, home: &Path, env: &[(&str, &str)]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_verkstead"));
    command
        .arg("desktop")
        .env_remove("RUST_LOG")
        .env_remove("VERKSTEAD_LISTEN")
        .env_remove("VERKSTEAD_DATA_DIR")
        .env_remove("VERKSTEAD_WATCHED_PATHS");
    if let Some(opener) = opener {
        command.env("PATH", &opener.bin);
    }
    profile(&mut command, home);
    for (name, value) in env {
        command.env(name, value);
    }
    command
}

/// The home directory the app is pointed at, said in the variables this
/// platform keeps its parts in — and everything of this machine's that would
/// otherwise be read instead, taken away.
///
/// The point of both halves is the same: a run of these tests reads and writes a
/// directory of the test's own, and a machine with Verkstead configured for real
/// is not what a test reads.
#[cfg(unix)]
fn profile(command: &mut Command, home: &Path) {
    command
        .env("HOME", home)
        // `DISPLAY` and `WAYLAND_DISPLAY` go, which is what says there is no
        // screen here — see `verkstead_desktop::dialog`. A test that left them
        // would be one that popped a dialog up in front of whoever was running
        // it.
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("XDG_DATA_HOME")
        // Where the log file goes is the platform's answer alone — there is no
        // flag for it — so a machine that has one set is a machine whose own
        // state directory these tests would otherwise write into.
        .env_remove("XDG_STATE_HOME")
        // And where the startup registration goes is the platform's answer
        // alone as well, which matters more: a test that left this would be one
        // that wrote into the autostart directory of whoever ran it.
        .env_remove("XDG_CONFIG_HOME")
        // An AppImage says where the file the human has is, and a machine
        // running these tests out of one would otherwise have the registration
        // named for it rather than for the binary the test built.
        .env_remove("APPIMAGE");
}

/// Windows keeps the same answer in three variables rather than one, and every
/// directory Verkstead resolves comes out of them: `%USERPROFILE%` is what
/// `HOME` is here, `%APPDATA%` is where the Data Directory goes and
/// `%LOCALAPPDATA%` is where the Log Directory and the Build Cache do — see
/// `verkstead_server::platform`. So a profile of the test's own is those three,
/// laid out under `home` the way a real one is laid out under a real profile.
///
/// There is no screen to take away, and none is taken: what says whether a
/// Windows session has one is the window station rather than anything in the
/// environment — see `verkstead_desktop::screen`.
#[cfg(windows)]
fn profile(command: &mut Command, home: &Path) {
    command
        .env("USERPROFILE", home)
        .env("APPDATA", home.join("AppData").join("Roaming"))
        .env("LOCALAPPDATA", home.join("AppData").join("Local"));
}

/// The flags a test starts the app with: its own address and its own Data
/// Directory, which is everything that would otherwise be this machine's.
fn flags(port: u16, data_dir: &Path) -> [String; 4] {
    [
        "--listen".into(),
        format!("127.0.0.1:{port}"),
        "--data-dir".into(),
        data_dir.to_str().unwrap().into(),
    ]
}

/// And the same with a Watched Path of the test's own, for the tests that put
/// something inside one.
fn watching(port: u16, data_dir: &Path, watched: &Path) -> [String; 6] {
    let [listen, address, dir, at] = flags(port, data_dir);

    [
        listen,
        address,
        dir,
        at,
        "--watched-path".into(),
        watched.to_str().unwrap().into(),
    ]
}

fn as_args(flags: &[String]) -> Vec<&str> {
    flags.iter().map(String::as_str).collect()
}

/// The whole of what making the tray app a verb was for: the process serving
/// the workbench is the process an agent asks, out of one file. A session handed
/// this image gets a `verkstead` that answers `ask` against the very server that
/// spawned it, which is the skew ADR-0012 was amended to close.
#[test]
fn the_verb_serves_the_server_the_same_binary_asks() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let watched = tmp.path().join("watched");
    let data_dir = tmp.path().join("data");
    std::fs::create_dir_all(&watched).unwrap();
    let port = free_port();

    let flags = watching(port, &data_dir, &watched);
    let mut args = as_args(&flags);
    args.push("--no-open");
    let mut app = App::start(port, None, &home, &args, &[]);

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

/// Started with nothing said about opening, the app serves the viewer and hands
/// it to the browser: the whole of what double-clicking an icon is for, and the
/// one thing `--no-open` is the absence of.
///
/// A Unix machine's, for the browser: the opener there is a program on the
/// `PATH` and the stand-in is what says what was handed over. What a Windows
/// machine has instead is the human who double-clicks the shortcut.
#[cfg(unix)]
#[test]
fn the_app_serves_the_viewer_and_opens_it() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut app = App::start(port, Some(&opener), tmp.path(), &as_args(&flags), &[]);

    opener.await_asked_for(&format!("http://127.0.0.1:{port}/"));

    let health = ureq::get(format!("{}/api/v1/health", app.url))
        .call()
        .unwrap()
        .body_mut()
        .read_to_string()
        .unwrap();
    assert_eq!(health, "ok");

    assert!(
        data_dir.join("verkstead.db").exists(),
        "the app runs the server out of the Data Directory, and there is no \
         database in {}",
        data_dir.display()
    );

    app.stop();
}

/// Told nothing about where its work goes, the app keeps it in the platform's
/// own Data Directory — which is the whole reason an icon and a shell start the
/// same Verkstead. The home directory of the test's own is what makes that a
/// directory of the test's own on every platform; which directory each platform
/// answers with is [`verkstead_server::platform`]'s own unit tests, and where
/// that leaves it under a home is [`platform_data_dir`].
#[test]
fn nothing_said_puts_the_work_in_the_platform_data_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let port = free_port();

    let mut app = App::start(
        port,
        Some(&opener),
        &home,
        &["--listen", &format!("127.0.0.1:{port}"), "--no-open"],
        &[],
    );

    let chosen = platform_data_dir(&home);
    assert!(
        chosen.join("verkstead.db").exists(),
        "the database belongs in the platform directory at {}",
        chosen.display()
    );

    app.stop();
}

/// Where the Data Directory goes under a home that says nothing else about
/// itself, which is what every one of these tests gives the app.
#[cfg(target_os = "linux")]
fn platform_data_dir(home: &Path) -> PathBuf {
    home.join(".local/share/verkstead")
}

#[cfg(target_os = "macos")]
fn platform_data_dir(home: &Path) -> PathBuf {
    home.join("Library/Application Support/Verkstead")
}

#[cfg(windows)]
fn platform_data_dir(home: &Path) -> PathBuf {
    home.join("AppData").join("Roaming").join("Verkstead")
}

/// `--no-open` is about the window appearing and nothing else: the server comes
/// up exactly as it otherwise would, and no browser is asked for.
///
/// A Unix machine's for the second half of that, which is the half a stand-in
/// can see — see [`the_app_serves_the_viewer_and_opens_it`].
#[cfg(unix)]
#[test]
fn no_open_serves_just_the_same_and_opens_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut args = as_args(&flags);
    args.push("--no-open");
    let mut app = App::start(port, Some(&opener), tmp.path(), &args, &[]);

    // The browser is asked for before the server has opened a database, so a
    // health check that has answered is well past the moment an open would have
    // happened at: nothing here is waiting for something not to happen.
    assert_eq!(
        opener.asked_for(),
        None,
        "`--no-open` should have opened nothing"
    );

    app.stop();
}

/// A taken address is the one failure the app draws rather than prints, and it
/// is found before anything has been made: there is a second Verkstead on this
/// machine, and the one that lost is not the one that should have written to its
/// directory.
///
/// A Unix machine's, because of the drawing: the refusal is a dialog wherever
/// there is a screen to put one on, and this test waits for the process to end.
/// These tests have no screen on a Unix and every Windows session somebody is
/// signed into has one, so the same run there is a message box waiting for a
/// dismissal that a test cannot give it. What settles it on Windows is the human
/// at the machine; the words that dialog carries are the desktop crate's own
/// unit test, which needs no machine at all.
#[cfg(unix)]
#[test]
fn a_taken_address_is_refused_by_the_port_it_names_and_makes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let data_dir = tmp.path().join("never-made");
    let port = free_port();

    // Whatever was there first: a second copy of the app, or the daemon a NixOS
    // module starts. All this test needs of it is the socket.
    let _first = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();

    let flags = flags(port, &data_dir);
    let refused = command(Some(&opener), tmp.path(), &[])
        .args(as_args(&flags))
        .output()
        .expect("the verkstead binary should be built for its own tests");

    assert!(
        !refused.status.success(),
        "the app should have given up rather than served, got {:?}",
        refused.status
    );

    let said = stderr(&refused);
    assert!(
        said.contains(&format!("127.0.0.1:{port}")),
        "the refusal should name the address it could not take, got:\n{said}"
    );

    assert!(
        !data_dir.exists(),
        "nothing should have been made: {} is there",
        data_dir.display()
    );
    assert_eq!(
        opener.asked_for(),
        None,
        "a browser should not have been opened on a server that never started"
    );
}

/// A session that names a screen the toolkit cannot open — a stale `DISPLAY`
/// left in a shell profile, an X server that has gone away — is a machine with
/// no tray and every other reason to go on serving. The screen every other test
/// here has is *no* screen, which is the case the app never even asks the
/// toolkit about; this is the other one, where it asks and is refused.
///
/// Linux's, because `DISPLAY` is: the other two platforms name no screen in the
/// environment at all.
#[cfg(target_os = "linux")]
#[test]
fn a_screen_that_is_named_and_is_not_there_serves_without_a_tray() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let flags = flags(port, &data_dir);
    // A display number nothing on this machine is answering on: an X server
    // would be listening on a socket named after it, and none is.
    let mut app = App::start(
        port,
        Some(&opener),
        tmp.path(),
        &as_args(&flags),
        &[("DISPLAY", ":917")],
    );

    opener.await_asked_for(&format!("http://127.0.0.1:{port}/"));

    app.stop();
}

/// Where the log file goes on a machine that says nothing but where its home is,
/// which is each platform's own default and the case every desktop actually has.
#[cfg(target_os = "linux")]
fn log_file(home: &Path) -> PathBuf {
    home.join(".local/state/verkstead/verkstead.log")
}

#[cfg(target_os = "macos")]
fn log_file(home: &Path) -> PathBuf {
    home.join("Library/Logs/Verkstead/verkstead.log")
}

#[cfg(windows)]
fn log_file(home: &Path) -> PathBuf {
    home.join("AppData")
        .join("Local")
        .join("Verkstead")
        .join("verkstead.log")
}

/// A tray app has no stdout anybody will ever read, so the server's log goes to
/// the Log Directory instead — made by the app, because the resolving the server
/// does deliberately makes nothing.
#[test]
fn the_log_file_holds_the_servers_own_startup_line() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut args = as_args(&flags);
    args.push("--no-open");
    let mut app = App::start(port, Some(&opener), &home, &args, &[]);

    let logged = std::fs::read_to_string(log_file(&home)).unwrap_or_else(|why| {
        panic!(
            "the app should have written to {} ({why})",
            log_file(&home).display()
        )
    });

    assert!(
        logged.contains("verkstead is listening"),
        "the log should carry the server's own startup line, got:\n{logged}"
    );

    app.stop();
}

/// And what is written there is filtered the way `verkstead serve`'s stdout is:
/// where the events go is the verb's call, and which of them are worth writing
/// is `RUST_LOG`'s.
#[test]
fn rust_log_filters_the_file_as_it_filters_the_clis_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut args = as_args(&flags);
    args.push("--no-open");
    let mut app = App::start(port, Some(&opener), &home, &args, &[("RUST_LOG", "error")]);

    let logged = std::fs::read_to_string(log_file(&home))
        .expect("the file is opened whatever is filtered out of it");

    assert!(
        !logged.contains("verkstead is listening"),
        "`RUST_LOG=error` should have silenced the startup line, got:\n{logged}"
    );

    app.stop();
}

/// A machine with nowhere to keep a log file has only lost the log, which is
/// nothing like a machine with nowhere to keep a Data Directory: the app says
/// where the logging went instead and goes on serving.
///
/// A Unix machine's, and the reason is in what gets it there: a relative home —
/// see `verkstead_server::platform`, where a directory resolved against wherever
/// the app was launched from is the thing the platform default replaces. Windows
/// reaches the same state by naming no `%LOCALAPPDATA%` at all, which is not a
/// profile anybody has.
#[cfg(unix)]
#[test]
fn nowhere_to_keep_a_log_file_serves_and_says_where_the_log_went() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let data_dir = tmp.path().join("data");
    let cache = tmp.path().join("cache");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut args = as_args(&flags);
    args.push("--no-open");
    // The Build Cache is resolved out of the home as well, and that one *does*
    // refuse startup — so it is told where it goes, leaving the log file as the
    // one thing this machine has nowhere for.
    let mut app = App::start(
        port,
        Some(&opener),
        Path::new("a-relative-home"),
        &args,
        &[("XDG_CACHE_HOME", cache.to_str().unwrap())],
    );

    // Serving, which is the whole point: `App::start` has already waited for it.
    let said = app.stop_saying();

    assert!(
        said.contains("nowhere to keep a log file"),
        "the app should have said why there is no log file, got:\n{said}"
    );
    assert!(
        said.contains("verkstead is listening"),
        "and the logging should have gone to stderr instead, got:\n{said}"
    );
}

/// Everything that goes wrong after the address is taken goes wrong where there
/// is a log file to say so in, and this is what says it lands there: an icon
/// that appeared and vanished is otherwise the whole of what the human was told.
/// A Watched Path that is not there is the shortest of those failures — the
/// server resolves them before it makes anything — and it stands here for the
/// Data Directory that cannot be written and the machine with no `HOME`.
///
/// It is also a thing about the *verb*, its events carrying a target of their
/// own that the app's log filter has to admit: what says the app stopped is
/// written from `crates/cli` now rather than from the crate the filter was
/// written around.
///
/// The dialog beside it is the one part of this no test here can see, wanting a
/// screen these deliberately do not have — which is what makes it a Unix
/// machine's, for the reason
/// [`a_taken_address_is_refused_by_the_port_it_names_and_makes_nothing`] is.
#[cfg(unix)]
#[test]
fn a_startup_that_fails_after_the_address_says_so_in_the_log() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let data_dir = tmp.path().join("data");
    let missing = tmp.path().join("not-a-directory");

    let flags = watching(free_port(), &data_dir, &missing);
    let mut args = as_args(&flags);
    args.push("--no-open");

    let refused = command(Some(&opener), &home, &[])
        .args(&args)
        .output()
        .expect("the verkstead binary should be built for its own tests");

    assert!(
        !refused.status.success(),
        "the app should have given up rather than served, got {:?}",
        refused.status
    );

    let logged = std::fs::read_to_string(log_file(&home)).unwrap_or_else(|why| {
        panic!(
            "the app should have written to {} ({why})",
            log_file(&home).display()
        )
    });

    assert!(
        logged.contains(missing.to_str().unwrap()),
        "the log should carry the failure that stopped the app, got:\n{logged}"
    );
    assert!(
        stderr(&refused).contains(missing.to_str().unwrap()),
        "and so should the standard error, got:\n{}",
        stderr(&refused)
    );
}

#[cfg(unix)]
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Where the startup registration goes on a machine that says nothing but where
/// its home is, which is the platform's own default and the case every desktop
/// actually has: the XDG autostart entry here, and the launch agent on macOS.
/// Windows keeps its own in the registry rather than under a home, which is what
/// leaves the two tests below a Unix machine's — see this file's own docs.
#[cfg(target_os = "linux")]
fn registration(home: &Path) -> PathBuf {
    home.join(".config/autostart/net.tobico.Verkstead.desktop")
}

#[cfg(target_os = "macos")]
fn registration(home: &Path) -> PathBuf {
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

/// The command a registration has to carry, as this platform words one: the
/// executable that is running, the verb that is the app, and the flag that keeps
/// a browser out of the human's way at login.
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

/// A Verkstead that has moved leaves a registration naming where it used to be,
/// and the next launch by hand is where that is put right — the whole of why
/// this is "rewritten every launch" rather than "written once". The launch is an
/// ordinary one in every other way: it serves, and it opens the viewer.
///
/// And what it writes names the verb beside the path: this image has verbs that
/// are not the app — `verkstead ask` is the same file — so a registration naming
/// the path and nothing else would come up printing the Guide at the next login.
///
/// A Unix machine's, because the registration is a file under a home there.
#[cfg(unix)]
#[test]
fn a_launch_rewrites_a_registration_to_name_the_verb_and_the_running_executable() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let data_dir = tmp.path().join("data");
    let port = free_port();

    // Registered already, naming a Verkstead that has since moved: an
    // unregistered machine stays unregistered, so a launch has nothing to
    // rewrite unless somebody had asked for one.
    let entry = registration(&home);
    std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
    std::fs::write(&entry, naming_somewhere_else()).unwrap();

    let flags = flags(port, &data_dir);
    let mut app = App::start(port, Some(&opener), &home, &as_args(&flags), &[]);

    opener.await_asked_for(&format!("http://127.0.0.1:{port}/"));

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

/// And a machine nobody asked to be started on is left alone: a launch rewrites
/// what somebody asked for rather than deciding it for them, and the box is the
/// only thing that ever registers Verkstead.
#[cfg(unix)]
#[test]
fn a_launch_registers_nothing_nobody_asked_for() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut args = as_args(&flags);
    args.push("--no-open");
    let mut app = App::start(port, Some(&opener), &home, &args, &[]);

    assert!(
        !registration(&home).exists(),
        "{} should not be there",
        registration(&home).display()
    );

    app.stop();
}

/// The one flag of the app's own, beside the server's own — which is how a
/// developer runs it out of a checkout.
#[test]
fn the_verbs_help_names_the_flag_and_the_server_options_it_sits_beside() {
    let help = run(&["desktop", "--help"]);
    let help = String::from_utf8(help.stdout).unwrap();

    for phrase in ["--no-open", "--data-dir", "--listen", "VERKSTEAD_DATA_DIR"] {
        assert!(
            help.contains(phrase),
            "`verkstead desktop --help` should mention {phrase:?}, got:\n{help}"
        );
    }
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
