//! One request to GitHub, over WASI HTTP: the whole of what this component
//! reaches the outside with.
//!
//! **The host is the only one it can reach.** A request goes out through
//! `wasi:http/outgoing-handler`, which the host holds to the `outbound:` hosts
//! the manifest asked for — `api.github.com`, and whatever host a self-hosted
//! instance was added as. A request to anything else comes back refused
//! without a connection being made, and that refusal is answered as
//! [`Trouble::Refused`] rather than swallowed: a Forge that could not ask is
//! not a Forge that found nothing.
//!
//! **The token is read out of the environment, per call.** The host sets it in
//! the variable the manifest's `secrets` capability names, for the one call it
//! was set for — so there is nothing to hold here, and nothing of a token is
//! ever answered back. An environment with none is [`Trouble::NotLoggedIn`]:
//! there is nobody to ask as, which is the Author holding no token for this
//! Forge.

use wasi::http::outgoing_handler;
use wasi::http::types::{Fields, Method, OutgoingBody, OutgoingRequest, Scheme};
use wasi::io::streams::StreamError;

use crate::Trouble;

/// The variable the host sets the Author's token in, which is the one the
/// manifest's `secrets` capability names.
pub const TOKEN: &str = "GH_TOKEN";

/// What a request is made with: GitHub's media type, the API version it was
/// written against, and a name for the log at the other end — this product's,
/// and nothing of whoever is running it.
const ACCEPT: &str = "application/vnd.github+json";
const VERSION: &str = "2022-11-28";
const AGENT: &str = "verkstead";

/// How much of a body one `blocking-write-and-flush` may take, which is WASI's
/// own limit rather than a choice: a body longer than this goes out in pieces.
const CHUNK: usize = 4096;

/// How many bytes one read of the answer asks for.
const READ: u64 = 16 * 1024;

/// The three verbs this component asks in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Get,
    Post,
    Delete,
}

/// What GitHub answered: the status, its headers — which is where a token's
/// scopes are and nowhere else — and the body.
pub struct Answer {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Answer {
    /// The first value of the header `name`, matched without regard to case,
    /// or `None` where the answer carried none.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(said, _)| said.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// What it said went wrong, in its own words: the `message` of the error
    /// document GitHub answers with, with the status in front of it, or the
    /// status alone where the body said nothing.
    pub fn refusal(&self) -> String {
        let said = serde_json::from_str::<serde_json::Value>(&self.body)
            .ok()
            .and_then(|body| {
                body.get("message")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .filter(|said| !said.trim().is_empty());

        match said {
            Some(said) => format!("HTTP {}: {said}", self.status),
            None => format!("GitHub answered HTTP {}.", self.status),
        }
    }

    /// The answer read as the error kind its status names, or `Ok(())` where
    /// GitHub took it.
    ///
    /// `401` is the token: it is there and GitHub will not have it, which is
    /// the same thing to the human as holding none. A `404` is *not* read as
    /// anything in particular here — what is missing depends on what was asked
    /// for, and the caller is the one that knows. See [`crate::api`].
    pub fn refused(&self) -> Result<(), Trouble> {
        match self.status {
            status if (200..300).contains(&status) => Ok(()),
            401 => Err(Trouble::NotLoggedIn),
            _ => Err(Trouble::Refused(self.refusal())),
        }
    }
}

/// The token this call is to be made with, read out of the environment now.
pub fn token() -> Result<String, Trouble> {
    std::env::vars()
        .find(|(name, _)| name == TOKEN)
        .map(|(_, value)| value)
        .filter(|token| !token.trim().is_empty())
        .ok_or(Trouble::NotLoggedIn)
}

/// Ask `host` at `path`, with `body` where there is one, as the token in the
/// environment.
///
/// `https` always: the scheme is this component's to say, and the one host it
/// could be asked to reach over anything else is a test's stand-in — which the
/// host redirects to on its own side, the grant and the scheme written here
/// untouched.
pub fn ask(verb: Verb, host: &str, path: &str, body: Option<&str>) -> Result<Answer, Trouble> {
    let token = token()?;

    let headers = Fields::new();
    let set = |name: &str, value: &str| {
        headers.append(name, value.as_bytes()).map_err(|error| {
            Trouble::Refused(format!("The request's headers were refused: {error:?}."))
        })
    };

    set("accept", ACCEPT)?;
    set("authorization", &format!("Bearer {token}"))?;
    set("user-agent", AGENT)?;
    set("x-github-api-version", VERSION)?;
    if body.is_some() {
        set("content-type", "application/json")?;
    }

    let request = OutgoingRequest::new(headers);
    let refused = |what: &str| Trouble::Refused(format!("The request's {what} was refused."));

    request
        .set_method(&match verb {
            Verb::Get => Method::Get,
            Verb::Post => Method::Post,
            Verb::Delete => Method::Delete,
        })
        .map_err(|()| refused("method"))?;
    request
        .set_scheme(Some(&Scheme::Https))
        .map_err(|()| refused("scheme"))?;
    request
        .set_authority(Some(host))
        .map_err(|()| refused("host"))?;
    request
        .set_path_with_query(Some(path))
        .map_err(|()| refused("path"))?;

    // Taken before the request is handed over, which is the order WASI asks
    // for: the handler owns the request from then on, and what is left here is
    // the body to write into and the answer to wait for.
    let writing = request
        .body()
        .map_err(|()| Trouble::Refused("The request's body was refused.".to_owned()))?;

    let answering = outgoing_handler::handle(request, None).map_err(|error| {
        Trouble::Refused(format!("GitHub couldn't be asked: {}.", worded(&error)))
    })?;

    // **The answer is waited for whether or not the body went out.** A
    // request the host would not send — a host outside the grant — closes the
    // stream this is written to, so the first thing that fails is the writing
    // and the reason is on the answer. So what goes wrong here is held, and
    // said only where the answer itself had nothing to say.
    let sent = written(&writing, body.unwrap_or_default()).and_then(|()| {
        OutgoingBody::finish(writing, None).map_err(|error| {
            Trouble::Refused(format!(
                "The request's body couldn't be sent: {}.",
                worded(&error)
            ))
        })
    });

    let answered = {
        let waiting = answering.subscribe();
        loop {
            if let Some(answered) = answering.get() {
                break answered;
            }
            waiting.block();
        }
    };

    let answer = answered
        .map_err(|()| Trouble::Refused("GitHub was asked twice for one answer.".to_owned()))?
        .map_err(|error| {
            Trouble::Refused(format!("GitHub couldn't be asked: {}.", worded(&error)))
        })?;

    sent?;

    let status = answer.status();
    let headers = answer
        .headers()
        .entries()
        .into_iter()
        .map(|(name, value)| (name, String::from_utf8_lossy(&value).into_owned()))
        .collect();

    Ok(Answer {
        status,
        headers,
        body: read(answer)?,
    })
}

/// Write `body` into the request, in the pieces WASI takes it in.
fn written(writing: &OutgoingBody, body: &str) -> Result<(), Trouble> {
    if body.is_empty() {
        return Ok(());
    }

    let stream = writing
        .write()
        .map_err(|()| Trouble::Refused("The request's body was refused.".to_owned()))?;

    for piece in body.as_bytes().chunks(CHUNK) {
        stream.blocking_write_and_flush(piece).map_err(|error| {
            Trouble::Refused(format!(
                "The request's body couldn't be sent: {}.",
                streamed(&error)
            ))
        })?;
    }

    Ok(())
}

/// The answer's body, read to its end.
fn read(answer: wasi::http::types::IncomingResponse) -> Result<String, Trouble> {
    let body = answer
        .consume()
        .map_err(|()| Trouble::Refused("GitHub's answer couldn't be read.".to_owned()))?;
    let stream = body
        .stream()
        .map_err(|()| Trouble::Refused("GitHub's answer couldn't be read.".to_owned()))?;

    let mut bytes: Vec<u8> = Vec::new();
    loop {
        match stream.blocking_read(READ) {
            Ok(piece) => bytes.extend(piece),
            Err(StreamError::Closed) => break,
            Err(error) => {
                return Err(Trouble::Refused(format!(
                    "GitHub's answer couldn't be read: {}.",
                    streamed(&error)
                )));
            }
        }
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// What WASI said went wrong, in one line.
fn worded(error: &wasi::http::types::ErrorCode) -> String {
    format!("{error:?}")
}

/// And the same for a stream's.
fn streamed(error: &StreamError) -> String {
    match error {
        StreamError::Closed => "it was closed".to_owned(),
        StreamError::LastOperationFailed(error) => error.to_debug_string(),
    }
}
