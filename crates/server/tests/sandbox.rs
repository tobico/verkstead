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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use verkstead_server::handoffs::Handoffs;
use verkstead_server::sandbox::{Home, Reachable, Sandbox, SandboxConfig, under_dev_shell};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// Where the server this Conversation belongs to is listening — which is what a
/// session inside is told to put its Question Sets to.
const LISTENING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8422);

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

    /// And where the handoff documents go, which is a root under the same
    /// directory — one directory per Conversation, made as its sandbox is built.
    handoffs: Handoffs,

    /// The settings files, in that directory again. Nothing is in them until a
    /// test says so — see [`Grilling::configure_github_token`] and
    /// [`Grilling::configure_git_author`] — which is what an installation nobody
    /// has been to the settings page of looks like.
    settings: Settings,
}

impl Grilling {
    /// The sandbox this Conversation's session would run in, with `extra` as
    /// whatever Sandbox Configuration asked for.
    fn sandbox(&self, extra: Vec<PathBuf>) -> Sandbox {
        self.sandbox_reaching(LISTENING, extra)
    }

    /// The same, for a server that is really listening somewhere — which is what
    /// a session inside has to be able to reach to ask anything.
    fn sandbox_reaching(&self, listening: SocketAddr, extra: Vec<PathBuf>) -> Sandbox {
        Sandbox::for_conversation(
            &self.conversation,
            &self.profile,
            self.home(),
            &Reachable::at(listening),
            &self.skills,
            &self.handoffs,
            // Read here rather than at startup, which is where the server reads
            // them too: a sandbox carries the token and the author that were
            // configured when it was built.
            &self.settings.secrets(),
            &self.settings.config(),
            extra,
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// Write `secrets.yaml` as the settings page would, so that the sandboxes
    /// built after this carry the token.
    fn configure_github_token(&self, yaml: &str) {
        std::fs::write(self.settings.secrets_path(), yaml).unwrap();
    }

    /// And `config.yaml`, which is who those sandboxes commit as.
    fn configure_git_author(&self, yaml: &str) {
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

/// Stand one up.
async fn grilling() -> Grilling {
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

    let skills = Skills::installed(state.path()).expect("this binary carries skills");
    let handoffs = Handoffs::under(state.path());
    let settings = Settings::in_data_dir(state.path());

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
    fixture.configure_git_author("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

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

    let sandbox = fixture.sandbox_reaching(listening, vec![]);

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
        asked[0].set.title,
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
