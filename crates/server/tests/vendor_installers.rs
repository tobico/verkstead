//! The two rows that are a vendor's own installer rather than a package: what a
//! ticked one runs, where it lands, and the `session_path` write that puts what
//! it landed on a session's `PATH`.
//!
//! **One test in a binary of its own, and that is the whole design of this
//! file** — `tests/session_path.rs`'s design, for its reason. What is asserted
//! here runs through the list this *process* holds: an install that lands in a
//! directory appends it to `session_path` and to that list at once, and the next
//! probe is composed with it. Standing a machine up whose server has no
//! `~/.local/bin` on its `PATH` means moving the environment, and
//! `std::env::set_var` is `unsafe` under this edition because it races every
//! other thread in the process reading one. So there is one test here, and no
//! other thread to race.
//!
//! Everything else about the run — the elevated batch, a dismissed dialog, a
//! cancel, an installer that exited non-zero — is `tests/installing.rs`, where a
//! machine is stated and its `PATH` stands still.
//!
//! **Nothing here reaches the network and nothing installs anything.** What a
//! ticked Claude row runs is `curl -fsSL https://claude.ai/install.sh | bash`
//! and what a ticked Grok Build row runs is xAI's line beside it; the `curl` on
//! this machine's `PATH` is a stub that answers with a script of this suite's
//! own, and that script copies a program into the directory the vendor's
//! installer really uses. So the `session_path` write and the re-probe are
//! proven, and no vendor is asked for anything.
//!
//! Unix only, for `tests/session_path.rs`'s reason: what is written here is a
//! `HOME`, a colon-separated `PATH` and a handful of shell scripts, and the
//! Windows arm reads none of them.
#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::{
    Dependency, DependencyState, DependencyView, InstallPress, InstallState, OnboardingView,
    RunPhase,
};
use verkstead_server::github::Gh;
use verkstead_server::onboarding::Machine;
use verkstead_server::platform::{Environment, Platform};
use verkstead_server::remote::{Elevate, Raised};
use verkstead_server::settings::Settings;
use verkstead_server::{open_database, router_onboarding_elevating, sandbox};

/// A harness that is there and does nothing, which is the whole of what a row
/// probed by a `PATH` walk asks of one.
const A_PROGRAM: &str = "#!/bin/sh\nexit 0\n";

/// Where Anthropic's own installer lands `claude`, under the home it ran as.
const CLAUDE_LANDS: &str = ".local/bin";

/// And where xAI's lands `grok`.
const GROK_LANDS: &str = ".grok/bin";

/// How long the test waits on the run's own thread before giving up on it.
const PATIENCE: Duration = Duration::from_secs(20);

/// Ticking the sandbox, git and both vendor rows on an Ubuntu: the elevated
/// batch first, then an installer apiece run as the user — and the directories
/// they landed in are on `session_path`, on the next probe and on the next
/// session's `PATH`, with nothing restarted.
#[tokio::test]
async fn the_vendor_installers_run_as_the_user_and_what_they_land_is_on_the_path() {
    let home = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();

    // Where the machine's own programs go: the stubs this suite writes, and
    // whatever the elevated batch installs. Under the home, so that a session's
    // `PATH` is composed with it.
    let bin = home.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();

    // The two programs the stub installer uses, named in full because the `PATH`
    // a unit running as the user is given is the machine's own — see
    // `onboarding::install`'s `AsTheUser` — and this machine's has the stubs on
    // it and little else. Resolved before the environment moves.
    let borrowed = Borrowed::of_this_box();

    // What a vendor's installer leaves behind, which the stub copies into place:
    // a program, because what makes a row go present is a `PATH` walk finding
    // one.
    let installed = home.path().join("what-the-installer-lands");
    program(&installed, A_PROGRAM);

    a_package_manager(&bin, &borrowed, &installed);
    an_installer(&bin, &borrowed, &installed);

    // The machine this server is coming up on: a `PATH` with the stubs on it and
    // nothing of a harness anywhere, which is what a fresh install has.
    //
    // Safe for the reason the module says: this is the only test in this binary,
    // so there is no other thread in the process to be reading an environment
    // while it is written.
    let path = OsString::from(bin.to_string_lossy().into_owned());

    unsafe {
        std::env::set_var("HOME", home.path());
        std::env::set_var("PATH", &path);
    }

    // Startup: the read the server makes as it comes up, which on a machine
    // Verkstead has installed nothing on yet holds nothing.
    let settings = Settings::in_data_dir(data.path());
    sandbox::hold_session_path(&settings);

    assert!(
        settings.config().session_path().is_empty(),
        "nothing has been installed yet",
    );

    let pool = open_database(&data.path().join("verkstead.db"))
        .await
        .unwrap();

    let dialog = Arc::new(Dialog {
        raised: Mutex::new(Vec::new()),
        path: path.clone(),
    });

    // A stated Ubuntu — so that the packages are `apt-get`'s and the test says
    // which distribution this is rather than asking the runner — composing its
    // `PATH` the way the machine this server runs on does, that being the half
    // an install moves.
    let machine = Machine::stated(
        Platform::Linux,
        path.clone(),
        path.clone(),
        None,
        Some("ID=ubuntu\n".to_owned()),
        &Environment {
            home: Some(home.path().to_owned()),
            ..Environment::default()
        },
    )
    .composing();

    let app = router_onboarding_elevating(
        pool,
        data.path().to_owned(),
        machine,
        Gh::on_path(),
        Some(dialog.clone() as Arc<dyn Elevate>),
    );

    let ticked = [
        Dependency::Sandbox,
        Dependency::Git,
        Dependency::Claude,
        Dependency::Grok,
    ];

    let pressed = install(&app, &ticked).await;

    assert_eq!(
        pressed.run.expect("a press makes a run").total,
        4,
        "one row of the run per ticked row",
    );

    let landed = over(&app).await;
    let run = landed.run.clone().expect("the run it was started with");

    assert_eq!(run.done, 4);
    assert_eq!(run.status, "Everything that was ticked is installed");

    // The elevated half is the package manager and nothing else: an installer
    // that writes under this user's home has no business behind a password
    // dialog, and one raised there would have written under root's.
    let raised = dialog.raised.lock().unwrap().clone();

    assert_eq!(
        raised.len(),
        1,
        "the batch was raised and neither installer was: {raised:?}",
    );
    assert!(
        raised[0].last().expect("a shell line").contains("apt-get"),
        "and the one that was, is the package manager: {raised:?}",
    );

    let local = home.path().join(CLAUDE_LANDS);
    let grok = home.path().join(GROK_LANDS);

    assert_eq!(
        state(&landed, Dependency::Claude),
        &DependencyState::Present {
            at: Some(local.join("claude").to_string_lossy().into_owned()),
            target: None,
        },
        "the row is present at the directory Anthropic's installer lands in",
    );
    assert_eq!(
        state(&landed, Dependency::Grok),
        &DependencyState::Present {
            at: Some(grok.join("grok").to_string_lossy().into_owned()),
            target: None,
        },
        "and at the one xAI's does",
    );

    for installed in ticked {
        assert_eq!(
            install_state(&landed, installed),
            &InstallState::Idle,
            "{installed:?}: nothing is left on a row the run installed",
        );
    }

    assert_eq!(
        settings.config().session_path(),
        [
            local.to_string_lossy().into_owned(),
            grok.to_string_lossy().into_owned(),
        ],
        "both directories are in `config.yaml`, in the order the units ran, so \
         the next start reads them back",
    );

    let entries = looks_in(&sandbox::machine_path(Platform::Linux));

    assert_eq!(
        entries
            .iter()
            .position(|entry| Some(entry.as_str()) == local.to_str()),
        Some(0),
        "and the next session's `PATH` leads with them: {entries:?}",
    );
    assert_eq!(
        entries
            .iter()
            .position(|entry| Some(entry.as_str()) == grok.to_str()),
        Some(1),
        "{entries:?}",
    );

    assert_eq!(
        landed.path.first().map(String::as_str),
        local.to_str(),
        "which is the list the wizard draws, being where a session looks",
    );

    assert!(
        landed.steps.dependencies,
        "the step the wizard was held on is met by what the run installed",
    );
}

/// The two programs of this box's own that the stub installer uses.
///
/// The `PATH` a unit running as the user is handed is the machine's — the stubs
/// and the platform's own floor — so a stub that has a directory to make and a
/// file to put in it names both in full. Resolved on the suite's own `PATH`,
/// before the environment moves.
#[derive(Debug)]
struct Borrowed {
    mkdir: PathBuf,
    cp: PathBuf,
}

impl Borrowed {
    fn of_this_box() -> Borrowed {
        Borrowed {
            mkdir: found("mkdir"),
            cp: found("cp"),
        }
    }
}

/// Where `program` is on the `PATH` this suite is running with.
fn found(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("a `PATH` to run with"))
        .map(|directory| directory.join(program))
        .find(|at| at.is_file())
        .unwrap_or_else(|| panic!("{program} on the suite's own `PATH`"))
}

/// A stub `apt-get` in `bin`, which installs what it was named by putting a
/// program where a session would look for it.
///
/// **It really installs**, for `tests/installing.rs`'s reason: a row goes
/// present because a `PATH` walk finds a program, so a package manager that
/// installed nothing would be a run reporting success over rows that stayed
/// absent.
fn a_package_manager(bin: &Path, borrowed: &Borrowed, installed: &Path) {
    program(
        &bin.join("apt-get"),
        &format!(
            "#!/bin/sh\n\
             test \"$1\" = update && exit 0\n\
             test \"$1 $2\" = 'install -y' || exit 2\n\
             shift 2\n\
             for package in \"$@\"; do\n\
             case \"$package\" in\n\
             bubblewrap) name=bwrap;;\n\
             *) name=$package;;\n\
             esac\n\
             '{cp}' '{installed}' '{bin}'/$name\n\
             done\n",
            cp = borrowed.cp.display(),
            installed = installed.display(),
            bin = bin.display(),
        ),
    );
}

/// And the `curl` and `bash` a vendor's own installer is, stubbed.
///
/// The line each vendor publishes is `curl -fsSL <the script> | bash`, so what
/// this `curl` answers with is a script that puts a program in the directory
/// that vendor's installer really uses — `~/.local/bin` for Anthropic's and
/// `~/.grok/bin` for xAI's. A URL it does not know is a `curl` that failed,
/// which is what a stub standing in for the network should be.
///
/// `bash` is the machine's `sh`: what is being asked about here is the run and
/// where it lands, rather than which shell reads the script.
fn an_installer(bin: &Path, borrowed: &Borrowed, installed: &Path) {
    program(&bin.join("bash"), "#!/bin/sh\nexec /bin/sh \"$@\"\n");

    program(
        &bin.join("curl"),
        &format!(
            "#!/bin/sh\n\
             into=\n\
             name=\n\
             for argument in \"$@\"; do\n\
             case \"$argument\" in\n\
             *claude.ai*) into=\"$HOME/{CLAUDE_LANDS}\"; name=claude;;\n\
             *x.ai*) into=\"$HOME/{GROK_LANDS}\"; name=grok;;\n\
             esac\n\
             done\n\
             test -n \"$name\" || {{ printf '%s\\n' 'curl: (6) Could not resolve host' >&2; exit 6; }}\n\
             printf '%s\\n' \"'{mkdir}' -p '$into'\" \"'{cp}' '{installed}' '$into/$name'\"\n",
            mkdir = borrowed.mkdir.display(),
            cp = borrowed.cp.display(),
            installed = installed.display(),
        ),
    );
}

/// A file that is there and runnable, which is what a `PATH` walk is looking
/// for.
fn program(path: &Path, contents: &str) {
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// The platform's password dialog, stubbed: what it was handed, and the command
/// run with an answer nobody had to type.
#[derive(Debug)]
struct Dialog {
    raised: Mutex<Vec<Vec<String>>>,

    /// The `PATH` a raised command is run with — the elevated half of a real
    /// machine searches root's own, and this is this machine's.
    path: OsString,
}

impl Elevate for Dialog {
    fn raise(&self, command: &[String]) -> Raised {
        self.raised.lock().unwrap().push(command.to_vec());

        let (program, arguments) = command.split_first().expect("a program to run");

        let told = std::process::Command::new(program)
            .args(arguments)
            .env("PATH", &self.path)
            .output()
            .expect("the stub package manager to run");

        if told.status.success() {
            return Raised::Done;
        }

        Raised::Refused {
            why: String::from_utf8_lossy(&told.stderr).trim().to_owned(),
        }
    }
}

/// The directories a `PATH` names, in the order a session searches them.
fn looks_in(path: &std::ffi::OsStr) -> Vec<String> {
    path.to_string_lossy()
        .split(':')
        .map(str::to_owned)
        .collect()
}

/// One row of a reading.
fn row(reading: &OnboardingView, dependency: Dependency) -> &DependencyView {
    reading
        .dependencies
        .iter()
        .find(|row| row.dependency == dependency)
        .expect("every dependency is a row")
}

/// What the machine says about it.
fn state(reading: &OnboardingView, dependency: Dependency) -> &DependencyState {
    &row(reading, dependency).state
}

/// And what the run has made of it.
fn install_state(reading: &OnboardingView, dependency: Dependency) -> &InstallState {
    &row(reading, dependency).install
}

/// The press that starts a run, which answers with the reading made again.
async fn install(app: &Router, ticked: &[Dependency]) -> OnboardingView {
    let press = InstallPress {
        dependencies: ticked.to_vec(),
    };

    answered(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/onboarding/install")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&press).unwrap()))
            .unwrap(),
    )
    .await
}

/// The reading the wizard is drawn from.
async fn reading(app: &Router) -> OnboardingView {
    answered(
        app,
        Request::builder()
            .uri("/api/ui/onboarding")
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

/// One request, answered with a reading.
async fn answered(app: &Router, request: Request<Body>) -> OnboardingView {
    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    serde_json::from_slice(&body).expect("the reading the wizard is drawn from")
}

/// Read until the run is over, which is the poll the install screen makes.
async fn over(app: &Router) -> OnboardingView {
    let gave_up = Instant::now() + PATIENCE;

    loop {
        let reading = reading(app).await;

        if reading
            .run
            .as_ref()
            .is_some_and(|run| run.phase == RunPhase::Done)
        {
            return reading;
        }

        assert!(
            Instant::now() < gave_up,
            "waited {PATIENCE:?} for the run to end: {:?}",
            reading.run,
        );

        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
