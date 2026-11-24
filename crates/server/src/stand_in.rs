//! GitHub's stand-in: an HTTP server a test stands up and the Forge that
//! reaches it, which is what every suite here puts where GitHub goes.
//!
//! **The Forge is the real one.** What answers is the built-in GitHub plugin's
//! own component, loaded out of this binary, speaking the REST and GraphQL it
//! speaks to github.com, under the outbound grant its manifest asks for — only
//! the connection goes to a listener the test stood up, through
//! [`crate::plugins::component::Redirect`]. So a test says what *GitHub*
//! answered rather than what a program printed, and what is proved is the
//! component the binary ships.
//!
//! A stand-in rather than GitHub, for the reason the `gh` stand-in it replaced
//! was a shell script: asking the real one would be a test that needed a
//! network, an account and somebody's pull request.
//!
//! **Two halves.** [`GitHub`] is the listener: it reads each request as the
//! one question it is — see [`Asking`] — and answers what the test says. And
//! [`Forge`] is what a router is stood up with, which is the component pointed
//! at that listener. A test writes
//!
//! ```ignore
//! let github = GitHub::answering(|asked| match asked.asking {
//!     Asking::Checks => Said::About(green()),
//!     _ => Said::About(the_pull_request()),
//! })
//! .await;
//!
//! let app = router_asking_github(pool, data_dir, Forge::standing_in(&github));
//! ```
//!
//! and reads back what it was asked with [`GitHub::asked`] — which is also
//! where the token each call carried is, since that is what says whether the
//! host set the Author's secret for it.
//!
//! **Compiled into the binary rather than behind `cfg(test)`**, because the
//! suites that need it are integration tests in another crate — the same
//! reason [`crate::plugins::component::Redirect`] is public. Nothing on a
//! running Verkstead builds one.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::plugins::component::Redirect;
use crate::settings::Settings;

/// Where the component asks for GitHub, which is the host a stand-in stands in
/// for.
const DOT_COM: &str = "api.github.com";

/// The account a Repo whose remote names no host is asked about under — see
/// [`crate::forges::plugged::StandingIn`], which says why there is one.
const UNDER: &str = "https://github.com/tobico";

/// The token a stand-in's Forge asks as, where nothing says whose it should
/// be. What a test reads off [`Asked::token`] to know the host set one.
pub const TOKEN: &str = "ghp_thestandin";

/// One question Verkstead asks a Forge, read off the request that carries it.
///
/// One for each operation the `forge` world has, which is what makes a test
/// readable: a stand-in says what GitHub answers about the *checks* rather
/// than what it answers to a GraphQL document containing a particular
/// substring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Asking {
    /// The pull request open on a branch.
    PullRequest,

    /// The pull request a number names, open or not.
    Numbered,

    /// Every pull request the repository has open, which is how a stack is
    /// walked.
    Open,

    /// Opening one as a draft. What a stand-in answers is its number, which
    /// the component then reads back as [`Asking::Numbered`].
    Opening,

    /// Everything the details pane draws: the commits, the conversation, the
    /// checks, whether it merges and where it has got to.
    Details,

    /// The checks against its head, its head, and whether it merges.
    Checks,

    /// Whether it merges and where it has got to, and nothing else — which is
    /// the sweep's own question about a pull request nothing is watching.
    Landing,

    /// What has been said in its conversation and at the top of its reviews.
    Comments,

    /// And what has been said on the lines of its diff.
    OnTheDiff,

    /// Saying something on it.
    Saying,

    /// Publishing a Share as a gist, and taking one down again.
    Publishing,
    Unpublishing,

    /// Making a repository.
    Making,

    /// Who the token in this call's environment is, and what it may do.
    Verifying,

    /// Something no operation of a Forge asks, which is a stand-in being asked
    /// a question nobody wrote.
    Unasked,
}

/// One request a stand-in was asked, read.
#[derive(Debug, Clone)]
pub struct Asked {
    /// Which question it is.
    pub asking: Asking,

    /// The repository it is about — the `name` of `owner/name` — or empty
    /// where the question is about no repository.
    pub repository: String,

    /// The pull request's number, where the question names one.
    pub number: Option<i64>,

    /// The branch, where the question names one.
    pub branch: Option<String>,

    /// The path as it arrived, which is what a test about a URL reads.
    pub path: String,

    /// And the body, for the questions whose subject is what was written —
    /// the comment left, the gist published.
    pub body: String,

    /// What it carried as its token, which is what says whether the host set
    /// the Author's secret in the component's environment for this one call.
    pub token: Option<String>,
}

/// A pull request as a stand-in holds one: the facts every question about it
/// is answered out of.
///
/// **The field names are GitHub's own**, which is also what `gh pr view
/// --json` printed: a `conclusion` is a conclusion and a `mergeable` is
/// `MERGEABLE`. What differs between the questions is the shape they are
/// wrapped in, and that is this module's to know rather than a test's — see
/// [`Said::About`].
#[derive(Debug, Clone)]
pub struct PullRequest {
    pub number: i64,
    pub title: String,
    pub url: String,

    /// Its branch, and the branch it is against.
    pub head: String,
    pub base: String,

    /// Whether its head is in another repository, which is what a fork is and
    /// what a take-up refuses.
    pub fork: bool,

    /// Where it has got to: `OPEN`, `MERGED` or `CLOSED`.
    pub state: String,

    /// Whether it merges: `MERGEABLE`, `CONFLICTING` or `UNKNOWN`.
    pub mergeable: String,

    /// The commit GitHub's record of it is at, which a rollup is a fact about.
    /// Empty where a test is not about the head having moved.
    pub head_oid: String,

    /// The checks against that head: `{"__typename":"CheckRun","name":…,
    /// "status":…,"conclusion":…,"detailsUrl":…}` entries, or the older
    /// `{"__typename":"StatusContext","context":…,"state":…}` ones.
    pub checks: Vec<Value>,

    /// Its commits: `{"oid":…,"messageHeadline":…}` entries.
    pub commits: Vec<Value>,

    /// What has been said in its conversation:
    /// `{"author":{"login":…},"body":…,"createdAt":…}` entries.
    pub comments: Vec<Value>,

    /// And at the top of its reviews, with `submittedAt` where a comment has
    /// `createdAt`.
    pub reviews: Vec<Value>,

    /// And on the lines of its diff, which is REST's own shape:
    /// `{"user":{"login":…},"body":…,"created_at":…,"path":…,"line":…}`.
    pub on_the_diff: Vec<Value>,
}

impl Default for PullRequest {
    /// A pull request that is open, merges, and has nothing on it.
    fn default() -> PullRequest {
        PullRequest {
            number: 0,
            title: String::new(),
            url: String::new(),
            head: String::new(),
            base: String::new(),
            fork: false,
            state: "OPEN".to_owned(),
            mergeable: "MERGEABLE".to_owned(),
            head_oid: String::new(),
            checks: Vec::new(),
            commits: Vec::new(),
            comments: Vec::new(),
            reviews: Vec::new(),
            on_the_diff: Vec::new(),
        }
    }
}

impl PullRequest {
    /// Its node as a list and a view answer it.
    fn node(&self) -> Value {
        json!({
            "number": self.number,
            "title": self.title,
            "url": self.url,
            "headRefName": self.head,
            "baseRefName": self.base,
            "isCrossRepository": self.fork,
            "state": self.state,
        })
    }

    /// Its check rollup, under the alias the two questions that ask for it
    /// use — see the `ROLLUP` the component asks with.
    fn rollup(&self) -> Value {
        json!({ "nodes": [{ "commit": {
            "statusCheckRollup": { "contexts": { "nodes": self.checks } },
        }}]})
    }
}

/// What a stand-in answers a question with.
///
/// **Not boxed**, though one variant carries a whole pull request and another
/// a list of them: these are written by hand in a test, and
/// `Said::About(Box::new(…))` at two hundred sites would cost the reader more
/// than the move costs the machine.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Said {
    /// This pull request, in whatever shape the question asks for it.
    About(PullRequest),

    /// Every pull request open, which is what a stack walk asks for.
    Open(Vec<PullRequest>),

    /// Nothing under what was asked about: no pull request on that branch or
    /// under that number, nothing said, nothing open.
    Nothing,

    /// Whatever JSON, for the questions a pull request is not the answer to —
    /// a gist, a repository.
    This(String),

    /// Who the token is, which `/user` answers — with what GitHub says it may
    /// do, which is in a header and nowhere else. `None` is the header left
    /// off, which is what GitHub does for a fine-grained token.
    Account {
        login: String,
        scopes: Option<String>,
    },

    /// A refusal: the status and what GitHub said. `401` is the token, which
    /// every caller reads as nobody being logged in.
    Refused(u16, String),

    /// And a refusal GraphQL makes, which comes back with a `200` and an
    /// `errors` list rather than a status — the one a pull request GitHub has
    /// nothing under is made with.
    Erring(String),
}

impl Said {
    /// What GitHub would not have: a token it turns down.
    pub fn not_logged_in() -> Said {
        Said::Refused(401, "Bad credentials".to_owned())
    }

    /// The request, answered: the status, whatever headers the answer carries,
    /// and the body.
    fn answered(&self, asking: Asking) -> (u16, Vec<(String, String)>, String) {
        let (status, said) = match self {
            Said::Refused(status, said) => (*status, json!({ "message": said }).to_string()),
            Said::Erring(said) => (200, json!({ "errors": [{ "message": said }] }).to_string()),
            Said::This(said) => (200, said.clone()),
            Said::Nothing => nothing(asking),
            Said::About(pull_request) => about(asking, pull_request),
            Said::Open(open) => (
                200,
                graph(json!({ "repository": { "pullRequests": {
                    "nodes": open.iter().map(PullRequest::node).collect::<Vec<Value>>(),
                }}})),
            ),

            // The scopes are a header, which is the whole of why this is an
            // answer of its own: what a token may do is not a field of any
            // resource.
            Said::Account { login, scopes } => {
                let headers = scopes
                    .iter()
                    .map(|scopes| ("x-oauth-scopes".to_owned(), scopes.clone()))
                    .collect();

                return (200, headers, json!({ "login": login }).to_string());
            }
        };

        (status, Vec::new(), said)
    }
}

/// `data`, as GraphQL wraps an answer.
fn graph(data: Value) -> String {
    json!({ "data": data }).to_string()
}

/// A pull request under the one GraphQL question that asks about it.
fn viewed(fields: Value) -> String {
    graph(json!({ "repository": { "pullRequest": fields } }))
}

/// `pull_request`, answered in the shape `asking` asks for it in.
fn about(asking: Asking, pull_request: &PullRequest) -> (u16, String) {
    let said = match asking {
        Asking::PullRequest => graph(json!({ "repository": { "pullRequests": {
            "nodes": [pull_request.node()],
        }}})),

        Asking::Numbered => viewed(pull_request.node()),

        Asking::Open => graph(json!({ "repository": { "pullRequests": {
            "nodes": [pull_request.node()],
        }}})),

        // The create answers the resource REST made, and the component reads
        // it back by its number — which is the next question it asks.
        Asking::Opening => json!({ "number": pull_request.number }).to_string(),

        Asking::Details => viewed(json!({
            "mergeable": pull_request.mergeable,
            "state": pull_request.state,
            "commits": { "nodes": pull_request.commits.iter()
                .map(|commit| json!({ "commit": commit }))
                .collect::<Vec<Value>>() },
            "comments": { "nodes": pull_request.comments },
            "rollup": pull_request.rollup(),
        })),

        Asking::Checks => viewed(json!({
            "headRefOid": pull_request.head_oid,
            "mergeable": pull_request.mergeable,
            "rollup": pull_request.rollup(),
        })),

        Asking::Landing => viewed(json!({
            "mergeable": pull_request.mergeable,
            "state": pull_request.state,
        })),

        Asking::Comments => viewed(json!({
            "comments": { "nodes": pull_request.comments },
            "reviews": { "nodes": pull_request.reviews },
        })),

        Asking::OnTheDiff => Value::Array(pull_request.on_the_diff.clone()).to_string(),

        // Where it was said, which is the one thing a say answers.
        Asking::Saying => json!({
            "html_url": format!("{}#issuecomment-1", pull_request.url),
        })
        .to_string(),

        Asking::Publishing
        | Asking::Unpublishing
        | Asking::Making
        | Asking::Verifying
        | Asking::Unasked => return nothing(asking),
    };

    (200, said)
}

/// Nothing under what `asking` asked about, in the words that question's
/// answer says it in.
fn nothing(asking: Asking) -> (u16, String) {
    match asking {
        Asking::PullRequest | Asking::Open => (
            200,
            graph(json!({ "repository": { "pullRequests": { "nodes": [] } } })),
        ),

        // GraphQL answers a thing it has nothing under with a null, which is
        // what every caller reads as no pull request.
        Asking::Numbered | Asking::Details | Asking::Checks | Asking::Landing => {
            (200, viewed(Value::Null))
        }

        Asking::Comments => (
            200,
            viewed(json!({ "comments": { "nodes": [] }, "reviews": { "nodes": [] } })),
        ),

        Asking::OnTheDiff => (200, "[]".to_owned()),

        // And REST, with the answer it gives for a path it has nothing under.
        Asking::Opening
        | Asking::Saying
        | Asking::Publishing
        | Asking::Unpublishing
        | Asking::Making
        | Asking::Verifying
        | Asking::Unasked => (404, json!({ "message": "Not Found" }).to_string()),
    }
}

/// What a stand-in answers with, which is a function of the question: a test
/// whose answer changes part way through — once a file a session writes is
/// there, say — is one closure rather than two stand-ins.
type Answering = Arc<dyn Fn(&Asked) -> Said + Send + Sync>;

/// GitHub's stand-in, listening on a port of its own.
pub struct GitHub {
    address: String,
    answering: Answering,
    asked: Arc<Mutex<Vec<Asked>>>,
}

impl GitHub {
    /// A stand-in answering every question out of `answering`, listening on a
    /// port the machine chose.
    ///
    /// It is served on the runtime this is called from, so a test that holds
    /// its thread without awaiting is one the stand-in cannot answer while it
    /// holds it — which is why a test that drives a Forge from its own thread
    /// wants a multi-thread runtime.
    pub async fn answering(
        answering: impl Fn(&Asked) -> Said + Send + Sync + 'static,
    ) -> Arc<GitHub> {
        let asked: Arc<Mutex<Vec<Asked>>> = Arc::new(Mutex::new(Vec::new()));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a port for GitHub's stand-in");
        let address = listener
            .local_addr()
            .expect("the stand-in's own address")
            .to_string();

        let standing = Arc::new(GitHub {
            address,
            answering: Arc::new(answering),
            asked,
        });

        let app = Router::new().fallback(answer).with_state(standing.clone());

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        standing
    }

    /// And one that answers the same thing whatever it is asked.
    pub async fn saying(said: Said) -> Arc<GitHub> {
        GitHub::answering(move |_| said.clone()).await
    }

    /// Where it is listening, which is where the Forge's component is sent.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// What it was asked, in the order it was asked.
    pub fn asked(&self) -> Vec<Asked> {
        self.asked
            .lock()
            .expect("nothing panics holding it")
            .clone()
    }

    /// And the questions of one kind, which is what *asked once* and *never
    /// asked again* are read off.
    pub fn asked_about(&self, asking: Asking) -> Vec<Asked> {
        self.asked()
            .into_iter()
            .filter(|asked| asked.asking == asking)
            .collect()
    }
}

/// Whatever the stand-in says about the question this request is.
async fn answer(State(standing): State<Arc<GitHub>>, request: Request) -> impl IntoResponse {
    let (parts, body) = request.into_parts();
    let path = parts.uri.to_string();
    let token = parts
        .headers
        .get("authorization")
        .and_then(|said| said.to_str().ok())
        .map(|said| said.trim_start_matches("Bearer ").to_owned());
    let body = axum::body::to_bytes(body, 1 << 22)
        .await
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();

    let asked = read(&parts.method, &path, &body, token);

    standing
        .asked
        .lock()
        .expect("nothing panics holding it")
        .push(asked.clone());

    let (status, headers, said) = (standing.answering)(&asked).answered(asked.asking);

    let mut answered = axum::http::HeaderMap::new();
    answered.insert("content-type", "application/json".parse().unwrap());
    for (name, value) in headers {
        if let (Ok(name), Ok(value)) = (
            axum::http::HeaderName::try_from(name),
            axum::http::HeaderValue::try_from(value),
        ) {
            answered.insert(name, value);
        }
    }

    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        answered,
        Bytes::from(said),
    )
}

/// The request read as the one question it is.
fn read(method: &Method, path: &str, body: &str, token: Option<String>) -> Asked {
    let variables: Value = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|body| body.get("variables").cloned())
        .unwrap_or(Value::Null);

    let said = |key: &str| {
        variables
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };

    // Which repository, off the GraphQL variables where the question carries
    // them and off the REST path where it does not: `/repos/owner/name/…`.
    let repository = said("name").unwrap_or_else(|| {
        path.split('/')
            .skip_while(|step| *step != "repos")
            .nth(2)
            .unwrap_or_default()
            .to_owned()
    });

    let number = variables
        .get("number")
        .and_then(Value::as_i64)
        .or_else(|| numbered(path));

    Asked {
        asking: asking(method, path, body),
        repository,
        number,
        branch: said("branch"),
        path: path.to_owned(),
        body: body.to_owned(),
        token,
    }
}

/// The pull request a REST path names, where it names one:
/// `/repos/owner/name/pulls/41/comments` and `/repos/owner/name/issues/41/…`.
fn numbered(path: &str) -> Option<i64> {
    let mut steps = path.split('/');

    while let Some(step) = steps.next() {
        if step == "pulls" || step == "issues" {
            return steps.next().and_then(|named| named.parse().ok());
        }
    }

    None
}

/// Which question a request is, read off its verb, its path and — for the
/// GraphQL ones, which are all one path — the document it carries.
///
/// Matched on the shape of each question rather than on the whole of it, so a
/// field added to one of the component's questions does not make a suite
/// unreadable. The arms are in the order they have to be told apart in: a
/// repository is made under `/user/repos`, which a match on `/user` would take
/// first.
fn asking(method: &Method, path: &str, body: &str) -> Asking {
    // Without the query, which is the page a list is asked for and nothing
    // that says which question it is.
    let path = path.split('?').next().unwrap_or(path);

    if path.ends_with("/graphql") {
        return match body {
            _ if body.contains("pullRequests(headRefName") => Asking::PullRequest,
            _ if body.contains("pullRequests(states:OPEN") => Asking::Open,
            _ if body.contains("headRefOid") => Asking::Checks,
            _ if body.contains("commits(first:") => Asking::Details,
            _ if body.contains("reviews(first:") => Asking::Comments,
            _ if body.contains("isCrossRepository") => Asking::Numbered,
            _ if body.contains("mergeable state") => Asking::Landing,
            _ => Asking::Unasked,
        };
    }

    match (method, path) {
        (&Method::POST, path) if path.ends_with("/pulls") => Asking::Opening,
        (&Method::GET, path) if path.contains("/pulls/") && path.ends_with("/comments") => {
            Asking::OnTheDiff
        }
        (&Method::POST, path) if path.contains("/issues/") && path.ends_with("/comments") => {
            Asking::Saying
        }
        (&Method::POST, "/gists") => Asking::Publishing,
        (&Method::DELETE, path) if path.starts_with("/gists/") => Asking::Unpublishing,
        (&Method::POST, "/user/repos") => Asking::Making,
        (&Method::GET, "/user") => Asking::Verifying,
        _ => Asking::Unasked,
    }
}

/// The Forge a router under test is stood up with: GitHub's own component,
/// pointed at a stand-in.
///
/// A recipe rather than the Forge itself, because what a Forge needs beside
/// the stand-in is the router's own Data Directory and database — so a test
/// says *which GitHub* and the router constructor it is handed to supplies the
/// rest. See [`crate::router_asking_github`].
#[derive(Debug, Clone)]
pub struct Forge {
    /// Where the component reaches `api.github.com`, or nowhere at all.
    redirect: Redirect,

    /// Whether it reads its tokens out of the router's own Data Directory the
    /// way the served router does — as the Repo's Author, and not at all
    /// where they hold no token. `false` is a stand-in's own token, which is
    /// every Forge that is not under test itself.
    as_the_author: bool,
}

impl Forge {
    /// Reaching `github` where the component asks for GitHub, asking as a
    /// token of the stand-in's own — which is what a test that is about
    /// something else wants.
    pub fn standing_in(github: &GitHub) -> Forge {
        Forge {
            redirect: Redirect::to(DOT_COM, github.address()),
            as_the_author: false,
        }
    }

    /// And the same asked as the Repo's Author, out of the Data Directory's
    /// own `secrets.yaml`, which is how the served router asks: a Repo whose
    /// Author holds no token for this Forge is not asked about at all.
    pub fn as_the_author(github: &GitHub) -> Forge {
        Forge {
            as_the_author: true,
            ..Forge::standing_in(github)
        }
    }

    /// A Forge with nothing behind it: the component is there and whatever it
    /// asks reaches nobody, which is what a test about a GitHub that cannot be
    /// asked stands up.
    ///
    /// Port 1, where nothing listens: the connection is refused at once rather
    /// than waited on.
    ///
    /// Asked as the Repo's Author, like the served router's: an Author with no
    /// token is not asked at all, which is a different answer from one whose
    /// token reached nobody, and a test about either wants the one it set up.
    pub fn unanswerable() -> Forge {
        Forge {
            redirect: Redirect::to(DOT_COM, "127.0.0.1:1"),
            as_the_author: true,
        }
    }

    /// The Forge itself, over the Data Directory `settings` and the database
    /// `pool` the router was built with.
    pub(crate) fn forge(
        &self,
        settings: Settings,
        pool: &SqlitePool,
    ) -> Arc<dyn crate::forges::Forge> {
        self.standing(Some(settings), Some(pool))
    }

    /// And the same with no router behind it, which is what an in-crate test
    /// builds: there is no Data Directory and no database, so what it asks as
    /// is the stand-in's own token.
    #[cfg(test)]
    pub(crate) fn alone(&self) -> Arc<dyn crate::forges::Forge> {
        self.standing(None, None)
    }

    fn standing(
        &self,
        settings: Option<Settings>,
        pool: Option<&SqlitePool>,
    ) -> Arc<dyn crate::forges::Forge> {
        let standing_in = (!self.as_the_author).then(|| crate::forges::plugged::StandingIn {
            under: UNDER.to_owned(),
            token: TOKEN.to_owned(),
        });

        crate::forges::plugged::standing_in_for_github(
            settings.filter(|_| self.as_the_author),
            pool,
            self.redirect.clone(),
            standing_in,
        )
    }
}

impl Forge {
    /// GitHub itself: the component reaching github.com where its name says,
    /// reading its tokens the way the served router does.
    ///
    /// What a router stood up without a stand-in gets — a constructor that
    /// takes one of these and is handed this is a router that would ask the
    /// real GitHub, which nothing under test does.
    pub fn itself() -> Forge {
        Forge {
            redirect: Redirect::none(),
            as_the_author: true,
        }
    }
}
