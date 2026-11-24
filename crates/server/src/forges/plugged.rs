//! A Forge a plugin's component answers for: the implementation of
//! [`Forge`](crate::forges::Forge) every Forge is from now on, GitHub's own
//! among them (ADR-0030, *Forge*).
//!
//! **One call, one instance.** The component is compiled once, when the plugin
//! loads, and started afresh for each call with the Author's token set in the
//! environment variable its manifest's `secrets` capability names — see
//! [`crate::plugins::component::Again::holding`]. The token is read out of
//! `secrets.yaml` at the moment of the call, so one saved through the settings
//! page reaches the next call without a restart, and nothing of it is held
//! here between calls or ever answered back.
//!
//! **On threads of this component's own.** Every operation of the trait
//! blocks and the component's exports are async, so the calls are carried over
//! to where they can be awaited and the answer carried back — the same
//! arrangement a plugin's harness is run under, and for the same reason, with
//! a handful of them at once rather than one. See [`Across`].
//!
//! **What stays the host's**: which Author's token to read, what Verkstead's
//! own git authenticates as, and whether the tool a session is lent carries an
//! extension. None of the three is a question about the Forge's API — the
//! first two are about this installation's own files, and the third is about a
//! session's home, which the host is the only one holding. The last is read
//! off the directory the extension would be in rather than asked of the tool
//! — see [`extensions`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::Duration;

use wasmtime::Store;

use crate::forges::{
    Account, AsWhom, Check, Checked, Comment, Credential, Details, Forge, Gist, Landing, Mergeable,
    Numbered, Opening, Scopes, Stands, Suite, Trouble, awaited,
};
use crate::plugins::component::worlds::category::forge as world;
use crate::plugins::component::{Again, Host, Loaded, Redirect, TICK};
use crate::plugins::manifest::forge::Tool;
use crate::settings::Settings;
use crate::store;
use crate::targets::NamedPullRequest;
use crate::unseen::Unseen;

/// How long one call into a Forge's component may take before it is stopped.
///
/// Generous, because most of it is somebody else's network: GitHub answering
/// slowly is a wait rather than a fault. What this is in front of is a
/// component that has stopped answering at all, which would otherwise hold
/// one of this Forge's threads for ever.
const CALLING: Duration = Duration::from_secs(60);

/// A Forge answered by a plugin's component.
#[derive(Clone)]
pub(crate) struct Plugged {
    /// The uuid of the plugin whose Forge this is, which is what an Author's
    /// token is held under — one token per Forge per Author (ADR-0030).
    forge: String,

    /// The command-line tool it lends a session, where it declares one: what
    /// [`Forge::tool_extension`] asks about.
    tool: Option<Tool>,

    /// Where the Author's token is read from, or `None` where this Forge has
    /// no settings behind it — a router watching nothing has no Data
    /// Directory to read one out of.
    settings: Option<Settings>,

    /// The uuid of the Author the Repo being asked about recorded, whose token
    /// is read. `None` is a Repo that records none, which is the one Author's
    /// where there is only one.
    author: Option<String>,

    /// What stands in for this installation's own files, where a suite put a
    /// stand-in where the Forge's API goes — `None` on every running
    /// Verkstead. See [`StandingIn`].
    standing_in: Option<StandingIn>,

    across: Arc<Across>,
}

/// What a Forge asked of a stand-in is given in place of this installation's
/// own files: the account a Repo whose remote names no host is asked about
/// under, and the token every call is made with.
///
/// **A seam for the suites, the way [`Redirect`] is, and nothing a plugin or a
/// human can set.** A Verkstead under test registers its repositories as
/// checkouts on this machine, and a checkout's `origin` is a path or nothing
/// at all — which is no repository on any Forge, and what a Forge answers
/// about one is [`Trouble::NoRemote`]. So a stand-in's Forge asks about it
/// under the account named here, as the repository the checkout's own
/// directory is called: the suite's `verkstead` becomes
/// `https://github.com/tobico/verkstead.git`, which is the remote it stands
/// for, and a stand-in telling two repositories apart reads the name out of
/// what it was asked. A remote that *does* name a host is left exactly as it
/// is — see [`crate::matching::host_of`].
///
/// And the token, because a Forge with nothing behind it to read one out of
/// asks as nobody, and a component asked as nobody answers
/// [`Trouble::NotLoggedIn`] of its own accord. A suite that wants the real
/// reading — as the Repo's Author, and not at all where they hold no token —
/// hands the Forge its Data Directory's settings instead, and those win.
#[derive(Debug, Clone)]
pub(crate) struct StandingIn {
    /// Where a Repo with a pathless remote is taken to live, with no trailing
    /// slash: `https://github.com/tobico`.
    pub(crate) under: String,

    /// The token every call is made with, where there are no settings saying
    /// whose it should be.
    pub(crate) token: String,
}

impl Plugged {
    /// The Forge `forge` provides, answered by the component `again` starts,
    /// reading its tokens out of `settings`.
    pub(crate) fn new(
        forge: &str,
        tool: Option<Tool>,
        settings: Option<Settings>,
        again: Again,
    ) -> Plugged {
        Plugged {
            forge: forge.to_owned(),
            tool,
            settings,
            author: None,
            standing_in: None,
            across: Arc::new(Across::standing(again)),
        }
    }

    /// And the same asked of a stand-in rather than of the Forge's own API —
    /// see [`StandingIn`].
    pub(crate) fn standing_in(self, standing_in: Option<StandingIn>) -> Plugged {
        Plugged {
            standing_in,
            ..self
        }
    }

    /// And the Forge of a plugin whose component did not load, every call
    /// answering `why`.
    ///
    /// **A start is not stopped over it**, the way a built-in that will not
    /// load does not stop one: what the human gets is the reason, on the
    /// Timeline and beside whatever press asked, rather than a workbench that
    /// would not come up.
    pub(crate) fn unloaded(forge: &str, why: String) -> Plugged {
        Plugged {
            forge: forge.to_owned(),
            tool: None,
            settings: None,
            author: None,
            standing_in: None,
            across: Arc::new(Across::none(why)),
        }
    }

    /// Who to authenticate as, read now — see [`crate::forges::whom`], and
    /// [`StandingIn`], whose token is whose a Forge with no settings behind it
    /// asks as under a stand-in.
    fn whom(&self) -> AsWhom {
        match (&self.settings, &self.standing_in) {
            (None, Some(standing_in)) => AsWhom::Token(standing_in.token.clone()),
            _ => crate::forges::whom(self.settings.as_ref(), self.author.as_deref(), &self.forge),
        }
    }

    /// The token to ask with, or why there is to be none.
    ///
    /// **A Forge with no settings behind it has none**, and is asked all the
    /// same: a component with nothing in its environment answers
    /// `not-logged-in` of its own accord, which is the honest answer where
    /// there is nowhere to have read a token from.
    fn token(&self) -> Result<Option<String>, Trouble> {
        match self.whom() {
            AsWhom::Host => Ok(None),
            AsWhom::Token(token) => Ok(Some(token)),
            AsWhom::Untokened => Err(Trouble::NotLoggedIn),
            AsWhom::Awaited => Err(Trouble::Ours(awaited())),
        }
    }

    /// The `origin` of the Repo at `repo`, which is the whole of what says
    /// which repository on which host the Forge is being asked about — and
    /// which is read here rather than in the component, a component having no
    /// handle on anybody's checkout.
    ///
    /// A Repo with no remote is handed none, and what the Forge says about
    /// that is [`Trouble::NoRemote`] — unless a stand-in is standing where the
    /// Forge's API goes, which is where a checkout's path of a remote is read
    /// as a repository all the same. See [`StandingIn`].
    fn remote(&self, repo: &Path) -> String {
        let origin = crate::repos::origin(repo).unwrap_or_default();

        match &self.standing_in {
            Some(standing_in) if crate::matching::host_of(&origin).is_none() => {
                let named = repo
                    .file_name()
                    .map(|named| named.to_string_lossy().into_owned())
                    .unwrap_or_default();

                format!("{}/{named}.git", standing_in.under)
            }
            _ => origin,
        }
    }

    /// Ask `asking` of the component as this Forge's Author, the token set in
    /// its environment for that one call.
    fn asked<T, F>(&self, asking: F) -> Result<T, Trouble>
    where
        T: Send + 'static,
        F: AsyncFnOnce(
                &mut Store<Host>,
                &world::Forge,
            ) -> wasmtime::Result<Result<T, world::Trouble>>
            + Send
            + 'static,
    {
        self.across.asked(self.token()?, asking)
    }

    /// And the same as `token` and no other, which is what a write the caller
    /// has already read a token for is made as — see
    /// [`crate::publishing`].
    fn asked_as<T, F>(&self, token: &str, asking: F) -> Result<T, Trouble>
    where
        T: Send + 'static,
        F: AsyncFnOnce(
                &mut Store<Host>,
                &world::Forge,
            ) -> wasmtime::Result<Result<T, world::Trouble>>
            + Send
            + 'static,
    {
        self.across.asked(Some(token.to_owned()), asking)
    }
}

/// Never the token: a `Plugged` in a log line says which Forge it is and whose
/// Author it asks as, and no more.
impl std::fmt::Debug for Plugged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plugged")
            .field("forge", &self.forge)
            .field("author", &self.author)
            .finish_non_exhaustive()
    }
}

impl Forge for Plugged {
    fn as_author(&self, recorded: Option<&str>) -> Arc<dyn Forge> {
        Arc::new(Plugged {
            author: recorded.map(str::to_owned),
            ..self.clone()
        })
    }

    fn credential(&self) -> Credential {
        Credential(self.whom())
    }

    fn tokened(&self) -> bool {
        matches!(self.token(), Ok(Some(_)))
    }

    fn verify(&self, token: &str) -> Result<Account, Trouble> {
        let account = self.asked_as(token, async move |store, forge| {
            forge.call_verify(store).await
        })?;

        Ok(Account {
            login: account.login,
            scopes: match account.scopes {
                world::Scopes::Named(named) => Scopes::Named(named),
                world::Scopes::Unsaid => Scopes::Unsaid,
            },
        })
    }

    fn pull_request(&self, repo: &Path, branch: &str) -> Result<store::PullRequest, Trouble> {
        let (remote, branch) = (self.remote(repo), branch.to_owned());
        let found = self.asked(async move |store, forge| {
            forge.call_pull_request(store, &remote, &branch).await
        })?;

        Ok(store::PullRequest {
            number: found.number,
            title: found.title,
            url: found.url,
            head: found.head,
            base: found.base,
            // Unnamed: which repository this was asked in is what the caller
            // already knows — see [`store::PullRequest`].
            repo: None,
        })
    }

    fn pull_request_numbered(&self, repo: &Path, number: i64) -> Result<Numbered, Trouble> {
        let remote = self.remote(repo);

        Ok(numbered(self.asked(async move |store, forge| {
            forge
                .call_pull_request_numbered(store, &remote, number)
                .await
        })?))
    }

    fn open_pull_request(&self, repo: &Path, opening: &Opening) -> Result<Numbered, Trouble> {
        let remote = self.remote(repo);
        let opening = world::Opening {
            head: opening.head.clone(),
            base: opening.base.clone(),
            title: opening.title.clone(),
            body: opening.body.clone(),
        };

        Ok(numbered(self.asked(async move |store, forge| {
            forge.call_open_pull_request(store, &remote, &opening).await
        })?))
    }

    fn open_pull_requests(&self, repo: &Path) -> Result<Vec<Numbered>, Trouble> {
        let remote = self.remote(repo);
        let open = self
            .asked(async move |store, forge| forge.call_open_pull_requests(store, &remote).await)?;

        Ok(open.into_iter().map(numbered).collect())
    }

    fn details(&self, repo: &Path, number: i64) -> Result<Details, Trouble> {
        let remote = self.remote(repo);
        let read =
            self.asked(async move |store, forge| forge.call_details(store, &remote, number).await)?;

        let checks: Vec<Check> = read.checks.into_iter().map(check).collect();

        Ok(Details {
            pane: verkstead_render::pull_request_details(
                read.commits
                    .into_iter()
                    .map(|commit| verkstead_render::PullRequestCommit {
                        sha: commit.sha,
                        subject: commit.subject,
                    })
                    .collect(),
                read.comments
                    .into_iter()
                    .map(|said| verkstead_render::Comment {
                        author: said.author,
                        at: said.at,
                        markdown: said.markdown,
                    })
                    .collect(),
                checks.iter().map(drawn).collect(),
            ),
            checks,
            landing: landing(read.landing),
        })
    }

    fn checks(&self, repo: &Path, number: i64) -> Result<Suite, Trouble> {
        let remote = self.remote(repo);
        let suite =
            self.asked(async move |store, forge| forge.call_checks(store, &remote, number).await)?;

        Ok(Suite {
            head: suite.head,
            checks: suite.checks.into_iter().map(check).collect(),
            mergeable: mergeable(suite.mergeable),
        })
    }

    fn landing(&self, repo: &Path, number: i64) -> Result<Landing, Trouble> {
        let remote = self.remote(repo);

        Ok(landing(self.asked(async move |store, forge| {
            forge.call_landing(store, &remote, number).await
        })?))
    }

    fn comments(&self, repo: &Path, number: i64) -> Result<Vec<Comment>, Trouble> {
        let remote = self.remote(repo);
        let said = self
            .asked(async move |store, forge| forge.call_comments(store, &remote, number).await)?;

        Ok(said
            .into_iter()
            .map(|said| Comment {
                which: said.which,
                author: said.author,
                at: said.at,
                about: said.about,
                markdown: said.markdown,
            })
            .collect())
    }

    fn comment(
        &self,
        repo: &Path,
        token: &str,
        number: i64,
        body: &str,
    ) -> Result<String, Trouble> {
        let (remote, body) = (self.remote(repo), body.to_owned());

        self.asked_as(token, async move |store, forge| {
            forge.call_comment(store, &remote, number, &body).await
        })
    }

    fn reply(&self, repo: &Path, number: i64, body: &str) -> Result<String, Trouble> {
        let (remote, body) = (self.remote(repo), body.to_owned());

        self.asked(async move |store, forge| {
            forge.call_comment(store, &remote, number, &body).await
        })
    }

    fn create_gist(
        &self,
        token: &str,
        description: &str,
        name: &str,
        content: &str,
    ) -> Result<Gist, Trouble> {
        let (description, name, content) =
            (description.to_owned(), name.to_owned(), content.to_owned());

        let published = self.asked_as(token, async move |store, forge| {
            forge
                .call_publish(store, &description, &name, &content)
                .await
        })?;

        Ok(Gist {
            id: published.id,
            url: published.url,
            push: published.push,
        })
    }

    fn delete_gist(&self, token: &str, id: &str) -> Result<(), Trouble> {
        let id = id.to_owned();

        self.asked_as(token, async move |store, forge| {
            forge.call_unpublish(store, &id).await
        })
    }

    /// Make the repository, then write `origin` and push: the Forge holds the
    /// repository and the host holds the checkout, so the remote it answers
    /// with is written here.
    ///
    /// Both halves or neither to the caller — [`crate::repos::create`] takes
    /// a failure as *made here only* and says so — so a remote written and a
    /// push that would not go through is a failure, with the remote left
    /// where a `git push` by hand finishes it.
    fn create_repository(&self, repo: &Path, name: &str) -> Result<(), Trouble> {
        let name = name.to_owned();
        let made =
            self.asked(async move |store, forge| forge.call_create_repository(store, &name).await)?;

        pushed(repo, &made.push, &self.credential())
    }

    fn tool_extension(
        &self,
        name: &str,
        environment: &[(String, OsString)],
    ) -> Result<(), Trouble> {
        let Some(tool) = &self.tool else {
            // A Forge that lends no tool has no extension to be missing, so
            // there is nothing here to stop anything over.
            return Ok(());
        };

        let where_it_would_be = extensions(tool, name, environment);

        if where_it_would_be.is_empty() {
            return Err(Trouble::Ours(format!(
                "Verkstead can't tell whether a session would find `{} {name}`: \
                 it doesn't know which home one runs under.",
                tool.name
            )));
        }

        // Verkstead's own reading rather than anything the Forge said, so the
        // host's kind: a sentence of Verkstead's own, which is what
        // [`Trouble::Ours`] is for.
        match where_it_would_be.iter().any(|dir| dir.is_dir()) {
            true => Ok(()),
            false => Err(Trouble::Ours(format!(
                "There's no `{} {name}` installed under the home a session here runs with.",
                tool.name
            ))),
        }
    }

    fn lacks_scope(&self, trouble: &Trouble) -> bool {
        let Trouble::Refused(said) = trouble else {
            return false;
        };
        let said = said.clone();

        self.across
            .asked(None, async move |store, forge| {
                forge.call_lacks_scope(store, &said).await.map(Ok)
            })
            .unwrap_or(false)
    }

    fn target_in(&self, prose: &str) -> Option<NamedPullRequest> {
        let prose = prose.to_owned();

        self.across
            .asked(None, async move |store, forge| {
                forge.call_target_in(store, &prose).await.map(Ok)
            })
            .ok()
            .flatten()
            .map(target)
    }

    fn target_named(&self, target_: &str) -> Option<NamedPullRequest> {
        let named = target_.to_owned();

        self.across
            .asked(None, async move |store, forge| {
                forge.call_target_named(store, &named).await.map(Ok)
            })
            .ok()
            .flatten()
            .map(target)
    }

    fn repository_in(&self, url: &str) -> Option<String> {
        let url = url.to_owned();

        self.across
            .asked(None, async move |store, forge| {
                forge.call_repository_in(store, &url).await.map(Ok)
            })
            .ok()
            .flatten()
    }

    fn repository_of(&self, origin: &str) -> Option<String> {
        let origin = origin.to_owned();

        self.across
            .asked(None, async move |store, forge| {
                forge.call_repository_of(store, &origin).await.map(Ok)
            })
            .ok()
            .flatten()
    }
}

/// Where the extension `name` of `tool` would be, under the home a session
/// gets — every directory it could be, since which one it is depends on the
/// platform and on what the environment says.
///
/// **Read rather than run** (Set 1878, Q4). An extension of a command-line
/// tool is a directory inside the tool's own data directory, and that
/// directory is under the home the tool is run with — so whether a session
/// would find one is a question about a directory, and the server answers it
/// by looking. Running the tool to ask was what `gh stack --help` did, and the
/// server runs no `gh` any more.
///
/// The convention is `gh`'s, which is every tool lent as a Forge's so far:
/// `<data>/<tool>/extensions/<tool>-<name>`, where `<data>` is
/// `$XDG_DATA_HOME` or `~/.local/share`. On Windows the data directory is
/// `%LOCALAPPDATA%` and the directory inside it is `GitHub CLI` rather than
/// the tool's own name, which is the one thing here that is gh's spelling and
/// not a convention — so both are looked at, and a tool that keeps its
/// extensions in neither is one whose Forge would want to say where.
///
/// Empty where the environment names no home at all, which is a question that
/// cannot be answered rather than an extension that is missing.
fn extensions(tool: &Tool, name: &str, environment: &[(String, OsString)]) -> Vec<PathBuf> {
    /// What gh calls its data directory on Windows.
    const ON_WINDOWS: &str = "GitHub CLI";

    let named = |which: &str| {
        environment
            .iter()
            .find(|(said, _)| said == which)
            .map(|(_, value)| PathBuf::from(value))
            .filter(|path| !path.as_os_str().is_empty())
    };

    let mut data: Vec<PathBuf> = Vec::new();
    data.extend(named("XDG_DATA_HOME"));
    data.extend(named("HOME").map(|home| home.join(".local").join("share")));
    data.extend(named("LOCALAPPDATA"));

    let under = format!("{}-{name}", tool.name);

    data.iter()
        .flat_map(|data| [data.join(&tool.name), data.join(ON_WINDOWS)])
        .map(|dir| dir.join("extensions").join(&under))
        .collect()
}

/// Write `push` as `origin` on the repository at `repo` and push its branch
/// there, as `credential`.
///
/// `git push -u`, so the branch is tracking what it was pushed to: what the
/// human gets is the repository they would have got by hand.
fn pushed(repo: &Path, push: &str, credential: &Credential) -> Result<(), Trouble> {
    for (what, args) in [
        ("write the remote", vec!["remote", "add", "origin", push]),
        ("push the branch", vec!["push", "-u", "origin", "HEAD"]),
    ] {
        let mut command = Command::new("git");
        credential.on(&mut command);

        let output = command
            .current_dir(repo)
            .args(&args)
            .unseen()
            .stdin(Stdio::null())
            .output()
            .map_err(|error| Trouble::Ours(format!("git couldn't be run: {error}.")))?;

        if !output.status.success() {
            return Err(Trouble::Refused(format!(
                "The repository was made, and git couldn't {what}: {}",
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .unwrap_or("it would not say why")
            )));
        }
    }

    Ok(())
}

/// One pull request as the world answers it.
fn numbered(numbered: world::Numbered) -> Numbered {
    Numbered {
        number: numbered.number,
        title: numbered.title,
        url: numbered.url,
        head: numbered.head,
        base: numbered.base,
        fork: numbered.fork,
        open: numbered.open,
    }
}

fn check(check: world::Check) -> Check {
    Check {
        name: check.name,
        how: match check.how {
            world::Checked::Passed => Checked::Passed,
            world::Checked::Failed => Checked::Failed,
            world::Checked::Running => Checked::Running,
        },
        link: check.link,
    }
}

/// One check as the details pane draws it.
fn drawn(check: &Check) -> verkstead_render::PullRequestCheck {
    verkstead_render::PullRequestCheck {
        name: check.name.clone(),
        how: match check.how {
            Checked::Passed => verkstead_render::Checked::Passed,
            Checked::Running => verkstead_render::Checked::Running,
            Checked::Failed => verkstead_render::Checked::Failed,
        },
        link: check.link.clone(),
    }
}

fn landing(landing: world::Landing) -> Landing {
    Landing {
        mergeable: mergeable(landing.mergeable),
        stands: match landing.stands {
            world::Stands::Open => Stands::Open,
            world::Stands::Merged => Stands::Merged,
            world::Stands::Closed => Stands::Closed,
            world::Stands::Unknown => Stands::Unknown,
        },
    }
}

fn mergeable(mergeable: world::Mergeable) -> Mergeable {
    match mergeable {
        world::Mergeable::Cleanly => Mergeable::Cleanly,
        world::Mergeable::Conflicting => Mergeable::Conflicting,
        world::Mergeable::Unknown => Mergeable::Unknown,
    }
}

fn target(named: world::NamedPullRequest) -> NamedPullRequest {
    NamedPullRequest {
        number: named.number,
        repository: named.repository,
    }
}

/// The error kinds of the interface, as the world names them. `Ours` is not
/// among them: a failure in Verkstead rather than at the Forge is the host's,
/// and is worded where it happens.
fn troubled(trouble: world::Trouble) -> Trouble {
    match trouble {
        world::Trouble::NoPullRequest => Trouble::NoPullRequest,
        world::Trouble::NotLoggedIn => Trouble::NotLoggedIn,
        world::Trouble::NoRemote => Trouble::NoRemote,
        world::Trouble::Refused(said) => Trouble::Refused(said),
    }
}

/// A Forge's component, asked on threads of its own, each with a runtime of
/// its own.
///
/// **The threads are what make the trait's answers blocking.** Every caller
/// asks from `spawn_blocking` and the component's exports are async, so the
/// call is carried across to one of these, awaited on its runtime, and the
/// answer carried back — which is also the one arrangement that cannot wake up
/// inside somebody else's runtime.
struct Across {
    /// Where a call is sent, or `None` where the component did not load —
    /// which is what [`Across::why`] says.
    calls: Option<mpsc::Sender<Call>>,

    /// Why there is no component to ask, where there is none.
    why: String,
}

/// One call into the component: what to do with what starts it. It answers
/// through a channel of its own.
type Call = Box<dyn FnOnce(&Again, &tokio::runtime::Runtime) + Send>;

/// How many calls into a Forge may be in flight at once.
///
/// **More than one, because one would be a queue behind the slowest.** Most of
/// a call is somebody else's network, and a wrap-up watching three pull
/// requests asks about all three: a Forge that answered them one at a time
/// would make a poll as long as the sum of them, and a single call that hung
/// would hold up the Repos page's reading of which repository a remote is.
/// Bounded, because each call is a wasm instance of its own and an unbounded
/// number of them is an unbounded amount of memory.
const AT_ONCE: usize = 4;

impl Across {
    fn standing(again: Again) -> Across {
        let (calls, taken) = mpsc::channel::<Call>();

        // One receiver between the threads, each holding it only while it
        // waits for the next call: whichever thread is free takes it, and the
        // others go on waiting.
        let taken = Arc::new(std::sync::Mutex::new(taken));

        for _ in 0..AT_ONCE {
            let (again, taken) = (again.clone(), taken.clone());

            let started = std::thread::Builder::new()
                .name("plugin-forge".to_owned())
                .spawn(move || {
                    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    else {
                        return;
                    };

                    loop {
                        // The lock is held for the wait and let go before the
                        // call, so a call that takes a minute is one thread's
                        // and not every thread's.
                        let call = {
                            let taken = taken.lock().expect("nothing panics holding it");
                            match taken.recv() {
                                Ok(call) => call,
                                // Every handle on the channel has gone, which
                                // is this Forge being dropped.
                                Err(_) => return,
                            }
                        };

                        call(&again, &runtime);
                    }
                });

            if let Err(error) = started {
                tracing::error!(error = ?error, "a forge plugin's component couldn't be started");
            }
        }

        Across {
            calls: Some(calls),
            why: String::new(),
        }
    }

    /// And no component at all, every call answering `why`: a Forge whose
    /// plugin would not load is one nothing can be asked of, which is an
    /// answer for the human rather than a start that fails.
    fn none(why: String) -> Across {
        Across { calls: None, why }
    }

    /// `asking` of the component, started with `secret` in its environment for
    /// this one call — see [`crate::plugins::component::Again::holding`].
    fn asked<T, F>(&self, secret: Option<String>, asking: F) -> Result<T, Trouble>
    where
        T: Send + 'static,
        F: AsyncFnOnce(
                &mut Store<Host>,
                &world::Forge,
            ) -> wasmtime::Result<Result<T, world::Trouble>>
            + Send
            + 'static,
    {
        let Some(calls) = &self.calls else {
            return Err(Trouble::Ours(self.why.clone()));
        };

        let (answered, answer) = mpsc::channel();

        calls
            .send(Box::new(move |again, runtime| {
                let _ = answered.send(runtime.block_on(calling(again, secret, asking)));
            }))
            .map_err(|_| Trouble::Ours("The forge's component isn't running.".to_owned()))?;

        // Twice the call's own time: enough for one that ran out of it and the
        // next instance started after it, and never a wait without end.
        answer
            .recv_timeout(CALLING * 2)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => Trouble::Ours(format!(
                    "The forge took longer than {} seconds to answer.",
                    (CALLING * 2).as_secs()
                )),
                mpsc::RecvTimeoutError::Disconnected => {
                    Trouble::Ours("The forge's component stopped.".to_owned())
                }
            })?
    }
}

/// One instance of the component, started holding `secret`, asked `asking`,
/// and dropped — so the secret is in its environment for this call and no
/// other.
async fn calling<T, F>(again: &Again, secret: Option<String>, asking: F) -> Result<T, Trouble>
where
    F: AsyncFnOnce(&mut Store<Host>, &world::Forge) -> wasmtime::Result<Result<T, world::Trouble>>,
{
    let mut loaded: Loaded = match &secret {
        Some(secret) => again.holding(secret).await,
        None => again.afresh().await,
    }
    .map_err(|trouble| Trouble::Ours(format!("The forge's component didn't start: {trouble}")))?;

    let exports = world::Forge::new(&mut loaded.store, &loaded.instance).map_err(|error| {
        Trouble::Ours(format!(
            "The forge's component doesn't export what a forge needs: {}.",
            crate::plugins::component::worded(&error)
        ))
    })?;

    loaded
        .store
        .set_epoch_deadline(CALLING.div_duration_f64(TICK).ceil() as u64);

    match asking(&mut loaded.store, &exports).await {
        Ok(Ok(answered)) => Ok(answered),
        Ok(Err(trouble)) => Err(troubled(trouble)),
        Err(error) => Err(Trouble::Ours(format!(
            "The forge's component stopped while it was answering: {}.",
            crate::plugins::component::worded(&error)
        ))),
    }
}

/// The built-in GitHub's Forge, its component loaded out of this binary and
/// its tokens read out of `settings`: what every reach into GitHub goes
/// through on a running Verkstead.
///
/// **Loaded the first time anything asks it something** — see [`Lazily`]. A
/// router is built where nothing may be awaited and most routers are never
/// asked a thing, so the component is compiled and started at the first
/// question rather than at the start that might never ask one.
///
/// A component that will not load is a Forge that answers the reason — see
/// [`Plugged::unloaded`] — rather than a start that fails.
pub(crate) fn github(settings: Settings, pool: &sqlx::SqlitePool) -> Arc<dyn Forge> {
    standing_in_for_github(Some(settings), Some(pool), Redirect::none(), None)
}

/// And the same asked of a stand-in rather than of GitHub: the component
/// reaches `redirect` where it asks for `api.github.com`, and a Repo whose
/// remote names no host is asked about under `standing_in`.
///
/// **A seam for the suites** — see [`StandingIn`] and [`Redirect`], and
/// [`crate::stand_in`], which is where a suite stands one up.
pub(crate) fn standing_in_for_github(
    settings: Option<Settings>,
    pool: Option<&sqlx::SqlitePool>,
    redirect: Redirect,
    standing_in: Option<StandingIn>,
) -> Arc<dyn Forge> {
    let uuid = crate::plugins::built_in::GITHUB;
    let pool = pool.cloned();
    let holding = settings.clone();

    Arc::new(Lazily {
        forge: uuid.to_owned(),
        settings,
        author: None,
        loaded: Arc::new(OnceLock::new()),
        loading: Arc::new(move || {
            loading(
                holding.clone(),
                pool.clone(),
                redirect.clone(),
                standing_in.clone(),
            )
        }),
    })
}

/// GitHub's own plugin, read and loaded now — the whole of what [`Lazily`]
/// does the once.
fn loading(
    settings: Option<Settings>,
    pool: Option<sqlx::SqlitePool>,
    redirect: Redirect,
    standing_in: Option<StandingIn>,
) -> Arc<dyn Forge> {
    let built_in = crate::plugins::built_in::github();
    let uuid = crate::plugins::built_in::GITHUB;

    let unloaded = |why: String| -> Arc<dyn Forge> {
        tracing::error!(why = %why, "GitHub's own plugin didn't load");

        Arc::new(Plugged::unloaded(
            uuid,
            format!(
                "GitHub's own plugin didn't load, so Verkstead can't ask GitHub anything: {why}"
            ),
        ))
    };

    let mut manifest = match built_in.manifest(crate::platform::Platform::HERE) {
        Ok(manifest) => manifest,
        Err(refusal) => return unloaded(refusal.why),
    };

    // On a thread of its own with a runtime of its own, for the reason every
    // call into a component is on one: loading is async, and what asks for
    // this Forge is a blocking call made from inside somebody else's runtime,
    // where a runtime cannot be started.
    let loaded = std::thread::Builder::new()
        .name("forge-loading".to_owned())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| format!("a runtime to load it on couldn't be made: {error}."))?;

            runtime.block_on(async move {
                if let Some(pool) = &pool {
                    reaching(&mut manifest, &added(pool, uuid).await);
                }

                of(built_in.files(), &manifest, settings, redirect, standing_in)
                    .await
                    .map(Arc::new)
                    .map_err(|trouble| trouble.why)
            })
        })
        .map_err(|error| format!("a thread to load it on couldn't be started: {error}."))
        .and_then(|started| {
            started
                .join()
                .map_err(|_| "loading it stopped part way through.".to_owned())
        });

    match loaded {
        Ok(Ok(plugged)) => plugged,
        Ok(Err(why)) | Err(why) => unloaded(why),
    }
}

/// A Forge whose component is loaded the first time anything asks it
/// something, and never where nothing does.
///
/// **Because a router is built where nothing can be awaited.** Every router
/// holds a Forge, the way the served one does, and a component loads
/// asynchronously — so what a router is handed is this, and the load happens
/// inside the first blocking call that needs an answer. Which is also what
/// makes a Forge on every router cheap: a workbench stood up to ask what a
/// page draws never loads one.
///
/// **What it answers itself** is what is not the component's: whose token a
/// call would be made with, and what Verkstead's own git authenticates as.
/// Both are questions about this installation's files — see
/// [`crate::forges::whom`] — and a `credential()` that loaded a component to
/// answer would load one for every git on every Repo.
struct Lazily {
    /// The uuid of the plugin whose Forge this is, which is what an Author's
    /// token is held under.
    forge: String,

    /// Where that token is read from, or `None` where there is nowhere to read
    /// one.
    settings: Option<Settings>,

    /// The Author the Repo being asked about recorded — carried here rather
    /// than through the component, so [`Forge::as_author`] loads nothing.
    author: Option<String>,

    /// What loading it is, which is run once.
    loading: Arc<dyn Fn() -> Arc<dyn Forge> + Send + Sync>,

    /// And what it loaded, shared by every [`Forge::as_author`] of it: one
    /// component per Forge rather than one per Repo asked about.
    loaded: Arc<OnceLock<Arc<dyn Forge>>>,
}

impl Clone for Lazily {
    fn clone(&self) -> Lazily {
        Lazily {
            forge: self.forge.clone(),
            settings: self.settings.clone(),
            author: self.author.clone(),
            loading: self.loading.clone(),
            loaded: self.loaded.clone(),
        }
    }
}

impl std::fmt::Debug for Lazily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lazily")
            .field("forge", &self.forge)
            .field("author", &self.author)
            .field("loaded", &self.loaded.get().is_some())
            .finish_non_exhaustive()
    }
}

impl Lazily {
    /// The Forge itself, loaded if this is the first question, asking as the
    /// Author this one records.
    fn forge(&self) -> Arc<dyn Forge> {
        self.loaded
            .get_or_init(|| (self.loading)())
            .as_author(self.author.as_deref())
    }
}

impl Forge for Lazily {
    fn as_author(&self, recorded: Option<&str>) -> Arc<dyn Forge> {
        Arc::new(Lazily {
            author: recorded.map(str::to_owned),
            ..self.clone()
        })
    }

    fn credential(&self) -> Credential {
        Credential(crate::forges::whom(
            self.settings.as_ref(),
            self.author.as_deref(),
            &self.forge,
        ))
    }

    fn tokened(&self) -> bool {
        matches!(
            crate::forges::whom(self.settings.as_ref(), self.author.as_deref(), &self.forge),
            AsWhom::Token(_)
        )
    }

    fn verify(&self, token: &str) -> Result<Account, Trouble> {
        self.forge().verify(token)
    }

    fn pull_request(&self, repo: &Path, branch: &str) -> Result<store::PullRequest, Trouble> {
        self.forge().pull_request(repo, branch)
    }

    fn pull_request_numbered(&self, repo: &Path, number: i64) -> Result<Numbered, Trouble> {
        self.forge().pull_request_numbered(repo, number)
    }

    fn open_pull_request(&self, repo: &Path, opening: &Opening) -> Result<Numbered, Trouble> {
        self.forge().open_pull_request(repo, opening)
    }

    fn open_pull_requests(&self, repo: &Path) -> Result<Vec<Numbered>, Trouble> {
        self.forge().open_pull_requests(repo)
    }

    fn details(&self, repo: &Path, number: i64) -> Result<Details, Trouble> {
        self.forge().details(repo, number)
    }

    fn checks(&self, repo: &Path, number: i64) -> Result<Suite, Trouble> {
        self.forge().checks(repo, number)
    }

    fn landing(&self, repo: &Path, number: i64) -> Result<Landing, Trouble> {
        self.forge().landing(repo, number)
    }

    fn comments(&self, repo: &Path, number: i64) -> Result<Vec<Comment>, Trouble> {
        self.forge().comments(repo, number)
    }

    fn comment(
        &self,
        repo: &Path,
        token: &str,
        number: i64,
        body: &str,
    ) -> Result<String, Trouble> {
        self.forge().comment(repo, token, number, body)
    }

    fn reply(&self, repo: &Path, number: i64, body: &str) -> Result<String, Trouble> {
        self.forge().reply(repo, number, body)
    }

    fn create_gist(
        &self,
        token: &str,
        description: &str,
        name: &str,
        content: &str,
    ) -> Result<Gist, Trouble> {
        self.forge().create_gist(token, description, name, content)
    }

    fn delete_gist(&self, token: &str, id: &str) -> Result<(), Trouble> {
        self.forge().delete_gist(token, id)
    }

    fn create_repository(&self, repo: &Path, name: &str) -> Result<(), Trouble> {
        self.forge().create_repository(repo, name)
    }

    fn tool_extension(
        &self,
        name: &str,
        environment: &[(String, OsString)],
    ) -> Result<(), Trouble> {
        self.forge().tool_extension(name, environment)
    }

    fn lacks_scope(&self, trouble: &Trouble) -> bool {
        self.forge().lacks_scope(trouble)
    }

    fn target_in(&self, prose: &str) -> Option<NamedPullRequest> {
        self.forge().target_in(prose)
    }

    fn target_named(&self, target: &str) -> Option<NamedPullRequest> {
        self.forge().target_named(target)
    }

    fn repository_in(&self, url: &str) -> Option<String> {
        self.forge().repository_in(url)
    }

    fn repository_of(&self, origin: &str) -> Option<String> {
        self.forge().repository_of(origin)
    }
}

/// The hosts the human added to the Forge `uuid` in its settings, which is how
/// a self-hosted instance is reached (ADR-0030, *Forge*).
async fn added(pool: &sqlx::SqlitePool, uuid: &str) -> Vec<String> {
    match crate::forges::built_in(pool).await {
        Ok(enabled) => enabled
            .into_iter()
            .find(|forge| forge.uuid == uuid)
            .map(|forge| forge.hosts)
            .unwrap_or_default(),
        Err(error) => {
            tracing::error!(error = ?error, "reading the hosts added to a forge failed");
            Vec::new()
        }
    }
}

/// Widen `manifest`'s outbound grant to `hosts`, so that a Repo on a
/// self-hosted instance can be asked about: a host added to a Forge is a host
/// its component may reach, and one it may reach is one somebody put in that
/// list.
///
/// **Read once, as the Forge is built.** A host added reaches the matching of
/// a Repo's Forge at the next question — see [`crate::forges::of`] — and
/// reaches this grant at the next start, WASI fixing what a store may reach
/// when the store is made.
fn reaching(manifest: &mut crate::plugins::manifest::Manifest, hosts: &[String]) {
    use crate::plugins::manifest::Capability;

    for host in hosts {
        let granted = manifest.capabilities.iter().any(|capability| {
            matches!(capability, Capability::Outbound { host: granted } if granted.eq_ignore_ascii_case(host))
        });

        if !granted {
            manifest
                .capabilities
                .push(Capability::Outbound { host: host.clone() });
        }
    }
}

/// What a Forge's component is, where the plugin that provides it is
/// `built_in::GITHUB` or any other: its manifest's `forge` block and the
/// component its files hold, loaded under the capabilities the manifest asked
/// for.
pub(crate) async fn of(
    files: crate::plugins::manifest::Files<'_>,
    manifest: &crate::plugins::manifest::Manifest,
    settings: Option<Settings>,
    redirect: crate::plugins::component::Redirect,
    standing_in: Option<StandingIn>,
) -> Result<Plugged, crate::plugins::component::Trouble> {
    let declared = manifest
        .provides
        .iter()
        .find_map(|provision| provision.forge())
        .ok_or_else(|| crate::plugins::component::Trouble {
            why: "It provides no forge.".to_owned(),
        })?;

    let loaded = crate::plugins::component::load_redirecting(files, manifest, None, redirect)
        .await?
        .ok_or_else(|| crate::plugins::component::Trouble {
            why: "It provides a forge and has no component.".to_owned(),
        })?;

    Ok(Plugged::new(
        &manifest.uuid,
        declared.tool.clone(),
        settings,
        loaded.again.clone(),
    )
    .standing_in(standing_in))
}

/// Every test here runs GitHub's own component — the one this binary embeds —
/// against an HTTP stand-in for GitHub, reached through the host's outbound
/// redirect with the component's grant untouched: the request is still made to
/// `api.github.com`, still checked against what the manifest asked for, and
/// only the connection goes to a listener this test stood up. See
/// [`crate::plugins::component::Redirect`].
///
/// A stand-in rather than GitHub, for the reason the `gh` stand-in was a
/// script: asking the real one would be a test that needed a network, an
/// account and somebody's pull request.
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use axum::Router;
    use axum::body::Bytes;
    use axum::extract::{Request, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::*;

    /// One answer the stand-in gives: the text to look for in the request's
    /// path and body together, the status, and what to say.
    #[derive(Clone)]
    struct Answer {
        asked_for: &'static str,
        status: u16,
        said: String,
    }

    /// An answer to a request naming `asked_for`.
    fn answering(asked_for: &'static str, said: &str) -> Answer {
        Answer {
            asked_for,
            status: 200,
            said: said.to_owned(),
        }
    }

    /// And one GitHub refuses with.
    fn refusing(asked_for: &'static str, status: u16, said: &str) -> Answer {
        Answer {
            asked_for,
            status,
            said: said.to_owned(),
        }
    }

    /// One request the stand-in was asked: where, and what it carried as its
    /// token — which is what says whether the host set the secret for this
    /// call.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Asked {
        path: String,
        token: Option<String>,
    }

    #[derive(Clone)]
    struct Standing {
        answers: Vec<Answer>,
        asked: Arc<Mutex<Vec<Asked>>>,
    }

    /// GitHub's stand-in, listening on a port of its own.
    struct Stood {
        address: String,
        asked: Arc<Mutex<Vec<Asked>>>,
    }

    impl Stood {
        /// What it was asked, in the order it was asked.
        fn asked(&self) -> Vec<Asked> {
            self.asked.lock().unwrap().clone()
        }

        /// And the token the one call it was asked carried.
        fn token(&self) -> Option<String> {
            self.asked().first().and_then(|asked| asked.token.clone())
        }
    }

    async fn stood(answers: &[Answer]) -> Stood {
        let asked: Arc<Mutex<Vec<Asked>>> = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new().fallback(answered).with_state(Standing {
            answers: answers.to_vec(),
            asked: asked.clone(),
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        Stood {
            address: address.to_string(),
            asked,
        }
    }

    /// Whatever the stand-in has for the request, or a `404` saying it has
    /// nothing — which is GitHub's own answer for a path it does not know.
    async fn answered(State(standing): State<Standing>, request: Request) -> impl IntoResponse {
        let path = request.uri().to_string();
        let token = request
            .headers()
            .get("authorization")
            .and_then(|said| said.to_str().ok())
            .map(|said| said.trim_start_matches("Bearer ").to_owned());
        let body = axum::body::to_bytes(request.into_body(), 1 << 20)
            .await
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();

        standing.asked.lock().unwrap().push(Asked {
            path: path.clone(),
            token,
        });

        let asked = format!("{path} {body}");
        match standing
            .answers
            .iter()
            .find(|answer| asked.contains(answer.asked_for))
        {
            Some(answer) => (
                StatusCode::from_u16(answer.status).unwrap(),
                [("content-type", "application/json")],
                Bytes::from(answer.said.clone()),
            ),
            None => (
                StatusCode::NOT_FOUND,
                [("content-type", "application/json")],
                Bytes::from_static(br#"{"message":"Not Found"}"#),
            ),
        }
    }

    /// GitHub's own component, as this binary embeds it, asking as the one
    /// Author's token and reaching `stood` where it asks for
    /// `api.github.com`.
    async fn forge(stood: &Stood) -> (tempfile::TempDir, Plugged) {
        tokened(stood, Some("author_tokens:\n  work: ghp_theworkone\n")).await
    }

    /// And the same with `secrets` as the file holds it — `None` for an
    /// Author holding no token at all.
    async fn tokened(stood: &Stood, secrets: Option<&str>) -> (tempfile::TempDir, Plugged) {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());
        std::fs::write(settings.config_path(), "authors:\n  - id: work\n").unwrap();
        if let Some(secrets) = secrets {
            std::fs::write(settings.secrets_path(), secrets).unwrap();
        }

        (dir, plugged(stood, Some(settings)).await)
    }

    async fn plugged(stood: &Stood, settings: Option<Settings>) -> Plugged {
        let built_in = crate::plugins::built_in::github();
        let manifest = built_in
            .manifest(crate::platform::Platform::HERE)
            .expect("GitHub's manifest reads");

        of(
            built_in.files(),
            &manifest,
            settings,
            Redirect::to("api.github.com", &stood.address),
            None,
        )
        .await
        .expect("GitHub's component loads")
    }

    /// A checkout whose `origin` is `remote`, which is the whole of what says
    /// which repository on which host the Forge is asked about.
    fn checkout(remote: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        for args in [
            vec!["init", "--initial-branch", "main"],
            vec!["remote", "add", "origin", remote],
        ] {
            let ran = std::process::Command::new("git")
                .current_dir(dir.path())
                .args(&args)
                .output()
                .expect("git runs");
            assert!(ran.status.success(), "git {args:?}");
        }

        dir
    }

    /// The remote every test but the Enterprise one is about.
    const ORIGIN: &str = "https://github.com/tobico/verkstead.git";

    /// One of GraphQL's answers about a pull request.
    fn answered_about(fields: &str) -> String {
        format!(r#"{{"data":{{"repository":{{"pullRequest":{{{fields}}}}}}}}}"#)
    }

    /// The pull request on a branch comes back as the five facts worth
    /// recording, read off the branch GitHub answered about rather than off
    /// the selector — and it is asked for as the Author's own token.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_branch_with_a_pull_request_reads_back_as_its_number_title_url_and_branches() {
        let stood = stood(&[answering(
            "pullRequests(headRefName",
            r#"{"data":{"repository":{"pullRequests":{"nodes":[
                 {"number":41,"title":"Share the limiter","url":"https://github.com/tobico/verkstead/pull/41",
                  "headRefName":"tobi/rate-limiting","baseRefName":"main"}]}}}}"#,
        )])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        let found = forge
            .pull_request(repo.path(), "tobi/rate-limiting")
            .expect("the branch has one open");

        assert_eq!(found.number, 41);
        assert_eq!(found.title, "Share the limiter");
        assert_eq!(found.url, "https://github.com/tobico/verkstead/pull/41");
        assert_eq!(found.head.as_deref(), Some("tobi/rate-limiting"));
        assert_eq!(found.base.as_deref(), Some("main"));
        assert_eq!(stood.token().as_deref(), Some("ghp_theworkone"));
    }

    /// A branch with nothing open on it is an answer rather than a failure,
    /// and so is a number GitHub cannot resolve: both are the one kind every
    /// caller asks again about.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn nothing_open_under_what_was_asked_about_is_no_pull_request() {
        let stood = stood(&[
            answering(
                "pullRequests(headRefName",
                r#"{"data":{"repository":{"pullRequests":{"nodes":[]}}}}"#,
            ),
            answering(
                "pullRequest(number",
                r#"{"data":null,"errors":[{"message":"Could not resolve to a PullRequest with the number of 4040."}]}"#,
            ),
        ])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        assert_eq!(
            forge.pull_request(repo.path(), "tobi/nothing").err(),
            Some(Trouble::NoPullRequest),
        );
        assert_eq!(
            forge.pull_request_numbered(repo.path(), 4040).err(),
            Some(Trouble::NoPullRequest),
        );
    }

    /// A token GitHub will not have is nobody logged in, which is the same
    /// thing to the human as holding none.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_token_github_turns_down_is_nobody_logged_in() {
        let stood = stood(&[refusing("graphql", 401, r#"{"message":"Bad credentials"}"#)]).await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        assert_eq!(
            forge.landing(repo.path(), 41).err(),
            Some(Trouble::NotLoggedIn),
        );
    }

    /// And a repository with no remote for the Forge to work out which of its
    /// repositories it is comes back as that, with nothing asked of GitHub at
    /// all: there is nobody to ask about.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_repository_with_no_remote_says_so_and_asks_nothing() {
        let stood = stood(&[]).await;
        let repo = checkout("/srv/upstream.git");
        let (_dir, forge) = forge(&stood).await;

        assert_eq!(
            forge.landing(repo.path(), 41).err(),
            Some(Trouble::NoRemote),
        );
        assert_eq!(stood.asked(), Vec::new());
    }

    /// Anything else is what GitHub said, quoted rather than guessed at.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unfamiliar_refusal_is_quoted() {
        let stood = stood(&[refusing("graphql", 502, r#"{"message":"Server Error"}"#)]).await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        assert_eq!(
            forge.landing(repo.path(), 41).err(),
            Some(Trouble::Refused("HTTP 502: Server Error".to_owned())),
        );
    }

    /// The open pull requests come back with their heads, their bases and
    /// whether each head is in another repository — which is the whole of what
    /// a stack is walked out of, both ways from the one taken up, with a fork
    /// left out of the chain.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_open_pull_requests_walk_a_stack_both_ways_and_leave_a_fork_out() {
        let stood = stood(&[answering(
            "pullRequests(states:OPEN",
            r#"{"data":{"repository":{"pullRequests":{"nodes":[
                 {"number":3,"title":"Top","url":"u3","headRefName":"three","baseRefName":"two","isCrossRepository":false},
                 {"number":9,"title":"Somebody's fork","url":"u9","headRefName":"two","baseRefName":"main","isCrossRepository":true},
                 {"number":2,"title":"Middle","url":"u2","headRefName":"two","baseRefName":"one","isCrossRepository":false},
                 {"number":1,"title":"Bottom","url":"u1","headRefName":"one","baseRefName":"main","isCrossRepository":false}]}}}}"#,
        )])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        let open = forge
            .open_pull_requests(repo.path())
            .expect("the repository lists them");

        let walked: Vec<i64> = crate::stacks::chain(&open, 2)
            .into_iter()
            .map(|link| link.number)
            .collect();

        assert_eq!(walked, vec![1, 2, 3], "walked down and up from #2");
        assert!(
            open.iter().any(|listed| listed.number == 9 && listed.fork),
            "the fork is listed as one: {open:?}",
        );
        assert!(
            crate::stacks::chain(&open, 9).is_empty(),
            "and is no chain of its own",
        );
    }

    /// The checks and whether the pull request merges come back on one
    /// question, with the commit the checks are about: a rollup is a fact
    /// about one commit rather than about the branch.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_checks_the_head_and_whether_it_merges_come_back_on_one_question() {
        let stood = stood(&[answering(
            "headRefOid",
            &answered_about(
                r#""headRefOid":"abc123","mergeable":"CONFLICTING",
                   "rollup":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{"nodes":[
                     {"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"FAILURE","detailsUrl":"https://runs/1"},
                     {"__typename":"CheckRun","name":"Web","status":"IN_PROGRESS","conclusion":""},
                     {"__typename":"StatusContext","context":"buildkite","state":"SUCCESS","targetUrl":"https://runs/2"}]}}}}]}"#,
            ),
        )])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        let suite = forge.checks(repo.path(), 41).expect("the checks read");

        assert_eq!(suite.head, "abc123");
        assert_eq!(suite.mergeable, Mergeable::Conflicting);
        assert_eq!(
            suite
                .checks
                .iter()
                .map(|check| (check.name.as_str(), check.how, check.link.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Rust", Checked::Failed, "https://runs/1"),
                ("Web", Checked::Running, ""),
                ("buildkite", Checked::Passed, "https://runs/2"),
            ],
        );
    }

    /// A Share is published as an unlisted gist, with the git remote the bytes
    /// arrive over — as the token the caller has read rather than as whatever
    /// the Author's is.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_share_is_published_as_an_unlisted_gist() {
        let stood = stood(&[answering(
            "/gists",
            r#"{"id":"g1","html_url":"https://gist.github.com/g1","git_push_url":"https://gist.github.com/g1.git"}"#,
        )])
        .await;
        let (_dir, forge) = forge(&stood).await;

        let gist = forge
            .create_gist("ghp_thepasted", "A share", "share.md", "placeholder")
            .expect("the gist is made");

        assert_eq!(gist.id, "g1");
        assert_eq!(gist.url, "https://gist.github.com/g1");
        assert_eq!(gist.push, "https://gist.github.com/g1.git");
        assert_eq!(stood.token().as_deref(), Some("ghp_thepasted"));
    }

    /// A token is verified as the account GitHub answers for it, with the
    /// scopes off the headers — and a token GitHub named none for is not known
    /// to lack any.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_token_is_verified_as_the_account_github_answers_for_it() {
        let stood = stood(&[answering("/user", r#"{"login":"tobico"}"#)]).await;
        let (_dir, forge) = forge(&stood).await;

        let account = forge.verify("ghp_thepasted").expect("the token verifies");

        assert_eq!(account.login, "tobico");
        assert_eq!(account.scopes, Scopes::Unsaid);
        assert_eq!(account.short_of(&["gist".to_owned()]), Vec::<String>::new());
        assert_eq!(stood.token().as_deref(), Some("ghp_thepasted"));
    }

    /// And a token GitHub turns down is that refusal in GitHub's own words,
    /// rather than nobody being logged in.
    ///
    /// **The one place a `401` is not read as having no token**: what is being
    /// asked about *is* the token, so a page that has just been handed one is
    /// owed what GitHub said about it — *there is no token* would be telling
    /// the human to do the thing they have done. See the component's `verify`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_token_github_turns_down_is_verified_as_what_github_said() {
        let stood = stood(&[refusing("/user", 401, r#"{"message":"Bad credentials"}"#)]).await;
        let (_dir, forge) = forge(&stood).await;

        assert_eq!(
            forge.verify("ghp_thepasted").err(),
            Some(Trouble::Refused("HTTP 401: Bad credentials".to_owned())),
        );
    }

    /// **The token is in the component's environment for the one call it was
    /// set for.** Each call is a fresh instance holding whatever the host read
    /// a moment ago, so a call made as the Author carries the Author's token
    /// and a call made as a pasted one carries that — and an Author holding
    /// none leaves the variable unset, which the component answers
    /// `not-logged-in` to without asking GitHub anything.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_token_is_in_the_components_environment_for_the_one_call() {
        let stood = stood(&[
            answering("/user", r#"{"login":"tobico"}"#),
            answering(
                "mergeable state",
                &answered_about(r#""mergeable":"MERGEABLE","state":"OPEN""#),
            ),
        ])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        forge.verify("ghp_thepasted").expect("the token verifies");
        forge
            .landing(repo.path(), 41)
            .expect("and the pull request reads");

        assert_eq!(
            stood
                .asked()
                .iter()
                .map(|asked| asked.token.clone())
                .collect::<Vec<_>>(),
            vec![
                Some("ghp_thepasted".to_owned()),
                Some("ghp_theworkone".to_owned()),
            ],
            "each call carried the token it was made with and no other",
        );

        let (_dir, untokened) = tokened(&stood, None).await;
        assert_eq!(
            untokened.landing(repo.path(), 41).err(),
            Some(Trouble::NotLoggedIn),
            "an Author with no token leaves the Forge unasked",
        );
        assert_eq!(stood.asked().len(), 2, "and asked GitHub nothing more");
    }

    /// A component asking a host its grant does not name is refused before a
    /// connection is made: a Repo on a GitHub Enterprise host nobody added to
    /// this Forge's hosts is a Repo whose API the component may not reach, and
    /// what comes back is the refusal in words rather than an answer.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_host_outside_the_grant_is_refused() {
        let stood = stood(&[answering(
            "mergeable state",
            &answered_about(r#""mergeable":"MERGEABLE","state":"OPEN""#),
        )])
        .await;
        let repo = checkout("https://github.example.com/tobico/verkstead.git");
        let (_dir, forge) = forge(&stood).await;

        let trouble = forge
            .landing(repo.path(), 41)
            .expect_err("the host was never granted");

        assert!(
            matches!(&trouble, Trouble::Refused(said) if said.contains("couldn't be asked")),
            "{trouble:?}",
        );
        assert_eq!(stood.asked(), Vec::new(), "and nothing was asked of GitHub");
    }

    /// The string work is the component's too: the pull request a Brief's
    /// prose or a Target field names, and the repository a remote is — asked
    /// with no token at all, there being nothing to ask GitHub.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_target_and_a_remote_are_read_by_the_component() {
        let stood = stood(&[]).await;
        let (_dir, forge) = forge(&stood).await;

        assert_eq!(
            forge.target_in("Have a look at https://github.com/tobico/verkstead/pull/41 please"),
            Some(NamedPullRequest {
                number: 41,
                repository: Some("tobico/verkstead".to_owned()),
            }),
        );
        assert_eq!(
            forge.target_named("#7"),
            Some(NamedPullRequest {
                number: 7,
                repository: None,
            }),
        );
        assert_eq!(forge.target_named("tobi/rate-limiting"), None);
        assert_eq!(
            forge.repository_of("git@github.com:tobico/verkstead.git"),
            Some("tobico/verkstead".to_owned()),
        );
        assert_eq!(forge.repository_of("/srv/upstream.git"), None);
        assert_eq!(stood.asked(), Vec::new());
    }

    /// What the details pane draws comes back on one question with the facts
    /// a card draws: the commits, what was said, the checks and whether it
    /// merges — the commit list and the check rollup each asked for with the
    /// arguments they want, which one question can only do under a name of
    /// its own.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_commits_the_comments_and_the_checks_come_back_on_one_question() {
        let stood = stood(&[answering(
            "messageHeadline",
            &answered_about(
                r#""mergeable":"MERGEABLE","state":"OPEN",
                   "commits":{"nodes":[
                     {"commit":{"oid":"c1","messageHeadline":"Share the limiter"}},
                     {"commit":{"oid":"c2","messageHeadline":"Count against one window"}}]},
                   "comments":{"nodes":[
                     {"author":{"login":"tobico"},"body":"Looks right.","createdAt":"2026-11-22T09:00:00Z"}]},
                   "rollup":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{"nodes":[
                     {"__typename":"CheckRun","name":"Rust","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://runs/1"}]}}}}]}"#,
            ),
        )])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        let details = forge.details(repo.path(), 41).expect("the details read");

        assert_eq!(
            details
                .pane
                .commits
                .iter()
                .map(|commit| (commit.sha.as_str(), commit.subject.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("c1", "Share the limiter"),
                ("c2", "Count against one window"),
            ],
        );
        assert_eq!(details.pane.comments.len(), 1);
        assert_eq!(details.pane.comments[0].author, "tobico");
        assert_eq!(
            details
                .checks
                .iter()
                .map(|check| (check.name.as_str(), check.how))
                .collect::<Vec<_>>(),
            vec![("Rust", Checked::Passed)],
        );
        assert_eq!(details.landing.mergeable, Mergeable::Cleanly);
        assert_eq!(details.landing.stands, Stands::Open);
        assert_eq!(stood.asked().len(), 1, "one question: {:?}", stood.asked());
    }

    /// Everything said on a pull request comes back in the order it was said,
    /// out of all three places a human writes: the conversation, the words at
    /// the top of a review, and the lines of the diff — which are REST's, and
    /// a second question. A review with nothing written at the top of it is
    /// nothing to address.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn everything_said_comes_back_in_the_order_it_was_said() {
        let stood = stood(&[
            answering(
                "reviews(first",
                &answered_about(
                    r#""comments":{"nodes":[
                         {"id":"IC_1","url":"https://said/1","author":{"login":"tobico"},
                          "body":"Needs a test.","createdAt":"2026-11-22T09:00:00Z"}]},
                       "reviews":{"nodes":[
                         {"id":"PRR_1","author":{"login":"ada"},"body":"","submittedAt":"2026-11-22T08:00:00Z"},
                         {"id":"PRR_2","author":{"login":"ada"},"body":"Two things.","submittedAt":"2026-11-22T11:00:00Z"}]}"#,
                ),
            ),
            answering(
                "/comments",
                r#"[{"node_id":"PRRC_1","html_url":"https://said/2","user":{"login":"ada"},
                     "body":"This line.","created_at":"2026-11-22T10:00:00Z",
                     "path":"crates/server/src/limits.rs","line":12}]"#,
            ),
        ])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        let said = forge.comments(repo.path(), 41).expect("the comments read");

        assert_eq!(
            said.iter()
                .map(|said| (
                    said.which.as_str(),
                    said.author.as_str(),
                    said.about.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("IC_1", "tobico", ""),
                ("PRRC_1", "ada", "`crates/server/src/limits.rs` line 12"),
                ("PRR_2", "ada", ""),
            ],
            "in the order they were said, the wordless review left out",
        );
    }

    /// A draft is opened from the head, the base, the title and the body, and
    /// read back by the number GitHub gave it — the one shape a pull request
    /// asked for by number comes back in, rather than a second reading of what
    /// the create answered.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_draft_is_opened_and_read_back_by_its_number() {
        let stood = stood(&[
            answering("/pulls {", r#"{"number":41}"#),
            answering(
                "pullRequest(number",
                &answered_about(
                    r#""number":41,"title":"Share the limiter","url":"https://github.com/tobico/verkstead/pull/41",
                       "headRefName":"tobi/rate-limiting","baseRefName":"main",
                       "isCrossRepository":false,"state":"OPEN""#,
                ),
            ),
        ])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        let opened = forge
            .open_pull_request(
                repo.path(),
                &Opening {
                    head: "tobi/rate-limiting".to_owned(),
                    base: "main".to_owned(),
                    title: "Share the limiter".to_owned(),
                    body: "What it does.".to_owned(),
                },
            )
            .expect("the draft opens");

        assert_eq!(opened.number, 41);
        assert_eq!(opened.base, "main");
        assert!(opened.open && !opened.fork);

        let asked = stood.asked();
        assert!(
            asked[0].path.ends_with("/repos/tobico/verkstead/pulls"),
            "{asked:?}",
        );
    }

    /// Something said on a pull request comes back as where it was said, which
    /// is what the human is owed a link to — as the token the caller read
    /// rather than as the Author's.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_comment_is_said_and_answered_with_where_it_was_said() {
        let stood = stood(&[answering(
            "/issues/41/comments",
            r#"{"html_url":"https://github.com/tobico/verkstead/pull/41#issuecomment-1"}"#,
        )])
        .await;
        let repo = checkout(ORIGIN);
        let (_dir, forge) = forge(&stood).await;

        assert_eq!(
            forge
                .comment(repo.path(), "ghp_thepasted", 41, "Published a share.")
                .expect("it is said"),
            "https://github.com/tobico/verkstead/pull/41#issuecomment-1",
        );
        assert_eq!(stood.token().as_deref(), Some("ghp_thepasted"));

        // And the same question as the Author, which is what a wrap-up's reply
        // is.
        assert!(forge.reply(repo.path(), 41, "Addressed.").is_ok());
        assert_eq!(
            stood.asked()[1].token.as_deref(),
            Some("ghp_theworkone"),
            "{:?}",
            stood.asked(),
        );
    }

    /// And a published Share is taken down again, which is what a publish that
    /// fell over after the gist was made does with it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_published_share_is_taken_down_again() {
        let stood = stood(&[answering("/gists/g1", "{}")]).await;
        let (_dir, forge) = forge(&stood).await;

        forge
            .delete_gist("ghp_thepasted", "g1")
            .expect("it is taken down");

        assert_eq!(stood.asked().len(), 1);
        assert!(stood.asked()[0].path.ends_with("/gists/g1"));
    }

    /// **Create repo is two halves.** The component makes the repository and
    /// answers the remote it is at; the host writes that as `origin` and
    /// pushes the branch, because the host is the one holding the checkout.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_repository_is_made_and_the_branch_pushed_to_it() {
        let made = tempfile::tempdir().unwrap();
        let bare = made.path().join("verkstead.git");
        let ran = std::process::Command::new("git")
            .args(["init", "--bare", "--initial-branch", "main"])
            .arg(&bare)
            .output()
            .expect("git runs");
        assert!(ran.status.success());

        let stood = stood(&[answering(
            "/user/repos",
            &format!(
                r#"{{"html_url":"https://github.com/tobico/verkstead","clone_url":"{}"}}"#,
                bare.display()
            ),
        )])
        .await;

        // A checkout with something on it: there is nothing to push before the
        // first commit, which is why the create runs after it.
        let repo = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "--initial-branch", "main"],
            vec![
                "-c",
                "user.name=Ada",
                "-c",
                "user.email=ada@example.com",
                "commit",
                "--allow-empty",
                "-m",
                "First",
            ],
        ] {
            let ran = std::process::Command::new("git")
                .current_dir(repo.path())
                .args(&args)
                .output()
                .expect("git runs");
            assert!(ran.status.success(), "git {args:?}");
        }

        let (_dir, forge) = forge(&stood).await;

        forge
            .create_repository(repo.path(), "verkstead")
            .expect("the repository is made and pushed");

        assert_eq!(
            crate::repos::origin(repo.path()).as_deref(),
            Some(bare.to_str().unwrap())
        );
        let landed = std::process::Command::new("git")
            .current_dir(&bare)
            .args(["log", "--oneline", "main"])
            .output()
            .expect("git runs");
        assert!(
            String::from_utf8_lossy(&landed.stdout).contains("First"),
            "the commit landed on the remote: {landed:?}",
        );
    }

    /// Whether a refusal is the Forge turning the token down for what it may
    /// do is the component's reading too — and nothing is asked of GitHub to
    /// make it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_refusal_about_what_the_token_may_do_is_told_apart() {
        let stood = stood(&[]).await;
        let (_dir, forge) = forge(&stood).await;

        assert!(forge.lacks_scope(&Trouble::Refused(
            "HTTP 403: Resource not accessible by personal access token".to_owned()
        )));
        assert!(forge.lacks_scope(&Trouble::Refused("HTTP 404: Not Found".to_owned())));
        assert!(!forge.lacks_scope(&Trouble::Refused("HTTP 422: Validation Failed".to_owned())));
        assert!(!forge.lacks_scope(&Trouble::NotLoggedIn));
        assert_eq!(stood.asked(), Vec::new());
    }

    /// And a Forge whose plugin would not load answers the reason, rather than
    /// a workbench that would not come up.
    #[test]
    fn a_forge_whose_component_did_not_load_answers_why() {
        let forge = Plugged::unloaded(
            crate::plugins::built_in::GITHUB,
            "GitHub's own plugin didn't load.".to_owned(),
        );

        assert_eq!(
            forge.landing(Path::new("/nowhere"), 41).err(),
            Some(Trouble::Ours("GitHub's own plugin didn't load.".to_owned())),
        );
    }
}
