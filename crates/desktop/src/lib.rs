//! Verkstead started from an icon: the server in-process, the viewer in the
//! default browser, and an icon in the tray over the two of them.
//!
//! A library rather than a binary, and reached as `verkstead desktop` (ADR-0012,
//! amended): one image serves the workbench and answers `ask`, so a session
//! handed the running server's own file can ask with it. What is left of the old
//! `verkstead-desktop` is the Windows shim beside this file, which starts that
//! verb for a Start-menu shortcut that cannot say one.
//!
//! There is no window here and no second UI (ADR-0012). The viewer is embedded
//! in the server and installs as a PWA, so what a desktop app adds is lifecycle
//! rather than interface: take the address, run the server, put the viewer in
//! front of the human, and stay out of the way. The headless `verkstead` is
//! untouched by any of it — the GUI dependencies are this crate's alone, which
//! is why this is a crate at all, and turning the CLI's `desktop` feature off is
//! what leaves them out.
//!
//! **The address is settled first, and by the app.** A Verkstead started from an
//! icon has no terminal for a startup error to be read in, so the one failure
//! that has to be *shown* — something already listening on the port — is found
//! before the server has made anything, while there is still nothing to undo.
//! That is [`Desktop::settle`], and it is why the server has a
//! [`verkstead_server::run_on_keyed`] to be handed the socket — and the
//! Workbench Key, which the app has to hold before it can open a browser that
//! is logged in. See [`Desktop::run`].
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
//! **And the operator grant is asked for rather than shown.** The Remote access
//! pane's serve switch runs a command Tailscale refuses from anybody but the
//! tailnet's operator, and the server has no privilege to raise — so what it
//! hands a refused press is the line that lifts it. An app has something a
//! daemon has not, which is somebody at the machine to ask: this one hands the
//! server the platform's own password dialog on its way in, and a refused press
//! raises that instead. The dialog is the server crate's own — it is three
//! spawned commands with no toolkit behind them, so nothing about it needed this
//! crate: see [`verkstead_server::elevate`], and [`verkstead_server::remote`]
//! for the other arm.
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
//! alone is not a command that starts the app. Whatever entered here says which
//! verb it came in through: see [`startup::Entered`], handed to [`Desktop::run`].

/// The two things this app draws that carry words.
pub mod dialog;
/// Where the server's `tracing` goes, and what View Logs opens.
pub mod logs;
/// Handing a URL or a file to whatever this desktop opens it with.
pub mod opener;
/// Whatever draws the tray on this desktop, and whether it is there yet.
///
/// Linux's alone: it is the one platform where the thing that draws the icon is
/// a program that can be missing, or late.
#[cfg(target_os = "linux")]
pub mod panel;
/// Whether Verkstead comes up when the desktop session does.
pub mod startup;
/// The loop the tray lives on, and the two ways it ends.
pub mod toolkit;
/// The icon in the system tray, and what is on its menu.
pub mod tray;

use std::fmt;
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::mpsc::sync_channel;

use anyhow::{Context, Result};
use tray_icon::TrayIcon;
use verkstead_server::Config;
use verkstead_server::key::{HandsOverTheLink, WorkbenchKey, login_link, workbench_address};

/// What Verkstead is called wherever a platform asks for an identifier rather
/// than a name (ADR-0012).
///
/// The tray's own id, and the name the startup registration is written under —
/// one string, because a platform that was told two would have two Verksteads.
pub const APP_ID: &str = "net.tobico.Verkstead";

/// How the desktop app is started.
///
/// One flag of its own, and the server's own beneath it: the app *is* the
/// server, so what points it at a Data Directory and an address is what points
/// `verkstead serve` at them, said the same way and read from the same
/// environment. Nothing here resolves a directory — started with nothing
/// said, that is the platform's own Data Directory, which is the server's
/// default rather than the app's doing.
///
/// [`clap::Args`] rather than [`clap::Parser`], because this is what a verb
/// takes rather than what a binary is: what names the command, versions it and
/// describes it is `crates/cli`'s own enum — see `Command::Desktop` there.
#[derive(Debug, clap::Args)]
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
    /// **The runtime runs on threads of its own.** `verkstead serve` builds one
    /// and blocks the main thread on it; this verb cannot, because the toolkit
    /// the tray icon is drawn with wants the main thread and will not share it.
    /// So the server is spawned onto a runtime and the main thread goes to the
    /// tray's loop — or waits on the server, where there is no tray to have.
    ///
    /// `entered` is how this app was run — the verb it came in through — which
    /// is what a startup registration written from in here has to name beside
    /// the executable's own path. See [`startup::Entered`].
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
        // Before anything has anything to report, and by the app rather than by
        // the server: where the events go is the starting binary's call, and
        // this verb of it was started from an icon — see [`logs`].
        let logging = logs::start();

        // And before anything else this launch does, because it is about *this*
        // launch: while the box is checked, the startup registration is
        // rewritten with the invocation that is running, so a binary somebody
        // moved heals its own registration the next time they start it by hand
        // — see [`startup`].
        let startup = startup::Startup::here(entered);
        startup.refresh();

        // The Workbench Key, before there is a server to ask one of. The
        // workbench answers 401 to anything that has not shown it (ADR-0015), so
        // a browser opened on the bare address would land on a refusal — and the
        // key has to be the *server's*, which is what handing this handle to it
        // below makes it. See [`verkstead_server::Config::workbench_key`].
        let key = self.server.workbench_key()?;
        let listen = self.server.listen;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("starting the async runtime")?;

        // And handed over with it: holding the key already is the same fact as
        // handing the link out here rather than leaving it to a log line. The
        // browser below is opened on it and the tray's **Open** opens another
        // whenever it is pressed, so the server's startup line names the
        // address alone — the log file **View Logs** opens is a file on
        // somebody's desk, and a workbench key in it is a login anybody reading
        // over a shoulder has (ADR-0015).
        //
        // **Said here because the line is written before the tray is raised**,
        // and a tray that cannot be raised is not known about until it is
        // tried. Where that happens the app says the link itself, below, rather
        // than leaving a machine with nothing to log in with: this is what the
        // install *intends*, and the fallback is what it does where the
        // intention turns out not to hold.
        //
        // Which is why this arm is said here rather than taken from
        // [`StartedBy::hands_over_the_link`], where the sidecar's own answer to
        // a missing display is the daemon's line: the sidecar has nothing after
        // its startup line, and this app has the line below.
        //
        // The grant *is* taken from there, because there is nothing about it
        // this app answers differently — see [`verkstead_server::StartedBy`],
        // which is the one place the question is asked now.
        let started_by = verkstead_server::StartedBy::TheDesktopApp;
        let serving = runtime.spawn(verkstead_server::run_on_keyed(
            listener,
            self.server,
            key.clone(),
            started_by.escalation(verkstead_server::display::there_is_one()),
            HandsOverTheLink::TheCaller,
        ));

        // After the socket is bound and before the server is up, which is the
        // only moment there is: nothing announces that the router is mounted,
        // and nothing has to. The address is taken, so the browser's request
        // waits in the socket's own queue rather than being refused.
        if !self.no_open {
            // Not being able to open a browser is not a reason to stop serving:
            // the tray's **Open** hands the same link over at every press, a
            // browser pointed at it by hand reaches the same viewer, and so
            // does every other device on the tailnet.
            //
            // **And what is said about it is the address rather than the
            // link.** The link is the whole of logging in, so a line carrying
            // one is the secret in the log — which is the thing this install
            // keeps out of the file **View Logs** opens, and a warning is as
            // much that file as the startup line is (ADR-0015). What a reader
            // of it needs is which Verkstead would not open and why, and the
            // address is that — the error's own account of what it was opening
            // included, which is why the opener is told what to name.
            let workbench = workbench_address(listen);

            if let Err(error) = opener::url_reported_as(&login_link(listen, &key), &workbench) {
                tracing::warn!(%workbench, "{error:#}");
            }
        }

        let raised = raise(listen, &key, &logging, &startup);

        // **The link, wherever there is no icon to press Open in — which
        // includes the run that is about to grow one.** The startup line names
        // the address alone on the reasoning that this install hands the link
        // out itself, and the whole of that handing out is the browser above and
        // the tray's **Open**. A run that reached here has no tray yet, and one
        // that reached here with `--no-open` or a browser that would not start
        // has had neither: leaving the link off *here* would be the
        // redacting-everywhere that ADR-0015 rejected, and would leave a machine
        // serving a workbench nobody can get into. So this is the daemon's way,
        // taken by the app exactly where the app is a daemon — for the whole run
        // below, or for however much of it passes before a tray turns up.
        if !matches!(raised, Raised::Now(_)) {
            tracing::info!(
                workbench = %login_link(listen, &key),
                "there is no tray to press Open in, so this is the way in",
            );
        }

        let tray = match raised {
            Raised::Now(icon) => Some(icon),
            // The loop is held anyway, because the icon can still go up in it:
            // what is waiting for a tray raises the icon from inside this loop
            // and leaves it with [`tray::keep`], so there is nothing to hold
            // here. See [`raise`].
            Raised::WhenATrayArrives => None,
            // No tray to be in and none coming, so this is `verkstead serve`
            // with a browser opened: the main thread waits on the server, and
            // the process is stopped the way that one is.
            Raised::Nowhere => {
                return runtime
                    .block_on(serving)
                    .context("the thread the server was running on ended")?;
            }
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
        // And the other one, where the icon went up inside the loop that has
        // just ended: the loop's thread is this thread, which is the thread
        // holding it. See [`tray::keep`].
        tray::let_go();
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

/// Where the icon went, which is not always up and not always now.
enum Raised {
    /// It is in the tray, and this is it: dropping it takes it out again.
    Now(TrayIcon),
    /// Not yet, and not never. There is a screen and a toolkit here and nothing
    /// drawing a tray on the bus — so the icon is offered again when something
    /// arrives at the name a tray owns, and until then this app is the loop with
    /// no icon in it. Linux's alone; see [`panel`].
    WhenATrayArrives,
    /// Nowhere, and nothing about this run will change that.
    Nowhere,
}

/// Put the icon in the tray, or say why it is not there.
///
/// **None of the ways it is not there is a reason to stop serving.** No screen
/// at all — over SSH, in a container, under a test — is a Verkstead serving
/// browsers elsewhere and nothing wrong with it; a screen that is named and
/// cannot be opened, or a tray that will not take the icon, is a machine to say
/// something about in the log. What is left in each case is the server and the
/// viewer, which is the useful half of the app.
///
/// **A Linux desktop with nothing drawing a tray is not one of them either, and
/// it is not an answer that holds for the session.** What draws the icon there
/// is a program — part of a panel, or a shell extension — and this app can be
/// started before it: a session that launches Verkstead at login, which is what
/// the Launch on Startup box arranges, is a race this app loses about as often
/// as it wins. The icon is offered again when one turns up, which is
/// [`Raised::WhenATrayArrives`]. macOS and Windows have no such question — a
/// menu bar and a notification area are the session's own and are always there.
///
/// An icon that *was* taken and whose tray then went away is nobody's problem
/// here: the item stays published, and the backend registers with the next tray
/// to arrive on its own account.
///
/// `listen` and `key` are where Open sends the browser: the same login link
/// that was opened at startup, built again at each press rather than captured
/// once, so that a key which has been re-issued since is the one the next press
/// hands over. `logging` is what
/// View Logs opens, or what it says where this machine had nowhere to keep a log
/// file. `startup` is the registration the Launch on Startup box is drawn from
/// and written to.
fn raise(
    listen: SocketAddr,
    key: &WorkbenchKey,
    logging: &logs::Kept,
    startup: &startup::Startup,
) -> Raised {
    if !verkstead_server::display::there_is_one() {
        tracing::info!("there is no display here, so Verkstead is running as the server alone");
        return Raised::Nowhere;
    }

    if let Err(error) = toolkit::start() {
        tracing::warn!("the desktop toolkit would not start, so there is no tray icon: {error:#}");
        return Raised::Nowhere;
    }

    // Said where the refusal above is said, and for the reader who is owed the
    // line saying nothing went wrong. It is also the whole of what the release
    // workflow's AppImage leg reads: on Linux a toolkit that started is the
    // bundled GTK having been found, loaded and initialised, which is the one
    // claim about the bundle that a run with a screen can make for itself. The
    // tray is no claim of the bundle's any more — it is spoken onto the session
    // bus in Rust, and `crates/desktop/tests/tray.rs` is where that is asserted.
    // See `.github/workflows/release.yml` and `.github/workflows/ci.yml`.
    tracing::info!("the desktop toolkit is up");

    let key = key.clone();
    let logging = logging.clone();
    // Two of them, because the icon can be offered twice: what the handler
    // below does with the registration is the same work whichever offer was
    // taken, and what the box is ticked to has to be read again at the second
    // one — a tick is what the registration said as the menu was made, and a
    // menu made later is a later reading.
    let ticking = startup.clone();
    let startup = startup.clone();

    // **Behind an `Arc` rather than handed straight over**, for the same
    // reason: where nothing is drawing a tray yet the icon is offered again
    // from inside the loop, and what is offered with it is this same handler
    // rather than a second one built out of second copies of everything it
    // holds.
    let chosen: Arc<dyn Fn(tray::Chosen) + Send + Sync> = Arc::new(move |chosen| match chosen {
        tray::Chosen::Open => {
            // The link rather than the bare address, and built here rather than
            // captured: what makes a browser the human's is the key on it, and a
            // press months after the browser forgot the cookie has to be as good
            // as the first one.
            let viewer = login_link(listen, &key);
            let workbench = workbench_address(listen);

            // Said and carried on, for the reason the open at startup is: the
            // viewer is reachable from every browser on the tailnet, and a
            // desktop that would not open one is no reason to stop serving them.
            // Said as the address rather than as the link, for the reason the
            // open at startup is too — a press that failed is a line in the
            // very file this install keeps the key out of.
            if let Err(error) = opener::url_reported_as(&viewer, &workbench) {
                tracing::warn!(%workbench, "{error:#}");
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

    // **Asked before the icon is offered, rather than read off a refusal.**
    // There is nothing to put an icon in until something owns the name a tray
    // owns, and the backend answers that with an error among its others — so
    // the app asks the bus itself, and what comes back is the difference
    // between a tray that is not there yet and a tray that would not have it.
    // See [`panel`].
    #[cfg(target_os = "linux")]
    if !panel::is_there() {
        // The same offer, made from inside the loop when there is somewhere for
        // it to go. What it hands `panel` is the hop rather than the work: every
        // pick the menu leads to is the loop thread's, and so is the menu — see
        // [`toolkit::later`], and [`tray::keep`] for who holds an icon raised
        // where there is no caller left to hold it.
        let offering = move || {
            let ticked = ticking.possible().then(|| ticking.on());

            if let Some(icon) = said(tray::show(ticked, move |picked| chosen(picked))) {
                tray::keep(icon);
            }
        };

        return match panel::when_one_arrives(move || toolkit::later(offering)) {
            Ok(()) => {
                tracing::info!(
                    "nothing is drawing a tray on this desktop yet, so the icon goes up when \
                     something is"
                );
                Raised::WhenATrayArrives
            }
            // Which is the bus itself rather than the tray on it: no session bus
            // to publish an icon on, and none coming.
            Err(error) => {
                tracing::warn!("putting the icon in the tray: {error:#}");
                Raised::Nowhere
            }
        };
    }

    // What the box is ticked to as the menu is made, which is what the
    // registration says right now — or nothing to tick, on a machine with
    // nowhere to keep one.
    let ticked = ticking.possible().then(|| ticking.on());

    said(tray::show(ticked, move |picked| chosen(picked))).map_or(Raised::Nowhere, Raised::Now)
}

/// Say what came of offering the icon, and hand back whatever there is to hold.
///
/// **The line on the way through is as much the point as the icon.** The log is
/// the only mark any of this leaves, and a reader who has been told what *would*
/// have gone wrong is owed the line saying nothing did. It is also what the
/// release workflow's macOS and Windows legs read to know that the app they have
/// just built can raise a tray at all — a headless run reaches neither the
/// toolkit nor the item, so this line is the whole of what tells the two apart.
/// See `.github/workflows/release.yml`.
fn said(raised: Result<TrayIcon>) -> Option<TrayIcon> {
    match raised {
        Ok(icon) => {
            tracing::info!("Verkstead is in the tray");
            Some(icon)
        }
        Err(error) => {
            tracing::warn!("{error:#}");
            None
        }
    }
}

/// The address could not be taken, which is the one failure the app draws
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
