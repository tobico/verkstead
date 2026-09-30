//! Trying a declared MCP server, once, at the moment it is saved.
//!
//! One MCP `initialize` over streamable HTTP, made by this server with the
//! declaration's own headers, **after** the save has been written — a token is
//! pasted once out of a page that will not show it again, and a URL is typed
//! once beside it; neither is worth throwing away because the network was
//! briefly down. So the declaration lands and this says what happened when it
//! was spoken to, which is [`verkstead_render::Verified`]'s arrangement said
//! about a server.
//!
//! **A report rather than a refusal, and at save rather than at launch.** A
//! server that cannot be reached from here may be reachable tomorrow, or only
//! from inside a session's network, so nothing here turns a save down. And
//! nothing tries a server when it is attached or when a session starts: an
//! unreachable server never holds a launch, which is ADR-0021's decision and
//! the reason this is the one place a request is made at all.
//!
//! **The handshake is not finished, on purpose.** What follows an `initialize`
//! in a real client is a `notifications/initialized` and, on a stateful server,
//! a session to close afterwards. Nothing here sends either: the question is
//! whether anything answers at all, and a probe that opened a session would be
//! one the page had to clean up after. A server that holds one open drops it on
//! its own timeout, the way it drops any client that goes away.
//!
//! **Nothing the server sends back is quoted.** Its headers are secrets, and a
//! service that echoed one into an error message would otherwise put it on the
//! page — see [`Tried`]. What is said is Verkstead's own words about which of
//! the three ways it went wrong, and the one thing carried over from the server
//! itself is the name it gives for itself.

use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use verkstead_render::{ServerTried, Tried};

use crate::settings::AttachedServer;

/// How long a declaration has to answer before the save comes back without it.
///
/// Short on purpose: this is paid inside a press somebody is waiting on, and
/// what it buys is a sentence on a row. A server too slow to answer in this is
/// one the human is told did not answer, and the declaration is saved all the
/// same.
pub(crate) const ANSWERS_WITHIN: Duration = Duration::from_secs(5);

/// The protocol version Verkstead introduces itself as speaking.
///
/// A server that speaks another answers with its own, which is the negotiation
/// working: what is being asked here is whether anything answers `initialize`
/// at all, and a version this one has never heard of is still an answer.
const PROTOCOL: &str = "2025-06-18";

/// Who is asking, in the `clientInfo` every `initialize` carries.
const ASKING: &str = "verkstead";

/// What `initialize` is answered as, of which two fields are any of our
/// business: that there is a result at all, and what the server calls itself.
#[derive(serde::Deserialize)]
struct Answer {
    result: Option<Initialized>,
}

#[derive(serde::Deserialize)]
struct Initialized {
    #[serde(rename = "serverInfo")]
    server_info: Option<ServerInfo>,
}

#[derive(serde::Deserialize)]
struct ServerInfo {
    name: Option<String>,
}

/// Try each of `servers`, and say what came of each by name.
///
/// All at once rather than one after another: the page saves the section as one
/// list, so the press is waiting on the slowest of them rather than on the sum.
pub(crate) async fn tried(servers: &[AttachedServer]) -> Vec<ServerTried> {
    let client = match reqwest::Client::builder().timeout(ANSWERS_WITHIN).build() {
        Ok(client) => client,
        // Nothing left to say about any of them. A client that will not build is
        // this process's own trouble rather than anything about a declaration,
        // and the save itself has already landed.
        Err(error) => {
            tracing::warn!(error = ?error, "no HTTP client to try the declared MCP servers with");
            return Vec::new();
        }
    };

    // Each on a task of its own, and then awaited in the order they were
    // declared: what comes back is the list the page draws, and the press pays
    // the slowest of them rather than the sum.
    let asking: Vec<_> = servers
        .iter()
        .map(|server| {
            let client = client.clone();
            let server = server.clone();

            tokio::spawn(async move {
                let outcome = initialize(&client, &server).await;

                ServerTried {
                    server: server.name().to_owned(),
                    outcome,
                }
            })
        })
        .collect();

    let mut tried = Vec::with_capacity(asking.len());

    for asked in asking {
        match asked.await {
            Ok(outcome) => tried.push(outcome),
            // Nothing to say about that one. The save has landed either way, and
            // a row with nothing beside it is what a server nobody asked about
            // looks like.
            Err(error) => {
                tracing::warn!(error = ?error, "trying a declared MCP server did not finish");
            }
        }
    }

    tried
}

/// Speak to one, and say what it made of being spoken to.
async fn initialize(client: &reqwest::Client, server: &AttachedServer) -> Tried {
    let headers = match sending(server) {
        Ok(headers) => headers,
        Err(header) => {
            return Tried::Refused {
                why: format!(
                    "It could not be spoken to: the {header} header is not one HTTP can carry."
                ),
            };
        }
    };

    // Both content types offered, because streamable HTTP answers in either:
    // a server with nothing to stream answers `application/json`, and one that
    // streams answers a `text/event-stream` with the same JSON-RPC inside it.
    let answered = client
        .post(server.url())
        .headers(headers)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL,
                "capabilities": {},
                "clientInfo": { "name": ASKING, "version": env!("CARGO_PKG_VERSION") },
            },
        }))
        .send()
        .await;

    let answered = match answered {
        Ok(answered) => answered,
        Err(error) => {
            return Tried::Refused {
                why: unanswered(&error),
            };
        }
    };

    let status = answered.status();

    // The one status that means something a human can act on without being told
    // anything else: what a declaration authenticates with is its headers, so a
    // server that will not take them is saying the value is wrong or missing.
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Tried::Refused {
            why: format!(
                "It answered {status}, which means the headers it was sent were not accepted."
            ),
        };
    }

    if !status.is_success() {
        return Tried::Refused {
            why: format!("What answered is not an MCP server: it answered {status}."),
        };
    }

    // The body read here rather than streamed: what is being asked is whether
    // the first thing it says is an `initialize` result, and the deadline above
    // covers the reading as well as the connecting.
    let body = match answered.text().await {
        Ok(body) => body,
        Err(error) => {
            return Tried::Refused {
                why: unanswered(&error),
            };
        }
    };

    match named(&body) {
        Some(named) => Tried::Reached { named },
        // Everything that answered and was not an `initialize` result, which is
        // a page, a proxy's apology, a JSON-RPC error, or a server speaking
        // something else entirely. One sentence for the lot, because the one
        // thing that would tell them apart is the body — and the body is what
        // this is not allowed to quote.
        None => Tried::Refused {
            why: "What answered is not an MCP server: it did not answer `initialize`.".to_owned(),
        },
    }
}

/// The headers the declaration is spoken to with, or the name of the first that
/// will not go on the wire.
///
/// The name is named and the value never is: a value that is not a header value
/// is still a secret, and the name is what the human has to go and look at.
fn sending(server: &AttachedServer) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();

    for (name, value) in server.headers() {
        let header = HeaderName::try_from(name.as_str()).map_err(|_| name.clone())?;
        let value = HeaderValue::try_from(value.as_str()).map_err(|_| name.clone())?;

        headers.insert(header, value);
    }

    Ok(headers)
}

/// Why nothing answered, in words to put on the row.
///
/// The deadline is named where it was the deadline, because that is the one
/// case where the human's next move is to wonder whether the server is merely
/// slow. Everything else is the reason the request itself gave — a name that
/// would not resolve, a connection refused, a certificate — which is what
/// distinguishes a URL with a typo in it from a server that is down.
fn unanswered(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return format!(
            "It did not answer within {} seconds.",
            ANSWERS_WITHIN.as_secs()
        );
    }

    format!("It did not answer: {}.", underneath(error))
}

/// The innermost thing that went wrong, which is the one worth reading: the
/// outer layers of a request error say it was a request that failed, and the
/// last one says a name would not resolve or a connection was refused.
///
/// No header ever reaches one of these — they are about the socket rather than
/// about what was to go down it — so nothing here can carry a value out.
fn underneath(error: &reqwest::Error) -> String {
    let mut deepest: &dyn std::error::Error = error;

    while let Some(under) = deepest.source() {
        deepest = under;
    }

    deepest.to_string()
}

/// The name a server gives for itself in what it answered, where what it
/// answered is an `initialize` result at all.
///
/// `None` is *not an answer to `initialize`*, and `Some(None)` is an answer
/// from a server that named itself nothing — which is reachable, said in fewer
/// words.
fn named(body: &str) -> Option<Option<String>> {
    let answer: Answer = serde_json::from_str(body).ok().or_else(|| streamed(body))?;

    Some(
        answer
            .result?
            .server_info
            .and_then(|server| server.name)
            .filter(|name| !name.trim().is_empty()),
    )
}

/// And the same body read as an event stream, which is the other shape
/// streamable HTTP answers in: `data:` lines carrying the JSON-RPC message.
///
/// The first one that parses is the answer. There is one request in flight and
/// its response is what comes back first; anything after it is a notification
/// this is not waiting for.
fn streamed(body: &str) -> Option<Answer> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .find_map(|data| serde_json::from_str(data.trim()).ok())
}

#[cfg(test)]
mod tests {
    use super::named;

    /// What a server answers `initialize` with, as the shape this reads.
    fn initialized(server_info: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"result":{{"protocolVersion":"2025-06-18","capabilities":{{}}{server_info}}}}}"#
        )
    }

    #[test]
    fn a_server_that_names_itself_is_reached_by_that_name() {
        assert_eq!(
            named(&initialized(
                r#","serverInfo":{"name":"docs","version":"1.0"}"#
            )),
            Some(Some("docs".to_owned())),
        );
    }

    #[test]
    fn one_that_names_itself_nothing_is_reached_all_the_same() {
        assert_eq!(named(&initialized("")), Some(None));
        assert_eq!(
            named(&initialized(r#","serverInfo":{"version":"1.0"}"#)),
            Some(None)
        );
        assert_eq!(
            named(&initialized(r#","serverInfo":{"name":"  "}"#)),
            Some(None)
        );
    }

    /// The other shape streamable HTTP answers in, which is the same JSON-RPC
    /// message on a `data:` line.
    #[test]
    fn an_event_stream_is_read_for_the_same_answer() {
        let body = format!(
            "event: message\ndata: {}\n\n",
            initialized(r#","serverInfo":{"name":"docs"}"#)
        );

        assert_eq!(named(&body), Some(Some("docs".to_owned())));
    }

    #[test]
    fn a_json_rpc_error_is_not_an_answer_to_initialize() {
        assert_eq!(
            named(r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"no"}}"#),
            None,
        );
    }

    #[test]
    fn a_page_is_not_an_answer_to_initialize() {
        assert_eq!(named("<!doctype html><title>Hello</title>"), None);
        assert_eq!(named(""), None);
    }
}
