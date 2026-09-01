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
use verkstead_server::sandbox::{Executable, Home, Reachable, Sandbox};
use verkstead_server::settings::Settings;
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
/// Absolute, as everything the probe runs is: the `PATH` inside is still the
/// Linux one until the task that makes it a Mac's, and what saves the probe
/// meanwhile is the `/usr/bin` on the end of it rather than anything decided.
const CURL: &str = "/usr/bin/curl";

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

    skills: Skills,
    verkstead: Executable,
    handoffs: Handoffs,
    settings: Settings,
}

impl Grilling {
    /// The sandbox this Conversation's session would run in.
    ///
    /// With no shared build cache and no configured binds: this stage builds
    /// the floor a session stands on, and what is composed over it is the two
    /// tasks after it.
    fn sandbox(&self) -> Sandbox {
        Sandbox::for_conversation(
            &self.conversation,
            &self.profile,
            Home {
                path: self.home.path().to_owned(),
            },
            &Reachable::at(LISTENING),
            &self.skills,
            &self.verkstead,
            &self.handoffs,
            &self.settings.secrets(),
            &self.settings.config(),
            &BuildCache::none(),
            vec![],
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
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

    fn home_path(&self) -> &Path {
        self.home.path()
    }
}

/// Stand one up.
async fn grilling() -> Grilling {
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

    store::start_grilling(&pool, id, &commit, &worktree, &[])
        .await
        .unwrap();

    let conversation = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    let skills = Skills::installed(state.path()).expect("this binary carries skills");
    let handoffs = Handoffs::under(state.path());
    let settings = Settings::in_data_dir(state.path());

    let image = state.path().join("bin/verkstead");
    std::fs::create_dir_all(image.parent().unwrap()).unwrap();
    std::fs::write(&image, SAYS_WHICH_BUILD).unwrap();
    std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o755)).unwrap();
    let verkstead = Executable::at(image).expect("the executable was just written");

    Grilling {
        watched,
        state,
        home,
        repo,
        sibling,
        conversation,
        profile,
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
