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

/// One check GitHub is running against a pull request's head commit.
///
/// The name is what a human calls it by and what Verkstead counts fix attempts
/// against — see [`store::fix_attempts`]. The link is where the run itself is,
/// which is the one thing an Interruption about a red check cannot be answered
/// without.
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
/// A pull request with nothing running against it comes back empty, which is a
/// repository with no CI. That is nothing to wait on rather than something
/// missing — see [`crate::checks`], where it is read as green.
pub(crate) fn checks(gh: &Gh, repo: &Path, number: i64) -> Result<Vec<Check>, Trouble> {
    /// What `--json statusCheckRollup` comes back as.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Rollup {
        #[serde(default)]
        status_check_rollup: Vec<Reported>,
    }

    /// One entry of it, which is one of two different things.
    ///
    /// A `CheckRun` is an Actions job and carries a `status` and a `conclusion`;
    /// a `StatusContext` is the older commit-status API and carries a `state`
    /// alone. Both are read here rather than only the first, because a
    /// repository is free to use either and a Verkstead that saw only Actions
    /// would call a red Buildkite green.
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

    let said = gh.ask(
        repo,
        &[
            "pr",
            "view",
            &number.to_string(),
            "--json",
            "statusCheckRollup",
        ],
    )?;

    let rollup: Rollup = serde_json::from_str(&said)
        .map_err(|error| Trouble::Refused(format!("gh answered something unreadable: {error}")))?;

    Ok(rollup
        .status_check_rollup
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
        .collect())
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
            checks(&gh, dir.path(), 41).unwrap(),
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

        let checks = checks(&gh, dir.path(), 41).unwrap();

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

        let checks = checks(&gh, dir.path(), 41).unwrap();

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
            checks(&gh, dir.path(), 41).unwrap(),
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

        assert!(checks(&gh, dir.path(), 41).unwrap().is_empty());
    }

    /// And the reason the checks are asked for this way at all: a `gh` that
    /// cannot answer has to arrive as [`Trouble`], because *Verkstead could not
    /// ask* is a third thing beside green and red.
    #[test]
    fn a_gh_that_cannot_answer_about_checks_says_so_rather_than_saying_green() {
        let (dir, gh) = stub("", "gh: To use GitHub CLI, run: gh auth login");

        assert_eq!(checks(&gh, dir.path(), 41), Err(Trouble::NotLoggedIn));
    }

    /// A PR nobody has said anything on, which is every PR the moment it opens.
    #[test]
    fn a_pull_request_with_no_comments_reads_back_as_none() {
        let (dir, gh) = stub(r#"{"commits":[],"comments":[]}"#, "");

        let details = details(&gh, dir.path(), 41).unwrap();

        assert!(details.commits.is_empty() && details.comments.is_empty());
    }
}
