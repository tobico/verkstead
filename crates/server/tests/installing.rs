//! Installing what the wizard's first step is missing, asked of the server over
//! the viewer's namespace: the press, what it raises, and what the reading says
//! while it runs.
//!
//! **Nothing here is asked of the box the suite is on.** The machine is stated
//! the way the wizard's own suite states one — a `PATH` of the suite's making,
//! with a `bwrap` and a package manager that are shell scripts in it — and the
//! password dialog is a stub that records the command it was handed and answers
//! for the human. So every distribution and both arms of the handle are asked
//! about on the Linux runner, and no test here puts a dialog on anybody's screen
//! or a package on anybody's machine.
//!
//! **The stub package manager really installs.** What it writes into that `PATH`
//! is the programs the packages it was named would have left there, so the rows
//! go present the way they would on a machine: the probe behind them is the same
//! `PATH` walk and the same `bwrap` run either way, and a suite that asserted
//! the run's own word for it would be asserting nothing.
//!
//! Unix only, for the reason the wizard's own suite is: what a package manager
//! did is a shell script here, and a script is what a machine with a shell can
//! be handed.
#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    Dependency, DependencyState, DependencyView, InstallPress, InstallState, OnboardingView,
    RunPhase,
};
use verkstead_server::github::Gh;
use verkstead_server::onboarding::Machine;
use verkstead_server::platform::{Environment, Platform};
use verkstead_server::remote::{Elevate, Raised};
use verkstead_server::{open_database, router_onboarding_elevating};

/// What this machine calls itself, which is the one thing in the status line
/// that is a fact about the box.
const HOSTNAME: &str = "ada-box";

/// What a dismissed password dialog says, which is the line every row of the
/// refused unit carries.
const DISMISSED: &str = "Error: (-128) User canceled.";

/// The ticking this suite is mostly about: the two rows that gate the step, and
/// a harness that comes off npm — which is the press that has a package
/// manager and an `npm install -g` on one line.
const TICKED: &[Dependency] = &[Dependency::Sandbox, Dependency::Git, Dependency::Codex];

/// What an Ubuntu says about itself.
const OS_RELEASE: &str = "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n";

/// How long a test waits on the run's own thread before giving up on it.
const PATIENCE: Duration = Duration::from_secs(10);

/// A Data Directory with a database in it, an author and a Profile: everything
/// but the dependencies, so that what the wizard is held on is the step this
/// suite presses.
async fn ready() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool)
}

/// A server over it, probing a stated Ubuntu and raising through `dialog`.
///
/// The stubs go on that machine's `PATH` before the router is stood up, the
/// mode being settled by the first reading of a machine that has what it has.
fn served(dir: &Path, pool: &SqlitePool, dialog: Option<Arc<Dialog>>) -> Router {
    served_on(dir, pool, OS_RELEASE, dialog)
}

/// The same, on a machine that says `os_release` about itself — which is the
/// other half of the arm: a distribution Verkstead has no command for.
fn served_on(
    dir: &Path,
    pool: &SqlitePool,
    os_release: &str,
    dialog: Option<Arc<Dialog>>,
) -> Router {
    let machine = Machine::stated(
        Platform::Linux,
        OsString::from(bin(dir).as_os_str()),
        OsString::from(bin(dir).as_os_str()),
        None,
        Some(os_release.to_owned()),
        &Environment {
            home: Some(home(dir)),
            ..Environment::default()
        },
    )
    .called(HOSTNAME.to_owned());

    router_onboarding_elevating(
        pool.clone(),
        dir.to_owned(),
        machine,
        Gh::on_path(),
        dialog.map(|dialog| dialog as Arc<dyn Elevate>),
    )
}

/// The directory the stated machine searches, which is where a stub package
/// manager installs.
fn bin(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();

    bin
}

/// And the home it was started under, which is nothing this suite puts anything
/// in: what is asked about here is the `PATH`.
fn home(dir: &Path) -> PathBuf {
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();

    home
}

/// A script at `path`, executable.
fn program(path: &Path, script: &str) {
    std::fs::write(path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A stub `apt-get` and a stub `npm` in that directory: each of them writes the
/// programs the packages it was named would have left behind.
///
/// **Which is the whole of why they are scripts.** A row goes present because a
/// `PATH` walk finds a program and a `bwrap` runs, so a package manager that
/// installed nothing would be a run that reported success over rows that stayed
/// absent — which is precisely the failure worth catching.
fn a_package_manager(dir: &Path) {
    let into = bin(dir).to_string_lossy().into_owned();

    // Outside the `PATH` the machine searches, and deliberately: a machine that
    // already had an `npm` is a machine node never goes into the elevated batch
    // for, and what puts this one where a session would find it is the
    // `apt-get` below installing the package it belongs to.
    let once_installed = dir.join("npm-once-installed");

    program(
        &once_installed,
        &format!(
            "#!/bin/sh\n\
             test \"$1 $2\" = 'install -g' || exit 2\n\
             shift 2\n\
             for package in \"$@\"; do\n\
             case \"$package\" in\n\
             @openai/codex) name=codex;;\n\
             opencode-ai) name=opencode;;\n\
             *) name=$package;;\n\
             esac\n\
             printf '#!/bin/sh\\nexit 0\\n' > '{into}'/$name\n\
             chmod 755 '{into}'/$name\n\
             done\n"
        ),
    );

    program(
        &bin(dir).join("apt-get"),
        &format!(
            "#!/bin/sh\n\
             test \"$1 $2\" = 'install -y' || exit 2\n\
             shift 2\n\
             for package in \"$@\"; do\n\
             if [ \"$package\" = npm ]; then\n\
             cp '{}' '{into}'/npm\n\
             continue\n\
             fi\n\
             case \"$package\" in\n\
             bubblewrap) name=bwrap;;\n\
             nodejs) name=node;;\n\
             *) name=$package;;\n\
             esac\n\
             printf '#!/bin/sh\\nexit 0\\n' > '{into}'/$name\n\
             chmod 755 '{into}'/$name\n\
             done\n",
            once_installed.display(),
        ),
    );
}

/// The platform's password dialog, stubbed: what it was handed, what it does
/// with it, and what it answers.
#[derive(Debug)]
struct Dialog {
    /// Every command it was asked to raise, in order.
    raised: Mutex<Vec<Vec<String>>>,

    /// Set the moment one has been handed over, which is how a test asks about
    /// a run while it is under way rather than racing it.
    entered: AtomicBool,

    /// And what it waits for before answering, where a test wants that window
    /// held open. Nothing is a dialog somebody answers at once.
    held: Option<Arc<AtomicBool>>,

    /// Whether the command runs, or the human dismissed the dialog.
    answers: Answer,

    /// Where the stubs are, which is the `PATH` a raised command is run with:
    /// the elevated half of a real machine searches root's own, and this is
    /// this machine's.
    bin: PathBuf,
}

/// What the human did with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// Typed the password: the command runs, and what it exits with is the
    /// answer.
    Typed,

    /// Or dismissed the dialog, which is the privilege not taken.
    Dismissed,
}

impl Dialog {
    /// A dialog that is answered at once.
    fn answered(dir: &Path, answers: Answer) -> Arc<Dialog> {
        Arc::new(Dialog {
            raised: Mutex::new(Vec::new()),
            entered: AtomicBool::new(false),
            held: None,
            answers,
            bin: bin(dir),
        })
    }

    /// And one that is held open until `held` is set, which is the window an
    /// install screen is drawn in.
    fn held(dir: &Path, held: &Arc<AtomicBool>) -> Arc<Dialog> {
        Arc::new(Dialog {
            raised: Mutex::new(Vec::new()),
            entered: AtomicBool::new(false),
            held: Some(held.clone()),
            answers: Answer::Typed,
            bin: bin(dir),
        })
    }

    /// What it has been handed.
    fn commands(&self) -> Vec<Vec<String>> {
        self.raised.lock().unwrap().clone()
    }

    /// The one command it has been handed, as the shell line inside it.
    fn line(&self) -> String {
        let raised = self.commands();

        let [command] = raised.as_slice() else {
            panic!("one press is one dialog: {raised:?}");
        };

        command.last().expect("a shell line to run").clone()
    }

    /// Hold until a command has been handed over, so that what a test reads
    /// next is a run with a unit under way.
    fn wait(&self) {
        until(
            || self.entered.load(Ordering::SeqCst),
            "a command to be raised",
        );
    }
}

impl Elevate for Dialog {
    fn raise(&self, command: &[String]) -> Raised {
        self.raised.lock().unwrap().push(command.to_vec());
        self.entered.store(true, Ordering::SeqCst);

        if let Some(held) = &self.held {
            until(|| held.load(Ordering::SeqCst), "the dialog to be answered");
        }

        if self.answers == Answer::Dismissed {
            return Raised::Refused {
                why: DISMISSED.to_owned(),
            };
        }

        let (program, arguments) = command.split_first().expect("a program to run");

        let told = std::process::Command::new(program)
            .args(arguments)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.bin.display(),
                    std::env::var("PATH").unwrap_or_default(),
                ),
            )
            .output()
            .expect("the stub package manager to run");

        if told.status.success() {
            return Raised::Done;
        }

        Raised::Refused {
            why: String::from_utf8_lossy(&told.stderr)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("the command failed")
                .to_owned(),
        }
    }
}

/// Hold until `settled`, or give up saying what was waited for.
fn until(settled: impl Fn() -> bool, what: &str) {
    let gave_up = Instant::now() + PATIENCE;

    while !settled() {
        assert!(Instant::now() < gave_up, "waited {PATIENCE:?} for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
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

/// The press that starts a run, which answers with that reading made again.
async fn install(app: &Router, ticked: &[Dependency]) -> OnboardingView {
    let response = pressed(app, ticked).await;

    assert_eq!(response.0, StatusCode::OK);

    serde_json::from_slice(&response.1).expect("the reading the wizard is drawn from")
}

/// The same press, with whatever it was answered with — which is how the two
/// refusals are asked about.
async fn pressed(app: &Router, ticked: &[Dependency]) -> (StatusCode, axum::body::Bytes) {
    let press = InstallPress {
        dependencies: ticked.to_vec(),
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/onboarding/install")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&press).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();

    (status, body)
}

/// And the press that stops one.
async fn cancel(app: &Router) -> OnboardingView {
    answered(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/onboarding/install/cancel")
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

/// The wizard's last Continue, which is what takes the mode off.
async fn finished(app: &Router) -> OnboardingView {
    answered(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/ui/onboarding/finished")
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

/// One row of a reading.
fn row(reading: &OnboardingView, dependency: Dependency) -> &DependencyView {
    reading
        .dependencies
        .iter()
        .find(|row| row.dependency == dependency)
        .expect("every dependency is a row")
}

/// Whether a row says the machine has the thing.
fn present(reading: &OnboardingView, dependency: Dependency) -> bool {
    matches!(
        row(reading, dependency).state,
        DependencyState::Present { .. }
    )
}

/// And what the run has made of it.
fn install_state(reading: &OnboardingView, dependency: Dependency) -> &InstallState {
    &row(reading, dependency).install
}

/// Ticking the sandbox, git and Codex on an Ubuntu raises one command naming
/// every package, and the rows go present once it has run.
///
/// The whole of the feature in one test: one press, one dialog, one command —
/// with node in it because this machine has no npm and the `npm install -g`
/// joined onto the same line — and rows that read installing while it runs and
/// present when it is over, off the same probe every other reading is drawn
/// from.
#[tokio::test]
async fn ticking_three_rows_raises_one_command_and_the_rows_go_present() {
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());

    let held = Arc::new(AtomicBool::new(false));
    let dialog = Dialog::held(dir.path(), &held);
    let app = served(dir.path(), &pool, Some(dialog.clone()));

    let pressed = install(&app, TICKED).await;
    let run = pressed.run.expect("a press makes a run");

    assert_eq!(run.total, 3, "one unit per ticked row: {run:?}");
    assert_eq!(run.done, 0);
    assert_eq!(
        run.status,
        format!("Waiting for the password dialog on {HOSTNAME}"),
        "the line names the machine the dialog is on",
    );

    // Held there until the dialog has been handed the command, which is the
    // window the install screen is drawn in.
    dialog.wait();

    let waiting = reading(&app).await;

    for ticked in TICKED {
        assert_eq!(
            install_state(&waiting, *ticked),
            &InstallState::Installing,
            "{ticked:?} is what the run is doing",
        );
        assert!(!present(&waiting, *ticked), "{ticked:?} is not there yet");
    }

    assert_eq!(
        dialog.line(),
        format!(
            ": > '{}' && apt-get install -y bubblewrap git nodejs npm && \
             npm install -g @openai/codex",
            started(&dialog),
        ),
        "one command, naming every package and the npm install on the same line",
    );

    held.store(true, Ordering::SeqCst);

    let landed = over(&app).await;
    let run = landed.run.clone().expect("the run it was started with");

    assert_eq!(run.done, 3);
    assert_eq!(run.status, "Everything that was ticked is installed");
    assert_eq!(dialog.commands().len(), 1, "one press is one dialog");

    for ticked in TICKED {
        assert!(present(&landed, *ticked), "{ticked:?}: {landed:?}");
        assert_eq!(install_state(&landed, *ticked), &InstallState::Idle);
    }

    assert!(
        landed.steps.dependencies,
        "the step the wizard was held on is met by what the run installed",
    );
}

/// The marker the raised command touches, taken back off the line it is in.
///
/// Its path is a temporary directory of the run's own making, so what a test
/// can assert about it is that it is there and that the line begins by writing
/// it — see the `STARTED` marker in `onboarding::install`.
fn started(dialog: &Dialog) -> String {
    let line = dialog.line();
    let (_, rest) = line.split_once('\'').expect("a marker in the line");
    let (path, _) = rest.split_once('\'').expect("a marker in the line");

    path.to_owned()
}

/// A dismissed dialog leaves every row of that unit failed, in the words the
/// asking was refused in.
#[tokio::test]
async fn a_dismissed_dialog_fails_every_ticked_row() {
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());

    let app = served(
        dir.path(),
        &pool,
        Some(Dialog::answered(dir.path(), Answer::Dismissed)),
    );

    install(&app, TICKED).await;

    let landed = over(&app).await;

    for ticked in TICKED {
        assert_eq!(
            install_state(&landed, *ticked),
            &InstallState::Failed {
                why: DISMISSED.to_owned(),
            },
            "{ticked:?}",
        );
        assert!(!present(&landed, *ticked), "{ticked:?} was not installed");
    }

    assert_eq!(
        landed.run.expect("the run").status,
        "3 of 3 could not be installed",
    );
}

/// And a server nothing handed a way to ask fails them the same way, without
/// anything being raised.
#[tokio::test]
async fn a_server_with_no_way_to_ask_never_raises_anything() {
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());

    let app = served(dir.path(), &pool, None);

    let pressed = install(&app, TICKED).await;
    let run = pressed
        .run
        .clone()
        .expect("a press makes a run even where nothing can be raised");

    assert_eq!(
        run.phase,
        RunPhase::Done,
        "there was never anything to wait for"
    );
    assert_eq!((run.done, run.total), (3, 3));

    for ticked in TICKED {
        let InstallState::Failed { why } = install_state(&pressed, *ticked) else {
            panic!("{ticked:?}: {:?}", install_state(&pressed, *ticked));
        };

        assert!(why.contains("desktop app"), "{why}");
    }
}

/// And a distribution Verkstead has no command for fails them without raising
/// anything either — with a line saying what that distribution installs from
/// instead.
#[tokio::test]
async fn a_distribution_with_no_command_never_raises_anything() {
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());

    let dialog = Dialog::answered(dir.path(), Answer::Typed);
    let app = served_on(dir.path(), &pool, "ID=nixos\n", Some(dialog.clone()));

    let pressed = install(&app, TICKED).await;

    assert_eq!(
        pressed.run.clone().expect("a press makes a run").phase,
        RunPhase::Done,
    );
    assert!(
        dialog.commands().is_empty(),
        "a machine there is no command for is not one to put a dialog on: {:?}",
        dialog.commands(),
    );

    for ticked in TICKED {
        let InstallState::Failed { why } = install_state(&pressed, *ticked) else {
            panic!("{ticked:?}: {:?}", install_state(&pressed, *ticked));
        };

        assert!(why.contains("configuration"), "{why}");
    }
}

/// Cancel is pressed while the dialog is still up: the unit under way finishes,
/// and the run reads done.
#[tokio::test]
async fn cancelling_reads_cancelling_until_the_unit_under_way_is_over() {
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());

    let held = Arc::new(AtomicBool::new(false));
    let dialog = Dialog::held(dir.path(), &held);
    let app = served(dir.path(), &pool, Some(dialog.clone()));

    install(&app, TICKED).await;
    dialog.wait();

    let cancelling = cancel(&app).await.run.expect("the run being cancelled");

    assert!(cancelling.cancelling, "the unit under way has not finished");
    assert_ne!(cancelling.phase, RunPhase::Done);

    held.store(true, Ordering::SeqCst);

    let landed = over(&app).await;
    let run = landed.run.clone().expect("the run");

    assert!(!run.cancelling, "a run that is over is cancelling nothing");
    assert!(
        present(&landed, Dependency::Git),
        "what was already running was left to finish: {landed:?}",
    );
}

/// The press is refused while a run is going, and once the wizard is over.
#[tokio::test]
async fn the_press_is_refused_twice_over_and_after_the_wizard() {
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());

    let held = Arc::new(AtomicBool::new(false));
    let dialog = Dialog::held(dir.path(), &held);
    let app = served(dir.path(), &pool, Some(dialog.clone()));

    install(&app, TICKED).await;
    dialog.wait();

    assert_eq!(
        pressed(&app, TICKED).await.0,
        StatusCode::CONFLICT,
        "a second press while a run is going is the same press twice",
    );

    held.store(true, Ordering::SeqCst);
    over(&app).await;

    assert_eq!(
        pressed(&app, &[Dependency::Gh]).await.0,
        StatusCode::OK,
        "and a press once that run is over is a run of its own",
    );

    over(&app).await;
    finished(&app).await;

    assert_eq!(
        pressed(&app, &[Dependency::OpenCode]).await.0,
        StatusCode::CONFLICT,
        "the wizard is over, so there is nothing here to install",
    );

    assert_eq!(
        dialog.commands().len(),
        2,
        "the two presses that were taken, and neither of the two that were not",
    );
}

/// Where the golden fixtures are written, relative to this crate — the same
/// directory the wizard's own suite writes its readings to.
const FIXTURES: &str = "../../web/tests/fixtures";

/// Leave the viewer's own tests a reading of each shape the install screen and
/// the hint screen are drawn over.
///
/// Committed, and rewritten by every run of this test: the diff is the review.
/// Two of them, because two are what those screens have to draw — a run under
/// way, and one that is over with something left on it.
///
/// Nothing here is read off the machine the suite is on: the `PATH`, the
/// `bwrap`, `/etc/os-release` and the hostname are all stated, so a run today
/// and a run on another box write the same bytes.
#[tokio::test]
async fn the_viewers_own_tests_are_fed_from_here() {
    // A run under way: the dialog is up, every ticked row is installing, and
    // nothing is done of the three.
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());

    let held = Arc::new(AtomicBool::new(false));
    let dialog = Dialog::held(dir.path(), &held);
    let app = served(dir.path(), &pool, Some(dialog.clone()));

    install(&app, TICKED).await;
    dialog.wait();

    write(
        "onboarding-installing.json",
        &reading(&app).await,
        dir.path(),
    );

    held.store(true, Ordering::SeqCst);
    over(&app).await;

    // And one that is over with two kinds of trouble on it: a dialog somebody
    // dismissed, and a row this distribution has no package for at all. Which
    // is the hint screen's own reading — the rows that were ticked and are
    // still absent, each with a sentence under it.
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());

    let app = served(
        dir.path(),
        &pool,
        Some(Dialog::answered(dir.path(), Answer::Dismissed)),
    );

    install(
        &app,
        &[Dependency::Sandbox, Dependency::Git, Dependency::Claude],
    )
    .await;

    write("onboarding-failed.json", &over(&app).await, dir.path());
}

/// What a home reads as in a fixture, whoever ran the suite.
const A_HOME: &str = "/home/you";

/// And what the rest of the directory this run was given reads as.
const A_MACHINE: &str = "/machine";

/// One fixture, as the server would have written it — with the paths this run
/// made written back out as ones anybody would recognise as their own.
fn write(name: &str, reading: &OnboardingView, ran_in: &Path) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES);
    std::fs::create_dir_all(&dir).unwrap();

    let pretty = serde_json::to_string_pretty(reading).unwrap();
    let pretty = pretty.replace(&home(ran_in).to_string_lossy().into_owned(), A_HOME);
    let pretty = pretty.replace(&ran_in.to_string_lossy().into_owned(), A_MACHINE) + "\n";

    std::fs::write(dir.join(name), pretty).unwrap();
}
