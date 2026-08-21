//! Verkstead's own reach into GitHub: the *host's* `gh`, run against a Repo.
//!
//! Not the sandbox's. The sessions keep using their own `gh` inside their
//! sandboxes to push and to open a pull request — that is the repository's
//! review process being followed by an agent — and this is the server asking
//! GitHub what happened afterwards: which PR the branch has, what is on it, and
//! what has been said about it.
//!
//! It reuses whatever auth the host already has. There is no token store here
//! and no GitHub App: `gh` on the machine Verkstead runs on is logged in as
//! somebody, and that is the account Verkstead reads as.
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

use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::store;

/// The host's `gh`, as Verkstead runs it.
#[derive(Debug, Clone)]
pub struct Gh {
    /// What to run, before the arguments of whatever is being asked. The whole
    /// argv rather than a path, so a test can stand a shell script where `gh`
    /// goes.
    program: Vec<String>,
}

impl Gh {
    /// The real thing: whatever `gh` the host has on its PATH.
    pub fn on_path() -> Gh {
        Gh::running(vec!["gh".to_owned()])
    }

    /// The same, with something else where `gh` goes — see [`Gh::program`].
    pub fn running(program: Vec<String>) -> Gh {
        Gh { program }
    }

    /// Run it inside `repo` and take its stdout, or say why there is none.
    ///
    /// Blocking, like everything that shells out. The callers are on
    /// `spawn_blocking` — see [`pull_request`] and [`details`].
    fn ask(&self, repo: &Path, args: &[&str]) -> Result<String, Trouble> {
        let (program, before) = self
            .program
            .split_first()
            .expect("a Gh is built with at least the program to run");

        let output = match Command::new(program)
            .args(before)
            .args(args)
            .current_dir(repo)
            // Nothing here is interactive: a `gh` that stopped to ask for a
            // password would be a server thread waiting on a terminal nobody is
            // at.
            .stdin(Stdio::null())
            .output()
        {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Trouble::NoGh);
            }
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
                "this machine's `gh` is not logged in, so Verkstead cannot ask GitHub anything"
                    .to_owned()
            }
            Trouble::NoRemote => {
                "the repository has no GitHub remote for `gh` to ask about".to_owned()
            }
            Trouble::NoPullRequest => {
                "GitHub has no pull request on this branch, so the finish step opened none"
                    .to_owned()
            }
            Trouble::Refused(said) => format!("`gh` said: {said}"),
        }
    }
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
    })
}

/// What is on the pull request now: the commits it carries and what has been
/// said about it.
///
/// Fetched rather than remembered, which is the whole arrangement — the same way
/// the task list is read off the Worktree rather than stored. A PR is being
/// worked on while the human is looking at it, and a commit list written down
/// when it opened would be wrong by the time anybody read it.
pub(crate) fn details(
    gh: &Gh,
    repo: &Path,
    number: i64,
) -> Result<verkstead_render::PullRequestDetails, Trouble> {
    /// What `--json commits,comments` comes back as, of which this takes the
    /// fields the details pane draws.
    #[derive(Deserialize)]
    struct Carried {
        #[serde(default)]
        commits: Vec<Landed>,
        #[serde(default)]
        comments: Vec<Said>,
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

    /// Who said it. A comment left by an account that has since gone comes back
    /// with no author at all, which is a comment to draw rather than a reason to
    /// refuse the whole pane.
    #[derive(Default, Deserialize)]
    struct Login {
        #[serde(default)]
        login: String,
    }

    let said = gh.ask(
        repo,
        &[
            "pr",
            "view",
            &number.to_string(),
            "--json",
            "commits,comments",
        ],
    )?;

    let carried: Carried = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    Ok(verkstead_render::pull_request_details(
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
    ))
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

        let details = details(&gh, dir.path(), 41).unwrap();

        assert_eq!(details.commits.len(), 1);
        assert_eq!(details.commits[0].sha, "c0ffee1");
        assert_eq!(details.commits[0].subject, "feat: count the requests");

        assert_eq!(details.comments.len(), 1);
        assert_eq!(details.comments[0].author, "tobico");
        assert_eq!(details.comments[0].at, "2026-08-21T09:00:00Z");
        assert!(
            details.comments[0].html.contains("<strong>good</strong>"),
            "a comment is markdown, and it arrives rendered: {:?}",
            details.comments[0].html,
        );
    }

    /// A PR nobody has said anything on, which is every PR the moment it opens.
    #[test]
    fn a_pull_request_with_no_comments_reads_back_as_none() {
        let (dir, gh) = stub(r#"{"commits":[],"comments":[]}"#, "");

        let details = details(&gh, dir.path(), 41).unwrap();

        assert!(details.commits.is_empty() && details.comments.is_empty());
    }
}
