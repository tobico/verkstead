//! What a session on a Mac can reach, asked by running a probe inside a
//! Conversation's sandbox.
//!
//! The macOS half of `tests/sandbox.rs`, and settled the same way: nothing here
//! reads the policy the sandbox was built with. The policy *is* what is being
//! tested, and a test that asserts it asserts itself — it would go on passing
//! while Apple changed what one of its rules meant, or while a later rule
//! quietly widened another. What settles whether the rest of the machine is
//! still reachable is a command inside the sandbox trying to reach it, and
//! reporting what happened.
//!
//! **The vocabulary is a word apart from the Linux suite's**, and that word is
//! the difference between the two boundaries (ADR-0012). A path a session may
//! not reach on Linux is `absent`, because the mount namespace it is in was
//! never given one; the same path here is `refused` — really there, its
//! metadata readable, and every open of it denied. So a probe that finds
//! `absent` on a Mac has found something odd, and one that finds `refused` has
//! found the boundary working.
//!
//! With three exceptions, and every one of them is the boundary working rather
//! than odd. A session's HOME is a directory of Verkstead's own here, so what
//! is *not* in it — the machine's keys, somebody's shell history — is absent
//! rather than refused: those are in another directory entirely, and that one
//! is refused. The account's own skills are refused so far that they are not
//! there to be looked at either: what covers them is a rule denying everything
//! about the path, metadata included, which is the strongest thing a policy can
//! say and the closest it comes to a mount standing on it. And a path nobody
//! ever made is absent inside because it is absent outside — which is what a
//! configured bind naming a directory that is not there comes to: the entry is
//! skipped, and what a session finds at that name is nothing rather than a
//! refusal.
//!
//! **Built everywhere and run on one machine.** Every test here is compiled on
//! the Linux runner and *ignored* there rather than left out of the build: what
//! they run is `sandbox-exec` and what they assert is a policy only a Mac
//! enforces, but a test nobody compiles is one that rots against the types it
//! is written for. So the Linux job reports them ignored, and the `macos-15`
//! job runs them.

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Bind, Executable, Homes, Reachable, Sandbox, SandboxConfig};
use verkstead_server::settings::{RustBuildCache, Settings};
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// Where the server this Conversation belongs to is listening — which is what a
/// session inside is told to put its Question Sets to.
const LISTENING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8422);

/// What stands in for the server's own image: an executable that says which
/// build it is, as the Linux suite's does.
const SAYS_WHICH_BUILD: &str = "#!/bin/sh\nprintf 'verkstead 0.0.0-the-servers-own\\n'\n";

/// The shell the probe is written in, by its whole path.
///
/// Which is also the one path on a Mac that reads `/private/var/select/sh` on
/// its way up, and so the one that would fail if the system list forgot it.
const SH: &str = "/bin/sh";

/// And the one tool the probe reaches for that a shell has no builtin of.
///
/// Absolute, as everything the probe runs is: what a session's `PATH` holds is
/// one of the things being probed, so a probe that found its own tools through
/// it would be leaning on the answer.
const CURL: &str = "/usr/bin/curl";

/// And the one the companion probes read a file with. `git` is the other, and
/// it is looked for rather than written out — see [`on_the_host`].
const CAT: &str = "/bin/cat";

/// And the one thing here asked of the machine rather than of a sandbox: the
/// process table, which is where a lifetime is read off.
const PS: &str = "/bin/ps";

/// What the stub sccache says when it is asked to be a session's client, which
/// is the half a session runs — see [`Grilling::sccache`] for the other one.
const SAYS_WHICH_SCCACHE: &str = "printf 'sccache 0.0.0-the-one-resolved\\n'\n";

/// What the compile server's report is called inside the build cache, which is
/// the one directory both it and this test can write to and read.
const COMPILE_SERVER_REPORT: &str = "compile-server-report";

/// A Conversation part-way through its first grilling: a Repo inside a Watched
/// Path, a Profile to run as, and a worktree under Verkstead's own state
/// directory.
///
/// Everything is real, for the reason the Linux fixture is: what the sandbox
/// describes is read off those, and a fixture that hand-built the paths would
/// prove the probe works rather than that the sandbox does.
struct Grilling {
    /// Kept alive for as long as the fixture is: the directories go when these
    /// drop, and a worktree that vanished mid-probe would fail obscurely.
    watched: tempfile::TempDir,
    state: tempfile::TempDir,
    home: tempfile::TempDir,

    /// Where the Repo is, and the sibling checkout beside it that no session
    /// has any business seeing.
    repo: PathBuf,
    sibling: PathBuf,

    conversation: store::Conversation,
    profile: store::Profile,

    /// The store the Conversation and its Profiles were written into, for the
    /// tests that want a second account to run under.
    pool: sqlx::SqlitePool,

    skills: Skills,
    verkstead: Executable,
    handoffs: Handoffs,
    settings: Settings,
}

impl Grilling {
    /// The sandbox this Conversation's session would run in.
    ///
    /// With no shared build cache and no configured binds, which is what every
    /// test that is not about one of those wants: a cache is one directory and
    /// four variables, and a test asking what else is inside should not have to
    /// know about them.
    fn sandbox(&self) -> Sandbox {
        self.sandbox_under(&self.profile)
    }

    /// And one around a Profile that is not the fixture's own, which is how
    /// each of the other account shapes gets a sandbox to be probed inside.
    fn sandbox_under(&self, profile: &store::Profile) -> Sandbox {
        self.composed(profile, &BuildCache::none(), vec![])
    }

    /// One with `extra` on top of it, which is whatever Sandbox Configuration
    /// asked for.
    fn sandbox_with(&self, extra: Vec<Bind>) -> Sandbox {
        self.composed(&self.profile, &BuildCache::none(), extra)
    }

    /// And one built around `cache`, which is what the tests about the shared
    /// build cache ask for. What is switched on and how big it may grow is read
    /// out of `config.yaml` as the sandbox is built, so a test says that by
    /// writing the file — see [`Grilling::configure`].
    fn sandbox_caching(&self, cache: &BuildCache) -> Sandbox {
        self.composed(&self.profile, cache, vec![])
    }

    fn composed(&self, profile: &store::Profile, cache: &BuildCache, extra: Vec<Bind>) -> Sandbox {
        Sandbox::for_conversation(
            &self.conversation,
            profile,
            &self.homes(),
            &Reachable::at(LISTENING),
            &self.skills,
            &self.verkstead,
            &self.handoffs,
            &self.settings.secrets(),
            &self.settings.config(),
            cache,
            extra,
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// `config.yaml`, which is who a session commits as and how its shared
    /// build cache is set — read as each sandbox is built, so a test writes it
    /// before asking for one.
    fn configure(&self, yaml: &str) {
        std::fs::write(self.settings.config_path(), yaml).unwrap();
    }

    /// And the file the GitHub token is in, which the compile server has no
    /// business reaching.
    fn configure_github_token(&self, yaml: &str) {
        std::fs::write(self.settings.secrets_path(), yaml).unwrap();
    }

    /// Where the shared build cache is on the host, which is a directory of the
    /// fixture's own rather than the machine's XDG one.
    fn cache_dir(&self) -> PathBuf {
        self.state.path().join("build-cache")
    }

    /// The Worktrees directory the compile server is shown of the Data
    /// Directory, which is where this fixture's own worktree already is.
    fn worktrees_dir(&self) -> PathBuf {
        self.state.path().join("worktrees")
    }

    /// A cache at that directory, with a stub sccache where `compiling` says
    /// so.
    ///
    /// The stub is a script that says which build it is, for the reason
    /// [`SAYS_WHICH_BUILD`] is one: what has to be shown is that the file the
    /// server resolved is the file a session finds beside its own `verkstead`,
    /// and a real sccache would answer that question with whatever the machine
    /// happened to have installed.
    fn cache(&self, compiling: bool) -> BuildCache {
        let dir = self.cache_dir();
        std::fs::create_dir_all(&dir).unwrap();

        BuildCache::at(
            dir,
            compiling.then(|| self.sccache()),
            self.state.path().to_owned(),
        )
    }

    /// The stub sccache, written where the server would have found a real one.
    ///
    /// Both halves of one, as the Linux suite's is. Asked to be the **compile
    /// server** it writes down what its own sandbox can reach and then sits
    /// there holding it, which is what a real sccache does and the only thing
    /// about it worth asserting: the paths it is checked against are this
    /// fixture's, so they are written into the script rather than looked for in
    /// an environment that is closed by the time it runs. Asked anything else
    /// it is a session's client, and says which build it is.
    fn sccache(&self) -> PathBuf {
        let path = self.state.path().join("sccache");
        let settings = Settings::in_data_dir(self.state.path());

        let script = format!(
            r#"#!/bin/sh
{PROBE}
if [ "${{SCCACHE_START_SERVER-}}" = "1" ]; then
    {{
        # Which is the pid the server holds, `sandbox-exec` having exec'd this
        # rather than started it beside itself — and so the process group
        # everything about this sandbox's lifetime is said about.
        say pid $$
        say home "$HOME"
        say no-daemon "${{SCCACHE_NO_DAEMON-unset}}"
        say idle "${{SCCACHE_IDLE_TIMEOUT-unset}}"
        say size "${{SCCACHE_CACHE_SIZE-unset}}"
        say sccache-dir "${{SCCACHE_DIR-unset}}"
        dir {worktrees} worktrees
        dir {cache} cache
        dir {handoffs} handoffs
        file {database} database
        file {config} config
        file {secrets} secrets
    }} > {report}
    # Held open, because what is being asserted is a server that is *there* —
    # and bounded, because nothing in the test stops it.
    sleep 30
    exit 0
fi
{SAYS_WHICH_SCCACHE}"#,
            worktrees = quoted(&self.worktrees_dir()),
            cache = quoted(&self.cache_dir()),
            handoffs = quoted(&self.state.path().join("handoffs")),
            database = quoted(&self.state.path().join("verkstead.db")),
            config = quoted(&settings.config_path()),
            secrets = quoted(&settings.secrets_path()),
            report = quoted(&self.cache_dir().join(COMPILE_SERVER_REPORT)),
        );

        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        path
    }

    /// The companion of that name, as the Conversation now carries it — where
    /// it was checked out, and what it holds.
    fn companion(&self, name: &str) -> &store::Companion {
        self.conversation
            .companions
            .iter()
            .find(|companion| companion.repo.name == name)
            .unwrap_or_else(|| panic!("the fixture added {name} as a companion"))
    }

    /// And where it was checked out, which is what a session is given.
    fn companion_worktree(&self, name: &str) -> &Path {
        self.companion(name)
            .worktree
            .as_deref()
            .expect("a grilling Conversation's companions are checked out")
    }

    fn worktree(&self) -> &Path {
        self.conversation
            .worktree
            .as_deref()
            .expect("a grilling Conversation has a worktree")
    }

    /// The Repo's own git directory, which is what a commit inside writes to.
    fn git_dir(&self) -> PathBuf {
        self.repo.join(".git")
    }

    /// A Profile of the second agent type, whose whole account is one home,
    /// with something of the account's inside it — so that "the account is
    /// there" is a claim about a directory with contents.
    async fn codex_profile(&self) -> store::Profile {
        let home = self.watched.path().join("codex-account/.codex");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("config.toml"), "# the account's own\n").unwrap();

        self.profile_of("codex", store::Account::Codex { home }, "gpt-5-codex")
            .await
    }

    /// And one of the third, whose account is one home as the second's is.
    async fn grok_profile(&self) -> store::Profile {
        let home = self.watched.path().join("grok-account/.grok");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("user-settings.json"), "{}\n").unwrap();

        self.profile_of("grok", store::Account::Grok { home }, "grok-4.6")
            .await
    }

    /// And one of the fourth, whose account is one home holding the two
    /// directories opencode keeps an account in — made the way a human makes
    /// one, a `HOME=<it> opencode` run leaving exactly these.
    async fn opencode_profile(&self) -> store::Profile {
        let home = self.watched.path().join("opencode-account/opencode");
        let config = home.join(".config/opencode");
        let data = home.join(".local/share/opencode");

        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("opencode.json"), "{}\n").unwrap();
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("auth.json"), "{}\n").unwrap();

        self.profile_of(
            "opencode",
            store::Account::OpenCode { home },
            "opencode/big-pickle",
        )
        .await
    }

    /// One of those, saved into the same store the fixture's own was.
    async fn profile_of(&self, name: &str, account: store::Account, model: &str) -> store::Profile {
        store::create_profile(
            &self.pool,
            &store::ProfileFacts {
                name: name.to_owned(),
                account,
                models: vec![model.to_owned()],
            },
        )
        .await
        .unwrap()
        .expect("nothing is called that yet")
    }

    /// The homes this server hands out: the fixture's own directory where the
    /// machine's is, and its state directory to make a session's own under.
    ///
    /// The two are not the same thing here, and that is what half of these
    /// tests turn on: the first is the host's home, which is refused, and a
    /// session's own HOME is a directory of Verkstead's made fresh under the
    /// second.
    fn homes(&self) -> Homes {
        Homes::on(
            Platform::HERE,
            self.home.path().to_owned(),
            self.state.path(),
        )
    }

    /// Where the machine's own home is, which is nobody inside's.
    fn home_path(&self) -> &Path {
        self.home.path()
    }
}

/// Stand one up, with no companion repos — which is the ordinary Conversation
/// and what most of these tests are about.
async fn grilling() -> Grilling {
    grilling_alongside(&[]).await
}

/// And one configured with companion repos, each registered under the name
/// given and added in the mode given.
///
/// They are added while the Conversation is still drafting, which is the only
/// time they can be, and then checked out beside its own the way a grill start
/// checks them out: a read-write companion on a branch of its own, a read-only
/// one detached at the commit its base resolved to.
async fn grilling_alongside(companions: &[(&str, store::CompanionMode)]) -> Grilling {
    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // A gitconfig of the machine's and a private key beside it: neither is any
    // session's business, and both are here so that "the host's home is
    // refused" is a claim about a directory with something in it.
    std::fs::write(
        home.path().join(".gitconfig"),
        "[user]\n\tname = Whoever The Host Is\n\temail = host@verkstead.invalid\n",
    )
    .unwrap();
    std::fs::create_dir_all(home.path().join(".ssh")).unwrap();
    std::fs::write(home.path().join(".ssh/id_ed25519"), "a private key\n").unwrap();

    let repo = repository(watched.path().join("verkstead"));
    let sibling = repository(watched.path().join("something-else"));

    let pool = store::open_database(&state.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo_row = store::register_repo(&pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("the Repo registers");

    let claude_dir = watched.path().join("account/.claude");
    let config_file = watched.path().join("account/.claude.json");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(claude_dir.join("settings.json"), "{}\n").unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    // And skills of the account's own, which are what a session must not be
    // grilled by: a Profile is an account to run as rather than a second
    // opinion about how to work. Really there, so that a probe finding nothing
    // has found the rule covering them rather than an empty directory.
    std::fs::create_dir_all(claude_dir.join("skills/the-accounts-own")).unwrap();
    std::fs::write(
        claude_dir.join("skills/the-accounts-own/SKILL.md"),
        "# what the account would have been grilled by\n",
    )
    .unwrap();

    let profile = store::create_profile(
        &pool,
        &store::ProfileFacts {
            name: "work".to_owned(),
            account: store::Account::Claude {
                claude_dir,
                config_file,
            },
            models: vec!["claude-opus-5".to_owned()],
        },
    )
    .await
    .unwrap()
    .expect("the Profile saves");

    let id = store::start_conversation(&pool, repo_row.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Conversation starts");

    store::set_grilling_pairing(&pool, id, profile.id, profile.model())
        .await
        .unwrap();

    // The worktree git itself made, where the server puts one.
    let worktree = state.path().join("worktrees/verkstead-rate-limiting");
    std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "rate-limiting",
            &worktree.to_string_lossy(),
            &commit,
        ],
    );

    // And one beside it per companion the test asked for, in the shape its mode
    // gives it — see [`grilling_alongside`].
    let mut checkouts = Vec::new();

    for (name, mode) in companions {
        let path = repository(watched.path().join(name));
        let registered = store::register_repo(&pool, &path, name, "main")
            .await
            .unwrap()
            .expect("the companion Repo registers");

        assert_eq!(
            store::add_companion(&pool, id, registered.id)
                .await
                .unwrap(),
            store::Adding::Added,
        );
        assert_eq!(
            store::configure_companion(&pool, id, registered.id, store::Change::Mode(*mode))
                .await
                .unwrap(),
            store::Configured::Saved,
        );

        let at = git(&path, &["rev-parse", "HEAD"]).trim().to_owned();

        // Named for the Repo and what the checkout holds, as the real one is:
        // the branch where there is one, and the base it stands at where there
        // is not.
        let checkout = match mode {
            store::CompanionMode::ReadOnly => {
                let checkout = state.path().join(format!("worktrees/{name}-main"));
                git(
                    &path,
                    &[
                        "worktree",
                        "add",
                        "--detach",
                        &checkout.to_string_lossy(),
                        &at,
                    ],
                );
                checkout
            }
            store::CompanionMode::ReadWrite => {
                let checkout = state.path().join(format!("worktrees/{name}-rate-limiting"));
                git(
                    &path,
                    &[
                        "worktree",
                        "add",
                        "-b",
                        "rate-limiting",
                        &checkout.to_string_lossy(),
                        &at,
                    ],
                );
                checkout
            }
        };

        checkouts.push(store::CompanionWorktree {
            repo_id: registered.id,
            path: checkout,
            base_commit: Some(at),
        });
    }

    store::start_grilling(&pool, id, &commit, &worktree, &checkouts)
        .await
        .unwrap();

    let conversation = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    let skills = Skills::installed(state.path()).expect("this binary carries skills");
    let handoffs = Handoffs::under(state.path());
    let settings = Settings::in_data_dir(state.path());

    // Not under `bin/`, which is where this platform links what a session finds
    // — the image is the server's own file wherever it happens to be, and a
    // fixture that stood it where the link goes would be proving nothing about
    // the link.
    let image = state.path().join("image/verkstead");
    std::fs::create_dir_all(image.parent().unwrap()).unwrap();
    std::fs::write(&image, SAYS_WHICH_BUILD).unwrap();
    std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o755)).unwrap();
    let verkstead = Executable::at(image, state.path()).expect("the executable was just written");

    Grilling {
        watched,
        state,
        home,
        repo,
        sibling,
        conversation,
        profile,
        pool,
        skills,
        verkstead,
        handoffs,
        settings,
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

    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// What the probe says about one path: `write`, `read`, `refused` or `absent`.
///
/// The three that are not `absent` are attempted rather than asked of the
/// metadata, for the reason the Linux probe attempts them: a read-only rule and
/// a directory somebody has no write permission on look identical to `test -w`,
/// and only one of them is the surface being described. And `absent` is asked
/// of the metadata, which on this platform a session may always read — so a
/// path that is really there and wholly denied comes back `refused` rather than
/// `absent`, and the difference is the whole of what makes this boundary
/// Apple's.
const PROBE: &str = r#"
say() { printf '%s=%s\n' "$1" "$2"; }

dir() {
    if [ ! -d "$1" ]; then say "$2" absent; return; fi
    if (exec 3> "$1/.verkstead-probe") 2>/dev/null; then
        rm -f "$1/.verkstead-probe"
        say "$2" write
    elif ls "$1" >/dev/null 2>&1; then
        say "$2" read
    else
        say "$2" refused
    fi
}

file() {
    if [ ! -f "$1" ]; then say "$2" absent; return; fi
    # Opening for append is the write, and it changes not a byte of what is
    # there. In a subshell, because a redirection that fails takes the shell
    # attempting it down with it.
    if (exec 3>> "$1") 2>/dev/null; then
        say "$2" write
    elif cat "$1" >/dev/null 2>&1; then
        say "$2" read
    else
        say "$2" refused
    fi
}
"#;

/// Run `script` inside `sandbox` and read back what it reported.
fn probe(sandbox: &Sandbox, script: &str) -> BTreeMap<String, String> {
    let whole = format!("{PROBE}\n{script}\n");

    let output = sandbox
        .command(&[SH, "-c", &whole])
        .stdin(Stdio::null())
        .output()
        .expect("sandbox-exec is part of macOS");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the probe failed inside the sandbox: {stderr}"
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// Where a program a session could run is on this host, absolute.
///
/// One tool the probe needs is looked for rather than named, because a Mac
/// keeps more than one `git` and which of them a machine has is its own
/// business: Homebrew's where somebody installed it, and Apple's under
/// `/usr/bin`, which is a shim into whichever developer tools are selected.
///
/// Looked for where a **session** would look rather than on the test's own
/// `PATH`, and that is the point of the list: the two are different, and a
/// probe running a copy the policy does not reach would be a test failing over
/// where this machine keeps its tools. So this is the reachable part of a
/// session's own `PATH`, in its order — Homebrew's prefix, the system, and
/// nix-darwin's — with `/usr/local` left out of it, which is on that `PATH`
/// and in none of the system list behind it.
fn on_the_host(program: &str) -> PathBuf {
    [
        "/opt/homebrew/bin",
        "/usr/bin",
        "/bin",
        "/run/current-system/sw/bin",
    ]
    .into_iter()
    .map(|dir| Path::new(dir).join(program))
    .find(|path| path.is_file())
    .unwrap_or_else(|| panic!("{program} should be somewhere a session could run it"))
}

/// A path as the probe's shell will read it: one word, whatever the directory
/// somebody's temporary files are under is called.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display())
}

/// And what a path really is, which is what a session inside sees.
///
/// A Mac is made of symlinks that matter — a temporary directory is under
/// `/private/var/folders` however it was handed over — so a test comparing what
/// the probe printed against a path this process is holding has to compare the
/// resolved ones.
fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).expect("the fixture's directories are all there")
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_worktree_and_the_git_directory_are_what_can_be_written() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {worktree} worktree
            dir {git_dir} git-dir
            "#,
            worktree = quoted(fixture.worktree()),
            git_dir = quoted(&fixture.git_dir()),
        ),
    );

    assert_eq!(
        reported["worktree"], "write",
        "the Worktree is where the work is done"
    );
    assert_eq!(
        reported["git-dir"], "write",
        "and the git directory behind it is what a commit writes to"
    );
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_system_comes_in_read_only() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        r#"
        dir /usr/bin usr-bin
        file /bin/sh sh
        dir /usr/local no-writing-in-the-system
        "#,
    );

    assert_eq!(
        reported["usr-bin"], "read",
        "the tools a session runs are the machine's own"
    );
    assert_eq!(
        reported["sh"], "read",
        "and so is the shell it runs them from"
    );
    assert_ne!(
        reported["no-writing-in-the-system"], "write",
        "a session may run what the machine has and may not change it"
    );
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn no_other_checkout_on_the_machine_is_reachable() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {sibling} sibling
            dir {watched} watched-path
            file {readme} repo-readme
            file {gitconfig} host-gitconfig
            file {key} host-key
            "#,
            sibling = quoted(&fixture.sibling),
            watched = quoted(fixture.watched.path()),
            readme = quoted(&fixture.repo.join("README.md")),
            gitconfig = quoted(&fixture.home_path().join(".gitconfig")),
            key = quoted(&fixture.home_path().join(".ssh/id_ed25519")),
        ),
    );

    assert_eq!(
        reported["sibling"], "refused",
        "another repository under the same Watched Path is another \
         Conversation's business"
    );
    assert_eq!(
        reported["repo-readme"], "refused",
        "the checkout the worktree was made from is not the worktree"
    );
    assert_eq!(
        reported["host-gitconfig"], "refused",
        "who a session commits as is configured rather than found lying about \
         in a home directory"
    );
    assert_eq!(
        reported["host-key"], "refused",
        "and the machine's keys are nobody inside's"
    );
    assert_eq!(
        reported["watched-path"], "refused",
        "a Watched Path is where Verkstead may operate, not where a session may"
    );
}

/// And Verkstead's own Data Directory least of all: it holds every
/// Conversation's record and the settings files the human's credentials are in.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn verksteads_own_state_is_no_sessions_business() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {data} data-dir
            file {database} database
            "#,
            data = quoted(fixture.state.path()),
            database = quoted(&fixture.state.path().join("verkstead.db")),
        ),
    );

    assert_eq!(reported["data-dir"], "refused");
    assert_eq!(
        reported["database"], "refused",
        "the record of every Conversation is the server's and nobody else's"
    );
}

/// The difference between this boundary and bubblewrap's, proved rather than
/// argued (ADR-0012).
///
/// A session on a Mac can see that the machine has a home directory full of
/// somebody's work, and cannot read a byte of it. The Linux suite's own version
/// of this test asserts `absent` for the same paths, and both are right about
/// their own platform: what the two mechanisms share is what is *reachable*,
/// which is what the description says and the whole of what it says.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_machine_is_refused_rather_than_hidden() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            if [ -d {sibling} ]; then say seen yes; else say seen no; fi
            dir {sibling} sibling
            "#,
            sibling = quoted(&fixture.sibling),
        ),
    );

    assert_eq!(
        reported["seen"], "yes",
        "Apple's sandbox denies rather than hides, and a test that read \
         `absent` here would be reading a path that had gone rather than one \
         that is refused"
    );
    assert_eq!(reported["sibling"], "refused");
}

/// The data volume is firmlinked in under `/System`, so every home directory on
/// the machine has a second name — which is why the system list holds
/// `/System/Library` rather than `/System`.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_data_volume_is_no_way_round_the_system_being_readable() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    // The sibling's own path, under the volume it is really on: what a session
    // would reach for if `/System` were readable whole.
    let round_the_back =
        Path::new("/System/Volumes/Data").join(real(&fixture.sibling).strip_prefix("/").unwrap());

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {round_the_back} round-the-back
            "#,
            round_the_back = quoted(&round_the_back),
        ),
    );

    assert!(
        reported["round-the-back"] == "refused" || reported["round-the-back"] == "absent",
        "a checkout no session may read is one it may not read by its other \
         name either, got {}",
        reported["round-the-back"],
    );
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_network_is_the_hosts_own() {
    use std::io::Write;
    use std::net::TcpListener;

    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    // A listener in this process. Nothing here touches the internet: what is
    // being shown is that the loopback inside is the machine's, which is what
    // `--share-net` leaves on Linux and what a policy with `network*` in it
    // leaves here.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let answering = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the probe connects");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
            .unwrap();
    });

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            if {CURL} --silent --max-time 10 --output /dev/null "http://127.0.0.1:{port}/"; then
                say network reached
            else
                say network unreachable
            fi
            "#,
        ),
    );

    assert_eq!(
        reported["network"], "reached",
        "the filesystem is the boundary; the network is not"
    );

    answering.join().unwrap();
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn a_session_starts_in_its_worktree_with_nothing_of_the_servers_environment() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        r#"
        say cwd "$(pwd)"
        say server "${VERKSTEAD_SERVER-unset}"
        say cargo "${CARGO_MANIFEST_DIR-unset}"
        "#,
    );

    assert_eq!(
        Path::new(&reported["cwd"]),
        real(fixture.worktree()),
        "a session starts where the work is"
    );
    assert!(
        reported["server"].starts_with("http://127.0.0.1:8422/"),
        "and knows where to put a Question Set, got {}",
        reported["server"],
    );
    assert_eq!(
        reported["cargo"], "unset",
        "and carries nothing of whatever started the server: the environment \
         inside is the description's, whole"
    );
}

/// The Profile's account is where its backend looks for it, inside a HOME that
/// is the session's own and holds nothing else.
///
/// The Linux suite's claim word for word, and a different mechanism under it:
/// there, HOME is an empty filesystem with the account mounted into it; here it
/// is a real directory of Verkstead's own with the account linked into it, and
/// what keeps the machine's own home out of it is the policy rather than the
/// directory never having been there.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_profiles_pair_is_the_whole_of_what_home_holds() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        r#"
        dir "$HOME/.claude" claude-dir
        file "$HOME/.claude/settings.json" settings
        file "$HOME/.claude.json" claude-config
        file "$HOME/.ssh/id_ed25519" private-key
        say home "$(ls -A "$HOME" | sort | tr '\n' ' ')"
        "#,
    );

    assert_eq!(
        reported["claude-dir"], "write",
        "a session writes its own session logs and settings"
    );
    assert_eq!(
        reported["settings"], "write",
        "and what the account already kept there is there to be read"
    );
    assert_eq!(reported["claude-config"], "write");
    assert_eq!(
        reported["private-key"], "absent",
        "a session's HOME is its own, so the machine's is not even the same \
         directory to be refused in"
    );
    assert_eq!(
        reported["home"], ".claude .claude.json ",
        "and nothing else is in it"
    );
}

/// And for each of the three backends whose whole account is one home or two,
/// that account is where *its* backend looks and nothing of Claude's is beside
/// it.
///
/// Two accounts of different shapes cannot both be right about `~`, so what is
/// linked in is the account's own type's and only that.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn a_home_account_is_where_its_backend_looks_for_it() {
    let fixture = grilling().await;

    for (profile, held) in [
        (fixture.codex_profile().await, ".codex "),
        (fixture.grok_profile().await, ".grok "),
        (fixture.opencode_profile().await, ".config .local "),
    ] {
        let sandbox = fixture.sandbox_under(&profile);

        let reported = probe(
            &sandbox,
            r#"
            dir "$HOME/.codex" codex-home
            file "$HOME/.codex/config.toml" codex-config
            dir "$HOME/.grok" grok-home
            dir "$HOME/.config/opencode" opencode-config
            file "$HOME/.local/share/opencode/auth.json" opencode-auth
            dir "$HOME/.claude" claude-dir
            say home "$(ls -A "$HOME" | sort | tr '\n' ' ')"
            "#,
        );

        assert_eq!(
            reported["home"], held,
            "the account this session runs under is the whole of what HOME holds"
        );
        assert_eq!(
            reported["claude-dir"], "absent",
            "and no account of another shape is in it"
        );

        match held {
            ".codex " => {
                assert_eq!(reported["codex-home"], "write");
                assert_eq!(
                    reported["codex-config"], "write",
                    "with what the account already kept there"
                );
            }
            ".grok " => assert_eq!(reported["grok-home"], "write"),
            _ => {
                assert_eq!(reported["opencode-config"], "write");
                assert_eq!(
                    reported["opencode-auth"], "write",
                    "which is the half of an opencode account that is the login"
                );
            }
        }
    }
}

/// And one account's sessions cannot reach another's.
///
/// The thing the mount does for nothing on Linux, where an account nothing
/// bound in was never in the namespace at all. Here both accounts are real
/// directories on the machine and the session is running under one of them:
/// what makes the other one nobody's business is the policy, and this is the
/// probe that says so.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn one_accounts_sessions_cannot_reach_anothers() {
    let fixture = grilling().await;
    let other = fixture.codex_profile().await;
    let sandbox = fixture.sandbox();

    let store::Account::Codex { home: theirs } = &other.account else {
        panic!("a Codex Profile's account is one home");
    };

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {theirs} the-other-account
            file {config} the-other-config
            "#,
            theirs = quoted(theirs),
            config = quoted(&theirs.join("config.toml")),
        ),
    );

    assert_eq!(
        reported["the-other-account"], "refused",
        "a session runs as one account, and the rest are somebody else's"
    );
    assert_eq!(reported["the-other-config"], "refused");
}

/// The skills a session reads are the ones this binary ships, and only those.
///
/// At a path of Verkstead's own on both platforms, and the same path only on
/// one of them: nothing can be created at `/` on a Mac, so what a session is
/// told to read is the directory they were really written into — which is the
/// one part of the Data Directory a session may reach, and read-only at that.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_skills_inside_are_the_bundled_ones_and_only_those() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let inside = fixture.skills.inside().to_owned();

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            file {grilling} grilling
            say installed "$(ls {inside} | sort | tr '\n' ' ')"
            dir {data} data-dir
            file "$HOME/.claude/CLAUDE.md" claude-md
            dir "$HOME/.claude/skills" the-accounts-own
            "#,
            grilling = quoted(&inside.join("grilling/SKILL.md")),
            inside = quoted(&inside),
            data = quoted(fixture.state.path()),
        ),
    );

    assert_eq!(
        reported["grilling"], "read",
        "the bundled grilling skill is installed, and is not a session's to rewrite"
    );
    assert_eq!(
        reported["installed"],
        installed_on_the_host(&fixture.skills),
        "and the whole of what this binary ships is there"
    );
    assert_eq!(
        reported["data-dir"], "refused",
        "while the directory holding them is otherwise no session's business"
    );
    assert_eq!(
        reported["claude-md"], "absent",
        "there is no global CLAUDE.md in here to say how to reach the human"
    );
    assert_eq!(
        reported["the-accounts-own"], "absent",
        "and what the account keeps under its own skills is refused so far \
         that there is nothing there to look at — which the fixture put a skill \
         in to be able to say"
    );

    assert!(
        fixture
            .watched
            .path()
            .join("account/.claude/skills/the-accounts-own/SKILL.md")
            .is_file(),
        "while nothing of the account's own was written over to do it",
    );
}

/// The skill names as they sit on the host, in the shape the probe reports them
/// from inside — so the assertion above is "the installed set, whatever it is
/// this release" rather than a list to be edited every time a skill lands.
fn installed_on_the_host(skills: &Skills) -> String {
    let mut names: Vec<String> = std::fs::read_dir(skills.path())
        .expect("the skills are installed")
        .map(|entry| entry.expect("a readable entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();

    names.sort();

    names.iter().map(|name| format!("{name} ")).collect()
}

/// `verkstead` inside is the executable serving the session, and it is what a
/// bare `verkstead` finds.
///
/// The two halves of an ask are the CLI a session runs and the server it puts a
/// Set to, and they have to be one build. So the server hands over its own
/// image — linked into a directory holding nothing else, ahead of every path
/// the machine could have installed a `verkstead` on.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_verkstead_a_session_asks_with_is_the_one_serving_it() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        r#"
        found=$(command -v verkstead)
        say found "$found"
        say version "$(verkstead)"
        say beside "$(ls "$(dirname "$found")")"
        say first "${PATH%%:*}"
        file "$found" binary
        "#,
    );

    assert_eq!(
        reported["version"], "verkstead 0.0.0-the-servers-own",
        "running it runs the server's own build, not whatever the machine has"
    );
    assert_eq!(
        reported["beside"], "verkstead",
        "the directory holds the one executable the server put there and nothing else"
    );
    assert_eq!(
        Path::new(&reported["first"]),
        fixture.state.path().join("bin"),
        "which is looked in before every path an install could have landed on"
    );
    assert_eq!(
        reported["found"],
        format!("{}/verkstead", reported["first"]),
        "and a bare `verkstead` is what is in it"
    );
    assert_eq!(
        reported["binary"], "read",
        "and it is no more a session's to rewrite mid-run than the skills are"
    );
}

/// A session runs the machine's own tools, whichever of them this Mac has.
///
/// Homebrew's and the system's are what a Mac nobody has been to any trouble
/// over holds; nix's are what one running nix-darwin has and nothing else does.
/// Both are on the `PATH` and both are reachable, and the entries for whichever
/// of them is not installed resolve to nothing without costing a session
/// anything — which is what "neither is made to do without the other" comes to
/// from inside. What holds the two lists to each other is a unit test beside
/// the lists themselves.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn a_session_runs_the_tools_this_mac_has_and_is_not_stopped_by_the_ones_it_has_not() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        r#"
        say path "$PATH"
        say shell "$SHELL"
        say git "$(command -v git || echo none)"
        say sed "$(command -v sed || echo none)"
        "#,
    );

    for looked_in in [
        "/opt/homebrew/bin",
        "/usr/bin",
        "/run/current-system/sw/bin",
    ] {
        assert!(
            reported["path"].split(':').any(|entry| entry == looked_in),
            "{looked_in} is where one kind of Mac keeps its tools, and this \
             session would never look there: {}",
            reported["path"],
        );
    }

    assert_eq!(
        reported["shell"], "/bin/sh",
        "and the shell a tool shells out to is one this platform really has"
    );
    assert_ne!(
        reported["git"], "none",
        "a session that cannot find git cannot commit, whatever else is on the PATH"
    );
    assert_ne!(reported["sed"], "none");
}

/// What a session writes in its handoff directory is there when it has gone.
///
/// The directory is a link on this platform rather than a mount, and it is the
/// one thing inside a sandbox that outlives the session: a grilling writes the
/// handoff there and the Conversation reads it afterwards, so a link to the
/// wrong place would be a handoff nothing ever finds.
///
/// **Under HOME rather than at `/tmp/verkstead`**, which is the whole of what
/// this platform changes about it. There is one real `/tmp` on a Mac and every
/// Conversation would be sharing a link written in it — the second session to
/// start repointing it, and the first then writing its handoff into a directory
/// its own policy refuses. A HOME is already one directory per Conversation, so
/// the handoff goes under it and the skills are installed saying so. Written
/// here the way a session reads it: `$HOME`, expanded by the shell inside out of
/// the environment the sandbox handed it.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn what_a_session_writes_in_its_handoff_directory_is_there_when_it_has_gone() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox();

    let reported = probe(
        &sandbox,
        r#"
        printf '# What we settled\n' > "$HOME/verkstead/handoff.md"
        dir "$HOME/verkstead" handoff
        "#,
    );

    assert_eq!(reported["handoff"], "write");

    let written = fixture
        .handoffs
        .directory(fixture.conversation.id)
        .expect("the directory is made as the sandbox is built")
        .join("handoff.md");

    assert_eq!(
        std::fs::read_to_string(written).expect("the handoff is outside the sandbox"),
        "# What we settled\n",
        "which is the whole point of the directory being Verkstead's own",
    );
}

/// And the skills say that path rather than the mount, so a session told where
/// to write its handoff is told somewhere it can write.
///
/// The prompt and the skill text are two halves of one instruction — see the
/// server's `skills::said_at` — and this is the half that is a file on disk. Run
/// on whatever machine the suite is on: what it reads is what this build
/// installed, and where that is, is decided by the platform rather than probed.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the skills this reads are installed as a Mac's"
)]
async fn the_grilling_skill_names_the_handoff_where_a_session_would_find_it() {
    let fixture = grilling().await;

    let grilling_skill =
        std::fs::read_to_string(fixture.skills.path().join("grilling/SKILL.md")).unwrap();

    assert!(
        grilling_skill.contains("$HOME/verkstead/handoff.md"),
        "the skill has to name a path a session on this platform can open",
    );
    assert!(
        !grilling_skill.contains("/tmp/verkstead"),
        "and not the one every Conversation would be sharing",
    );
}

/// What Sandbox Configuration asks for is inside and writable, composed the way
/// it is on Linux: the global set reaches every session, a named one reaches
/// only sessions working in the Repo registered under that name.
///
/// And the one this boundary says differently: another Repo's bind is a
/// directory that really is on the machine and is simply not in the policy, so
/// a session finds it refused rather than absent.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_extra_binds_sandbox_configuration_asks_for_are_there_and_writable() {
    let fixture = grilling().await;

    // A global one and this Repo's own, read off the configuration as the
    // orchestrator will read them: everything gets the first, and the Repo
    // called `verkstead` also gets the second.
    let global = fixture.state.path().join("shared-cache");
    let per_repo = fixture.state.path().join("verkstead-cargo-home");
    std::fs::create_dir_all(&global).unwrap();
    std::fs::create_dir_all(&per_repo).unwrap();

    let config = SandboxConfig::resolve(&[
        global.display().to_string(),
        format!("verkstead={}", per_repo.display()),
        // Another Repo's, which this one is no part of.
        format!("something-else={}", fixture.sibling.display()),
    ])
    .unwrap();

    let reported = probe(
        &fixture.sandbox_with(config.binds_for(&fixture.conversation)),
        &format!(
            r#"
            dir {global} global
            dir {per_repo} per-repo
            dir {other} another-repos
            "#,
            global = quoted(&global),
            per_repo = quoted(&per_repo),
            other = quoted(&fixture.sibling),
        ),
    );

    assert_eq!(
        reported["global"], "write",
        "what the machine gives every session"
    );
    assert_eq!(
        reported["per-repo"], "write",
        "and what this repository asked for on top of it"
    );
    assert_eq!(
        reported["another-repos"], "refused",
        "a bind configured for another Repo is that Repo's — and on this \
         platform it is a directory a session can see and cannot open"
    );
}

/// The same configuration said in `config.yaml` instead: its entries are inside
/// and writable, they are added to what the installation configured rather than
/// standing in for it, and one naming a directory that is not there is skipped
/// with the session starting anyway.
///
/// Which is the whole of what a standalone install needs — a bare binary whose
/// sandbox is set up from the settings page, with no flag and no environment
/// variable in front of it.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_binds_the_settings_file_holds_are_inside_and_writable_too() {
    let fixture = grilling().await;

    let installed = fixture.state.path().join("the-installations-cache");
    let global = fixture.state.path().join("the-settings-cache");
    let per_repo = fixture.state.path().join("the-settings-verkstead-cargo");
    for made in [&installed, &global, &per_repo] {
        std::fs::create_dir_all(made).unwrap();
    }

    // And one nobody made, which is the opposite answer to the flag's: a file
    // edited from a phone is a file with typos in it, and a Conversation that
    // would not start because one of its binds was misspelled is a worse
    // failure than a build running without its cache.
    let missing = fixture.state.path().join("never-made");

    fixture.configure(&format!(
        "sandbox_binds:\n  - {global}\n  - verkstead={per_repo}\n  - {missing}\n  \
         - something-else={other}\n",
        global = global.display(),
        per_repo = per_repo.display(),
        missing = missing.display(),
        other = fixture.sibling.display(),
    ));

    // And one on the command line beside them, which is the composition worth
    // asking about: neither set is the other's replacement.
    let installation = SandboxConfig::resolve(&[installed.display().to_string()]).unwrap();

    let reported = probe(
        &fixture.sandbox_with(installation.binds_for(&fixture.conversation)),
        &format!(
            r#"
            dir {installed} installed
            dir {global} global
            dir {per_repo} per-repo
            dir {missing} missing
            dir {other} another-repos
            "#,
            installed = quoted(&installed),
            global = quoted(&global),
            per_repo = quoted(&per_repo),
            missing = quoted(&missing),
            other = quoted(&fixture.sibling),
        ),
    );

    assert_eq!(
        reported["installed"], "write",
        "what the installation configured is still there"
    );
    assert_eq!(
        reported["global"], "write",
        "and what the settings file adds to it"
    );
    assert_eq!(
        reported["per-repo"], "write",
        "including what it adds for this Repo by name"
    );
    assert_eq!(
        reported["missing"], "absent",
        "the entry that would not resolve is skipped, and the session starts \
         without it rather than not starting"
    );
    assert_eq!(
        reported["another-repos"], "refused",
        "and what it adds for a Repo this Conversation has nothing to do with \
         is no more reachable than the rest of the machine"
    );
}

/// A read-only companion is there to be read: the checkout and the history
/// behind it, and nothing a session does inside it changes either.
///
/// Both halves are asked of git rather than of the policy. A checkout made
/// readable with a writable git directory would still take a commit, and one
/// whose git directory is read-only refuses more than committing — the index
/// and its lock live in there too, so what has to be shown is that reading
/// really does work rather than that writing really does not.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn a_read_only_companion_is_there_to_read_and_a_commit_from_it_is_refused() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadOnly)]).await;
    fixture.configure("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

    let companion = fixture.companion_worktree("askance").to_owned();

    let reported = probe(
        &fixture.sandbox(),
        &format!(
            r#"
            dir {companion} worktree
            dir {git_dir} git-dir
            say readme "$({CAT} {companion}/README.md)"
            say history "$({git} -C {companion} log --oneline | wc -l | tr -d ' ')"

            if {git} -C {companion} status --porcelain >/dev/null 2>&1; then
                say status yes
            else
                say status no
            fi

            if {git} -C {companion} commit --quiet --allow-empty -m 'from inside' 2>/dev/null; then
                say committed yes
            else
                say committed no
            fi

            if {git} -C {companion} branch verkstead-probe 2>/dev/null; then
                say branched yes
            else
                say branched no
            fi
            "#,
            companion = quoted(&companion),
            git_dir = quoted(&fixture.companion("askance").repo.path.join(".git")),
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(
        reported["worktree"], "read",
        "a read-only companion is a checkout to read and leave alone"
    );
    assert_eq!(
        reported["git-dir"], "read",
        "and the object database behind it, which is what makes it a repository at all"
    );
    assert_eq!(
        reported["readme"], "# a repository",
        "reading a file in one really works"
    );
    assert_eq!(
        reported["history"], "1",
        "and so does asking git what the history is, which is the half worth \
         proving rather than assuming"
    );
    assert_eq!(
        reported["status"], "yes",
        "and so does the question an agent asks a checkout first of all, which \
         git answers without writing the index it would rather refresh"
    );
    assert_eq!(
        reported["committed"], "no",
        "there is nowhere for a commit to be written"
    );
    assert_eq!(
        reported["branched"], "no",
        "and no ref can be moved, which is what the last step of a push is"
    );
}

/// A read-write companion is somewhere the work is done: a commit lands on the
/// branch that was cut for it, and it is there when the session has gone.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn a_read_write_companion_takes_a_commit_on_its_own_branch() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadWrite)]).await;
    fixture.configure("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

    let companion = fixture.companion_worktree("askance").to_owned();

    let reported = probe(
        &fixture.sandbox(),
        &format!(
            r#"
            printf 'from inside\n' > {companion}/NOTES.md
            {git} -C {companion} add NOTES.md

            if {git} -C {companion} commit --quiet -m 'from inside' 2>/tmp/git-said; then
                say committed yes
            else
                say committed "no: $({CAT} /tmp/git-said)"
            fi
            "#,
            companion = quoted(&companion),
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(reported["committed"], "yes");

    // Read outside, where the branch the checkout was cut on is: what a session
    // committed in a companion is in that companion's repository, on the branch
    // the Conversation's own name was mirrored into.
    assert_eq!(
        git(&companion, &["log", "-1", "--format=%s"]).trim(),
        "from inside"
    );
    assert_eq!(
        git(&companion, &["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
        "rate-limiting",
        "a read-write companion is cut a branch of its own, mirroring the \
         Conversation's where nothing else was typed"
    );
}

/// What a companion repo's own Sandbox Configuration asks for is inside and
/// writable, whatever the companion's mode.
///
/// A configured bind is a hole somebody opened on purpose, and it sits outside
/// the repository: a read-only companion is a checkout not to be changed rather
/// than a repository whose builds should fail on a cold cache.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn a_read_only_companions_own_configured_binds_are_still_writable() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadOnly)]).await;

    let cache = fixture.state.path().join("askance-node-modules");
    std::fs::create_dir_all(&cache).unwrap();

    let config = SandboxConfig::resolve(&[format!("askance={}", cache.display())]).unwrap();

    let reported = probe(
        &fixture.sandbox_with(config.binds_for(&fixture.conversation)),
        &format!(
            r#"
            dir {cache} cache
            dir {companion} worktree
            "#,
            cache = quoted(&cache),
            companion = quoted(fixture.companion_worktree("askance")),
        ),
    );

    assert_eq!(
        reported["cache"], "write",
        "a companion's builds need its caches like any other repository's"
    );
    assert_eq!(
        reported["worktree"], "read",
        "and the checkout beside it is still only there to be read"
    );
}

/// The shared Rust build cache, with nothing configured — which is the feature
/// on, because a human who has never opened the settings page should not be the
/// one paying for every dependency to be compiled twice.
///
/// The directory is writable at the same path inside, and `CARGO_HOME` points
/// into it: that is the half of the cache that works with no sccache anywhere,
/// and it is what stops two Conversations downloading one crate twice.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_build_cache_is_writable_inside_and_cargos_home_is_in_it() {
    let fixture = grilling().await;
    let cache = fixture.cache(false);

    let reported = probe(
        &fixture.sandbox_caching(&cache),
        &format!(
            r#"
            dir {dir} cache
            say cargo-home "${{CARGO_HOME-unset}}"
            say wrapper "${{RUSTC_WRAPPER-unset}}"
            say sccache-dir "${{SCCACHE_DIR-unset}}"
            "#,
            dir = quoted(&fixture.cache_dir()),
        ),
    );

    assert_eq!(
        reported["cache"], "write",
        "a cache a session cannot write to is no cache"
    );
    assert_eq!(
        reported["cargo-home"],
        fixture.cache_dir().join("cargo").display().to_string(),
        "the registry every session downloads into is the one inside the policy"
    );
    assert_eq!(
        reported["wrapper"], "unset",
        "with no sccache resolved there is nothing to wrap rustc in, and a \
         RUSTC_WRAPPER naming a path that is not reachable would break every build"
    );
    assert_eq!(reported["sccache-dir"], "unset");
}

/// And with an sccache the server resolved: a session finds it beside the
/// `verkstead` binary, it is *that* file rather than whatever the machine has
/// installed, and it is what `RUSTC_WRAPPER` names.
///
/// Which is what makes a Rust build inside a session compile through the one
/// sccache this machine has: the wrapper every `rustc` goes through is the
/// server's own file, and what it hands the command line to is the compile
/// server below.
///
/// The wrapper is named absolutely on purpose. A session's command may be
/// wrapped in `nix develop`, which puts the project's own shell in front of the
/// sandbox's `PATH` — so a bare `sccache` would resolve to whatever that shell
/// had, or to nothing.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_sccache_the_server_resolved_is_what_rustc_is_wrapped_in() {
    let fixture = grilling().await;
    let cache = fixture.cache(true);

    let reported = probe(
        &fixture.sandbox_caching(&cache),
        &format!(
            r#"
            say wrapper "${{RUSTC_WRAPPER-unset}}"
            say sccache-dir "${{SCCACHE_DIR-unset}}"
            say size "${{SCCACHE_CACHE_SIZE-unset}}"
            say which "$("${{RUSTC_WRAPPER}}")"
            dir {dir} cache
            "#,
            dir = quoted(&fixture.cache_dir()),
        ),
    );

    assert_eq!(
        reported["wrapper"],
        fixture
            .state
            .path()
            .join("bin/sccache")
            .display()
            .to_string(),
        "beside the binary a session asks with, in the directory of Verkstead's \
         own that leads the PATH — really there on this platform, because \
         nothing here can make a directory out of nothing"
    );
    assert_eq!(
        reported["which"], "sccache 0.0.0-the-one-resolved",
        "and what a session compiles through is the binary the server resolved, \
         not whatever this Mac has installed"
    );
    assert_eq!(
        reported["sccache-dir"],
        fixture.cache_dir().join("sccache").display().to_string(),
        "which writes its objects inside the one directory, beside cargo's own"
    );
    assert_eq!(
        reported["size"], "30G",
        "the default where the human has configured no size"
    );
    assert_eq!(reported["cache"], "write");
}

/// The compile server: Verkstead's own, in a sandbox holding the Worktrees
/// directory and the cache and nothing else Verkstead keeps.
///
/// This is what stops two Conversations building Rust at once from breaking
/// each other. An sccache server is what executes `rustc` — the client in a
/// session only hands it a command line — and every sandbox shares the host's
/// network, so clients left to start their own all reach for one port and the
/// session that lost the race has its compiles run inside the winner's sandbox,
/// where its Worktree is not reachable and the build fails outright.
///
/// So the server has to see **every** Worktree, which is why it gets the
/// directory rather than any one of them — a Conversation grilled after it
/// started is one it can already compile for. And it must not see the rest of
/// the Data Directory: `rustc` runs proc macros while it compiles, so a server
/// with the database and the settings files in reach would be every Rust
/// dependency on the machine holding the GitHub token.
///
/// Its sandbox is composed by the code a session's is and rendered by the same
/// renderer, so this is the same claim the Linux suite makes, asked of the
/// other mechanism — and answered in this one's own word: what it may not reach
/// is refused rather than absent.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_compile_server_holds_the_worktrees_and_none_of_the_data_directory() {
    let fixture = grilling().await;

    // The four things it must not reach, really on disk so that their being
    // refused inside is the policy rather than the fixture.
    fixture.configure("git_author:\n  name: Tobias Cohen\n");
    fixture.configure_github_token("github_token: ghp_thetoken\n");
    std::fs::write(fixture.state.path().join("verkstead.db"), "the database\n").unwrap();
    std::fs::create_dir_all(fixture.state.path().join("handoffs")).unwrap();

    let cache = fixture.cache(true);
    cache.compiling(&RustBuildCache::default());

    let reported = compile_server_report(&fixture);

    assert_eq!(
        reported["worktrees"], "write",
        "every Conversation's checkout, writable, because a compile writes its \
         output into the Worktree's own target/"
    );
    assert_eq!(
        reported["cache"], "write",
        "and the cache it reads its dependency sources out of and writes its \
         objects into"
    );

    assert_eq!(
        reported["database"], "refused",
        "the database is not the compile server's, and a proc macro compiles as \
         whoever Verkstead runs as"
    );
    assert_eq!(reported["config"], "refused");
    assert_eq!(
        reported["secrets"], "refused",
        "least of all the file the GitHub token is in"
    );
    assert_eq!(
        reported["handoffs"], "refused",
        "and nothing else of the Data Directory either — the Worktrees are the \
         whole of what it is shown"
    );

    assert_eq!(
        reported["size"], "30G",
        "started with the size the human left, which sccache reads once"
    );
    assert_eq!(
        reported["no-daemon"], "1",
        "in the foreground, so it is a child Verkstead holds rather than a \
         daemon nothing can ask about"
    );
    assert_eq!(
        reported["idle"], "0",
        "and it does not time out: an unattended Conversation may go a long \
         while between builds"
    );
    assert_eq!(
        reported["sccache-dir"],
        fixture.cache_dir().join("sccache").display().to_string(),
        "writing into the same objects a session's own fallback server would"
    );
    assert_eq!(
        reported["home"],
        fixture.state.path().join("home").display().to_string(),
        "with a HOME of Verkstead's own for the config file sccache looks for \
         there — a real directory here, where a bind made one on Linux"
    );
}

/// And how long it lives, which on this platform is nothing the sandbox itself
/// can say.
///
/// `--die-with-parent` is bubblewrap's and Apple's has no equivalent, so a
/// compile server on a Mac would otherwise be an ordinary child left running
/// with the machine's build cache open when Verkstead has gone. What answers
/// that is a process group of its own and a keeper watching it — see the
/// server's `sandbox::outliving`, whose own tests are what prove that a keeper
/// ends the group. What is asked here is the half only this platform can
/// answer: that a compile server started by the real code path gets both.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this probes is a Mac's"
)]
async fn the_compile_server_is_kept_from_outliving_verkstead() {
    let fixture = grilling().await;

    let cache = fixture.cache(true);
    cache.compiling(&RustBuildCache::default());

    let compiling = compile_server_report(&fixture)["pid"].clone();

    assert_eq!(
        group_of(&compiling),
        compiling,
        "a group of its own, so that ending it ends what it started rather \
         than whatever else Verkstead is in a group with",
    );
    assert!(
        keeper_watching(&compiling),
        "and a keeper of Verkstead's own watching that group, which is what \
         the platform has instead of a flag",
    );
}

/// What process group `pid` is in, off the machine's own process table.
fn group_of(pid: &str) -> String {
    let output = Command::new(PS)
        .args(["-o", "pgid=", "-p", pid])
        .output()
        .expect("ps is part of macOS");

    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Whether a keeper of Verkstead's is watching the group `pid` leads, waited
/// for the way the report above is waited for.
///
/// Read off the process table rather than out of anything Verkstead holds,
/// because a keeper is deliberately nobody's by the time it matters: it is in a
/// session of its own with no parent, and the process table is the only place
/// left that knows it is there. Which is also why it says what it is in its own
/// first line — see the server's `sandbox::outliving`.
fn keeper_watching(pid: &str) -> bool {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);

    while std::time::Instant::now() < until {
        let output = Command::new(PS)
            .args(["-axww", "-o", "command="])
            .output()
            .expect("ps is part of macOS");

        if String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.contains("verkstead") && line.contains(pid))
        {
            return true;
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    false
}

/// What the compile server wrote down about its own sandbox, waited for.
///
/// Waited rather than read, because starting it is a `spawn` and what is being
/// read is a file the child writes: the alternative is a test that passes or
/// fails on how busy the machine was.
fn compile_server_report(fixture: &Grilling) -> BTreeMap<String, String> {
    let report = fixture.cache_dir().join(COMPILE_SERVER_REPORT);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(20);

    while std::time::Instant::now() < until {
        // Written whole and then read, so a report caught half-written is one
        // more go round rather than a missing key: the last line is the one
        // every caller asserts on.
        if let Ok(said) = std::fs::read_to_string(&report)
            && said.contains("secrets=")
        {
            return said
                .lines()
                .filter_map(|line| line.split_once('='))
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect();
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    panic!(
        "the compile server never wrote its report to {}",
        report.display()
    );
}
