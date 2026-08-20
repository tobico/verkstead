//! What a session can reach, asked by running a probe inside a Conversation's
//! sandbox.
//!
//! Nothing here reads the flags the sandbox was built with. The flags *are* what
//! is being tested, and a test that asserts them asserts itself — it would go on
//! passing while bwrap changed what one of them meant, or while a later bind
//! quietly mounted something over another. What settles whether the rest of the
//! machine is still reachable is a command inside the sandbox trying to reach
//! it, and reporting what happened.
//!
//! The probe is a shell script that prints one `key=value` line per fact. Each
//! path comes back as `write`, `read`, or `absent`, and the difference between
//! the three is attempted rather than asked of the metadata: a read-only bind
//! and a directory somebody has no write permission on look identical to
//! `test -w`, and only one of them is the surface being described.

use std::collections::BTreeMap;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use verkstead_server::sandbox::{Home, Sandbox, SandboxConfig, under_dev_shell};
use verkstead_server::store;

/// A Conversation part-way through its first grilling: a Repo inside a Watched
/// Path, a Profile to run as, and a worktree under Verkstead's own state
/// directory.
///
/// Everything is real. The repository is a repository, the worktree is one git
/// made, and the Conversation is a row the store wrote — because what the
/// sandbox binds is read off those, and a fixture that hand-built the paths
/// would prove the probe works rather than that the sandbox does.
struct Grilling {
    /// Kept alive for as long as the fixture is: the directories go when these
    /// drop, and a worktree that vanished mid-probe would fail obscurely.
    watched: tempfile::TempDir,
    state: tempfile::TempDir,
    home: tempfile::TempDir,

    /// Where the Repo is, and the sibling checkout beside it that no session has
    /// any business seeing.
    repo: PathBuf,
    sibling: PathBuf,

    conversation: store::Conversation,
    profile: store::Profile,
}

impl Grilling {
    /// The sandbox this Conversation's session would run in, with `extra` as
    /// whatever Sandbox Configuration asked for.
    fn sandbox(&self, extra: Vec<PathBuf>) -> Sandbox {
        Sandbox::for_conversation(&self.conversation, &self.profile, self.home(), extra)
            .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// The host home the sandbox reads out of — the fixture's rather than
    /// whoever is running the tests, so what `~` holds is decided here.
    ///
    /// The gh config is deliberately not at `~/.config/gh`, which is what an
    /// `XDG_CONFIG_HOME` set to anything else means: inside the sandbox it has
    /// to arrive where `gh` looks regardless, because there is no
    /// `XDG_CONFIG_HOME` in there to point it anywhere else.
    fn home(&self) -> Home {
        Home {
            path: self.home.path().to_owned(),
            gh_config: self.home.path().join(".xdg-config/gh"),
        }
    }

    /// Where `~` is, as the probe will see it.
    fn home_path(&self) -> &Path {
        self.home.path()
    }

    fn worktree(&self) -> &Path {
        self.conversation
            .worktree
            .as_deref()
            .expect("a grilling Conversation has a worktree")
    }
}

/// Stand one up.
async fn grilling() -> Grilling {
    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // The two host files every sandbox reads and never writes.
    std::fs::write(
        home.path().join(".gitconfig"),
        "[user]\n\tname = Verkstead Test\n",
    )
    .unwrap();
    std::fs::create_dir_all(home.path().join(".xdg-config/gh")).unwrap();
    std::fs::write(
        home.path().join(".xdg-config/gh/hosts.yml"),
        "github.com:\n    user: nobody\n",
    )
    .unwrap();

    // Something in HOME that is none of the sandbox's business, so that "the
    // rest of HOME is absent" is a claim about this run rather than about an
    // empty directory.
    std::fs::write(home.path().join(".bash_history"), "rm -rf /\n").unwrap();
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
            claude_dir,
            config_file,
            model: "claude-opus-5".to_owned(),
            agent_type: store::AgentType::Claude,
        },
    )
    .await
    .unwrap()
    .expect("the Profile saves");

    let id = store::start_conversation(&pool, repo_row.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Conversation starts");

    store::set_grilling_profile(&pool, id, profile.id)
        .await
        .unwrap();
    store::set_implementation_profile(&pool, id, profile.id)
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

    store::start_grilling(&pool, id, &commit, &worktree)
        .await
        .unwrap();

    let conversation = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    Grilling {
        watched,
        state,
        home,
        repo,
        sibling,
        conversation,
        profile,
    }
}

/// A git repository at `path`, with one commit on `main`.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "test@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Test"]);
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

/// The shell the probe is written in, and the one path in the sandbox that is
/// guaranteed to hold one: `/bin/sh` is what the system bind puts there.
const SH: &str = "/bin/sh";

/// What the probe says about one path.
///
/// Everything the sandbox does *not* bind reads as `absent`, which is the whole
/// of what "everything else in HOME simply absent" means from inside.
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
    # Opening for append is the write, and it changes not a byte of what is
    # there. In a subshell, because a redirection that fails takes the shell
    # attempting it down with it.
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

    let output = sandbox
        .command(&[SH, "-c", &whole])
        .stdin(Stdio::null())
        .output()
        .expect("bwrap should be on the PATH: the dev shell declares bubblewrap");

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

/// Where a program is on the host, absolute.
///
/// The probe calls the few tools it needs by their whole path, because the
/// sandbox's `PATH` is the machine's system profile rather than the shell the
/// tests were started from — and everything under `/nix` is reachable inside
/// either way.
fn on_the_host(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{program} should be on the PATH for these tests"))
}

#[tokio::test]
async fn the_worktree_and_the_repos_git_directory_are_the_two_writable_things() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {worktree} worktree
            dir {git_dir} git-dir
            "#,
            worktree = quoted(fixture.worktree()),
            git_dir = quoted(&fixture.repo.join(".git")),
        ),
    );

    assert_eq!(reported["worktree"], "write", "a session commits its work");
    assert_eq!(
        reported["git-dir"], "write",
        "the objects and refs a commit is written into are the Repo's, not the worktree's"
    );
}

#[tokio::test]
async fn the_system_and_the_hosts_two_files_come_in_read_only() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir /nix nix
            file {gitconfig} gitconfig
            file "$HOME/.config/gh/hosts.yml" gh
            dir {outside} gh-where-the-host-keeps-it
            "#,
            gitconfig = quoted(&fixture.home_path().join(".gitconfig")),
            outside = quoted(&fixture.home_path().join(".xdg-config")),
        ),
    );

    assert_eq!(reported["nix"], "read");
    assert_eq!(
        reported["gitconfig"], "read",
        "who a session commits as is the human's to set, not the session's to change"
    );
    assert_eq!(
        reported["gh"], "read",
        "the gh login arrives where gh looks for it, there being no XDG_CONFIG_HOME inside"
    );
    assert_eq!(
        reported["gh-where-the-host-keeps-it"], "absent",
        "and nowhere else: what the host calls it is the host's business"
    );
}

#[tokio::test]
async fn the_profiles_pair_is_the_whole_of_what_home_holds() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    // Every path here is `$HOME`'s, which the sandbox sets: what is being asked
    // is what a session finds when it looks where it lives.
    let reported = probe(
        &sandbox,
        r#"
            dir "$HOME/.claude" claude-dir
            file "$HOME/.claude.json" claude-config
            file "$HOME/.ssh/id_ed25519" private-key
            file "$HOME/.bash_history" history
            say home "$(ls -A "$HOME" | sort | tr '\n' ' ')"
        "#,
    );

    assert_eq!(
        reported["claude-dir"], "write",
        "a session writes its own transcripts and settings"
    );
    assert_eq!(reported["claude-config"], "write");
    assert_eq!(reported["private-key"], "absent");
    assert_eq!(reported["history"], "absent");
    assert_eq!(
        reported["home"], ".claude .claude.json .config .gitconfig ",
        "everything else in HOME is simply not there"
    );
}

#[tokio::test]
async fn no_other_checkout_on_the_machine_is_reachable() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {sibling} sibling
            file {readme} repo-readme
            say watched-holds "$(ls -A {watched} | sort | tr '\n' ' ')"
            say repo-holds "$(ls -A {repo} | sort | tr '\n' ' ')"
            "#,
            watched = quoted(fixture.watched.path()),
            repo = quoted(&fixture.repo),
            sibling = quoted(&fixture.sibling),
            readme = quoted(&fixture.repo.join("README.md")),
        ),
    );

    assert_eq!(
        reported["sibling"], "absent",
        "another repository under the same Watched Path is another Conversation's business"
    );
    assert_eq!(
        reported["repo-readme"], "absent",
        "the checkout the worktree was made from is not the worktree"
    );

    // A bind's parent directories have to exist for it to land on, so the
    // Watched Path is inside as a scaffold: empty tmpfs directories holding
    // nothing but what was deliberately bound, and writing in them writes
    // nothing the host will ever see.
    assert_eq!(
        reported["watched-holds"], "verkstead ",
        "nothing under a Watched Path arrives except by being bound — and the \
         Profile's pair arrives in HOME rather than where it lives"
    );
    assert_eq!(
        reported["repo-holds"], ".git ",
        "the Repo is inside as its git directory and nothing else"
    );
}

#[tokio::test]
async fn the_network_is_the_hosts_own() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    // A listener in this process, which is in the host's network namespace. A
    // sandbox with a namespace of its own would have its own empty loopback and
    // find nothing at this port — so reaching it is the sharing, proved without
    // anything here touching the internet.
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
            if {curl} --silent --max-time 10 --output /dev/null "http://127.0.0.1:{port}/"; then
                say network reached
            else
                say network unreachable
            fi
            "#,
            curl = quoted(&on_the_host("curl")),
        ),
    );

    assert_eq!(
        reported["network"], "reached",
        "the filesystem is the boundary; the network is not"
    );

    answering.join().unwrap();
}

#[tokio::test]
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

    let sandbox = fixture.sandbox(config.binds_for(&fixture.conversation.repo.name));

    let reported = probe(
        &sandbox,
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
        reported["another-repos"], "absent",
        "a bind configured for another Repo is that Repo's"
    );
}

/// The composition itself, without a sandbox to run in: what a Repo gets is the
/// global set and then its own.
#[test]
fn a_repos_own_binds_compose_over_the_global_ones() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let cargo = dir.path().join("cargo");
    let node = dir.path().join("node");
    for made in [&cache, &cargo, &node] {
        std::fs::create_dir(made).unwrap();
    }

    let config = SandboxConfig::resolve(&[
        cache.display().to_string(),
        format!("verkstead={}", cargo.display()),
        format!("askance={}", node.display()),
    ])
    .unwrap();

    assert_eq!(
        config.binds_for("verkstead"),
        vec![cache.clone(), cargo],
        "the global set is not given up by a Repo asking for one of its own"
    );
    assert_eq!(config.binds_for("askance"), vec![cache.clone(), node]);
    assert_eq!(
        config.binds_for("something-nobody-configured"),
        vec![cache],
        "a Repo with nothing of its own still gets what every one of them gets"
    );
}

#[test]
fn a_bind_that_is_not_there_refuses_to_resolve() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("never-made");

    assert!(
        SandboxConfig::resolve(&[missing.display().to_string()]).is_err(),
        "a bind bwrap could not make is every session in that Repo failing to start"
    );
    assert!(SandboxConfig::resolve(&[format!("verkstead={}", missing.display())]).is_err(),);
}

#[test]
fn a_bind_that_is_neither_a_path_nor_a_named_one_is_refused() {
    for refused in ["cache", "verkstead=cache", "=/var/cache"] {
        assert!(
            SandboxConfig::resolve(&[refused.to_owned()]).is_err(),
            "{refused:?} should be refused"
        );
    }
}

/// A path as one shell word.
///
/// Single quotes, because a temporary directory's name is not the test's to
/// choose and a space in `TMPDIR` would otherwise split the probe's argument in
/// two.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

#[test]
fn a_repository_whose_flake_has_a_dev_shell_runs_its_command_under_nix_develop() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("flake.nix"), DEV_SHELL).unwrap();

    assert_eq!(
        under_dev_shell(dir.path(), &["claude".to_owned()]),
        vec![
            "nix".to_owned(),
            "develop".to_owned(),
            "--command".to_owned(),
            "claude".to_owned()
        ],
    );
}

#[test]
fn a_flake_that_defines_no_shell_runs_the_command_as_it_stands() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("flake.nix"), NO_DEV_SHELL).unwrap();

    assert_eq!(
        under_dev_shell(dir.path(), &["claude".to_owned()]),
        vec!["claude".to_owned()],
        "`nix develop` errors out where none of the attributes it falls through exist"
    );
}

#[test]
fn a_repository_with_no_flake_at_all_runs_the_command_as_it_stands() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        under_dev_shell(dir.path(), &["claude".to_owned()]),
        vec!["claude".to_owned()],
    );
}

/// A flake with a dev shell and no inputs, so the evaluation this provokes needs
/// nothing fetched and no store path built.
const DEV_SHELL: &str = r#"
{
  outputs = { self }: {
    devShells.x86_64-linux.default = "a dev shell";
    devShells.aarch64-linux.default = "a dev shell";
  };
}
"#;

/// And one that builds a package under a name `nix develop` does not fall
/// through to.
const NO_DEV_SHELL: &str = r#"
{
  outputs = { self }: {
    packages.x86_64-linux.something = "not a shell";
    packages.aarch64-linux.something = "not a shell";
  };
}
"#;
