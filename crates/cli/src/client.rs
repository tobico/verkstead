//! The CLI's end of the API: submit a Set, then hold a reconnecting long-poll
//! until the Response lands — or, where the session ended its turn instead of
//! idling, come back for that Response in one poll that holds for nothing.
//!
//! There is no expiry on the waiting (ADR-0001). The client owns retry, so
//! "nothing yet", a dropped connection, a refused connection and a server
//! restart are all the same thing here: go back and open another wait. Only
//! delivery, a refusal the server will keep repeating — including the Set having
//! been locked unanswered — or a kill ends it.

use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use verkstead_schema::{ApiError, QuestionSet, Response, SetCreated};

/// How long the server is asked to hold each wait open. Its own ceiling is a
/// minute; well under that leaves room for a reply to make it back before any
/// intermediary decides the connection is idle.
const HOLD: Duration = Duration::from_secs(30);

/// How long a single request may take before the client gives up on it and
/// opens another. Comfortably longer than [`HOLD`], so an ordinary "nothing
/// yet" is never mistaken for a dead connection.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// How long to wait before the first retry, and the ceiling the wait doubles
/// up to. Short: the human may be answering right now.
const FIRST_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(10);

pub struct Client {
    agent: ureq::Agent,

    /// What every request is composed onto: the server as it was given, where
    /// that was a URL — and the placeholder URL a pipe is dialled at, where it
    /// was a pipe, because ureq is handed URLs and nothing else.
    base: String,

    /// And what the server is *called*, which is what was given either way.
    /// Everything read by anybody says this, so a pipe that could not be
    /// reached is reported as the pipe rather than as a host that is not one.
    said: String,
}

impl Client {
    /// A client pointed at a Verkstead server, e.g. `http://127.0.0.1:8422` —
    /// or at a named pipe, spelt `pipe://<name>`, which is Windows' own and is
    /// [`crate::pipe`]'s.
    pub fn new(server: &str) -> Result<Self> {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            // Statuses are answers, not errors: a 204 means "nothing yet" and
            // a 422 carries the violations worth printing.
            .http_status_as_error(false)
            .build();

        // Which of the two it is is read off the scheme and nothing else. A
        // pipe wants an agent built around a transport of Verkstead's own; a
        // URL is what ureq does unaided.
        let (agent, base) = match crate::pipe::spelt(server) {
            Some(named) => crate::pipe::dialling(&named, config)?,
            None => (config.into(), server.to_owned()),
        };

        Ok(Client {
            agent,
            base: base.trim_end_matches('/').to_owned(),
            said: server.to_owned(),
        })
    }

    /// Submit a Set and take back the id the wait is held on — or, for a
    /// Deferred Ask, the id and nothing to wait on.
    ///
    /// This one does not retry. A Set is not idempotent — a retry could leave
    /// the human with the same questions twice — and an agent that cannot reach
    /// the server is better off being told so now than blocking forever on a
    /// Set that was never accepted.
    ///
    /// Which kind it is rides in the query string rather than in the Set: the
    /// body is what the agent wrote, and this is how it ran the CLI.
    pub fn submit(&self, set: &QuestionSet, deferred: bool) -> Result<SetCreated> {
        let body = set
            .to_yaml()
            .context("rendering the Question Set as YAML")?;

        // Left off entirely for a blocking ask, which is what every ask was
        // before there were two kinds: the server reads an absent parameter as
        // the blocking one.
        let deferred = match deferred {
            true => "?deferred=true",
            false => "",
        };

        let mut reply = self
            .agent
            .post(format!("{}/api/v1/sets{deferred}", self.base))
            .header("Content-Type", "application/yaml")
            .send(&body)
            .with_context(|| format!("submitting the Question Set to {}", self.said))?;

        let status = reply.status().as_u16();
        let text = reply
            .body_mut()
            .read_to_string()
            .context("reading the server's reply")?;

        if status != 201 {
            bail!("the server refused the Question Set: {}", refusal(&text));
        }

        serde_saphyr::from_str(&text)
            .with_context(|| format!("the server's reply was not a stored Set: {text}"))
    }

    /// Poll once for Set `id`'s Response: the Response itself, or the reason
    /// there is none to take.
    ///
    /// The same endpoint the wait is held on, asked to hold for nothing. This
    /// one does not retry, because every ending here is one the agent is owed
    /// straight away: a Set nobody has answered yet, a Set the human locked
    /// unanswered, a Set asked deferred, and an id that names no Set of this
    /// Conversation are four different things, and each is said as itself.
    ///
    /// Which of this Conversation's Sets is the base URL's business, as it is
    /// everywhere else: an id belonging to another Conversation names nothing
    /// here.
    pub fn fetch(&self, id: i64) -> Result<Response> {
        let url = format!("{}/api/v1/sets/{id}/response?hold=0", self.base);

        let mut reply = self
            .agent
            .get(&url)
            .call()
            .with_context(|| format!("fetching the Response from {}", self.said))?;

        match reply.status().as_u16() {
            200 => {
                let text = reply
                    .body_mut()
                    .read_to_string()
                    .context("reading the Response")?;
                Response::from_yaml(&text).map_err(|error| {
                    anyhow!("the server sent something that is not a Response: {error}\n{text}")
                })
            }
            204 => bail!(
                "Question Set {id} has not been answered yet, so there is nothing \
                 to fetch — wait to be told its Answers have landed"
            ),
            404 => bail!(
                "this Conversation has no Question Set {id} — check the id the ask \
                 that stored it printed"
            ),
            409 => bail!(
                "Question Set {id} was sent with `--deferred`, so its Answers are not \
                 fetched — they go into the prompt of a later session of this \
                 Conversation, and nothing this session does will see them"
            ),
            410 => bail!(
                "Question Set {id} was locked unanswered — the human closed it \
                 without answering, so no Response is coming"
            ),
            status => {
                let text = reply.body_mut().read_to_string().unwrap_or_default();
                bail!("the server answered {status}: {}", refusal(&text))
            }
        }
    }

    /// Block until Set `id` has been answered, reconnecting for as long as it
    /// takes. Retries are reported on stderr, so stdout carries the Response
    /// and nothing else — and as a YAML comment, so that a harness merging the
    /// two streams into one file still hands its agent something that parses.
    pub fn wait(&self, id: i64) -> Result<Response> {
        let mut backoff = FIRST_BACKOFF;

        loop {
            match self.poll(id) {
                Ok(Some(response)) => return Ok(response),
                // Nothing yet: the hold window simply closed. Straight back in.
                Ok(None) => backoff = FIRST_BACKOFF,
                Err(Interrupted::Fatal(error)) => return Err(error),
                Err(Interrupted::Transient(error)) => {
                    eprintln!(
                        "# verkstead: {error:#}; retrying in {}s",
                        backoff.as_secs().max(1)
                    );
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
            }
        }
    }

    /// One wait: the Response, `None` for "nothing yet", or a reason to retry.
    fn poll(&self, id: i64) -> Result<Option<Response>, Interrupted> {
        let hold = HOLD.as_secs();
        let url = format!("{}/api/v1/sets/{id}/response?hold={hold}", self.base);

        let mut reply =
            self.agent.get(&url).call().map_err(|error| {
                Interrupted::Transient(anyhow!("the wait was cut short: {error}"))
            })?;

        match reply.status().as_u16() {
            200 => {
                let text = reply.body_mut().read_to_string().map_err(|error| {
                    Interrupted::Transient(anyhow!("reading the Response: {error}"))
                })?;
                Response::from_yaml(&text)
                    .map(Some)
                    // A reply that is not a Response will not become one on the
                    // next attempt.
                    .map_err(|error| {
                        Interrupted::Fatal(anyhow!(
                            "the server sent something that is not a Response: {error}\n{text}"
                        ))
                    })
            }
            204 => Ok(None),
            404 => Err(Interrupted::Fatal(anyhow!(
                "the server has no Question Set {id} — it may be running against \
                 a different database"
            ))),
            // The human closed the Set without answering it, which is a thing
            // only they can do and is not undone. Retrying would be waiting on a
            // Set nobody is ever going to answer.
            410 => Err(Interrupted::Fatal(anyhow!(
                "Question Set {id} was locked unanswered — the human closed it \
                 without answering, so no Response is coming"
            ))),
            status => {
                let text = reply.body_mut().read_to_string().unwrap_or_default();
                Err(Interrupted::Transient(anyhow!(
                    "the server answered {status}: {}",
                    refusal(&text)
                )))
            }
        }
    }
}

/// Why a wait ended early: because it is worth trying again, or because it
/// never will be.
enum Interrupted {
    Transient(anyhow::Error),
    Fatal(anyhow::Error),
}

/// What the server said when it refused, unpacked from its YAML envelope if it
/// sent one. Violations are listed one per line, each naming its Question.
fn refusal(body: &str) -> String {
    let Ok(error) = serde_saphyr::from_str::<ApiError>(body) else {
        return body.trim().to_owned();
    };

    let mut refusal = error.error;
    for violation in &error.violations {
        refusal.push_str("\n  ");
        refusal.push_str(&violation.to_string());
    }
    refusal
}
