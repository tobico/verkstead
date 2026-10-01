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
//!    says two sessions racing do not damage the store. **And the store they
//!    start from is one the sweep has just been through**: an install before
//!    theirs fills it, and the sweep is given half of that, taking whole units
//!    out until it is under — see [`filled_and_swept`]. So both succeeding says too
//!    that what a sweep leaves is a store a tool can install into.
//! 2. A third Sandbox, on that same Build Cache, installing it again with the
//!    tool **denied its registry** — the offline flag the tool documents, and
//!    no bind of the registry either. An install that succeeds with nothing to
//!    fetch from succeeded out of the store. **Where a tool documents no such
//!    flag** — bun, poetry, pipenv and NuGet — the third install is the second
//!    one word for word, against a registry that has stopped answering, which
//!    is the same proof by the other route, and arguably the plainer one: there
//!    is no flag that could be doing the work instead of the store.
//! 3. And the control: a fourth Sandbox, denied its registry the same way, on a
//!    Build Cache **nothing has filled**. It must fail. Without it, step 2 is a
//!    branch that would pass whatever the tool did with its variable — a build
//!    needing nothing looks exactly like a build served out of a store.
//!
//! **C/C++ is the one descriptor whose proof is a build instead**, because
//! what it names is no store but the Compile Server: a CMake project built
//! twice in one Sandbox and once more in a second Conversation's, read off the
//! server's own stats — see
//! [`cmake_compiles_through_the_compile_server_and_a_rebuild_or_a_second_conversation_is_all_hits`].
//!
//! *Found the store populated* is deliberately nowhere in that list. The store
//! is populated because the first two installs populated it, whether or not the
//! third read a byte of it.
//!
//! **The registry is this machine's own.** Nothing here reaches the internet: a
//! proof lays a package out in whatever shape its tool fetches from, and hands
//! it to the two Sandboxes that are allowed it and to no others. Which is
//! **measured rather than intended** — bind an empty `resolv.conf` over
//! `/etc/resolv.conf`, run this file inside that, and every proof in it still
//! passes. See
//! [`pipenv_fills_one_cache_and_a_third_install_reads_it`], where one of them
//! did not until it was told which index its distribution comes from. So *denied its
//! registry* is a fact about the machine as well as about the flag — and which
//! fact depends on what the tool can read a registry off:
//!
//! - Go's is a module proxy under a `file://` URL, which `go help goproxy` says
//!   is a proxy like any other, **bound read-only** into the Sandboxes that may
//!   have it and into no others.
//! - Nothing in the npm ecosystem reads a registry off the disk, so the
//!   JavaScript proofs really serve one, over the loopback and out of this
//!   process — see [`Registry`], whose `shut` is what denies it: the port stops
//!   answering before the install that must not reach it starts. deno and bun
//!   install out of that same registry, which is what makes them Node's rather
//!   than entries of their own.
//! - And Python's four serve one of their own, PEP 503's rather than npm's — see
//!   [`pypi_registry`] — shut the same way. All four install out of it, which is
//!   what makes them one entry as well.
//! - NuGet reads a directory of `.nupkg` files as a source, so it could have
//!   gone Go's way — and does not, because a source that is never fetched from
//!   would leave the http cache half of its descriptor unproved. So the .NET
//!   proofs serve the v3 protocol's flat container over the loopback — see
//!   [`nuget_registry`] — and deny it two ways at once: the Worktree's own
//!   `NuGet.config` clears every source, and the port stops answering.
//!
//! **A tool this machine cannot run inside a Sandbox is skipped in a line
//! naming it**, so a checkout run on a machine that only builds Rust stays
//! green. Two ways to be that, and one answer to both — see [`found`]: nothing
//! of that name on the `PATH` at all, and a name on the `PATH` resolving
//! somewhere no Sandbox binds, which is what a runner image's link into its own
//! tool cache is. Set [`REQUIRED`] and either is a failure instead, which is
//! what CI's own job does: a tool cannot quietly leave the list, or quietly
//! move out of reach of the thing that has to run it.
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
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Bind, Closing, Executable, Homes, Reachable, Sandbox};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

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

    /// The one Build Cache again, this time **with the sccache the machine
    /// has**, which is what a Compile Server is started from — the only proof
    /// in this file that wants one. See the CMake proof at the foot of the file.
    ///
    /// Its Data Directory is the fixture's own rather than one beside it: the
    /// Compile Server binds the Worktrees directory under it, and the Worktrees
    /// the sandboxes build in are the ones [`machine`] made there.
    fn compiling_cache(&self, sccache: &Path) -> BuildCache {
        let dir = self.state.path().join("cache");

        std::fs::create_dir_all(&dir).expect("a Build Cache directory to hand out");

        let cache = BuildCache::at(dir, Some(sccache.to_owned()), self.state.path().to_owned());
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

        let id = store::start_conversation(&pool, repo_row.id, &branch, THIS_DEVICE)
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

/// What this machine has of one program, which is three answers rather than
/// two — see [`found`].
enum Reach {
    /// It is here, at the path a session reaches it by.
    Here(PathBuf),

    /// Nothing of that name anywhere on this machine's `PATH`.
    Nowhere,

    /// On the `PATH`, and resolving to somewhere no Sandbox binds — so it is
    /// installed and a session cannot run it. Carries the path it really is at,
    /// because that is the whole of what a skip line has to say about it.
    Outside(PathBuf),
}

/// Where `program` is on the host, absolute and followed, and whether a session
/// could run it there.
///
/// Absolute because a sandbox's `PATH` is the machine's system profile rather
/// than the shell the tests were started from, and a tool the suite found on
/// its own `PATH` is one it has to name in full to reach inside.
///
/// **And followed, because the link is where the trouble is.** A program under
/// a directory every Sandbox binds may still be a symlink into one that none of
/// them does — a runner image keeps its own toolchains under
/// `/opt/hostedtoolcache` and links them onto `/usr/bin`, which is a `go` a
/// session opens and finds absent. Handed that path, a proof dies on
/// `/bin/sh: /usr/bin/go: not found` rather than skipping, which is the one
/// thing a machine that has not got a tool is supposed not to do.
///
/// So the question is asked of what the name resolves to and it is asked of the
/// server — [`verkstead_server::sandbox::reaches`], which is the same floor a
/// session is really given rather than a second copy of it written out here.
/// The CI job that installs these tools asks it of each of them with
/// `readlink -f`; this is the suite asking it wherever it runs.
fn found(program: &str) -> Reach {
    let Some(on_the_path) = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
    else {
        return Reach::Nowhere;
    };

    // Whatever it really is, which is what a session opens when it runs the
    // name above. A path that will not canonicalise at all is one nothing could
    // open either, so it is the same answer as a link leading nowhere.
    let Ok(real) = std::fs::canonicalize(&on_the_path) else {
        return Reach::Nowhere;
    };

    match verkstead_server::sandbox::reaches(Platform::HERE, &real) {
        true => Reach::Here(on_the_path),
        false => Reach::Outside(real),
    }
}

/// Every tool a proof needs, or `None` and a line naming the first one this
/// machine has not got — or has somewhere a session cannot reach, which is the
/// same answer for the same reason.
///
/// Where [`REQUIRED`] is set, either of those is a failure instead: that is the
/// job whose whole business is having them installed, and a proof quietly
/// skipped there would be a tool that had left the list without anybody
/// hearing about it.
fn tools(proof: &str, wanted: &[&str]) -> Option<Vec<PathBuf>> {
    let mut found_them = Vec::new();

    for program in wanted {
        let missing = match found(program) {
            Reach::Here(path) => {
                found_them.push(path);
                continue;
            }
            Reach::Nowhere => format!("this machine has no `{program}` on its PATH"),
            Reach::Outside(real) => format!(
                "this machine's `{program}` is {}, which no session's sandbox binds — see \
                 LINUX_SYSTEM in crates/server/src/sandbox.rs",
                real.display(),
            ),
        };

        println!("skipping the {proof} package-store proof: {missing}");

        assert!(
            std::env::var_os(REQUIRED).is_none(),
            "the {proof} package-store proof was skipped for want of a `{program}` a session \
             can run — {missing} — and {REQUIRED} is set, which is the run where every tool on \
             the list has to be installed somewhere a Sandbox reaches. Install it there in that \
             job, or take {proof} off the list.",
        );

        return None;
    }

    Some(found_them)
}

/// What a store's units are expected to be, which is one of two shapes.
#[derive(Clone, Copy)]
enum Units {
    /// Packages by name: each unit names exactly one of these — in its own
    /// path or in a file inside it — and every file under where the units are
    /// looked for that names one is inside a unit. Which is *every file a
    /// package installed is inside exactly one unit, and no unit spans two*.
    Naming(&'static [&'static str]),

    /// Keyed by a hash rather than a name: a content-addressed store, whose
    /// tool checks every blob it reads, or an HTTP cache keyed by URL. Each
    /// unit is one file, or one directory of files with nothing under it, so a
    /// unit is never a shard of many.
    Hashed,
}

/// Every file and link under `dir`, relative to it, for a failure to be read
/// against — what the tool really left, rather than what was expected of it.
fn tree(dir: &Path) -> Vec<String> {
    fn under(dir: &Path, at: &Path, into: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(at) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };

            match metadata.is_dir() {
                true => under(dir, &path, into),
                false => into.push(path.strip_prefix(dir).unwrap().display().to_string()),
            }
        }
    }

    let mut found = Vec::new();
    under(dir, dir, &mut found);
    found.sort();

    found
}

/// The units the built-in `language` names in its store `store`, found in that
/// store on `cache` once a real tool has filled it — and the assertion that
/// each is one whole package, the way `expected` says a package looks in it.
///
/// **This is the descriptor's units against the tool's real layout**, which is
/// the one thing a unit test over a hand-made tree cannot say: a release that
/// moved its packages one level down would leave a unit spanning every package
/// there is, and only an install shows that.
fn whole_packages(cache: &BuildCache, language: &str, store: &str, expected: Units) {
    use verkstead_server::languages::Bounded;

    let descriptor = verkstead_server::languages::built_in()
        .get(language)
        .expect("a built-in language");
    let machine = cache
        .machine()
        .expect("the fixture's cache has a directory");

    let dir = descriptor
        .stores(&machine)
        .into_iter()
        .find_map(|(name, dir)| (name == store).then_some(dir))
        .unwrap_or_else(|| panic!("{language} names a store {store}"));

    let Bounded::ByUnit(units) = descriptor.bounded(store) else {
        panic!("{language}'s store {store} is swept by unit");
    };

    let found = verkstead_server::units::listed(&dir, units);
    let held = tree(&dir).join("\n");

    let relative = |path: &Path| {
        path.strip_prefix(&dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/")
    };

    assert!(
        !found.is_empty(),
        "{language}'s store {store} holds a unit once a tool has filled it. It holds:\n{held}",
    );

    println!(
        "{language}'s store {store} holds the units:\n  {}",
        found
            .iter()
            .map(|unit| relative(unit))
            .collect::<Vec<_>>()
            .join("\n  "),
    );

    for unit in &found {
        let metadata = std::fs::symlink_metadata(unit).unwrap();

        assert!(
            !metadata.is_symlink(),
            "a link is never a unit: {} in {language}'s store {store}",
            relative(unit),
        );
    }

    match expected {
        Units::Hashed => {
            for unit in &found {
                let leaf = std::fs::symlink_metadata(unit).unwrap().is_file()
                    || std::fs::read_dir(unit)
                        .unwrap()
                        .flatten()
                        .all(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()));

                assert!(
                    leaf,
                    "a unit of {language}'s hashed store {store} is one entry rather than a \
                     shard of many, and {} is not. The store holds:\n{held}",
                    relative(unit),
                );
            }
        }

        Units::Naming(packages) => {
            let naming = |path: &str| -> Vec<&str> {
                let path = path.to_lowercase();

                packages
                    .iter()
                    .copied()
                    .filter(|package| path.contains(&package.to_lowercase()))
                    .collect()
            };

            for unit in &found {
                let inside = relative(unit);

                let mut named: Vec<&str> = naming(&inside);
                named.extend(tree(unit).iter().flat_map(|file| naming(file)));
                named.sort_unstable();
                named.dedup();

                assert!(
                    named.len() == 1,
                    "a unit of {language}'s store {store} is one package, and {inside} names \
                     {named:?}. The store holds:\n{held}",
                );
            }

            // And the other half: nothing of a package's is left outside every
            // unit, where the units are looked for.
            let roots: Vec<PathBuf> = units.iter().map(|unit| unit.root(&dir)).collect();

            for file in tree(&dir) {
                let path = dir.join(&file);

                // A link is no package's file: bun keeps one per version beside
                // the directory that is the version, and it is the directory
                // that is the unit.
                if !roots.iter().any(|root| path.starts_with(root))
                    || naming(&file).is_empty()
                    || std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_symlink())
                {
                    continue;
                }

                assert!(
                    found.iter().any(|unit| path.starts_with(unit)),
                    "every file of a package is inside a unit, and {file} in {language}'s store \
                     {store} is in none. The units are:\n{}\nand the store holds:\n{held}",
                    found
                        .iter()
                        .map(|unit| relative(unit))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
        }
    }
}

/// One install in the `nth` Sandbox before a proof's own, filling `language`'s
/// stores on `cache` — and then **the sweep, given half of what that left**,
/// taking whole units out until they are under it or none is left. So the installs the proof is about run against a store the
/// sweep has just taken whole units out of, which is the store a sweep ever
/// leaves a tool: one with packages missing and none of them half there.
///
/// A unit gone is a package fetched again, so the two racing installs after
/// this one still have their registry and fill the store back up; what would
/// fail them is a unit taken in part, or one taken from under an index that
/// the tool then trusts.
fn filled_and_swept(
    machine: &Machine,
    nth: usize,
    cache: &BuildCache,
    extra: Vec<Bind>,
    script: &str,
    language: &str,
) {
    installing(&machine.sandbox(nth, cache, extra), script).worked(&format!(
        "an install before the proof's own fills {language}'s stores for the sweep to take from"
    ));

    let descriptor = verkstead_server::languages::built_in()
        .get(language)
        .expect("a built-in language");

    let held = verkstead_server::eviction::swept(cache, descriptor, u64::MAX).held;
    let size = held / 2;

    let swept = verkstead_server::eviction::swept(cache, descriptor, size);

    assert!(
        !swept.removed.is_empty() && swept.left < held && !swept.stopped,
        "the sweep takes units out of {language}'s stores, over the {size} bytes it was \
         given with the {held} an install left. It came to {swept:?}",
    );

    // And under the size: what is held to it is the units alone — a store's
    // indexes and metadata are no package of anybody's, so they are neither
    // taken nor counted — so taking units always gets there.
    assert!(
        swept.left <= size,
        "and stops once {language}'s units are under {size} bytes. It came to {swept:?}",
    );

    for unit in &swept.removed {
        assert!(
            std::fs::symlink_metadata(unit).is_err(),
            "and what it took is gone: {}",
            unit.display(),
        );
    }

    println!(
        "{language}'s stores held {held} bytes, and the sweep took them to {} by removing:\n  {}",
        swept.left,
        swept
            .removed
            .iter()
            .map(|unit| unit.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  "),
    );
}

/// **Clear** every store of `language` on `cache`, as the settings page's
/// button does, and assert it is empty: each store directory still there and
/// holding nothing.
fn cleared(cache: &BuildCache, language: &str) {
    let descriptor = verkstead_server::languages::built_in()
        .get(language)
        .expect("a built-in language");

    let emptied = verkstead_server::eviction::emptied(cache, descriptor, &[])
        .expect("nothing is running, so a Clear goes ahead");

    assert!(
        emptied > 0,
        "a Clear of a store an install filled takes something out of it"
    );

    let machine = cache
        .machine()
        .expect("the fixture's cache has a directory");

    for (name, dir) in descriptor.stores(&machine) {
        assert!(
            std::fs::read_dir(&dir).map_or(true, |mut entries| entries.next().is_none()),
            "{language}'s {name} store is empty after a Clear, and holds:\n  {}",
            tree(&dir).join("\n  "),
        );
    }
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

    // Five: the two that race, the one denied its registry, the control, and
    // the one that fills the store for the sweep before any of them.
    let machine = machine(5).await;
    let cache = machine.cache();

    let proxy = machine.registries.join("go");
    go_proxy(&proxy, zip);

    for nth in 0..5 {
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

    // A store the sweep has just taken units out of, which is what the two
    // below start from.
    filled_and_swept(
        &machine,
        4,
        &cache,
        reaching(),
        &building,
        verkstead_server::languages::GO,
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

    // And what the two left is units the sweep can take out whole: the module
    // and its download, and the build cache's blobs one at a time.
    whole_packages(
        &cache,
        verkstead_server::languages::GO,
        "modules",
        Units::Naming(&["greet"]),
    );
    whole_packages(
        &cache,
        verkstead_server::languages::GO,
        "build",
        Units::Hashed,
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

/// **A cleared Go store is fetched into again.** A build fills the store, a
/// Clear empties it — so a build denied its registry now fails, which is what
/// says the Clear really took the module — and the next build with its registry
/// fetches the module again, after which one denied it builds out of the store
/// once more.
#[tokio::test]
async fn a_cleared_go_store_is_empty_and_the_next_build_fetches_into_it_again() {
    let Some(tools) = tools("Go", &["go", "zip"]) else {
        return;
    };
    let (go, zip) = (&tools[0], &tools[1]);

    let machine = machine(2).await;
    let cache = machine.cache();

    let proxy = machine.registries.join("go");
    go_proxy(&proxy, zip);

    for nth in 0..2 {
        go_consumer(machine.worktree(nth));
    }

    let reaching = || vec![Bind::readable(proxy.clone())];

    let building = format!(
        "set -e\n{GO_SETTINGS}\nexport GOPROXY=file://{proxy}\n{go} build -o ./built ./...\n",
        proxy = proxy.display(),
        go = go.display(),
    );
    let offline = format!(
        "set -e\n{GO_SETTINGS}\nexport GOPROXY=off\n{go} build -o ./built ./...\n./built\n",
        go = go.display(),
    );

    installing(&machine.sandbox(0, &cache, reaching()), &building)
        .worked("a build fills the store");

    cleared(&cache, verkstead_server::languages::GO);

    let emptied = installing(&machine.sandbox(1, &cache, vec![]), &offline);

    assert!(
        !emptied.worked && emptied.said.contains("GOPROXY=off"),
        "with the store cleared and no registry, a build has nothing to build out of. It \
         said:\n{}",
        emptied.said,
    );

    installing(&machine.sandbox(0, &cache, reaching()), &building)
        .worked("the next build with its registry fetches the module into the cleared store");

    let refilled = installing(&machine.sandbox(1, &cache, vec![]), &offline);

    refilled.worked("and a build denied its registry builds out of the store it refilled");
    assert!(
        refilled.said.contains("out of the store"),
        "and what it built runs. It said:\n{}",
        refilled.said,
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

/// What two of this tool's installs at once are allowed to come to.
///
/// **The two-at-once half of every proof asks the same question and one tool
/// answers it differently**, so the answer is a field rather than a silence: a
/// suite that quietly tolerated a failure everywhere would be a suite with
/// nothing to say about the ten tools that never need it.
enum Racing {
    /// Both finish, which is what a store safe for concurrent writers means and
    /// what ten of the eleven tools give.
    BothFinish,

    /// One of the two may fail, and the store is still there for whoever comes
    /// next — **Yarn Classic, measured**.
    ///
    /// Classic creates its cache entry at the entry's *final* path and fills it
    /// afterwards, so a second install that arrives in between sees the
    /// directory, takes the entry for complete, and opens a `.yarn-tarball.tgz`
    /// the first one has not written yet: `ENOENT`, and that install stops.
    /// About one run in four here.
    ///
    /// **What is not damaged is the store**, which is the thing this proof is
    /// really for and which is asserted below whichever of the two lost: a third
    /// install with the registry gone still installs out of it and still runs
    /// what it installed. So the cache stays shared — one download for the
    /// machine is what it is for — and what a collision costs is the install
    /// that hit it, which is re-run.
    ///
    /// Berry is [`Racing::BothFinish`] and needs none of this: its cache is one
    /// zip per package written under a temporary name and renamed, so there is
    /// no half-made entry for anybody to find.
    OneMayLose,
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

    /// What two of them at once are allowed to come to — see [`Racing`].
    racing: Racing,

    /// The stores of Node's this tool fills, and what each one's units are
    /// expected to be once it has — see [`whole_packages`].
    units: &'static [(&'static str, Units)],
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

    // Five: the two that race, the one denied its registry, the control, and
    // the one that fills the store for the sweep before any of them.
    let machine = machine(5).await;
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

    for nth in 0..5 {
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

    // A store the sweep has just taken units out of, which is what the two
    // below start from.
    filled_and_swept(
        &machine,
        4,
        &cache,
        vec![],
        &filling,
        verkstead_server::languages::NODE,
    );

    // Started together and waited on together, which is the only way the two
    // are ever really writing the store at the same moment.
    let first = starting(&machine.sandbox(0, &cache, vec![]), &filling);
    let second = starting(&machine.sandbox(1, &cache, vec![]), &filling);

    let first = finished(first);
    let second = finished(second);

    match proof.racing {
        Racing::BothFinish => {
            first.worked(&format!(
                "the first session's {} install fills the store",
                proof.tool
            ));
            second.worked("and the second one racing it finishes just as well");
        }
        // And for the one tool that cannot promise that, the weaker thing that
        // is true — see [`Racing::OneMayLose`]. What the store was left in is
        // the third install's to say, below, which is the assertion that
        // matters either way.
        Racing::OneMayLose => assert!(
            first.worked || second.worked,
            "one of the two racing {} installs has to finish, whichever loses: \
             a collision that takes them both is a store nobody filled. The \
             first said:\n{}\nand the second said:\n{}",
            proof.tool,
            first.said,
            second.said,
        ),
    }

    // The lockfile a Repo would have committed, carried to the two that follow:
    // committed, and checked out into every Worktree.
    //
    // **Off whichever of the two finished**, because the one that lost a race
    // may have written none — and a `read` of the first Worktree alone would
    // then report a missing lockfile where what happened was the collision this
    // proof is about.
    let wrote_it = [(0, &first), (1, &second)]
        .into_iter()
        .filter(|(_, ran)| ran.worked)
        .map(|(nth, ran)| (machine.worktree(nth).join(proof.lockfile), ran))
        .find(|(path, _)| path.is_file());

    let Some((written, ran)) = wrote_it else {
        panic!(
            "{} should have written {} in the Worktree of an install that \
             finished. The first said:\n{}\nand the second said:\n{}",
            proof.tool, proof.lockfile, first.said, second.said,
        )
    };

    let lock = std::fs::read(&written).unwrap_or_else(|error| {
        panic!(
            "{} should have written {} ({error}), and it said:\n{}",
            proof.tool,
            written.display(),
            ran.said,
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

    // And what the installs left is units the sweep can take out whole.
    for (store, expected) in proof.units {
        whole_packages(&cache, verkstead_server::languages::NODE, store, *expected);
    }

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
        racing: Racing::BothFinish,
        units: &[("npm", Units::Hashed)],
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
        racing: Racing::BothFinish,
        units: &[
            ("pnpm", Units::Hashed),
            ("pnpm-metadata", Units::Naming(&[NPM_PACKAGE])),
        ],
    })
    .await;
}

/// Yarn Classic — yarn 1.x, still what `yarn` is on most machines:
/// `YARN_CACHE_FOLDER`, and `--offline`.
///
/// `--no-default-rc` so that what moved the cache was the variable the
/// descriptor set and nothing a `.yarnrc` anywhere said.
///
/// **And the one tool of the eleven whose two racing installs may not both
/// finish** — [`Racing::OneMayLose`], which is where the measurement is. This
/// was read as a flake first and is not one: it is Classic filling a cache
/// entry in place, under the entry's final name, so that a second install
/// arriving mid-write finds the directory and not the file inside it. The
/// descriptor shares the cache anyway, because what a collision costs is the
/// install that hit it and what it buys is one download for the machine.
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
        racing: Racing::OneMayLose,
        units: &[("yarn", Units::Naming(&[NPM_PACKAGE]))],
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
        racing: Racing::BothFinish,
        units: &[("yarn-berry", Units::Naming(&[NPM_PACKAGE]))],
    })
    .await;
}

/// deno: `DENO_DIR`, and `--cached-only`, which deno documents as "require that
/// remote dependencies are already cached".
///
/// **One variable for the whole of deno's cache**, where pnpm needed two: the
/// npm packages, the remote modules and the metadata are all directories under
/// `DENO_DIR`, so an offline install cannot be given half of it.
///
/// `NPM_CONFIG_REGISTRY` is how deno is pointed at the registry this proof
/// serves — deno's own documented name for it, and the only one of these six
/// tools that has no `--registry` of its own. `DENO_NO_UPDATE_CHECK` beside it
/// so that nothing here asks deno.com whether there is a newer deno.
///
/// It runs what it installed through `deno eval` rather than `node`: what is
/// being asked is whether *deno* resolved out of the store it was pointed at,
/// and `--cached-only` on the run says the answer was not fetched either.
///
/// The control fails `--cached-only is specified`, which is deno's own account
/// of a package it was told to find in the cache and could not.
#[tokio::test]
async fn deno_fills_one_cache_and_a_third_install_reads_it() {
    one_javascript_tools_store(Installs {
        tool: "deno",
        wants: &["deno", "tar", "sha1sum"],
        lockfile: "deno.lock",
        settings: "export DENO_NO_UPDATE_CHECK=1 NPM_CONFIG_REGISTRY={registry}",
        filling: "{deno} install",
        offline: "{deno} install --cached-only",
        running: "{deno} eval --cached-only \
                  \"import greeting from 'greet-from-the-store'; console.log(greeting)\"",
        denied: "--cached-only is specified",
        racing: Racing::BothFinish,
        units: &[("deno", Units::Naming(&[NPM_PACKAGE]))],
    })
    .await;
}

/// bun: `BUN_INSTALL_CACHE_DIR`, and **no offline flag at all**, which is the
/// one way this proof is not like the five above it.
///
/// `bun install --help` lists `--no-cache`, which is the opposite of what is
/// wanted, and nothing that says *cache only*. So the third install is the
/// second install **word for word** — the same command line, the same registry
/// URL — and the only thing that changed is that the registry has stopped
/// answering. Which is the proof by the other route, and arguably the plainer
/// one: there is no flag here that could be doing the work instead of the store.
///
/// That is also what makes the control carry more weight for bun than for the
/// others. `ConnectionRefused` is what bun says when it had to fetch, and the
/// control is the run that has to say it.
#[tokio::test]
async fn bun_fills_one_cache_and_a_third_install_reads_it() {
    one_javascript_tools_store(Installs {
        tool: "bun",
        wants: &["bun", "tar", "sha1sum"],
        lockfile: "bun.lock",
        settings: "",
        filling: "{bun} install --registry={registry}",
        // The same line, against a registry that is no longer there.
        offline: "{bun} install --registry={registry}",
        running: "{bun} -e \"process.stdout.write(require('greet-from-the-store'))\"",
        denied: "ConnectionRefused",
        racing: Racing::BothFinish,
        units: &[("bun", Units::Naming(&[NPM_PACKAGE]))],
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

/// What a tool that links out of its store has to be asked, when the tool will
/// not say for itself.
struct Links {
    tool: &'static str,
    wants: &'static [&'static str],

    /// The variable the descriptor sets for it, and what is under `{stores}`
    /// that it should name.
    variable: &'static str,
    under: &'static str,

    /// The install that fills it, with `{registry}` for the URL.
    install: &'static str,

    /// And where in the Worktree the installed file lands, which is the file the
    /// link count is read of. Not the same shape for any two of these tools:
    /// each lays `node_modules` out its own way.
    inside: &'static str,
}

/// deno's and bun's stores are beside the Worktrees too — **and a session gets
/// a copy out of each of them rather than a hardlink, exactly as it does out of
/// pnpm's.**
///
/// The same finding as
/// [`pnpms_store_is_beside_the_worktrees_and_a_session_copies_out_of_it`], one
/// task later and for two more tools, and it lands the same way: the store
/// works, a copy out of it fetches nothing, and what is gone is the disk space
/// the link would have saved. Two Worktrees and this directory are separate
/// **bind mounts** in a session's sandbox, and no placement a descriptor can
/// name gets a link across two of those.
///
/// **What is worse here than for pnpm is that neither tool says so.** pnpm
/// prints that packages were copied from the content-addressable store; deno
/// and bun fall back without a word and exit zero. So the link count is the
/// whole of the evidence, and a release that started linking again — which is
/// what one mount holding both would buy — would be read off this number and
/// nothing else.
///
/// **And it is a measurement rather than a reading for deno.** The task this
/// landed under expected deno's cache to be a `{cache}` store, nothing having
/// said deno links out of it. Outside a sandbox, on one filesystem, the
/// `index.js` under `node_modules/.deno/` and the one under `DENO_DIR/npm/` are
/// the same inode — so deno is where pnpm and bun are, and its store moved
/// beside the Worktrees because of what was measured rather than what was
/// assumed.
#[tokio::test]
async fn deno_and_bun_link_out_of_their_stores_too_and_so_copy_in_a_sandbox() {
    for links in [
        Links {
            tool: "deno",
            wants: &["deno", "tar", "sha1sum"],
            variable: "DENO_DIR",
            under: "deno",
            install: "export DENO_NO_UPDATE_CHECK=1 NPM_CONFIG_REGISTRY={registry}\n\
                      {deno} install",
            // deno's own virtual store, with the package's real files in it and
            // a symlink at the top of `node_modules` pointing down here.
            inside: ".deno/greet-from-the-store@1.0.0/node_modules/\
                     greet-from-the-store/index.js",
        },
        Links {
            tool: "bun",
            wants: &["bun", "tar", "sha1sum"],
            variable: "BUN_INSTALL_CACHE_DIR",
            under: "bun",
            install: "{bun} install --registry={registry}",
            // bun hoists, so the file is where `require` would look for it.
            inside: "greet-from-the-store/index.js",
        },
    ] {
        let Some(found_them) = tools(links.tool, links.wants) else {
            continue;
        };

        let machine = machine(1).await;
        let cache = machine.cache();

        let registry = npm_registry(
            &machine.registries.join(format!("{}-links", links.tool)),
            &found_them[1],
            &found_them[2],
        );

        npm_consumer(machine.worktree(0));

        let mut script = format!(
            "set -e\nprintf 'store=%s\\n' \"${{{variable}-unset}}\"\n{install}\n",
            variable = links.variable,
            install = links.install,
        )
        .replace("{registry}", &registry.url);

        for (program, path) in links.wants.iter().zip(&found_them) {
            script = script.replace(&format!("{{{program}}}"), &path.display().to_string());
        }

        let installed = installing(&machine.sandbox(0, &cache, vec![]), &script);

        registry.shut();

        installed.worked(&format!(
            "a {} session installs the package the registry is serving",
            links.tool,
        ));

        assert_eq!(
            line(&installed, "store"),
            machine
                .stores_beside("shared")
                .join(links.under)
                .display()
                .to_string(),
            "{}'s store is under the directory beside the Worktrees rather \
             than under the Build Cache, which is what {{stores}} means. The \
             session said:\n{}",
            links.tool,
            installed.said,
        );

        let inside = machine.worktree(0).join("node_modules").join(links.inside);

        let count = std::fs::metadata(&inside)
            .unwrap_or_else(|error| {
                panic!(
                    "{} should be there after a {} install ({error}), and it said:\n{}",
                    inside.display(),
                    links.tool,
                    installed.said,
                )
            })
            .nlink();

        assert_eq!(
            count,
            1,
            "the file in the Worktree is a copy of what was in {}'s store \
             rather than a link to it — which is what the two of them being two \
             bind mounts comes to, and what would have to change for this to \
             read above one is the sandbox rather than this descriptor. {} \
             said:\n{}",
            links.tool,
            inside.display(),
            installed.said,
        );
    }
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
///
/// **Each directory bound at its own path**, rather than at a name of this
/// test's choosing. A `--bind` needs its mountpoint to exist or be creatable,
/// and under `--dev-bind / /` a mountpoint named `/the-store` is one bwrap has
/// to `mkdir` on the real root — which fails outright anywhere `/` belongs to
/// root, so the probe died `Can't mkdir /the-store: Permission denied` on CI
/// while passing on a machine whose own root is a writable overlay. Bound where
/// they already are there is nothing to create, and it is the same two mounts
/// either way: what the kernel is being asked is about the binds and never
/// about what they are called.
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

    let (from, to) = (store.join("in-the-store"), project.join("linked"));

    let tried = Command::new(bwrap)
        .args(["--dev-bind", "/", "/"])
        .arg("--bind")
        .arg(&store)
        .arg(&store)
        .arg("--bind")
        .arg(&project)
        .arg(&project)
        .args([
            SH,
            "-c",
            &format!("ln {} {} 2>&1\n", from.display(), to.display()),
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

    /// And the flag that says the same thing on the command line, where this
    /// tool has one that the question above answers to.
    flag: Option<&'static str>,
}

/// Where a Repo asks for its own store, the session's variable wins and the
/// Repo's **command line** wins over that.
///
/// The task this landed under asked for the opposite — a Repo's own config
/// winning over the variable a session was given — and the tools say otherwise.
/// Every one of them puts the environment above its rc file, so a session's
/// `NPM_CONFIG_CACHE` beats an `.npmrc` that a Repo committed, and so on down
/// the list — bun's `bunfig.toml` included. That is not a thing a
/// descriptor can change: the grammar sets variables, and there is no lower
/// rung than the environment to set one on. What there *is* is a way for a Repo
/// to keep its own store all the same, and this is it — so what a Repo has to
/// do is written down and proved rather than left to be found out.
///
/// Both halves are asserted, because a silent loss is the worse of the two: a
/// Repo whose committed `.npmrc` stopped being read would otherwise be a thing
/// nobody noticed until an install went somewhere unexpected.
///
/// **bun has only the first half here**, and that is bun rather than this test:
/// `bun install --cache-dir` is documented, and `bun pm cache` — the only thing
/// that prints where the cache is — disregards it. So what is asked of bun is
/// what a Repo loses, which is the half that matters.
///
/// deno is not here at all: it documents no config key that moves `DENO_DIR`,
/// so there is nothing for a Repo to have committed and nothing to lose.
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
            flag: Some("--cache=/the-repos-own-store"),
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
            flag: Some("--store-dir=/the-repos-own-store"),
        },
        WhoWins {
            tool: "yarn",
            wants: &["yarn"],
            variable: "YARN_CACHE_FOLDER",
            asking: "{yarn} cache dir",
            config: ".yarnrc",
            says: "cache-folder \"/the-repos-own-store\"",
            flag: Some("--cache-folder /the-repos-own-store"),
        },
        WhoWins {
            tool: "bun",
            wants: &["bun"],
            variable: "BUN_INSTALL_CACHE_DIR",
            asking: "{bun} pm cache",
            // TOML dotted keys, so that one line says what two would: bun's
            // own place for this is `[install.cache]`'s `dir`.
            config: "bunfig.toml",
            says: "install.cache.dir = \"/the-repos-own-store\"",
            // `bun install --cache-dir` is documented, and `bun pm cache`
            // disregards it — so the flag is not askable the way the three
            // above it are, and the half that is asked is the half a Repo
            // loses.
            flag: None,
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

        // A manifest, because most of these will not answer without a project
        // around them.
        npm_consumer(machine.worktree(0));

        let asked = installing(
            &machine.sandbox(0, &cache, vec![]),
            &named(&format!(
                "set -e\n\
                 printf 'given=%s\\n' \"${{{variable}-unset}}\"\n\
                 printf 'plain=%s\\n' \"$({asking})\"\n\
                 printf '%s\\n' '{says}' > {config}\n\
                 printf 'configured=%s\\n' \"$({asking})\"\n\
                 {flagged}",
                variable = who.variable,
                asking = who.asking,
                says = who.says,
                config = who.config,
                flagged = who.flag.map_or_else(String::new, |flag| format!(
                    "printf 'flagged=%s\\n' \"$({asking} {flag})\"\n",
                    asking = who.asking,
                )),
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

        if who.flag.is_some() {
            assert!(
                line(&asked, "flagged").starts_with(THE_REPOS_OWN),
                "but the Repo's command line does move it, which is what a Repo \
                 that has to keep its own store passes. It said:\n{}",
                asked.said,
            );
        }
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

/// And the environment those installs ran in was the descriptor's: Node's seven
/// variables, four of them under the one Build Cache and pnpm's, deno's and
/// bun's under the directory beside the Worktrees, and a session can write in
/// both.
///
/// Beside the proofs rather than inside them, for the reason Go's is: what a
/// session is *told* is asserted on all three platforms in the sandbox suites,
/// and what a tool *does* with it is what an install is for. This is here so
/// that a run with none of the six tools installed still leaves something in
/// this file that ran.
#[tokio::test]
async fn a_session_is_given_the_six_javascript_tools_stores() {
    let machine = machine(1).await;
    let cache = machine.cache();
    let sandbox = machine.sandbox(0, &cache, vec![]);

    let dir = cache.dir().expect("the fixture's cache has a directory");
    let beside = machine.stores_beside("shared");

    let reported = installing(
        &sandbox,
        "set -e\n\
         for named in NPM_CONFIG_CACHE PNPM_CONFIG_STORE_DIR PNPM_CONFIG_CACHE_DIR \\\n\
                      YARN_CACHE_FOLDER YARN_GLOBAL_FOLDER DENO_DIR \\\n\
                      BUN_INSTALL_CACHE_DIR; do\n\
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

    for (name, under) in [
        ("PNPM_CONFIG_STORE_DIR", "pnpm"),
        ("DENO_DIR", "deno"),
        ("BUN_INSTALL_CACHE_DIR", "bun"),
    ] {
        assert!(
            reported
                .said
                .contains(&format!("{name}={}\n", beside.join(under).display())),
            "and {name} is beside the Worktrees rather than in the cache, \
             because that tool hardlinks out of it. The session said:\n{}",
            reported.said,
        );
        assert!(
            beside.join(under).join("written-from-inside").is_file(),
            "which is the second bind a session now gets, and it is writable \
             too — {under} included",
        );
    }
}

// ---------------------------------------------------------------------------
// Python: pip and uv.
// ---------------------------------------------------------------------------

/// The distribution every Python proof installs, the module it imports and the
/// version of it. Two names because Python has two: a distribution is named
/// with hyphens on an index and imported with underscores.
const PYPI_PACKAGE: &str = "greet-from-the-store";
const PYPI_MODULE: &str = "greet_from_the_store";
const PYPI_VERSION: &str = "1.0.0";

/// What the wheel is called, which is the one filename in this ecosystem that
/// has to be spelled exactly: `<distribution>-<version>-<tags>.whl`, with the
/// distribution's underscores rather than its hyphens.
const PYPI_WHEEL: &str = "greet_from_the_store-1.0.0-py3-none-any.whl";

/// One Python index, on a loopback port, serving [`PYPI_PACKAGE`] as a wheel.
///
/// The shape is PEP 503's — the *simple* repository API, which is the whole of
/// what an install of one pinned distribution reads: a page per project at
/// `/simple/<name>/` holding an anchor per file, and the files themselves. No
/// sdist and no build backend, because a wheel is installed by unpacking it and
/// an sdist would have this proof fetching setuptools.
///
/// **Every response says `Cache-Control: max-age`**, and pip is the reason. pip's
/// store is an HTTP cache, and `cachecontrol` keeps a response only where the
/// response said it could be kept — no `ETag`, no `max-age` and nothing is
/// written, so a registry that did not say this would be a registry pip
/// downloaded from twice and cached nothing of. A real index says it; the point
/// of saying it here is that the proof below is not resting on a header a
/// registry made up.
///
/// The wheel is a zip with two members and no compiled anything: the module,
/// and the `.dist-info` directory that says what it is. Built with the suite's
/// own `zip`, exactly as the Go proof builds its module archive.
///
/// **`RECORD` names every file by sha256 and size**, which is the spec's own
/// shape and is not decoration: poetry reads that file as it installs and says
/// so where an entry is missing, so a `RECORD` with the hashes left out is three
/// paragraphs of warning on every poetry install in this suite. The hash is
/// urlsafe base64 without padding, prefixed `sha256=`, and `RECORD`'s own line
/// carries neither — both of which the spec says.
fn pypi_registry(at: &Path, zip: &Path) -> Registry {
    let inside = at.join("what-goes-in-the-wheel");
    let dist_info = format!("{PYPI_MODULE}-{PYPI_VERSION}.dist-info");
    let module = inside.join(PYPI_MODULE);
    let metadata = inside.join(&dist_info);

    std::fs::create_dir_all(&module).unwrap();
    std::fs::create_dir_all(&metadata).unwrap();

    // The three files a wheel has to carry, and then the fourth that says what
    // the three of them are.
    let files = [
        (
            format!("{PYPI_MODULE}/__init__.py"),
            format!("GREETING = \"{OUT_OF_THE_STORE}\"\n"),
        ),
        (
            format!("{dist_info}/METADATA"),
            format!(
                "Metadata-Version: 2.1\nName: {PYPI_PACKAGE}\nVersion: {PYPI_VERSION}\n\
                 Summary: what a shared store held\n\n"
            ),
        ),
        (
            format!("{dist_info}/WHEEL"),
            String::from(
                "Wheel-Version: 1.0\nGenerator: verkstead-tests\nRoot-Is-Purelib: true\n\
                 Tag: py3-none-any\n",
            ),
        ),
    ];

    let mut record = String::new();

    for (named, content) in &files {
        std::fs::write(inside.join(named), content).unwrap();

        record.push_str(&format!(
            "{named},sha256={hash},{size}\n",
            hash = URL_SAFE_NO_PAD.encode(Sha256::digest(content.as_bytes())),
            size = content.len(),
        ));
    }

    record.push_str(&format!("{dist_info}/RECORD,,\n"));

    std::fs::write(metadata.join("RECORD"), &record).unwrap();

    let archive = at.join(PYPI_WHEEL);
    let made = Command::new(zip)
        .args(["-q", "-r", "-X", "-D"])
        .arg(&archive)
        .arg(PYPI_MODULE)
        .arg(&dist_info)
        .current_dir(&inside)
        .stdin(Stdio::null())
        .status()
        .expect("the suite's own `zip`");

    assert!(made.success(), "the wheel was not built");

    // Bound before anything is served, for [`npm_registry`]'s reason: the port
    // goes in the page, so it has to be known first.
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .expect("a loopback port for the index");
    let url = format!("http://{}", listener.local_addr().expect("the port it got"));

    listener
        .set_nonblocking(true)
        .expect("what tokio takes a standard listener over");

    let published = Published {
        packument: format!(
            "<!DOCTYPE html><html><body><a href=\"{url}/files/{PYPI_WHEEL}\">{PYPI_WHEEL}</a>\
             </body></html>"
        ),
        tarball: std::fs::read(&archive).unwrap(),
    };

    let page = |State(served): State<Published>| async move {
        (
            [
                (header::CONTENT_TYPE, "text/html"),
                (header::CACHE_CONTROL, "max-age=3600"),
            ],
            served.packument,
        )
    };

    let app = Router::new()
        // Both spellings of the project's page. A tool asks for the one with the
        // trailing slash — both of these do — and the other is here so that a
        // release which stopped doing that is a proof that fails on an
        // assertion rather than on a 404 nobody can read.
        .route(&format!("/simple/{PYPI_PACKAGE}/"), get(page))
        .route(&format!("/simple/{PYPI_PACKAGE}"), get(page))
        .route(
            &format!("/files/{{{PYPI_MODULE}}}"),
            get(|State(served): State<Published>| async move {
                (
                    [
                        (header::CONTENT_TYPE, "application/octet-stream"),
                        (header::CACHE_CONTROL, "max-age=3600"),
                    ],
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
            .expect("a runtime for the index's own thread");

        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener)
                .expect("the listener this thread was handed");

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

/// What one Python tool's install proof needs saying about it. Everything else
/// is the four Sandboxes below, which are the same for both.
struct Python {
    /// What the skip line names and the assertions call it.
    tool: &'static str,

    /// The programs it takes, each standing in the scripts below as
    /// `{its-own-name}` — and `zip`, which builds the wheel.
    ///
    /// **`python3` rather than `pip`**, for pip: pip is a module of the
    /// interpreter that carries it, `python -m pip` is how pip's own
    /// documentation says to run it, and a machine may have the module without
    /// a program of that name on its `PATH`. So the tool this proof is *about*
    /// and the program it is *found* by are two different things, which is why
    /// the skip line names the program.
    wants: &'static [&'static str],

    /// What every install of this tool is told, beside the cache variable the
    /// descriptor sets — which is the one this is here to prove. Each of these
    /// takes something out of the way rather than moving a store.
    settings: &'static str,

    /// What the Worktree holds before any of it runs: the manifest this tool
    /// reads, named and written, with `{registry}` and `{wheel}` in it the way
    /// the scripts have them.
    ///
    /// pip and uv name what they are installing on the command line and need
    /// none; poetry and pipenv read a project rather than an argument, which is
    /// also where each is told which index to use.
    manifest: Option<(&'static str, &'static str)>,

    /// And the lockfile the first install writes, which a Repo would have
    /// committed: the two installs after it are handed a copy, which is what a
    /// second Conversation on a checked-out Repo really starts from — see
    /// [`one_javascript_tools_store`], where the same thing is done for the same
    /// reason. pip and uv write none.
    lockfile: Option<&'static str>,

    /// The install that fills the store, with `{registry}` for the index's base
    /// URL and `{wheel}` for the wheel's own.
    filling: &'static str,

    /// And the same install with the tool **denied its registry**: the offline
    /// switch it documents, run against an index that has been taken off the
    /// air.
    offline: &'static str,

    /// What runs what was installed.
    running: &'static str,

    /// And what the control's failure has to say, so that what it failed over
    /// was the denial rather than anything else.
    denied: &'static str,

    /// The store of Python's this tool fills, and what its units are expected
    /// to be once it has — see [`whole_packages`].
    units: (&'static str, Units),
}

/// One Python tool's proof: two installs at once against one store, a third out
/// of what they left with the index gone, and the control that says the third
/// proved something.
///
/// The same four Sandboxes as [`one_javascript_tools_store`], and the same
/// reasoning throughout. One thing is its own: the registry is PEP 503's rather
/// than npm's — see [`pypi_registry`].
///
/// **Two of the four tools install out of a project and two out of a command
/// line**, which is the only other difference and is the reason the manifest and
/// the lockfile are each an `Option` here where they are not in Node's. pip and
/// uv are told what to install and which index to take it from as arguments, and
/// leave nothing behind that the next install reads; poetry and pipenv read a
/// file in the Worktree for both, and write a lockfile that a Repo would have
/// committed — so those two have one written into all four Worktrees, and the
/// first install's lock carried into the two that follow it.
async fn one_python_tools_store(proof: Python) {
    let Some(found_them) = tools(proof.tool, proof.wants) else {
        return;
    };

    let named = |script: &str, registry: &str| {
        let mut said = script
            .replace("{registry}", registry)
            .replace("{wheel}", &format!("{registry}/files/{PYPI_WHEEL}"));

        for (program, path) in proof.wants.iter().zip(&found_them) {
            said = said.replace(&format!("{{{program}}}"), &path.display().to_string());
        }

        said
    };

    // Five: the two that race, the one denied its index, the control, and the
    // one that fills the store for the sweep before any of them.
    let machine = machine(5).await;
    let cache = machine.cache();

    let registry = pypi_registry(
        &machine.registries.join(proof.tool),
        &found_them[proof
            .wants
            .iter()
            .position(|program| *program == "zip")
            .expect("every Python proof builds its wheel with `zip`")],
    );

    // Held rather than read off the registry, so that the scripts outlive
    // taking it off the air.
    let url = registry.url.clone();

    // And the manifest every one of the five Worktrees holds, where this tool
    // installs out of one: a Repo's own file, naming the index this proof serves
    // and the distribution it is to install out of it.
    if let Some((called, written)) = proof.manifest {
        for nth in 0..5 {
            std::fs::write(machine.worktree(nth).join(called), named(written, &url)).unwrap();
        }
    }

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

    // A store the sweep has just taken units out of, which is what the two
    // below start from.
    filled_and_swept(
        &machine,
        4,
        &cache,
        vec![],
        &filling,
        verkstead_server::languages::PYTHON,
    );

    // Started together and waited on together, which is the only way the two
    // are ever really writing the store at the same moment.
    let first = starting(&machine.sandbox(0, &cache, vec![]), &filling);
    let second = starting(&machine.sandbox(1, &cache, vec![]), &filling);

    let first = finished(first);

    first.worked(&format!(
        "the first session's {} install fills the store",
        proof.tool
    ));
    finished(second).worked("and the second one racing it finishes just as well");

    // The lockfile the first one wrote, carried to the two that follow the way
    // a Repo carries one: committed, and checked out into every Worktree.
    if let Some(called) = proof.lockfile {
        let written = machine.worktree(0).join(called);
        let lock = std::fs::read(&written).unwrap_or_else(|error| {
            panic!(
                "{} should have written {} ({error}), and it said:\n{}",
                proof.tool,
                written.display(),
                first.said,
            )
        });

        for nth in 2..4 {
            std::fs::write(machine.worktree(nth).join(called), &lock).unwrap();
        }
    }

    // And the proof. The index stops answering and the tool is told not to look
    // for one: what is left to install out of is the store the two above
    // filled.
    registry.shut();

    let offline = script(proof.offline);
    let third = installing(&machine.sandbox(2, &cache, vec![]), &offline);

    third.worked(&format!(
        "a third session installs the same distribution with {}'s index denied, \
         which it can only do out of the shared store",
        proof.tool,
    ));
    assert!(
        third.said.contains(OUT_OF_THE_STORE),
        "and what it installed really imports, so the store held the \
         distribution rather than something shaped like it. It said:\n{}",
        third.said,
    );

    // And what the installs left is units the sweep can take out whole.
    let (store, expected) = proof.units;
    whole_packages(&cache, verkstead_server::languages::PYTHON, store, expected);

    // The control. Everything the same but the Build Cache and the directory
    // beside its Worktrees, neither of which anything has filled — so a pass
    // here would mean the install above needed no store at all.
    let control = installing(
        &machine.sandbox(3, &machine.empty_cache(), vec![]),
        &offline,
    );

    assert!(
        !control.worked,
        "an empty store and no index has to fail, or the install above proved \
         nothing about either. It said:\n{}",
        control.said,
    );
    assert!(
        control.said.contains(proof.denied),
        "and it fails for want of the index rather than for some other reason: \
         `{}` is what {} says about that. It said:\n{}",
        proof.denied,
        proof.tool,
        control.said,
    );
}

/// pip: `PIP_CACHE_DIR`, and `--no-index`, which pip documents as "ignore
/// package index (only looking at `--find-links` URLs instead)".
///
/// **pip's store is an HTTP cache rather than an index an install can be
/// resolved against**, and this proof is shaped by that. Two consequences, both
/// measured here rather than read out of the documentation.
///
/// The first is that the third install cannot be the second word for word. pip
/// asks for an index page with `Cache-Control: max-age=0` on the *request*, so
/// it revalidates that page on every install however full the cache is — and
/// with the index unreachable, a requirement named by *name* cannot be resolved
/// at all. What the cache can serve with nothing on the air is a distribution
/// already asked for by **URL**, which is what a pinned requirements file holds.
/// So the two that fill the store install from the index the ordinary way, and
/// the third installs the wheel they left in the cache by the URL they fetched
/// it from. `--no-index` rides along saying no index may be consulted, and what
/// is left for pip to read is the store.
///
/// The second is `--trusted-host`. pip mounts a **non-caching** adapter for
/// plain `http://` and the caching one only for a host that was named trusted —
/// so without this flag a proof against a loopback registry would download
/// twice and cache nothing, and the third install would fail for a reason
/// nothing to do with the variable. The same shape as Berry's
/// `YARN_UNSAFE_HTTP_WHITELIST` above, and for the same reason: a registry on
/// this machine is not one with a certificate. A real index is HTTPS and needs
/// none of it.
///
/// The control fails on `Connection refused`, which is pip having had to fetch.
#[tokio::test]
async fn pip_fills_one_cache_and_a_third_install_reads_it() {
    one_python_tools_store(Python {
        tool: "pip",
        wants: &["python3", "zip"],
        // Nothing in the Worktree: what pip installs is named on its command
        // line, and what it writes is a directory rather than a lockfile.
        manifest: None,
        lockfile: None,
        // `PIP_DISABLE_PIP_VERSION_CHECK` so that nothing here asks pypi.org
        // whether there is a newer pip — the one thing in this proof that would
        // otherwise reach the internet — and `PIP_RETRIES=1` so that the
        // control's failure is a failure rather than half a minute of backoff.
        settings: "export PIP_DISABLE_PIP_VERSION_CHECK=1 PIP_RETRIES=1",
        // `--target`, for two reasons that are both about the machine rather
        // than the cache. `python3 -m venv` wants `ensurepip`, which plenty of
        // distributions package separately; and a distribution's own Python is
        // usually **externally managed** (PEP 668), which pip refuses to
        // install into at all — `--root`, `--target` and `--prefix` are the
        // three flags that turn that check off, because none of them is the
        // environment the distribution manages. So this is what makes the proof
        // run on the CI runner's `/usr/bin/python3`. uv's proof below is the one
        // with an environment in it.
        filling: "{python3} -m pip install --index-url {registry}/simple/ \
                  --trusted-host 127.0.0.1 --target ./installed greet-from-the-store",
        offline: "{python3} -m pip install --no-index \
                  --trusted-host 127.0.0.1 --target ./installed {wheel}",
        running: "PYTHONPATH=./installed {python3} -c \
                  \"import greet_from_the_store as it; print(it.GREETING)\"",
        denied: "Connection refused",
        units: ("pip", Units::Hashed),
    })
    .await;
}

/// uv: `UV_CACHE_DIR`, and `--offline`, which uv documents as "disable network
/// access, relying only on locally cached data and locally available files".
///
/// Word for word the same install as the two that filled the store, plus that
/// one flag — which is the shape pip could not have. uv caches the index
/// response as well as the wheel and is content to resolve out of it offline, so
/// nothing here has to be pinned by URL.
///
/// `--python` names the interpreter absolutely, for [`found`]'s reason: a
/// sandbox's `PATH` is the machine's system profile, so the `python3` this suite
/// found is one it has to name in full. `UV_PYTHON_DOWNLOADS=never` beside it so
/// that a session short of an interpreter fails rather than fetching one from
/// astral.sh.
///
/// The control fails `was not found in the cache`, which is uv's own account of
/// a distribution it was told to find there and could not.
#[tokio::test]
async fn uv_fills_one_store_and_a_third_install_reads_it() {
    one_python_tools_store(Python {
        tool: "uv",
        wants: &["uv", "python3", "zip"],
        // `uv pip install` is pip's interface, so this one names what it is
        // installing on the command line too, and `uv lock` — which would write
        // a file — is a different verb from the one being proved.
        manifest: None,
        lockfile: None,
        settings: "export UV_PYTHON_DOWNLOADS=never UV_NO_PROGRESS=1",
        filling: "{uv} venv --python {python3}\n\
                  {uv} pip install --index-url {registry}/simple/ greet-from-the-store",
        offline: "{uv} venv --python {python3}\n\
                  {uv} pip install --offline --index-url {registry}/simple/ \
                  greet-from-the-store",
        running: "./.venv/bin/python -c \
                  \"import greet_from_the_store as it; print(it.GREETING)\"",
        denied: "was not found in the cache",
        units: ("uv", Units::Naming(&[PYPI_MODULE])),
    })
    .await;
}

/// uv's cache is the one beside the Worktrees — **and a session gets a copy out
/// of it rather than a hardlink, which uv says out loud.**
///
/// The same finding as
/// [`pnpms_store_is_beside_the_worktrees_and_a_session_copies_out_of_it`] and
/// [`deno_and_bun_link_out_of_their_stores_too_and_so_copy_in_a_sandbox`], for
/// the fourth and last tool that links out of a store: a Worktree and this
/// directory are two **bind mounts**, `link(2)` answers `EXDEV` across two
/// mounts even where both are the same filesystem, and no placement a descriptor
/// can name changes it. The store still works — a copy out of it fetches
/// nothing, which is what the install proof above turns on — and what is gone is
/// the disk space the link would have saved.
///
/// **uv is the second of the four to say so**, and it says it better than pnpm
/// does: *Failed to hardlink files; falling back to full copy*, with the
/// filesystem named as the likely reason and `UV_LINK_MODE=copy` offered as the
/// way to silence it. That variable is deliberately not set — see
/// `crates/server/languages.yaml` — because this warning is the truest account
/// of the thing this stage found, and the day one mount holds both the Worktree
/// and the store it stops being printed by itself.
///
/// The task this landed under expected the other answer: a link count above one
/// and uv saying it had linked. Asserted as measured instead, both halves, which
/// is what the next release has to be held against.
///
/// **And the virtual environment stayed in the Worktree**, which is the other
/// half of what Python's entry promises. A venv holds absolute paths, so one
/// built in another Worktree is broken in this one — nothing of it belongs in a
/// shared store, and what is asserted is that the `site-packages` uv installed
/// into is under this Conversation's checkout while the cache it installed out
/// of is beside it.
#[tokio::test]
async fn uvs_cache_is_beside_the_worktrees_and_it_says_it_could_not_hardlink() {
    let Some(tools) = tools("uv", &["uv", "python3", "zip"]) else {
        return;
    };
    let (uv, python3, zip) = (&tools[0], &tools[1], &tools[2]);

    let machine = machine(1).await;
    let cache = machine.cache();

    let registry = pypi_registry(&machine.registries.join("uv-links"), zip);

    let installed = installing(
        &machine.sandbox(0, &cache, vec![]),
        &format!(
            "set -e\n\
             export UV_PYTHON_DOWNLOADS=never UV_NO_PROGRESS=1\n\
             printf 'store=%s\\n' \"${{UV_CACHE_DIR-unset}}\"\n\
             {uv} venv --python {python3}\n\
             {uv} pip install --index-url {registry}/simple/ {package}\n\
             printf 'installed=%s\\n' \
               \"$(./.venv/bin/python -c \
                  'import {module}; print({module}.__file__)')\"\n",
            uv = uv.display(),
            python3 = python3.display(),
            registry = registry.url,
            package = PYPI_PACKAGE,
            module = PYPI_MODULE,
        ),
    );

    registry.shut();

    installed.worked("a session installs the distribution the index is serving");

    assert_eq!(
        line(&installed, "store"),
        machine
            .stores_beside("shared")
            .join("uv")
            .display()
            .to_string(),
        "uv's cache is under the directory beside the Worktrees rather than \
         under the Build Cache, which is what {{stores}} means. The session \
         said:\n{}",
        installed.said,
    );

    // Read off the interpreter rather than composed here: the directory under
    // `.venv/lib` is named for the Python that made it, and which Python that
    // is is the machine's business.
    let inside = PathBuf::from(line(&installed, "installed"));
    let worktree = machine.worktree(0);

    assert!(
        inside.starts_with(worktree),
        "the environment uv installed into is inside this Conversation's \
         Worktree, which is where a venv has to stay: {} is not under {}",
        inside.display(),
        worktree.display(),
    );

    let count = std::fs::metadata(&inside)
        .unwrap_or_else(|error| {
            panic!(
                "{} should be there after a uv install ({error}), and it said:\n{}",
                inside.display(),
                installed.said,
            )
        })
        .nlink();

    assert_eq!(
        count,
        1,
        "the file in the Worktree is a copy of what was in uv's cache rather \
         than a link to it — which is what the two of them being two bind \
         mounts comes to, and what would have to change for this to read above \
         one is the sandbox rather than this descriptor. {} said:\n{}",
        inside.display(),
        installed.said,
    );

    assert!(
        installed
            .said
            .contains("Failed to hardlink files; falling back to full copy"),
        "and uv says so itself, which is the half of this evidence that does \
         not depend on how the two directories happen to be laid out — and the \
         half deno and bun do not give. It said:\n{}",
        installed.said,
    );
}

/// poetry: `POETRY_CACHE_DIR`, `POETRY_VIRTUALENVS_IN_PROJECT`, and **no offline
/// flag at all**, which is bun's shape rather than pip's.
///
/// `poetry install --help` lists nothing that says *cache only*, and poetry
/// needs nothing: the third install is the second **word for word**, against an
/// index that has stopped answering. Which is the plainer proof of the two —
/// there is no flag here that could be doing the work instead of the store.
///
/// **Two directories under that one variable, and an install offline needs
/// both**, which is pnpm's finding with one variable rather than two.
/// `artifacts` holds the distributions poetry downloaded and
/// `cache/repositories/<source>` the release information it resolved out of the
/// index — and an install handed the artifacts and not the repositories cache
/// goes to the network for the file's URL and fails. Measured rather than read:
/// both halves are under `POETRY_CACHE_DIR`, so what a session is given covers
/// it, but a descriptor naming a directory *inside* poetry's cache would have
/// covered half.
///
/// **What it installs into is `.venv` in the Worktree**, which is the other
/// variable. See
/// [`poetrys_environment_stays_in_the_worktree_and_the_store_holds_no_venv`] for
/// what that is there to stop.
///
/// `poetry env use` names the interpreter absolutely, for [`found`]'s reason: a
/// sandbox's `PATH` is the machine's system profile, so the `python3` this suite
/// found is one it has to name in full — and poetry left to itself takes
/// whatever `python3` that `PATH` happens to hold.
///
/// The control fails on `All attempts to connect to`, which is poetry's own
/// account of an index it had to reach and could not.
#[tokio::test]
async fn poetry_fills_one_cache_and_a_third_install_reads_it() {
    one_python_tools_store(Python {
        tool: "poetry",
        wants: &["poetry", "python3", "zip"],
        // A Repo's own project file, which is where poetry is told both what to
        // install and which index to install it from: poetry has no
        // `--index-url`, a source being a thing a project declares.
        //
        // `package-mode = false` so that what is installed is the dependency and
        // not the project itself, which has no package to build.
        manifest: Some((
            "pyproject.toml",
            "[project]\n\
             name = \"app\"\n\
             version = \"1.0.0\"\n\
             requires-python = \">=3.8\"\n\
             dependencies = [\"greet-from-the-store==1.0.0\"]\n\
             \n\
             [tool.poetry]\n\
             package-mode = false\n\
             \n\
             [[tool.poetry.source]]\n\
             name = \"the-one-this-proof-serves\"\n\
             url = \"{registry}/simple/\"\n\
             priority = \"primary\"\n",
        )),
        lockfile: Some("poetry.lock"),
        // Each of these takes something out of the way rather than moving a
        // store: nothing is asked of a human, no keyring is consulted for a
        // registry on the loopback, and virtualenv does not ask PyPI in the
        // background whether there is a newer pip to seed the environment with.
        settings: "export POETRY_NO_INTERACTION=1 POETRY_KEYRING_ENABLED=false \
                   VIRTUALENV_NO_PERIODIC_UPDATE=1",
        filling: "{poetry} env use {python3}\n{poetry} install",
        offline: "{poetry} env use {python3}\n{poetry} install",
        running: "./.venv/bin/python -c \
                  \"import greet_from_the_store as it; print(it.GREETING)\"",
        denied: "All attempts to connect to",
        units: ("poetry", Units::Hashed),
    })
    .await;
}

/// pipenv: `PIPENV_CACHE_DIR`, `PIPENV_VENV_IN_PROJECT`, and no offline flag
/// either — so the third install is the second word for word here as well.
///
/// **pipenv's store is pip's, and that is the thing this proof is really
/// about.** pipenv runs pip in an environment of its own making:
/// `PIP_CACHE_DIR` pointed at `PIPENV_CACHE_DIR`, `PIP_CONFIG_FILE` at the null
/// device, and not one other `PIP_` of the session's carried through. So the
/// `PIP_CACHE_DIR` the descriptor sets does nothing for a pipenv install, and
/// leaving pipenv's out would leave every pipenv install downloading into the
/// empty home each session gets — which no assertion about the environment could
/// have caught, and which this install is what catches.
///
/// And because the store is pip's, it is an **HTTP cache** with pip's limits:
/// what it can serve with nothing on the air is a distribution already asked for
/// by URL rather than one named by name. So the `Pipfile` pins the wheel by its
/// URL, exactly as [`pip_fills_one_cache_and_a_third_install_reads_it`] pins it
/// on the command line and for the same measured reason. `verify_ssl = false` is
/// what makes pipenv name the loopback trusted to pip, which is the other half
/// of that finding: pip mounts a non-caching adapter for plain `http://` and the
/// caching one only for a host that was named.
///
/// **And the `Pipfile` names the index the distribution comes from**, which is
/// one line that keeps this proof off the internet altogether. pipenv collects
/// hashes as it locks, and which way it goes about it turns on the index URL it
/// matched for the entry: a `[packages]` entry with no `index` key matches no
/// declared source, so the URL falls back to `https://pypi.org/simple/` and
/// pipenv asks `https://pypi.org/pypi/<name>/json` — a *hardcoded* host that
/// `PIPENV_PYPI_MIRROR` does not move. Naming the source sends it to
/// `get_hashes_from_remote_index_urls` instead, which asks the index this proof
/// serves.
///
/// **It passed either way on a machine with a resolver**, which is why it took a
/// runner to find: pypi.org answers 404 for a distribution only this suite has
/// ever heard of, `r.json()["releases"]` raises `KeyError`, and pipenv catches
/// that and shrugs. Where the name does not resolve it raises requests'
/// `ConnectionError`, which is not Python's builtin of that name and so is *not*
/// what the `except` beside it catches — so the lock dies. Reproduced here by
/// binding an empty `resolv.conf` over `/etc/resolv.conf` and running the suite
/// inside that, which is the condition a sandbox with no DNS is in; all of it
/// passes that way now, and this proof did not before.
///
/// The control fails on `Connection refused`, which is pip having had to fetch.
#[tokio::test]
async fn pipenv_fills_one_cache_and_a_third_install_reads_it() {
    one_python_tools_store(Python {
        tool: "pipenv",
        wants: &["pipenv", "python3", "zip"],
        manifest: Some((
            "Pipfile",
            "[[source]]\n\
             name = \"the-one-this-proof-serves\"\n\
             url = \"{registry}/simple/\"\n\
             verify_ssl = false\n\
             \n\
             [packages]\n\
             greet-from-the-store = {file = \"{wheel}\", \
             index = \"the-one-this-proof-serves\"}\n",
        )),
        lockfile: Some("Pipfile.lock"),
        settings: "export VIRTUALENV_NO_PERIODIC_UPDATE=1",
        filling: "{pipenv} install --python {python3}",
        offline: "{pipenv} install --python {python3}",
        running: "./.venv/bin/python -c \
                  \"import greet_from_the_store as it; print(it.GREETING)\"",
        denied: "Connection refused",
        units: ("pipenv", Units::Hashed),
    })
    .await;
}

/// Anything under `dir` that is a virtual environment, which is the one file
/// every one of them has: `pyvenv.cfg`, at the root of the environment.
///
/// Walked without following a link, because an environment is full of them —
/// `lib64` pointing at `lib` is the ordinary case — and a walk that followed one
/// would be a walk that could go round.
fn a_virtualenv_under(dir: &Path) -> Option<PathBuf> {
    let mut looking = vec![dir.to_owned()];

    while let Some(here) = looking.pop() {
        for entry in std::fs::read_dir(&here).into_iter().flatten().flatten() {
            let Ok(what) = entry.file_type() else {
                continue;
            };

            if what.is_dir() {
                looking.push(entry.path());
            } else if what.is_file() && entry.file_name() == "pyvenv.cfg" {
                return Some(entry.path());
            }
        }
    }

    None
}

/// What one of the two tools that had to be told where to keep an environment is
/// asked, once it has really installed something.
struct Environment {
    /// What the skip line names and the assertions call it.
    tool: &'static str,

    /// The programs it takes, standing in the script as `{its-own-name}`.
    wants: &'static [&'static str],

    /// The manifest its Worktree holds, with `{registry}` and `{wheel}` in it.
    manifest: (&'static str, &'static str),

    /// The variable that says where its downloads go, and what the session is
    /// expected to have been given for it — which is a directory named under the
    /// one Build Cache.
    store: (&'static str, &'static str),

    /// And the install, which has to end with the distribution installed into an
    /// environment of the Worktree's.
    installing: &'static str,
}

/// One tool's environment: **in the Worktree, and nothing of it in the shared
/// store** — asserted by looking in both.
///
/// This is what the second of each tool's two variables is for, and it is worth a
/// proof of its own because it is the half no install proof can see. A venv holds
/// absolute paths, so one built in another Worktree is broken in this one, and
/// two sessions racing on a shared one is worse than a cold install — and
/// **poetry's default place for one is the cache directory itself**
/// (`virtualenvs.path` is `{cache-dir}/virtualenvs`), which is the shared store.
/// So a session that was given the cache variable and not the setting beside it
/// would be a session handing its environment to every other Conversation on the
/// Repo, and every install proof would go on passing.
///
/// Both halves, because either on its own is half an answer: the module really
/// imported out of an environment under this Conversation's own checkout, and no
/// `pyvenv.cfg` anywhere under the Build Cache or the directory beside the
/// Worktrees. The second is the one that would catch a release that started
/// putting something else of an environment's there.
async fn one_tools_environment(proof: Environment) {
    let Some(found_them) = tools(proof.tool, proof.wants) else {
        return;
    };

    let machine = machine(1).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let registry = pypi_registry(
        &machine
            .registries
            .join(format!("{}-environment", proof.tool)),
        &found_them[proof
            .wants
            .iter()
            .position(|program| *program == "zip")
            .expect("every Python proof builds its wheel with `zip`")],
    );

    let named = |script: &str| {
        let mut said = script
            .replace("{registry}", &registry.url)
            .replace("{wheel}", &format!("{}/files/{PYPI_WHEEL}", registry.url));

        for (program, path) in proof.wants.iter().zip(&found_them) {
            said = said.replace(&format!("{{{program}}}"), &path.display().to_string());
        }

        said
    };

    let (called, written) = proof.manifest;
    std::fs::write(machine.worktree(0).join(called), named(written)).unwrap();

    let (variable, under) = proof.store;

    let installed = installing(
        &machine.sandbox(0, &cache, vec![]),
        &named(&format!(
            "set -e\n\
             printf 'store=%s\\n' \"${{{variable}-unset}}\"\n\
             {installing}\n\
             installed=\"$(./.venv/bin/python -c \
               'import {PYPI_MODULE}; print({PYPI_MODULE}.__file__)')\"\n\
             printf 'installed=%s\\n' \"$installed\"\n",
            installing = proof.installing,
        )),
    );

    registry.shut();

    installed.worked(&format!(
        "a {} session installs the distribution the index is serving",
        proof.tool
    ));

    assert_eq!(
        line(&installed, "store"),
        dir.join(under).display().to_string(),
        "{}'s downloads go under the one Build Cache, which is the half of this \
         that is shared. The session said:\n{}",
        proof.tool,
        installed.said,
    );

    // Read off the interpreter rather than composed here, for the reason uv's is:
    // the directory under `.venv/lib` is named for the Python that made it, and
    // which Python that is is the machine's business.
    let inside = PathBuf::from(line(&installed, "installed"));
    let worktree = machine.worktree(0);

    assert!(
        inside.starts_with(worktree),
        "the environment {} installed into is inside this Conversation's \
         Worktree, which is where a venv has to stay: {} is not under {}. It \
         said:\n{}",
        proof.tool,
        inside.display(),
        worktree.display(),
        installed.said,
    );
    assert!(
        inside.starts_with(worktree.join(".venv")),
        "and it is the `.venv` beside the manifest rather than somewhere else in \
         the checkout, which is what the setting names: {}",
        inside.display(),
    );

    // And the other half: nothing of an environment in either shared directory.
    for shared in [dir.to_owned(), machine.stores_beside("shared")] {
        assert_eq!(
            a_virtualenv_under(&shared),
            None,
            "and no environment of {}'s is in the shared {}: a venv holds \
             absolute paths, so one left there is one the next Conversation \
             would find broken. It said:\n{}",
            proof.tool,
            shared.display(),
            installed.said,
        );
    }
}

/// poetry's environment: in the Worktree, and its cache holds none.
///
/// The one this matters most for. poetry keeps its virtual environments under
/// `cache-dir` unless it is told otherwise — measured, not read — so the
/// descriptor's `POETRY_VIRTUALENVS_IN_PROJECT` is the whole of what keeps a
/// Conversation's environment out of the directory every other Conversation is
/// given. See [`one_tools_environment`].
#[tokio::test]
async fn poetrys_environment_stays_in_the_worktree_and_the_store_holds_no_venv() {
    one_tools_environment(Environment {
        tool: "poetry",
        wants: &["poetry", "python3", "zip"],
        manifest: (
            "pyproject.toml",
            "[project]\n\
             name = \"app\"\n\
             version = \"1.0.0\"\n\
             requires-python = \">=3.8\"\n\
             dependencies = [\"greet-from-the-store==1.0.0\"]\n\
             \n\
             [tool.poetry]\n\
             package-mode = false\n\
             \n\
             [[tool.poetry.source]]\n\
             name = \"the-one-this-proof-serves\"\n\
             url = \"{registry}/simple/\"\n\
             priority = \"primary\"\n",
        ),
        store: ("POETRY_CACHE_DIR", "poetry"),
        installing: "export POETRY_NO_INTERACTION=1 POETRY_KEYRING_ENABLED=false \
                     VIRTUALENV_NO_PERIODIC_UPDATE=1\n\
                     {poetry} env use {python3}\n\
                     {poetry} install",
    })
    .await;
}

/// And pipenv's, where the premise was half wrong and the answer is the same.
///
/// pipenv does not keep its environments under its cache variable the way poetry
/// does: they go to `WORKON_HOME`, which is under the session's own home. So a
/// shared cache was never a shared venv here, and what
/// `PIPENV_VENV_IN_PROJECT` buys is that the environment outlives the session
/// that made it — a home being a thing a sandbox throws away. Asserted the same
/// way for both, because *in the Worktree and not in the store* is what the
/// descriptor promises whichever of the two reasons it is true for.
#[tokio::test]
async fn pipenvs_environment_stays_in_the_worktree_and_the_store_holds_no_venv() {
    one_tools_environment(Environment {
        tool: "pipenv",
        wants: &["pipenv", "python3", "zip"],
        manifest: (
            "Pipfile",
            "[[source]]\n\
             name = \"the-one-this-proof-serves\"\n\
             url = \"{registry}/simple/\"\n\
             verify_ssl = false\n\
             \n\
             [packages]\n\
             greet-from-the-store = {file = \"{wheel}\", \
             index = \"the-one-this-proof-serves\"}\n",
        ),
        store: ("PIPENV_CACHE_DIR", "pipenv"),
        installing: "export VIRTUALENV_NO_PERIODIC_UPDATE=1\n\
                     {pipenv} install --python {python3}",
    })
    .await;
}

/// And the environment those installs ran in was the descriptor's: Python's six
/// variables — three of the four stores under the one Build Cache, uv's beside
/// the Worktrees, and the two settings that are not directories at all — with a
/// session able to write in both directories.
///
/// Beside the proofs rather than inside them, for the reason Go's and Node's
/// are: what a session is *told* is asserted on all three platforms in the
/// sandbox suites, and what a tool *does* with it is what an install is for.
/// This is here so that a run with none of the four tools installed still leaves
/// something of Python's in this file that ran.
#[tokio::test]
async fn a_session_is_given_the_four_python_tools_stores() {
    let machine = machine(1).await;
    let cache = machine.cache();
    let sandbox = machine.sandbox(0, &cache, vec![]);

    let dir = cache.dir().expect("the fixture's cache has a directory");
    let beside = machine.stores_beside("shared");

    let reported = installing(
        &sandbox,
        "set -e\n\
         for named in PIP_CACHE_DIR UV_CACHE_DIR POETRY_CACHE_DIR PIPENV_CACHE_DIR; do\n\
           eval \"value=\\${$named-unset}\"\n\
           printf '%s=%s\\n' \"$named\" \"$value\"\n\
           mkdir -p \"$value\"\n\
           : > \"$value/written-from-inside\"\n\
         done\n\
         for named in POETRY_VIRTUALENVS_IN_PROJECT PIPENV_VENV_IN_PROJECT; do\n\
           eval \"value=\\${$named-unset}\"\n\
           printf '%s=%s\\n' \"$named\" \"$value\"\n\
         done\n",
    );

    reported.worked("a session can make and write every directory it is pointed at");

    // The three under the one Build Cache, which is where a store goes that
    // nothing links a package out of: pip unpacks a wheel into `site-packages`,
    // poetry unpacks one out of its artifacts, and pipenv is pip.
    for (named, under) in [
        ("PIP_CACHE_DIR", "pip"),
        ("POETRY_CACHE_DIR", "poetry"),
        ("PIPENV_CACHE_DIR", "pipenv"),
    ] {
        assert_eq!(
            line(&reported, named),
            dir.join(under).display().to_string(),
            "{named} is {under} inside the one Build Cache. The session said:\n{}",
            reported.said,
        );
        assert!(
            dir.join(under).join("written-from-inside").is_file(),
            "and what the session wrote under {under} is on the host, in the \
             directory the next Conversation's session will be given",
        );
    }

    assert_eq!(
        line(&reported, "UV_CACHE_DIR"),
        beside.join("uv").display().to_string(),
        "and uv's is beside the Worktrees rather than in the cache, because uv \
         links out of it. The session said:\n{}",
        reported.said,
    );
    assert!(
        beside.join("uv/written-from-inside").is_file(),
        "which is the second bind a session gets, and it is writable under uv's \
         name too",
    );

    // And the two that name no directory: poetry and pipenv each told to keep a
    // virtual environment in the project, which is why there is no fifth
    // directory here for either of them to have kept one in.
    assert_eq!(
        line(&reported, "POETRY_VIRTUALENVS_IN_PROJECT"),
        "true",
        "poetry would otherwise keep its environments under the cache directory \
         above, which every Conversation on the Repo is given. The session \
         said:\n{}",
        reported.said,
    );
    assert_eq!(
        line(&reported, "PIPENV_VENV_IN_PROJECT"),
        "1",
        "and pipenv's, which is the same promise for the tool that would \
         otherwise keep one under a home the sandbox throws away",
    );
}

// ---------------------------------------------------------------------------
// .NET: NuGet.
// ---------------------------------------------------------------------------

/// The package every .NET proof installs, and the version of it.
///
/// Two spellings because NuGet has two: a package is *named* however its author
/// capitalised it and *addressed* in lower case, the flat container's paths and
/// the global packages folder's directories both being the lower-cased id.
const NUGET_PACKAGE: &str = "GreetFromTheStore";
const NUGET_ID: &str = "greetfromthestore";
const NUGET_VERSION: &str = "1.0.0";

/// What every `dotnet` in this suite is told, beside the two variables the
/// descriptor sets — which are the two this is here to prove.
///
/// Each of these takes something out of the way rather than moving a store.
/// `DOTNET_CLI_TELEMETRY_OPTOUT` so that nothing here talks to Microsoft,
/// `DOTNET_NOLOGO` and the two first-run switches so that a session's first
/// `dotnet` is an install rather than a welcome banner and an HTTPS
/// certificate, and `DOTNET_SKIP_WORKLOAD_INTEGRITY_CHECK` so that a warning
/// about workloads nothing here has is not in the middle of what a failure has
/// to be read out of.
///
/// **And the last three are why a proof finishes at all.** MSBuild leaves
/// worker nodes and Roslyn leaves a compiler server behind after a build, both
/// of them waiting a quarter of an hour for the next one — and a sandbox is
/// over when the last process in it is, so a session that left either would be
/// an install this suite waited fifteen minutes for. `bwrap` has no more to say
/// about it than `wait` does: the two switches and the property are what say
/// not to start them.
const DOTNET_SETTINGS: &str = "export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 \
                               DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 \
                               DOTNET_GENERATE_ASPNET_CERTIFICATE=false \
                               DOTNET_SKIP_WORKLOAD_INTEGRITY_CHECK=1 \
                               DOTNET_CLI_USE_MSBUILD_SERVER=0 MSBUILDDISABLENODEREUSE=1 \
                               UseSharedCompilation=false";

/// Lay [`NUGET_PACKAGE`] out under `at` as a `.nupkg`, and serve it.
///
/// **A `.nupkg` is a zip**, exactly as a Go module archive and a wheel are, so
/// this is built with the suite's own `zip` and nothing of .NET's own is needed
/// to publish. Three entries and no compiled anything: the `.nuspec` that says
/// what the package is, one C# source file, and a `build/<id>.targets` — which
/// NuGet imports into any project that references the package, and which here
/// adds that source file to the compile. So what the consumer below builds is
/// code that came out of the package, which is what makes running it evidence
/// that the store held the package rather than something shaped like it.
///
/// **Served over the loopback rather than laid out as a folder feed.** NuGet
/// reads a directory of `.nupkg` files as a source perfectly well, and the Go
/// proof's `file://` proxy is the precedent for using one — but a folder feed
/// is never fetched, so it would leave the http cache empty and half of this
/// descriptor unproved. What is served is the **v3 protocol's flat container**,
/// which is the whole of what restoring one package at an exact version reads:
/// a service index naming the base address, a version list per package, and the
/// `.nupkg` at a path composed out of the lower-cased id and version.
fn nuget_registry(at: &Path, zip: &Path) -> Registry {
    let inside = at.join("what-goes-in-the-nupkg");
    std::fs::create_dir_all(inside.join("build")).unwrap();
    std::fs::create_dir_all(inside.join("src")).unwrap();

    std::fs::write(
        inside.join(format!("{NUGET_PACKAGE}.nuspec")),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
             <package xmlns=\"http://schemas.microsoft.com/packaging/2012/06/nuspec.xsd\">\n\
             \x20 <metadata>\n\
             \x20   <id>{NUGET_PACKAGE}</id>\n\
             \x20   <version>{NUGET_VERSION}</version>\n\
             \x20   <authors>this suite</authors>\n\
             \x20   <description>What a package-store proof installs.</description>\n\
             \x20 </metadata>\n\
             </package>\n"
        ),
    )
    .unwrap();
    std::fs::write(
        inside.join(format!("build/{NUGET_PACKAGE}.targets")),
        "<Project>\n\
         \x20 <ItemGroup>\n\
         \x20   <Compile Include=\"$(MSBuildThisFileDirectory)../src/Greet.cs\" />\n\
         \x20 </ItemGroup>\n\
         </Project>\n",
    )
    .unwrap();
    std::fs::write(
        inside.join("src/Greet.cs"),
        format!(
            "public static class Greet {{ public static string Hello() => \
             \"{OUT_OF_THE_STORE}\"; }}\n"
        ),
    )
    .unwrap();

    let archive = at.join(format!("{NUGET_ID}.{NUGET_VERSION}.nupkg"));
    let made = Command::new(zip)
        .args(["-q", "-r", "-X", "-D"])
        .arg(&archive)
        .args([
            format!("{NUGET_PACKAGE}.nuspec"),
            "build".into(),
            "src".into(),
        ])
        .current_dir(&inside)
        .stdin(Stdio::null())
        .status()
        .expect("the suite's own `zip`");

    assert!(made.success(), "the package was not built");

    // Bound here rather than on the thread below, for the reason the npm
    // registry's is: the port it gets goes in the service index, so it has to
    // be known before anything is served.
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .expect("a loopback port for the registry");
    let url = format!("http://{}", listener.local_addr().expect("the port it got"));

    listener
        .set_nonblocking(true)
        .expect("what tokio takes a standard listener over");

    let served = Feed {
        index: format!(
            "{{\"version\":\"3.0.0\",\"resources\":[{{\"@id\":\"{url}/flat/\",\
              \"@type\":\"PackageBaseAddress/3.0.0\"}}]}}"
        ),
        versions: format!("{{\"versions\":[\"{NUGET_VERSION}\"]}}"),
        nupkg: std::fs::read(&archive).unwrap(),
    };

    let app = Router::new()
        .route(
            "/index.json",
            get(|State(served): State<Feed>| async move {
                ([(header::CONTENT_TYPE, "application/json")], served.index)
            }),
        )
        .route(
            &format!("/flat/{NUGET_ID}/index.json"),
            get(|State(served): State<Feed>| async move {
                (
                    [(header::CONTENT_TYPE, "application/json")],
                    served.versions,
                )
            }),
        )
        .route(
            &format!("/flat/{NUGET_ID}/{NUGET_VERSION}/{{nupkg}}"),
            get(|State(served): State<Feed>| async move {
                (
                    [(header::CONTENT_TYPE, "application/octet-stream")],
                    served.nupkg,
                )
            }),
        )
        .with_state(served);

    let (stop, stopping) = tokio::sync::oneshot::channel();

    let serving = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for the registry's own thread");

        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener)
                .expect("the listener this thread was handed");

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

/// The three things a v3 feed has to answer with, fixed once it is listening.
#[derive(Clone)]
struct Feed {
    index: String,
    versions: String,
    nupkg: Vec<u8>,
}

/// And what a Conversation's Worktree holds: a project referencing the package
/// in the registry, so that restoring it is a fetch.
///
/// `targeting` is [`dotnet_target`]'s answer rather than a framework written
/// here, for the reason given there.
///
/// **`NuGet.config` is where *denied its registry* lives for this tool.** Every
/// .NET machine has a user-level config naming nuget.org, written by the first
/// `dotnet` to run — so a project that said nothing about sources would restore
/// off the internet, a session's sandbox sharing the host's network. `<clear />`
/// is what takes every inherited source away, and `from` is the one this proof
/// serves put back where a Worktree is allowed it. A Worktree given `None` has
/// no source at all, which is the state the third restore has to succeed in.
///
/// `allowInsecureConnections` because a registry on this machine is not one
/// with a certificate: it is Berry's `YARN_UNSAFE_HTTP_WHITELIST` and pip's
/// `--trusted-host` again, and for the same reason. The SDKs this runs against
/// only warn about plain HTTP; newer ones refuse it without this.
fn nuget_consumer(worktree: &Path, targeting: &str, from: Option<&str>) {
    std::fs::write(
        worktree.join("app.csproj"),
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n\
             \x20 <PropertyGroup>\n\
             \x20   <OutputType>Exe</OutputType>\n\
             \x20   <TargetFramework>{targeting}</TargetFramework>\n\
             \x20 </PropertyGroup>\n\
             \x20 <ItemGroup>\n\
             \x20   <PackageReference Include=\"{NUGET_PACKAGE}\" Version=\"{NUGET_VERSION}\" />\n\
             \x20 </ItemGroup>\n\
             </Project>\n"
        ),
    )
    .unwrap();
    std::fs::write(
        worktree.join("Program.cs"),
        "public static class Program {\n\
         \x20 public static void Main() => System.Console.WriteLine(Greet.Hello());\n\
         }\n",
    )
    .unwrap();
    std::fs::write(
        worktree.join("NuGet.config"),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
             <configuration>\n\
             \x20 <packageSources>\n\
             \x20   <clear />\n\
             {named}\
             \x20 </packageSources>\n\
             </configuration>\n",
            named = from.map_or_else(String::new, |url| format!(
                "\x20   <add key=\"the-one-this-proof-serves\" value=\"{url}/index.json\" \
                 protocolVersion=\"3\" allowInsecureConnections=\"true\" />\n"
            )),
        ),
    )
    .unwrap();
}

/// The framework a proof's project targets: whichever one the SDK on this
/// machine is its own.
///
/// **Not a version written into this file**, and that is the whole of why this
/// function exists. An SDK ships the targeting pack for its own framework and
/// fetches one for any older framework as a NuGet package — from nuget.org,
/// which every Worktree here has cleared. So a project pinned to the dev
/// shell's framework would restore here and fail on a runner image that had
/// moved a major on, in a proof about package stores, for a reason that is
/// nothing to do with one. `dotnet --version` is the SDK's own version and its
/// major is the framework it needs to fetch nothing at all to build.
///
/// A `HOME` of the fixture's rather than the machine's, because a first
/// `dotnet` writes sentinels into one, and a suite that wrote into the
/// developer's home to read a version back would be doing something it was
/// never asked to.
fn dotnet_target(dotnet: &Path, home: &Path) -> String {
    let said = Command::new(dotnet)
        .arg("--version")
        .current_dir(home)
        .env("HOME", home)
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
        .env("DOTNET_GENERATE_ASPNET_CERTIFICATE", "false")
        .stdin(Stdio::null())
        .output()
        .expect("the `dotnet` this machine was found to have");

    assert!(
        said.status.success(),
        "`dotnet --version` should answer: {}",
        String::from_utf8_lossy(&said.stderr),
    );

    let version = String::from_utf8(said.stdout).expect("a version is ASCII");
    let major = version
        .trim()
        .split('.')
        .next()
        .filter(|major| major.chars().all(|digit| digit.is_ascii_digit()))
        .unwrap_or_else(|| panic!("`dotnet --version` said {version:?}, which has no major in it"));

    format!("net{major}.0")
}

/// What a `dotnet` proof runs: a restore, then the project it restored.
///
/// One script for every Sandbox in this section, filling and denied alike —
/// which is the shape bun's and poetry's proofs have and the plainest of the
/// three. NuGet documents no offline switch: `dotnet restore` has
/// `--no-http-cache`, which is the opposite of what is wanted, and nothing that
/// says *store only*. So what changes between a restore that fetches and one
/// that must not is the machine — the sources the Worktree names, and whether
/// anything is answering — and never the command line. There is no flag here
/// that could be doing the work instead of the store.
fn nuget_script(dotnet: &Path) -> String {
    format!(
        "set -e\n{DOTNET_SETTINGS}\n{dotnet} restore\n{dotnet} run --no-restore\n",
        dotnet = dotnet.display(),
    )
}

/// .NET: two Sandboxes restoring one package at once against one store, a third
/// restoring it with every source taken away, and the control that says the
/// third proved something.
///
/// **`NUGET_PACKAGES` is the store**, and what makes the third restore work is
/// that NuGet resolves a `<PackageReference>` at an exact version out of the
/// global packages folder before it asks a source anything — so a project whose
/// package is already there restores with no source configured at all. Which is
/// what the third Worktree has: a `NuGet.config` holding `<clear />` and nothing
/// after it, against a registry that has stopped answering as well.
///
/// **The two at once are the other half, and they are what found the third
/// variable.** NuGet does guard the global packages folder — it extracts a
/// package under a lock before it renames anything into place — but that lock
/// is a *file*, in NuGet's temp directory, and a session's `/tmp` is a tmpfs
/// of its own inside its sandbox. So the two below took a lock apiece, both
/// extracted, and one of them failed on a temporary file the other had renamed
/// away: `Could not find file '<store>/greetfromthestore/1.0.0/<random>'`,
/// about three runs in five with this file's other tests running beside it.
/// `NUGET_SCRATCH` is what the descriptor answers with — the lock goes where
/// the store it guards is — and dropping it turns this red again.
///
/// Which is also why it is not the only thing guarding that variable. A race
/// caught three runs in five is a poor guard, so
/// [`the_directories_nuget_names_for_itself_are_the_ones_the_descriptor_set`]
/// asks NuGet where its temp directory is and gets a straight answer.
///
/// The control fails `NU1100`, which is NuGet's own word for a package it could
/// resolve from nowhere.
#[tokio::test]
async fn two_dotnet_restores_at_once_fill_one_store_and_a_third_restores_out_of_it() {
    let Some(found_them) = tools(".NET", &["dotnet", "zip"]) else {
        return;
    };
    let (dotnet, zip) = (&found_them[0], &found_them[1]);

    // Five: the two that race, the one denied its registry, the control, and
    // the one that fills the store for the sweep before any of them.
    let machine = machine(5).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let targeting = dotnet_target(dotnet, &machine.registries);
    let registry = nuget_registry(&machine.registries.join("nuget"), zip);

    // The two that fill the store name the feed; the two that follow have every
    // source taken away, which is *denied its registry* for a tool with no
    // offline flag of its own.
    for nth in [0, 1, 4] {
        nuget_consumer(machine.worktree(nth), &targeting, Some(&registry.url));
    }

    for nth in 2..4 {
        nuget_consumer(machine.worktree(nth), &targeting, None);
    }

    let restoring = nuget_script(dotnet);

    // A store the sweep has just taken units out of, which is what the two
    // below start from.
    filled_and_swept(
        &machine,
        4,
        &cache,
        reaching_nothing(),
        &restoring,
        verkstead_server::languages::DOTNET,
    );

    // Started together and waited on together, which is the only way the two
    // are ever really writing the store at the same moment.
    let first = starting(&machine.sandbox(0, &cache, reaching_nothing()), &restoring);
    let second = starting(&machine.sandbox(1, &cache, reaching_nothing()), &restoring);

    let first = finished(first);

    first.worked("the first session's restore fills the store");
    finished(second).worked("and the second one racing it finishes just as well");

    assert!(
        dir.join(format!("nuget/packages/{NUGET_ID}/{NUGET_VERSION}"))
            .is_dir(),
        "and what they downloaded is under the directory the descriptor named, \
         which is the one the third session is about to be given. It said:\n{}",
        first.said,
    );

    // And the proof. The registry stops answering and the Worktree names no
    // source: what is left to restore out of is the store the two above filled.
    registry.shut();

    let third = installing(&machine.sandbox(2, &cache, reaching_nothing()), &restoring);

    third.worked(
        "a third session restores the same package with every source taken away, which it \
         can only do out of the shared global packages folder",
    );
    assert!(
        third.said.contains(OUT_OF_THE_STORE),
        "and what it built really runs, so the store held the package rather \
         than something shaped like it. It said:\n{}",
        third.said,
    );

    // And what the restores left is units the sweep can take out whole: a
    // version of the package, and a response of the http cache's at a time.
    whole_packages(
        &cache,
        verkstead_server::languages::DOTNET,
        "packages",
        Units::Naming(&[NUGET_ID]),
    );
    whole_packages(
        &cache,
        verkstead_server::languages::DOTNET,
        "http",
        Units::Hashed,
    );

    // The control. Everything the same but the Build Cache, which nothing has
    // filled — so a pass here would mean the restore above needed no store at
    // all, and the proof was asserting nothing.
    let control = installing(
        &machine.sandbox(3, &machine.empty_cache(), reaching_nothing()),
        &restoring,
    );

    assert!(
        !control.worked,
        "an empty store and no source has to fail, or the restore above proved \
         nothing about either. It said:\n{}",
        control.said,
    );
    assert!(
        control.said.contains("NU1100"),
        "and it fails for want of anywhere to get the package rather than for \
         some other reason: `NU1100` is what NuGet says about that. It \
         said:\n{}",
        control.said,
    );
}

/// And the **other** directory the descriptor names, which no store proof can
/// reach: the http cache, read when the global packages folder has gone.
///
/// This entry sets two variables and the proof above turns on one of them. A
/// descriptor naming `NUGET_HTTP_CACHE_PATH` and a NuGet that had stopped
/// reading it would leave every assertion up there green — the packages folder
/// answers first, so the http cache is never the thing a restore needs. Which
/// makes it exactly the branch this stage says a tool quietly ignoring its
/// variable would get through, and this is the run that closes it.
///
/// So: one session fills both halves off the registry. Then the registry stops
/// answering **and the packages folder is taken away on the host** — a fact
/// about the machine rather than anything the session was told, and the only
/// way to leave a session with one half of its Build Cache and not the other,
/// both being under the one directory. What is left for the second session to
/// restore out of is the responses NuGet cached under the variable this is
/// about: the service index, the version list, and the `.nupkg` as it came off
/// the wire.
///
/// The control is the same run on a Build Cache nothing has filled, where
/// neither half is there: it fails `NU1301`, NuGet's own word for a source it
/// had to reach and could not — which is what says the run above did not simply
/// need nothing.
#[tokio::test]
async fn the_http_cache_the_descriptor_names_answers_when_the_packages_folder_has_gone() {
    let Some(found_them) = tools(".NET", &["dotnet", "zip"]) else {
        return;
    };
    let (dotnet, zip) = (&found_them[0], &found_them[1]);

    // Three: the one that fills both halves, the one that reads the half that
    // is left, and the control.
    let machine = machine(3).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let targeting = dotnet_target(dotnet, &machine.registries);
    let registry = nuget_registry(&machine.registries.join("nuget-http"), zip);

    // Every one of them names the feed, which is the difference from the proof
    // above: what is denied here is the answer rather than the address.
    for nth in 0..3 {
        nuget_consumer(machine.worktree(nth), &targeting, Some(&registry.url));
    }

    let restoring = nuget_script(dotnet);

    let filled = installing(&machine.sandbox(0, &cache, reaching_nothing()), &restoring);

    filled.worked("the first session's restore fills both halves off the registry");

    let packages = dir.join("nuget/packages");
    let http = dir.join("nuget/http");

    assert!(
        holds_anything(&http),
        "and the responses went under the directory the descriptor named rather \
         than under the session's own home, which is what the rest of this is \
         about. It said:\n{}",
        filled.said,
    );

    registry.shut();

    // The half this is *not* about, taken away — so that what the next session
    // finds is the http cache and nothing else.
    std::fs::remove_dir_all(&packages).expect("the packages half of the Build Cache");

    let second = installing(&machine.sandbox(1, &cache, reaching_nothing()), &restoring);

    second.worked(
        "a second session restores with the packages folder gone and the registry off the \
         air, which it can only do out of the http cache the descriptor named",
    );
    assert!(
        second.said.contains(OUT_OF_THE_STORE),
        "and what it built really runs, so the cache held the package as it came \
         off the wire. It said:\n{}",
        second.said,
    );
    assert!(
        packages.is_dir(),
        "and it filled the packages folder back in out of it, which is what \
         restoring out of an http cache *is*",
    );

    // The control. The same Worktree and the same registry gone, on a Build
    // Cache where neither half was ever filled.
    let control = installing(
        &machine.sandbox(2, &machine.empty_cache(), reaching_nothing()),
        &restoring,
    );

    assert!(
        !control.worked,
        "an empty cache and a registry that is not answering has to fail, or \
         the restore above proved nothing about either. It said:\n{}",
        control.said,
    );
    assert!(
        control.said.contains("NU1301"),
        "and it fails for want of the registry rather than for some other \
         reason: `NU1301` is what NuGet says about a source it could not load. \
         It said:\n{}",
        control.said,
    );
}

/// Whether anything at all is under `at`, which is how the proof above says a
/// directory the descriptor named is the one a tool really wrote in.
fn holds_anything(at: &Path) -> bool {
    std::fs::read_dir(at).is_ok_and(|mut entries| entries.next().is_some())
}

/// What every .NET Sandbox in this section is opened onto beside its own: the
/// registry is served rather than laid out on disk, so the answer is nothing.
///
/// Said once and named, rather than `vec![]` in six places, because *nothing*
/// is the assertion: a Sandbox here reaches its registry over the loopback or
/// not at all, and there is no directory anywhere that could be quietly
/// standing in for one.
fn reaching_nothing() -> Vec<Bind> {
    Vec::new()
}

/// And the environment those restores ran in was the descriptor's: NuGet's
/// three directories inside the one Build Cache, and a session can write in
/// every one of them.
///
/// Beside the proofs rather than inside them, for the reason Go's, Node's and
/// Python's are: what a session is *told* is asserted on all three platforms in
/// the sandbox suites, and what a tool *does* with it is what a restore is for.
/// This is here so that a run on a machine with no .NET SDK still leaves
/// something of .NET's in this file that ran.
#[tokio::test]
async fn a_session_is_given_nugets_three_directories_inside_the_one_build_cache() {
    let machine = machine(1).await;
    let cache = machine.cache();
    let sandbox = machine.sandbox(0, &cache, reaching_nothing());

    let dir = cache.dir().expect("the fixture's cache has a directory");

    let reported = installing(
        &sandbox,
        "set -e\n\
         for named in NUGET_PACKAGES NUGET_HTTP_CACHE_PATH NUGET_SCRATCH; do\n\
           eval \"value=\\${$named-unset}\"\n\
           printf '%s=%s\\n' \"$named\" \"$value\"\n\
           mkdir -p \"$value\"\n\
           : > \"$value/written-from-inside\"\n\
         done\n",
    );

    reported.worked("a session can make and write every directory it is pointed at");

    for (named, under) in [
        ("NUGET_PACKAGES", "nuget/packages"),
        ("NUGET_HTTP_CACHE_PATH", "nuget/http"),
        ("NUGET_SCRATCH", "nuget/scratch"),
    ] {
        assert_eq!(
            line(&reported, named),
            dir.join(under).display().to_string(),
            "{named} is {under} inside the one Build Cache. The session said:\n{}",
            reported.said,
        );
        assert!(
            dir.join(under).join("written-from-inside").is_file(),
            "and what the session wrote under {under} is on the host, in the \
             directory the next Conversation's session will be given",
        );
    }
}

/// And nothing that is a session's own is in either of them: the configuration
/// NuGet keeps, and the credentials that go in it, stay in the session's home.
///
/// .NET is the entry where this needed asking, because its defaults put the two
/// next to each other. The global packages folder is `~/.nuget/packages` and
/// the user-level `NuGet.Config` — which is where `<packageSourceCredentials>`
/// lives — is `~/.nuget/NuGet/NuGet.Config`: one directory up, and a descriptor
/// that had moved `~/.nuget` rather than the folder under it would have handed
/// every Conversation on the machine one login. The http cache is the same
/// shape, `http-cache` beside `plugin-cache` under `~/.local/share/NuGet`.
///
/// So this restores for real and then looks in three places: the session's own
/// config is in its home, where a home that goes with the sandbox is what
/// keeps it a session's; there is no config of any kind under either shared
/// directory; and the plugins cache — which is a credential provider's account
/// of itself rather than a download — is not there either, that variable being
/// deliberately unset.
#[tokio::test]
async fn nothing_of_a_sessions_own_is_in_the_shared_nuget_directories() {
    let Some(found_them) = tools(".NET", &["dotnet", "zip"]) else {
        return;
    };
    let (dotnet, zip) = (&found_them[0], &found_them[1]);

    let machine = machine(1).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let targeting = dotnet_target(dotnet, &machine.registries);
    let registry = nuget_registry(&machine.registries.join("nuget-nothing-of-its-own"), zip);

    nuget_consumer(machine.worktree(0), &targeting, Some(&registry.url));

    let restored = installing(
        &machine.sandbox(0, &cache, reaching_nothing()),
        &format!(
            "{script}\
             printf 'own-config=%s\\n' \"$(ls \"$HOME/.nuget/NuGet/NuGet.Config\" 2>/dev/null \
               || echo none)\"\n",
            script = nuget_script(dotnet),
        ),
    );

    registry.shut();

    restored.worked("a session restores the package the registry is serving");

    assert_ne!(
        line(&restored, "own-config"),
        "none",
        "and it wrote a configuration of its own, which is the thing this is \
         about: without one there would be nothing that could have landed in \
         the wrong place. It said:\n{}",
        restored.said,
    );

    let shared = dir.join("nuget");

    assert_eq!(
        named_under(&shared, "NuGet.Config"),
        None,
        "and no configuration is under either shared directory: what a session \
         is given is the store, one level below the directory NuGet keeps its \
         own things in. It said:\n{}",
        restored.said,
    );
    assert_eq!(
        named_under(&shared, "plugin-cache"),
        None,
        "and neither is the plugins cache, which is a credential provider's \
         account of what it can do rather than anything downloaded — \
         `NUGET_PLUGINS_CACHE_PATH` is unset on purpose, so it goes to the home \
         the sandbox throws away",
    );
}

/// Whatever under `at` is called `named`, at any depth — file or directory.
///
/// A walk rather than a `join`, because what is being asked is whether a tool
/// put something anywhere in a directory a session shares, and where it might
/// have put it is exactly what nobody knows. [`a_virtualenv_under`] is the same
/// question asked of Python.
fn named_under(at: &Path, named: &str) -> Option<PathBuf> {
    let mut looking = vec![at.to_owned()];

    while let Some(here) = looking.pop() {
        for entry in std::fs::read_dir(&here).into_iter().flatten().flatten() {
            if entry.file_name() == named {
                return Some(entry.path());
            }

            if entry.file_type().is_ok_and(|what| what.is_dir()) {
                looking.push(entry.path());
            }
        }
    }

    None
}

/// And NuGet's own account of where its directories are, which is the
/// deterministic half of all three variables.
///
/// `dotnet nuget locals all --list` prints the four directories NuGet works
/// out of, and three of them are this descriptor's: the global packages
/// folder, the http cache, and the temp directory it takes a lock in. Asked of
/// the tool inside a session, so what comes back is what NuGet made of the
/// environment rather than what the environment said — the same question the
/// variables were read out of when this entry was written, asked again on
/// every run.
///
/// **This is what guards the third variable.** The two-at-once proof is what
/// found it — two sessions with a private `/tmp` each took a lock apiece and
/// wrote over one another — but a race caught three runs in five is a poor
/// guard against a release that stopped reading `NUGET_SCRATCH`, and this is
/// not a race at all.
///
/// And the fourth line is the other half of
/// [`nothing_of_a_sessions_own_is_in_the_shared_nuget_directories`]: the
/// plugins cache is a credential provider's account of itself rather than a
/// download, `NUGET_PLUGINS_CACHE_PATH` is deliberately unset, and what this
/// says is that it really did go to the home the sandbox throws away.
#[tokio::test]
async fn the_directories_nuget_names_for_itself_are_the_ones_the_descriptor_set() {
    let Some(found_them) = tools(".NET", &["dotnet"]) else {
        return;
    };
    let dotnet = &found_them[0];

    let machine = machine(1).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let asked = installing(
        &machine.sandbox(0, &cache, reaching_nothing()),
        &format!(
            "set -e\n{DOTNET_SETTINGS}\n\
             printf 'home=%s\\n' \"$HOME\"\n\
             {dotnet} nuget locals all --list\n",
            dotnet = dotnet.display(),
        ),
    );

    asked.worked("a session asks its own `dotnet` where it keeps things");

    for (said, under) in [
        ("global-packages", "nuget/packages"),
        ("http-cache", "nuget/http"),
        ("temp", "nuget/scratch"),
    ] {
        assert_eq!(
            listed(&asked, said),
            dir.join(under).display().to_string(),
            "NuGet's own `{said}` is {under} inside the one Build Cache, which \
             is what says it read the variable rather than that the variable \
             was set. It said:\n{}",
            asked.said,
        );
    }

    let home = line(&asked, "home");

    assert!(
        listed(&asked, "plugins-cache").starts_with(&home),
        "and the one directory this descriptor deliberately leaves alone is \
         under the session's own home, where a credential provider's account of \
         itself belongs. It said:\n{}",
        asked.said,
    );
}

/// And what `dotnet nuget locals` printed under `<name>: `, which is NuGet's
/// own shape rather than the `key=` [`line`] reads: this one line is the tool
/// speaking for itself, so it is read as the tool writes it.
fn listed(ran: &Ran, name: &str) -> String {
    ran.said
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{name}: ")))
        .unwrap_or_else(|| {
            panic!(
                "`dotnet nuget locals all --list` was supposed to print a \
                 `{name}: ` line, and the session said:\n{}",
                ran.said
            )
        })
        .trim()
        .to_owned()
}

// ---------------------------------------------------------------------------
// C and C++: CMake, through the Compile Server.
// ---------------------------------------------------------------------------

/// The port an sccache client asks for its server on, which is sccache's own
/// default: nothing Verkstead sets names another one, so it is the port the
/// Compile Server listens on and the one a session's client dials.
const SCCACHE_PORT: u16 = 4226;

/// Held by each proof starting a Compile Server for as long as it runs: there
/// is one port for it to listen on, so two at once would be one proof's client
/// reaching the other's server.
static ON_THE_PORT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A small CMake project that is nonetheless a real one: a C source and a C++
/// source, and a header under `include/` so that a `-I` is on every compile
/// line. The minimum is the first release that reads the two launchers out of
/// the environment, which is the whole of what the descriptor sets.
fn cmake_project(at: &Path) {
    std::fs::create_dir_all(at.join("include")).unwrap();

    std::fs::write(
        at.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.17)\n\
         project(greeting C CXX)\n\
         add_executable(greeting main.cpp greet.c)\n\
         target_include_directories(greeting PRIVATE include)\n",
    )
    .unwrap();
    std::fs::write(at.join("include/greet.h"), "int greet(void);\n").unwrap();
    std::fs::write(
        at.join("greet.c"),
        "#include \"greet.h\"\nint greet(void) { return 42; }\n",
    )
    .unwrap();
    std::fs::write(
        at.join("main.cpp"),
        "extern \"C\" {\n#include \"greet.h\"\n}\n\
         int main() { return greet() == 42 ? 0 : 1; }\n",
    )
    .unwrap();
}

/// What the Compile Server said about itself, read off the file a session's
/// client wrote it into.
fn stats(worktree: &Path, name: &str) -> serde_json::Value {
    let written = std::fs::read_to_string(worktree.join(name))
        .unwrap_or_else(|error| panic!("the session was supposed to write {name}: {error}"));

    serde_json::from_str(&written)
        .unwrap_or_else(|error| panic!("{name} is sccache's JSON ({error}): {written}"))
}

/// How many compiles of C or C++ one of those counted under `under` — the
/// hits or the misses.
fn c_family(stats: &serde_json::Value, under: &str) -> u64 {
    stats["stats"][under]["counts"]["C/C++"]
        .as_u64()
        .unwrap_or(0)
}

/// Wait for the Compile Server to answer on its port, which is what a session
/// starting any later than this would find.
///
/// Waited for rather than assumed, because a client that finds nothing
/// listening starts a server of its own inside its Sandbox — which is the one
/// thing this proof exists to see not happen, and a race here would be it
/// happening for a reason that is the fixture's rather than the server's.
fn answering() {
    let began = std::time::Instant::now();

    while std::net::TcpStream::connect((Ipv4Addr::LOCALHOST, SCCACHE_PORT)).is_err() {
        assert!(
            began.elapsed() < std::time::Duration::from_secs(30),
            "the Compile Server never answered on {SCCACHE_PORT}: see the server's log above \
             for why it would not start",
        );

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// **A real CMake build, compiled through Verkstead's Compile Server** — the
/// C/C++ descriptor's proof, the way every other descriptor's is an install.
///
/// What the descriptor sets is `CMAKE_C_COMPILER_LAUNCHER` and
/// `CMAKE_CXX_COMPILER_LAUNCHER` and nothing else, and what it cannot say is
/// whether a compile put through them really reaches the server outside the
/// Sandbox, and whether that server can really run the compiler it is handed.
/// So a Worktree is configured with the Ninja generator and built, and then
/// its build directory is taken away and it is configured and built again.
///
/// **Three things are read off the server rather than inferred**, all through
/// the same client the build used:
///
/// 1. The first build's compiles were requests to it, and every one ran.
/// 2. The clean rebuild was served entirely out of its cache: not one miss, and
///    as many C/C++ hits as there were requests.
/// 3. And the server that answered is the one Verkstead started: it names the
///    cache this proof handed it.
///
/// **And then the same project in a second Conversation's Worktree**, at a
/// path of its own — which sccache hashes into a C or C++ compile, so without
/// `SCCACHE_BASEDIRS` it is every compile a miss. That Worktree is made while
/// the first session is still running, so the spawn that starts the second one
/// finds nothing running and starts the Compile Server again, told both:
///
/// 4. The second Conversation's build missed nothing, and every request was a
///    hit, out of what the first one compiled.
/// 5. And the server it compiled through is one told the second Worktree, which
///    is the restart having happened rather than the paths happening to agree.
///
/// When that restart is *held back* — a spawn while another session is running
/// — is a decision rather than a timing, and is proven as one in
/// `build_cache`'s own tests.
///
/// **And no sccache server was started inside the Sandbox.** An sccache client
/// that finds no server starts one of its own, which would build perfectly
/// well and prove nothing — so the Sandbox's own process namespace is searched
/// for one after the build. The Compile Server is in no session's namespace, so
/// anything of that name in there is a server the session started.
///
/// **The compiler is found the way a session finds one** — the machine's, off
/// the `PATH` this suite runs on, which under the dev shell is a nix
/// `cc-wrapper` in `/nix/store` and on a runner is `/usr/bin`. Both are inside
/// what the Compile Server's own Sandbox stands on (see `compile_server` in
/// `crates/server/src/build_cache.rs`), which is why nothing there needed
/// widening for C.
///
/// **Skipped where the port is taken**, which is a machine already running an
/// sccache server of its own: a Verkstead session building this checkout is
/// one. The Compile Server this starts could not listen, the client would
/// reach the other server instead, and the answer would be about that one.
/// Run the suite in a network namespace of its own — `bwrap --dev-bind / /
/// --unshare-net` — to prove it there.
#[tokio::test]
async fn cmake_compiles_through_the_compile_server_and_a_rebuild_or_a_second_conversation_is_all_hits()
 {
    let _turn = ON_THE_PORT.lock().await;

    let Some(found) = tools("CMake", &["sccache", "cmake", "ninja", "cc", "c++"]) else {
        return;
    };

    let [sccache, cmake, ninja, cc, cxx] = &found[..] else {
        unreachable!("five tools were asked for");
    };

    if std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, SCCACHE_PORT)).is_err() {
        let missing = format!(
            "port {SCCACHE_PORT} is already taken on this machine, which is another sccache \
             server that a session's client would reach instead of the Compile Server"
        );

        println!("skipping the CMake Compile Server proof: {missing}");

        assert!(
            std::env::var_os(REQUIRED).is_none(),
            "the CMake Compile Server proof was skipped — {missing} — and {REQUIRED} is set",
        );

        return;
    }

    let machine = machine(2).await;
    let cache = machine.compiling_cache(sccache);

    // The second Conversation's Worktree is not made until the first session
    // is running — see below — so it is set aside until then. Nothing asks git
    // about it in between, so a rename out and back is the Worktree being made
    // later as far as the Compile Server can tell.
    let second = machine.worktree(1);
    let aside = machine.state.path().join("not-yet-made");
    std::fs::rename(second, &aside).unwrap();

    // What a session's spawn does before the session, on a machine where C/C++
    // is on — which every built-in language is, left alone. Held for as long
    // as that session runs, as a spawn's own is.
    let running = cache.compiling(&machine.settings.config(), None);
    answering();

    let worktree = machine.worktree(0);
    cmake_project(&worktree.join("greeting"));

    // The script a session runs, building the project in its own Worktree,
    // and what it does after: `after` is run once the first build is in.
    let session = |after: &str| {
        format!(
            r#"set -e
            printf 'c-launcher=%s\n' "${{CMAKE_C_COMPILER_LAUNCHER-unset}}"
            printf 'cxx-launcher=%s\n' "${{CMAKE_CXX_COMPILER_LAUNCHER-unset}}"

            build() {{
                '{cmake}' -S greeting -B greeting/build -G Ninja \
                    -DCMAKE_MAKE_PROGRAM='{ninja}' \
                    -DCMAKE_C_COMPILER='{cc}' -DCMAKE_CXX_COMPILER='{cxx}'
                '{cmake}' --build greeting/build
                ./greeting/build/greeting
            }}

            build
            "$CMAKE_C_COMPILER_LAUNCHER" --show-stats --stats-format=json > first.json
            {after}

            # Every sccache in this Sandbox's own process namespace, which the
            # Compile Server is not in.
            started=0
            for comm in /proc/[0-9]*/comm; do
                if [ "$(cat "$comm" 2>/dev/null)" = sccache ]; then
                    started=$((started + 1))
                fi
            done
            printf 'started-inside=%s\n' "$started"
            "#,
            cmake = cmake.display(),
            ninja = ninja.display(),
            cc = cc.display(),
            cxx = cxx.display(),
        )
    };

    let sandbox = machine.sandbox(0, &cache, vec![]);

    let built = installing(
        &sandbox,
        &session(
            r#""$CMAKE_C_COMPILER_LAUNCHER" --zero-stats
            rm -rf greeting/build
            build
            "$CMAKE_C_COMPILER_LAUNCHER" --show-stats --stats-format=json > again.json"#,
        ),
    );

    built.worked("a CMake project configures, builds and runs inside a session's Sandbox");

    assert_eq!(
        line(&built, "c-launcher"),
        "/verkstead/bin/sccache",
        "C is launched through the sccache the server resolved. It said:\n{}",
        built.said,
    );
    assert_eq!(line(&built, "cxx-launcher"), "/verkstead/bin/sccache");

    assert_eq!(
        line(&built, "started-inside"),
        "0",
        "no sccache server was started inside the session's Sandbox: every compile \
         was the Compile Server's. It said:\n{}",
        built.said,
    );

    let first = stats(worktree, "first.json");

    let requests = first["stats"]["compile_requests"].as_u64().unwrap_or(0);
    let executed = first["stats"]["requests_executed"].as_u64().unwrap_or(0);

    assert!(
        requests >= 2,
        "the C source and the C++ source were both requests to the Compile Server, \
         and so is every compile CMake's configure makes of its own: {first}",
    );
    assert_eq!(
        executed, requests,
        "and it ran every one of them — which is the server reaching the compiler \
         from its own Sandbox: {first}",
    );
    assert_eq!(
        c_family(&first, "cache_misses"),
        requests,
        "into a cache nothing had compiled into, so every one was a miss: {first}",
    );
    assert_eq!(
        first["cache_location"],
        format!(
            "Local disk: {:?}",
            cache.dir().unwrap().join("sccache").display().to_string()
        ),
        "and the server that answered is the one Verkstead started, writing into \
         the cache this proof handed it",
    );
    assert_eq!(
        first["max_cache_size"].as_u64(),
        Some(30 * 1024 * 1024 * 1024),
        "at the size the Compile Server is started with",
    );

    let again = stats(worktree, "again.json");

    assert!(
        again["stats"]["compile_requests"].as_u64().unwrap_or(0) >= 2,
        "the rebuild compiled through the Compile Server too: {again}",
    );
    assert_eq!(
        c_family(&again, "cache_misses"),
        0,
        "the clean rebuild in the same Worktree missed nothing: {again}",
    );
    assert_eq!(
        c_family(&again, "cache_hits"),
        again["stats"]["compile_requests"].as_u64().unwrap_or(0),
        "every one of its compiles was served from the cache: {again}",
    );

    // The second Conversation's Worktree is made while the first session is
    // still running, and then that session ends.
    std::fs::rename(&aside, second).unwrap();
    drop(running);

    // So the spawn of the second Conversation's session finds nothing running
    // and starts the Compile Server again, told both Worktrees.
    let _second_running = cache.compiling(&machine.settings.config(), None);
    answering();

    cmake_project(&second.join("greeting"));

    let elsewhere = installing(&machine.sandbox(1, &cache, vec![]), &session(""));

    elsewhere.worked("the same project builds in a second Conversation's Worktree");

    assert_eq!(
        line(&elsewhere, "started-inside"),
        "0",
        "and no sccache server was started inside that Sandbox either. It said:\n{}",
        elsewhere.said,
    );

    let across = stats(second, "first.json");

    let told: Vec<&str> = across["basedirs"]
        .as_array()
        .map(|dirs| dirs.iter().filter_map(|dir| dir.as_str()).collect())
        .unwrap_or_default();

    for worktree in [worktree, second] {
        assert!(
            told.iter().any(|dir| Path::new(dir) == worktree),
            "the Compile Server it compiled through was started again and told {} as \
             a base directory: {across}",
            worktree.display(),
        );
    }

    let requests = across["stats"]["compile_requests"].as_u64().unwrap_or(0);

    assert!(
        requests >= 2,
        "the second Conversation's build compiled through the Compile Server: {across}",
    );
    assert_eq!(
        c_family(&across, "cache_misses"),
        0,
        "and at a path of its own it missed nothing: {across}",
    );
    assert_eq!(
        c_family(&across, "cache_hits"),
        requests,
        "every one of its compiles was served out of what the first Conversation \
         compiled: {across}",
    );
}

/// **Rust compiles again through a cleared sccache.** A crate compiled through
/// the Compile Server fills its store; a Clear of Rust stops the server and
/// empties the store; and the next launch starts the server again, through
/// which the same crate compiles — a miss, the cache being empty — and the
/// store holds what it compiled once more.
///
/// A library compiled with `rustc` alone rather than a `cargo build`, so the
/// Sandbox needs no linker: what is asked is the Compile Server's, and a `.rlib`
/// is as much a compile through it as anything.
///
/// **Skipped where the port is taken**, for the CMake proof's reason above.
#[tokio::test]
async fn rust_compiles_again_through_a_cleared_sccache() {
    let _turn = ON_THE_PORT.lock().await;

    let Some(found) = tools("Rust Compile Server", &["sccache", "rustc"]) else {
        return;
    };

    let [sccache, rustc] = &found[..] else {
        unreachable!("two tools were asked for");
    };

    if std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, SCCACHE_PORT)).is_err() {
        let missing = format!(
            "port {SCCACHE_PORT} is already taken on this machine, which is another sccache \
             server that a session's client would reach instead of the Compile Server"
        );

        println!("skipping the cleared-sccache proof: {missing}");

        assert!(
            std::env::var_os(REQUIRED).is_none(),
            "the cleared-sccache proof was skipped — {missing} — and {REQUIRED} is set",
        );

        return;
    }

    let machine = machine(1).await;
    let cache = machine.compiling_cache(sccache);
    let store = cache.dir().unwrap().join("sccache");

    let worktree = machine.worktree(0);
    std::fs::write(
        worktree.join("probe.rs"),
        "pub fn probe() -> u32 { 41 + 1 }\n",
    )
    .unwrap();

    let compiling = |stats: &str| {
        format!(
            r#"set -e
            mkdir -p out
            "$RUSTC_WRAPPER" '{rustc}' --crate-name probe --crate-type lib --emit=link \
                --out-dir out probe.rs
            "$RUSTC_WRAPPER" --show-stats --stats-format=json > {stats}
            "#,
            rustc = rustc.display(),
        )
    };

    let running = cache.compiling(&machine.settings.config(), None);
    answering();

    installing(
        &machine.sandbox(0, &cache, vec![]),
        &compiling("first.json"),
    )
    .worked("a crate compiles through the Compile Server");

    let first = stats(worktree, "first.json");

    assert!(
        first["stats"]["cache_misses"]["counts"]["Rust"]
            .as_u64()
            .unwrap_or(0)
            >= 1,
        "into an empty cache, so a miss: {first}",
    );
    assert!(
        !tree(&store).is_empty(),
        "and its store holds what it compiled"
    );

    // The session over, so nothing runs and the Clear goes ahead.
    drop(running);

    cleared(&cache, verkstead_server::languages::RUST);

    // Gone a moment after its sandbox is: the sccache inside dies with the
    // bwrap it was started under, which the Clear waited for.
    let began = std::time::Instant::now();

    while std::net::TcpStream::connect((Ipv4Addr::LOCALHOST, SCCACHE_PORT)).is_ok() {
        assert!(
            began.elapsed() < std::time::Duration::from_secs(10),
            "a Clear of Rust stops the Compile Server, so its index does not go on naming \
             files that are gone",
        );

        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let _running = cache.compiling(&machine.settings.config(), None);
    answering();

    std::fs::remove_dir_all(worktree.join("out")).unwrap();

    installing(
        &machine.sandbox(0, &cache, vec![]),
        &compiling("again.json"),
    )
    .worked("the same crate compiles through the Compile Server started again");

    let again = stats(worktree, "again.json");

    assert!(
        again["stats"]["cache_misses"]["counts"]["Rust"]
            .as_u64()
            .unwrap_or(0)
            >= 1
            && again["stats"]["cache_hits"]["counts"]["Rust"]
                .as_u64()
                .unwrap_or(0)
                == 0,
        "out of a cache the Clear emptied, so a miss and not a hit: {again}",
    );
    assert!(
        !tree(&store).is_empty(),
        "and the cleared store holds what it compiled again",
    );
}

// ---------------------------------------------------------------------------
// The JVM: Maven and Gradle, and the JDK they run on.
// ---------------------------------------------------------------------------

/// The three JVM tools each answer inside a Sandbox, before anything asks them
/// to build.
///
/// This is the proof the later JVM ones stand on, and it is asked on its own
/// because a JDK is the likeliest tool in this file not to be reachable at all:
/// the runner image keeps its JDKs under `/usr/lib/jvm` behind
/// `/etc/alternatives`, nixpkgs' Maven and Gradle are wrappers that name their
/// JDK by a `/nix/store` path, and a Sandbox is handed no `JAVA_HOME` of the
/// host's. Any of those landing outside a bind is a build that dies on a
/// missing `java` rather than on anything to do with a store, and this is the
/// line that says so first.
///
/// Gradle is told where its home is, and it is a directory of this Sandbox's
/// Worktree: `--version` writes into it, and which home a session is really
/// given is the `jvm` descriptor's to say rather than this proof's.
#[tokio::test]
async fn java_maven_and_gradle_each_answer_inside_a_sandbox() {
    let Some(found) = tools("JVM", &["java", "mvn", "gradle"]) else {
        return;
    };

    let [java, mvn, gradle] = &found[..] else {
        unreachable!("three tools were asked for");
    };

    let machine = machine(1).await;
    let cache = machine.cache();
    let sandbox = machine.sandbox(0, &cache, vec![]);

    let ran = installing(
        &sandbox,
        &format!(
            "set -e\n\
             '{java}' -version\n\
             '{mvn}' --batch-mode -v\n\
             '{gradle}' --gradle-user-home \"$PWD/gradle-home\" --version\n",
            java = java.display(),
            mvn = mvn.display(),
            gradle = gradle.display(),
        ),
    );

    ran.worked("`java -version`, `mvn -v` and `gradle --version` each succeed in a session");

    for (tool, says) in [
        ("java", "version \""),
        ("mvn", "Apache Maven "),
        ("gradle", "Gradle "),
    ] {
        assert!(
            ran.said.contains(says),
            "`{tool}` printed its version, which starts `{says}`. It said:\n{}",
            ran.said,
        );
    }
}

/// The two artifacts every Maven proof resolves, both under this group and at
/// this version: a plugin, and a jar the plugin calls into.
const MAVEN_GROUP: &str = "example.test";
const MAVEN_VERSION: &str = "1.0.0";

/// Where the plugin's own dependency is published, and the class in it.
const MAVEN_GREET: &str = "greet";

/// And the plugin, which the consumer below binds to `validate`.
const MAVEN_PLUGIN: &str = "say-maven-plugin";

/// What every `mvn` in this suite is told, beside the variable the descriptor
/// sets, which is the one this is here to prove. `--batch-mode` so that nothing
/// asks a terminal anything, and `--no-transfer-progress` so that what a
/// failure has to be read out of is not a download counter.
const MVN_FLAGS: &str = "--batch-mode --no-transfer-progress";

/// Where the Maven installation `mvn` runs is: the `Maven home:` line of its
/// own `mvn -v`.
///
/// Asked rather than worked out from where `mvn` is, because nixpkgs' `mvn`
/// is a wrapper script in a `bin` beside no `lib`, and the runner image's is a
/// link into `/usr/local/maven`. The plugin below is compiled against the
/// `maven-plugin-api` jar in that installation's `lib`.
fn maven_home(mvn: &Path) -> PathBuf {
    let said = Command::new(mvn)
        .args(["--batch-mode", "-v"])
        .stdin(Stdio::null())
        .output()
        .expect("the `mvn` this machine was found to have");

    assert!(
        said.status.success(),
        "`mvn -v` should answer: {}",
        String::from_utf8_lossy(&said.stderr),
    );

    let said = String::from_utf8_lossy(&said.stdout).into_owned();

    said.lines()
        .find_map(|line| line.strip_prefix("Maven home: "))
        .map(|home| PathBuf::from(home.trim()))
        .unwrap_or_else(|| panic!("`mvn -v` names no Maven home. It said:\n{said}"))
}

/// Lay a Maven repository out under `at`, holding the plugin and the jar it
/// calls into, and serve it over the loopback.
///
/// **A plugin, because nothing else a Maven build resolves can be served from
/// here.** Even `validate` on a `jar` project reaches for the default
/// lifecycle's plugins, which live on Maven Central. So the consumer is a
/// `pom` project, whose `validate` binds nothing, with one plugin of this
/// suite's own bound to it. Maven resolves that plugin, the jar it depends
/// on, and the project's own dependency on the same jar through the local
/// repository, which is the store this is about.
///
/// **And what it prints is code out of the store.** The mojo calls
/// `Greet.hello()` from the other jar, so a build that says
/// [`OUT_OF_THE_STORE`] ran both jars as they came out of the repository.
///
/// Compiled with the host's `javac` against the `maven-plugin-api` jar in the
/// installation `mvn` runs, and zipped with the suite's own `zip`: a jar is a
/// zip. The plugin descriptor is written by hand, as `maven-plugin-plugin`
/// would have written it, with the parts Maven 3.9 reads to run a mojo.
///
/// **Served rather than laid out on disk.** Maven reads a `file://` repository
/// perfectly well, but a download over the network is the case two sessions
/// racing each other is about. Checksums are not served: Maven's default is to
/// warn about a missing one and go on, and nothing here is about checksums.
fn maven_registry(at: &Path, mvn: &Path, javac: &Path, zip: &Path) -> Registry {
    let lib = maven_home(mvn).join("lib");
    let api = std::fs::read_dir(&lib)
        .expect("the lib directory of the Maven installation")
        .flatten()
        .map(|entry| entry.path())
        .find(|jar| {
            jar.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("maven-plugin-api-"))
        })
        .unwrap_or_else(|| panic!("no maven-plugin-api jar under {}", lib.display()));

    let sources = at.join("sources");
    let greet = at.join("classes/greet");
    let plugin = at.join("classes/plugin");

    for dir in [&sources, &greet, &plugin.join("META-INF/maven")] {
        std::fs::create_dir_all(dir).unwrap();
    }

    std::fs::write(
        sources.join("Greet.java"),
        format!(
            "package example.greet;\n\
             public final class Greet {{\n\
             \x20 public static String hello() {{ return \"{OUT_OF_THE_STORE}\"; }}\n\
             }}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        sources.join("Say.java"),
        "package example.say;\n\
         public final class Say extends org.apache.maven.plugin.AbstractMojo {\n\
         \x20 public void execute() { getLog().info(\"said: \" + example.greet.Greet.hello()); }\n\
         }\n",
    )
    .unwrap();
    std::fs::write(
        plugin.join("META-INF/maven/plugin.xml"),
        format!(
            "<plugin>\n\
             \x20 <name>say</name>\n\
             \x20 <groupId>{MAVEN_GROUP}</groupId>\n\
             \x20 <artifactId>{MAVEN_PLUGIN}</artifactId>\n\
             \x20 <version>{MAVEN_VERSION}</version>\n\
             \x20 <goalPrefix>say</goalPrefix>\n\
             \x20 <mojos>\n\
             \x20   <mojo>\n\
             \x20     <goal>hello</goal>\n\
             \x20     <implementation>example.say.Say</implementation>\n\
             \x20     <language>java</language>\n\
             \x20     <requiresDependencyResolution>runtime</requiresDependencyResolution>\n\
             \x20     <requiresProject>true</requiresProject>\n\
             \x20     <instantiationStrategy>per-lookup</instantiationStrategy>\n\
             \x20     <executionStrategy>once-per-session</executionStrategy>\n\
             \x20     <threadSafe>true</threadSafe>\n\
             \x20     <parameters/>\n\
             \x20   </mojo>\n\
             \x20 </mojos>\n\
             \x20 <dependencies>\n\
             \x20   <dependency>\n\
             \x20     <groupId>{MAVEN_GROUP}</groupId>\n\
             \x20     <artifactId>{MAVEN_GREET}</artifactId>\n\
             \x20     <type>jar</type>\n\
             \x20     <version>{MAVEN_VERSION}</version>\n\
             \x20   </dependency>\n\
             \x20 </dependencies>\n\
             </plugin>\n"
        ),
    )
    .unwrap();

    let compiled = |into: &Path, classpath: &str, source: &str| {
        let made = Command::new(javac)
            .args(["--release", "11", "-nowarn", "-d"])
            .arg(into)
            .args(["-cp", classpath])
            .arg(sources.join(source))
            .stdin(Stdio::null())
            .output()
            .expect("the `javac` this machine was found to have");

        assert!(
            made.status.success(),
            "{source} did not compile: {}",
            String::from_utf8_lossy(&made.stderr),
        );
    };

    compiled(&greet, "", "Greet.java");
    compiled(
        &plugin,
        &format!("{}:{}", greet.display(), api.display()),
        "Say.java",
    );

    let jarred = |classes: &Path| {
        let jar = classes.with_extension("jar");
        let made = Command::new(zip)
            .args(["-q", "-r", "-X"])
            .arg(&jar)
            .arg(".")
            .current_dir(classes)
            .stdin(Stdio::null())
            .status()
            .expect("the suite's own `zip`");

        assert!(made.success(), "{} was not jarred", classes.display());

        std::fs::read(&jar).unwrap()
    };

    let pom = |artifact: &str, packaging: &str, dependencies: &str| {
        format!(
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n\
             \x20 <modelVersion>4.0.0</modelVersion>\n\
             \x20 <groupId>{MAVEN_GROUP}</groupId>\n\
             \x20 <artifactId>{artifact}</artifactId>\n\
             \x20 <version>{MAVEN_VERSION}</version>\n\
             \x20 <packaging>{packaging}</packaging>\n\
             {dependencies}\
             </project>\n"
        )
        .into_bytes()
    };

    // Every file a build of the consumer fetches, by the path Maven asks for
    // it at: `<group as directories>/<artifact>/<version>/<artifact>-<version>.<ext>`.
    let under = |artifact: &str, ext: &str| {
        format!(
            "/{group}/{artifact}/{MAVEN_VERSION}/{artifact}-{MAVEN_VERSION}.{ext}",
            group = MAVEN_GROUP.replace('.', "/"),
        )
    };

    let served: std::collections::HashMap<String, Vec<u8>> = [
        (under(MAVEN_GREET, "jar"), jarred(&greet)),
        (under(MAVEN_GREET, "pom"), pom(MAVEN_GREET, "jar", "")),
        (under(MAVEN_PLUGIN, "jar"), jarred(&plugin)),
        (
            under(MAVEN_PLUGIN, "pom"),
            pom(
                MAVEN_PLUGIN,
                "maven-plugin",
                &format!(
                    "\x20 <dependencies>\n\
                     \x20   <dependency>\n\
                     \x20     <groupId>{MAVEN_GROUP}</groupId>\n\
                     \x20     <artifactId>{MAVEN_GREET}</artifactId>\n\
                     \x20     <version>{MAVEN_VERSION}</version>\n\
                     \x20   </dependency>\n\
                     \x20 </dependencies>\n"
                ),
            ),
        ),
    ]
    .into_iter()
    .collect();

    // Bound here rather than on the thread below, for the reason the npm
    // registry's is: the consumer names the port, so it has to be known before
    // anything is served.
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .expect("a loopback port for the registry");
    let url = format!("http://{}", listener.local_addr().expect("the port it got"));

    listener
        .set_nonblocking(true)
        .expect("what tokio takes a standard listener over");

    // Anything else Maven asks for — a checksum, a `maven-metadata.xml` — is
    // answered 404, which is what a repository without it says.
    let app = Router::new()
        .fallback(get(
            |State(served): State<std::sync::Arc<std::collections::HashMap<String, Vec<u8>>>>,
             uri: axum::http::Uri| async move {
                match served.get(uri.path()) {
                    Some(file) => Ok((
                        [(header::CONTENT_TYPE, "application/octet-stream")],
                        file.clone(),
                    )),
                    None => Err(axum::http::StatusCode::NOT_FOUND),
                }
            },
        ))
        .with_state(std::sync::Arc::new(served));

    let (stop, stopping) = tokio::sync::oneshot::channel();

    let serving = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for the registry's own thread");

        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener)
                .expect("the listener this thread was handed");

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

/// And what a Conversation's Worktree holds: a `pom` project depending on the
/// jar in the registry, with the plugin bound to `validate`, so that
/// `mvn validate` resolves all of it and runs the plugin.
///
/// **The registry replaces Central by taking its id.** A repository named
/// `central` in a project overrides the one Maven's super POM declares, so
/// nothing a build here resolves is looked for on the internet. `from` is
/// `None` where a Repo's own settings file is what names it instead.
///
/// Plain HTTP on the loopback is allowed: Maven's blocker of `http://`
/// repositories matches external hosts only.
fn maven_consumer(worktree: &Path, from: Option<&str>) {
    let repositories = from.map_or_else(String::new, |url| {
        format!(
            "\x20 <repositories>\n\
             \x20   <repository><id>central</id><url>{url}</url></repository>\n\
             \x20 </repositories>\n\
             \x20 <pluginRepositories>\n\
             \x20   <pluginRepository><id>central</id><url>{url}</url></pluginRepository>\n\
             \x20 </pluginRepositories>\n"
        )
    });

    std::fs::write(
        worktree.join("pom.xml"),
        format!(
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n\
             \x20 <modelVersion>4.0.0</modelVersion>\n\
             \x20 <groupId>{MAVEN_GROUP}</groupId>\n\
             \x20 <artifactId>app</artifactId>\n\
             \x20 <version>{MAVEN_VERSION}</version>\n\
             \x20 <packaging>pom</packaging>\n\
             {repositories}\
             \x20 <dependencies>\n\
             \x20   <dependency>\n\
             \x20     <groupId>{MAVEN_GROUP}</groupId>\n\
             \x20     <artifactId>{MAVEN_GREET}</artifactId>\n\
             \x20     <version>{MAVEN_VERSION}</version>\n\
             \x20   </dependency>\n\
             \x20 </dependencies>\n\
             \x20 <build>\n\
             \x20   <plugins>\n\
             \x20     <plugin>\n\
             \x20       <groupId>{MAVEN_GROUP}</groupId>\n\
             \x20       <artifactId>{MAVEN_PLUGIN}</artifactId>\n\
             \x20       <version>{MAVEN_VERSION}</version>\n\
             \x20       <executions>\n\
             \x20         <execution>\n\
             \x20           <phase>validate</phase>\n\
             \x20           <goals><goal>hello</goal></goals>\n\
             \x20         </execution>\n\
             \x20       </executions>\n\
             \x20     </plugin>\n\
             \x20   </plugins>\n\
             \x20 </build>\n\
             </project>\n"
        ),
    )
    .unwrap();
}

/// Where the artifacts a build resolved land in a Maven local repository.
fn resolved_into(repository: &Path) -> PathBuf {
    repository
        .join(MAVEN_GROUP.replace('.', "/"))
        .join(MAVEN_PLUGIN)
        .join(MAVEN_VERSION)
        .join(format!("{MAVEN_PLUGIN}-{MAVEN_VERSION}.jar"))
}

/// Maven: two Sandboxes building at once against one local repository, a third
/// building with `-o` and the registry off the air, and the control that says
/// the third proved something.
///
/// **The two at once are the half the locks are for.** Maven 3.9 guards its
/// local repository with locks inside one JVM unless told otherwise, so two
/// sessions would each take a lock the other cannot see. The descriptor's
/// `file-lock` puts them in the repository instead, where both sessions see
/// them. Both builds succeeding is what says the store came through; the third
/// resolving everything out of it is what says it came through whole.
///
/// The control fails with Maven's own words for an artifact it has nowhere to
/// get: offline, and "has not been downloaded from it before".
#[tokio::test]
async fn two_maven_builds_at_once_fill_one_repository_and_a_third_builds_out_of_it() {
    let Some(found) = tools("Maven", &["mvn", "javac", "zip"]) else {
        return;
    };
    let [mvn, javac, zip] = &found[..] else {
        unreachable!("three tools were asked for");
    };

    // Five: the two that race, the one denied its registry, the control, and
    // the one that fills the store for the sweep before any of them.
    let machine = machine(5).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let registry = maven_registry(&machine.registries.join("maven"), mvn, javac, zip);

    for nth in 0..5 {
        maven_consumer(machine.worktree(nth), Some(&registry.url));
    }

    let building = format!(
        "set -e\n'{mvn}' {MVN_FLAGS} validate\n",
        mvn = mvn.display()
    );
    let offline = format!(
        "set -e\n'{mvn}' {MVN_FLAGS} --offline validate\n",
        mvn = mvn.display()
    );

    // A repository the sweep has just taken units out of, which is what the
    // two below start from.
    filled_and_swept(
        &machine,
        4,
        &cache,
        reaching_nothing(),
        &building,
        verkstead_server::languages::JVM,
    );

    // Started together and waited on together, which is the only way the two
    // are ever really writing the repository at the same moment.
    let first = starting(&machine.sandbox(0, &cache, reaching_nothing()), &building);
    let second = starting(&machine.sandbox(1, &cache, reaching_nothing()), &building);

    let first = finished(first);

    first.worked("the first session's build fills the repository");
    finished(second).worked("and the second one racing it finishes just as well");

    assert!(
        resolved_into(&dir.join("maven/repository")).is_file(),
        "and what they downloaded is under the directory the descriptor named, \
         which is the one the third session is about to be given. It said:\n{}",
        first.said,
    );

    // And the proof. The registry stops answering and Maven is told not to
    // look: what is left to build out of is the repository the two filled.
    registry.shut();

    let third = installing(&machine.sandbox(2, &cache, reaching_nothing()), &offline);

    third.worked(
        "a third session builds offline with the registry gone, which it can only do out \
         of the shared local repository",
    );
    assert!(
        third.said.contains(&format!("said: {OUT_OF_THE_STORE}")),
        "and the plugin it ran really came out of the store, calling into the \
         jar beside it. It said:\n{}",
        third.said,
    );

    // And what the builds left is units the sweep can take out whole: one
    // version of an artifact at a time, the plugin's apart from its library's.
    whole_packages(
        &cache,
        verkstead_server::languages::JVM,
        "maven",
        Units::Naming(&[MAVEN_GREET, MAVEN_PLUGIN]),
    );

    // The control. Everything the same but the Build Cache, which nothing has
    // filled.
    let control = installing(
        &machine.sandbox(3, &machine.empty_cache(), reaching_nothing()),
        &offline,
    );

    assert!(
        !control.worked,
        "an empty repository and no registry has to fail, or the build above \
         proved nothing about either. It said:\n{}",
        control.said,
    );
    assert!(
        control
            .said
            .contains("has not been downloaded from it before"),
        "and it fails for want of anywhere to get the plugin, in Maven's own \
         words, rather than for some other reason. It said:\n{}",
        control.said,
    );
}

/// And Maven's own account of what it was told: the local repository it used,
/// and the locks it took on it, out of its debug log.
///
/// **This is what guards the locks.** Two builds at once succeeding is a race
/// that could go either way, and a Maven that stopped reading the two lock
/// properties would pass it most runs. The debug log is not a race: Resolver
/// names the lock factory and the name mapper it built, and they are the ones
/// the descriptor set. `rwlock-local` and `gaecv` are what 3.9 says without
/// them.
///
/// And the lock files are where every session sees them: `.locks` inside the
/// shared repository, not in the session's own `/tmp`.
#[tokio::test]
async fn the_repository_and_the_locks_maven_names_for_itself_are_the_ones_the_descriptor_set() {
    let Some(found) = tools("Maven", &["mvn", "javac", "zip"]) else {
        return;
    };
    let [mvn, javac, zip] = &found[..] else {
        unreachable!("three tools were asked for");
    };

    let machine = machine(1).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let registry = maven_registry(&machine.registries.join("maven-locks"), mvn, javac, zip);

    maven_consumer(machine.worktree(0), Some(&registry.url));

    // Only the two lines this is about, so that a failure is readable: the
    // debug log of one build is thousands of lines.
    let asked = installing(
        &machine.sandbox(0, &cache, reaching_nothing()),
        &format!(
            "'{mvn}' {MVN_FLAGS} -X validate > debug.log 2>&1 || {{ cat debug.log; exit 1; }}\n\
             grep -E 'Using local repository at|Creating adapter using' debug.log\n",
            mvn = mvn.display(),
        ),
    );

    registry.shut();

    asked.worked("a session builds with Maven's debug log on");

    let repository = dir.join("maven/repository");

    assert!(
        asked.said.contains(&format!(
            "Using local repository at {}",
            repository.display()
        )),
        "Maven's local repository is the one inside the Build Cache, which is \
         what says it read the property rather than that it was set. It \
         said:\n{}",
        asked.said,
    );
    assert!(
        asked
            .said
            .contains("Creating adapter using nameMapper 'file-gav' and factory 'file-lock'"),
        "and the locks it takes on it are files, which two sessions both see. \
         It said:\n{}",
        asked.said,
    );
    assert!(
        repository.join(".locks").is_dir(),
        "and the lock files are inside the shared repository, rather than \
         anywhere a session keeps to itself",
    );
}

/// And a Repo's own configuration still wins: a local repository it pins in
/// `.mvn/maven.config` is the one it gets, and a settings file it names there
/// with `-s` is still read.
///
/// **Which way this goes is a fact about Maven.** `.mvn/maven.config` holds
/// command-line arguments, so a `-D` in it is a *user* property, and the
/// descriptor's is a *system* property out of `MAVEN_OPTS`. Maven reads the
/// local repository out of the user properties first.
///
/// Two Worktrees, and in both the registry is named by the Repo's settings
/// file rather than the project, through a profile: a build that fetches
/// anything at all read that file. The first also pins its repository inside
/// the Worktree, and keeps it. The second pins nothing, and resolves into the
/// shared repository, so naming a settings file does not take a Repo out of
/// the store.
#[tokio::test]
async fn a_repos_own_maven_config_is_still_read_and_its_own_repository_wins() {
    let Some(found) = tools("Maven", &["mvn", "javac", "zip"]) else {
        return;
    };
    let [mvn, javac, zip] = &found[..] else {
        unreachable!("three tools were asked for");
    };

    let machine = machine(2).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let registry = maven_registry(&machine.registries.join("maven-own"), mvn, javac, zip);

    for nth in 0..2 {
        let worktree = machine.worktree(nth);

        maven_consumer(worktree, None);

        std::fs::create_dir_all(worktree.join(".mvn")).unwrap();
        std::fs::write(
            worktree.join(".mvn/settings.xml"),
            format!(
                "<settings>\n\
                 \x20 <profiles>\n\
                 \x20   <profile>\n\
                 \x20     <id>the-repos-own</id>\n\
                 \x20     <repositories>\n\
                 \x20       <repository><id>central</id><url>{url}</url></repository>\n\
                 \x20     </repositories>\n\
                 \x20     <pluginRepositories>\n\
                 \x20       <pluginRepository><id>central</id><url>{url}</url></pluginRepository>\n\
                 \x20     </pluginRepositories>\n\
                 \x20   </profile>\n\
                 \x20 </profiles>\n\
                 \x20 <activeProfiles><activeProfile>the-repos-own</activeProfile></activeProfiles>\n\
                 </settings>\n",
                url = registry.url,
            ),
        )
        .unwrap();
    }

    let pinned = machine.worktree(0).join("its-own-repository");

    std::fs::write(
        machine.worktree(0).join(".mvn/maven.config"),
        format!(
            "-s\n.mvn/settings.xml\n-Dmaven.repo.local={}\n",
            pinned.display()
        ),
    )
    .unwrap();
    std::fs::write(
        machine.worktree(1).join(".mvn/maven.config"),
        "-s\n.mvn/settings.xml\n",
    )
    .unwrap();

    let building = format!(
        "set -e\n'{mvn}' {MVN_FLAGS} validate\n",
        mvn = mvn.display()
    );

    let own = installing(&machine.sandbox(0, &cache, reaching_nothing()), &building);

    own.worked(
        "a Repo naming its settings file in `.mvn/maven.config` builds, which it can only \
         do by reading that file: nothing else names the registry",
    );
    assert!(
        resolved_into(&pinned).is_file(),
        "and a Repo pinning its own local repository there keeps it. It said:\n{}",
        own.said,
    );
    assert!(
        !resolved_into(&dir.join("maven/repository")).exists(),
        "and nothing of its went into the shared one",
    );

    let shared = installing(&machine.sandbox(1, &cache, reaching_nothing()), &building);

    registry.shut();

    shared.worked("a Repo naming only its settings file builds as well");
    assert!(
        resolved_into(&dir.join("maven/repository")).is_file(),
        "and resolves into the shared repository, since a settings file of its \
         own does not take it out of the store. It said:\n{}",
        shared.said,
    );
}

/// And a Build Cache whose path has a space in it, which is what a Windows
/// user name with one in it gives by default: Maven still builds, with a
/// repository of the session's own.
///
/// **The descriptor's variable is a line of flags**, since Maven has no
/// variable for its local repository, and Maven 3's `mvn` splits `MAVEN_OPTS`
/// on whitespace without reading a quote. Handed a path with a space in it,
/// every `mvn` in the session would die: "Could not find or load main class"
/// and the half of the path after the space. So the loader leaves a line of
/// words out wherever a directory in it would split it, and this is that rule
/// against a real Maven: the session is not given `MAVEN_OPTS`, the build
/// works, and nothing of it went under the Build Cache.
#[tokio::test]
async fn a_build_cache_with_a_space_in_its_path_leaves_maven_working_and_unshared() {
    let Some(found) = tools("Maven", &["mvn", "javac", "zip"]) else {
        return;
    };
    let [mvn, javac, zip] = &found[..] else {
        unreachable!("three tools were asked for");
    };

    let machine = machine(1).await;
    let cache = machine.cache_named("a cache with spaces");
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let registry = maven_registry(&machine.registries.join("maven-spaced"), mvn, javac, zip);

    maven_consumer(machine.worktree(0), Some(&registry.url));

    let built = installing(
        &machine.sandbox(0, &cache, reaching_nothing()),
        &format!(
            "set -e\n\
             printf 'maven-opts=%s\\n' \"${{MAVEN_OPTS-unset}}\"\n\
             '{mvn}' {MVN_FLAGS} validate\n",
            mvn = mvn.display(),
        ),
    );

    registry.shut();

    built.worked("a session on a Build Cache with a space in its path still builds with Maven");

    assert_eq!(
        line(&built, "maven-opts"),
        "unset",
        "and it was not handed the line of flags that would have split at the \
         space. It said:\n{}",
        built.said,
    );
    assert!(
        built.said.contains(&format!("said: {OUT_OF_THE_STORE}")),
        "and the plugin ran. It said:\n{}",
        built.said,
    );
    assert!(
        !dir.join("maven").exists(),
        "and nothing of it went under the Build Cache, which it was not given",
    );
}

/// What every `gradle` in this suite is told, beside the two variables the
/// descriptor sets, which are the two this is here to prove. A plain console,
/// so that what a failure has to be read out of is lines rather than a
/// progress bar redrawn.
const GRADLE_FLAGS: &str = "--console=plain";

/// What a Gradle build in a proof prints for the Sandbox it ran in: a file in
/// that Sandbox's own `/tmp`, which no other Sandbox can read.
///
/// **This is the whole of how a build is caught in the wrong Sandbox.** Every
/// Sandbox is handed a `/tmp` of its own, so a build that prints another
/// session's marker is running in that other session's processes, which is
/// what a daemon shared through the Gradle home would be.
fn marking(marker: &str) -> String {
    format!("printf '%s\\n' '{marker}' > /tmp/marker\n")
}

/// A Gradle build in a Conversation's Worktree, with two tasks.
///
/// `say` resolves the jar [`maven_registry`] serves, from `from`, and calls
/// into it: a build that prints [`OUT_OF_THE_STORE`] ran code that came out of
/// the Gradle home. Then it prints the Sandbox's [`marking`]. **Served rather
/// than laid out**, because Gradle does not cache a `file://` repository at
/// all: a build reading one reads it in place, and the store would never be
/// filled.
///
/// `mark` prints the marker and resolves nothing, for the proofs about the
/// daemon rather than the store. `runFiles` is Kotlin's, and resolves nothing
/// either: see
/// [`a_kotlin_compile_daemons_run_files_stay_in_the_sandbox_that_started_it`].
///
/// Either task, given `-Pmeet=<dir> -Pme=<name> -Pother=<name>`, **meets the
/// other session's build before it finishes**: it leaves its own name in
/// `<dir>` and waits for the other's. Which is what makes two builds really
/// run at once rather than probably: neither ends until both have got as far
/// as their task. The meeting place is under the Build Cache, the one
/// directory both Sandboxes are bound.
fn gradle_consumer(worktree: &Path, from: Option<&str>) {
    let repositories = from.map_or_else(String::new, |url| {
        format!(
            "repositories {{\n\
             \x20 maven {{ url = uri('{url}'); allowInsecureProtocol = true }}\n\
             }}\n"
        )
    });

    std::fs::write(
        worktree.join("settings.gradle"),
        "rootProject.name = 'app'\n",
    )
    .unwrap();
    std::fs::write(
        worktree.join("build.gradle"),
        format!(
            "{repositories}\
             configurations {{ greet }}\n\
             dependencies {{ greet '{MAVEN_GROUP}:{MAVEN_GREET}:{MAVEN_VERSION}' }}\n\
             \n\
             def meeting = {{\n\
             \x20 if (!project.hasProperty('meet')) return\n\
             \x20 def meet = new File(project.property('meet'))\n\
             \x20 new File(meet, project.property('me')).text = 'here'\n\
             \x20 def other = new File(meet, project.property('other'))\n\
             \x20 def deadline = System.currentTimeMillis() + 120_000\n\
             \x20 while (!other.exists()) {{\n\
             \x20   if (System.currentTimeMillis() > deadline) throw new GradleException('the other build never came')\n\
             \x20   sleep 100\n\
             \x20 }}\n\
             \x20 println 'met: ' + project.property('other')\n\
             }}\n\
             \n\
             tasks.register('say') {{\n\
             \x20 def jars = configurations.greet\n\
             \x20 doLast {{\n\
             \x20   def loader = new URLClassLoader(jars.files.collect {{ it.toURI().toURL() }} as URL[])\n\
             \x20   println 'said: ' + loader.loadClass('example.greet.Greet').getMethod('hello').invoke(null)\n\
             \x20   println 'marker: ' + new File('/tmp/marker').text.trim()\n\
             \x20   meeting()\n\
             \x20 }}\n\
             }}\n\
             \n\
             tasks.register('mark') {{\n\
             \x20 doLast {{\n\
             \x20   println 'marker: ' + new File('/tmp/marker').text.trim()\n\
             \x20   meeting()\n\
             \x20 }}\n\
             }}\n\
             \n\
             tasks.register('runFiles') {{\n\
             \x20 doLast {{\n\
             \x20   def runs = new File(System.getProperty('user.home'), '.kotlin/daemon')\n\
             \x20   runs.mkdirs()\n\
             \x20   def marker = new File('/tmp/marker').text.trim()\n\
             \x20   new File(runs, marker + '.run').text = 'here'\n\
             \x20   println 'run-files=' + runs\n\
             \x20   meeting()\n\
             \x20   println 'seen=' + runs.list().sort().join(',')\n\
             \x20 }}\n\
             }}\n"
        ),
    )
    .unwrap();
}

/// What `gradle --status` says when the Gradle home it reads has no daemon
/// registered in it — which is the registry in the shared home, since that is
/// the home a session is given.
const NO_DAEMONS: &str = "No Gradle daemons are running.";

/// One build in the shared Gradle home before a proof starts two at once, so
/// that the two find its daemon registry already written.
///
/// **Gradle's own race on a home's first use, and nothing of a Sandbox's.** A
/// daemon registers itself in `daemon/<version>/registry.bin` under a lock
/// file, and a lock file Gradle has only just made starts out marked as not
/// unlocked cleanly. So the first registration in a home is refused as an
/// integrity violation, and Gradle recovers by writing a fresh registry
/// holding only the daemon in hand — under a second lock, not the one the
/// refusal was read under. Two daemons starting at the same moment in a new
/// home both take that path, the later write drops the earlier daemon, and
/// that daemon's client waits thirty seconds for an entry that is gone:
/// `Timeout waiting to connect to the Gradle daemon`. Read out of 9.7.1's
/// classes and reproduced with it, about one run in twenty on two busy
/// cores. It would happen to any two first builds in one new home, Sandboxed
/// or not, and it cannot happen once the registry has been written cleanly.
///
/// A machine's shared home is new once; every proof's is new every run. So
/// the proofs take that first write out of the race, with `help`, which
/// resolves nothing and leaves the stores as empty as it found them.
fn gradle_home_used_once(machine: &Machine, cache: &BuildCache, gradle: &Path) {
    let once = installing(
        &machine.sandbox(0, cache, reaching_nothing()),
        &format!("set -e\n'{}' {GRADLE_FLAGS} -q help\n", gradle.display()),
    );

    once.worked("a build before the proof's own writes the Gradle home's daemon registry");
}

/// Gradle: two Sandboxes building at once against one Gradle home, each in its
/// own Sandbox, a third building with `--offline` and the registry off the air,
/// and the control that says the third proved something.
///
/// **Each in its own Sandbox is the half the daemon is about.** Both builds
/// print the marker only their own Sandbox's `/tmp` holds, and both are still
/// inside their task when the other reaches its own, because they meet there
/// — so neither could have been the other's daemon's without printing the
/// wrong marker. And after both, the shared home has no daemon registered in
/// it, which is what a session given `-Dorg.gradle.daemon=false` leaves
/// behind: its build ran in a single-use daemon that was gone when it ended.
///
/// The third resolves the jar out of `caches/modules-2` in the shared home and
/// runs it, and the control fails in Gradle's own words for a module it has
/// nowhere to get.
#[tokio::test]
async fn two_gradle_builds_at_once_each_in_its_own_sandbox_fill_one_home_and_a_third_builds_out_of_it()
 {
    let Some(found) = tools("Gradle", &["gradle", "mvn", "javac", "zip"]) else {
        return;
    };
    let [gradle, mvn, javac, zip] = &found[..] else {
        unreachable!("four tools were asked for");
    };

    let machine = machine(5).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    let registry = maven_registry(&machine.registries.join("gradle"), mvn, javac, zip);

    for nth in 0..5 {
        gradle_consumer(machine.worktree(nth), Some(&registry.url));
    }

    gradle_home_used_once(&machine, &cache, gradle);

    // A home the sweep has just taken units out of, which is what the two
    // below start from: one `say` on its own, with nobody to meet, resolves
    // the jar into it first.
    filled_and_swept(
        &machine,
        4,
        &cache,
        reaching_nothing(),
        &format!(
            "set -e\n{marking}'{gradle}' {GRADLE_FLAGS} say\n",
            marking = marking("before"),
            gradle = gradle.display(),
        ),
        verkstead_server::languages::JVM,
    );

    let meet = dir.join("meeting");
    std::fs::create_dir_all(&meet).unwrap();

    let building = |marker: &str, other: &str| {
        format!(
            "set -e\n\
             {marking}\
             '{gradle}' {GRADLE_FLAGS} say -Pmeet='{meet}' -Pme={marker} -Pother={other}\n\
             '{gradle}' --status\n",
            marking = marking(marker),
            gradle = gradle.display(),
            meet = meet.display(),
        )
    };

    let first = starting(
        &machine.sandbox(0, &cache, reaching_nothing()),
        &building("first", "second"),
    );
    let second = starting(
        &machine.sandbox(1, &cache, reaching_nothing()),
        &building("second", "first"),
    );

    let first = finished(first);
    let second = finished(second);

    first.worked("the first session's build fills the Gradle home");
    second.worked("and the second one racing it finishes just as well");

    for (ran, marker, other) in [(&first, "first", "second"), (&second, "second", "first")] {
        assert!(
            ran.said.contains(&format!("marker: {marker}\n"))
                && ran.said.contains(&format!("met: {other}\n")),
            "the {marker} build ran in the {marker} session's own Sandbox, and was still \
             running when the {other} one reached its task. It said:\n{}",
            ran.said,
        );
        assert!(
            ran.said.contains(NO_DAEMONS),
            "and once it was over the shared Gradle home had no daemon registered in it, \
             so there was nothing for another session to attach to. It said:\n{}",
            ran.said,
        );
    }

    assert!(
        dir.join("gradle/caches/modules-2/files-2.1")
            .join(MAVEN_GROUP)
            .join(MAVEN_GREET)
            .is_dir(),
        "and what they downloaded is in the Gradle home the descriptor named, which \
         is the one the third session is about to be given. It said:\n{}",
        first.said,
    );

    // And the proof. The registry stops answering and Gradle is told not to
    // look: what is left to build out of is the home the two filled.
    registry.shut();

    let offline = format!(
        "set -e\n{marking}'{gradle}' {GRADLE_FLAGS} --offline say\n",
        marking = marking("third"),
        gradle = gradle.display(),
    );

    let third = installing(&machine.sandbox(2, &cache, reaching_nothing()), &offline);

    third.worked(
        "a third session builds offline with the registry gone, which it can only do out \
         of the shared Gradle home",
    );
    assert!(
        third.said.contains(&format!("said: {OUT_OF_THE_STORE}")),
        "and the jar it ran really came out of the store. It said:\n{}",
        third.said,
    );

    // And what the builds left is units the sweep can take out whole: one
    // version of a module at a time.
    whole_packages(
        &cache,
        verkstead_server::languages::JVM,
        "gradle",
        Units::Naming(&[MAVEN_GREET]),
    );

    let control = installing(
        &machine.sandbox(3, &machine.empty_cache(), reaching_nothing()),
        &offline,
    );

    assert!(
        !control.worked,
        "an empty Gradle home and no registry has to fail, or the build above \
         proved nothing about either. It said:\n{}",
        control.said,
    );
    assert!(
        control.said.contains("No cached version of")
            && control.said.contains("available for offline mode"),
        "and it fails for want of anywhere to get the jar, in Gradle's own words, \
         rather than for some other reason. It said:\n{}",
        control.said,
    );
}

/// And **a Repo asking for a daemon still gets none**, nor does a session that
/// asks for one in the shared home.
///
/// Gradle reads `org.gradle.daemon` out of a project's `gradle.properties` and
/// out of the Gradle home's, and the home's outranks the project's. The
/// descriptor's `-Dorg.gradle.daemon=false` is a system property of the
/// client, out of `GRADLE_OPTS`, and it outranks both. With
/// `org.gradle.jvmargs` beside the request as well: a Repo naming JVM options
/// is one whose build Gradle runs in a daemon forked for it, and that daemon is
/// a single-use one that goes when the build does.
///
/// **The home's file is the one a session could write.** It is shared, so a
/// session asking for a daemon there would be asking on behalf of every other
/// session on the machine. It still gets none.
///
/// One Sandbox and one build after another: each is followed by
/// `gradle --status`, which reads the registry in the shared home.
#[tokio::test]
async fn a_repo_or_the_shared_home_asking_for_a_daemon_still_gets_none() {
    let Some(found) = tools("Gradle", &["gradle"]) else {
        return;
    };
    let [gradle] = &found[..] else {
        unreachable!("one tool was asked for");
    };

    let machine = machine(1).await;
    let cache = machine.cache();

    gradle_consumer(machine.worktree(0), None);

    let asking = [
        ("gradle.properties", "org.gradle.daemon=true\n"),
        (
            "gradle.properties",
            "org.gradle.daemon=true\norg.gradle.jvmargs=-Xmx384m -Dasked=yes\n",
        ),
        (
            "\"$GRADLE_USER_HOME\"/gradle.properties",
            "org.gradle.daemon=true\n",
        ),
        (
            "\"$GRADLE_USER_HOME\"/gradle.properties",
            "org.gradle.daemon=true\norg.gradle.jvmargs=-Xmx384m -Dasked=yes\n",
        ),
    ];

    for (file, asks) in asking {
        let ran = installing(
            &machine.sandbox(0, &cache, reaching_nothing()),
            &format!(
                "set -e\n\
                 rm -f gradle.properties \"$GRADLE_USER_HOME\"/gradle.properties\n\
                 mkdir -p \"$GRADLE_USER_HOME\"\n\
                 printf '%s' '{asks}' > {file}\n\
                 {marking}\
                 '{gradle}' {GRADLE_FLAGS} mark\n\
                 '{gradle}' --status\n",
                marking = marking("asking"),
                gradle = gradle.display(),
            ),
        );

        ran.worked(&format!(
            "a build whose {file} asks for a daemon still works"
        ));
        assert!(
            ran.said.contains(NO_DAEMONS),
            "and {file} saying\n{asks}left no daemon registered in the shared Gradle \
             home. It said:\n{}",
            ran.said,
        );
    }
}

/// Two Sandboxes sharing one Gradle home, one build after the other: the first
/// session builds `mark` with `first`, then holds its Sandbox open while the
/// second builds `mark` with `second`. Each is the shell said before `gradle`
/// and the flags on its command line.
///
/// What comes back is the first session's run, and the second's with the exit
/// of its `gradle` under `built`, since whether it works is what is asked.
async fn one_session_beside_anothers_daemon(
    gradle: &Path,
    first: (&str, &str),
    second: (&str, &str),
) -> (Ran, Ran) {
    let machine = machine(2).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    for nth in 0..2 {
        gradle_consumer(machine.worktree(nth), None);
    }

    let meet = dir.join("meeting");
    std::fs::create_dir_all(&meet).unwrap();

    // A wait of two minutes at most, in the shell, on a file the other
    // Sandbox leaves under the Build Cache.
    let waiting = |on: &str| {
        format!(
            "i=0; while [ ! -e '{meet}/{on}' ]; do i=$((i+1)); \
             [ $i -gt 1200 ] && exit 3; sleep 0.1; done\n",
            meet = meet.display(),
        )
    };

    let (first_said, first_flag) = first;
    let (second_said, second_flag) = second;

    let first = starting(
        &machine.sandbox(0, &cache, reaching_nothing()),
        &format!(
            "set -e\n\
             {marking}\
             {first_said}\
             '{gradle}' {GRADLE_FLAGS} {first_flag} mark\n\
             touch '{meet}/built'\n\
             {waiting}",
            marking = marking("first"),
            gradle = gradle.display(),
            meet = meet.display(),
            waiting = waiting("done"),
        ),
    );

    let second = installing(
        &machine.sandbox(1, &cache, reaching_nothing()),
        &format!(
            "{waiting}\
             {marking}\
             {second_said}\
             '{gradle}' {GRADLE_FLAGS} {second_flag} mark\n\
             echo \"built=$?\"\n\
             touch '{meet}/done'\n",
            marking = marking("second"),
            gradle = gradle.display(),
            meet = meet.display(),
            waiting = waiting("built"),
        ),
    );

    (finished(first), second)
}

/// The control, which says the suite can see what the descriptor is there to
/// stop: **with the daemon left on, a second session's build lands in the
/// first session's daemon**, in the first session's Sandbox, where the second
/// one's Worktree is not bound.
///
/// Two ways back to a daemon, both with the Gradle home still shared:
///
/// - `GRADLE_OPTS` taken out, which is Gradle's own default, and what the
///   descriptor says without its second variable.
/// - And an explicit `--daemon` in both sessions, with `GRADLE_OPTS` in place.
///   The command line beats the variable, and **that is accepted rather than
///   defeated**: the failure is loud, and stage 06 found that a daemon of
///   Verkstead's own would not close it and turned down the per-session
///   registry that would. This line is here so that it stays a known hole
///   rather than a surprise.
///
/// The first session builds, leaves its daemon up and waits. The second builds
/// while it waits and fails, in Gradle's words, to change into a directory its
/// daemon cannot see.
#[tokio::test]
async fn with_the_daemon_back_on_a_second_sessions_build_lands_in_the_firsts_sandbox() {
    let Some(found) = tools("Gradle", &["gradle"]) else {
        return;
    };
    let [gradle] = &found[..] else {
        unreachable!("one tool was asked for");
    };

    for (how, back_on, flag) in [
        ("with GRADLE_OPTS taken out", "unset GRADLE_OPTS\n", ""),
        ("with --daemon on the command line", "", "--daemon"),
    ] {
        let (first, second) =
            one_session_beside_anothers_daemon(gradle, (back_on, flag), (back_on, flag)).await;

        first.worked(&format!("{how}, the first session builds, in a daemon"));
        assert!(
            first.said.contains("marker: first"),
            "in its own Sandbox. It said:\n{}",
            first.said,
        );

        assert_ne!(
            line(&second, "built"),
            "0",
            "{how}, the second session's build cannot work: it reached the first \
             session's daemon. It said:\n{}",
            second.said,
        );
        assert!(
            second
                .said
                .contains("Could not set process working directory")
                && second.said.contains("could not setcwd()"),
            "and it fails because the daemon it reached is in the first session's \
             Sandbox, where the second session's Worktree is not there to change \
             into. It said:\n{}",
            second.said,
        );
    }
}

/// And **the `--daemon` hole reaches only a session that asks for a daemon
/// itself.** A daemon one session's explicit `--daemon` left up is registered
/// in the shared home, but a session given `-Dorg.gradle.daemon=false` never
/// looks there: Gradle starts a single-use daemon of its own for every such
/// build, whatever is idle beside it.
///
/// So the second session here, with `GRADLE_OPTS` as the descriptor gave it and
/// no flag, builds in its own Sandbox while the first one's daemon is up.
#[tokio::test]
async fn a_daemon_left_up_by_one_sessions_daemon_flag_is_not_reached_by_a_session_with_it_off() {
    let Some(found) = tools("Gradle", &["gradle"]) else {
        return;
    };
    let [gradle] = &found[..] else {
        unreachable!("one tool was asked for");
    };

    let (first, second) =
        one_session_beside_anothers_daemon(gradle, ("", "--daemon"), ("", "")).await;

    first.worked("the first session builds, in a daemon its `--daemon` asked for");
    assert!(
        first.said.contains("marker: first"),
        "in its own Sandbox. It said:\n{}",
        first.said,
    );

    assert_eq!(
        line(&second, "built"),
        "0",
        "a session with the daemon off builds while the other session's daemon is up. \
         It said:\n{}",
        second.said,
    );
    assert!(
        second.said.contains("marker: second"),
        "and in its own Sandbox, rather than in the daemon the first session left \
         registered in the shared home. It said:\n{}",
        second.said,
    );
}

/// And **Kotlin's compile daemon stays in the Sandbox that started it**, which
/// is what makes a Kotlin build on Gradle need nothing of its own.
///
/// A Kotlin build compiles in a daemon of the Kotlin Gradle plugin's, which
/// outlives the build and is reached over the loopback, the way Gradle's is.
/// What a second build finds it by is its *run files*. Measured with the
/// Kotlin Gradle plugin 2.2.20 in a Sandbox with the shared Gradle home, the
/// plugin starts the daemon with `--daemon-runFilesPath <user.home>/.kotlin/daemon`
/// — **the JVM's `user.home`**, which is the account's home out of the password
/// database rather than the session's `HOME`, and in no case under the Gradle
/// home.
///
/// No Sandbox binds the account's home, so that path is a directory of each
/// Sandbox's own, made in the root it was built on. This is that, observed
/// from inside Gradle's own JVM, where the plugin reads it: two builds at
/// once, each leaving a run file where the plugin would and meeting before
/// they look. Each finds its own and not the other's.
///
/// **A proxy for the plugin rather than the plugin itself**, because it
/// resolves from the Gradle Plugin Portal, which nothing in this file reaches.
/// What is proven is the directory; which directory the plugin names is the
/// measurement above.
#[tokio::test]
async fn a_kotlin_compile_daemons_run_files_stay_in_the_sandbox_that_started_it() {
    let Some(found) = tools("Gradle", &["gradle"]) else {
        return;
    };
    let [gradle] = &found[..] else {
        unreachable!("one tool was asked for");
    };

    let machine = machine(2).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    for nth in 0..2 {
        gradle_consumer(machine.worktree(nth), None);
    }

    gradle_home_used_once(&machine, &cache, gradle);

    let meet = dir.join("meeting");
    std::fs::create_dir_all(&meet).unwrap();

    let running = |marker: &str, other: &str| {
        format!(
            "set -e\n\
             {marking}\
             '{gradle}' {GRADLE_FLAGS} runFiles -Pmeet='{meet}' -Pme={marker} -Pother={other}\n",
            marking = marking(marker),
            gradle = gradle.display(),
            meet = meet.display(),
        )
    };

    let first = starting(
        &machine.sandbox(0, &cache, reaching_nothing()),
        &running("first", "second"),
    );
    let second = starting(
        &machine.sandbox(1, &cache, reaching_nothing()),
        &running("second", "first"),
    );

    let first = finished(first);
    let second = finished(second);

    for (ran, marker) in [(&first, "first"), (&second, "second")] {
        ran.worked(&format!("the {marker} session's build runs"));

        let runs = line(ran, "run-files");

        assert!(
            !Path::new(&runs).starts_with(dir),
            "the {marker} session's Kotlin run files are not under the Build Cache, \
             where the Gradle home is: they are at {runs}",
        );
        assert_eq!(
            line(ran, "seen"),
            format!("{marker}.run"),
            "and the {marker} session sees only its own run file there, while the \
             other session's build is still running with one of its own. It said:\n{}",
            ran.said,
        );
    }
}

/// A Gradle build with one cacheable task, `shout`, which writes its input out
/// in capitals under `build/`.
///
/// The input is declared relative, so the task's cache key is the same in any
/// Conversation's Worktree whatever its path: what one Conversation stored is
/// what another would look up. `caching` is the Repo's
/// `org.gradle.caching=true` in its `gradle.properties`, and `own` its
/// `settings.gradle` pointing `buildCache.local` at a directory of its own
/// checkout.
fn gradle_cacheable(worktree: &Path, caching: bool, own: bool) {
    let settings = if own {
        "rootProject.name = 'app'\n\
         buildCache { local { directory = file('own-build-cache') } }\n"
    } else {
        "rootProject.name = 'app'\n"
    };

    std::fs::write(worktree.join("settings.gradle"), settings).unwrap();
    std::fs::write(
        worktree.join("gradle.properties"),
        if caching {
            "org.gradle.caching=true\n"
        } else {
            ""
        },
    )
    .unwrap();
    std::fs::write(worktree.join("greeting.txt"), "hello from the cache\n").unwrap();
    std::fs::write(
        worktree.join("build.gradle"),
        "tasks.register('shout') {\n\
         \x20 def from = file('greeting.txt')\n\
         \x20 def to = layout.buildDirectory.file('shout.txt')\n\
         \x20 inputs.file(from).withPathSensitivity(PathSensitivity.RELATIVE)\n\
         \x20 outputs.file(to)\n\
         \x20 outputs.cacheIf { true }\n\
         \x20 doLast { to.get().asFile.text = from.text.toUpperCase() }\n\
         }\n",
    )
    .unwrap();
}

/// Gradle's build cache: **shared between Conversations for a Repo that
/// switches it on, and switched on for none by Verkstead.**
///
/// The local build cache lives under the Gradle home, at
/// `caches/build-cache-1` (measured with 8.14.4), and the descriptor shares
/// that home. So a Repo with `org.gradle.caching=true` already has one cache
/// for the machine, and the descriptor needs nothing more to give it one.
/// It sets nothing about caching, because `-Dorg.gradle.caching=true` in
/// `GRADLE_OPTS` would beat a Repo that wrote `org.gradle.caching=false` on
/// purpose.
///
/// Four Conversations, one after another, each in a fresh Worktree with no
/// `build/` in it:
///
/// 1. A Repo that switches caching on runs `shout`, and its output is stored
///    in the shared home.
/// 2. The same Repo in a second Conversation takes `shout` `FROM-CACHE`.
/// 3. The same build with caching left off runs `shout`, with the entry it
///    would have hit sitting in the cache: Verkstead switched nothing on.
/// 4. And a Repo whose `settings.gradle` points `buildCache.local` at its own
///    directory keeps it. `shout` runs, rather than coming out of the shared
///    cache, and its output is stored in the Repo's directory.
#[tokio::test]
async fn a_task_one_conversation_caches_comes_from_the_cache_in_another_only_where_the_repo_asked()
{
    let Some(found) = tools("Gradle", &["gradle"]) else {
        return;
    };
    let [gradle] = &found[..] else {
        unreachable!("one tool was asked for");
    };

    let machine = machine(4).await;
    let cache = machine.cache();
    let dir = cache.dir().expect("the fixture's cache has a directory");

    gradle_cacheable(machine.worktree(0), true, false);
    gradle_cacheable(machine.worktree(1), true, false);
    gradle_cacheable(machine.worktree(2), false, false);
    gradle_cacheable(machine.worktree(3), true, true);

    let shouting = format!(
        "set -e\n'{gradle}' {GRADLE_FLAGS} shout\ncat build/shout.txt\n",
        gradle = gradle.display(),
    );
    let shout =
        |nth: usize| installing(&machine.sandbox(nth, &cache, reaching_nothing()), &shouting);

    let ran = "> Task :shout\n";
    let from_the_cache = "> Task :shout FROM-CACHE\n";

    let first = shout(0);

    first.worked("the first Conversation's build runs `shout`");
    assert!(
        first.said.contains(ran) && first.said.contains("HELLO FROM THE CACHE"),
        "and runs it, with nothing in the cache yet to take it from. It said:\n{}",
        first.said,
    );

    let stored = dir.join("gradle/caches/build-cache-1");
    assert!(
        holds_anything(&stored),
        "and the output is stored in the build cache under the shared Gradle home, at \
         {}. It said:\n{}",
        stored.display(),
        first.said,
    );

    let second = shout(1);

    second.worked("a second Conversation's build of the same Repo works");
    assert!(
        second.said.contains(from_the_cache) && second.said.contains("HELLO FROM THE CACHE"),
        "and takes `shout` from the cache the first Conversation filled, in a Worktree \
         of its own with nothing built in it. It said:\n{}",
        second.said,
    );

    let unasked = shout(2);

    unasked.worked("a Repo that does not switch caching on builds");
    assert!(
        unasked.said.contains(ran) && !unasked.said.contains("FROM-CACHE"),
        "and runs `shout` itself, with the entry it would have hit in the shared \
         cache: nothing a session is given switches caching on. It said:\n{}",
        unasked.said,
    );

    let own = shout(3);

    own.worked("a Repo with a build cache of its own builds");
    assert!(
        own.said.contains(ran) && !own.said.contains("FROM-CACHE"),
        "and runs `shout` rather than taking it from the shared cache, because the \
         Repo's `buildCache.local` wins over the Gradle home's. It said:\n{}",
        own.said,
    );
    assert!(
        holds_anything(&machine.worktree(3).join("own-build-cache")),
        "and stores its output in the directory the Repo named. It said:\n{}",
        own.said,
    );
}

// ---------------------------------------------------------------------------
// Rust: the cargo half, which is a store the sweep bounds.
// ---------------------------------------------------------------------------

/// The crate the Rust proof fetches out of a registry, and the version of it.
const CRATE: &str = "greet";
const CRATE_VERSION: &str = "1.0.0";

/// And the one it fetches out of a git repository, which is the other half of
/// what `CARGO_HOME` holds.
const GIT_CRATE: &str = "wave";

/// Lay a **sparse registry** out under `at`, holding [`CRATE`], and serve it —
/// the protocol `cargo` speaks to crates.io itself: a `config.json` naming where
/// a crate is downloaded from, and an index file per crate at the path its name
/// spells, one line of JSON per version.
///
/// The `.crate` is a gzipped tar with everything under `<name>-<version>/`,
/// which is what `cargo package` writes, and its `cksum` in the index is the
/// sha256 of the file as served — which cargo checks before it unpacks a byte.
fn cargo_registry(at: &Path, tar: &Path) -> Registry {
    let named = format!("{CRATE}-{CRATE_VERSION}");
    let inside = at.join(&named);
    std::fs::create_dir_all(inside.join("src")).unwrap();

    std::fs::write(
        inside.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{CRATE}\"\nversion = \"{CRATE_VERSION}\"\nedition = \"2021\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        inside.join("src/lib.rs"),
        format!("pub const GREETING: &str = \"{OUT_OF_THE_STORE}\";\n"),
    )
    .unwrap();

    let archive = at.join(format!("{named}.crate"));
    let made = Command::new(tar)
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(at)
        .arg(&named)
        .stdin(Stdio::null())
        .status()
        .expect("the suite's own `tar`");

    assert!(made.success(), "the .crate was not built");

    let crate_file = std::fs::read(&archive).unwrap();
    let cksum = format!("{:x}", Sha256::digest(&crate_file));

    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .expect("a loopback port for the registry");
    let url = format!("http://{}", listener.local_addr().expect("the port it got"));

    listener
        .set_nonblocking(true)
        .expect("what tokio takes a standard listener over");

    let config = format!("{{\"dl\":\"{url}/crates\",\"api\":null}}");
    let index = format!(
        "{{\"name\":\"{CRATE}\",\"vers\":\"{CRATE_VERSION}\",\"deps\":[],\"cksum\":\"{cksum}\",\
          \"features\":{{}},\"yanked\":false}}\n"
    );

    // A name of five letters or more is under its first two and its next two.
    let app = Router::new()
        .route(
            "/config.json",
            get(move || async move { ([(header::CONTENT_TYPE, "application/json")], config) }),
        )
        .route(
            &format!("/{}/{}/{CRATE}", &CRATE[..2], &CRATE[2..4]),
            get(move || async move { index }),
        )
        .route(
            &format!("/crates/{CRATE}/{CRATE_VERSION}/download"),
            get(move || async move {
                (
                    [(header::CONTENT_TYPE, "application/octet-stream")],
                    crate_file,
                )
            }),
        );

    let (stop, stopping) = tokio::sync::oneshot::channel();

    let serving = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for the registry's own thread");

        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener)
                .expect("the listener this thread was handed");

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

/// A git repository holding [`GIT_CRATE`], which a manifest names by a
/// `file://` URL — read-only, and bound only where a proof hands it over.
fn cargo_git_crate(at: &Path) -> PathBuf {
    let repo = repository(at.to_owned());

    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(
        repo.join("Cargo.toml"),
        format!("[package]\nname = \"{GIT_CRATE}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .unwrap();
    std::fs::write(repo.join("src/lib.rs"), "pub fn wave() {}\n").unwrap();
    git(&repo, &["add", "Cargo.toml", "src/lib.rs"]);
    git(&repo, &["commit", "-m", "a crate"]);

    repo
}

/// And what a Conversation's Worktree holds: a package depending on both, with
/// the registry named in its own `.cargo/config.toml` the way a Repo names an
/// alternative registry.
fn cargo_consumer(worktree: &Path, registry: &str, git_crate: &Path) {
    std::fs::create_dir_all(worktree.join("src")).unwrap();
    std::fs::create_dir_all(worktree.join(".cargo")).unwrap();

    std::fs::write(
        worktree.join("Cargo.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\n\
             {CRATE} = {{ version = \"{CRATE_VERSION}\", registry = \"proof\" }}\n\
             {GIT_CRATE} = {{ git = \"file://{}\" }}\n",
            git_crate.display(),
        ),
    )
    .unwrap();
    std::fs::write(worktree.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        worktree.join(".cargo/config.toml"),
        format!("[registries.proof]\nindex = \"sparse+{registry}/\"\n"),
    )
    .unwrap();
}

/// Rust's cargo half: `CARGO_HOME`, two Sandboxes fetching at once, a third
/// fetching with `--offline` and the registry gone, and the control.
///
/// **Here for the sweep rather than for the variable**, which every Rust
/// session has had since before the package stores: the cargo half is swept
/// under Rust's size like any other store, so its units have to be whole crates
/// against the layout cargo really writes — a registry's `.crate` and the
/// source unpacked out of it, and a git dependency's database and checkout.
///
/// `cargo fetch` rather than a build, because a fetch is the whole of what
/// fills this store and a build would want a linker in the Sandbox besides.
/// What says the third was served out of the store is that `--offline`
/// succeeded and the source it unpacked is there to read.
#[tokio::test]
async fn two_cargo_fetches_at_once_fill_one_cargo_home_and_a_third_fetches_out_of_it() {
    let Some(found_them) = tools("Rust", &["cargo", "tar"]) else {
        return;
    };
    let (cargo, tar) = (&found_them[0], &found_them[1]);

    let machine = machine(5).await;
    let cache = machine.cache();

    let registry = cargo_registry(&machine.registries.join("cargo"), tar);
    let git_crate = cargo_git_crate(&machine.registries.join(GIT_CRATE));

    for nth in 0..5 {
        cargo_consumer(machine.worktree(nth), &registry.url, &git_crate);
    }

    let reaching = || vec![Bind::readable(git_crate.clone())];

    let fetching = format!("set -e\n{cargo} fetch\n", cargo = cargo.display());

    // A CARGO_HOME the sweep has just taken units out of, which is what the two
    // below start from.
    filled_and_swept(
        &machine,
        4,
        &cache,
        reaching(),
        &fetching,
        verkstead_server::languages::RUST,
    );

    let first = starting(&machine.sandbox(0, &cache, reaching()), &fetching);
    let second = starting(&machine.sandbox(1, &cache, reaching()), &fetching);

    finished(first).worked("the first session's fetch fills CARGO_HOME");
    finished(second).worked("and the second one racing it finishes just as well");

    // The lockfile a Repo would have committed, which pins the git dependency
    // to the revision the store holds.
    let lock = std::fs::read(machine.worktree(0).join("Cargo.lock"))
        .expect("the first fetch writes a Cargo.lock");

    for nth in 2..4 {
        std::fs::write(machine.worktree(nth).join("Cargo.lock"), &lock).unwrap();
    }

    registry.shut();

    let offline = format!(
        "set -e\n{cargo} fetch --offline\ncat \"$CARGO_HOME\"/registry/src/*/{CRATE}-{CRATE_VERSION}/src/lib.rs\n",
        cargo = cargo.display(),
    );

    let third = installing(&machine.sandbox(2, &cache, vec![]), &offline);

    third.worked(
        "a third session fetches with its registry and the git repository denied, which it \
         can only do out of the shared CARGO_HOME",
    );
    assert!(
        third.said.contains(OUT_OF_THE_STORE),
        "and the source it reads is the crate's own. It said:\n{}",
        third.said,
    );

    // And what the fetches left is units the sweep can take out whole: a
    // crate's download apart from its source, and the git dependency's
    // database apart from its checkout.
    whole_packages(
        &cache,
        verkstead_server::languages::RUST,
        "cargo",
        Units::Naming(&[CRATE, GIT_CRATE]),
    );

    let control = installing(
        &machine.sandbox(3, &machine.empty_cache(), vec![]),
        &offline,
    );

    assert!(
        !control.worked,
        "an empty CARGO_HOME and no registry has to fail, or the fetch above proved \
         nothing. It said:\n{}",
        control.said,
    );
    assert!(
        control.said.contains("offline"),
        "and it fails for want of the registry rather than for some other reason. It \
         said:\n{}",
        control.said,
    );
}
