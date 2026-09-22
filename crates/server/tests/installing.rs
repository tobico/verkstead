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
use verkstead_server::settings::Settings;
use verkstead_server::{open_database, router_onboarding_elevating};

/// What this machine calls itself, which is the one thing in the status line
/// that is a fact about the box.
const HOSTNAME: &str = "ada-box";

/// And who it runs as, which is who a Mac hands Homebrew's prefix to.
const USER: &str = "ada";

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
    stood_up(
        dir,
        pool,
        Platform::Linux,
        Some(os_release.to_owned()),
        None,
        dialog,
    )
}

/// And a stated Apple-silicon Mac, which says nothing about itself but which
/// processor it has: there is no `/etc/os-release` on a Mac, and what is left
/// of the answer is `hw.optional.arm64` — see `onboarding::mac`.
fn served_mac(dir: &Path, pool: &SqlitePool, dialog: Option<Arc<Dialog>>) -> Router {
    stood_up(dir, pool, Platform::MacOs, None, ARM64, dialog)
}

/// And the other Mac, which is the same platform saying nothing at all: the
/// OID is absent on an Intel Mac, and a machine that will not answer for its
/// processor is the Mac Homebrew has dropped.
fn served_intel_mac(dir: &Path, pool: &SqlitePool, dialog: Option<Arc<Dialog>>) -> Router {
    stood_up(dir, pool, Platform::MacOs, None, None, dialog)
}

/// What an Apple-silicon Mac answers with, which is the whole of what makes it
/// Homebrew's.
const ARM64: Option<&str> = Some("1");

/// A server over a machine stated whichever way, raising through `dialog`.
///
/// **A Mac searches one directory more than the others**, and that is the
/// platform rather than the suite: a Mac session's `PATH` is composed with the
/// home's own `.local/bin` at its head — see ADR-0016's *Macs* — which is where
/// Anthropic's installer puts `claude`. Everywhere else the stubs' directory is
/// the whole of it, the two vendor rows on a Linux being
/// `tests/vendor_installers.rs`'s.
fn stood_up(
    dir: &Path,
    pool: &SqlitePool,
    platform: Platform,
    os_release: Option<String>,
    arm64: Option<&str>,
    dialog: Option<Arc<Dialog>>,
) -> Router {
    let searches = match platform {
        Platform::MacOs => {
            std::env::join_paths([bin(dir), local_bin(dir), opencode_bin(dir)]).unwrap()
        }
        Platform::Linux | Platform::Windows => OsString::from(bin(dir).as_os_str()),
    };

    let machine = Machine::stated(
        platform,
        searches,
        OsString::from(bin(dir).as_os_str()),
        None,
        os_release,
        &Environment {
            home: Some(home(dir)),
            user: Some(USER.to_owned()),
            ..Environment::default()
        },
    )
    .called(HOSTNAME.to_owned())
    .arm64(arm64.map(str::to_owned));

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

/// And the home it was started under, which is where a vendor's own installer
/// lands what it installs.
fn home(dir: &Path) -> PathBuf {
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();

    home
}

/// The directory under it Anthropic's installer really writes into, which is on
/// the floor a Mac session's `PATH` is composed from — see ADR-0016's *Macs*,
/// and `sandbox::composed`.
///
/// Named rather than made: what makes the row go present is the installer
/// having made it and put a program in it.
fn local_bin(dir: &Path) -> PathBuf {
    home(dir).join(".local/bin")
}

/// And the one OpenCode's own installer writes into, which a Mac session
/// reaches for the other reason: it is on no floor at all, and what puts it on
/// a session's `PATH` is the `session_path` write the unit makes when it lands
/// — see `tests/vendor_installers.rs`, where that write is followed through a
/// machine composing its own `PATH`. Stated here, this machine's `PATH` being
/// a word of the suite's that stands still.
fn opencode_bin(dir: &Path) -> PathBuf {
    home(dir).join(".opencode/bin")
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
    package_manager(dir, REFRESHED);
}

/// What a stub `apt-get update` exits with where the archive answered, and
/// where one of its sources did not.
///
/// **Which is the ordinary desktop rather than a broken one.** `apt-get update`
/// exits non-zero for a single unreachable source, and a machine carrying a
/// third-party list that has gone away is exactly that — so the batch joins the
/// refresh to the install with `;` rather than `&&`, and this is the pair that
/// holds it to it.
const REFRESHED: i32 = 0;
const ONE_SOURCE_GONE: i32 = 1;

/// The same, where the refresh in front of the install exits with `refresh`.
fn package_manager(dir: &Path, refresh: i32) {
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
             test \"$1\" = update && exit {refresh}\n\
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

/// And a stub `brew` in it, which installs what it is named the way the real
/// one does: a program in the prefix's own `bin`, which is the directory this
/// machine searches.
///
/// **The same reason the package managers above are scripts.** What makes a Mac
/// row go present is a `PATH` walk finding a program under Homebrew's prefix, so
/// a `brew` that installed nothing would be a run reporting success over rows
/// that stayed absent.
fn a_homebrew(dir: &Path) {
    // What a formula leaves behind, and the `cp` that puts it there named in
    // full: a unit that runs as the user is handed the machine's own `PATH`,
    // which here is the prefix's `bin` and nothing else — so a stub that has a
    // file to copy names the program that copies it.
    let installed = dir.join("what-a-formula-lands");
    program(&installed, "#!/bin/sh\nexit 0\n");

    program(
        &bin(dir).join("brew"),
        &format!(
            "#!/bin/sh\n\
             test \"$1\" = install || exit 2\n\
             shift\n\
             if [ \"$1\" = --cask ]; then shift; fi\n\
             for formula in \"$@\"; do\n\
             '{cp}' '{installed}' '{into}'/$formula\n\
             done\n",
            cp = found("cp").display(),
            installed = installed.display(),
            into = bin(dir).display(),
        ),
    );
}

/// Where `program` is on the `PATH` this suite is running with.
fn found(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("a `PATH` to run with"))
        .map(|directory| directory.join(program))
        .find(|at| at.is_file())
        .unwrap_or_else(|| panic!("{program} on the suite's own `PATH`"))
}

/// A `curl` and a `bash` in that directory, where what the `curl` answers with
/// is Anthropic's installer doing what it does: a script that puts `claude` in
/// the home's `.local/bin`.
///
/// **Stubbed the way `tests/vendor_installers.rs` stubs it**, and for that
/// file's reason: the line a ticked Claude row runs is `curl -fsSL
/// https://claude.ai/install.sh | bash`, and a suite that ran it would install
/// a harness on whoever's box it happened to be on. A unit that runs as the
/// user searches the machine's own `PATH`, so this is what the line finds
/// instead — and the directory it writes into is the one the real installer
/// uses, which a Mac session reaches because a Mac's floor carries it.
///
/// The two programs it needs are named in full: the `PATH` that unit is handed
/// is this machine's, which holds the stubs and nothing else.
fn an_installer(dir: &Path) {
    program(&bin(dir).join("bash"), "#!/bin/sh\nexec /bin/sh \"$@\"\n");

    let installed = dir.join("what-the-installer-lands");
    program(&installed, "#!/bin/sh\nexit 0\n");

    // Whose installer it is, is the address it was asked for: the one stub
    // answers for Anthropic and for OpenCode, each with the program and the
    // directory that vendor's own script really leaves behind. Which is the
    // one thing a unit says about itself that this suite can check — a `curl`
    // that answered the same script whatever it was asked would prove nothing
    // about which line a ticked row runs.
    program(
        &bin(dir).join("curl"),
        &format!(
            "#!/bin/sh\n\
             case \"$*\" in\n\
             *opencode.ai*) into='{opencode}'; name=opencode ;;\n\
             *) into='{into}'; name=claude ;;\n\
             esac\n\
             printf '%s\n' \"'{mkdir}' -p '$into'\" \"'{cp}' '{installed}' '$into/$name'\"\n",
            opencode = opencode_bin(dir).display(),
            into = local_bin(dir).display(),
            mkdir = found("mkdir").display(),
            cp = found("cp").display(),
            installed = installed.display(),
        ),
    );
}

/// And the same pair where what the `curl` answers with is a script that fails.
///
/// **Stubs rather than the vendor's own, and that is the whole point of them.**
/// What a ticked Claude row runs is `curl -fsSL https://claude.ai/install.sh |
/// bash`, and a suite that ran that would install a harness on whoever's box it
/// happened to be on, over a network it happened to have. A unit that runs as
/// the user searches the machine's own `PATH` — see `onboarding::install` — so
/// these two are what that line finds instead. `bash` is the machine's `sh`,
/// what is being asked about here being the run rather than the shell.
///
/// Builtins only, both of them: the `PATH` this machine is stated with holds
/// the stubs and nothing else, which is what makes the line's own reach the
/// thing being asked about.
fn an_installer_that_fails(dir: &Path) {
    program(&bin(dir).join("bash"), "#!/bin/sh\nexec /bin/sh \"$@\"\n");
    program(
        &bin(dir).join("curl"),
        &format!(
            "#!/bin/sh\n\
             printf '%s\\n' '{UNREACHABLE}' >&2\n\
             printf '%s\\n' 'exit 1'\n"
        ),
    );
}

/// What that script says, which is the line the row it failed carries.
const UNREACHABLE: &str = "The installer could not reach the network.";

/// The version GitHub's releases answer with here, and the tag it is the tag
/// of.
///
/// **A number of this suite's own, and nothing reads it off the box.** What is
/// being asked about is that the version the line installs is the one the
/// redirect named — see `onboarding::install`'s `GH_RELEASE` — so the stub
/// below answers this tag and then refuses every download but this tag's, and a
/// line that had a version written into it would be a run that failed here.
const RELEASED: &str = "2.101.0";

/// The programs Verkstead's own `gh` line reaches for, on the stated machine's
/// `PATH`.
///
/// **The box's own rather than stubs, and put here rather than named in the
/// line.** What a ticked `gh` row runs is Verkstead's line rather than a
/// vendor's script, so it names its tools the way a Mac does — off the `PATH`
/// it is handed — and a stated machine whose `PATH` is one directory of this
/// suite's making has to hold them. They do what they do; the `curl` beside
/// them is the stub, which is the whole of what is standing in for GitHub.
fn the_tools_a_download_wants(dir: &Path) {
    for tool in ["mktemp", "unzip", "mkdir", "cp", "chmod", "rm"] {
        std::os::unix::fs::symlink(found(tool), bin(dir).join(tool))
            .expect("the tools a download wants, on the machine's own `PATH`");
    }
}

/// And a stub `curl` answering for GitHub's releases: the redirect with a tag,
/// and that tag's `macOS_amd64` zip with a `gh` inside it.
///
/// **Stubbed for the reason every network here is**, and for one more: a suite
/// that really asked GitHub would install whatever `gh` was released this
/// morning, over a network a runner happens to have, and would be asking about
/// GitHub rather than about the line. So the two calls the line makes are
/// answered here — the redirect off [`RELEASED`], and the download off a zip
/// this suite built, which is the release's own shape: a directory named for
/// the version with the binary at `bin/gh` inside it.
///
/// **And the download is answered for one URL alone.** The version in it is the
/// one the redirect just named, so a line that asked for any other — a version
/// written into Verkstead, or a tag it never read — is a `curl` that fails and
/// a row that stays absent.
fn a_release(dir: &Path) {
    program(&unpacked(dir).join("bin/gh"), "#!/bin/sh\nexit 0\n");

    let zipped = dir.join("gh.zip");

    let made = std::process::Command::new(found("zip"))
        .arg("--quiet")
        .arg("--recurse-paths")
        .arg(&zipped)
        .arg(unpacked(dir).file_name().expect("the release's directory"))
        .current_dir(dir)
        .status()
        .expect("the suite's own `zip`");

    assert!(
        made.success(),
        "the release this suite answers with: {made}"
    );

    program(
        &bin(dir).join("curl"),
        &format!(
            "#!/bin/sh\n\
             case \"$*\" in\n\
             *releases/latest*)\n\
             printf '%s' 'https://github.com/cli/cli/releases/tag/v{RELEASED}'\n\
             exit 0\n\
             ;;\n\
             *'/download/v{RELEASED}/gh_{RELEASED}_macOS_amd64.zip') ;;\n\
             *)\n\
             printf '%s\n' 'curl was asked for a release GitHub never named' >&2\n\
             exit 22\n\
             ;;\n\
             esac\n\
             out=\n\
             while [ $# -gt 0 ]; do\n\
             if [ \"$1\" = -o ]; then out=$2; fi\n\
             shift\n\
             done\n\
             '{cp}' '{zipped}' \"$out\"\n",
            cp = found("cp").display(),
            zipped = zipped.display(),
        ),
    );
}

/// What that zip holds, which is the shape GitHub's own release has: a
/// directory named for the version, with the binary at `bin/gh` inside it.
fn unpacked(dir: &Path) -> PathBuf {
    let unpacked = dir.join(format!("gh_{RELEASED}_macOS_amd64/bin"));
    std::fs::create_dir_all(&unpacked).unwrap();

    unpacked
        .parent()
        .expect("the release's directory")
        .to_owned()
}

/// And the same `curl` where the redirect answers and the download does not,
/// which is the Mac that cannot reach GitHub.
fn a_release_that_cannot_be_downloaded(dir: &Path) {
    program(
        &bin(dir).join("curl"),
        &format!(
            "#!/bin/sh\n\
             case \"$*\" in\n\
             *releases/latest*)\n\
             printf '%s' 'https://github.com/cli/cli/releases/tag/v{RELEASED}'\n\
             exit 0\n\
             ;;\n\
             esac\n\
             printf '%s\n' '{UNREACHABLE}' >&2\n\
             exit 7\n"
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
            ": > '{}' && apt-get update; apt-get install -y bubblewrap git nodejs npm && \
             npm install -g @openai/codex",
            started(&dialog),
        ),
        "one command, naming every package and the npm install on the same line, \
         with this archive's list brought up to date in front of it",
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

/// Ticking git and Claude Code on a Mac with Homebrew runs a `brew` line for
/// the one Homebrew carries and Anthropic's own installer for the other, both
/// as the user, and both rows go present.
///
/// **Nothing is raised at all.** Every package on a Mac is Homebrew's, Homebrew
/// refuses to run as root, and the prefix it installs into is already this
/// user's — so there is nothing here for a password dialog to be in front of,
/// and a run that put one there would be asking for a privilege to do something
/// that wants none. Anthropic's installer wants none either: it writes under
/// this user's home, which is the whole of why it is out of the dialog on every
/// platform but the one whose dialog keeps the same user.
#[tokio::test]
async fn a_mac_installs_its_packages_with_homebrew_and_claude_with_anthropics() {
    let (dir, pool) = ready().await;
    a_homebrew(dir.path());
    an_installer(dir.path());

    let dialog = Dialog::answered(dir.path(), Answer::Typed);
    let app = served_mac(dir.path(), &pool, Some(dialog.clone()));

    let ticked = [Dependency::Git, Dependency::Claude];

    let pressed = install(&app, &ticked).await;
    let run = pressed.run.expect("a press makes a run");

    assert_eq!(run.total, 2, "one unit per ticked row: {run:?}");

    let landed = over(&app).await;
    let run = landed.run.clone().expect("the run it was started with");

    assert_eq!(run.done, 2);
    assert_eq!(run.status, "Everything that was ticked is installed");

    assert!(
        dialog.commands().is_empty(),
        "a Mac raises nothing: {:?}",
        dialog.commands(),
    );

    for ticked in ticked {
        assert_eq!(install_state(&landed, ticked), &InstallState::Idle);
        assert_eq!(
            row(&landed, ticked).state,
            DependencyState::Present {
                at: Some(
                    match ticked {
                        Dependency::Git => bin(dir.path()).join("git"),
                        _ => local_bin(dir.path()).join("claude"),
                    }
                    .to_string_lossy()
                    .into_owned(),
                ),
                target: None,
            },
            "{ticked:?} is there, where what installed it puts it",
        );
    }

    assert!(
        landed.steps.dependencies,
        "the step the wizard was held on is met by what the run installed",
    );
}

/// And ticking Claude Code, OpenCode and git on an Intel Mac runs the two
/// vendors' own installers as the user, raises nothing at all, and sends the
/// row it has no command for to the hint screen with what to do about it.
///
/// **Which is the press that failed on the release.** This machine used to be
/// the other Mac: every ticked row was Homebrew's, so the run began by making
/// Homebrew's prefix behind the password dialog — `chmod /usr/local`, which no
/// Mac since Catalina allows — and a human watching it got
/// `Operation not permitted` and a hint screen telling them to run the
/// installer Homebrew refuses this machine with. There is no dialog here at
/// all now, and what the two units install is under this user's own home.
#[tokio::test]
async fn an_intel_mac_installs_with_the_vendors_own_and_raises_nothing() {
    let (dir, pool) = ready().await;
    an_installer(dir.path());

    let dialog = Dialog::answered(dir.path(), Answer::Typed);
    let app = served_intel_mac(dir.path(), &pool, Some(dialog.clone()));

    let ticked = [Dependency::Claude, Dependency::OpenCode, Dependency::Git];

    let pressed = install(&app, &ticked).await;
    let run = pressed.run.expect("a press makes a run");

    assert_eq!(
        run.total, 3,
        "every ticked row is a row of the run, the one nothing installs included: {run:?}",
    );

    let landed = over(&app).await;
    let run = landed.run.clone().expect("the run it was started with");

    assert_eq!(run.done, 3);
    assert_eq!(run.status, "1 of 3 could not be installed");

    assert!(
        dialog.commands().is_empty(),
        "an Intel Mac raises nothing: {:?}",
        dialog.commands(),
    );

    // The two rows a vendor installs are present where that vendor's own
    // installer puts them, both of them under this user's home.
    for (installed, at) in [
        (Dependency::Claude, local_bin(dir.path()).join("claude")),
        (
            Dependency::OpenCode,
            opencode_bin(dir.path()).join("opencode"),
        ),
    ] {
        assert_eq!(install_state(&landed, installed), &InstallState::Idle);
        assert_eq!(
            row(&landed, installed).state,
            DependencyState::Present {
                at: Some(at.to_string_lossy().into_owned()),
                target: None,
            },
            "{installed:?} is there, where its vendor's installer puts it",
        );
    }

    // And both directories are written down, so that the next start reads them
    // back and the next session's `PATH` leads with them — which for
    // `~/.opencode/bin` is the whole of how a session reaches it, it being on
    // no floor.
    assert_eq!(
        Settings::in_data_dir(dir.path()).config().session_path(),
        [
            local_bin(dir.path()).to_string_lossy().into_owned(),
            opencode_bin(dir.path()).to_string_lossy().into_owned(),
        ],
        "in the order the units ran",
    );

    // And the row nothing here installs is the hint screen's, in the words that
    // say what it wants: Apple's own dialog, which is on that Mac's screen
    // rather than a command Verkstead can run for anybody.
    assert!(
        !present(&landed, Dependency::Git),
        "nothing was installed for it: {landed:?}",
    );

    let InstallState::Failed { why } = install_state(&landed, Dependency::Git) else {
        panic!("the row carries why nothing was run for it: {landed:?}");
    };

    assert!(
        why.contains("xcode-select --install"),
        "which says what installs it: {why}",
    );
    assert!(
        !why.contains("brew") && !why.contains("Homebrew"),
        "and says nothing about a Homebrew this Mac cannot have: {why}",
    );
}

/// And ticking `gh` on that Mac unpacks GitHub's own release into the home's
/// `.local/bin`, raises nothing, and writes the directory down.
///
/// **The row the other tab installs with Homebrew**, which this one has none
/// of: `brew install gh` on an Intel Mac compiles from source under a Homebrew
/// whose installer refuses the machine, so what a tick runs here is the zip
/// every release carries. The version is the redirect's — the stub answers the
/// download for that version's URL and no other — so a `gh` in `~/.local/bin`
/// at the end of it is the version GitHub named a moment earlier.
#[tokio::test]
async fn an_intel_mac_unpacks_gh_out_of_githubs_release() {
    let (dir, pool) = ready().await;
    the_tools_a_download_wants(dir.path());
    a_release(dir.path());

    let dialog = Dialog::answered(dir.path(), Answer::Typed);
    let app = served_intel_mac(dir.path(), &pool, Some(dialog.clone()));

    install(&app, &[Dependency::Gh]).await;
    let landed = over(&app).await;

    assert!(
        dialog.commands().is_empty(),
        "a download under this user's own home asks nobody for a password: {:?}",
        dialog.commands(),
    );

    assert_eq!(install_state(&landed, Dependency::Gh), &InstallState::Idle);
    assert_eq!(
        row(&landed, Dependency::Gh).state,
        DependencyState::Present {
            at: Some(
                local_bin(dir.path())
                    .join("gh")
                    .to_string_lossy()
                    .into_owned(),
            ),
            target: None,
        },
        "the row is present at the next probe, where the release was unpacked",
    );

    // And the directory is written down, which is what puts it on the next
    // session's `PATH` — a Mac's floor carries it already, and `session_path`
    // is what every unit that lands somewhere writes.
    assert_eq!(
        Settings::in_data_dir(dir.path()).config().session_path(),
        [local_bin(dir.path()).to_string_lossy().into_owned()],
    );

    // And nothing of the unpacking is left behind: the zip was opened somewhere
    // temporary and what was copied out of it is the binary alone.
    assert_eq!(
        std::fs::read_dir(local_bin(dir.path()))
            .expect("the directory the release landed in")
            .count(),
        1,
        "the binary and nothing beside it",
    );
}

/// A download that could not be made fails the `gh` row in curl's own line and
/// leaves nothing in `~/.local/bin`.
///
/// **Which is the Mac that cannot reach GitHub**, and the row it leaves is the
/// hint screen's: the first line of what `curl` said goes under it, and the tab
/// beneath that carries the releases link for the human who will fetch the zip
/// by hand — see `web/src/setup/instructions.ts`.
///
/// **And the directory is untouched**, the zip having been opened somewhere
/// temporary rather than over a directory a session searches: a half-downloaded
/// `gh` on the `PATH` would be a row that went present over a program that
/// cannot run.
#[tokio::test]
async fn a_gh_download_that_failed_leaves_nothing_behind() {
    let (dir, pool) = ready().await;
    the_tools_a_download_wants(dir.path());
    a_release_that_cannot_be_downloaded(dir.path());

    let dialog = Dialog::answered(dir.path(), Answer::Typed);
    let app = served_intel_mac(dir.path(), &pool, Some(dialog.clone()));

    install(&app, &[Dependency::Gh]).await;
    let landed = over(&app).await;

    assert_eq!(
        install_state(&landed, Dependency::Gh),
        &InstallState::Failed {
            why: UNREACHABLE.to_owned(),
        },
        "the row carries the first line curl printed",
    );
    assert!(
        !present(&landed, Dependency::Gh),
        "and nothing was installed: {landed:?}",
    );

    assert!(
        !local_bin(dir.path()).join("gh").exists(),
        "nothing is left in ~/.local/bin",
    );
    assert!(
        Settings::in_data_dir(dir.path())
            .config()
            .session_path()
            .is_empty(),
        "and a directory nothing landed in is not written down",
    );
}

/// A vendor's installer that exited non-zero fails its own row in the line it
/// printed, and the rows of every other unit are untouched by it.
///
/// **And it was never elevated**: the dialog is handed the package manager and
/// nothing else, the installer running as the user beside it — which is what
/// keeps a harness that installs under `$HOME` out of root's home.
#[tokio::test]
async fn an_installer_that_failed_fails_its_own_row_and_no_other() {
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());
    an_installer_that_fails(dir.path());

    let dialog = Dialog::answered(dir.path(), Answer::Typed);
    let app = served(dir.path(), &pool, Some(dialog.clone()));

    install(
        &app,
        &[Dependency::Sandbox, Dependency::Git, Dependency::Claude],
    )
    .await;

    let landed = over(&app).await;

    assert_eq!(
        install_state(&landed, Dependency::Claude),
        &InstallState::Failed {
            why: UNREACHABLE.to_owned(),
        },
        "the row carries the first line the installer printed",
    );
    assert!(
        !present(&landed, Dependency::Claude),
        "and nothing was installed: {landed:?}",
    );

    for installed in [Dependency::Sandbox, Dependency::Git] {
        assert!(present(&landed, installed), "{installed:?}: {landed:?}");
        assert_eq!(
            install_state(&landed, installed),
            &InstallState::Idle,
            "{installed:?} is another unit's row and the failure is not its",
        );
    }

    assert_eq!(
        landed.run.expect("the run").status,
        "1 of 3 could not be installed",
    );

    assert_eq!(
        dialog.line(),
        format!(
            ": > '{}' && apt-get update; apt-get install -y bubblewrap git",
            started(&dialog),
        ),
        "one dialog, and the vendor's installer was not put behind it",
    );
}

/// A refresh that could not reach everything still leaves the install behind it
/// running.
///
/// **Which is the ordinary desktop.** `apt-get update` exits non-zero for one
/// unreachable source, and a machine carrying a third-party list that has gone
/// away is exactly that — so a refresh joined to the install with `&&` would
/// have refused, over one dead list, an install of packages the machine's own
/// archive still carries. It is joined with `;` instead, and the archive being
/// out of date is what the refresh is there for rather than what it is
/// permission for.
#[tokio::test]
async fn a_refresh_that_could_not_reach_everything_installs_anyway() {
    let (dir, pool) = ready().await;
    package_manager(dir.path(), ONE_SOURCE_GONE);

    let app = served(
        dir.path(),
        &pool,
        Some(Dialog::answered(dir.path(), Answer::Typed)),
    );

    install(&app, TICKED).await;

    let landed = over(&app).await;

    for ticked in TICKED {
        assert!(
            present(&landed, *ticked),
            "{ticked:?} should have been installed over a refresh that only \
             half worked: {landed:?}",
        );
    }

    assert_eq!(
        landed.run.expect("the run").status,
        "Everything that was ticked is installed",
    );
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

    // And one that is over with both kinds of trouble on it: a dialog somebody
    // dismissed, and a vendor's own installer that would not run. Which is the
    // hint screen's own reading — the rows that were ticked and are still
    // absent, each with a sentence under it.
    let (dir, pool) = ready().await;
    a_package_manager(dir.path());
    an_installer_that_fails(dir.path());

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
