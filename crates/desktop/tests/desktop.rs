//! `verkstead-desktop` as a human starts it: the binary itself, run with a
//! stand-in for the browser on its `PATH` and a Data Directory of the test's
//! own.
//!
//! What is judged here is what the app does before anything else exists — it
//! serves the viewer, it puts the viewer in front of the human unless told not
//! to, it writes its log where a human can find it, it puts right a startup
//! registration naming a binary that has moved, and it refuses an address
//! somebody else is already listening on without having made anything on the
//! way.

use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

/// How long a test waits on the app it just started before calling it dead.
const PATIENCE: Duration = Duration::from_secs(30);

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

/// What the app finds when it goes looking for a browser: an `xdg-open` that
/// writes down what it was asked to open and exits.
///
/// The first thing on the list the opener tries, and the only thing on the
/// `PATH` the app is started with — so what is written here is what a browser
/// would have been shown, and an empty file is a browser that was never asked.
struct Opener {
    bin: PathBuf,
    opened: PathBuf,
}

impl Opener {
    fn in_dir(dir: &Path) -> Opener {
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

    /// What the app asked for, or nothing where it has asked for nothing.
    fn asked_for(&self) -> Option<String> {
        std::fs::read_to_string(&self.opened).ok()
    }

    /// Wait for the app to ask for `url`, which it does as it comes up.
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

/// The desktop app as an icon starts it: a child process with a Data Directory
/// of its own, killed when the test is done with it.
struct App {
    child: Option<Child>,
    url: String,
}

impl App {
    /// Start the app with `args`, and block until it answers.
    ///
    /// The environment is cut down to what a desktop session gives it and no
    /// more: `PATH` is the stand-in browser alone, so nothing on this machine is
    /// really opened, and everything Verkstead reads out of the environment is
    /// dropped, so a machine with Verkstead configured for real is not what a
    /// test reads.
    ///
    /// `DISPLAY` and `WAYLAND_DISPLAY` go with them, which is what says there is
    /// no screen here — see [`verkstead_desktop::dialog`]. A test that left them
    /// would be one that popped a dialog up in front of whoever was running it.
    ///
    /// `env` is what a test puts back, which is only ever what it is about.
    fn start(port: u16, opener: &Opener, home: &Path, args: &[&str], env: &[(&str, &str)]) -> App {
        let child = command(opener, home, env)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the desktop binary should be built for its own tests");

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
                "the desktop app never answered on {}",
                self.url
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Stop the app and hand back everything it said on stderr.
    ///
    /// Read once it has gone rather than while it runs: the pipe is where it
    /// writes, and reading to the end of one is what says there is no more.
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

/// The binary, with the environment every test gives it and whatever `env` the
/// test itself is about.
fn command(opener: &Opener, home: &Path, env: &[(&str, &str)]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_verkstead-desktop"));
    command
        .env("PATH", &opener.bin)
        .env("HOME", home)
        .env_remove("RUST_LOG")
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
        .env_remove("APPIMAGE")
        .env_remove("VERKSTEAD_LISTEN")
        .env_remove("VERKSTEAD_DATA_DIR")
        .env_remove("VERKSTEAD_WATCHED_PATHS");
    for (name, value) in env {
        command.env(name, value);
    }
    command
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

fn as_args(flags: &[String; 4]) -> Vec<&str> {
    flags.iter().map(String::as_str).collect()
}

/// Started with nothing said about opening, the app serves the viewer and hands
/// it to the browser: the whole of what double-clicking an icon is for.
#[test]
fn the_app_serves_the_viewer_and_opens_it() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut app = App::start(port, &opener, tmp.path(), &as_args(&flags), &[]);

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
/// same Verkstead. The XDG variable stands in for the platform here so the test
/// reads a directory of its own; which directory each platform answers with is
/// [`verkstead_server::platform`]'s own unit tests.
#[test]
fn nothing_said_puts_the_work_in_the_platform_data_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let xdg = home.join("data");
    std::fs::create_dir_all(&xdg).unwrap();
    let port = free_port();

    let mut app = App::start(
        port,
        &opener,
        &home,
        &["--listen", &format!("127.0.0.1:{port}"), "--no-open"],
        &[("XDG_DATA_HOME", xdg.to_str().unwrap())],
    );

    let chosen = xdg.join("verkstead");
    assert!(
        chosen.join("verkstead.db").exists(),
        "the database belongs in the platform directory at {}",
        chosen.display()
    );

    app.stop();
}

/// `--no-open` is about the window appearing and nothing else: the server comes
/// up exactly as it otherwise would, and no browser is asked for.
#[test]
fn no_open_serves_just_the_same_and_opens_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut args = as_args(&flags);
    args.push("--no-open");
    let mut app = App::start(port, &opener, tmp.path(), &args, &[]);

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

/// A taken address is the one failure this binary draws rather than prints, and
/// it is found before the app has made anything: there is a second Verkstead on
/// this machine, and the one that lost is not the one that should have written
/// to its directory.
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
    let refused = command(&opener, tmp.path(), &[])
        .args(as_args(&flags))
        .output()
        .expect("the desktop binary should be built for its own tests");

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

/// The one flag of the app's own, beside the server's own — which is how a
/// developer runs it out of a checkout.
#[test]
fn the_help_names_the_flag_and_the_server_options_it_sits_beside() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());

    let help = command(&opener, tmp.path(), &[])
        .arg("--help")
        .output()
        .expect("the desktop binary should be built for its own tests");
    let help = String::from_utf8(help.stdout).unwrap();

    for phrase in ["--no-open", "--data-dir", "--listen", "VERKSTEAD_DATA_DIR"] {
        assert!(
            help.contains(phrase),
            "`verkstead-desktop --help` should mention {phrase:?}, got:\n{help}"
        );
    }
}

/// A session that names a screen the toolkit cannot open — a stale `DISPLAY`
/// left in a shell profile, an X server that has gone away — is a machine with
/// no tray and every other reason to go on serving. The screen every other test
/// here has is *no* screen, which is the case the app never even asks the
/// toolkit about; this is the other one, where it asks and is refused.
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
        &opener,
        tmp.path(),
        &as_args(&flags),
        &[("DISPLAY", ":917")],
    );

    opener.await_asked_for(&format!("http://127.0.0.1:{port}/"));

    app.stop();
}

/// Where the log file goes on a machine that says nothing but where its home
/// is, which is the Linux default and the case every desktop actually has.
fn log_file(home: &Path) -> PathBuf {
    home.join(".local/state/verkstead/verkstead.log")
}

/// A tray app has no stdout anybody will ever read, so the server's log goes to
/// the Log Directory instead — made by this binary, because the resolving that
/// stage 01 landed deliberately makes nothing.
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
    let mut app = App::start(port, &opener, &home, &args, &[]);

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
/// where the events go is the starting binary's call, and which of them are
/// worth writing is `RUST_LOG`'s.
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
    let mut app = App::start(port, &opener, &home, &args, &[("RUST_LOG", "error")]);

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
/// where the logging went instead and goes on serving. A relative home is how a
/// Unix machine gets there — see `verkstead_server::platform`, where a directory
/// resolved against wherever the app was launched from is the thing the platform
/// default replaces.
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
        &opener,
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
/// that appeared and vanished is otherwise the whole of what the human was
/// told. A Watched Path that is not there is the shortest of those failures —
/// the server resolves them before it makes anything — and it stands here for
/// the Data Directory that cannot be written and the machine with no `HOME`.
///
/// The dialog beside it is the one part of this no test here can see, wanting a
/// screen that these deliberately do not have.
#[test]
fn a_startup_that_fails_after_the_address_says_so_in_the_log() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let data_dir = tmp.path().join("data");
    let missing = tmp.path().join("not-a-directory");
    let port = free_port();

    let flags = flags(port, &data_dir);
    let mut args = as_args(&flags);
    args.push("--no-open");
    args.push("--watched-path");
    args.push(missing.to_str().unwrap());

    let refused = command(&opener, &home, &[])
        .args(&args)
        .output()
        .expect("the desktop binary should be built for its own tests");

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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Where the startup registration goes on a machine that says nothing but where
/// its home is, which is the XDG default and the case every desktop actually
/// has.
fn autostart_entry(home: &Path) -> PathBuf {
    home.join(".config/autostart/net.tobico.Verkstead.desktop")
}

/// A binary that has moved leaves a registration naming where it used to be,
/// and the next launch by hand is where that is put right — the whole of why
/// this is "rewritten every launch" rather than "written once". The launch is an
/// ordinary one in every other way: it serves, and it opens the viewer.
#[test]
fn a_launch_rewrites_a_startup_registration_that_names_somewhere_else() {
    let tmp = tempfile::tempdir().unwrap();
    let opener = Opener::in_dir(tmp.path());
    let home = tmp.path().join("home");
    let data_dir = tmp.path().join("data");
    let port = free_port();

    let entry = autostart_entry(&home);
    std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
    std::fs::write(
        &entry,
        "[Desktop Entry]\nType=Application\nName=Verkstead\n\
         Exec=\"/somewhere/it/used/to/be/verkstead-desktop\" --no-open\n",
    )
    .unwrap();

    let flags = flags(port, &data_dir);
    let mut app = App::start(port, &opener, &home, &as_args(&flags), &[]);

    opener.await_asked_for(&format!("http://127.0.0.1:{port}/"));

    let registered = std::fs::read_to_string(&entry).unwrap();
    assert!(
        registered.contains(env!("CARGO_BIN_EXE_verkstead-desktop")),
        "the registration should name the executable that is running, got:\n{registered}"
    );
    assert!(
        !registered.contains("/somewhere/it/used/to/be/"),
        "and not the one it was written for, got:\n{registered}"
    );
    assert!(
        registered.contains("--no-open"),
        "a login should not be handed a browser window, got:\n{registered}"
    );

    app.stop();
}

/// And a machine nobody asked to be started on is left alone: a launch rewrites
/// what somebody asked for rather than deciding it for them, and the box is the
/// only thing that ever registers Verkstead.
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
    let mut app = App::start(port, &opener, &home, &args, &[]);

    assert!(
        !autostart_entry(&home).exists(),
        "{} should not be there",
        autostart_entry(&home).display()
    );

    app.stop();
}
