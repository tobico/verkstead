//! The package stores, proved by really installing something.
//!
//! Every language Verkstead ships knowing about is a **descriptor** and nothing
//! else — YAML naming the variables that move a tool's store under the Build
//! Cache (ADR-0021). What a descriptor cannot say is whether the tool *reads*
//! the variable it names, and a suite asserting the environment would go on
//! passing while a release quietly renamed one. So each tool gets a proof here,
//! and a proof is an install.
//!
//! **What one proof does**, per tool:
//!
//! 1. Two Sandboxes, one per Conversation, against **one** Build Cache, each
//!    installing the same thing **at the same time**. Both succeeding is what
//!    says two sessions racing do not damage the store.
//! 2. A third Sandbox, on that same Build Cache, installing it again with the
//!    tool **denied its registry** — the offline flag the tool documents, and
//!    no bind of the registry either. An install that succeeds with nothing to
//!    fetch from succeeded out of the store.
//! 3. And the control: a fourth Sandbox, denied its registry the same way, on a
//!    Build Cache **nothing has filled**. It must fail. Without it, step 2 is a
//!    branch that would pass whatever the tool did with its variable — a build
//!    needing nothing looks exactly like a build served out of a store.
//!
//! *Found the store populated* is deliberately nowhere in that list. The store
//! is populated because the first two installs populated it, whether or not the
//! third read a byte of it.
//!
//! **The registry is this machine's own.** Nothing here reaches the internet: a
//! proof lays a package out on disk in whatever shape its tool fetches from —
//! for Go, a module proxy under a `file://` URL, which `go help goproxy` says
//! is a proxy like any other — and binds it read-only into the two Sandboxes
//! that are allowed it. So *denied its registry* is a fact about the sandbox as
//! well as about the flag.
//!
//! **A tool that is not installed is skipped in a line naming it**, so a
//! checkout run on a machine that only builds Rust stays green. Set
//! [`REQUIRED`] and a skip is a failure instead, which is what CI's own job
//! does: a tool cannot quietly leave the list.
//!
//! Linux only. A store is shared through a bind, and a bind is the Linux
//! sandbox's; what the other two platforms hand a session is asserted where
//! Rust's is — `tests/sandbox_macos.rs` and `tests/sessions_windows.rs`.

#![cfg(target_os = "linux")]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Bind, Closing, Executable, Homes, Reachable, Sandbox};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// What turns a skip into a failure: set wherever every one of these tools is
/// supposed to be installed, which is the CI job that installs them.
///
/// A variable of Verkstead's own rather than `CI`, because `CI` is set for the
/// Rust job too and that job installs none of this. What it guards is the one
/// thing a skip cannot say for itself: whether the tool being missing is a
/// developer's machine or a list somebody let rot.
const REQUIRED: &str = "VERKSTEAD_TEST_STORES";

/// Where the server each Conversation belongs to is listening. Nothing in this
/// suite asks it anything — an install is the whole of what runs inside — but a
/// sandbox is built around a reachable server either way.
const LISTENING: &str = "127.0.0.1:8422";

/// The shell every proof's script is written in, and the one path in a sandbox
/// guaranteed to hold one.
const SH: &str = "/bin/sh";

/// What stands in for the server's own image, which every sandbox is equipped
/// with — see `tests/sandbox.rs`, where the same stub is argued.
const SAYS_WHICH_BUILD: &str = "#!/bin/sh\nprintf 'verkstead 0.0.0-the-servers-own\\n'\n";

/// The machine a proof runs on: one Repo, one Build Cache and as many
/// Conversations as the proof stands Sandboxes up for, each with a Worktree git
/// itself made.
///
/// Everything is real, for the reason `tests/sandbox.rs` gives: what a sandbox
/// binds is read off the record and the checkout, and a fixture that hand-built
/// the paths would be proving the harness works rather than the server.
struct Machine {
    /// Kept alive for as long as the fixture is: the directories go when these
    /// drop, and a Worktree that vanished mid-install would fail obscurely.
    _elsewhere: tempfile::TempDir,
    state: tempfile::TempDir,
    _home: tempfile::TempDir,

    /// Where a proof lays its registry out. Outside every directory a sandbox
    /// binds of its own accord, so that a Sandbox reaches it only where the
    /// proof hands it the bind.
    registries: PathBuf,

    /// One `Homes` for the whole fixture, the way the server has one: two
    /// sandboxes asking separately would get two registers of what is running
    /// in which root.
    homes: Homes,

    conversations: Vec<store::Conversation>,
    profile: store::Profile,

    skills: Skills,
    verkstead: Executable,
    handoffs: Handoffs,
    attachments: Attachments,
    settings: Settings,
}

impl Machine {
    /// The Build Cache every Sandbox shares — the one directory a store goes
    /// under, and the whole of what makes an install a second Conversation's
    /// as well as the first's.
    ///
    /// No sccache: nothing here compiles Rust, and a Compile Server started
    /// under a suite that does not need one is a process to clean up.
    fn cache(&self) -> BuildCache {
        self.cache_named("build-cache")
    }

    /// And one nothing has ever filled, which is what the control installs
    /// against.
    fn empty_cache(&self) -> BuildCache {
        self.cache_named("nothing-in-here")
    }

    fn cache_named(&self, name: &str) -> BuildCache {
        let dir = self.state.path().join(name);
        std::fs::create_dir_all(&dir).expect("a Build Cache directory to hand out");

        BuildCache::at(dir, None, self.state.path().to_owned())
    }

    /// The `n`th Conversation's Worktree, which is where its install runs: the
    /// sandbox chdirs into it.
    fn worktree(&self, n: usize) -> &Path {
        self.conversations[n]
            .worktree
            .as_deref()
            .expect("a grilling Conversation has a Worktree")
    }

    /// The sandbox the `n`th Conversation's session would run in, against
    /// `cache`, reaching whatever `extra` opens for it.
    fn sandbox(&self, n: usize, cache: &BuildCache, extra: Vec<Bind>) -> Sandbox {
        Sandbox::for_conversation(
            &self.conversations[n],
            &self.profile,
            &self.homes,
            &Reachable::at(LISTENING.parse().expect("a socket address")),
            &self.skills,
            &self.verkstead,
            &self.handoffs,
            &self.attachments,
            &self.settings.secrets(),
            &self.settings.config(),
            cache,
            extra,
        )
        .expect("a grilling Conversation has a Worktree to build a sandbox around")
    }
}

/// Stand one up, with `conversations` Conversations on the one Repo.
async fn machine(conversations: usize) -> Machine {
    let elsewhere = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let repo = repository(elsewhere.path().join("verkstead"));

    let pool = store::open_database(&state.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo_row = store::register_repo(&pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("the Repo registers");

    // The account a session is given, which a sandbox joins into its root — so
    // it has to be there even though nothing here logs in anywhere.
    let claude_dir = elsewhere.path().join("account/.claude");
    let config_file = elsewhere.path().join("account/.claude.json");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(claude_dir.join("settings.json"), "{}\n").unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    let profile = store::create_profile(
        &pool,
        &store::ProfileFacts {
            name: Some("work".to_owned()),
            account: store::Account::Claude {
                claude_dir,
                config_file,
            },
            models: vec!["claude-opus-5".to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap()
    .expect("the Profile saves");

    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let mut rows = Vec::new();

    for nth in 0..conversations {
        let branch = format!("installing-{nth}");

        let id = store::start_conversation(&pool, repo_row.id, &branch)
            .await
            .unwrap()
            .expect("the Conversation starts");

        store::set_grilling_pairing(&pool, id, profile.id, profile.model())
            .await
            .unwrap();
        store::set_implementation_pairing(&pool, id, profile.id, profile.model())
            .await
            .unwrap();

        let worktree = state.path().join(format!("worktrees/verkstead-{branch}"));
        std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                &worktree.to_string_lossy(),
                &commit,
            ],
        );

        store::start_grilling(&pool, id, &commit, &worktree, &[])
            .await
            .unwrap();

        rows.push(
            store::load_conversation(&pool, id)
                .await
                .unwrap()
                .expect("the Conversation is there"),
        );
    }

    let image = state.path().join("image/verkstead");
    std::fs::create_dir_all(image.parent().unwrap()).unwrap();
    std::fs::write(&image, SAYS_WHICH_BUILD).unwrap();
    std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o755)).unwrap();

    let registries = elsewhere.path().join("registries");
    std::fs::create_dir_all(&registries).unwrap();

    Machine {
        homes: Homes::on(Platform::HERE, home.path().to_owned(), state.path()),
        skills: Skills::installed(Platform::HERE, state.path())
            .expect("this binary carries skills"),
        verkstead: Executable::at(Platform::HERE, image, state.path())
            .expect("the executable was just written"),
        handoffs: Handoffs::under(state.path()),
        attachments: Attachments::under(state.path()),
        settings: Settings::in_data_dir(state.path()),
        conversations: rows,
        profile,
        registries,
        _elsewhere: elsewhere,
        state,
        _home: home,
    }
}

/// A git repository at `path`, with one commit on `main`.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "local@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Whatever The Repo Says"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "README.md"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("git should be on the PATH for these tests");

    assert!(
        output.status.success(),
        "git {args:?} failed in {}",
        dir.display()
    );

    String::from_utf8(output.stdout).unwrap()
}

/// What an install said for itself: whether it worked, and everything it
/// printed, which is what a failure has to be explained by.
struct Ran {
    worked: bool,
    said: String,
}

impl Ran {
    /// The one assertion an install that was supposed to work gets, with what
    /// it printed as the reason.
    fn worked(&self, what: &str) {
        assert!(self.worked, "{what}, and it said:\n{}", self.said);
    }
}

/// Start `script` inside `sandbox` without waiting for it, which is how two
/// installs come to be running at once.
///
/// The `Closing` is what a session's ending is left to see to, and it is held
/// alongside the child rather than dropped here: what is running is running
/// until the install is over.
fn starting(sandbox: &Sandbox, script: &str) -> (Child, Closing) {
    let (rendering, closing) = sandbox
        .command(&[SH, "-c", script])
        .expect("a rendering on a platform with no identity to make");

    let child = Command::try_from(&rendering)
        .expect("a rendering with no container")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bwrap should be on the PATH: the dev shell declares bubblewrap");

    (child, closing)
}

/// And what it came to once it finished.
fn finished((child, closing): (Child, Closing)) -> Ran {
    let output = child
        .wait_with_output()
        .expect("the install to be waited on");

    drop(closing);

    Ran {
        worked: output.status.success(),
        said: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}

/// One install, run to the end.
fn installing(sandbox: &Sandbox, script: &str) -> Ran {
    finished(starting(sandbox, script))
}

/// Where `program` is on the host, absolute, or `None` where this machine has
/// none.
///
/// Absolute because a sandbox's `PATH` is the machine's system profile rather
/// than the shell the tests were started from, and a tool the suite found on
/// its own `PATH` is one it has to name in full to reach inside.
fn found(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
}

/// Every tool a proof needs, or `None` and a line naming the first one this
/// machine has not got.
///
/// Where [`REQUIRED`] is set, a missing tool is a failure instead: that is the
/// job whose whole business is having them installed, and a proof quietly
/// skipped there would be a tool that had left the list without anybody
/// hearing about it.
fn tools(proof: &str, wanted: &[&str]) -> Option<Vec<PathBuf>> {
    let mut found_them = Vec::new();

    for program in wanted {
        let Some(path) = found(program) else {
            println!(
                "skipping the {proof} package-store proof: this machine has no `{program}` on \
                 its PATH"
            );

            assert!(
                std::env::var_os(REQUIRED).is_none(),
                "the {proof} package-store proof was skipped for want of `{program}`, and \
                 {REQUIRED} is set — which is the run where every tool on the list has to be \
                 installed. Install it in that job, or take {proof} off the list.",
            );

            return None;
        };

        found_them.push(path);
    }

    Some(found_them)
}

/// What every Go command in this suite is told, beside the two variables the
/// descriptor sets — which are the two this is here to prove.
///
/// Each of these takes something out of the way rather than moving a store:
/// `GOTOOLCHAIN=local` so that a build never fetches a compiler, `GOSUMDB=off`
/// and `GOFLAGS=-mod=mod` so that a module signed by nobody is one this
/// registry can serve, `GOENV=off` so that a session's own `go env` file is not
/// read or written, `GOWORK=off` so that nothing above the Worktree is joined
/// in, and `CGO_ENABLED=0` so that a build wants no C toolchain.
const GO_SETTINGS: &str = "export GOTOOLCHAIN=local GOSUMDB=off GOFLAGS=-mod=mod \
                           GOENV=off GOWORK=off CGO_ENABLED=0";

/// The module every Go proof fetches, and the version of it.
const GO_MODULE: &str = "example.test/greet";
const GO_VERSION: &str = "v1.0.0";

/// Lay a Go module proxy out under `at`, holding [`GO_MODULE`] — the shape
/// `go help goproxy` describes, which is why a directory reached over `file://`
/// is a registry like any other: "even a site serving from a fixed file system
/// (including a file:/// URL) can be a module proxy".
///
/// The zip is what the proxy serves as the module itself, and its entries have
/// to be under `<module>@<version>/` and nothing else — `-D` is what keeps the
/// directory entries out, which `go` refuses the archive for.
fn go_proxy(at: &Path, zip: &Path) {
    let versions = at.join(GO_MODULE).join("@v");
    std::fs::create_dir_all(&versions).unwrap();

    let go_mod = format!("module {GO_MODULE}\n\ngo 1.21\n");

    std::fs::write(versions.join("list"), format!("{GO_VERSION}\n")).unwrap();
    std::fs::write(
        versions.join(format!("{GO_VERSION}.info")),
        format!("{{\"Version\":\"{GO_VERSION}\",\"Time\":\"2026-01-01T00:00:00Z\"}}\n"),
    )
    .unwrap();
    std::fs::write(versions.join(format!("{GO_VERSION}.mod")), &go_mod).unwrap();

    let inside = at.join("what-goes-in-the-zip");
    let package = inside.join(format!("{GO_MODULE}@{GO_VERSION}"));
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("go.mod"), &go_mod).unwrap();
    std::fs::write(
        package.join("greet.go"),
        "package greet\n\nfunc Hello() string { return \"out of the store\" }\n",
    )
    .unwrap();

    let archive = versions.join(format!("{GO_VERSION}.zip"));
    let made = Command::new(zip)
        .args(["-q", "-r", "-X", "-D"])
        .arg(&archive)
        .arg(GO_MODULE.split('/').next().expect("a module path"))
        .current_dir(&inside)
        .stdin(Stdio::null())
        .status()
        .expect("the suite's own `zip`");

    assert!(made.success(), "the module zip was not built");
}

/// And what a Conversation's Worktree holds: a module that imports the one in
/// the proxy, so that building it is a fetch.
fn go_consumer(worktree: &Path) {
    std::fs::write(
        worktree.join("go.mod"),
        format!("module example.test/app\n\ngo 1.21\n\nrequire {GO_MODULE} {GO_VERSION}\n"),
    )
    .unwrap();
    std::fs::write(
        worktree.join("main.go"),
        format!(
            "package main\n\nimport \"{GO_MODULE}\"\n\nfunc main() {{ println(greet.Hello()) }}\n"
        ),
    )
    .unwrap();
}

/// Go: two Sandboxes building one module at once against one store, a third
/// building it with `GOPROXY=off` and no proxy in reach, and the control that
/// says the third proved something.
///
/// `GOPROXY=off` is the flag Go documents for exactly this — no module
/// downloads are allowed, and only what is already in the module cache may be
/// used — so a build that succeeds under it is a build served out of
/// `GOMODCACHE`. Which is the variable the descriptor sets, and the whole of
/// what this is asking.
///
/// **The two at once are the other half.** Go's module cache is documented as
/// safe for several commands at once; what says it is safe *here* is two
/// sessions in two sandboxes writing one directory through two binds, and a
/// third session getting a usable store out of what they left.
#[tokio::test]
async fn two_go_builds_at_once_fill_one_store_and_a_third_builds_out_of_it() {
    let Some(tools) = tools("Go", &["go", "zip"]) else {
        return;
    };
    let (go, zip) = (&tools[0], &tools[1]);

    // Four: the two that race, the one denied its registry, and the control.
    let machine = machine(4).await;
    let cache = machine.cache();

    let proxy = machine.registries.join("go");
    go_proxy(&proxy, zip);

    for nth in 0..4 {
        go_consumer(machine.worktree(nth));
    }

    // The registry, read-only and only where a proof is allowed it. The two
    // that race have it; the two that follow do not, so *denied its registry*
    // is a fact about the sandbox and not only about the flag.
    let reaching = || vec![Bind::readable(proxy.clone())];

    let building = format!(
        "set -e\n{GO_SETTINGS}\nexport GOPROXY=file://{proxy}\n{go} build -o ./built ./...\n",
        proxy = proxy.display(),
        go = go.display(),
    );

    // Started together and waited on together, which is the only way the two
    // are ever really in the store at the same moment.
    let first = starting(&machine.sandbox(0, &cache, reaching()), &building);
    let second = starting(&machine.sandbox(1, &cache, reaching()), &building);

    finished(first).worked("the first session's build fills the store");
    finished(second).worked("and the second one racing it finishes just as well");

    // And the proof. No proxy bound, and the flag that says not to look for
    // one: what is left to build out of is the store the two above filled.
    let offline = format!(
        "set -e\n{GO_SETTINGS}\nexport GOPROXY=off\n{go} build -o ./built ./...\n./built\n",
        go = go.display(),
    );

    let third = installing(&machine.sandbox(2, &cache, vec![]), &offline);

    third.worked(
        "a third session builds the same module with its registry denied, which it can only \
         do out of the shared module cache",
    );
    assert!(
        third.said.contains("out of the store"),
        "and what it built really runs, so the store held the source rather than \
         something shaped like it. It said:\n{}",
        third.said,
    );

    // The control. Everything the same but the Build Cache, which nothing has
    // filled — so a pass here would mean the build above needed no store at
    // all, and the proof was asserting nothing.
    let control = installing(
        &machine.sandbox(3, &machine.empty_cache(), vec![]),
        &offline,
    );

    assert!(
        !control.worked,
        "an empty store and no registry has to fail, or the build above proved \
         nothing about either. It said:\n{}",
        control.said,
    );
    assert!(
        control.said.contains("GOPROXY=off"),
        "and it fails for want of the registry rather than for some other \
         reason. It said:\n{}",
        control.said,
    );
}

/// And the environment those installs ran in was the descriptor's, rather than
/// anything this suite set: the two variables point into the one Build Cache,
/// and a session can write there.
///
/// Beside the proof rather than inside it, because it is the cheap half: what
/// a session is *told* is asserted on all three platforms in the sandbox
/// suites, and what a tool *does* with it is what the install above is for.
/// This is here so that a Go proof skipped for want of `go` still leaves
/// something in this file that ran.
#[tokio::test]
async fn a_session_is_given_gos_two_directories_inside_the_one_build_cache() {
    let machine = machine(1).await;
    let cache = machine.cache();
    let sandbox = machine.sandbox(0, &cache, vec![]);

    let dir = cache.dir().expect("the fixture's cache has a directory");

    let reported = installing(
        &sandbox,
        "set -e\n\
         printf 'gomodcache=%s\\n' \"${GOMODCACHE-unset}\"\n\
         printf 'gocache=%s\\n' \"${GOCACHE-unset}\"\n\
         mkdir -p \"$GOMODCACHE\" \"$GOCACHE\"\n\
         : > \"$GOMODCACHE/written-from-inside\"\n",
    );

    reported.worked("a session can make and write both of the directories it is pointed at");

    for (name, under) in [("gomodcache", "go/mod"), ("gocache", "go/build")] {
        assert!(
            reported
                .said
                .contains(&format!("{name}={}\n", dir.join(under).display())),
            "{name} is {under} inside the one Build Cache. The session said:\n{}",
            reported.said,
        );
    }

    assert!(
        dir.join("go/mod/written-from-inside").is_file(),
        "and what the session wrote is on the host, in the directory the next \
         Conversation's session will be given"
    );
}
