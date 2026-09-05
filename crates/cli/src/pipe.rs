//! The named pipe a Windows session asks through, from the end that dials it.
//!
//! **Why a pipe at all** ([ADR-0014](../../../docs/adr/0014-windows-sessions.md)).
//! An AppContainer is refused the loopback interface, and the exemption is an
//! elevated command per machine that an unsigned per-user install cannot ask
//! for. So a session inside one cannot dial `127.0.0.1`, and what it asks
//! through is a named pipe — `crates/server/src/pipe.rs` is the end that
//! listens, and this is the end that opens it.
//!
//! **The spelling is `pipe://<name>`.** Windows' own `\\.\pipe\<name>` is what
//! the API takes, and it is not what a human is given: this goes in a terminal
//! and in an environment value, and backslashes there are the shell's. So the
//! name travels in a spelling a paste survives, and the rest of a base composes
//! onto it the way a URL's path does —
//! `pipe://verkstead-0123456789abcdef/conversations/7` is a Conversation-scoped
//! base, and `{base}/api/v1/sets` is still the whole of what the client asks.
//! The scheme is the only thing the two are told apart by.
//!
//! **ureq never sees it.** ureq asks a scheme for its default port and refuses
//! one that has none, so `pipe://` would not survive its first URL parse. The
//! spelling is therefore read here: the name is kept for the transport, and
//! what is dialled is a placeholder http URL carrying the path. Nothing
//! resolves that host — the agent is built with a resolver of Verkstead's own
//! that answers without asking anybody, and a connector that opens the pipe and
//! pays the address it is handed no attention at all.
//!
//! **The deadline is the part that does not come free.** A pipe opened as a
//! file has nothing like a socket's read timeout, and the client's whole retry
//! story stands on one: the long poll asks the server to hold for thirty
//! seconds inside a sixty-second request timeout, and a wait that overran is a
//! wait to reopen. So every read and every write here is run on a runtime and
//! given up on at the deadline ureq handed down. Underneath that is Windows'
//! overlapped I/O, which is what tokio's named pipes are; a deadline that
//! passes drops the operation, and the connection with it, so that what comes
//! next is a pipe opened afresh rather than a reply to the request that
//! overran.
//!
//! **This is outside ureq's semver promise.** `Connector`, `Transport` and
//! `Resolver` live under `ureq::unversioned`, whose own documentation says it
//! does not follow semver yet and that breaking changes to it land in *minor*
//! versions rather than major ones. The manifest pins ureq's minor for that
//! reason, and this module is the whole of what that pin is protecting.

use anyhow::{Result, bail};

/// How a `--server` value names a pipe rather than a URL.
///
/// Spelt as a scheme so that the two are told apart by the same thing that
/// tells any two URLs apart, and so that what follows it can be a path.
const SPELLING: &str = "pipe://";

/// A `--server` value that names a pipe, taken apart.
pub struct Named<'a> {
    /// What the pipe is called, without the `\\.\pipe\` Windows puts in front
    /// of one: `verkstead-0123456789abcdef`.
    pub pipe: &'a str,

    /// Everything after the name, which composes onto it the way a URL's path
    /// does — `/conversations/7` for a Conversation-scoped base, and nothing at
    /// all for the server's own.
    pub rest: &'a str,
}

/// The pipe `server` names, or `None` where it names none and is a URL.
///
/// Read off the scheme and nothing else: a value that starts `pipe://` is a
/// pipe, whatever follows, so a name this platform or this Windows will not
/// open is refused as the pipe it is rather than as an unparseable URL.
pub fn spelt(server: &str) -> Option<Named<'_>> {
    let rest = server.strip_prefix(SPELLING)?;
    let named = rest.find('/').unwrap_or(rest.len());

    Some(Named {
        pipe: &rest[..named],
        rest: &rest[named..],
    })
}

/// The agent that dials `named`, and the base its requests compose onto.
///
/// The base is the placeholder URL rather than the spelling that was given:
/// what ureq is handed has to be a URL, and everything a human is shown says
/// the pipe instead — see [`crate::client::Client`], which keeps both.
#[cfg(windows)]
pub fn dialling(named: &Named<'_>, config: ureq::config::Config) -> Result<(ureq::Agent, String)> {
    if named.pipe.is_empty() || named.pipe.contains('\\') {
        bail!(
            "`{SPELLING}{}{}` does not name a pipe: what follows `{SPELLING}` is the \
             pipe's own name, which is one path segment and carries no backslashes — \
             Windows' `\\\\.\\pipe\\` goes on here rather than in what you type",
            named.pipe,
            named.rest,
        );
    }

    let agent =
        ureq::Agent::with_parts(config, dialling::Opening::of(named.pipe), dialling::Unasked);

    Ok((agent, format!("{}{}", dialling::PLACEHOLDER, named.rest)))
}

/// And on the platforms that have no pipes, a refusal saying so.
///
/// Said here rather than left to fail further in: `pipe://` reaches ureq as a
/// scheme it has never heard of, and "unknown scheme" is not what happened.
#[cfg(not(windows))]
pub fn dialling(named: &Named<'_>, _config: ureq::config::Config) -> Result<(ureq::Agent, String)> {
    bail!(
        "`{SPELLING}{}{}` names a named pipe, and named pipes are Windows' own — \
         this build is not a Windows one, and a server here is reached at a URL. \
         Point --server, or VERKSTEAD_SERVER, at one",
        named.pipe,
        named.rest,
    )
}

/// Opening the pipe and speaking HTTP over it: the connector, the transport and
/// the resolver that never asks anybody anything.
///
/// Windows' own, and the whole of what is written against ureq's `unversioned`
/// API — see this module's own documentation for what that costs and what the
/// manifest's pin is for.
#[cfg(windows)]
mod dialling {
    use std::fmt;
    use std::future::Future;
    use std::io;
    use std::net::SocketAddr;
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
    use tokio::runtime::Runtime;
    use ureq::Error;
    use ureq::config::Config;
    use ureq::unversioned::resolver::{ResolvedSocketAddrs, Resolver};
    use ureq::unversioned::transport::{
        Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
    };
    use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

    /// The URL a request over a pipe is dialled at.
    ///
    /// ureq wants a URL and there is no address here, so it is given one that
    /// says what it is and cannot be anything else: `.invalid` is reserved
    /// precisely so that it never resolves, and nothing here asks — see
    /// [`Unasked`]. What the server sees of it is a `Host` header naming a host
    /// that is not one, which is as true as any name a pipe could be given.
    pub const PLACEHOLDER: &str = "http://pipe.verkstead.invalid";

    /// How long to wait before trying a pipe whose every instance is taken.
    ///
    /// A moment rather than a state: the server creates the next instance
    /// before it hands the connected one over, so this is waiting on the
    /// scheduler rather than on anything happening.
    const BUSY: Duration = Duration::from_millis(20);

    /// The connector the agent is built with: it opens the pipe, and pays the
    /// address it was handed no attention at all.
    #[derive(Debug)]
    pub struct Opening {
        /// The pipe as Win32 names one, which is the spelling `CreateFileW`
        /// takes and the one the server created it under.
        name: String,
    }

    impl Opening {
        /// The connector for the pipe called `pipe`, which is a bare name.
        pub fn of(pipe: &str) -> Opening {
            Opening {
                name: format!(r"\\.\pipe\{pipe}"),
            }
        }
    }

    impl Connector for Opening {
        type Out = Dialled;

        fn connect(
            &self,
            details: &ConnectionDetails,
            chained: Option<()>,
        ) -> Result<Option<Self::Out>, Error> {
            // Nothing is ever chained ahead of this one: it is the whole chain
            // the agent was built with, there being no proxy to go through and
            // no TLS to wrap a pipe in.
            debug_assert!(chained.is_none(), "the pipe connector is the whole chain");

            // A runtime per connection, because a pipe's deadline is the
            // runtime's — see this module's own documentation. Current-thread
            // and started here rather than shared: it drives its own I/O on the
            // thread that is blocking on it, so it costs no threads at all, and
            // a connection the pool hands to another thread brings its own.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .map_err(Error::Io)?;

            let pipe = runtime.block_on(open(&self.name, details.timeout))?;

            Ok(Some(Dialled {
                runtime,
                pipe,
                buffers: LazyBuffers::new(
                    details.config.input_buffer_size(),
                    details.config.output_buffer_size(),
                ),
                open: true,
            }))
        }
    }

    /// Open `name`, waiting out an instance that is momentarily busy, and
    /// giving up at the deadline the connect was given.
    async fn open(name: &str, timeout: NextTimeout) -> Result<NamedPipeClient, Error> {
        let opening = async {
            loop {
                match ClientOptions::new().open(name) {
                    Ok(pipe) => return Ok(pipe),
                    // Every instance taken this instant, which is a moment to
                    // wait out rather than an answer — see [`BUSY`].
                    Err(what) if what.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                        tokio::time::sleep(BUSY).await;
                    }
                    Err(what) => return Err(nothing_there(name, what)),
                }
            }
        };

        match timeout.not_zero() {
            None => opening.await,
            Some(after) => tokio::time::timeout(*after, opening)
                .await
                .unwrap_or_else(|_| Err(Error::Timeout(timeout.reason))),
        }
    }

    /// A pipe nothing is listening on, said the way a socket says the same
    /// thing.
    ///
    /// Windows answers an open on a name nobody has created with "the system
    /// cannot find the file specified", which is true and is not what happened:
    /// the server is not running. Said as a refused connection instead, which
    /// is what that is over TCP and what everything above the transport already
    /// knows how to report and retry.
    fn nothing_there(name: &str, what: io::Error) -> Error {
        match what.kind() {
            io::ErrorKind::NotFound => Error::Io(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("nothing is listening on {name}"),
            )),
            _ => Error::Io(what),
        }
    }

    /// One connection: the pipe, the buffers ureq fills and drains, and the
    /// runtime every read and write is given its deadline by.
    pub struct Dialled {
        runtime: Runtime,
        pipe: NamedPipeClient,
        buffers: LazyBuffers,

        /// Whether this is still worth handing back to the pool. A deadline
        /// that passed or an I/O that failed ends the connection here, so that
        /// the wait above reopens rather than retrying down a pipe whose
        /// request is still in flight.
        open: bool,
    }

    /// Which connection this is, for the log ureq keeps about its pool. The
    /// handle rather than the name: every connection here is to the one pipe,
    /// and what is worth saying about one of them is which instance it got.
    impl fmt::Debug for Dialled {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Dialled")
                .field("pipe", &self.pipe)
                .field("open", &self.open)
                .finish()
        }
    }

    impl Transport for Dialled {
        fn buffers(&mut self) -> &mut dyn Buffers {
            &mut self.buffers
        }

        fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), Error> {
            let Dialled {
                runtime,
                pipe,
                buffers,
                open,
            } = self;

            let output = &buffers.output()[..amount];

            within(runtime, timeout, pipe.write_all(output)).inspect_err(|_| *open = false)
        }

        fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, Error> {
            let Dialled {
                runtime,
                pipe,
                buffers,
                open,
            } = self;

            let input = buffers.input_append_buf();
            let amount =
                within(runtime, timeout, pipe.read(input)).inspect_err(|_| *open = false)?;
            buffers.input_appended(amount);

            Ok(amount > 0)
        }

        /// Whether the pool may hand this connection out again.
        ///
        /// The same probe ureq's own socket transport makes, in the shape a
        /// pipe answers it: a read that would block is the one right answer —
        /// there is nothing to read because nothing was asked for. Bytes
        /// waiting are a reply to a request nobody made, and anything else is
        /// the far end gone.
        fn is_open(&mut self) -> bool {
            if !self.open {
                return false;
            }

            let _inside = self.runtime.enter();
            let mut byte = [0u8; 1];

            matches!(
                self.pipe.try_read(&mut byte),
                Err(what) if what.kind() == io::ErrorKind::WouldBlock
            )
        }
    }

    /// Run `work` to its end, or give up on it at `timeout`.
    ///
    /// The deadline a pipe does not come with. `None` is ureq saying this one
    /// has none, which is not the same as a long one: it is what a request with
    /// no configured timeout gets, and waiting forever is then what was asked
    /// for.
    fn within<T>(
        runtime: &Runtime,
        timeout: NextTimeout,
        work: impl Future<Output = io::Result<T>>,
    ) -> Result<T, Error> {
        let done = runtime.block_on(async {
            match timeout.not_zero() {
                None => Some(work.await),
                Some(after) => tokio::time::timeout(*after, work).await.ok(),
            }
        });

        match done {
            Some(Ok(done)) => Ok(done),
            Some(Err(what)) => Err(Error::Io(what)),
            None => Err(Error::Timeout(timeout.reason)),
        }
    }

    /// The resolver the agent is built with: it answers without asking anybody.
    ///
    /// [`PLACEHOLDER`] is a host that does not exist and must not be looked up
    /// — a DNS query on the way to a pipe would be a query for nothing, and on
    /// a machine whose resolver is slow it would be that at sixty seconds a
    /// go. What comes back is one address the connector never reads.
    #[derive(Debug)]
    pub struct Unasked;

    impl Resolver for Unasked {
        fn resolve(
            &self,
            _uri: &ureq::http::Uri,
            _config: &Config,
            _timeout: NextTimeout,
        ) -> Result<ResolvedSocketAddrs, Error> {
            let mut resolved = self.empty();

            // One address, because a resolver owes at least one; this one is
            // the unspecified address, which is the honest answer to a question
            // about a host that is not there.
            resolved.push(SocketAddr::from(([0, 0, 0, 0], 0)));

            Ok(resolved)
        }
    }
}

/// The deadline, which is the one thing a pipe does not bring with it.
///
/// A socket has a read timeout; a pipe opened as a file has nothing, and a
/// server that stopped answering mid-request would hold an ask open for ever.
/// So this is Windows' own, and it is proved against a server that takes the
/// connection and answers nothing — which is what a wedged Verkstead is.
#[cfg(all(test, windows))]
mod deadlines {
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use tokio::net::windows::named_pipe::ServerOptions;

    use super::*;

    /// The whole of what one request is given here. Short on purpose: what is
    /// under test is that there is a deadline at all, and the client's own is
    /// sixty seconds — a test that waited one out would be a minute of nothing.
    const DEADLINE: Duration = Duration::from_millis(300);

    /// A request given up on is given up on near its deadline rather than
    /// somewhere the other side of it. Loose, because a test machine under load
    /// is allowed to be slow; tight enough that a request with no deadline at
    /// all fails this rather than passing it slowly.
    const AT_THE_LATEST: Duration = Duration::from_secs(10);

    /// A server that takes a client and answers it nothing.
    ///
    /// The instance is created before the test is told it is ready, so the
    /// client never dials a name that is not there yet. Nothing joins the
    /// thread: the wait it is standing in for is one nobody is coming back to,
    /// and the process ending is what ends it.
    fn a_server_that_answers_nothing(name: String) -> mpsc::Receiver<()> {
        let (ready, waiting) = mpsc::channel();

        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime for the stand-in server");

            runtime.block_on(async move {
                let instance = ServerOptions::new()
                    .first_pipe_instance(true)
                    .create(&name)
                    .expect("nothing else holds this name");

                ready.send(()).expect("the test is waiting on this");

                instance.connect().await.expect("the client dials in");

                // Held far longer than the deadline, and never answered.
                tokio::time::sleep(Duration::from_secs(60)).await;
            });
        });

        waiting
    }

    /// A request the server never answers is given up on at the deadline,
    /// rather than held for as long as the server holds it.
    #[test]
    fn a_request_the_server_never_answers_is_given_up_on() {
        let name = format!("verkstead-cli-deadline-{}", std::process::id());
        a_server_that_answers_nothing(format!(r"\\.\pipe\{name}"))
            .recv()
            .expect("the stand-in server comes up");

        let config = ureq::Agent::config_builder()
            .timeout_global(Some(DEADLINE))
            .http_status_as_error(false)
            .build();

        let server = format!("pipe://{name}");
        let named = spelt(&server).expect("that is a pipe");
        let (agent, base) = dialling(&named, config).expect("the pipe is there to open");

        let began = Instant::now();
        let outcome = agent
            .get(format!("{base}/api/v1/sets/1/response?hold=30"))
            .call();
        let took = began.elapsed();

        assert!(
            matches!(outcome, Err(ureq::Error::Timeout(_))),
            "a request that overran its deadline is a timeout, got {outcome:?}"
        );
        assert!(
            took < AT_THE_LATEST,
            "it should have been given up on at the deadline, and it took {took:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A URL is not a pipe, whatever else is true of it.
    #[test]
    fn a_url_names_no_pipe() {
        assert!(spelt("http://127.0.0.1:8422").is_none());
        assert!(spelt("http://127.0.0.1:8422/conversations/7").is_none());
        assert!(spelt("https://verkstead.example/conversations/7").is_none());
    }

    /// A server's own base is the pipe and nothing after it.
    #[test]
    fn a_pipe_on_its_own_is_the_whole_of_the_base() {
        let named = spelt("pipe://verkstead-0123456789abcdef").expect("that is a pipe");

        assert_eq!(named.pipe, "verkstead-0123456789abcdef");
        assert_eq!(named.rest, "");
    }

    /// And a Conversation-scoped one carries its path, which is what
    /// `{base}/api/v1/sets` composes onto.
    #[test]
    fn a_conversation_scoped_base_keeps_its_path() {
        let named =
            spelt("pipe://verkstead-0123456789abcdef/conversations/7").expect("that is a pipe");

        assert_eq!(named.pipe, "verkstead-0123456789abcdef");
        assert_eq!(named.rest, "/conversations/7");
    }
}
