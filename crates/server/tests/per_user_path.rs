//! A harness installed under the server's own home is one a session can really
//! run — asked by running it.
//!
//! Task 01 put the server's own `PATH` in front of a session, in the order the
//! human wrote it. A directory on that `PATH` is worth nothing unless the
//! session can read it, and neither Unix lets one by default: on Linux `~`
//! inside is an empty directory of Verkstead's own and what is not bound is not
//! there at all, and a Mac's policy denies everything it was not told about. So
//! what is asserted here is the grant that closes that gap — a `claude` under
//! `~/.local/bin`, where the vendor's own installer puts one, being a program a
//! session can actually start.
//!
//! **And the install it is a link into with it.** That installer leaves
//! `~/.local/bin/claude` a symlink into `~/.local/share/claude/versions/`, so
//! the directory on the `PATH` holds a link and the program is somewhere no
//! `PATH` names at all. The fixture is that shape rather than a plain file,
//! because a plain file would prove the grant a real machine does not have: what
//! has to run inside is the link *and* what it lands on.
//!
//! **And a directory Verkstead installed into, which is on no `PATH` the server
//! was started with.** `session_path` in `config.yaml` is the whole of what puts
//! one in front of a session — see `tests/session_path.rs`, which is that key's
//! own suite — and a directory told about and not granted is a name a session
//! finds and cannot start. So the second harness here is run by name too, and
//! read the same way round: on the list ahead of the system directories, and
//! read-only inside.
//!
//! **One test per machine, in a binary of its own, and that is the whole design
//! of this file.** The `PATH` and the home a session composes from are the
//! *process's* own — read once at startup, held for the run — so the only way
//! to stand a machine up with a harness installed under the server's home is to
//! move the environment and then ask. `std::env::set_var` is `unsafe` under this
//! edition because it races every other thread in the process reading one, so
//! the two tests below are cfg-ed to run on one platform each: whichever
//! machine this is, exactly one of them runs and there is no second thread to
//! race. Both are compiled on both Unixes all the same — a test nobody compiles
//! is one that rots against the types it is written for, which is the rule
//! `tests/sandbox_macos.rs` is written by.
//!
//! **The two halves ask what their own boundary can answer.** Linux hides what
//! it was not given, so the question is whether the program runs and whether
//! anything else came in with it. A Mac refuses instead, and what refuses is a
//! policy — so that half reads the rules the description came to, the one thing
//! about this platform a machine that is not a Mac can still be shown.
//!
//! Everything else about which directories are granted is a unit test beside
//! the function that decides it, where a `PATH` and a home are values a test
//! hands over: see `sandbox::per_user`.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{Executable, Homes, Reachable, Rendering, Sandbox};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// Where the server this Conversation belongs to is listening.
const LISTENING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8422);

/// What stands in for the server's own image, as in every other sandbox suite.
const SAYS_WHICH_BUILD: &str = "#!/bin/sh\nprintf 'verkstead 0.0.0-the-servers-own\\n'\n";

/// Where the vendor's own installer puts a harness: a link under the home of
/// whoever ran it, and the version it lands on somewhere else under that home.
/// The install this whole feature is about.
const NATIVE_INSTALL: &str = ".local/bin";
const NATIVE_VERSION: &str = ".local/share/claude/versions/0.0.0";

/// And the half of that path a session must *not* be given: the directory the
/// version's own is under, which holds everything else a machine keeps there —
/// and a file of the human's in it, which is how a session is asked whether it
/// got the whole of that directory or the one version it runs.
///
/// Asked as a file rather than as the directory, because the directory is
/// *there* inside on Linux either way: the grant below it is a mount, and a
/// mount has parents. What says whether it is the human's own is whether what
/// they keep in it came with it.
const ABOVE_THE_VERSIONS: &str = ".local/share/claude";
const BESIDE_THE_VERSIONS: &str = ".local/share/claude/what-the-human-keeps-there";

/// And where *Verkstead* installed one, which is on no `PATH` the server was
/// started with: `session_path` in `config.yaml` is the whole of what puts it in
/// front of a session — see `tests/session_path.rs`, which is that key's own
/// suite. Here to be run and read, which is what a list of directories is worth
/// nothing without.
const VERKSTEAD_INSTALL: &str = ".verkstead/bin";

/// The `codex` it put there, and what it says when it runs.
const A_CODEX: &str = "#!/bin/sh\nprintf '%s\\n' 'the harness Verkstead installed'\n";
const CODEX_SAYS: &str = "the harness Verkstead installed";

/// The `claude` that install left there, and what it says when it runs.
///
/// A script rather than a binary, for the reason every stub in these suites is
/// one: what has to be shown is that *this* file was found and started, and no
/// real harness could say so.
const A_CLAUDE: &str = "#!/bin/sh\nprintf '%s\\n' 'the harness the human installed'\n";
const CLAUDE_SAYS: &str = "the harness the human installed";

/// The shell a probe is written in.
const SH: &str = "/bin/sh";

/// A Conversation part-way through its first grilling, on a machine whose human
/// has installed a harness under their own home.
///
/// The fixture the two sandbox suites keep, trimmed to what this asks about:
/// everything is real, because what a description binds is read off a
/// Conversation's own rows and a fixture that hand-built the paths would prove
/// the probe works rather than that the sandbox does.
struct Standing {
    /// Kept alive for as long as the fixture is: the directories go when these
    /// drop.
    _elsewhere: tempfile::TempDir,
    state: tempfile::TempDir,
    home: tempfile::TempDir,

    /// Where the human's own install is, which is on the server's `PATH` and
    /// under the server's home.
    install: PathBuf,

    /// And where the `claude` in it links to: the version's own directory,
    /// which no `PATH` names and which a session reaches on the program's
    /// account alone.
    version: PathBuf,

    /// And a directory that is on that `PATH` and under neither the home nor
    /// the platform's own floor — which the composing drops, and which nothing
    /// here binds.
    outside: PathBuf,

    /// And where Verkstead's own install went, which `session_path` is the
    /// whole of the reason a session ever looks in.
    installed: PathBuf,

    conversation: store::Conversation,
    profile: store::Profile,
    skills: Skills,
    verkstead: Executable,
    handoffs: Handoffs,
    attachments: Attachments,
    settings: Settings,
}

impl Standing {
    /// The sandbox this Conversation's session would run in.
    fn sandbox(&self) -> Sandbox {
        Sandbox::for_conversation(
            &self.conversation,
            &self.profile,
            &Homes::on(
                Platform::HERE,
                self.home.path().to_owned(),
                self.state.path(),
            ),
            &Reachable::at(LISTENING),
            &self.skills,
            &self.verkstead,
            &self.handoffs,
            &self.attachments,
            &self.settings.secrets(),
            &self.settings.config(),
            &BuildCache::none(),
            vec![],
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
    }
}

/// Stand one up: the machine's environment first, and everything else after it.
///
/// **The environment before anything reads one.** What a session's `PATH` is
/// composed from is read once, at the first ask, and held for the rest of the
/// process — so this has to be the state of the machine before a sandbox, a
/// probe or a home has been asked about at all.
async fn standing() -> Standing {
    let elsewhere = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // What the human installed, in the shape their installer left it: the
    // program under a versions directory of its own, and a link to it on the
    // `PATH`.
    let install = home.path().join(NATIVE_INSTALL);
    let version = home.path().join(NATIVE_VERSION);
    std::fs::create_dir_all(&install).unwrap();
    std::fs::create_dir_all(&version).unwrap();
    program(&version.join("claude"), A_CLAUDE);
    std::os::unix::fs::symlink(version.join("claude"), install.join("claude")).unwrap();
    std::fs::write(
        home.path().join(BESIDE_THE_VERSIONS),
        "what the human keeps beside their install\n",
    )
    .unwrap();

    // And a directory on the same `PATH` that is nobody's home and no part of
    // the machine's own toolchain — an `/opt/something/bin`, which is what the
    // composing drops as somewhere a session could not reach.
    let outside = elsewhere.path().join("opt/bin");
    std::fs::create_dir_all(&outside).unwrap();

    // And what Verkstead installed for the human, which goes on no `PATH` the
    // server was started with: `session_path` is the whole of what a session
    // ever hears about it.
    let installed = home.path().join(VERKSTEAD_INSTALL);
    std::fs::create_dir_all(&installed).unwrap();
    program(&installed.join("codex"), A_CODEX);

    // The machine's own `PATH` stays on the end of it: what the *server*
    // process runs — git, and the wrapper a session is rendered behind — is
    // found on this one, and only a session's is composed from it.
    let path = format!(
        "{}:{}:{}",
        install.display(),
        outside.display(),
        std::env::var("PATH").unwrap_or_default(),
    );

    // Safe here for the reason the module says: exactly one test of this binary
    // runs on any machine, so there is no other thread in the process reading
    // an environment while it is written.
    unsafe {
        std::env::set_var("HOME", home.path());
        std::env::set_var("PATH", path);
    }

    // And what this Data Directory was told, read the way a server coming up
    // reads it: the directory Verkstead installed into, which a session's `PATH`
    // is composed with ahead of everything above.
    let settings = Settings::in_data_dir(state.path());
    std::fs::write(
        settings.config_path(),
        format!("session_path:\n  - {}\n", installed.display()),
    )
    .unwrap();
    verkstead_server::sandbox::hold_session_path(&settings);

    let repo = repository(elsewhere.path().join("verkstead"));

    let pool = store::open_database(&state.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo_row = store::register_repo(&pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("the Repo registers");

    let claude_dir = elsewhere.path().join("account/.claude");
    let config_file = elsewhere.path().join("account/.claude.json");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(claude_dir.join("settings.json"), "{}\n").unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    // And skills of the account's own, which a session must not be grilled by.
    // Really there, so that a probe finding nothing has found the refusal over
    // them rather than an empty directory — which is the thing a per-user grant
    // said before it must not have moved.
    std::fs::create_dir_all(claude_dir.join("skills/the-accounts-own")).unwrap();
    std::fs::write(
        claude_dir.join("skills/the-accounts-own/SKILL.md"),
        "# what the account would have been grilled by\n",
    )
    .unwrap();

    let profile = store::create_profile(
        &pool,
        &store::ProfileFacts {
            name: Some("work".to_owned()),
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

    store::start_grilling(&pool, id, &commit, &worktree, &[])
        .await
        .unwrap();

    let conversation = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    let skills =
        Skills::installed(Platform::HERE, state.path()).expect("this binary carries skills");
    let handoffs = Handoffs::under(state.path());
    let attachments = Attachments::under(state.path());

    let image = state.path().join("image/verkstead");
    std::fs::create_dir_all(image.parent().unwrap()).unwrap();
    program(&image, SAYS_WHICH_BUILD);
    let verkstead = Executable::at(Platform::HERE, image, state.path())
        .expect("the executable was just written");

    Standing {
        _elsewhere: elsewhere,
        state,
        home,
        install,
        version,
        outside,
        installed,
        conversation,
        profile,
        skills,
        verkstead,
        handoffs,
        attachments,
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
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git should be on the PATH for these tests");

    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A file that is there and runnable, which is what a `PATH` walk is looking
/// for.
fn program(path: &Path, contents: &str) {
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// What the probe says about one path — the Linux suite's own vocabulary: a
/// path the description did not name is `absent`, because the mount namespace
/// it is in was never given one.
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
        say "$2" hidden
    fi
}

file() {
    if [ ! -f "$1" ]; then say "$2" absent; return; fi
    if (exec 3>> "$1") 2>/dev/null; then
        say "$2" write
    elif cat "$1" >/dev/null 2>&1; then
        say "$2" read
    else
        say "$2" hidden
    fi
}
"#;

/// Run `script` inside `sandbox` and read back what it reported.
fn probe(sandbox: &Sandbox, script: &str) -> BTreeMap<String, String> {
    let whole = format!("{PROBE}\n{script}\n");
    let (rendering, _closing) = sandbox
        .command(&[SH, "-c", &whole])
        .expect("a rendering on a platform with no identity to make");

    let output = started(&rendering);

    assert!(
        output.status.success(),
        "the probe failed inside the sandbox: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// And what a rendering comes to when it is really started.
fn started(rendering: &Rendering) -> std::process::Output {
    Command::try_from(rendering)
        .expect("a rendering with no container")
        .stdin(Stdio::null())
        .output()
        .expect("the wrapper a session is rendered behind should be on the PATH")
}

/// A path as the probe's shell will read it: one word, whatever the directory
/// somebody's temporary files are under is called.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display())
}

/// The `claude` the human installed is the one a Linux session runs, and
/// nothing else came in with the directory it is in.
#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the boundary this runs behind is a mount namespace"
)]
async fn the_harness_under_the_servers_home_is_what_a_linux_session_runs() {
    let standing = standing().await;
    let sandbox = standing.sandbox();

    // The whole claim, asked the only way that settles it: a session starting
    // the program by name, the way its agent would.
    let (rendering, _closing) = sandbox
        .command(&["claude"])
        .expect("a rendering on a platform with no identity to make");
    let output = started(&rendering);

    assert!(
        output.status.success(),
        "the harness the human installed under their own home did not run \
         inside a session: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        CLAUDE_SAYS,
        "and what ran was that install rather than something else of the name",
    );

    // And the harness Verkstead installed, which is on the list for the one
    // reason `session_path` says so — run by name here too, a directory told
    // about and not granted being a name a session finds and cannot start.
    let (rendering, _closing) = sandbox
        .command(&["codex"])
        .expect("a rendering on a platform with no identity to make");
    let output = started(&rendering);

    assert!(
        output.status.success(),
        "the harness Verkstead installed did not run inside a session: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        CODEX_SAYS,
        "and what ran was that install rather than something else of the name",
    );

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            say path "$PATH"
            say updater "${{DISABLE_AUTOUPDATER-unset}}"
            dir {install} install
            dir {version} version
            file {beside} beside-the-versions
            dir {outside} outside
            dir {installed} installed
            file "$HOME/.claude/skills/the-accounts-own/SKILL.md" the-accounts-own
            dir "$HOME/.claude/skills" account-skills
            "#,
            install = quoted(&standing.install),
            version = quoted(&standing.version),
            beside = quoted(&standing.home.path().join(BESIDE_THE_VERSIONS)),
            outside = quoted(&standing.outside),
            installed = quoted(&standing.installed),
        ),
    );

    assert_eq!(
        reported["installed"], "read",
        "the directory `session_path` named is read-only inside, exactly as a \
         directory the server's own `PATH` named is: what put it on the list is \
         no reason for a session to be able to write there",
    );
    assert!(
        reported["path"]
            .split(':')
            .take_while(|entry| *entry != "/usr/bin")
            .any(|entry| entry == standing.installed.to_string_lossy()),
        "and it is ahead of the system directories on the `PATH` a session \
         really gets, which is what the whole key is for: {}",
        reported["path"],
    );

    assert_eq!(
        reported["install"], "read",
        "the human's own install is read-only inside: a session that could \
         write there could rewrite the harness the next one runs",
    );
    assert_eq!(
        reported["version"], "read",
        "and the versions directory the link lands in is read-only with it — \
         a `PATH` grant alone would leave a session a dangling link",
    );
    assert_eq!(
        reported["beside-the-versions"], "absent",
        "the directory holding the file and nothing above it: what a session \
         needs is the version it runs, and what the human keeps beside it is \
         theirs",
    );
    assert_eq!(
        reported["updater"], "1",
        "and a Claude session is told not to update itself, that install being \
         read-only inside and the human's own besides",
    );
    assert_eq!(
        reported["outside"], "absent",
        "and a `PATH` entry under neither the home nor the machine's own floor \
         is nothing a session was given",
    );
    assert!(
        !reported["path"]
            .split(':')
            .any(|entry| entry == standing.outside.to_string_lossy()),
        "which is what the composing already said of it: {}",
        reported["path"],
    );
    assert_eq!(
        reported["the-accounts-own"], "absent",
        "and what a session is grilled by is still the product's: a grant said \
         ahead of the refusal is one the refusal stands over",
    );
    assert_eq!(
        reported["account-skills"], "read",
        "the directory covering them being no more a session's to fill in than \
         the skills are",
    );
}

/// And a Mac's policy, rendered for that same description, lets a session read
/// and run what is in there and write nothing.
#[tokio::test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "the boundary this reads the rules of is a Mac's"
)]
async fn the_harness_under_the_servers_home_is_read_and_run_by_a_macs_policy() {
    let standing = standing().await;
    let sandbox = standing.sandbox();

    let (rendering, _closing) = sandbox
        .command(&[SH, "-c", "true"])
        .expect("a rendering on a platform with no identity to make");
    let policy = policy_of(&rendering);

    let install = in_policy(&real(&standing.install));

    assert!(
        policy.contains(&format!(
            "(allow file-read* file-map-executable process-exec* (subpath {install}))"
        )),
        "the human's own install is what a session reads and runs:\n{policy}",
    );
    assert!(
        !policy.contains(&format!("(allow file-write* (subpath {install}))")),
        "and read-only is the whole of what read-only means:\n{policy}",
    );
    assert!(
        !policy.contains(&in_policy(&real(&standing.outside))),
        "while a `PATH` entry under neither the home nor the machine's own \
         floor is in no rule at all:\n{policy}",
    );

    let installed = in_policy(&real(&standing.installed));

    assert!(
        policy.contains(&format!(
            "(allow file-read* file-map-executable process-exec* (subpath {installed}))"
        )),
        "and the directory `session_path` named is read and run the same way, \
         what put it on the list being no reason to grant it differently:\n{policy}",
    );
    assert!(
        !policy.contains(&format!("(allow file-write* (subpath {installed}))")),
        "read-only there too:\n{policy}",
    );

    let version = in_policy(&real(&standing.version));

    assert!(
        policy.contains(&format!(
            "(allow file-read* file-map-executable process-exec* (subpath {version}))"
        )),
        "the versions directory the `claude` on the `PATH` links into is read \
         and run the same way, a link nothing followed being a link to \
         nothing:\n{policy}",
    );
    assert!(
        !policy.contains(&in_policy(&real(
            &standing.home.path().join(ABOVE_THE_VERSIONS)
        ))),
        "and nothing above it is in a rule at all:\n{policy}",
    );
}

/// The policy a rendering applies, read back off the argument it carries it in.
fn policy_of(rendering: &Rendering) -> String {
    let mut args = rendering.argv().iter();

    while let Some(arg) = args.next() {
        if arg == "-p" {
            return args
                .next()
                .map(|policy| policy.to_string_lossy().into_owned())
                .unwrap_or_default();
        }
    }

    String::from("(there is no -p on this command at all)")
}

/// A path as a policy's own string literal.
fn in_policy(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

/// And what a path really is, which is what a policy is matched against: a Mac
/// is made of symlinks that matter, and a temporary directory is under
/// `/private/var/folders` however it was handed over.
fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).expect("the fixture's directories are all there")
}
