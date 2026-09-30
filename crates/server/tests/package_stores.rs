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
//! proof lays a package out in whatever shape its tool fetches from, and hands
//! it to the two Sandboxes that are allowed it and to no others. So *denied its
//! registry* is a fact about the machine as well as about the flag — and which
//! fact depends on what the tool can read a registry off:
//!
//! - Go's is a module proxy under a `file://` URL, which `go help goproxy` says
//!   is a proxy like any other, **bound read-only** into the Sandboxes that may
//!   have it and into no others.
//! - Nothing in the npm ecosystem reads a registry off the disk, so the
//!   JavaScript proofs really serve one, over the loopback and out of this
//!   process — see [`Registry`], whose `shut` is what denies it: the port stops
//!   answering before the install that must not reach it starts.
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

use std::net::Ipv4Addr;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use axum::Router;
use axum::extract::State;
use axum::http::header;
use axum::routing::get;

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
        self.cache_named("shared")
    }

    /// And one nothing has ever filled, which is what the control installs
    /// against.
    fn empty_cache(&self) -> BuildCache {
        self.cache_named("nothing-in-here")
    }

    /// Where a store that is not under the Build Cache goes, for the cache of
    /// that name — pnpm's, and every later `{stores}` one.
    fn stores_beside(&self, name: &str) -> PathBuf {
        verkstead_server::languages::stores(&self.state.path().join(name))
    }

    /// One Build Cache, and **a Data Directory of its own beside it**.
    ///
    /// The second half is what the control rests on. A store under `{stores}`
    /// is a directory of the Data Directory rather than of the cache — see
    /// [`verkstead_server::languages::stores`] — so two Build Caches sharing
    /// one Data Directory would be two caches sharing one pnpm store, and the
    /// control would install out of the very store the proof had just filled.
    ///
    /// `opening` is what makes the second directory, rather than this: it is
    /// what a session spawn calls, so the directory the proofs bind is the one
    /// the server would have made.
    fn cache_named(&self, name: &str) -> BuildCache {
        let data_dir = self.state.path().join(name);
        let dir = data_dir.join("cache");

        std::fs::create_dir_all(&dir).expect("a Build Cache directory to hand out");

        let cache = BuildCache::at(dir, None, data_dir);
        cache.opening(&self.settings.config());

        cache
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

// ---------------------------------------------------------------------------
// Node: npm, pnpm, and the two yarns.
// ---------------------------------------------------------------------------

/// The package every JavaScript proof installs, and the version of it.
const NPM_PACKAGE: &str = "greet-from-the-store";
const NPM_VERSION: &str = "1.0.0";

/// What that package says when it is required, which is how a proof says the
/// store held the code rather than something shaped like it.
const OUT_OF_THE_STORE: &str = "out of the store";

/// One npm registry, on a loopback port, serving [`NPM_PACKAGE`].
///
/// `go help goproxy` let the Go proof lay its registry out as a directory and
/// reach it over `file://`. Nothing in the npm ecosystem reads a registry off
/// the disk, so this one is really served — over the loopback, out of this
/// process, which is as far from the internet as that proxy was.
/// **Served from a thread of its own**, with a runtime of its own on it. An
/// install is waited on by blocking — see [`finished`] — and a proof runs
/// several of those; a registry sharing the test's runtime would be a registry
/// nothing polled while any of them was outstanding, which is an install
/// waiting on a fetch that will never be answered.
struct Registry {
    url: String,

    /// Both taken by [`Registry::close`], so that shutting it twice — once
    /// asked for and once on the way out of a proof that panicked — does it
    /// once.
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    serving: Option<std::thread::JoinHandle<()>>,
}

impl Registry {
    /// Take it off the air, which is the other half of *denied its registry*:
    /// the flag says not to go looking, and this says there is nothing to find.
    ///
    /// Waited on rather than dropped and forgotten, so that the port has
    /// stopped answering before the install that must not reach it starts.
    /// Where the Go proof withheld a bind, this withholds the listener — a
    /// shared store is reached through a bind and a registry over the network,
    /// so denying each one is a different act.
    fn shut(mut self) {
        self.close();
    }

    fn close(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }

        if let Some(serving) = self.serving.take() {
            serving.join().expect("the registry's own thread");
        }
    }
}

impl Drop for Registry {
    /// A proof that panicked before it shut one leaves no thread behind, so
    /// the failure a run reports is the assertion rather than a suite that
    /// would not finish.
    fn drop(&mut self) {
        self.close();
    }
}

/// What the registry has to answer with, which is fixed once it is listening.
#[derive(Clone)]
struct Published {
    packument: String,
    tarball: Vec<u8>,
}

/// Lay [`NPM_PACKAGE`] out under `at` and serve it.
///
/// Two routes, which is the whole of the protocol an install of one package
/// uses: the **packument** at `/<name>`, and the tarball its `dist.tarball`
/// points at. `dist.shasum` is published with it because Yarn Classic refuses a
/// version that has none — npm, pnpm and Berry each record an integrity of
/// their own out of what they were handed — and a sha1 of the tarball is what a
/// registry puts there.
///
/// The archive's entries have to be under `package/` and nothing else, which is
/// the shape `npm pack` writes and every one of these four unpacks.
fn npm_registry(at: &Path, tar: &Path, sha1sum: &Path) -> Registry {
    let inside = at.join("package");
    std::fs::create_dir_all(&inside).unwrap();

    std::fs::write(
        inside.join("package.json"),
        format!(
            "{{\"name\":\"{NPM_PACKAGE}\",\"version\":\"{NPM_VERSION}\",\
              \"main\":\"index.js\"}}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        inside.join("index.js"),
        format!("module.exports = '{OUT_OF_THE_STORE}';\n"),
    )
    .unwrap();

    let archive = at.join("published.tgz");
    let made = Command::new(tar)
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(at)
        .arg("package")
        .stdin(Stdio::null())
        .status()
        .expect("the suite's own `tar`");

    assert!(made.success(), "the package tarball was not built");

    let summed = Command::new(sha1sum)
        .arg(&archive)
        .stdin(Stdio::null())
        .output()
        .expect("the suite's own `sha1sum`");

    assert!(summed.status.success(), "the tarball could not be summed");

    let shasum = String::from_utf8(summed.stdout)
        .expect("a sum is ASCII")
        .split_whitespace()
        .next()
        .expect("sha1sum prints the sum first")
        .to_owned();

    // Bound here rather than on the thread below, because the port it gets is
    // what goes in the packument — so it has to be known before anything is
    // served, and before the caller is handed the URL.
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .expect("a loopback port for the registry");
    let url = format!("http://{}", listener.local_addr().expect("the port it got"));

    listener
        .set_nonblocking(true)
        .expect("what tokio takes a standard listener over");

    let published = Published {
        packument: format!(
            "{{\"name\":\"{NPM_PACKAGE}\",\"dist-tags\":{{\"latest\":\"{NPM_VERSION}\"}},\
              \"versions\":{{\"{NPM_VERSION}\":{{\"name\":\"{NPM_PACKAGE}\",\
              \"version\":\"{NPM_VERSION}\",\"main\":\"index.js\",\"dist\":{{\
              \"shasum\":\"{shasum}\",\
              \"tarball\":\"{url}/{NPM_PACKAGE}/-/{NPM_PACKAGE}-{NPM_VERSION}.tgz\"}}}}}}}}"
        ),
        tarball: std::fs::read(&archive).unwrap(),
    };

    let app = Router::new()
        .route(
            &format!("/{NPM_PACKAGE}"),
            get(|State(served): State<Published>| async move {
                (
                    [(header::CONTENT_TYPE, "application/json")],
                    served.packument,
                )
            }),
        )
        .route(
            &format!("/{NPM_PACKAGE}/-/{{tarball}}"),
            get(|State(served): State<Published>| async move {
                (
                    [(header::CONTENT_TYPE, "application/octet-stream")],
                    served.tarball,
                )
            }),
        )
        .with_state(published);

    let (stop, stopping) = tokio::sync::oneshot::channel();

    let serving = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for the registry's own thread");

        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener)
                .expect("the listener this thread was handed");

            // Selected on rather than shut down gracefully: what is being
            // asked for is a port that has stopped answering, and a graceful
            // shutdown waits on connections an installer may have left open.
            tokio::select! {
                served = axum::serve(listener, app) => { let _ = served; }
                _ = stopping => {}
            }
        });
    });

    Registry {
        url,
        stop: Some(stop),
        serving: Some(serving),
    }
}

/// And what a Conversation's Worktree holds: a manifest asking for the package
/// in the registry, so that installing it is a fetch.
fn npm_consumer(worktree: &Path) {
    std::fs::write(
        worktree.join("package.json"),
        format!(
            "{{\n  \"name\": \"app\",\n  \"version\": \"1.0.0\",\n  \"private\": true,\n  \
              \"dependencies\": {{ \"{NPM_PACKAGE}\": \"{NPM_VERSION}\" }}\n}}\n"
        ),
    )
    .unwrap();
}

/// What one JavaScript tool's install proof needs saying about it. Everything
/// else about it is the four Sandboxes below, which are the same for all four.
struct Installs {
    /// What the skip line names and the assertions call it.
    tool: &'static str,

    /// The programs it takes, in no particular order, each of them standing in
    /// the scripts below as `{its-own-name}` — the tool itself, `node` where
    /// the proof runs what it installed, and `tar` and `sha1sum`, which are
    /// what lay the registry out.
    wants: &'static [&'static str],

    /// The lockfile the first install writes, which a Repo would have
    /// committed: the two installs after it are handed a copy, which is what a
    /// second Conversation on a checked-out Repo really starts from.
    lockfile: &'static str,

    /// What every install of this tool is told, beside the store variables the
    /// descriptor sets — which are the ones this is here to prove. Each of
    /// these takes something out of the way rather than moving a store.
    settings: &'static str,

    /// The install that fills the store.
    filling: &'static str,

    /// And the same install with the tool **denied its registry**: the offline
    /// switch this tool documents, run against a registry that has been taken
    /// off the air.
    offline: &'static str,

    /// What runs the package that was installed.
    running: &'static str,

    /// And what the control's failure has to say, so that what it failed over
    /// was the denial rather than anything else.
    denied: &'static str,
}

/// One tool's proof: two installs at once against one store, a third out of
/// what they left with the registry gone, and the control that says the third
/// proved something.
///
/// The same four Sandboxes as the Go proof above, and the same reasoning. What
/// is different is only that the registry is served rather than bound, so
/// *denied its registry* is [`Registry::shut`] rather than a bind withheld.
async fn one_javascript_tools_store(proof: Installs) {
    let Some(found_them) = tools(proof.tool, proof.wants) else {
        return;
    };

    // Every program this proof names, standing in its scripts under its own
    // name: absolute, for [`found`]'s reason.
    let named = |script: &str, registry: &str| {
        let mut said = script.replace("{registry}", registry);

        for (program, path) in proof.wants.iter().zip(&found_them) {
            said = said.replace(&format!("{{{program}}}"), &path.display().to_string());
        }

        said
    };

    // Four: the two that race, the one denied its registry, and the control.
    let machine = machine(4).await;
    let cache = machine.cache();

    let registry = npm_registry(
        &machine.registries.join(proof.tool),
        &found_them[proof
            .wants
            .iter()
            .position(|program| *program == "tar")
            .expect("every JavaScript proof lays its registry out with `tar`")],
        &found_them[proof
            .wants
            .iter()
            .position(|program| *program == "sha1sum")
            .expect("and sums it with `sha1sum`")],
    );

    for nth in 0..4 {
        npm_consumer(machine.worktree(nth));
    }

    // Held rather than read off the registry, so that the scripts outlive
    // taking it off the air: the install that must not reach it is still told
    // where it was.
    let url = registry.url.clone();

    let script = |install: &str| {
        named(
            &format!(
                "set -e\n{settings}\n{install}\n{running}\n",
                settings = proof.settings,
                running = proof.running,
            ),
            &url,
        )
    };

    let filling = script(proof.filling);

    // Started together and waited on together, which is the only way the two
    // are ever really writing the store at the same moment.
    let first = starting(&machine.sandbox(0, &cache, vec![]), &filling);
    let second = starting(&machine.sandbox(1, &cache, vec![]), &filling);

    let first = finished(first);
    let second = finished(second);

    first.worked(&format!(
        "the first session's {} install fills the store",
        proof.tool
    ));
    second.worked("and the second one racing it finishes just as well");

    // The lockfile the first one wrote, carried to the two that follow the way
    // a Repo carries one: committed, and checked out into every Worktree.
    let written = machine.worktree(0).join(proof.lockfile);
    let lock = std::fs::read(&written).unwrap_or_else(|error| {
        panic!(
            "{} should have written {} ({error}), and it said:\n{}",
            proof.tool,
            written.display(),
            first.said,
        )
    });

    for nth in 2..4 {
        std::fs::write(machine.worktree(nth).join(proof.lockfile), &lock).unwrap();
    }

    // And the proof. The registry stops answering and the tool is told not to
    // look for one: what is left to install out of is the store the two above
    // filled.
    registry.shut();

    let offline = script(proof.offline);
    let third = installing(&machine.sandbox(2, &cache, vec![]), &offline);

    third.worked(&format!(
        "a third session installs the same package with {}'s registry denied, \
         which it can only do out of the shared store",
        proof.tool,
    ));
    assert!(
        third.said.contains(OUT_OF_THE_STORE),
        "and what it installed really runs, so the store held the package \
         rather than something shaped like it. It said:\n{}",
        third.said,
    );

    // The control. Everything the same but the Build Cache and the directory
    // beside its Worktrees, neither of which anything has filled — so a pass
    // here would mean the install above needed no store at all, and the proof
    // was asserting nothing.
    let control = installing(
        &machine.sandbox(3, &machine.empty_cache(), vec![]),
        &offline,
    );

    assert!(
        !control.worked,
        "an empty store and no registry has to fail, or the install above \
         proved nothing about either. It said:\n{}",
        control.said,
    );
    assert!(
        control.said.contains(proof.denied),
        "and it fails for want of the registry rather than for some other \
         reason: `{}` is what {} says about that. It said:\n{}",
        proof.denied,
        proof.tool,
        control.said,
    );
}

/// npm: `NPM_CONFIG_CACHE`, and `--offline`, which npm documents as "force
/// offline mode: no network requests will be done during install".
///
/// The control fails `ENOTCACHED` — npm's own word for a request it was told to
/// serve out of the cache and could not — which is what says the third install
/// was served out of the cache rather than needing nothing.
#[tokio::test]
async fn npm_fills_one_cache_and_a_third_install_reads_it() {
    one_javascript_tools_store(Installs {
        tool: "npm",
        wants: &["npm", "node", "tar", "sha1sum"],
        lockfile: "package-lock.json",
        settings: "",
        filling: "{npm} install --registry={registry} --no-audit --no-fund",
        offline: "{npm} install --offline --registry={registry} --no-audit --no-fund",
        running: "{node} -e \"process.stdout.write(require('greet-from-the-store'))\"",
        denied: "ENOTCACHED",
    })
    .await;
}

/// pnpm: `PNPM_CONFIG_STORE_DIR` and `PNPM_CONFIG_CACHE_DIR`, and `--offline`.
///
/// **Both variables, because an install needs both halves.** The control fails
/// `ERR_PNPM_NO_OFFLINE_META` rather than for want of the packages: pnpm
/// resolves out of the metadata under `cache-dir` before it reaches for the
/// content under `store-dir`, so a session given one and not the other installs
/// nothing offline.
#[tokio::test]
async fn pnpm_fills_one_store_and_a_third_install_reads_it() {
    one_javascript_tools_store(Installs {
        tool: "pnpm",
        wants: &["pnpm", "node", "tar", "sha1sum"],
        lockfile: "pnpm-lock.yaml",
        settings: "",
        filling: "{pnpm} install --registry={registry}",
        offline: "{pnpm} install --offline --registry={registry}",
        running: "{node} -e \"process.stdout.write(require('greet-from-the-store'))\"",
        denied: "ERR_PNPM_NO_OFFLINE_META",
    })
    .await;
}

/// Yarn Classic — yarn 1.x, still what `yarn` is on most machines:
/// `YARN_CACHE_FOLDER`, and `--offline`.
///
/// `--no-default-rc` so that what moved the cache was the variable the
/// descriptor set and nothing a `.yarnrc` anywhere said.
#[tokio::test]
async fn yarn_classic_fills_one_cache_and_a_third_install_reads_it() {
    one_javascript_tools_store(Installs {
        tool: "yarn",
        wants: &["yarn", "node", "tar", "sha1sum"],
        lockfile: "yarn.lock",
        settings: "",
        filling: "{yarn} install --no-default-rc --registry={registry}",
        offline: "{yarn} install --no-default-rc --offline --registry={registry}",
        running: "{node} -e \"process.stdout.write(require('greet-from-the-store'))\"",
        denied: "Can't make a request in offline mode",
    })
    .await;
}

/// Yarn Berry — yarn 2 and up: `YARN_GLOBAL_FOLDER`, and `enableNetwork`,
/// which Berry documents as "if false, Yarn will never make any request to the
/// network".
///
/// **`YARN_CACHE_FOLDER` is not the variable here**, which is the trap this
/// proof is what catches. `enableGlobalCache` is on by default in Yarn 4, and
/// Berry then disregards `cacheFolder` and keeps the cache in
/// `globalFolder/cache` — so a session given only Classic's variable would go
/// on downloading into the empty HOME every session gets, and every install
/// proof but this one would still pass.
///
/// Three settings beside it, none of them a store: telemetry off so that
/// nothing here talks to anybody, immutable installs off because the first
/// install is the one that writes the lockfile, and the loopback whitelisted
/// because Berry refuses a plain-HTTP registry that has not been named.
///
/// It runs what it installed through `yarn node` rather than `node`: Berry's
/// default linker is Plug'n'Play, which resolves out of the zip in the cache
/// through a runtime of its own rather than out of a `node_modules`.
#[tokio::test]
async fn yarn_berry_fills_one_cache_and_a_third_install_reads_it() {
    one_javascript_tools_store(Installs {
        tool: "yarn-berry",
        wants: &["yarn-berry", "tar", "sha1sum"],
        lockfile: "yarn.lock",
        settings: "export YARN_ENABLE_TELEMETRY=0 YARN_ENABLE_IMMUTABLE_INSTALLS=false \
                   YARN_UNSAFE_HTTP_WHITELIST=127.0.0.1 \
                   YARN_NPM_REGISTRY_SERVER={registry}",
        filling: "{yarn-berry} install",
        offline: "YARN_ENABLE_NETWORK=false {yarn-berry} install",
        running: "{yarn-berry} node -e \
                  \"process.stdout.write(require('greet-from-the-store'))\"",
        denied: "YN0080",
    })
    .await;
}

/// pnpm's store is the one beside the Worktrees — **and a session gets a copy
/// out of it rather than a hardlink, whatever disk it is on.**
///
/// This is the one thing none of the install proofs above can see, and the
/// task this landed under asked for the other answer. pnpm links a package out
/// of its store into `node_modules` and falls back to copying the whole of it
/// where the two are on different filesystems — which is why `{stores}` is in
/// the grammar at all, a directory beside the Worktrees being one a link out of
/// never crosses a disk. A copy fetches nothing either, so an offline install
/// goes on passing with the space a link would have saved gone.
///
/// What it turns out to cross is a **mount**. A Worktree is one bind in a
/// session's sandbox and this directory is another, and `link(2)` answers
/// `EXDEV` across two mounts even where both are the same filesystem — see
/// [`a_hardlink_across_two_of_a_sandboxs_binds_is_refused_by_the_kernel`],
/// which is that on its own with no pnpm in it. So the fall-back pnpm
/// documents for two filesystems is what every session gets, and no placement
/// a descriptor can name changes it: closing it means one mount holding both
/// the Worktree and the store, which is what a session may reach rather than
/// what a store is called.
///
/// Asserted as it stands rather than left out, in both of pnpm's own words and
/// the link count, because this is the shape of the thing the next release has
/// to be measured against — and because `{stores}` is still the right side of
/// that line, and is where the fix would land.
#[tokio::test]
async fn pnpms_store_is_beside_the_worktrees_and_a_session_copies_out_of_it() {
    let Some(tools) = tools("pnpm", &["pnpm", "tar", "sha1sum"]) else {
        return;
    };
    let (pnpm, tar, sha1sum) = (&tools[0], &tools[1], &tools[2]);

    let machine = machine(1).await;
    let cache = machine.cache();

    let registry = npm_registry(&machine.registries.join("pnpm-links"), tar, sha1sum);
    npm_consumer(machine.worktree(0));

    let installed = installing(
        &machine.sandbox(0, &cache, vec![]),
        &format!(
            "set -e\n\
             printf 'store=%s\\n' \"${{PNPM_CONFIG_STORE_DIR-unset}}\"\n\
             {pnpm} install --registry={registry}\n",
            pnpm = pnpm.display(),
            registry = registry.url,
        ),
    );

    registry.shut();

    installed.worked("a session installs the package the registry is serving");

    assert!(
        installed.said.contains(&format!(
            "store={}\n",
            machine.stores_beside("shared").join("pnpm").display()
        )),
        "pnpm's store is under the directory beside the Worktrees rather than \
         under the Build Cache, which is what {{stores}} means. The session \
         said:\n{}",
        installed.said,
    );

    // Followed rather than read of the link: pnpm puts a symlink at the
    // package's own name in `node_modules` and the real file under the virtual
    // store beside it, so what the count has to be read of is what that
    // resolves to.
    let inside = machine
        .worktree(0)
        .join("node_modules")
        .join(NPM_PACKAGE)
        .join("index.js");

    let links = std::fs::metadata(&inside)
        .unwrap_or_else(|error| {
            panic!(
                "{} should be there after an install ({error}), and it said:\n{}",
                inside.display(),
                installed.said,
            )
        })
        .nlink();

    assert_eq!(
        links,
        1,
        "the file in the Worktree is a copy of what was in the store rather \
         than a link to it — which is what the two of them being two bind \
         mounts comes to, and what would have to change for this to read \
         above one is the sandbox rather than this descriptor. {} said:\n{}",
        inside.display(),
        installed.said,
    );

    assert!(
        installed
            .said
            .contains("Packages are copied from the content-addressable store"),
        "and pnpm says so itself, which is the half of this evidence that does \
         not depend on how the two directories happen to be laid out. It \
         said:\n{}",
        installed.said,
    );
}

/// And why, with no package manager anywhere in it: **a hardlink from one of a
/// sandbox's binds into another is refused by the kernel**, both of them being
/// the same filesystem or not.
///
/// `link(2)`: "EXDEV — oldpath and newpath are not on the same mounted
/// filesystem", which is a mount rather than a device. Two `--bind` arguments
/// naming two directories of one tmpfs are two mounts, and the device number is
/// the same in both — so nothing a descriptor says about *where* a store goes
/// can make a link out of it into a Worktree work, and this is what says the
/// fall-back above is not a placement that could have been chosen better.
///
/// Said here rather than argued in a comment, because it is the finding this
/// stage turned up about the placeholder stage 01 added, and the next session
/// to read `{stores}` should find it proved rather than asserted.
#[test]
fn a_hardlink_across_two_of_a_sandboxs_binds_is_refused_by_the_kernel() {
    let Some(tools) = tools("the cross-bind hardlink", &["bwrap"]) else {
        return;
    };
    let bwrap = &tools[0];

    let dir = tempfile::tempdir().unwrap();
    let (store, project) = (dir.path().join("store"), dir.path().join("project"));

    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(store.join("in-the-store"), "a package\n").unwrap();

    let same =
        std::fs::metadata(&store).unwrap().dev() == std::fs::metadata(&project).unwrap().dev();

    assert!(
        same,
        "the two are one filesystem to begin with, which is the whole point \
         of a store beside the Worktrees"
    );

    let tried = Command::new(bwrap)
        .args(["--dev-bind", "/", "/"])
        .arg("--bind")
        .arg(&store)
        .arg("/the-store")
        .arg("--bind")
        .arg(&project)
        .arg("/the-project")
        .args([
            SH,
            "-c",
            "ln /the-store/in-the-store /the-project/linked 2>&1\n",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("bwrap should be on the PATH: the dev shell declares bubblewrap");

    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&tried.stdout),
        String::from_utf8_lossy(&tried.stderr)
    );

    assert!(
        !tried.status.success(),
        "a link across the two has to be refused, or pnpm copying above was \
         something else. It said:\n{said}",
    );
    assert!(
        said.contains("cross-device"),
        "and refused as EXDEV, which is the kernel saying *two mounts* rather \
         than two disks. It said:\n{said}",
    );
}

/// What a session printed under `key=`, which is how the proofs below ask a
/// tool several questions in one run.
fn line(ran: &Ran, key: &str) -> String {
    ran.said
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| {
            panic!(
                "the session was supposed to print a `{key}=` line, and it said:\n{}",
                ran.said
            )
        })
        .trim()
        .to_owned()
}

/// What a Repo has to say to keep its own store, per tool: the config that says
/// it, and the command line that says it where the config cannot.
struct WhoWins {
    tool: &'static str,
    wants: &'static [&'static str],

    /// The variable the descriptor sets for this tool, which is what the
    /// session was given.
    variable: &'static str,

    /// What prints where this tool thinks its store is. Some of them add a
    /// layout version underneath — pnpm's `v11`, Classic's `v6` — so what is
    /// asserted is the directory this is *under*.
    asking: &'static str,

    /// The Repo's own config: the file, and what it says.
    config: &'static str,
    says: &'static str,

    /// And the flag that says the same thing on the command line.
    flag: &'static str,
}

/// Where a Repo asks for its own store, the session's variable wins and the
/// Repo's **command line** wins over that.
///
/// The task this landed under asked for the opposite — a Repo's own config
/// winning over the variable a session was given — and the tools say otherwise.
/// All three put the command line above the environment and the environment
/// above their rc files, so a session's `NPM_CONFIG_CACHE` beats an `.npmrc`
/// that a Repo committed, and so on for the other two. That is not a thing a
/// descriptor can change: the grammar sets variables, and there is no lower
/// rung than the environment to set one on. What there *is* is a way for a Repo
/// to keep its own store all the same, and this is it — so what a Repo has to
/// do is written down and proved rather than left to be found out.
///
/// Both halves are asserted, because a silent loss is the worse of the two: a
/// Repo whose committed `.npmrc` stopped being read would otherwise be a thing
/// nobody noticed until an install went somewhere unexpected.
///
/// Yarn Berry is not here. It has no flag of its own for this, so its answer
/// is a different one and has a proof of its own — see
/// [`a_repo_that_switches_berrys_global_cache_off_gets_a_shared_one_anyway`].
#[tokio::test]
async fn a_repos_config_loses_to_the_session_and_its_command_line_wins() {
    // Somewhere no store of this machine's is, so that what the tool answers
    // can only have come from what the Repo asked for.
    const THE_REPOS_OWN: &str = "/the-repos-own-store";

    for who in [
        WhoWins {
            tool: "npm",
            wants: &["npm"],
            variable: "NPM_CONFIG_CACHE",
            asking: "{npm} config get cache",
            config: ".npmrc",
            says: "cache=/the-repos-own-store",
            flag: "--cache=/the-repos-own-store",
        },
        WhoWins {
            tool: "pnpm",
            wants: &["pnpm"],
            variable: "PNPM_CONFIG_STORE_DIR",
            asking: "{pnpm} store path",
            // pnpm 11 reads neither `store-dir` out of an `.npmrc` nor
            // `npm_config_store_dir` out of the environment: the Repo's own
            // place to say it is the workspace file.
            config: "pnpm-workspace.yaml",
            says: "storeDir: /the-repos-own-store",
            flag: "--store-dir=/the-repos-own-store",
        },
        WhoWins {
            tool: "yarn",
            wants: &["yarn"],
            variable: "YARN_CACHE_FOLDER",
            asking: "{yarn} cache dir",
            config: ".yarnrc",
            says: "cache-folder \"/the-repos-own-store\"",
            flag: "--cache-folder /the-repos-own-store",
        },
    ] {
        let Some(found_them) = tools(who.tool, who.wants) else {
            continue;
        };

        let named = |script: &str| {
            let mut said = script.to_owned();

            for (program, path) in who.wants.iter().zip(&found_them) {
                said = said.replace(&format!("{{{program}}}"), &path.display().to_string());
            }

            said
        };

        let machine = machine(1).await;
        let cache = machine.cache();

        // A manifest, because two of the three will not answer without a
        // project around them.
        npm_consumer(machine.worktree(0));

        let asked = installing(
            &machine.sandbox(0, &cache, vec![]),
            &named(&format!(
                "set -e\n\
                 printf 'given=%s\\n' \"${{{variable}-unset}}\"\n\
                 printf 'plain=%s\\n' \"$({asking})\"\n\
                 printf '%s\\n' '{says}' > {config}\n\
                 printf 'configured=%s\\n' \"$({asking})\"\n\
                 printf 'flagged=%s\\n' \"$({asking} {flag})\"\n",
                variable = who.variable,
                asking = who.asking,
                says = who.says,
                config = who.config,
                flag = who.flag,
            )),
        );

        asked.worked(&format!("{} answers where its store is", who.tool));

        let given = line(&asked, "given");

        assert_ne!(given, "unset", "the descriptor sets {}", who.variable);

        assert!(
            line(&asked, "plain").starts_with(&given),
            "{}'s store is where the session's {} pointed it",
            who.tool,
            who.variable,
        );

        assert!(
            line(&asked, "configured").starts_with(&given),
            "and a Repo's own {} does not move it: the session's variable is \
             above an rc file for every one of these tools, so what a Repo \
             committed loses. It said:\n{}",
            who.config,
            asked.said,
        );

        assert!(
            line(&asked, "flagged").starts_with(THE_REPOS_OWN),
            "but the Repo's command line does move it, which is what a Repo \
             that has to keep its own store passes. It said:\n{}",
            asked.said,
        );
    }
}

/// And Yarn Berry's answer, which is the same answer by a longer route: a Repo
/// saying `enableGlobalCache: false` **still gets a shared cache**, at the
/// folder the session's other yarn variable names.
///
/// That switch is not a path but a refusal of the whole arrangement — a Repo
/// doing zero-installs vendors its cache into the checkout on purpose. What it
/// does is send Berry from `globalFolder/cache` back to `cacheFolder`, whose
/// default is `./.yarn/cache` inside the project — and `cacheFolder` is
/// `YARN_CACHE_FOLDER`, which the session set for Yarn Classic. So the one
/// config of the four that looked like it won is a Repo landing in the other
/// shared directory instead of its own.
///
/// **`YARN_ENABLE_GLOBAL_CACHE` is deliberately not set**, which would settle
/// it either way: moving a store is what a descriptor is for and overriding a
/// Repo's policy switch is not, and a session that had done both would be
/// taking the decision twice.
///
/// So what a Repo loses here is the project-local cache, and what it keeps is
/// an install that works out of a store. Which is worth saying out loud, and
/// is what the adoption doc has to carry.
#[tokio::test]
async fn a_repo_that_switches_berrys_global_cache_off_gets_a_shared_one_anyway() {
    let Some(tools) = tools("yarn-berry", &["yarn-berry"]) else {
        return;
    };
    let berry = &tools[0];

    let machine = machine(1).await;
    let cache = machine.cache();

    npm_consumer(machine.worktree(0));

    let asked = installing(
        &machine.sandbox(0, &cache, vec![]),
        &format!(
            "set -e\n\
             export YARN_ENABLE_TELEMETRY=0\n\
             printf 'global=%s\\n' \"${{YARN_GLOBAL_FOLDER-unset}}\"\n\
             printf 'classic=%s\\n' \"${{YARN_CACHE_FOLDER-unset}}\"\n\
             printf 'by-default=%s\\n' \"$({berry} config get cacheFolder)\"\n\
             printf 'enableGlobalCache: false\\n' > .yarnrc.yml\n\
             printf 'switched-off=%s\\n' \"$({berry} config get cacheFolder)\"\n",
            berry = berry.display(),
        ),
    );

    asked.worked("Berry answers where its cache is");

    let global = line(&asked, "global");
    let classic = line(&asked, "classic");

    assert_ne!(global, "unset", "the descriptor sets YARN_GLOBAL_FOLDER");
    assert_ne!(classic, "unset", "and YARN_CACHE_FOLDER beside it");

    assert!(
        line(&asked, "by-default").starts_with(&global),
        "with nothing said, Berry's cache is under the global folder the \
         session was given — which is what says YARN_GLOBAL_FOLDER rather \
         than YARN_CACHE_FOLDER is Berry's variable. It said:\n{}",
        asked.said,
    );

    let switched_off = line(&asked, "switched-off");

    assert_eq!(
        switched_off, classic,
        "and a Repo that switched the global cache off lands in the folder the \
         session named for Classic rather than in its own checkout: still a \
         shared store, and not the one it asked for. It said:\n{}",
        asked.said,
    );
    assert!(
        !switched_off.starts_with(&machine.worktree(0).display().to_string()),
        "which is the project-local cache that Repo would have had without \
         Verkstead, and does not have with it"
    );
}

/// And the environment those installs ran in was the descriptor's: Node's five
/// variables, four of them under the one Build Cache and pnpm's under the
/// directory beside the Worktrees, and a session can write in both.
///
/// Beside the proofs rather than inside them, for the reason Go's is: what a
/// session is *told* is asserted on all three platforms in the sandbox suites,
/// and what a tool *does* with it is what an install is for. This is here so
/// that a run with none of the four tools installed still leaves something in
/// this file that ran.
#[tokio::test]
async fn a_session_is_given_the_four_javascript_tools_stores() {
    let machine = machine(1).await;
    let cache = machine.cache();
    let sandbox = machine.sandbox(0, &cache, vec![]);

    let dir = cache.dir().expect("the fixture's cache has a directory");
    let beside = machine.stores_beside("shared");

    let reported = installing(
        &sandbox,
        "set -e\n\
         for named in NPM_CONFIG_CACHE PNPM_CONFIG_STORE_DIR PNPM_CONFIG_CACHE_DIR \\\n\
                      YARN_CACHE_FOLDER YARN_GLOBAL_FOLDER; do\n\
           eval \"value=\\${$named-unset}\"\n\
           printf '%s=%s\\n' \"$named\" \"$value\"\n\
           mkdir -p \"$value\"\n\
           : > \"$value/written-from-inside\"\n\
         done\n",
    );

    reported.worked("a session can make and write every one of the directories it is pointed at");

    for (name, under) in [
        ("NPM_CONFIG_CACHE", "npm"),
        ("PNPM_CONFIG_CACHE_DIR", "pnpm/metadata"),
        ("YARN_CACHE_FOLDER", "yarn/cache"),
        ("YARN_GLOBAL_FOLDER", "yarn/global"),
    ] {
        assert!(
            reported
                .said
                .contains(&format!("{name}={}\n", dir.join(under).display())),
            "{name} is {under} inside the one Build Cache. The session said:\n{}",
            reported.said,
        );

        assert!(
            dir.join(under).join("written-from-inside").is_file(),
            "and what the session wrote under {name} is on the host, in the \
             directory the next Conversation's session will be given",
        );
    }

    assert!(
        reported.said.contains(&format!(
            "PNPM_CONFIG_STORE_DIR={}\n",
            beside.join("pnpm").display()
        )),
        "and pnpm's store is beside the Worktrees rather than in the cache, \
         because pnpm hardlinks out of it. The session said:\n{}",
        reported.said,
    );
    assert!(
        beside.join("pnpm/written-from-inside").is_file(),
        "which is the second bind a session now gets, and it is writable too"
    );
}
