//! The sandbox a session runs in: a bwrap surface built around one
//! Conversation's worktree, and nothing else.
//!
//! Evolved from `tobico-scripts/bin/sandbox`, which is the working reference —
//! but narrowed where it matters. That script binds the whole of `~/src`
//! read-write, so every session it starts can reach every repository on the
//! machine; here the only checkout inside is the Conversation's own. The
//! filesystem is the boundary, and a boundary with everything behind it is not
//! one.
//!
//! What is inside:
//!
//! - **read-write** — the Conversation's worktree, the Repo's common `.git`
//!   directory, and the Profile's pair at `~/.claude` and `~/.claude.json`
//! - **read-only** — `/nix` and the system paths, and the bundled skills over
//!   `~/.claude/skills`
//! - **tmpfs** — `/tmp`, and everything else in HOME simply absent
//!
//! Credentials are on none of those lists, and neither is who a session commits
//! as. Both arrive in the environment out of the settings files the human filled
//! in — see [`crate::settings`] — GitHub auth as `GH_TOKEN`, and the whole of
//! git's configuration as `GIT_CONFIG_COUNT` and the pairs it counts. So there
//! are no gh files inside a sandbox and no `.gitconfig` either, and no question
//! of which account a session turned out to be running as.
//!
//! The network is not a boundary here: it is shared with the host, whole and
//! unfiltered. An agent has to reach GitHub, the npm registry, the model's API
//! and whatever a build downloads, and an allowlist that has to hold all of that
//! is one nobody will keep honest. What stops a session doing damage is that
//! there is nothing to damage: it can push the branch it is on and write the
//! worktree it is in. A proxy allowlist can come later, in front of this, and
//! the seam for it is that nothing here reads the network's absence.

use std::ffi::OsStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::handoffs::{self, Handoffs};
use crate::settings::{Config, GitAuthor, Secrets};
use crate::skills::{self, Skills};
use crate::store;

/// The system directories a sandbox gets read-only, in the order bwrap is told
/// about them.
///
/// `/nix` is the whole of what makes the box usable — every binary a session
/// runs is under it — and the rest is what a dynamic linker, a certificate store
/// and a shell need to exist at all. `/run/current-system` is NixOS's own, and
/// is what `PATH` below points at.
///
/// One that is not there is skipped rather than refused: `/lib64` is an x86
/// fact, and a machine without `/run/current-system` is one that is not NixOS,
/// which is a thing to notice elsewhere than in a mount table.
///
/// `/lib` and `/lib64` are both here because a shell off NixOS needs both and
/// says so in a way that reads as something else entirely: Ubuntu's `/bin/sh` is
/// `dash`, whose interpreter is under `/lib64` but whose `libc.so.6` is under
/// `/lib`, and `execvp` reports a library it cannot find as `No such file or
/// directory` — naming the binary rather than the library. On a merged-`/usr`
/// distribution neither is a widening: both are symlinks into `/usr`, which is
/// bound above, so this reaches the same files by their other name. It is on
/// NixOS that they are nothing, which is why their absence went unnoticed —
/// there, `/nix` is the whole answer.
const SYSTEM: [&str; 7] = [
    "/nix",
    "/usr",
    "/bin",
    "/lib",
    "/lib64",
    "/etc",
    "/run/current-system",
];

/// What a session's `PATH` is inside.
///
/// The system profile first, then the Nix default profile, then the paths a
/// non-NixOS `/usr` would put things in. Not inherited from the server's own
/// environment: what a session can run should be a fact about the sandbox rather
/// than about however the unit that started the orchestrator happened to be
/// launched.
const PATH: &str = "/run/current-system/sw/bin:/nix/var/nix/profiles/default/bin:/usr/bin:/bin";

/// And what a session's `SHELL` is: the one path the system bind is certain to
/// have a shell at, on NixOS and everywhere else.
const SHELL: &str = "/bin/sh";

/// GitHub over HTTPS, which is the one host a sandbox is given credentials for
/// and the one every SSH remote is rewritten to — see [`Sandbox::git_config`].
///
/// Without a trailing slash, because a credential scope has none and the URL
/// rewrite says its own.
const GITHUB: &str = "https://github.com";

/// The host directory `~` means inside a sandbox.
///
/// A session's HOME is the server's own, at the same path inside — and that is
/// the whole of what this is for. Nothing is read out of it any more: it used to
/// give up two things, what `gh` was authenticated as and who git committed as,
/// and both are now said rather than found — a token and an author in the
/// settings files, handed to the session in its environment. What is left is a
/// path, not a place credentials are collected from.
///
/// Nothing of the directory comes through either. HOME inside is an empty
/// directory with the Profile's pair mounted into it, which is what makes one
/// account's sessions invisible to another's.
#[derive(Debug, Clone)]
pub struct Home {
    /// The directory itself, which is what `~` resolves to inside.
    pub path: PathBuf,
}

impl Home {
    /// The home of whoever is running the server.
    ///
    /// Read from the environment rather than from the passwd database: a service
    /// unit says what HOME is, and that is the answer that should count — under
    /// the packaged unit it is what the module sets, and in a development shell
    /// it is the human's own.
    pub fn of_the_server() -> Option<Home> {
        Some(Home {
            path: PathBuf::from(std::env::var_os("HOME")?),
        })
    }
}

/// Where Verkstead is reachable from inside a sandbox, before the Conversation a
/// session is asking from.
///
/// The one thing a session is given that is not a directory. The network inside a
/// sandbox is the host's own — see this module's own documentation — so what a
/// session dials is what anything else on the machine would.
///
/// An address the server was told to listen on for the sake of the tailnet is
/// still that address. An unspecified one — `0.0.0.0`, `[::]` — is every
/// interface at once, and there is no such thing to put in a URL: the loopback
/// is the one of them certain to answer, and it is the one a session shares.
#[derive(Debug, Clone)]
pub struct Reachable {
    base: String,
}

impl Reachable {
    /// Where a server listening on `listen` can be reached.
    pub fn at(listen: SocketAddr) -> Reachable {
        let host = match listen.ip() {
            IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
            named => named,
        };

        Reachable {
            // Through `SocketAddr` rather than by hand, because that is what puts
            // the brackets round an IPv6 address — `[::1]:8422` is a URL and
            // `::1:8422` is not a number anything could parse.
            base: format!("http://{}", SocketAddr::new(host, listen.port())),
        }
    }

    /// The base URL a session on `conversation_id` is given, which is what makes
    /// its Question Sets that Conversation's.
    ///
    /// Explicit rather than inferred: the CLI derives the project and the branch
    /// from the working directory, and two Conversations against one Repo would
    /// be indistinguishable by either.
    pub(crate) fn asking_from(&self, conversation_id: i64) -> String {
        format!("{}{}/{conversation_id}", self.base, crate::ASKING_FROM)
    }
}

/// Sandbox Configuration: the extra read-write binds a sandbox gets beyond the
/// surface every one of them has.
///
/// Two sets, composed. The global one every sandbox gets, and a per-Repo one so
/// that a repository needing a build cache can say so without every repository
/// getting it. A bind here is a hole in the one boundary a sandbox is, which is
/// why they are configured where the Watched Paths are — in the environment, at
/// installation — rather than anywhere a session or a browser could reach.
///
/// A Repo is named by its *name*, which is the directory's own: the human writes
/// what they call the repository rather than a path they would then have to keep
/// in step with the registration. Two Repos of one name in different places
/// therefore share what is configured for the name, which is a collision the
/// human can see in their own configuration and rename their way out of.
#[derive(Debug, Clone, Default)]
pub struct SandboxConfig {
    /// What every sandbox gets.
    global: Vec<PathBuf>,

    /// And what only the Repo of that name does.
    per_repo: std::collections::BTreeMap<String, Vec<PathBuf>>,
}

impl SandboxConfig {
    /// The binds as configured, checked once at startup.
    ///
    /// Each is either an absolute path — a global bind — or `name=path` for the
    /// Repo called `name`. The two are told apart by the leading `/`, so a Repo
    /// can be called anything without a spelling of it turning into a path.
    ///
    /// A bind that is not there is refused rather than skipped, which is where
    /// this parts company with the settings files: a setting nobody has filled in
    /// is an installation part-way through being set up, and a missing configured
    /// bind is a typo that would otherwise take every session in that repository
    /// down with it, weeks later, with nobody watching. Not created either — a
    /// directory outside the Data Directory is not Verkstead's to make.
    pub fn resolve(binds: &[String]) -> anyhow::Result<SandboxConfig> {
        let mut config = SandboxConfig::default();

        for bind in binds {
            let (repo, path) = read_bind(bind)?;

            if !path.exists() {
                anyhow::bail!(
                    "the sandbox bind {} is not there: a bind Verkstead cannot make is one \
                     every session it applies to would fail to start on",
                    path.display()
                );
            }

            match repo {
                Some(repo) => config.per_repo.entry(repo).or_default().push(path),
                None => config.global.push(path),
            }
        }

        Ok(config)
    }

    /// What a sandbox for the Repo called `repo` binds beyond the decided
    /// surface: the global set, then that Repo's own.
    ///
    /// In that order, and both kept: a Repo's set composes over the global one
    /// rather than replacing it, because the global set is what the machine
    /// gives every session and a repository asking for a cache of its own is not
    /// asking to give the rest up.
    pub fn binds_for(&self, repo: &str) -> Vec<PathBuf> {
        let mut binds = self.global.clone();

        if let Some(own) = self.per_repo.get(repo) {
            binds.extend(own.iter().cloned());
        }

        binds
    }

    /// Every bind configured, for the line the server logs about what it will
    /// hand out.
    pub fn count(&self) -> usize {
        self.global.len() + self.per_repo.values().map(Vec::len).sum::<usize>()
    }
}

/// One `--sandbox-bind`, as the Repo it belongs to — `None` for every Repo — and
/// the directory it binds.
fn read_bind(bind: &str) -> anyhow::Result<(Option<String>, PathBuf)> {
    if bind.starts_with('/') {
        return Ok((None, PathBuf::from(bind)));
    }

    let Some((repo, path)) = bind.split_once('=') else {
        anyhow::bail!(
            "the sandbox bind {bind:?} is neither an absolute path nor `name=path`: \
             a global bind is a directory, and a Repo's own is the name it is \
             registered under and then the directory"
        );
    };

    if repo.is_empty() {
        anyhow::bail!("the sandbox bind {bind:?} names no Repo before its `=`");
    }

    if !path.starts_with('/') {
        anyhow::bail!(
            "the sandbox bind {bind:?} is relative: a bind has to name one directory, \
             whichever directory the server was started in"
        );
    }

    Ok((Some(repo.to_owned()), PathBuf::from(path)))
}

/// One session's sandbox: everything that decides what a command run inside can
/// see.
///
/// Built rather than run — [`Sandbox::command`] hands back a [`Command`] for the
/// caller to spawn, because what a session needs around it (a pty, a Capture
/// being written) is the next stage's business and none of it belongs in a
/// mount table.
#[derive(Debug, Clone)]
pub struct Sandbox {
    /// The Conversation's worktree, read-write, and the directory the command
    /// starts in.
    worktree: PathBuf,

    /// The Repo's common `.git` directory, read-write.
    ///
    /// Read-write because a session commits, and read *common* because a
    /// worktree's own git directory lives inside the repository's rather than
    /// beside the checkout: the `.git` in the worktree is a file pointing back
    /// into this one. The Repo's working files are not bound, so the checkout
    /// the worktree was made from stays invisible — what is shared is the object
    /// database and the refs, which is what a commit and a push need.
    git_dir: PathBuf,

    /// The Profile's claude directory, mounted over `~/.claude`.
    claude_dir: PathBuf,

    /// And its config file, over `~/.claude.json`. The pair is what keeps
    /// accounts apart, so the two travel together or not at all.
    config_file: PathBuf,

    /// The bundled skills, mounted read-only inside the Profile's directory at
    /// `~/.claude/skills` — see [`crate::skills`] for why they are Verkstead's
    /// rather than the account's, and why they are read-only.
    skills: PathBuf,

    /// The Conversation's own directory outside the worktree, read-write, at
    /// [`handoffs::INSIDE`].
    ///
    /// Where the handoff document is written, and the one writable place a
    /// session has that git will never see — see [`crate::handoffs`]. Every
    /// session of the Conversation gets it, not only the grilling one that
    /// writes the handoff: it is somewhere to put what is Verkstead's rather
    /// than the project's, and which session is doing that does not change what
    /// the surface is.
    handoff_dir: PathBuf,

    home: Home,

    /// What `gh` inside authenticates as, or `None` where nothing is configured.
    ///
    /// Taken as the sandbox is built rather than held from startup, so a token
    /// the human rotates applies from the next session — and a session already
    /// running keeps the one it started with, which is the only thing an
    /// environment variable could mean.
    github_token: Option<String>,

    /// And who it commits as, taken at the same moment and for the same reason.
    ///
    /// Either half may be missing, and a missing half is left unsaid rather than
    /// filled in: git's own "tell me who you are" is the failure worth having,
    /// because it says what to go and configure.
    git_author: GitAuthor,

    /// Where the session inside reaches Verkstead: this Conversation's own base
    /// URL, which is what `verkstead ask` puts its Sets to.
    server: String,

    /// The extra read-write binds Sandbox Configuration asks for, global ones
    /// first and the Repo's own after them.
    extra: Vec<PathBuf>,
}

impl Sandbox {
    /// The sandbox a session for `conversation` runs in, under the account
    /// `profile` names.
    ///
    /// Which Profile is the caller's to choose: a Conversation fixes two, and
    /// which of them a session runs under is what the session is for rather than
    /// anything the sandbox can tell.
    ///
    /// `None` where the Conversation has nowhere to run yet — no worktree, or
    /// one git will not own — which is every Conversation before grilling starts
    /// and every one that has been aborted. Its handoff directory is made here
    /// where it is not already there, and failing to make one is the same
    /// answer: a bind with nothing behind it is a sandbox that will not start.
    ///
    /// Git is asked here and the filesystem is written to, so this blocks.
    ///
    /// The parameter list is the surface: every one of these is something the
    /// session gets, and spelling them out is what lets a reader of a call site
    /// see the whole of what one is given. A struct grouping them would hide
    /// exactly the thing worth reading.
    #[allow(clippy::too_many_arguments)]
    pub fn for_conversation(
        conversation: &store::Conversation,
        profile: &store::Profile,
        home: Home,
        reachable: &Reachable,
        skills: &Skills,
        handoffs: &Handoffs,
        secrets: &Secrets,
        config: &Config,
        extra: Vec<PathBuf>,
    ) -> Option<Sandbox> {
        let worktree = conversation.worktree.clone()?;
        let git_dir = crate::worktrees::common_git_dir(&worktree)?;
        let handoff_dir = handoffs.directory(conversation.id)?;

        Some(Sandbox {
            worktree,
            git_dir,
            claude_dir: profile.claude_dir.clone(),
            config_file: profile.config_file.clone(),
            skills: skills.path().to_owned(),
            handoff_dir,
            home,
            github_token: secrets.github_token().map(str::to_owned),
            git_author: config.git_author().clone(),
            server: reachable.asking_from(conversation.id),
            extra,
        })
    }

    /// `argv` as it will be run inside the sandbox.
    ///
    /// The command is not put through a shell: what runs inside is an argument
    /// vector the orchestrator built, and a shell between it and bwrap would be
    /// one more thing to quote for.
    pub fn command<S: AsRef<OsStr>>(&self, argv: &[S]) -> Command {
        let mut bwrap = Command::new("bwrap");

        // Nothing of the server's environment comes through. What the sandbox
        // holds is decided here, and a variable the unit happened to be started
        // with — where the database is, what the server listens on — is not part
        // of that decision.
        bwrap.env_clear();

        bwrap.args([
            // A session outlives nothing: if the orchestrator goes, so does
            // whatever it left running.
            "--die-with-parent",
            // Every namespace, and then the network back — see the module's own
            // documentation for why that one.
            "--unshare-all",
            "--share-net",
            "--hostname",
            "verkstead",
        ]);

        for path in SYSTEM.iter().map(Path::new).filter(|path| path.exists()) {
            bwrap.arg("--ro-bind").arg(path).arg(path);
        }

        // `/proc` and `/dev` are made rather than bound: they are the sandbox's
        // own, which is what makes the unshared pid namespace mean anything.
        bwrap.args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"]);

        // HOME before anything mounted into it: the directory has to be there
        // for the pair to land in, and everything else about it stays absent.
        bwrap.arg("--dir").arg(&self.home.path);

        bwrap
            .arg("--bind")
            .arg(&self.worktree)
            .arg(&self.worktree)
            .arg("--bind")
            .arg(&self.git_dir)
            .arg(&self.git_dir)
            .arg("--bind")
            .arg(&self.claude_dir)
            .arg(self.home.path.join(".claude"))
            .arg("--bind")
            .arg(&self.config_file)
            .arg(self.home.path.join(".claude.json"));

        // After `/tmp` is made, and inside it: the tmpfs above would otherwise
        // land over this and leave the session writing its handoff into memory
        // nothing outside will ever read.
        bwrap
            .arg("--bind")
            .arg(&self.handoff_dir)
            .arg(handoffs::INSIDE);

        // After the Profile's directory, and inside it: a bind is applied in the
        // order it is given, so this one lands over whatever `~/.claude/skills`
        // the account itself keeps. Read-only, because what a session is grilled
        // by is the product's and not a file the session can rewrite mid-run.
        bwrap
            .arg("--ro-bind")
            .arg(&self.skills)
            .arg(self.home.path.join(skills::INSIDE_HOME));

        for extra in &self.extra {
            bwrap.arg("--bind").arg(extra).arg(extra);
        }

        bwrap
            .arg("--chdir")
            .arg(&self.worktree)
            .arg("--setenv")
            .arg("HOME")
            .arg(&self.home.path)
            .arg("--setenv")
            .arg("PATH")
            .arg(PATH)
            // Which shell is inside, for the same reason `PATH` is said here:
            // the environment is cleared, so a tool that shells out reaches for
            // whatever this holds — and with nothing in it, `script` would fall
            // back to whatever login shell the passwd file gives the user the
            // server happens to run as.
            .arg("--setenv")
            .arg("SHELL")
            .arg(SHELL)
            // What makes a session's Question Sets its own Conversation's. The
            // variable the bundled CLI reads, scoped to one Conversation, so
            // nothing is inferred from the project or the branch — two
            // Conversations against one Repo would be indistinguishable by
            // either.
            .arg("--setenv")
            .arg("VERKSTEAD_SERVER")
            .arg(&self.server);

        // What `gh` inside authenticates as, which it reads from here without
        // being told to and without a file anywhere. Set only where there is one
        // to set: `GH_TOKEN` present and empty is a login `gh` fails on obscurely
        // where its absence is a login it says plainly it does not have.
        if let Some(token) = &self.github_token {
            bwrap.arg("--setenv").arg("GH_TOKEN").arg(token);
        }

        // And the whole of git's configuration, in the environment for the same
        // reason: there is no file inside for it to be in.
        let git_config = self.git_config();

        for (n, (key, value)) in git_config.iter().enumerate() {
            bwrap
                .arg("--setenv")
                .arg(format!("GIT_CONFIG_KEY_{n}"))
                .arg(key)
                .arg("--setenv")
                .arg(format!("GIT_CONFIG_VALUE_{n}"))
                .arg(value);
        }

        bwrap
            .arg("--setenv")
            .arg("GIT_CONFIG_COUNT")
            .arg(git_config.len().to_string())
            // And nothing git cannot answer for itself is asked of anybody.
            // Nobody is at this terminal: a push with no usable credentials has
            // to come back saying so, where a prompt for a username would be a
            // session sitting on a pty until something noticed.
            .arg("--setenv")
            .arg("GIT_TERMINAL_PROMPT")
            .arg("0");

        bwrap.args(argv);

        bwrap
    }

    /// Every `key = value` git is configured with inside, in the order the
    /// environment will number them.
    ///
    /// Three things, and the last two are the same thing said twice over: who
    /// the commits are by, how a push proves who it is, and which URLs that
    /// proof is any use for.
    ///
    /// The credential helper is `gh`'s own, which answers out of `GH_TOKEN` — so
    /// a `git push` over HTTPS authenticates as whoever the settings file says,
    /// with nothing stored anywhere and nothing to log in to. It is scoped to
    /// `https://github.com` rather than left open: a helper on every host is one
    /// asked about hosts it has no business being asked about.
    ///
    /// And the rewrites are what make that reachable at all. A repository cloned
    /// over SSH has an SSH remote, and there are no keys inside a sandbox for it
    /// to offer — so a push would fail on a missing key rather than fall back to
    /// anything. `insteadOf` turns the two spellings of a GitHub SSH remote into
    /// the HTTPS one as git resolves the URL, leaving what the repository has
    /// written down untouched: a session pushes, and `.git/config` still says
    /// what the human cloned.
    fn git_config(&self) -> Vec<(String, String)> {
        let mut config = Vec::new();

        // Each half on its own: a name configured and no address is a name
        // configured, and git says for itself what is still missing.
        if let Some(name) = self.git_author.name() {
            config.push(("user.name".to_owned(), name.to_owned()));
        }

        if let Some(email) = self.git_author.email() {
            config.push(("user.email".to_owned(), email.to_owned()));
        }

        config.push((
            format!("credential.{GITHUB}.helper"),
            // The `!` is what makes git run it as a command rather than look for
            // a `git-credential-` binary of that name.
            "!gh auth git-credential".to_owned(),
        ));

        // Both spellings of the same remote, as two values of the one
        // multi-valued key — which is what repeating it in the environment
        // means, the counted pairs being read as one more configuration file.
        for ssh in ["git@github.com:", "ssh://git@github.com/"] {
            config.push((format!("url.{GITHUB}/.insteadOf"), ssh.to_owned()));
        }

        config
    }
}

/// `argv` wrapped in `nix develop` where the worktree's flake actually provides
/// a shell, and `argv` as it stands where it does not.
///
/// A `flake.nix` alone is not enough, which is the whole reason this asks rather
/// than looking for the file: `nix develop` falls through a list of attributes
/// and errors out when the flake defines none of them, so a repository with a
/// flake that only builds a package would have every session in it fail to
/// start.
///
/// Asked on the host, before the sandbox exists, because the answer decides what
/// the sandbox is told to run. It is a `nix eval` per attribute and it is asked
/// once per session.
pub fn under_dev_shell(worktree: &Path, argv: &[String]) -> Vec<String> {
    if !dev_shell(worktree) {
        return argv.to_vec();
    }

    let mut wrapped = vec![
        "nix".to_owned(),
        "develop".to_owned(),
        "--command".to_owned(),
    ];
    wrapped.extend_from_slice(argv);
    wrapped
}

/// Whether `nix develop` in `worktree` would find a shell to enter.
fn dev_shell(worktree: &Path) -> bool {
    if !worktree.join("flake.nix").is_file() {
        return false;
    }

    let Some(system) = nix(
        worktree,
        &[
            "eval",
            "--impure",
            "--raw",
            "--expr",
            "builtins.currentSystem",
        ],
    ) else {
        return false;
    };

    let system = system.trim();

    // The attributes `nix develop` itself falls through, in its own order.
    [
        format!("devShells.{system}.default"),
        format!("devShell.{system}"),
        format!("packages.{system}.default"),
        format!("defaultPackage.{system}"),
    ]
    .iter()
    .any(|attr| {
        // `--apply` forces the attribute enough to say whether it is there,
        // without building whatever it evaluates to.
        nix(
            worktree,
            &["eval", &format!(".#{attr}"), "--apply", "x: true"],
        )
        .is_some()
    })
}

/// Run nix in `dir` and take its stdout, or `None` if it failed.
///
/// The experimental features are named rather than assumed: this runs as
/// whichever user the server does, and a flake command that works in the human's
/// shell and not in the service's would be a repository that mysteriously stops
/// having a dev shell.
fn nix(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("nix")
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}
