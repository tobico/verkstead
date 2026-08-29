//! Verkstead's own reach into GitHub: the *host's* `gh`, run against a Repo.
//!
//! Not the sandbox's. The sessions keep using their own `gh` inside their
//! sandboxes to push and to open a pull request — that is the repository's
//! review process being followed by an agent — and this is the server asking
//! GitHub what happened afterwards: which PR the branch has, what is on it, and
//! what has been said about it.
//!
//! It authenticates as the configured token — the one in `secrets.yaml` that
//! every session's sandbox gets too — handed to `gh` as `GH_TOKEN` in the
//! environment of the call. The file is read at the moment of the call rather
//! than held from startup, so a token saved or rotated through the settings
//! page reaches the next `gh` without the server being restarted.
//!
//! With nothing configured the call is made as it always was, and falls back to
//! whatever login the host's `gh` has. Which may be none, and that is
//! [`Trouble::NotLoggedIn`]: an answer for the human, naming the settings page
//! it is fixed on.
//!
//! Everything it asks can fail for ordinary reasons — no `gh` on the PATH, an
//! account that is not logged in, a repository with no remote, a branch with no
//! PR yet — and every one of those is an answer rather than an error to fall
//! over on. That is what [`Trouble`] is: a reason in words, for the human to
//! read on a Timeline and do something about.
//!
//! Which is also why the program is a field rather than the literal `gh`,
//! exactly as the agent is on [`crate::Agents`]: what has to be provable here is
//! that each of those answers reaches the human as itself, and asking that of
//! the real GitHub would be a test that needed a network, an account and a
//! repository with a pull request on it.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::settings::Settings;
use crate::store;

/// The host's `gh`, as Verkstead runs it.
#[derive(Debug, Clone)]
pub struct Gh {
    /// What to run, before the arguments of whatever is being asked. The whole
    /// argv rather than a path, so a test can stand a shell script where `gh`
    /// goes.
    program: Vec<String>,

    /// Where the configured token is read from, or `None` where this `Gh` has
    /// no settings behind it — a router watching nothing has no Data Directory
    /// to read one out of, and nothing on it ever asks GitHub anything.
    settings: Option<Settings>,
}

impl Gh {
    /// The real thing: whatever `gh` the host has on its PATH.
    pub fn on_path() -> Gh {
        Gh::running(vec!["gh".to_owned()])
    }

    /// The same, with something else where `gh` goes — see [`Gh::program`].
    pub fn running(program: Vec<String>) -> Gh {
        Gh {
            program,
            settings: None,
        }
    }

    /// And the same again, authenticating as whatever token `settings` holds at
    /// the moment of each call.
    ///
    /// The settings rather than the token, and deliberately: a token read here
    /// would be the one the server started with, where the whole point of
    /// configuring it through a page is that saving a new one takes effect
    /// without a restart.
    pub fn authenticated_by(self, settings: Settings) -> Gh {
        Gh {
            settings: Some(settings),
            ..self
        }
    }

    /// The token to authenticate as, read now — `None` where none is
    /// configured, which leaves the call to whatever login the host's `gh` has.
    fn token(&self) -> Option<String> {
        self.settings
            .as_ref()?
            .secrets()
            .github_token()
            .map(str::to_owned)
    }

    /// Run it inside `repo` and take its stdout, or say why there is none.
    ///
    /// Blocking, like everything that shells out. The callers are on
    /// `spawn_blocking` — see [`pull_request`] and [`details`].
    fn ask(&self, repo: &Path, args: &[&str]) -> Result<String, Trouble> {
        self.run(Some(repo), self.token(), args)
    }

    /// And run it about nothing in particular, authenticating as `token`.
    ///
    /// Nowhere to run it, because what this asks is about the token rather than
    /// about a repository: a `gh` given a working directory would be one that
    /// could fail for the directory's reasons. And `token` rather than the
    /// configured one, because what is being asked about is a token that has
    /// just been typed and may not be the configured one yet — see
    /// [`authenticates_as`].
    fn ask_as(&self, token: &str, args: &[&str]) -> Result<String, Trouble> {
        self.run(None, Some(token.to_owned()), args)
    }

    /// And the same again with a body written to it, which is how a request
    /// carrying JSON is made: `gh api --input -` reads the payload from stdin.
    ///
    /// Its own entry point rather than a parameter on the others because
    /// everything else here asks questions — `stdin` is null on those for a
    /// reason, and a `gh` that stopped to read from a terminal nobody is at
    /// would be a server thread waiting for ever. See [`create_gist`], which is
    /// the one caller and the first write.
    fn tell_as(&self, token: &str, args: &[&str], body: &str) -> Result<String, Trouble> {
        self.written(None, Some(token.to_owned()), args, Some(body))
    }

    /// And the same inside a repository, which is what a write about one has to
    /// be: a pull request's number means something else in another repository, or
    /// nothing. See [`comment`], which is the one caller.
    fn tell_in(
        &self,
        repo: &Path,
        token: &str,
        args: &[&str],
        body: &str,
    ) -> Result<String, Trouble> {
        self.written(Some(repo), Some(token.to_owned()), args, Some(body))
    }

    /// What the four above are: `gh`, run somewhere or nowhere, as somebody or
    /// as whoever the host is logged in as, read for its stdout or for why there
    /// is none.
    fn run(
        &self,
        dir: Option<&Path>,
        token: Option<String>,
        args: &[&str],
    ) -> Result<String, Trouble> {
        self.written(dir, token, args, None)
    }

    /// And the same with whatever is to be written to it, which is `None` on
    /// every read.
    fn written(
        &self,
        dir: Option<&Path>,
        token: Option<String>,
        args: &[&str],
        body: Option<&str>,
    ) -> Result<String, Trouble> {
        let (program, before) = self
            .program
            .split_first()
            .expect("a Gh is built with at least the program to run");

        let mut command = Command::new(program);

        command
            .args(before)
            .args(args)
            // Nothing here is interactive: a `gh` that stopped to ask for a
            // password would be a server thread waiting on a terminal nobody is
            // at. A body to write is the one exception, and it is written and
            // the pipe shut below rather than left open for anything to be asked
            // through.
            .stdin(match body {
                Some(_) => Stdio::piped(),
                None => Stdio::null(),
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = dir {
            command.current_dir(dir);
        }

        // What `gh` authenticates as, which it reads from here without being
        // told to. Set only where there is one to set, for the reason a
        // sandbox's is — see [`crate::sandbox`]: `GH_TOKEN` present and empty is
        // a login `gh` fails on obscurely, and leaving it unset is what lets a
        // Verkstead with nothing configured go on using the host's own login.
        if let Some(token) = token {
            command.env("GH_TOKEN", token);
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Trouble::NoGh);
            }
            Err(error) => return Err(Trouble::Refused(error.to_string())),
        };

        // Written before the output is waited on, and the pipe dropped so that
        // `gh` sees the end of it: a body held open is a child that never
        // finishes reading and a parent that never finishes waiting.
        if let Some(body) = body {
            let written = child
                .stdin
                .take()
                .expect("a piped stdin is there to be written to")
                .write_all(body.as_bytes());

            if let Err(error) = written {
                return Err(Trouble::Refused(error.to_string()));
            }
        }

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(error) => return Err(Trouble::Refused(error.to_string())),
        };

        if !output.status.success() {
            return Err(Trouble::read(&String::from_utf8_lossy(&output.stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Why `gh` could not answer.
///
/// Each of these is a different thing for the human to go and do, which is the
/// whole reason they are told apart: a machine with no `gh` installed and an
/// account that was never logged in are the same failure to a caller and two
/// different afternoons to a person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Trouble {
    /// There is no `gh` on the host's PATH.
    NoGh,

    /// There is, and it is not logged in to anything.
    NotLoggedIn,

    /// The repository has no remote for `gh` to work out which GitHub
    /// repository it is.
    NoRemote,

    /// GitHub has no pull request for this branch.
    NoPullRequest,

    /// Something else, in `gh`'s own words.
    Refused(String),
}

impl Trouble {
    /// What `gh` said, read as one of the reasons above.
    ///
    /// By what it printed rather than by an exit status, because `gh` has one
    /// failing status and several failures. The words are matched loosely and
    /// the catch-all keeps whatever was actually said: a `gh` that reworded a
    /// message should degrade to quoting it, not to claiming the wrong reason.
    fn read(stderr: &str) -> Trouble {
        let said = stderr.to_lowercase();

        if said.contains("no pull requests found") || said.contains("no open pull requests") {
            return Trouble::NoPullRequest;
        }

        if said.contains("auth login")
            || said.contains("not logged in")
            || said.contains("authentication token")
        {
            return Trouble::NotLoggedIn;
        }

        if said.contains("git remote") || said.contains("not a git repository") {
            return Trouble::NoRemote;
        }

        Trouble::Refused(
            stderr
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("gh said nothing about why")
                .to_owned(),
        )
    }

    /// The sentence the Timeline carries, which is the one place any of this is
    /// ever read.
    pub(crate) fn why(&self) -> String {
        match self {
            Trouble::NoGh => {
                "there is no `gh` on this machine's PATH, so Verkstead cannot ask GitHub anything"
                    .to_owned()
            }
            Trouble::NotLoggedIn => {
                "this machine's `gh` is not logged in and no GitHub token is configured, \
                 so Verkstead cannot ask GitHub anything — put one in on the settings page"
                    .to_owned()
            }
            Trouble::NoRemote => {
                "the repository has no GitHub remote for `gh` to ask about".to_owned()
            }
            Trouble::NoPullRequest => {
                "GitHub has no pull request on this branch, so the session that finished opened none"
                    .to_owned()
            }
            Trouble::Refused(said) => format!("`gh` said: {said}"),
        }
    }
}

/// Who `token` authenticates as, and what GitHub will let it do.
///
/// What the settings page verifies a pasted token with. A token is a string of
/// characters that either is or is not somebody's, and the difference is not
/// visible on the page: asking GitHub is the only way to tell a working token
/// from one that was copied short, revoked last week, or issued against the
/// wrong account entirely — and the account name is what makes the answer worth
/// reading, because a token that authenticates as the *wrong* person is the
/// mistake that would otherwise be found in a commit's author line.
///
/// `gh api user` rather than `gh auth status`, and the reason is the same one
/// [`checks`] uses the rollup for: `auth status` reports what it found by
/// failing, and this asks a question that has an answer. The answer is parsed
/// here rather than with `--jq`, so that nothing depends on which jq that `gh`
/// was built with.
///
/// **`-i`, because the scopes are in the headers and nowhere else.** GitHub
/// answers every request with `X-OAuth-Scopes`, and what a token may do is not a
/// field of any resource — so the whole response is read and split at the blank
/// line that ends the headers. See [`Scopes`] for what the absence of that
/// header means, which is not *none*.
///
/// Blocking, like everything else here — see [`Gh::run`].
pub(crate) fn authenticates_as(gh: &Gh, token: &str) -> Result<Account, Trouble> {
    /// The one field of `gh api user` this asks for.
    #[derive(Deserialize)]
    struct Login {
        login: String,
    }

    let said = gh.ask_as(token, &["api", "-i", "user"])?;
    let (headers, body) = split(&said);

    let account: Login = serde_json::from_str(body)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    Ok(Account {
        login: account.login,
        scopes: scopes(headers),
    })
}

/// Who a token is, and what it may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Account {
    /// The login GitHub answered with.
    pub(crate) login: String,

    /// And the scopes it says the token carries.
    pub(crate) scopes: Scopes,
}

/// What GitHub said about a token's scopes, which has three answers rather than
/// two.
///
/// The third is the one worth the type: a **fine-grained** token has permissions
/// rather than scopes, and GitHub answers for one with no scopes header at all.
/// A missing header read as an empty list would have Verkstead refuse to publish
/// with a token that publishes perfectly well, and send the human to re-issue it
/// — so *nothing said* is kept apart from *said, and `gist` was not among them*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Scopes {
    /// GitHub named them. Empty is a classic token with no scopes ticked, which
    /// really can do nothing.
    Named(Vec<String>),

    /// GitHub named none, which says nothing either way.
    Unsaid,
}

impl Scopes {
    /// Whether this token is known *not* to carry `scope`.
    ///
    /// The question asked in the negative, and deliberately: what a caller wants
    /// to know is whether to refuse, and a token GitHub said nothing about is
    /// not one to refuse — it is one to try.
    pub(crate) fn known_to_lack(&self, scope: &str) -> bool {
        match self {
            Scopes::Named(named) => !named.iter().any(|named| named == scope),
            Scopes::Unsaid => false,
        }
    }
}

/// The scope publishing a share needs, which is the one Verkstead's own writes
/// to GitHub turn on.
pub(crate) const GIST: &str = "gist";

/// One `gh api -i` response, split into its headers and its body at the blank
/// line between them.
///
/// Both endings, because the separator is a header block's rather than a text
/// file's. Something that carried no blank line at all is read as a body on its
/// own: a response with no headers to read says nothing about scopes, which is
/// the same answer as a response whose headers did not mention them.
fn split(said: &str) -> (&str, &str) {
    for ending in ["\r\n\r\n", "\n\n"] {
        if let Some(at) = said.find(ending) {
            return (&said[..at], &said[at + ending.len()..]);
        }
    }

    ("", said)
}

/// The scopes named in a response's headers.
///
/// Matched without regard to case, the way a header name is compared everywhere:
/// `gh` prints them as GitHub sent them, and GitHub has spelled this one
/// `X-OAuth-Scopes` and `X-Oauth-Scopes` at different times.
fn scopes(headers: &str) -> Scopes {
    const NAMED: &str = "x-oauth-scopes:";

    for line in headers.lines() {
        let Some((name, said)) = line.split_once(':') else {
            continue;
        };

        if !format!("{name}:").eq_ignore_ascii_case(NAMED) {
            continue;
        }

        return Scopes::Named(
            said.split(',')
                .map(str::trim)
                .filter(|scope| !scope.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }

    Scopes::Unsaid
}

/// A gist Verkstead has just made: where a reader goes, where git puts the file
/// in, and what to take back if it never gets one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Gist {
    /// GitHub's id for it, which is what deleting one names.
    pub(crate) id: String,

    /// The page a link to a share points at.
    pub(crate) url: String,

    /// And the git remote behind that page, as GitHub gave it — a gist is a
    /// repository, and this is how the file that matters gets into one.
    pub(crate) push: String,
}

/// Make a secret gist holding `name`, and answer where it is.
///
/// **`content` is a placeholder rather than the file.** The Gists API's cap on
/// what a gist may be *created* with is undocumented and has been reported at a
/// megabyte for a decade; a share is several, mermaid and every diff riding
/// along. So what the API is asked for is a gist with the right name on it, and
/// the bytes arrive over git afterwards — which has no such cap, and which is
/// what GitHub's own documentation points at for a gist file too big to fetch
/// through the API. See [`crate::publishing`], where the two halves are one act.
///
/// Secret rather than public, which is `public: false`: a share is read by
/// whoever was sent the link and by nobody else, and GitHub's word for that is a
/// gist that is not listed. It is not private — possession of the link is the
/// whole of the privacy — which is what the Brief settled and what the human is
/// choosing when they publish.
///
/// `--input -` rather than a field per value: the description is somebody's
/// branch name and the file's name is built from it, and a request whose shape
/// depended on what `gh` makes of a bracket in a key would be one that broke on
/// a Conversation nobody could have predicted.
pub(crate) fn create_gist(
    gh: &Gh,
    token: &str,
    description: &str,
    name: &str,
    content: &str,
) -> Result<Gist, Trouble> {
    /// The three fields of the created gist worth having.
    #[derive(Deserialize)]
    struct Made {
        id: String,
        html_url: String,
        git_push_url: String,
    }

    let body = serde_json::json!({
        "description": description,
        "public": false,
        "files": { name: { "content": content } },
    });

    let said = gh.tell_as(
        token,
        &["api", "-X", "POST", "/gists", "--input", "-"],
        &body.to_string(),
    )?;

    let made: Made = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    Ok(Gist {
        id: made.id,
        url: made.html_url,
        push: made.git_push_url,
    })
}

/// And take one back.
///
/// What a publish that fell over after the gist was made does with it. A gist
/// holding a placeholder and no share is worse than no gist at all: it is a link
/// that resolves, to a file that says nothing, in an account the human will find
/// it in months later.
pub(crate) fn delete_gist(gh: &Gh, token: &str, id: &str) -> Result<(), Trouble> {
    gh.ask_as(token, &["api", "-X", "DELETE", &format!("/gists/{id}")])?;

    Ok(())
}

/// Say something on a pull request, and answer where it was said.
///
/// What the one-click share leaves behind: the link to a Published Share and an
/// itemization of what is in it — see [`crate::commenting`], which puts one on
/// every pull request a Conversation holds.
///
/// **As the configured token rather than as whoever the host is logged in as.**
/// A write is a write, and the rule [`create_gist`] follows holds here for the
/// same reason: a comment left under a login nobody chose is a comment in
/// somebody else's name on somebody else's pull request. The token comes from
/// the caller, which has already read it to publish with.
///
/// `gh api` in the repository rather than `gh pr comment`, so what comes back is
/// the comment as a resource — the human is owed a link to what was left in
/// their name — and `--input -` for [`create_gist`]'s reason: a comment is a
/// document, and a body on the command line would be one every shell has an
/// opinion about.
///
/// The number is a fact about a repository, so this is run inside the one the
/// pull request was opened in: `#7` in another repository is something else, or
/// nothing.
pub(crate) fn comment(
    gh: &Gh,
    repo: &Path,
    token: &str,
    number: i64,
    body: &str,
) -> Result<String, Trouble> {
    /// The one field of the comment GitHub makes that is worth keeping: where a
    /// human goes to read it.
    #[derive(Deserialize)]
    struct Said {
        html_url: String,
    }

    let said = gh.tell_in(
        repo,
        token,
        &[
            "api",
            "-X",
            "POST",
            &format!("repos/{{owner}}/{{repo}}/issues/{number}/comments"),
            "--input",
            "-",
        ],
        &serde_json::json!({ "body": body }).to_string(),
    )?;

    let said: Said = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    Ok(said.html_url)
}

/// The pull request on `branch`, as the host's `gh` finds it.
///
/// The three facts worth recording and no more — see
/// [`store::PullRequest`]. Whether it is a draft, whether its checks are green
/// and what is on it are all things that move while the PR is open, and this
/// runs once.
pub(crate) fn pull_request(
    gh: &Gh,
    repo: &Path,
    branch: &str,
) -> Result<store::PullRequest, Trouble> {
    /// What `--json number,title,url` comes back as.
    #[derive(Deserialize)]
    struct Opened {
        number: i64,
        title: String,
        url: String,
    }

    // `--` is not gh's; the branch goes where gh takes a PR selector, which is a
    // number, a URL or a branch name.
    let said = gh.ask(repo, &["pr", "view", branch, "--json", "number,title,url"])?;

    let opened: Opened = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    Ok(store::PullRequest {
        number: opened.number,
        title: opened.title,
        url: opened.url,
        // Unnamed: which repository this was asked in is what the caller already
        // knows, and the name on a recorded pull request is the label a reader
        // wants rather than anything written here. See [`store::PullRequest`].
        repo: None,
    })
}

/// One check GitHub is running against a pull request's head commit.
///
/// The name is what a human calls it by and what Verkstead counts fix attempts
/// against — see [`store::fix_attempts`]. The link is where the run itself is,
/// which is the one thing a stop over a red check cannot be read without.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Check {
    pub(crate) name: String,
    pub(crate) how: Checked,

    /// Where the run is, as GitHub gives it. Empty where it gave none.
    pub(crate) link: String,
}

/// How one check is getting on.
///
/// Three answers rather than GitHub's dozen, because three is what wrap-up
/// decides between: a green check settles, a red one dispatches a fix session,
/// and one still running is nothing to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Checked {
    /// It finished and it is green. A check that was skipped or that reported
    /// nothing either way counts here: neither is something to fix.
    Passed,

    /// It finished and it is not green.
    ///
    /// Cancelled counts here, which is the one reading worth saying out loud: a
    /// cancelled run is not passing and is not going to start passing on its
    /// own, so reading it as *still running* would be a wrap-up that waited for
    /// ever rather than one that eventually asks the human.
    Failed,

    /// It has not finished.
    Running,
}

/// Every check on pull request `number`, as the host's `gh` finds them now.
///
/// `pr view --json statusCheckRollup` rather than `pr checks`, and the reason is
/// the exit status: `pr checks` reports what it found by failing — one status
/// for red and another for still-running — so a red suite would arrive here as
/// [`Trouble`], which is what *Verkstead could not ask* means. This asks a
/// question that has an answer.
///
/// The rollup is about the pull request's head commit, so it moves when a fix
/// session pushes: the run that was green belonged to the commit before it, and
/// what comes back here is the new one — see [`crate::checks`], where that is
/// what puts a settled wrap-up back to waiting.
///
/// A pull request with nothing running against it comes back empty, which is a
/// repository with no CI. That is nothing to wait on rather than something
/// missing — see [`crate::checks`], where it is read as green.
///
/// **The head is asked for beside them**, because the rollup on its own does not
/// say which commit it is about. GitHub answers this pull request as its record
/// currently stands, and that record is behind the branch for a while after a
/// push: what came back here on 2026-08-29, with the branch three commits along,
/// was a green suite belonging to the head before them. A green rollup is only
/// evidence about the commit it names, so the commit it names comes back too —
/// see [`crate::checks`], where it is compared with what origin is holding.
pub(crate) fn checks(gh: &Gh, repo: &Path, number: i64) -> Result<Suite, Trouble> {
    /// What `--json statusCheckRollup,headRefOid` comes back as.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Rollup {
        #[serde(default)]
        head_ref_oid: String,
        #[serde(default)]
        status_check_rollup: Vec<Reported>,
    }

    let said = gh.ask(
        repo,
        &[
            "pr",
            "view",
            &number.to_string(),
            "--json",
            "statusCheckRollup,headRefOid",
        ],
    )?;

    let rollup: Rollup = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    Ok(Suite {
        head: rollup.head_ref_oid,
        checks: read_checks(rollup.status_check_rollup),
    })
}

/// What GitHub says about a pull request's checks, and which commit it is
/// saying it about.
///
/// The two together rather than the checks alone: a rollup is a fact about one
/// commit, and a wrap-up that read it as a fact about the branch would finish
/// over whatever had been pushed since — see [`crate::checks`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Suite {
    /// The commit GitHub has the pull request's branch on, which is the commit
    /// the checks below are about.
    ///
    /// Empty where `gh` answered without one, which is a question that came back
    /// short rather than a head of nothing: nothing is concluded from it and the
    /// checks stand on their own, as they did before this was asked for.
    pub(crate) head: String,

    /// Every check GitHub reported against that commit, which is empty for a
    /// commit nothing has run against.
    pub(crate) checks: Vec<Check>,
}

/// One entry of `statusCheckRollup`, which is one of two different things.
///
/// A `CheckRun` is an Actions job and carries a `status` and a `conclusion`; a
/// `StatusContext` is the older commit-status API and carries a `state` alone.
/// Both are read here rather than only the first, because a repository is free
/// to use either and a Verkstead that saw only Actions would call a red
/// Buildkite green.
///
/// Out here rather than inside [`checks`] because two questions come back with
/// this field on them: the watcher's, which asks about the checks alone, and the
/// details pane's, which asks about the whole pull request. One shape and one
/// reading for the two, so the icon on the card and the list in the pane cannot
/// come to disagree about a suite they are both looking at.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reported {
    #[serde(default)]
    name: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    details_url: String,
    #[serde(default)]
    target_url: String,
}

/// What GitHub reported, as the checks Verkstead reads: a name to count and
/// call it by, one of three words for how it is getting on, and where its run
/// is.
fn read_checks(reported: Vec<Reported>) -> Vec<Check> {
    reported
        .into_iter()
        .map(|reported| Check {
            name: match (reported.name.is_empty(), reported.context.is_empty()) {
                // A check with neither is one GitHub declined to name, and it
                // still has to be counted against something.
                (true, true) => "a check GitHub did not name".to_owned(),
                (true, false) => reported.context,
                _ => reported.name,
            },
            how: read_check(&reported.status, &reported.conclusion, &reported.state),
            link: match reported.details_url.is_empty() {
                true => reported.target_url,
                false => reported.details_url,
            },
        })
        .collect()
}

/// How one check is getting on, out of the three words GitHub says it in.
///
/// `status` is the Actions job's — anything but `COMPLETED` means it is still
/// going, whatever the conclusion column happens to hold. Then the outcome,
/// which is the job's `conclusion` or the older API's `state` depending on which
/// kind of thing this is; only one of the two is ever set.
///
/// Green is listed rather than red, which is the way round that matters: a word
/// this does not know is read as a failure, so a GitHub that invents a new way
/// for a check to go wrong stops a wrap-up rather than settling it.
fn read_check(status: &str, conclusion: &str, state: &str) -> Checked {
    if !status.is_empty() && !status.eq_ignore_ascii_case("COMPLETED") {
        return Checked::Running;
    }

    let outcome = match conclusion.is_empty() {
        true => state,
        false => conclusion,
    };

    // Nothing said either way. A `StatusContext` that has been created and not
    // reported yet, or a check run with no outcome on it at all.
    if outcome.is_empty() || outcome.eq_ignore_ascii_case("PENDING") {
        return Checked::Running;
    }

    match outcome.to_ascii_uppercase().as_str() {
        "SUCCESS" | "NEUTRAL" | "SKIPPED" => Checked::Passed,
        _ => Checked::Failed,
    }
}

/// One thing said on a pull request, as the wrap-up's comment watcher reads it.
///
/// Not [`verkstead_render::Comment`], which is the details pane's and arrives
/// rendered: this one is read by an agent rather than by a browser, so it keeps
/// the markdown it was written in — and it carries the identity, which is the
/// whole of how a comment already dispatched for is told from a new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Comment {
    /// What this comment is, for as long as it exists. GitHub's node id where
    /// there is one, its URL where there is not — see [`identity`].
    pub(crate) which: String,

    /// Who said it, empty for an account that has since gone.
    pub(crate) author: String,

    /// When, RFC 3339, as GitHub gives it.
    pub(crate) at: String,

    /// Where on the branch they said it: the file and the line, for a comment
    /// left on the diff. Empty for one said about the pull request as a whole,
    /// which is what the conversation and a review's own words are.
    pub(crate) about: String,

    /// And what they said, in the markdown they wrote.
    pub(crate) markdown: String,
}

/// Everything said on pull request `number`, as the host's `gh` finds it now.
///
/// **Three places a human writes, and all three count.** The pull request's
/// conversation, the words at the top of a review, and the comments left on the
/// lines of the diff — the last of which is where most code feedback actually
/// lands, so a Verkstead that read only the conversation would miss the reviews
/// it most needs to act on. They come back as one list in the order they were
/// said in, because that is what they are: one human talking.
///
/// Two calls rather than three: `pr view` answers for the conversation and the
/// reviews together, and the comments on the diff are the REST endpoint's, which
/// `gh pr view` has no field for.
///
/// Read afresh every poll and never written down. What Verkstead remembers is
/// which of them it has already dispatched a session for, which is a much
/// smaller thing than the comments themselves.
///
/// A pull request nobody has said anything on comes back empty, which is every
/// pull request the moment it opens.
pub(crate) fn comments(gh: &Gh, repo: &Path, number: i64) -> Result<Vec<Comment>, Trouble> {
    let mut said = conversation(gh, repo, number)?;
    said.extend(on_the_diff(gh, repo, number)?);

    // In the order they were said in, across all three places. The timestamps
    // are RFC 3339 in UTC, which sorts as text.
    said.sort_by(|one, next| one.at.cmp(&next.at));

    Ok(said)
}

/// What was said about the pull request as a whole: its conversation, and the
/// words at the top of each review.
///
/// A review with nothing written at the top of it is left out. An approval with
/// no words is somebody saying they are happy, and dispatching a session to
/// address it would be Verkstead inventing work out of agreement.
fn conversation(gh: &Gh, repo: &Path, number: i64) -> Result<Vec<Comment>, Trouble> {
    /// What `--json comments,reviews` comes back as.
    #[derive(Deserialize)]
    struct Said {
        #[serde(default)]
        comments: Vec<One>,
        #[serde(default)]
        reviews: Vec<Reviewed>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct One {
        #[serde(default)]
        id: String,
        #[serde(default)]
        url: String,
        #[serde(default)]
        author: Login,
        #[serde(default)]
        body: String,
        #[serde(default)]
        created_at: String,
    }

    /// A review carries its time in another field from a comment's, and that is
    /// the whole of the difference worth reading: what a review *is* — approved,
    /// changes requested — is not something to address, and what it says is.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Reviewed {
        #[serde(default)]
        id: String,
        #[serde(default)]
        author: Login,
        #[serde(default)]
        body: String,
        #[serde(default)]
        submitted_at: String,
    }

    let said = gh.ask(
        repo,
        &[
            "pr",
            "view",
            &number.to_string(),
            "--json",
            "comments,reviews",
        ],
    )?;

    let said: Said = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    let comments = said.comments.into_iter().map(|one| Comment {
        which: identity(&one.id, &one.url, &one.author.login, &one.created_at),
        author: one.author.login,
        at: one.created_at,
        about: String::new(),
        markdown: one.body,
    });

    let reviews = said
        .reviews
        .into_iter()
        .filter(|review| !review.body.trim().is_empty())
        .map(|review| Comment {
            which: identity(&review.id, "", &review.author.login, &review.submitted_at),
            author: review.author.login,
            at: review.submitted_at,
            about: String::new(),
            markdown: review.body,
        });

    Ok(comments.chain(reviews).collect())
}

/// And what was said on the lines of the diff, which is where a review of code
/// mostly happens.
///
/// `gh api` rather than `gh pr view`, because `pr view` has no field for these:
/// they are the REST API's review comments, and `{owner}` and `{repo}` are
/// filled in by `gh` from the repository it is run inside. Paginated, so a
/// review that left thirty comments arrives whole rather than one page of it.
fn on_the_diff(gh: &Gh, repo: &Path, number: i64) -> Result<Vec<Comment>, Trouble> {
    /// One entry of it. The REST API spells its fields with underscores and puts
    /// the author under `user`, where the GraphQL one `gh pr view` wraps says
    /// `author` — which is why this is read apart from the reviews above rather
    /// than into the same shape.
    #[derive(Deserialize)]
    struct OnALine {
        #[serde(default)]
        node_id: String,
        #[serde(default)]
        html_url: String,
        #[serde(default)]
        user: Login,
        #[serde(default)]
        body: String,
        #[serde(default)]
        created_at: String,
        #[serde(default)]
        path: String,

        /// Which line it is against now. Null for a comment on a line the branch
        /// has since moved past, which is a comment to read all the same.
        line: Option<i64>,
    }

    let said = gh.ask(
        repo,
        &[
            "api",
            &format!("repos/{{owner}}/{{repo}}/pulls/{number}/comments"),
            "--paginate",
        ],
    )?;

    // `--paginate` concatenates the pages as separate arrays where it cannot
    // merge them, so this reads what came back as one array and says so plainly
    // when it is something else.
    let said: Vec<OnALine> = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    Ok(said
        .into_iter()
        .map(|one| Comment {
            which: identity(
                &one.node_id,
                &one.html_url,
                &one.user.login,
                &one.created_at,
            ),
            author: one.user.login,
            at: one.created_at,
            about: where_said(&one.path, one.line),
            markdown: one.body,
        })
        .collect())
}

/// Who said it. A comment left by an account that has since gone comes back with
/// no author at all, which is a comment to act on rather than one to drop.
#[derive(Default, Deserialize)]
struct Login {
    #[serde(default)]
    login: String,
}

/// Where on the branch a comment was left, as the session that reads it is told.
///
/// The file alone where GitHub gave no line, which is a comment on a line the
/// branch has moved past — still worth pointing at the file, because that is
/// where whoever fixes it has to look.
fn where_said(path: &str, line: Option<i64>) -> String {
    match (path.trim().is_empty(), line) {
        (true, _) => String::new(),
        (false, Some(line)) => format!("`{path}` line {line}"),
        (false, None) => format!("`{path}`"),
    }
}

/// What a comment is called, for the table that remembers which ones have been
/// dispatched for.
///
/// GitHub's node id first, because that is what it is. The URL after it, which
/// carries the same id in a different spelling. And, where a `gh` gave neither,
/// who said it and when — weaker than the other two, but a comment with no
/// identity at all would be one dispatched for again on every poll for ever,
/// which is the one outcome worth ruling out.
fn identity(id: &str, url: &str, author: &str, at: &str) -> String {
    for named in [id, url] {
        if !named.trim().is_empty() {
            return named.trim().to_owned();
        }
    }

    format!("{author} at {at}")
}

/// What one reading of a pull request comes back with.
///
/// Two things out of the one question: what the details pane draws, and the
/// checks as [`checks`] would have read them. The second is not a second copy of
/// the first — it is what the rollup written down beside the Conversation is
/// taken from, and the caller wants the checks in the shape the aggregate is
/// read out of rather than in the shape a page draws.
pub(crate) struct Details {
    pub(crate) pane: verkstead_render::PullRequestDetails,
    pub(crate) checks: Vec<Check>,
}

/// What is on the pull request now: the commits it carries, what GitHub is
/// running against it and what has been said about it.
///
/// Fetched rather than remembered, which is the whole arrangement — the same way
/// the task list is read off the Worktree rather than stored. A PR is being
/// worked on while the human is looking at it, and a commit list written down
/// when it opened would be wrong by the time anybody read it.
///
/// The checks come back on the same question rather than a second one, `gh`
/// taking a list of fields: two questions would be two round trips over somebody
/// else's network, and a commit list from before a push beside checks from after
/// it.
pub(crate) fn details(gh: &Gh, repo: &Path, number: i64) -> Result<Details, Trouble> {
    /// What `--json commits,comments,statusCheckRollup` comes back as, of which
    /// this takes the fields the details pane draws.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Carried {
        #[serde(default)]
        commits: Vec<Landed>,
        #[serde(default)]
        comments: Vec<Said>,
        #[serde(default)]
        status_check_rollup: Vec<Reported>,
    }

    /// `gh` writes its JSON in the field names the GraphQL API uses, which are
    /// camel case: the rename is what holds those to this crate's own spelling.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Landed {
        oid: String,
        #[serde(default)]
        message_headline: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Said {
        #[serde(default)]
        author: Login,
        #[serde(default)]
        body: String,
        #[serde(default)]
        created_at: String,
    }

    let said = gh.ask(
        repo,
        &[
            "pr",
            "view",
            &number.to_string(),
            "--json",
            "commits,comments,statusCheckRollup",
        ],
    )?;

    let carried: Carried = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    let checks = read_checks(carried.status_check_rollup);

    Ok(Details {
        pane: verkstead_render::pull_request_details(
            carried
                .commits
                .into_iter()
                .map(|commit| verkstead_render::PullRequestCommit {
                    sha: commit.oid,
                    subject: commit.message_headline,
                })
                .collect(),
            carried
                .comments
                .into_iter()
                .map(|comment| verkstead_render::Comment {
                    author: comment.author.login,
                    at: comment.created_at,
                    markdown: comment.body,
                })
                .collect(),
            checks.iter().map(drawn).collect(),
        ),
        checks,
    })
}

/// One check as the details pane draws it.
///
/// The same three facts under a different set of names: this side of the wire is
/// the workbench's vocabulary, and [`Check`] is `gh`'s reading.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A `gh` that says `said` on stdout, or `wrong` on stderr and exits 1.
    ///
    /// A script rather than a mock, because what is being tested is the reading
    /// of a program: its stdout, its stderr and its status are the whole of the
    /// interface, and something that stood in for the process would be testing
    /// this module against itself.
    fn stub(said: &str, wrong: &str) -> (tempfile::TempDir, Gh) {
        let dir = tempfile::tempdir().unwrap();

        (
            dir,
            Gh::running(vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                format!(
                    "if [ -n '{wrong}' ]; then printf '%s' '{wrong}' >&2; exit 1; fi; \
                     printf '%s' '{said}'"
                ),
                // `sh -c` gives `$0` to the script's own name, so the arguments
                // Verkstead passes land in `$1` onwards and are simply ignored
                // here. What matters is that they were passed at all.
                "gh".to_owned(),
            ]),
        )
    }

    /// The ordinary answer: a branch with a PR on it.
    #[test]
    fn a_branch_with_a_pull_request_reads_back_as_its_number_title_and_url() {
        let (dir, gh) = stub(
            r#"{"number":41,"title":"Rate limiting","url":"https://github.com/tobico/verkstead/pull/41"}"#,
            "",
        );

        assert_eq!(
            pull_request(&gh, dir.path(), "rate-limiting").unwrap(),
            store::PullRequest {
                number: 41,
                title: "Rate limiting".to_owned(),
                url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
                // Which repository is the caller's to know: what `gh` was asked
                // in is not something it reads back.
                repo: None,
            },
        );
    }

    /// The one every finish step can reasonably run into: the branch is pushed
    /// and nothing was opened on it.
    #[test]
    fn a_branch_with_no_pull_request_is_an_answer_rather_than_a_failure() {
        let (dir, gh) = stub(
            "",
            "no pull requests found for branch \\\"rate-limiting\\\"",
        );

        assert_eq!(
            pull_request(&gh, dir.path(), "rate-limiting"),
            Err(Trouble::NoPullRequest),
        );
    }

    /// And the two that are about the machine rather than about the work.
    #[test]
    fn an_unauthenticated_gh_and_a_remoteless_repository_each_say_so() {
        let (dir, gh) = stub(
            "",
            "gh: To use GitHub CLI in a GitHub Actions workflow, run: gh auth login",
        );

        assert_eq!(
            pull_request(&gh, dir.path(), "rate-limiting"),
            Err(Trouble::NotLoggedIn),
        );
        assert!(
            Trouble::NotLoggedIn.why().contains("settings page"),
            "and says where the token goes: {}",
            Trouble::NotLoggedIn.why(),
        );

        let (dir, gh) = stub(
            "",
            "none of the git remotes configured for this repository point to a known GitHub host",
        );

        assert_eq!(
            pull_request(&gh, dir.path(), "rate-limiting"),
            Err(Trouble::NoRemote),
        );
    }

    /// A `gh` that is not installed at all, which is the answer no stderr can
    /// carry: there was no process to print one.
    #[test]
    fn a_machine_with_no_gh_says_that_rather_than_quoting_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let gh = Gh::running(vec!["verkstead-has-no-such-program".to_owned()]);

        assert_eq!(
            pull_request(&gh, dir.path(), "rate-limiting"),
            Err(Trouble::NoGh),
        );
        assert!(
            Trouble::NoGh.why().contains("PATH"),
            "and says where it looked: {}",
            Trouble::NoGh.why(),
        );
    }

    /// Anything else keeps `gh`'s own words: a message this does not know is
    /// still a message the human can act on.
    #[test]
    fn an_unfamiliar_refusal_is_quoted_rather_than_guessed_at() {
        let (dir, gh) = stub("", "HTTP 502: Bad gateway");

        assert_eq!(
            pull_request(&gh, dir.path(), "rate-limiting"),
            Err(Trouble::Refused("HTTP 502: Bad gateway".to_owned())),
        );
    }

    /// What the details pane fetches: the commit list and the comments, with the
    /// comments rendered as the markdown they are written in.
    #[test]
    fn the_commit_list_and_the_comments_are_read_off_the_pull_request() {
        let (dir, gh) = stub(
            r#"{"commits":[{"oid":"c0ffee1","messageHeadline":"feat: count the requests"}],
                "comments":[{"author":{"login":"tobico"},"body":"Looks **good**.","createdAt":"2026-08-21T09:00:00Z"}]}"#,
            "",
        );

        let pane = details(&gh, dir.path(), 41).unwrap().pane;

        assert_eq!(pane.commits.len(), 1);
        assert_eq!(pane.commits[0].sha, "c0ffee1");
        assert_eq!(pane.commits[0].subject, "feat: count the requests");

        assert_eq!(pane.comments.len(), 1);
        assert_eq!(pane.comments[0].author, "tobico");
        assert_eq!(pane.comments[0].at, "2026-08-21T09:00:00Z");
        assert!(
            pane.comments[0].html.contains("<strong>good</strong>"),
            "a comment is markdown, and it arrives rendered: {:?}",
            pane.comments[0].html,
        );
    }

    /// And the checks, off the same answer: the pane lists each of them by name
    /// with the run to follow, and the caller gets them in `gh`'s own shape as
    /// well, which is what the rollup beside the Conversation is written from.
    #[test]
    fn the_checks_are_read_off_the_same_answer_the_commits_are() {
        let (dir, gh) = stub(
            r#"{"commits":[],"comments":[],"statusCheckRollup":[
                {"__typename":"CheckRun","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2",
                 "name":"Rust","status":"COMPLETED"},
                {"__typename":"CheckRun","conclusion":"FAILURE","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/3",
                 "name":"Viewer","status":"COMPLETED"}]}"#,
            "",
        );

        let read = details(&gh, dir.path(), 41).unwrap();

        assert_eq!(
            read.pane.checks,
            [
                verkstead_render::PullRequestCheck {
                    name: "Rust".to_owned(),
                    how: verkstead_render::Checked::Passed,
                    link: "https://github.com/tobico/verkstead/actions/runs/1/job/2".to_owned(),
                },
                verkstead_render::PullRequestCheck {
                    name: "Viewer".to_owned(),
                    how: verkstead_render::Checked::Failed,
                    link: "https://github.com/tobico/verkstead/actions/runs/1/job/3".to_owned(),
                },
            ],
        );

        assert_eq!(
            read.checks
                .iter()
                .map(|check| check.how)
                .collect::<Vec<_>>(),
            [Checked::Passed, Checked::Failed],
        );
    }

    /// A check GitHub gave no run for is a name and nothing to follow, which the
    /// pane draws as plain text — the same empty link the fix session's list
    /// falls back to.
    #[test]
    fn a_check_with_no_run_reaches_the_pane_as_a_name_alone() {
        let (dir, gh) = stub(
            r#"{"commits":[],"comments":[],"statusCheckRollup":[
                {"__typename":"StatusContext","context":"buildkite","state":"PENDING"}]}"#,
            "",
        );

        let pane = details(&gh, dir.path(), 41).unwrap().pane;

        assert_eq!(
            pane.checks,
            [verkstead_render::PullRequestCheck {
                name: "buildkite".to_owned(),
                how: verkstead_render::Checked::Running,
                link: String::new(),
            }],
        );
    }

    /// The ordinary green suite, in the words a real `gh` answered a real pull
    /// request of this repository with.
    #[test]
    fn a_green_suite_reads_back_as_every_check_passing() {
        let (dir, gh) = stub(
            r#"{"statusCheckRollup":[
                {"__typename":"CheckRun","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2",
                 "name":"Rust","status":"COMPLETED","workflowName":"CI"},
                {"__typename":"CheckRun","conclusion":"SUCCESS","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/3",
                 "name":"Viewer","status":"COMPLETED","workflowName":"CI"}]}"#,
            "",
        );

        assert_eq!(
            checks(&gh, dir.path(), 41).unwrap().checks,
            vec![
                Check {
                    name: "Rust".to_owned(),
                    how: Checked::Passed,
                    link: "https://github.com/tobico/verkstead/actions/runs/1/job/2".to_owned(),
                },
                Check {
                    name: "Viewer".to_owned(),
                    how: Checked::Passed,
                    link: "https://github.com/tobico/verkstead/actions/runs/1/job/3".to_owned(),
                },
            ],
        );
    }

    /// And the one a fix session is dispatched for: one job red beside a green
    /// one, which is what a failing suite nearly always looks like.
    #[test]
    fn a_red_check_is_told_apart_from_the_green_ones_beside_it() {
        let (dir, gh) = stub(
            r#"{"statusCheckRollup":[
                {"__typename":"CheckRun","conclusion":"FAILURE","detailsUrl":"https://github.com/tobico/verkstead/actions/runs/1/job/2",
                 "name":"Rust","status":"COMPLETED"},
                {"__typename":"CheckRun","conclusion":"SUCCESS","detailsUrl":"","name":"Viewer","status":"COMPLETED"}]}"#,
            "",
        );

        let checks = checks(&gh, dir.path(), 41).unwrap().checks;

        assert_eq!(checks[0].how, Checked::Failed);
        assert_eq!(checks[0].name, "Rust");
        assert_eq!(checks[1].how, Checked::Passed);
    }

    /// A suite that has not finished is nothing to do, so *running* has to be an
    /// answer of its own — whatever the conclusion column happens to hold while
    /// the job is still going.
    #[test]
    fn a_check_that_has_not_finished_reads_as_running() {
        let (dir, gh) = stub(
            r#"{"statusCheckRollup":[
                {"__typename":"CheckRun","conclusion":"","name":"Rust","status":"IN_PROGRESS"},
                {"__typename":"CheckRun","conclusion":"","name":"Viewer","status":"QUEUED"}]}"#,
            "",
        );

        let checks = checks(&gh, dir.path(), 41).unwrap().checks;

        assert!(
            checks.iter().all(|check| check.how == Checked::Running),
            "{checks:?}"
        );
    }

    /// The older commit-status API, which a repository is free to be using: the
    /// name is in another field and the outcome is in a third. A Verkstead that
    /// read only Actions would call a red Buildkite green.
    #[test]
    fn the_older_status_api_is_read_as_well_as_actions() {
        let (dir, gh) = stub(
            r#"{"statusCheckRollup":[
                {"__typename":"StatusContext","context":"ci/buildkite","state":"FAILURE",
                 "targetUrl":"https://buildkite.example/1"},
                {"__typename":"StatusContext","context":"ci/pending","state":"PENDING","targetUrl":""}]}"#,
            "",
        );

        assert_eq!(
            checks(&gh, dir.path(), 41).unwrap().checks,
            vec![
                Check {
                    name: "ci/buildkite".to_owned(),
                    how: Checked::Failed,
                    link: "https://buildkite.example/1".to_owned(),
                },
                Check {
                    name: "ci/pending".to_owned(),
                    how: Checked::Running,
                    link: String::new(),
                },
            ],
        );
    }

    /// Green is the listed answer and everything else is red, which is the way
    /// round that matters: a way for a check to go wrong that this does not know
    /// about should stop a wrap-up rather than settle one.
    #[test]
    fn an_outcome_this_does_not_know_is_read_as_a_failure() {
        for outcome in ["TIMED_OUT", "CANCELLED", "ACTION_REQUIRED", "SOMETHING_NEW"] {
            assert_eq!(
                read_check("COMPLETED", outcome, ""),
                Checked::Failed,
                "{outcome} is not a check anybody should call green",
            );
        }

        // And the three that are green, of which two are green by not having run.
        for outcome in ["SUCCESS", "NEUTRAL", "SKIPPED"] {
            assert_eq!(read_check("COMPLETED", outcome, ""), Checked::Passed);
        }
    }

    /// A pull request with nothing running against it, which is a repository
    /// with no CI. An answer rather than a failure — what wrap-up makes of it is
    /// its own business.
    #[test]
    fn a_pull_request_with_no_checks_at_all_reads_back_as_none() {
        let (dir, gh) = stub(r#"{"statusCheckRollup":[]}"#, "");

        assert!(checks(&gh, dir.path(), 41).unwrap().checks.is_empty());
    }

    /// And the reason the checks are asked for this way at all: a `gh` that
    /// cannot answer has to arrive as [`Trouble`], because *Verkstead could not
    /// ask* is a third thing beside green and red.
    #[test]
    fn a_gh_that_cannot_answer_about_checks_says_so_rather_than_saying_green() {
        let (dir, gh) = stub("", "gh: To use GitHub CLI, run: gh auth login");

        assert_eq!(checks(&gh, dir.path(), 41), Err(Trouble::NotLoggedIn));
    }

    /// A PR nobody has said anything on and nothing is running against, which is
    /// every PR the moment it opens in a repository with no CI.
    #[test]
    fn a_pull_request_with_no_comments_reads_back_as_none() {
        let (dir, gh) = stub(r#"{"commits":[],"comments":[]}"#, "");

        let pane = details(&gh, dir.path(), 41).unwrap().pane;

        assert!(pane.commits.is_empty() && pane.comments.is_empty() && pane.checks.is_empty());
    }

    /// A `gh` that answers `viewed` for `gh pr view` and `on_the_diff` for `gh
    /// api`.
    ///
    /// Reading everything said on a pull request takes both: `gh pr view` has no
    /// field for the comments left on the lines of the diff, so those are the
    /// REST endpoint's and arrive spelled the REST API's way.
    fn stub_reading(viewed: &str, on_the_diff: &str) -> (tempfile::TempDir, Gh) {
        let dir = tempfile::tempdir().unwrap();

        (
            dir,
            Gh::running(vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                format!(
                    "if [ \"$1\" = api ]; then printf '%s' '{on_the_diff}'; \
                     else printf '%s' '{viewed}'; fi"
                ),
                "gh".to_owned(),
            ]),
        )
    }

    /// All three places a human writes on a pull request, read as one list in
    /// the order they were said in: the conversation, the words at the top of a
    /// review, and the comments left on the lines of the diff.
    ///
    /// The last is where a review of code mostly happens, so a Verkstead that
    /// read only the first would miss the feedback it most needs to act on.
    #[test]
    fn everything_said_on_a_pull_request_reads_back_in_the_order_it_was_said() {
        let (dir, gh) = stub_reading(
            r#"{"comments":[{"id":"IC_1","url":"https://github.com/tobico/verkstead/pull/41#issuecomment-1",
                 "author":{"login":"tobico"},"body":"Reading this now.","createdAt":"2026-08-21T09:00:00Z"}],
                "reviews":[{"id":"PRR_1","author":{"login":"tobico"},"state":"CHANGES_REQUESTED",
                 "body":"Two things.","submittedAt":"2026-08-21T09:02:00Z"}]}"#,
            r#"[{"node_id":"PRRC_1","html_url":"https://github.com/tobico/verkstead/pull/41#discussion_r1",
                 "user":{"login":"tobico"},"body":"Wrong way round.","created_at":"2026-08-21T09:01:00Z",
                 "path":"src/window.rs","line":12}]"#,
        );

        assert_eq!(
            comments(&gh, dir.path(), 41).unwrap(),
            vec![
                Comment {
                    which: "IC_1".to_owned(),
                    author: "tobico".to_owned(),
                    at: "2026-08-21T09:00:00Z".to_owned(),
                    about: String::new(),
                    markdown: "Reading this now.".to_owned(),
                },
                Comment {
                    which: "PRRC_1".to_owned(),
                    author: "tobico".to_owned(),
                    at: "2026-08-21T09:01:00Z".to_owned(),
                    about: "`src/window.rs` line 12".to_owned(),
                    markdown: "Wrong way round.".to_owned(),
                },
                Comment {
                    which: "PRR_1".to_owned(),
                    author: "tobico".to_owned(),
                    at: "2026-08-21T09:02:00Z".to_owned(),
                    about: String::new(),
                    markdown: "Two things.".to_owned(),
                },
            ],
        );
    }

    /// A review with nothing written at the top of it is somebody saying they
    /// are happy. Dispatching a session to address it would be Verkstead
    /// inventing work out of agreement.
    #[test]
    fn a_review_with_no_words_in_it_is_nothing_to_address() {
        let (dir, gh) = stub_reading(
            r#"{"comments":[],"reviews":[{"id":"PRR_1","author":{"login":"tobico"},
                 "state":"APPROVED","body":"","submittedAt":"2026-08-21T09:02:00Z"}]}"#,
            "[]",
        );

        assert!(comments(&gh, dir.path(), 41).unwrap().is_empty());
    }

    /// Where a comment on the diff was left is half of what it means, so it
    /// travels with it — and a line GitHub no longer has is still a file worth
    /// pointing at.
    #[test]
    fn a_comment_on_the_diff_is_placed_by_its_file_and_line() {
        assert_eq!(
            where_said("src/window.rs", Some(12)),
            "`src/window.rs` line 12"
        );
        assert_eq!(where_said("src/window.rs", None), "`src/window.rs`");
        assert_eq!(where_said("", None), "");
    }

    /// The identity falls back rather than being empty, because a comment with
    /// no identity would be one dispatched for again on every poll for ever.
    #[test]
    fn a_comment_gh_gave_no_id_for_is_still_told_apart_from_the_next_one() {
        assert_eq!(
            identity("IC_1", "https://…/#issuecomment-1", "tobico", ""),
            "IC_1"
        );
        assert_eq!(
            identity("", "https://…/#issuecomment-1", "tobico", ""),
            "https://…/#issuecomment-1",
        );
        assert_eq!(
            identity("", "", "tobico", "2026-08-21T09:00:00Z"),
            "tobico at 2026-08-21T09:00:00Z",
        );
    }

    /// A `gh` that cannot answer about the comments is *not knowing* rather than
    /// *nobody said anything*, the same way the checks are — and that holds
    /// whichever of the two calls it is that cannot answer.
    #[test]
    fn a_gh_that_cannot_answer_about_comments_says_so_rather_than_saying_none() {
        let (dir, gh) = stub("", "gh: To use GitHub CLI, run: gh auth login");

        assert_eq!(comments(&gh, dir.path(), 41), Err(Trouble::NotLoggedIn));

        // The conversation answers and the comments on the diff do not, which is
        // an account that has the pull request and not the endpoint. Half the
        // feedback is not an answer, so this is trouble too.
        let dir = tempfile::tempdir().unwrap();
        let gh = Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "if [ \"$1\" = api ]; then printf 'HTTP 404: Not Found\\n' >&2; exit 1; fi; \
             printf '%s' '{\"comments\":[],\"reviews\":[]}'"
                .to_owned(),
            "gh".to_owned(),
        ]);

        assert_eq!(
            comments(&gh, dir.path(), 41),
            Err(Trouble::Refused("HTTP 404: Not Found".to_owned())),
        );
    }
    /// A `gh` that answers every ask with the `GH_TOKEN` it was run with, put
    /// where that answer has a word to spare: the pull request's title, the
    /// check's name, the comment's author. `unset` where it was run with none.
    ///
    /// The whole of what this module does with a token is hand it to a child
    /// process, so a child that says what it was handed is the only witness
    /// worth having.
    fn stub_saying_its_token() -> (tempfile::TempDir, Gh) {
        let dir = tempfile::tempdir().unwrap();

        (
            dir,
            Gh::running(vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                // `$0` is the script's own name, so Verkstead's arguments are
                // `$1` onwards: `pr view <selector> --json <fields>`, or the
                // `api` call the comments on the diff come from.
                r#"token="${GH_TOKEN-unset}"
                   if [ "$1" = api ]; then printf '[]'; exit 0; fi
                   case "$5" in
                     number,title,url)
                       printf '{"number":41,"title":"%s","url":"u"}' "$token" ;;
                     statusCheckRollup,headRefOid)
                       printf '{"statusCheckRollup":[{"name":"%s","status":"COMPLETED","conclusion":"SUCCESS"}]}' "$token" ;;
                     comments,reviews)
                       printf '{"comments":[{"id":"IC_1","url":"u","author":{"login":"%s"},"body":"b","createdAt":"t"}],"reviews":[]}' "$token" ;;
                   esac"#
                    .to_owned(),
                "gh".to_owned(),
            ]),
        )
    }

    /// The settings files in a directory of their own, with `secrets.yaml`
    /// written as the settings page would write it.
    fn configured(yaml: Option<&str>) -> (tempfile::TempDir, Settings) {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        if let Some(yaml) = yaml {
            std::fs::write(settings.secrets_path(), yaml).unwrap();
        }

        (dir, settings)
    }

    /// One token in `secrets.yaml` is the whole of Verkstead's GitHub auth:
    /// every host-side ask runs as it, so a machine with no `gh auth login`
    /// anywhere on it still reads pull requests, checks and comments.
    #[test]
    fn the_configured_token_authenticates_the_views_the_checks_and_the_comments() {
        let (repo, gh) = stub_saying_its_token();
        let (_data_dir, settings) = configured(Some("github_token: ghp_theconfiguredone\n"));
        let gh = gh.authenticated_by(settings);

        assert_eq!(
            pull_request(&gh, repo.path(), "rate-limiting")
                .unwrap()
                .title,
            "ghp_theconfiguredone",
        );
        assert_eq!(
            checks(&gh, repo.path(), 41).unwrap().checks[0].name,
            "ghp_theconfiguredone",
        );
        assert_eq!(
            comments(&gh, repo.path(), 41).unwrap()[0].author,
            "ghp_theconfiguredone",
        );
    }

    /// And it is read at the moment of the call rather than held from startup,
    /// so a token saved or rotated through the settings page is what the next
    /// `gh` runs as — with nothing restarted.
    #[test]
    fn a_token_saved_after_startup_is_what_the_next_gh_runs_as() {
        let (repo, gh) = stub_saying_its_token();
        let (_data_dir, settings) = configured(Some("github_token: the-first\n"));
        let gh = gh.authenticated_by(settings.clone());

        assert_eq!(
            pull_request(&gh, repo.path(), "rate-limiting")
                .unwrap()
                .title,
            "the-first",
        );

        std::fs::write(settings.secrets_path(), "github_token: the-second\n").unwrap();

        assert_eq!(
            pull_request(&gh, repo.path(), "rate-limiting")
                .unwrap()
                .title,
            "the-second",
        );
    }

    /// With nothing configured, nothing is set: the call is made as it always
    /// was and falls back to whatever login the host's `gh` has.
    ///
    /// Against what this test process itself holds, because that is what a
    /// child inherits — the claim is that Verkstead sets no `GH_TOKEN` of its
    /// own here, not that the machine running the tests has none.
    #[test]
    fn with_no_token_configured_the_host_gh_keeps_its_own_login() {
        let inherited = std::env::var("GH_TOKEN").unwrap_or_else(|_| "unset".to_owned());

        // A `Gh` with no settings behind it at all, which is every router that
        // has no Data Directory to read them out of.
        let (repo, gh) = stub_saying_its_token();

        assert_eq!(
            pull_request(&gh, repo.path(), "rate-limiting")
                .unwrap()
                .title,
            inherited,
        );

        // And the three ways settings say nothing: no file, an empty one, and
        // one that will not parse.
        for yaml in [None, Some(""), Some("github_token: [oh\n")] {
            let (repo, gh) = stub_saying_its_token();
            let (_data_dir, settings) = configured(yaml);
            let gh = gh.authenticated_by(settings);

            assert_eq!(
                pull_request(&gh, repo.path(), "rate-limiting")
                    .unwrap()
                    .title,
                inherited,
                "with secrets.yaml {yaml:?}",
            );
        }
    }

    /// Verifying a token is the one call that authenticates as something other
    /// than the configured token — a page cannot ask about a token it has not
    /// saved yet — so what has to be true is that the candidate is what reaches
    /// the child.
    #[test]
    fn a_token_is_verified_as_the_account_gh_answers_for_it() {
        let gh = Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            r#"printf '{"login":"%s"}' "${GH_TOKEN-unset}""#.to_owned(),
            "gh".to_owned(),
        ]);

        assert_eq!(
            authenticates_as(&gh, "ghp_thetoken").unwrap().login,
            "ghp_thetoken",
            "the candidate token is what gh was run with",
        );
    }

    /// And the candidate wins over whatever is configured: saving a second token
    /// must not come back with the first one's account.
    #[test]
    fn the_token_being_verified_is_the_one_asked_about_rather_than_the_saved_one() {
        let gh = Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            r#"printf '{"login":"%s"}' "${GH_TOKEN-unset}""#.to_owned(),
            "gh".to_owned(),
        ]);

        let (_data_dir, settings) = configured(Some("github_token: the-saved-one\n"));

        assert_eq!(
            authenticates_as(&gh.authenticated_by(settings), "the-candidate")
                .unwrap()
                .login,
            "the-candidate",
        );
    }

    /// The scopes come out of the headers, which is why the whole response is
    /// asked for: what a token may do is on no resource GitHub serves.
    #[test]
    fn a_token_carries_the_scopes_github_named_in_its_headers() {
        let gh = answering(concat!(
            "HTTP/2.0 200 OK\r\n",
            "X-Oauth-Scopes: repo, gist, workflow\r\n",
            "\r\n",
            r#"{"login":"tobico"}"#,
        ));

        let account = authenticates_as(&gh, "ghp_thetoken").unwrap();

        assert_eq!(account.login, "tobico");
        assert!(!account.scopes.known_to_lack(GIST));
        assert!(account.scopes.known_to_lack("admin:org"));
    }

    /// And a token issued for reading repositories is known to lack it, which is
    /// what a publish refuses on.
    #[test]
    fn a_token_without_the_gist_scope_is_known_to_lack_it() {
        let gh = answering(concat!(
            "HTTP/2.0 200 OK\r\n",
            "X-Oauth-Scopes: read:org, repo, workflow\r\n",
            "\r\n",
            r#"{"login":"tobico"}"#,
        ));

        assert!(
            authenticates_as(&gh, "ghp_thetoken")
                .unwrap()
                .scopes
                .known_to_lack(GIST),
        );
    }

    /// A fine-grained token has permissions rather than scopes, and GitHub
    /// answers for one with no scopes header at all. Nothing said is not the
    /// same as none: read as an empty list it would have Verkstead refuse to
    /// publish with a token that publishes.
    #[test]
    fn a_token_github_named_no_scopes_for_is_not_known_to_lack_any() {
        let gh = answering(concat!(
            "HTTP/2.0 200 OK\r\n",
            "X-Accepted-Oauth-Scopes: \r\n",
            "\r\n",
            r#"{"login":"tobico"}"#,
        ));

        assert_eq!(
            authenticates_as(&gh, "ghp_thetoken").unwrap().scopes,
            Scopes::Unsaid,
        );
        assert!(
            !authenticates_as(&gh, "ghp_thetoken")
                .unwrap()
                .scopes
                .known_to_lack(GIST)
        );
    }

    /// A `gh` that prints what it is told to, headers and body alike.
    fn answering(said: &str) -> Gh {
        Gh::running(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            format!("printf '%s' {}", shell_quoted(said)),
            "gh".to_owned(),
        ])
    }

    /// One argument as `sh` takes it literally.
    fn shell_quoted(said: &str) -> String {
        format!("'{}'", said.replace('\'', r"'\''"))
    }

    /// A token GitHub will not accept is trouble in `gh`'s own words rather than
    /// a failure of its own, so that what the settings page prints is what the
    /// human would have seen in a terminal.
    #[test]
    fn a_token_github_refuses_comes_back_as_what_gh_said() {
        let (_dir, gh) = stub("", "gh: Bad credentials (HTTP 401)");

        assert_eq!(
            authenticates_as(&gh, "ghp_wrong"),
            Err(Trouble::Refused(
                "gh: Bad credentials (HTTP 401)".to_owned()
            )),
        );

        assert_eq!(
            authenticates_as(&gh, "ghp_wrong").unwrap_err().why(),
            "`gh` said: gh: Bad credentials (HTTP 401)",
        );
    }

    /// And a machine with no `gh` on it says that instead, rather than reading
    /// as a token the human should go and replace.
    #[test]
    fn a_machine_with_no_gh_cannot_verify_a_token_and_says_which() {
        let gh = Gh::running(vec!["verkstead-has-no-such-program".to_owned()]);

        assert_eq!(authenticates_as(&gh, "ghp_thetoken"), Err(Trouble::NoGh));
    }
}
