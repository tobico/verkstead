//! `verkstead-desktop` — Verkstead started from an icon: the server in-process,
//! and the viewer in the default browser.
//!
//! There is no window here and no second UI (ADR-0012). The viewer is embedded
//! in the server and installs as a PWA, so what a desktop app adds is lifecycle
//! rather than interface: take the address, run the server, put the viewer in
//! front of the human, and stay out of the way. The headless `verkstead` is
//! untouched by any of it — the GUI dependencies are this crate's alone, which
//! is why this is a crate at all.
//!
//! **The address is settled first, and by this binary.** A Verkstead started
//! from an icon has no terminal for a startup error to be read in, so the one
//! failure that has to be *shown* — something already listening on the port —
//! is found before the server has made anything, while there is still nothing
//! to undo. That is [`Desktop::settle`], and it is why the server has a
//! [`verkstead_server::run_on`] to be handed the socket.

/// Handing a URL to whatever this desktop opens links with.
pub mod browser;
/// The one thing this binary draws for itself.
pub mod dialog;

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};

use anyhow::{Context, Result};
use verkstead_server::Config;

/// What the server logs when `RUST_LOG` says nothing, which is what
/// `crates/cli` logs: its own startup line and whatever else it has to report,
/// and nothing from the crates beneath it.
const DEFAULT_FILTER: &str = "verkstead_server=info";

/// How the desktop app is started.
///
/// One flag of its own, and the server's own beneath it: the app *is* the
/// server, so what points it at a Data Directory, an address and a Watched Path
/// is what points `verkstead serve` at them, said the same way and read from the
/// same environment. Nothing here resolves a directory — started with nothing
/// said, that is the platform's own Data Directory, which is the server's
/// default rather than this binary's doing.
#[derive(Debug, clap::Parser)]
#[command(
    name = "verkstead-desktop",
    version,
    about = "Verkstead on the desktop"
)]
pub struct Desktop {
    /// Don't open the viewer in a browser at startup.
    ///
    /// The server runs exactly as it otherwise would; this is about the window
    /// appearing in front of whatever the human was doing, and nothing else.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub no_open: bool,

    #[command(flatten)]
    pub server: Config,
}

impl Desktop {
    /// Take the address before anything else happens.
    ///
    /// Everything the server does at startup makes something — the Data
    /// Directory, the Skills inside it, the database — and an address already
    /// spoken for is no reason to have made any of it. Bound here rather than
    /// left to the server so that the refusal arrives while there is still
    /// nothing to undo, and while the process has nothing to say but this.
    pub fn settle(&self) -> Result<TcpListener, Taken> {
        TcpListener::bind(self.server.listen).map_err(|why| Taken {
            address: self.server.listen,
            why,
        })
    }

    /// Serve on `listener` until the server stops, with the viewer opened in
    /// front of the human unless [`Desktop::no_open`] said not to.
    ///
    /// **The runtime runs on threads of its own.** `crates/cli` builds one and
    /// blocks the main thread on it; this binary cannot, because the toolkit a
    /// tray icon is drawn with wants the main thread and will not share it. So
    /// the server is spawned onto a runtime and the main thread stays free —
    /// waiting here on the server, for as long as there is nothing else asking
    /// for it.
    pub fn run(self, listener: TcpListener) -> Result<()> {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| DEFAULT_FILTER.into()),
            )
            .init();

        let viewer = viewer_url(self.server.listen);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("starting the async runtime")?;

        let serving = runtime.spawn(verkstead_server::run_on(listener, self.server));

        // After the socket is bound and before the server is up, which is the
        // only moment there is: nothing announces that the router is mounted,
        // and nothing has to. The address is taken, so the browser's request
        // waits in the socket's own queue rather than being refused.
        if !self.no_open {
            // Not being able to open a browser is not a reason to stop serving:
            // the address is in the server's own startup line, a browser
            // pointed at it by hand reaches the same viewer, and so does every
            // other device on the tailnet.
            if let Err(error) = browser::open(&viewer) {
                tracing::warn!(%viewer, "{error:#}");
            }
        }

        runtime
            .block_on(serving)
            .context("the thread the server was running on ended")?
    }
}

/// Where the viewer is, for a browser on this machine.
///
/// The address as it was given, unless that is the unspecified one: bound to
/// `0.0.0.0` the server is reachable on every interface this machine has, and
/// what a browser *here* should be pointed at is the loopback rather than a
/// literal `0.0.0.0` a URL bar has nothing to do with.
fn viewer_url(listen: SocketAddr) -> String {
    let host = match listen.ip() {
        IpAddr::V4(address) if address.is_unspecified() => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen.port())
        }
        IpAddr::V6(address) if address.is_unspecified() => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen.port())
        }
        _ => listen,
    };

    format!("http://{host}/")
}

/// The address could not be taken, which is the one failure this binary draws
/// rather than prints.
///
/// A second copy of the app and the daemon a NixOS module starts are the two
/// ways it happens, and neither is anything to do about here: fronting the
/// server that is already there would conflate two Verksteads of possibly
/// different versions over possibly different Data Directories, and picking
/// another port would leave the human's bookmark pointing at the wrong one.
/// Both were rejected in ADR-0012, so what is left is saying so and stopping.
#[derive(Debug)]
pub struct Taken {
    /// The address the app was told to serve on.
    pub address: SocketAddr,
    /// What the operating system said when it would not hand it over.
    pub why: std::io::Error,
}

impl fmt::Display for Taken {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "Verkstead cannot start: it could not take {} ({}).\n\n\
             Something is already listening there — another copy of this app, or a \
             Verkstead the machine starts for itself. Stop that one, then start \
             Verkstead again.",
            self.address, self.why
        )
    }
}

impl std::error::Error for Taken {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.why)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_viewer_url_is_the_address_the_server_was_given() {
        assert_eq!(
            viewer_url("127.0.0.1:8422".parse().unwrap()),
            "http://127.0.0.1:8422/"
        );
    }

    /// A browser on this machine is pointed at the loopback whatever interfaces
    /// the server was told to answer on — `http://0.0.0.0:8422/` is not an
    /// address a browser has anything to do with.
    #[test]
    fn a_server_on_every_interface_is_opened_on_the_loopback() {
        assert_eq!(
            viewer_url("0.0.0.0:8422".parse().unwrap()),
            "http://127.0.0.1:8422/"
        );
        assert_eq!(
            viewer_url("[::]:8422".parse().unwrap()),
            "http://127.0.0.1:8422/"
        );
    }

    /// The port is the whole of what the human can act on, so it is in the
    /// message rather than in the operating system's own words alone.
    #[test]
    fn the_refusal_names_the_address_it_could_not_take() {
        let taken = Taken {
            address: "127.0.0.1:8422".parse().unwrap(),
            why: std::io::Error::from(std::io::ErrorKind::AddrInUse),
        };

        assert!(taken.to_string().contains("127.0.0.1:8422"), "got: {taken}");
    }
}
