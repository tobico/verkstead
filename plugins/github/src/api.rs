//! What Verkstead asks GitHub, asked of GitHub's own two APIs.
//!
//! **GraphQL for everything about a pull request**, which is what `gh pr view`
//! wrapped and what every reading here was written against: one question comes
//! back with the commits, the comments, the reviews, the check rollup, whether
//! it merges and where it has got to, in the words
//! [`mergeable`], [`stands`] and [`checked`] read — so a pull request costs one
//! round trip over somebody else's network rather than five, and the readings
//! are the ones the server has always made.
//!
//! **REST for the rest**: the comments left on the lines of the diff, which
//! GraphQL's pull request has no field for; the comment a share leaves, which
//! is a write; a Share's gist; a repository; and the token's own account,
//! whose scopes are in a header rather than in any resource.
//!
//! **Which host is the remote's.** Every operation about a repository is
//! handed the `origin` remote, and where its API lives is read off that and
//! nowhere else — see [`crate::names::Repository`], which answers github.com's
//! own API host for github.com and the host itself for a GitHub Enterprise
//! Server. The three operations that are about no repository — verifying a
//! token, publishing a Share, making a repository — are github.com's: nothing
//! in what they are handed says which instance they are about.

use serde_json::{Value, json};

use crate::http::{self, Verb};
use crate::names::{self, Repository};
use crate::{
    Account, Check, Checked, Comment, Details, Found, Landed, Landing, Made, Mergeable, Numbered,
    Opening, Published, Scopes, Stands, Suite, Trouble,
};

/// Where github.com's own API lives, which is what the operations about no
/// repository ask.
const DOT_COM: &str = "api.github.com";

/// How many of a list one page asks for, which is GitHub's own maximum.
const PAGE: usize = 100;

/// And how many pages of the comments on a diff are followed before the rest
/// is left: a review that left three thousand comments is one nobody is going
/// to address in a session.
const PAGES: usize = 10;

/// Who the token in this call's environment is, and what GitHub will let it
/// do.
///
/// `GET /user` rather than anything about a repository: what is being asked
/// about is the token, and a question about a repository could fail for the
/// repository's reasons. The scopes are off the answer's headers — see
/// [`scopes`], and [`Scopes`], where what a missing header means is written
/// down.
pub fn verify() -> Result<Account, Trouble> {
    let answer = http::ask(Verb::Get, DOT_COM, "/user", None)?;

    // **A refusal in GitHub's own words, a `401` included**, which is the one
    // place that status is not read as nobody being logged in: what is being
    // asked about *is* the token, so GitHub turning it down is an answer about
    // it rather than a Verkstead with none. The settings page says what GitHub
    // said beside a token it saved anyway, and *there is no token* would be
    // telling the human to do the thing they have just done.
    if !(200..300).contains(&answer.status) {
        return Err(Trouble::Refused(answer.refusal()));
    }

    let login = serde_json::from_str::<Value>(&answer.body)
        .ok()
        .as_ref()
        .map(|user| text(user, "login"))
        .filter(|login| !login.is_empty())
        .ok_or_else(|| unreadable("the account"))?;

    Ok(Account {
        login,
        scopes: scopes(answer.header("x-oauth-scopes")),
    })
}

/// The scopes a header named, or that it named none.
///
/// Read as *nothing said* where the header is not there at all, which is a
/// fine-grained token: it has permissions rather than scopes, and GitHub
/// answers for one by leaving the header off. A token nothing is known about
/// is one to try rather than one to refuse.
fn scopes(said: Option<&str>) -> Scopes {
    match said {
        Some(said) => Scopes::Named(
            said.split(',')
                .map(str::trim)
                .filter(|scope| !scope.is_empty())
                .map(str::to_owned)
                .collect(),
        ),
        None => Scopes::Unsaid,
    }
}

/// The pull request open on `branch`.
///
/// Asked by the head branch rather than searched for: GitHub takes
/// `headRefName` as a filter on the repository's pull requests, so what comes
/// back is the open one on that branch and nothing else. None open on it is
/// [`Trouble::NoPullRequest`], which is what every caller asks again about.
pub fn pull_request(remote: &str, branch: &str) -> Result<Found, Trouble> {
    let repository = repository(remote)?;

    let answered = graph(
        &repository,
        "query($owner:String!,$name:String!,$branch:String!){\
           repository(owner:$owner,name:$name){\
             pullRequests(headRefName:$branch,states:OPEN,first:1){\
               nodes{number title url headRefName baseRefName}}}}",
        json!({ "owner": repository.owner, "name": repository.name, "branch": branch }),
    )?;

    let open = nodes(&answered, &["repository", "pullRequests"])
        .first()
        .cloned()
        .ok_or(Trouble::NoPullRequest)?;

    Ok(Found {
        number: number(&open, "number"),
        title: text(&open, "title"),
        url: text(&open, "url"),
        head: Some(text(&open, "headRefName")),
        base: Some(text(&open, "baseRefName")).filter(|base| !base.is_empty()),
    })
}

/// The pull request `number` names, open or not.
///
/// A number GitHub has nothing under at all is [`Trouble::NoPullRequest`];
/// one it has something merged or closed under comes back with `open` false.
/// The two are one refusal to the human and are told apart here because only
/// one of them is an error.
pub fn numbered(remote: &str, number: i64) -> Result<Numbered, Trouble> {
    let repository = repository(remote)?;
    read_numbered(&repository, number)
}

fn read_numbered(repository: &Repository, number: i64) -> Result<Numbered, Trouble> {
    let answered = graph(
        repository,
        "query($owner:String!,$name:String!,$number:Int!){\
           repository(owner:$owner,name:$name){\
             pullRequest(number:$number){\
               number title url headRefName baseRefName isCrossRepository state}}}",
        json!({ "owner": repository.owner, "name": repository.name, "number": number }),
    )?;

    let viewed = within(&answered, &["repository", "pullRequest"]).ok_or(Trouble::NoPullRequest)?;

    Ok(listed(viewed, None))
}

/// Every pull request the repository has open, which is what a stack is walked
/// out of.
///
/// One question rather than a link at a time: a chain is open pull requests
/// each based on the one below's head branch, and a list is the whole of what
/// it takes to assemble. `isCrossRepository` comes back with each, so a fork
/// is still refused at take-up — a head in another repository is not a branch
/// anything here can push to.
pub fn open_pull_requests(remote: &str) -> Result<Vec<Numbered>, Trouble> {
    let repository = repository(remote)?;

    let answered = graph(
        &repository,
        "query($owner:String!,$name:String!,$page:Int!){\
           repository(owner:$owner,name:$name){\
             pullRequests(states:OPEN,first:$page,\
                          orderBy:{field:CREATED_AT,direction:DESC}){\
               nodes{number title url headRefName baseRefName isCrossRepository}}}}",
        json!({ "owner": repository.owner, "name": repository.name, "page": PAGE }),
    )?;

    Ok(nodes(&answered, &["repository", "pullRequests"])
        .iter()
        // Everything `states: OPEN` answered with is open, which is what was
        // asked for.
        .map(|listed_| listed(listed_, Some(true)))
        .collect())
}

/// Open `opening` as a draft, and answer it as GitHub now has it.
///
/// The create is REST — a write of a resource, which is what REST is for —
/// and the answer is read back by its number through the same question
/// [`numbered`] asks, rather than read a second way out of what the create
/// printed.
pub fn open_pull_request(remote: &str, opening: &Opening) -> Result<Numbered, Trouble> {
    let repository = repository(remote)?;

    let answer = http::ask(
        Verb::Post,
        &repository.rest().0,
        &path(&repository, "/pulls"),
        Some(
            &json!({
                "title": opening.title,
                "head": opening.head,
                "base": opening.base,
                "body": opening.body,
                "draft": true,
            })
            .to_string(),
        ),
    )?;
    answer.refused()?;

    let opened: Value =
        serde_json::from_str(&answer.body).map_err(|_| unreadable("the pull request"))?;
    let number = number(&opened, "number");
    if number == 0 {
        return Err(Trouble::Refused(
            "GitHub opened a pull request and didn't say which number it is.".to_owned(),
        ));
    }

    read_numbered(&repository, number)
}

/// What the details pane draws of pull request `number`, and the facts that
/// ride the same question: the commits it carries, what has been said in its
/// conversation, what GitHub is running against it, whether it merges and
/// where it has got to.
///
/// One question for all of it, which is the whole reason this is GraphQL:
/// opening the pane is the one thing left that asks GitHub about a pull
/// request nothing is watching, so it freshens every fact a card draws rather
/// than only the one the pane came for.
pub fn details(remote: &str, number: i64) -> Result<Details, Trouble> {
    let repository = repository(remote)?;

    let answered = graph(
        &repository,
        &format!(
            "query($owner:String!,$name:String!,$number:Int!){{\
               repository(owner:$owner,name:$name){{\
                 pullRequest(number:$number){{\
                   mergeable state\
                   commits(first:{PAGE}){{nodes{{commit{{oid messageHeadline}}}}}}\
                   comments(first:{PAGE}){{nodes{{author{{login}} body createdAt}}}}\
                   {ROLLUP}}}}}}}"
        ),
        json!({ "owner": repository.owner, "name": repository.name, "number": number }),
    )?;

    let viewed = within(&answered, &["repository", "pullRequest"]).ok_or(Trouble::NoPullRequest)?;

    Ok(Details {
        commits: nodes(viewed, &["commits"])
            .iter()
            .filter_map(|node| node.get("commit"))
            .map(|commit| Landed {
                sha: text(commit, "oid"),
                subject: text(commit, "messageHeadline"),
            })
            .collect(),
        comments: nodes(viewed, &["comments"])
            .iter()
            .map(|said| Comment {
                which: identity(&text(said, "id"), &text(said, "url"), said),
                author: author(said),
                at: text(said, "createdAt"),
                about: String::new(),
                markdown: text(said, "body"),
            })
            .collect(),
        checks: rollup(viewed),
        landing: Landing {
            mergeable: mergeable(&text(viewed, "mergeable")),
            stands: stands(&text(viewed, "state")),
        },
    })
}

/// The checks against pull request `number`'s head, and whether it merges.
///
/// **The head comes back beside them**, because a rollup is a fact about one
/// commit rather than about the branch: GitHub's record of a pull request is
/// behind the branch for a while after a push, and a green rollup of the
/// commit before it is no evidence about what was pushed.
///
/// **And whether it merges rides the same call**, a pull request whose base
/// has moved under it being as much a reason to wait as a red check is.
pub fn checks(remote: &str, number: i64) -> Result<Suite, Trouble> {
    let repository = repository(remote)?;

    let answered = graph(
        &repository,
        &format!(
            "query($owner:String!,$name:String!,$number:Int!){{\
               repository(owner:$owner,name:$name){{\
                 pullRequest(number:$number){{\
                   headRefOid mergeable {ROLLUP}}}}}}}"
        ),
        json!({ "owner": repository.owner, "name": repository.name, "number": number }),
    )?;

    let viewed = within(&answered, &["repository", "pullRequest"]).ok_or(Trouble::NoPullRequest)?;

    Ok(Suite {
        head: text(viewed, "headRefOid"),
        checks: rollup(viewed),
        mergeable: mergeable(&text(viewed, "mergeable")),
    })
}

/// Whether pull request `number` merges into its base, and where it has got
/// to: the two facts one look at a pull request nobody is wrapping up any more
/// comes back with.
pub fn landing(remote: &str, number: i64) -> Result<Landing, Trouble> {
    let repository = repository(remote)?;

    let answered = graph(
        &repository,
        "query($owner:String!,$name:String!,$number:Int!){\
           repository(owner:$owner,name:$name){\
             pullRequest(number:$number){mergeable state}}}",
        json!({ "owner": repository.owner, "name": repository.name, "number": number }),
    )?;

    let viewed = within(&answered, &["repository", "pullRequest"]).ok_or(Trouble::NoPullRequest)?;

    Ok(Landing {
        mergeable: mergeable(&text(viewed, "mergeable")),
        stands: stands(&text(viewed, "state")),
    })
}

/// Everything said on pull request `number`, in the order it was said.
///
/// **Three places a human writes, and all three count**: the conversation, the
/// words at the top of a review, and the comments left on the lines of the
/// diff — the last of which is where most code feedback lands, so a reading
/// that took the conversation alone would miss the reviews it most needs to
/// act on. Two questions rather than three: GraphQL answers for the
/// conversation and the reviews together, and the diff's are REST's.
///
/// A review with nothing written at the top of it is left out: an approval
/// with no words is somebody saying they are happy, and a session sent to
/// address it would be work invented out of agreement.
pub fn comments(remote: &str, number: i64) -> Result<Vec<Comment>, Trouble> {
    let repository = repository(remote)?;

    let answered = graph(
        &repository,
        &format!(
            "query($owner:String!,$name:String!,$number:Int!){{\
               repository(owner:$owner,name:$name){{\
                 pullRequest(number:$number){{\
                   comments(first:{PAGE}){{nodes{{id url author{{login}} body createdAt}}}}\
                   reviews(first:{PAGE}){{nodes{{id author{{login}} body submittedAt}}}}}}}}}}"
        ),
        json!({ "owner": repository.owner, "name": repository.name, "number": number }),
    )?;

    let viewed = within(&answered, &["repository", "pullRequest"]).ok_or(Trouble::NoPullRequest)?;

    let mut said: Vec<Comment> = nodes(viewed, &["comments"])
        .iter()
        .map(|one| Comment {
            which: identity(&text(one, "id"), &text(one, "url"), one),
            author: author(one),
            at: text(one, "createdAt"),
            about: String::new(),
            markdown: text(one, "body"),
        })
        .collect();

    said.extend(
        nodes(viewed, &["reviews"])
            .iter()
            .filter(|review| !text(review, "body").trim().is_empty())
            .map(|review| Comment {
                which: identity(&text(review, "id"), "", review),
                author: author(review),
                at: text(review, "submittedAt"),
                about: String::new(),
                markdown: text(review, "body"),
            }),
    );

    said.extend(on_the_diff(&repository, number)?);

    // In the order they were said in, across all three places. The timestamps
    // are RFC 3339 in UTC, which sorts as text.
    said.sort_by(|one, next| one.at.cmp(&next.at));

    Ok(said)
}

/// What was said on the lines of the diff, which GraphQL's pull request has no
/// field for: the review comments, REST's own, a page at a time.
fn on_the_diff(repository: &Repository, number: i64) -> Result<Vec<Comment>, Trouble> {
    let mut said = Vec::new();

    for page in 1..=PAGES {
        let answer = http::ask(
            Verb::Get,
            &repository.rest().0,
            &path(
                repository,
                &format!("/pulls/{number}/comments?per_page={PAGE}&page={page}"),
            ),
            None,
        )?;
        answer.refused()?;

        let Ok(Value::Array(listed_)) = serde_json::from_str::<Value>(&answer.body) else {
            return Err(unreadable("what was said on the diff"));
        };

        let read = listed_.len();
        said.extend(listed_.iter().map(|one| Comment {
            which: identity(&text(one, "node_id"), &text(one, "html_url"), one),
            author: text(one.get("user").unwrap_or(&Value::Null), "login"),
            at: text(one, "created_at"),
            about: where_said(&text(one, "path"), one.get("line").and_then(Value::as_i64)),
            markdown: text(one, "body"),
        }));

        if read < PAGE {
            break;
        }
    }

    Ok(said)
}

/// Say `body` on pull request `number`, and answer where it was said.
///
/// The issues endpoint, a pull request's conversation being an issue's to
/// GitHub, and REST rather than GraphQL because what is wanted back is the
/// comment as a resource: the human is owed a link to what was left in their
/// name.
pub fn comment(remote: &str, number: i64, body: &str) -> Result<String, Trouble> {
    let repository = repository(remote)?;

    let answer = http::ask(
        Verb::Post,
        &repository.rest().0,
        &path(&repository, &format!("/issues/{number}/comments")),
        Some(&json!({ "body": body }).to_string()),
    )?;
    answer.refused()?;

    let said: Value = serde_json::from_str(&answer.body).map_err(|_| unreadable("the comment"))?;
    Ok(text(&said, "html_url"))
}

/// Publish a Share: one file, `name`, holding `content`, unlisted.
///
/// **`content` is a placeholder rather than the file.** What the API is asked
/// for is a gist with the right name on it, and the bytes arrive over git
/// afterwards — a share is several megabytes, mermaid and every diff riding
/// along, and the cap on what a gist may be created with is undocumented and
/// has been reported at a megabyte for a decade.
///
/// Secret rather than public, which is `public: false`: a share is read by
/// whoever was sent the link and by nobody else.
pub fn publish(description: &str, name: &str, content: &str) -> Result<Published, Trouble> {
    let answer = http::ask(
        Verb::Post,
        DOT_COM,
        "/gists",
        Some(
            &json!({
                "description": description,
                "public": false,
                "files": { name: { "content": content } },
            })
            .to_string(),
        ),
    )?;
    answer.refused()?;

    let made: Value = serde_json::from_str(&answer.body).map_err(|_| unreadable("the gist"))?;

    Ok(Published {
        id: text(&made, "id"),
        url: text(&made, "html_url"),
        push: text(&made, "git_push_url"),
    })
}

/// And take a published one down again.
///
/// What a publish that fell over after the gist was made does with it: a gist
/// holding a placeholder and no share is worse than no gist at all.
pub fn unpublish(id: &str) -> Result<(), Trouble> {
    http::ask(Verb::Delete, DOT_COM, &format!("/gists/{id}"), None)?.refused()
}

/// Make a private repository called `name`, and answer where it is and the
/// remote to push it to.
///
/// **Private**, always: a repository made from here is somebody's work before
/// it is anybody else's business, and public is a decision to take
/// deliberately rather than by leaving a box alone. The pushing is the host's
/// — it holds the repository this is a remote for.
pub fn create_repository(name: &str) -> Result<Made, Trouble> {
    let answer = http::ask(
        Verb::Post,
        DOT_COM,
        "/user/repos",
        Some(&json!({ "name": name, "private": true }).to_string()),
    )?;
    answer.refused()?;

    let made: Value =
        serde_json::from_str(&answer.body).map_err(|_| unreadable("the repository"))?;

    Ok(Made {
        url: text(&made, "html_url"),
        push: text(&made, "clone_url"),
    })
}

/// The check rollup of a pull request's head, as the checks Verkstead reads.
///
/// GitHub answers the rollup off the last commit, in two kinds: a `CheckRun`
/// is an Actions job and carries a `status` and a `conclusion`, and a
/// `StatusContext` is the older commit-status API and carries a `state` alone.
/// Both are read, because a repository is free to use either and a reading
/// that saw only Actions would call a red Buildkite green.
fn rollup(viewed: &Value) -> Vec<Check> {
    let Some(commit) = nodes(viewed, &["rollup"])
        .iter()
        .filter_map(|node| node.get("commit"))
        .next_back()
    else {
        return Vec::new();
    };

    nodes(commit, &["statusCheckRollup", "contexts"])
        .iter()
        .map(|reported| Check {
            name: match (
                text(reported, "name").is_empty(),
                text(reported, "context").is_empty(),
            ) {
                // A check with neither is one GitHub declined to name, and it
                // still has to be counted against something.
                (true, true) => "a check GitHub did not name".to_owned(),
                (true, false) => text(reported, "context"),
                _ => text(reported, "name"),
            },
            how: checked(
                &text(reported, "status"),
                &text(reported, "conclusion"),
                &text(reported, "state"),
            ),
            link: match text(reported, "detailsUrl").is_empty() {
                true => text(reported, "targetUrl"),
                false => text(reported, "detailsUrl"),
            },
        })
        .collect()
}

/// What a pull request's checks are asked for, which two questions both carry.
/// **Asked under a name of its own**, because the one question the details
/// pane raises asks for the pull request's commits as well, and the same field
/// twice with different arguments is a question GraphQL refuses. Under an
/// alias they are two answers, each with the arguments it wants — see
/// [`rollup`], which reads this one.
const ROLLUP: &str = "rollup:commits(last:1){nodes{commit{statusCheckRollup{\
                      contexts(first:100){nodes{\
                      __typename \
                      ...on CheckRun{name status conclusion detailsUrl} \
                      ...on StatusContext{context state targetUrl}}}}}}}";

/// How one check is getting on, out of the three words GitHub says it in.
///
/// `status` is the Actions job's — anything but `COMPLETED` means it is still
/// going, whatever the conclusion column happens to hold. Then the outcome,
/// which is the job's `conclusion` or the older API's `state` depending on
/// which kind of thing this is; only one of the two is ever set.
///
/// Green is listed rather than red, which is the way round that matters: a
/// word this does not know is read as a failure, so a GitHub that invents a
/// new way for a check to go wrong stops a wrap-up rather than settling it.
fn checked(status: &str, conclusion: &str, state: &str) -> Checked {
    if !status.is_empty() && !status.eq_ignore_ascii_case("COMPLETED") {
        return Checked::Running;
    }

    let outcome = match conclusion.is_empty() {
        true => state,
        false => conclusion,
    };

    // Nothing said either way: a status context created and not reported yet,
    // or a check run with no outcome on it at all.
    if outcome.is_empty() || outcome.eq_ignore_ascii_case("PENDING") {
        return Checked::Running;
    }

    match outcome.to_ascii_uppercase().as_str() {
        "SUCCESS" | "NEUTRAL" | "SKIPPED" => Checked::Passed,
        _ => Checked::Failed,
    }
}

/// Whether it merges, out of GitHub's word for it.
///
/// A word this does not know is read as *not known* rather than as a
/// conflict, which is the way round that matters: not knowing changes nothing,
/// and a conflict holds a wrap-up up.
fn mergeable(said: &str) -> Mergeable {
    match said.to_ascii_uppercase().as_str() {
        "MERGEABLE" => Mergeable::Cleanly,
        "CONFLICTING" => Mergeable::Conflicting,
        _ => Mergeable::Unknown,
    }
}

/// And where it has got to, which is [`mergeable`]'s rule turned around:
/// reading a word nobody has seen before as an ending would stop the asking
/// for good, so nothing but GitHub's own two words for one is one.
fn stands(said: &str) -> Stands {
    match said.to_ascii_uppercase().as_str() {
        "OPEN" => Stands::Open,
        "MERGED" => Stands::Merged,
        "CLOSED" => Stands::Closed,
        _ => Stands::Unknown,
    }
}

/// One pull request of a list or a view, read: `open` where the question
/// already said so, and off the `state` otherwise.
fn listed(viewed: &Value, open: Option<bool>) -> Numbered {
    Numbered {
        number: number(viewed, "number"),
        title: text(viewed, "title"),
        url: text(viewed, "url"),
        head: text(viewed, "headRefName"),
        base: text(viewed, "baseRefName"),
        fork: viewed
            .get("isCrossRepository")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        open: open.unwrap_or_else(|| text(viewed, "state").eq_ignore_ascii_case("open")),
    }
}

/// What a comment is called, for the table that remembers which ones have
/// been dispatched for.
///
/// GitHub's node id first, because that is what it is. The URL after it, which
/// carries the same id in a different spelling. And, where there is neither,
/// who said it and when — weaker than the other two, but a comment with no
/// identity at all would be one dispatched for again on every poll for ever.
fn identity(id: &str, url: &str, said: &Value) -> String {
    for named in [id, url] {
        if !named.trim().is_empty() {
            return named.trim().to_owned();
        }
    }

    let at = match text(said, "createdAt").is_empty() {
        true => text(said, "submittedAt"),
        false => text(said, "createdAt"),
    };

    format!("{} at {at}", author(said))
}

/// Who said it. A comment left by an account that has since gone comes back
/// with no author at all, which is a comment to act on rather than one to
/// drop.
fn author(said: &Value) -> String {
    text(said.get("author").unwrap_or(&Value::Null), "login")
}

/// Where on the branch a comment was left, as the session that reads it is
/// told: the file alone where GitHub gave no line, which is a comment on a
/// line the branch has moved past and still worth pointing at the file.
fn where_said(path: &str, line: Option<i64>) -> String {
    match (path.trim().is_empty(), line) {
        (true, _) => String::new(),
        (false, Some(line)) => format!("`{path}` line {line}"),
        (false, None) => format!("`{path}`"),
    }
}

/// The repository `remote` names, or [`Trouble::NoRemote`] where it names
/// none: a path on this machine, a `file://` URL, or a URL with no owner and
/// name under its host. There is no repository to ask about, which is what
/// that kind says.
fn repository(remote: &str) -> Result<Repository, Trouble> {
    names::repository(remote).ok_or(Trouble::NoRemote)
}

/// A REST path under the repository, which is where every one of them hangs:
/// `/api/v3` first on a GitHub Enterprise Server, and nothing on github.com.
fn path(repository: &Repository, under: &str) -> String {
    let (_, prefix) = repository.rest();

    format!(
        "{prefix}/repos/{}/{}{under}",
        repository.owner, repository.name
    )
}

/// Ask GraphQL `query` with `variables`, and answer its `data`.
///
/// **GraphQL answers a question it refused with a `200` and an `errors`
/// list**, so the status says nothing: what is read is the list. A pull
/// request GitHub has nothing under is the one error worth telling apart —
/// every caller asks again about that one — and everything else is what
/// GitHub said, quoted.
fn graph(repository: &Repository, query: &str, variables: Value) -> Result<Value, Trouble> {
    let (host, path) = repository.graphql();

    let answer = http::ask(
        Verb::Post,
        &host,
        &path,
        Some(&json!({ "query": query, "variables": variables }).to_string()),
    )?;
    answer.refused()?;

    let answered: Value =
        serde_json::from_str(&answer.body).map_err(|_| unreadable("GitHub's answer"))?;

    if let Some(Value::Array(errors)) = answered.get("errors")
        && !errors.is_empty()
    {
        let said: Vec<String> = errors
            .iter()
            .map(|error| text(error, "message"))
            .filter(|said| !said.trim().is_empty())
            .collect();

        if said.iter().any(|said| {
            said.to_lowercase()
                .contains("could not resolve to a pullrequest")
        }) {
            return Err(Trouble::NoPullRequest);
        }

        return Err(match said.is_empty() {
            true => Trouble::Refused("GitHub refused the question without saying why.".to_owned()),
            false => Trouble::Refused(said.join(" ")),
        });
    }

    answered
        .get("data")
        .filter(|data| !data.is_null())
        .cloned()
        .ok_or_else(|| unreadable("GitHub's answer"))
}

/// What `path` names inside `value`, or `None` where any step of it is not
/// there — which is GraphQL's own way of saying the thing does not exist.
fn within<'v>(value: &'v Value, path: &[&str]) -> Option<&'v Value> {
    let mut at = value;
    for step in path {
        at = at.get(step)?;
    }

    (!at.is_null()).then_some(at)
}

/// And the `nodes` of the connection at `path`, which is empty where there is
/// no connection there at all.
fn nodes<'v>(value: &'v Value, path: &[&str]) -> &'v [Value] {
    const NONE: &[Value] = &[];

    within(value, path)
        .and_then(|at| at.get("nodes"))
        .and_then(Value::as_array)
        .map_or(NONE, Vec::as_slice)
}

/// The string at `key`, or empty where there is none — every field GitHub may
/// leave out, read the way the server read `gh`'s own JSON.
fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// And the number at `key`, or nothing.
fn number(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or_default()
}

/// What GitHub answered could not be read, in Verkstead's own words: a
/// refusal, since a Forge that answered nonsense is a Forge that would not
/// answer.
fn unreadable(what: &str) -> Trouble {
    Trouble::Refused(format!(
        "GitHub answered something {what} couldn't be read from."
    ))
}
