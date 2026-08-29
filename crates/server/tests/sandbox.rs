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
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::sandbox::{
    Bind, Executable, Home, Reachable, Sandbox, SandboxConfig, under_dev_shell,
};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// Where the server this Conversation belongs to is listening — which is what a
/// session inside is told to put its Question Sets to.
const LISTENING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8422);

/// What stands in for the server's own image: an executable that says which
/// build it is.
///
/// A real server equips a session with the binary it is itself running, so that
/// the CLI a session asks with and the server it asks cannot disagree about a
/// schema. A test harness's own image is the test harness, which would prove
/// nothing about *which* binary arrived — so the fixture writes one of its own
/// and hands the sandbox that. The bind is the same either way, and this one
/// answers in words a probe can recognise.
const SAYS_WHICH_BUILD: &str = "#!/bin/sh\nprintf 'verkstead 0.0.0-the-servers-own\\n'\n";

/// And what stands in for the sccache the server resolved, for the same reason:
/// what has to be shown is that the binary a session compiles through is the
/// one the server found, which a real sccache could not say.
const SAYS_WHICH_SCCACHE: &str = "#!/bin/sh\nprintf 'sccache 0.0.0-the-one-resolved\\n'\n";

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

    /// The store the Conversation was written into, for the test that stands a
    /// real server up over it and lets a session inside the sandbox ask it
    /// something.
    pool: sqlx::SqlitePool,

    /// The bundled skills, installed where the server installs them: under the
    /// Data Directory, at startup.
    skills: Skills,

    /// And the executable a session asks with, which for a real server is its
    /// own image — see [`SAYS_WHICH_BUILD`] for what stands in for one here.
    verkstead: Executable,

    /// And where the handoff documents go, which is a root under the same
    /// directory — one directory per Conversation, made as its sandbox is built.
    handoffs: Handoffs,

    /// The settings files, in that directory again. Nothing is in them until a
    /// test says so — see [`Grilling::configure_github_token`] and
    /// [`Grilling::configure`] — which is what an installation nobody
    /// has been to the settings page of looks like.
    settings: Settings,
}

impl Grilling {
    /// The sandbox this Conversation's session would run in, with `extra` as
    /// whatever Sandbox Configuration asked for.
    ///
    /// With no shared build cache, which is what every test here that is not
    /// about one wants: the cache is a bind and four variables, and a test
    /// asking what else is inside should not have to know about them.
    fn sandbox(&self, extra: Vec<Bind>) -> Sandbox {
        self.sandbox_reaching(LISTENING, &BuildCache::none(), extra)
    }

    /// And one built around `cache`, which is what the tests about the build
    /// cache ask for. What is switched on and how big it may grow is read out
    /// of `config.yaml` as the sandbox is built, so a test says that by writing
    /// the file — see [`Grilling::configure`].
    fn sandbox_caching(&self, cache: &BuildCache) -> Sandbox {
        self.sandbox_reaching(LISTENING, cache, vec![])
    }

    /// Where the shared build cache is on the host, which is a directory of the
    /// fixture's own rather than the machine's XDG one.
    fn cache_dir(&self) -> PathBuf {
        self.state.path().join("build-cache")
    }

    /// A cache at that directory, with a stub sccache where `compiling` says so.
    ///
    /// The stub is a script that says which build it is, for the reason
    /// [`SAYS_WHICH_BUILD`] is one: what has to be shown is that the file the
    /// server resolved is the file a session finds at
    /// `/verkstead/bin/sccache`, and a real sccache would answer that question
    /// with whatever the machine happened to have installed.
    fn cache(&self, compiling: bool) -> BuildCache {
        let dir = self.cache_dir();
        std::fs::create_dir_all(&dir).unwrap();

        BuildCache::at(dir, compiling.then(|| self.sccache()))
    }

    /// The stub sccache, written where the server would have found a real one.
    fn sccache(&self) -> PathBuf {
        let path = self.state.path().join("sccache");

        std::fs::write(&path, SAYS_WHICH_SCCACHE).unwrap();
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

    /// The same, for a server that is really listening somewhere — which is what
    /// a session inside has to be able to reach to ask anything.
    fn sandbox_reaching(
        &self,
        listening: SocketAddr,
        cache: &BuildCache,
        extra: Vec<Bind>,
    ) -> Sandbox {
        Sandbox::for_conversation(
            &self.conversation,
            &self.profile,
            self.home(),
            &Reachable::at(listening),
            &self.skills,
            &self.verkstead,
            &self.handoffs,
            // Read here rather than at startup, which is where the server reads
            // them too: a sandbox carries the token and the author that were
            // configured when it was built.
            &self.settings.secrets(),
            &self.settings.config(),
            cache,
            extra,
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// Write `secrets.yaml` as the settings page would, so that the sandboxes
    /// built after this carry the token.
    fn configure_github_token(&self, yaml: &str) {
        std::fs::write(self.settings.secrets_path(), yaml).unwrap();
    }

    /// And `config.yaml`, which is who those sandboxes commit as and how their
    /// shared build cache is set.
    fn configure(&self, yaml: &str) {
        std::fs::write(self.settings.config_path(), yaml).unwrap();
    }

    /// The host home a sandbox is built around — the fixture's rather than
    /// whoever is running the tests, so what `~` holds is decided here.
    fn home(&self) -> Home {
        Home {
            path: self.home.path().to_owned(),
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

/// Stand one up, with no companion repos — which is the ordinary Conversation
/// and what most of these tests are about.
async fn grilling() -> Grilling {
    grilling_alongside(&[]).await
}

/// And one configured with companion repos, each registered under the name given
/// and added in the mode given.
///
/// They are added while the Conversation is still drafting, which is the only
/// time they can be, and then checked out beside its own the way a grill start
/// checks them out: a read-write companion on a branch of its own, a read-only
/// one detached at the commit its base resolved to.
async fn grilling_alongside(companions: &[(&str, store::CompanionMode)]) -> Grilling {
    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // A gitconfig of the machine's, which sandboxes used to be given and are
    // not: who a session commits as is configured now, so this is here to be
    // absent from inside rather than to be found there.
    std::fs::write(
        home.path().join(".gitconfig"),
        "[user]\n\tname = Whoever The Host Is\n\temail = host@verkstead.invalid\n",
    )
    .unwrap();

    // And a gh login of the machine's, in both the places gh keeps one: no
    // session has any business seeing either, whatever the host is logged in as.
    for config in [".config/gh", ".xdg-config/gh"] {
        std::fs::create_dir_all(home.path().join(config)).unwrap();
        std::fs::write(
            home.path().join(config).join("hosts.yml"),
            "github.com:\n    user: nobody\n",
        )
        .unwrap();
    }

    // Something in HOME that is none of the sandbox's business, so that "the
    // rest of HOME is absent" is a claim about this run rather than about an
    // empty directory.
    std::fs::write(home.path().join(".bash_history"), "rm -rf /\n").unwrap();
    std::fs::create_dir_all(home.path().join(".ssh")).unwrap();
    std::fs::write(home.path().join(".ssh/id_ed25519"), "a private key\n").unwrap();

    // The skills the host keeps for its own agents, in the checkout every one of
    // these sessions used to be given. Verkstead ships its own now, and this is
    // here so that "no `~/src/tobico-skills`" is a claim about a directory that
    // exists on the host rather than about one nobody made.
    std::fs::create_dir_all(home.path().join("src/tobico-skills/skills/grilling")).unwrap();
    std::fs::write(
        home.path()
            .join("src/tobico-skills/skills/grilling/SKILL.md"),
        "# the host's own\n",
    )
    .unwrap();

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

    // And a skill of the account's own, where Verkstead's are about to be
    // mounted: what a session is grilled by is the product's, so this is what
    // being hidden looks like from inside.
    std::fs::create_dir_all(claude_dir.join("skills/the-accounts-own")).unwrap();
    std::fs::write(
        claude_dir.join("skills/the-accounts-own/SKILL.md"),
        "# the account's own\n",
    )
    .unwrap();

    let profile = store::create_profile(
        &pool,
        &store::ProfileFacts {
            name: "work".to_owned(),
            claude_dir,
            config_file,
            models: vec!["claude-opus-5".to_owned()],
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

    store::set_grilling_pairing(&pool, id, profile.id, profile.model())
        .await
        .unwrap();
    store::set_implementation_pairing(&pool, id, profile.id, profile.model())
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

    // The executable a session is equipped with, somewhere no session can reach
    // it except through the bind — see [`SAYS_WHICH_BUILD`].
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
        pool,
        skills,
        verkstead,
        handoffs,
        settings,
    }
}

/// A git repository at `path`, with one commit on `main` and a GitHub remote it
/// was cloned over SSH from.
///
/// Both of those are what the sandbox has to be able to override. The local
/// identity is the one a repository happens to carry, and the SSH remote is the
/// one there are no keys inside a sandbox for — see
/// [`an_ssh_github_remote_resolves_to_https_and_the_token_is_what_pushes_it`].
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "local@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Whatever The Repo Says"]);
    git(
        &path,
        &[
            "remote",
            "add",
            "origin",
            "git@github.com:tobico/verkstead.git",
        ],
    );
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
async fn the_worktree_the_git_directory_and_the_handoff_directory_are_what_can_be_written() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {worktree} worktree
            dir {git_dir} git-dir
            dir /tmp/verkstead handoff
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
    assert_eq!(
        reported["handoff"], "write",
        "and the Conversation's own directory, which is the one writable place git will never see"
    );
}

/// The handoff directory is a bind and not part of the tmpfs `/tmp` is
/// otherwise made of — which is the whole of what makes it useful: a document
/// written in there has to be one Verkstead can read once the session is gone.
#[tokio::test]
async fn what_a_session_writes_in_its_handoff_directory_is_there_when_it_has_gone() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        r#"
        printf '# What we settled\n' > /tmp/verkstead/handoff.md
        say wrote yes
        "#,
    );

    assert_eq!(reported["wrote"], "yes");

    let outside = fixture
        .handoffs
        .directory(fixture.conversation.id)
        .expect("the directory the sandbox bound")
        .join("handoff.md");

    assert_eq!(
        std::fs::read_to_string(&outside).ok().as_deref(),
        Some("# What we settled\n"),
        "nothing written inside reached {}",
        outside.display()
    );
}

#[tokio::test]
async fn the_system_comes_in_read_only_and_the_hosts_gitconfig_not_at_all() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir /nix nix
            file {gitconfig} gitconfig
            "#,
            gitconfig = quoted(&fixture.home_path().join(".gitconfig")),
        ),
    );

    assert_eq!(reported["nix"], "read");
    assert_eq!(
        reported["gitconfig"], "absent",
        "who a session commits as is configured rather than found lying about in a home directory"
    );
}

/// Who a session commits as: the configured author, proved by a commit made
/// inside and read back outside.
///
/// The repository has an identity of its own and the host has a gitconfig, and
/// neither is what lands on the commit — which is the point of configuring one
/// at all. A session works in a checkout somebody else made, and what it commits
/// as should be a fact about the installation rather than about whatever that
/// checkout was left holding.
#[tokio::test]
async fn a_commit_made_inside_is_by_the_configured_author() {
    let fixture = grilling().await;
    fixture.configure("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            if {git} commit --quiet --allow-empty -m 'from inside' 2>/tmp/git-said; then
                say committed yes
            else
                say committed "no: $(cat /tmp/git-said)"
            fi
            "#,
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(reported["committed"], "yes");

    // Read outside, off the branch the worktree is on: the commit is in the
    // Repo's object database, which is what the session was given to write into.
    assert_eq!(
        git(fixture.worktree(), &["log", "-1", "--format=%an <%ae>"]).trim(),
        "Tobias Cohen <tobi@tobico.net>",
        "the commit is by whoever config.yaml says, not by the repository's own \
         local identity and not by the host's gitconfig"
    );
}

/// And with nobody configured, git's own refusal stands. No author is invented
/// — a commit by `verkstead@localhost` is the one nobody notices — and the
/// settings page is where the missing state gets surfaced.
///
/// The repository's own identity is taken away first, because a checkout that
/// carries one is not the case being asked about: what has to be true is that a
/// session with nothing to commit as says so rather than committing as
/// something.
#[tokio::test]
async fn no_author_configured_is_git_asking_to_be_told_who_you_are() {
    let fixture = grilling().await;

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            {git} config --unset user.name
            {git} config --unset user.email

            if {git} commit --quiet --allow-empty -m 'from inside' 2>/tmp/git-said; then
                say committed yes
            else
                say committed no
            fi

            say said "$({grep} -c 'tell me who you are' /tmp/git-said)"
            "#,
            git = quoted(&on_the_host("git")),
            grep = quoted(&on_the_host("grep")),
        ),
    );

    assert_eq!(
        reported["committed"], "no",
        "with nobody configured anywhere there is nobody to commit as"
    );
    assert_eq!(
        reported["said"], "1",
        "and what a session is left with is git's own answer, which says what to configure"
    );
}

/// A push out of a sandbox goes over HTTPS with the token, whatever the remote
/// the repository was cloned from says.
///
/// There are no SSH keys inside a sandbox and there is not going to be one: the
/// credentials are the token, and an SSH remote would fail on a key that is not
/// there rather than fall back to anything. So the URL is rewritten as git
/// resolves it — the repository's own `.git/config` is left saying exactly what
/// the human cloned — and the credential helper is `gh`'s, which answers out of
/// `GH_TOKEN`.
///
/// Asked of git inside rather than of the flags, like everything else here: what
/// settles it is git resolving the remote and naming the helper it would ask.
#[tokio::test]
async fn an_ssh_github_remote_resolves_to_https_and_the_token_is_what_pushes_it() {
    let fixture = grilling().await;
    fixture.configure_github_token("github_token: ghp_theconfiguredone\n");

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            say remote "$({git} ls-remote --get-url origin)"
            say written-down "$({git} config --get remote.origin.url)"
            say helper "$({git} config --get-urlmatch credential.helper https://github.com)"
            say token "${{GH_TOKEN-unset}}"
            say prompt "${{GIT_TERMINAL_PROMPT-unset}}"
            "#,
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(
        reported["remote"], "https://github.com/tobico/verkstead.git",
        "an SSH remote is resolved to the HTTPS one the token is any use for"
    );
    assert_eq!(
        reported["written-down"], "git@github.com:tobico/verkstead.git",
        "and the repository still says what the human cloned"
    );
    assert_eq!(
        reported["helper"], "!gh auth git-credential",
        "which is what turns GH_TOKEN into an authenticated push"
    );
    assert_eq!(reported["token"], "ghp_theconfiguredone");
    assert_eq!(
        reported["prompt"], "0",
        "and a push that cannot authenticate says so rather than asking a terminal \
         nobody is sitting at"
    );
}

/// The other spelling of the same remote, which a `.gitmodules` or an older
/// clone is as likely to hold.
#[tokio::test]
async fn the_url_form_of_an_ssh_github_remote_is_rewritten_too() {
    let fixture = grilling().await;

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            {git} remote set-url origin ssh://git@github.com/tobico/verkstead.git
            say remote "$({git} ls-remote --get-url origin)"
            "#,
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(
        reported["remote"], "https://github.com/tobico/verkstead.git",
        "`ssh://git@github.com/` is the same remote written another way"
    );
}

/// GitHub auth is said rather than found: the token the human configured, in
/// the environment `gh` reads it out of, and no gh files anywhere.
#[tokio::test]
async fn the_configured_token_is_in_the_environment_and_the_hosts_gh_login_is_not_inside() {
    let fixture = grilling().await;
    fixture.configure_github_token("github_token: ghp_theconfiguredone\n");
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            say token "${{GH_TOKEN-unset}}"
            file "$HOME/.config/gh/hosts.yml" gh
            dir "$HOME/.config/gh" gh-dir
            dir {outside} gh-where-the-host-might-keep-it
            "#,
            outside = quoted(&fixture.home_path().join(".xdg-config")),
        ),
    );

    assert_eq!(
        reported["token"], "ghp_theconfiguredone",
        "`gh` inside authenticates as whoever the settings file says"
    );
    assert_eq!(
        reported["gh"], "absent",
        "nothing of the host's gh login comes in: the token is the whole of it"
    );
    assert_eq!(reported["gh-dir"], "absent");
    assert_eq!(
        reported["gh-where-the-host-might-keep-it"], "absent",
        "and not under whatever the host's XDG_CONFIG_HOME called it either"
    );
}

/// The three ways there is no token — no file, an empty one, and one nothing
/// can parse — are one answer: a session that starts, with `gh` inside saying
/// for itself that it is not logged in.
#[tokio::test]
async fn no_token_configured_is_a_session_that_starts_with_no_gh_token() {
    for configured in [None, Some(""), Some("github_token: [oh\n")] {
        let fixture = grilling().await;

        if let Some(yaml) = configured {
            fixture.configure_github_token(yaml);
        }

        let reported = probe(&fixture.sandbox(vec![]), r#"say token "${GH_TOKEN-unset}""#);

        assert_eq!(
            reported["token"], "unset",
            "with {configured:?} in secrets.yaml the variable should not be there at all: \
             an empty GH_TOKEN is a login gh fails on obscurely"
        );
    }
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
        "a session writes its own session logs and settings"
    );
    assert_eq!(reported["claude-config"], "write");
    assert_eq!(reported["private-key"], "absent");
    assert_eq!(reported["history"], "absent");
    assert_eq!(
        reported["home"], ".claude .claude.json ",
        "everything else in HOME is simply not there"
    );
}

#[tokio::test]
async fn the_skills_inside_are_the_bundled_ones_and_only_those() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            file "$HOME/.claude/skills/grilling/SKILL.md" grilling
            file "$HOME/.claude/skills/the-accounts-own/SKILL.md" the-accounts-own
            file "$HOME/.claude/CLAUDE.md" claude-md
            dir {tobico} tobico-skills
            if {grep} -q 'verkstead ask' "$HOME/.claude/skills/grilling/SKILL.md"; then
                say ask-instruction inside
            else
                say ask-instruction missing
            fi
            "#,
            tobico = quoted(&fixture.home_path().join("src/tobico-skills")),
            grep = quoted(&on_the_host("grep")),
        ),
    );

    assert_eq!(
        reported["grilling"], "read",
        "the bundled grilling skill is installed, and is not a session's to rewrite"
    );
    assert_eq!(
        reported["the-accounts-own"], "absent",
        "what a session is grilled by is the product's, not whatever the account keeps"
    );
    assert_eq!(
        reported["tobico-skills"], "absent",
        "the host's own checkout of the skills is no longer bound in"
    );
    assert_eq!(
        reported["claude-md"], "absent",
        "there is no global CLAUDE.md in here to say how to reach the human"
    );
    assert_eq!(
        reported["ask-instruction"], "inside",
        "so the bundled skill has to carry the instruction itself"
    );
}

/// `verkstead` inside is the executable serving the session, and it is what a
/// bare `verkstead` finds.
///
/// The two halves of an ask are the CLI a session runs and the server it puts a
/// Set to, and they have to be one build: a machine's install is a separate one,
/// and the two have already disagreed about what a `proposal` may carry. So the
/// server hands over its own image, in a directory holding nothing else, ahead
/// of every path the host could have installed a `verkstead` on.
///
/// Asked of a shell inside rather than of the flags, like everything else here:
/// what settles which binary a session asks with is a session looking one up and
/// running it.
#[tokio::test]
async fn the_verkstead_a_session_asks_with_is_the_one_serving_it() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

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
        reported["found"], "/verkstead/bin/verkstead",
        "a bare `verkstead` is the one the server bound in"
    );
    assert_eq!(
        reported["version"], "verkstead 0.0.0-the-servers-own",
        "and running it runs the server's own build, not whatever the machine has"
    );
    assert_eq!(
        reported["beside"], "verkstead",
        "the directory holds the one executable the server put there and nothing else"
    );
    assert_eq!(
        reported["first"], "/verkstead/bin",
        "which is looked in before every path an install could have landed on"
    );
    assert_eq!(
        reported["binary"], "read",
        "and it is no more a session's to rewrite mid-run than the skills are"
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

/// The one thing a session is given that is not a directory, and the whole of
/// what makes its Question Sets its own Conversation's.
///
/// Asked from inside rather than read off the flags the sandbox was built with,
/// like every other claim in this file: what settles whether a session can reach
/// Verkstead is a session trying to, and a Set that lands on the right Timeline
/// is the only evidence that it did.
///
/// The server is real and listening on the host's loopback, which the sandbox
/// shares — see [`the_network_is_the_hosts_own`]. Everything but the agent is
/// real too: `curl` inside the sandbox is standing in for the bundled CLI, which
/// posts exactly this to exactly this URL.
#[tokio::test]
async fn a_session_puts_a_set_to_its_own_conversation_and_nothing_else() {
    let fixture = grilling().await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listening = listener.local_addr().unwrap();

    let serving = tokio::spawn({
        let pool = fixture.pool.clone();
        async move {
            let _ = axum::serve(listener, verkstead_server::router(pool)).await;
        }
    });

    let sandbox = fixture.sandbox_reaching(listening, &BuildCache::none(), vec![]);

    // Every part of the sandbox blocks, and this one is a process talking to a
    // server on the runtime this test is on.
    let reported = tokio::task::spawn_blocking(move || {
        probe(
            &sandbox,
            &format!(
                r#"
                say server "$VERKSTEAD_SERVER"

                {curl} --silent --show-error --fail \
                    --header 'Content-Type: application/yaml' \
                    --data-binary @- \
                    --output /tmp/created \
                    "$VERKSTEAD_SERVER/api/v1/sets" <<'YAML'
title: What a delivery that has failed forty times becomes
questions:
  - label: Q1
    text: How many failures before an endpoint is given up on?
    options:
      - n: 1
        text: Five
        recommended: true
YAML

                say submitted "$?"
                "#,
                curl = quoted(&on_the_host("curl")),
            ),
        )
    })
    .await
    .unwrap();

    assert_eq!(
        reported["server"],
        format!(
            "http://{listening}/conversations/{}",
            fixture.conversation.id
        ),
        "a session is pointed at its own Conversation, explicitly"
    );
    assert_eq!(
        reported["submitted"], "0",
        "the server should have taken the Set"
    );

    let timeline = store::timeline(&fixture.pool, fixture.conversation.id)
        .await
        .unwrap();

    let asked: Vec<&store::SetOnTimeline> = timeline
        .iter()
        .filter_map(|event| match &event.event {
            store::Event::QuestionSet(asked) => Some(asked.as_ref()),
            _ => None,
        })
        .collect();

    assert_eq!(asked.len(), 1, "the Timeline it landed on is its own");
    assert_eq!(
        asked[0]
            .set
            .set()
            .expect("the Set the session just sent reads back")
            .title,
        "What a delivery that has failed forty times becomes"
    );

    serving.abort();
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

    let sandbox = fixture.sandbox(config.binds_for(&fixture.conversation));

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

/// A read-only companion is there to be read: the checkout and the history
/// behind it, and nothing a session does inside it changes either.
///
/// Both halves are asked of git rather than of the flags. A worktree bound
/// read-only with a writable git directory would still take a commit, and one
/// whose git directory is read-only refuses more than committing — the index and
/// its lock live in there too, so what has to be shown is that reading really
/// does work rather than that writing really does not.
#[tokio::test]
async fn a_read_only_companion_is_there_to_read_and_a_commit_from_it_is_refused() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadOnly)]).await;
    fixture.configure("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

    let companion = fixture.companion_worktree("askance").to_owned();

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            dir {companion} worktree
            dir {git_dir} git-dir
            say readme "$({cat} {companion}/README.md)"
            say history "$({git} -C {companion} log --oneline | wc -l)"

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
            cat = quoted(&on_the_host("cat")),
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
async fn a_read_write_companion_takes_a_commit_on_its_own_branch() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadWrite)]).await;
    fixture.configure("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

    let companion = fixture.companion_worktree("askance").to_owned();

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            printf 'from inside\n' > {companion}/NOTES.md
            {git} -C {companion} add NOTES.md

            if {git} -C {companion} commit --quiet -m 'from inside' 2>/tmp/git-said; then
                say committed yes
            else
                say committed "no: $(cat /tmp/git-said)"
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
/// A build cache is the installer's own hole and it sits outside the repository:
/// a read-only companion is a checkout not to be changed rather than a
/// repository whose builds should fail on a cold cache.
#[tokio::test]
async fn a_read_only_companions_own_configured_binds_are_still_writable() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadOnly)]).await;

    let cache = fixture.state.path().join("askance-node-modules");
    std::fs::create_dir_all(&cache).unwrap();

    let config = SandboxConfig::resolve(&[format!("askance={}", cache.display())]).unwrap();
    let sandbox = fixture.sandbox(config.binds_for(&fixture.conversation));

    let reported = probe(
        &sandbox,
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

/// The composition itself, without a sandbox to run in: what a Conversation
/// gets is the global set, then its own Repo's, then each of its companions'.
///
/// The companion is read-only, which is the case worth asking about: what is
/// configured for a Repo is a build cache outside it, and a build writes to one
/// whether or not the checkout beside it may be written to.
#[tokio::test]
async fn a_repos_own_binds_compose_over_the_global_ones() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadOnly)]).await;

    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let cargo = dir.path().join("cargo");
    let node = dir.path().join("node");
    let nobodys = dir.path().join("nobodys");
    for made in [&cache, &cargo, &node, &nobodys] {
        std::fs::create_dir(made).unwrap();
    }

    let config = SandboxConfig::resolve(&[
        cache.display().to_string(),
        format!("verkstead={}", cargo.display()),
        format!("askance={}", node.display()),
        format!("something-nobody-added={}", nobodys.display()),
    ])
    .unwrap();

    assert_eq!(
        config.binds_for(&fixture.conversation),
        vec![
            Bind::writable(cache),
            Bind::writable(cargo),
            Bind::writable(node),
        ],
        "the global set, then the Conversation's own Repo's, then its companion's — \
         and a Repo it has nothing to do with brings nothing"
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
        "the registry every session downloads into is the one inside the bind"
    );
    assert_eq!(
        reported["wrapper"], "unset",
        "with no sccache resolved there is nothing to wrap rustc in, and a \
         RUSTC_WRAPPER naming a path that is not mounted would break every build"
    );
    assert_eq!(reported["sccache-dir"], "unset");
}

/// And with an sccache the server resolved: it is mounted beside the
/// `verkstead` binary, it is *that* file rather than whatever the machine has
/// installed, and it is what `RUSTC_WRAPPER` names.
///
/// The wrapper is named absolutely on purpose. A session's command may be
/// wrapped in `nix develop`, which puts the project's own shell in front of the
/// sandbox's `PATH` — so a bare `sccache` would resolve to whatever that shell
/// had, or to nothing.
#[tokio::test]
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
        reported["wrapper"], "/verkstead/bin/sccache",
        "absolute, because a project's dev shell decides what `PATH` holds"
    );
    assert_eq!(
        reported["which"], "sccache 0.0.0-the-one-resolved",
        "what a session compiles through is the binary the server resolved"
    );
    assert_eq!(
        reported["sccache-dir"],
        fixture.cache_dir().join("sccache").display().to_string(),
        "and it writes its objects inside the one bind, beside cargo's own"
    );
    assert_eq!(
        reported["size"], "30G",
        "the default where the human has configured no size"
    );
    assert_eq!(reported["cache"], "write");
}

/// The switch is the human's, in `config.yaml` and on the settings page, and it
/// is read as each sandbox is built — so turning it off is a next session with
/// no bind and none of the variables.
#[tokio::test]
async fn a_build_cache_switched_off_is_no_bind_and_no_variables() {
    let fixture = grilling().await;
    fixture.configure("rust_build_cache:\n  enabled: false\n");

    // The server still resolved one, sccache and all: what is being shown is
    // that the switch decides, not that there was nothing to hand out.
    let cache = fixture.cache(true);

    let reported = probe(
        &fixture.sandbox_caching(&cache),
        &format!(
            r#"
            dir {dir} cache
            say cargo-home "${{CARGO_HOME-unset}}"
            say wrapper "${{RUSTC_WRAPPER-unset}}"
            say sccache-dir "${{SCCACHE_DIR-unset}}"
            say size "${{SCCACHE_CACHE_SIZE-unset}}"
            file /verkstead/bin/sccache binary
            "#,
            dir = quoted(&fixture.cache_dir()),
        ),
    );

    assert_eq!(
        reported["cache"], "absent",
        "the switch closes the hole rather than leaving it open and unused"
    );
    assert_eq!(reported["cargo-home"], "unset");
    assert_eq!(reported["wrapper"], "unset");
    assert_eq!(reported["sccache-dir"], "unset");
    assert_eq!(reported["size"], "unset");
    assert_eq!(
        reported["binary"], "absent",
        "and the sccache goes with it: there is nothing left for it to compile into"
    );
}

/// The size is the human's word for one, handed to sccache as it was written —
/// nothing here parses it, because what sccache makes of a size is sccache's to
/// say.
#[tokio::test]
async fn the_size_the_human_configured_is_what_sccache_is_told() {
    let fixture = grilling().await;
    fixture.configure("rust_build_cache:\n  size: 5G\n");

    let reported = probe(
        &fixture.sandbox_caching(&fixture.cache(true)),
        r#"say size "${SCCACHE_CACHE_SIZE-unset}""#,
    );

    assert_eq!(reported["size"], "5G");
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
