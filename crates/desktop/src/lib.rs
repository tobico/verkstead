//! `verkstead-desktop` — Verkstead started from an icon: the server in-process,
//! the viewer in the default browser, and an icon in the tray over the two of
//! them.
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
//!
//! **The main thread is the tray's**, and the server runs beside it. The
//! platform's own toolkit holds the thread its loop is running on — see
//! [`toolkit`] — so the server is spawned onto a runtime of its own threads and
//! the two meet at the menu: what is picked off it is handled on the loop's
//! thread, and the server ending is brought back to that thread to end the loop
//! with it. A session with no tray to put an icon in — over SSH, in a
//! container, under a test — is not a failure and not a reason to stop serving,
//! so there the main thread waits on the server as `verkstead serve` does.
//!
//! **And the logging goes to a file**, which is the other thing about being
//! started from an icon: there is no terminal for a stdout to be read in, so
//! the server's `tracing` is written to the Log Directory and the tray gets the
//! item that opens it — see [`logs`].
//!
//! **Whether it comes up again next login is the desktop's answer rather than
//! Verkstead's.** The last item on the menu before Exit is a checkbox over the
//! platform's own startup registration — read from it, written to it, and
//! rewritten at every launch while it is there, with nothing of Verkstead's own
//! keeping a second copy of the answer. See [`startup`].
//!
//! **And what it registers is the invocation that started it**, which is the one
//! thing this library is told rather than reads. The image it is running out of
//! has other verbs — `verkstead ask` is the same file — so the executable's path
//! alone is no longer a command that starts the app. Whatever entered here says
//! which way it came in: see [`startup::Entered`], handed to [`Desktop::run`].

/// The two things this binary draws that carry words.
pub mod dialog;
/// Where the server's `tracing` goes, and what View Logs opens.
pub mod logs;
/// Handing a URL or a file to whatever this desktop opens it with.
pub mod opener;
/// Whether this process has a screen to draw on.
pub mod screen;
/// Whether Verkstead comes up when the desktop session does.
pub mod startup;
/// The loop the tray lives on, and the two ways it ends.
pub mod toolkit;
/// The icon in the system tray, and what is on its menu.
pub mod tray;

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::mpsc::sync_channel;

use anyhow::{Context, Result};
use tray_icon::TrayIcon;
use verkstead_server::Config;

/// What Verkstead is called wherever a platform asks for an identifier rather
/// than a name (ADR-0012).
///
/// The tray's own id, and the name the startup registration is written under —
/// one string, because a platform that was told two would have two Verksteads.
pub const APP_ID: &str = "net.tobico.Verkstead";

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

    /// Serve on `listener` until Exit is chosen or the server stops, with the
    /// viewer opened in front of the human unless [`Desktop::no_open`] said not
    /// to.
    ///
    /// **The runtime runs on threads of its own.** `crates/cli` builds one and
    /// blocks the main thread on it; this binary cannot, because the toolkit
    /// the tray icon is drawn with wants the main thread and will not share it.
    /// So the server is spawned onto a runtime and the main thread goes to the
    /// tray's loop — or waits on the server, where there is no tray to have.
    ///
    /// `entered` is how this app was run — the verb it came in through, where
    /// it came in through one — which is what a startup registration written
    /// from in here has to name beside the executable's own path. See
    /// [`startup::Entered`].
    ///
    /// **Exit is a stop where it stands.** The server has never had a shutdown
    /// path — nothing in it handles a signal, and under systemd it is stopped
    /// by SIGTERM and dies where it is — so the tray does not get machinery no
    /// other caller of the server has. What that leaves behind is nothing: the
    /// socket closes with the process, and every session and the shared compile
    /// server go when this goes, by whichever means the platform has for
    /// saying so — `bwrap --die-with-parent` on Linux, and a keeper watching
    /// from outside the process on a Mac, which has no such flag. Neither
    /// needs a word from here, which is why Exit can be a stop at all.
    pub fn run(self, listener: TcpListener, entered: startup::Entered) -> Result<()> {
        // Before anything has anything to report, and by this binary rather
        // than by the server: where the events go is the starting binary's
        // call, and this one was started from an icon — see [`logs`].
        let logging = logs::start();

        // And before anything else this launch does, because it is about *this*
        // launch: while the box is checked, the startup registration is
        // rewritten with the invocation that is running, so a binary somebody
        // moved heals its own registration the next time they start it by hand
        // — see [`startup`].
        let startup = startup::Startup::here(entered);
        startup.refresh();

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
            if let Err(error) = opener::url(&viewer) {
                tracing::warn!(%viewer, "{error:#}");
            }
        }

        let Some(tray) = raise(&viewer, &logging, &startup) else {
            // No tray to be in, so this is `verkstead serve` with a browser
            // opened: the main thread waits on the server, and the process is
            // stopped the way that one is.
            return runtime
                .block_on(serving)
                .context("the thread the server was running on ended")?;
        };

        // The server's own ending, brought to the thread the loop is about to
        // hold: a server that has stopped is an app with nothing left to be the
        // tray of, and nobody watching a terminal for it to say so in.
        let (ended, has_ended) = sync_channel(1);
        runtime.spawn(async move {
            let _ = ended.send(serving.await);
            toolkit::stop_from_elsewhere();
        });

        toolkit::run();

        // Read before the runtime is let go of below: letting go of it cancels
        // the task that does the sending, and a cancellation arriving here
        // would read as a server that stopped when what stopped was the app.
        let ended = has_ended.try_recv();

        // Out of the tray before the process goes, and then the runtime let go
        // of rather than waited on — see this method's own docs for what Exit
        // leaves behind, which is nothing.
        drop(tray);
        runtime.shutdown_background();

        match ended {
            // The server stopped, which is the only way the loop ends with
            // something to report.
            Ok(outcome) => outcome.context("the thread the server was running on ended")?,
            // Exit, which is the loop ending on its own account.
            Err(_) => Ok(()),
        }
    }
}

/// The icon in the tray, or `None` where there is nowhere to put one.
///
/// **None of the ways there is nowhere is a reason to stop serving.** No screen
/// at all — over SSH, in a container, under a test — is a Verkstead serving
/// browsers elsewhere and nothing wrong with it; a screen that is named and
/// cannot be opened, or a tray that will not take the icon, is a machine to say
/// something about in the log. What is left in each case is the server and the
/// viewer, which is the useful half of the app.
///
/// A desktop with no tray host running is *not* one of them: an appindicator
/// registers on the bus whether or not anything is drawing it, so an icon
/// nobody shows is one this cannot tell from an icon somebody does. macOS has
/// no such question — the menu bar is the session's own and always there.
///
/// `viewer` is where Open sends the browser: the same URL that was opened at
/// startup, now on demand. `logging` is what View Logs opens, or what it says
/// where this machine had nowhere to keep a log file. `startup` is the
/// registration the Launch on Startup box is drawn from and written to.
fn raise(viewer: &str, logging: &logs::Kept, startup: &startup::Startup) -> Option<TrayIcon> {
    if !screen::there_is_one() {
        tracing::info!("there is no screen here, so Verkstead is running as the server alone");
        return None;
    }

    if let Err(error) = toolkit::start() {
        tracing::warn!("the desktop toolkit would not start, so there is no tray icon: {error:#}");
        return None;
    }

    let viewer = viewer.to_owned();
    let logging = logging.clone();
    let startup = startup.clone();

    // What the box is ticked to as the menu is made, which is what the
    // registration says right now — or nothing to tick, on a machine with
    // nowhere to keep one.
    let ticked = startup.possible().then(|| startup.on());

    let raised = tray::show(ticked, move |chosen| match chosen {
        tray::Chosen::Open => {
            // Said and carried on, for the reason the open at startup is: the
            // viewer is reachable from every browser on the tailnet, and a
            // desktop that would not open one is no reason to stop serving them.
            if let Err(error) = opener::url(&viewer) {
                tracing::warn!(%viewer, "{error:#}");
            }
        }
        tray::Chosen::ViewLogs => match &logging {
            // The same handing-over the viewer gets, at whatever this desktop
            // reads a text file with.
            logs::Kept::In(file) => {
                if let Err(error) = opener::file(file) {
                    tracing::warn!("{error:#}");
                }
            }
            // And where there is no file, the reason there is none — said on
            // the screen, because the log it would otherwise be written to is
            // the thing this machine has not got.
            logs::Kept::Nowhere(why) => dialog::note(why),
        },
        tray::Chosen::LaunchOnStartup => {
            // What the box in front of them now says, which is what they were
            // reaching for: the item ticks itself before the pick is reported.
            // Read from the item rather than inverted from the registration,
            // because the menu is drawn once and a desktop's own settings can
            // turn the registration off underneath it — and then an inverted
            // read would register Verkstead for somebody who was unticking it.
            let wanted = tray::launch_on_startup_shows().unwrap_or(!startup.on());

            if let Err(error) = startup.set(wanted) {
                tracing::warn!("{error:#}");
                dialog::refusal(&format!("{error:#}"));
            }

            // Whatever actually holds, read back off the registration: a write
            // that did not happen is a tick that goes back where it was.
            tray::shows_launch_on_startup(startup.on());
        }
        // Which ends the loop `run` is blocked on, and with it the process.
        tray::Chosen::Exit => toolkit::stop(),
    });

    match raised {
        Ok(icon) => {
            // Said on the way through, where each of the two refusals above is
            // said: the log is the only mark any of this leaves, and a reader
            // who has been told what *would* have gone wrong is owed the line
            // saying nothing did. It is also what the release workflow's
            // desktop leg reads to know that the bundle it has just built can
            // raise a tray at all — a headless run reaches neither the toolkit
            // nor the appindicator, so this line is the whole of what tells the
            // two apart. See `.github/workflows/release.yml`.
            tracing::info!("Verkstead is in the tray");
            Some(icon)
        }
        Err(error) => {
            tracing::warn!("{error:#}");
            None
        }
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
