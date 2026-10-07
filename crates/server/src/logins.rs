//! A Profile's login, run by Verkstead rather than typed into a session — see
//! ADR-0022.
//!
//! **A separate process on the account, not keystrokes into a session.** A
//! session whose login has gone needs `/login` typed at its terminal and a code
//! pasted back into it, which is exactly what the browser terminal does badly.
//! So the login is `claude auth login --claudeai`, run on its own in the
//! Profile's sandbox with the Profile's Built Root — the root a session gets —
//! so that what it writes reaches the account by the write-back a session's
//! ending already does. See [`crate::sandbox::Sandbox::for_login`].
//!
//! **Over plain pipes, not a terminal.** What the harness prints and reads is
//! two lines: an address, and a prompt for the code that page hands back. Both
//! are read off its standard output and the code is written to its standard
//! input, and the modal draws the address as a link and takes the code in a
//! box. Nothing about it needs a screen.
//!
//! **On Windows as the session account**, the way a session runs there — so
//! started by Verkstead's own logon rather than the standard library's spawn,
//! with its input held open for the code (see [`Harness`]), and behind a
//! boundary of its own that is taken back when the login ends (see
//! [`crate::sandbox::login_taken_back`]).
//!
//! **One login per Profile.** A second device opening the modal joins the one
//! already running and is shown the same address, because a second process
//! would print a second address and only one of the two codes would ever be
//! the one that counted. The login is killed when the last device watching it
//! closes the modal, or when [`LIMIT`] has passed — a modal on a phone that
//! went in a pocket says nothing as it goes.
//!
//! **A wrong code starts again by itself.** Claude Code 2.1.283 refuses a code
//! two ways, checked against the real harness with a throwaway HOME. A code
//! with no `#` in it — empty, or cut short — gets [`CUT_SHORT`] on standard
//! error, and the harness goes on waiting on the same address. A code that is
//! shaped right but wrong gets `Login failed: Request failed with status code
//! 400` on standard error, and the harness exits `1`. The first is the modal
//! back at the same address with a word about it; the second is a fresh
//! harness with a fresh address, started once the first has gone, so that
//! there is still only ever one per Profile.
//!
//! **A login that lands resumes what it unblocks**: every run on the Profile
//! stopped as Signed out — see [`crate::signouts::logged_in`].
//!
//! **And a press is checked before it starts anything.** A press that would
//! start a session under a Claude Profile at home here asks `claude auth
//! status` of its account first — see [`asked`]. An account with no login is
//! remembered as signed out, so the Profile reads as broken on its row and the
//! press is refused naming it, before a session comes up with nothing to run
//! on. A login from the card clears it; the press is then the human's to make
//! again, there being no run to resume.
//!
//! Claude only, and only a Profile at home on this device: a mirror's account
//! is a copy, and a login written into a copy is one the next refresh from its
//! home device writes over.

use std::collections::{HashMap, HashSet};
use std::io;
use std::process::{ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};
use verkstead_render::LoginState;
use verkstead_schema::Nudge;

use crate::AppState;
use crate::platform::Platform;
use crate::sandbox::Rendering;
use crate::sessions::Agents;
use crate::store;

/// How long a login is left waiting for its code before it is killed.
///
/// Long enough to find the page on another device, log in there and copy the
/// code back; short enough that a modal nobody closed does not leave a process
/// holding the Profile's login for the rest of the day.
pub(crate) const LIMIT: Duration = Duration::from_secs(10 * 60);

/// What the harness prints in front of the address to log in at.
///
/// Read off Claude Code 2.1.283, which prints it on a line of its own —
/// `If the browser didn't open, visit: <url>` — and then the prompt for the
/// code, `Paste code here if prompted > `, with no line ending after it.
const VISIT: &str = "visit: ";

/// What the harness says on standard error to a code with no `#` in it, before
/// it goes on waiting for another — see the module note.
const CUT_SHORT: &str = "Invalid code";

/// What the modal says when the harness asked again for a code cut short.
const CUT_SHORT_SAID: &str =
    "That is not the whole code. Copy all of it from the page and paste it again.";

/// What the modal says when the harness refused a code and a fresh login has
/// taken its place.
const REFUSED_SAID: &str = "Claude did not accept that code. Open the new address below, \
                            log in again, and paste the code it gives you.";

/// The words of the harness's line a login runs.
const LOGGING_IN: &[&str] = &["auth", "login", "--claudeai"];

/// And the words that ask whether the account is logged in, which Claude Code
/// 2.1.283 answers by its exit status: `0` logged in, `1` not. An API key, in
/// the environment or from an `apiKeyHelper`, reads as logged in.
const STATUS: &[&str] = &["auth", "status"];

/// How long that question is waited on before a press goes ahead without the
/// answer. It reads files and answers at once; one that hangs is a harness
/// in trouble that the session will show more plainly than a refusal could.
const STATUS_LIMIT: Duration = Duration::from_secs(20);

/// What the environment's `BROWSER` is inside a login, so that the harness
/// opens nothing on the server: a program that does nothing and succeeds.
/// Found on `PATH` rather than at `/bin/true`, which NixOS does not have.
///
/// **The same word on Windows, where it is usually nothing at all.** Claude
/// Code 2.1.283 runs `BROWSER` with the address where it is set, and otherwise
/// `rundll32 url,OpenURL` — which would open a browser on the server's own
/// screen as the session account. A machine with Git's `usr\bin` on its `PATH`
/// has a `true` there; one without fails to start it, which the harness takes
/// as a browser that did not open, and it prints the address either way.
const NO_BROWSER: &str = "true";

/// Every Profile's login that is running, or has ended with a device still
/// looking at how.
///
/// Shared across every clone of the server's state, which is what makes a
/// second device's open a join rather than a second login.
#[derive(Clone, Default)]
pub(crate) struct Logins {
    held: Arc<Mutex<HashMap<i64, Login>>>,

    /// What tells an entry from the one that replaced it, so that a run that
    /// was killed does not write its ending over its successor's.
    runs: Arc<AtomicU64>,

    /// The Profiles whose account `claude auth status` last found with no
    /// login — see [`asked`]. Held here rather than in the store: it is a
    /// reading of the account, and a restart asks again at the next press.
    signed_out: Arc<Mutex<HashSet<i64>>>,
}

struct Login {
    run: u64,
    state: LoginState,
    viewers: HashSet<String>,

    /// The address the harness now running printed, once it has.
    url: Option<String>,

    /// Where a code goes to be written to the harness. `None` once it has
    /// ended.
    codes: Option<mpsc::UnboundedSender<String>>,

    /// What kills the harness. `None` once it has ended.
    stop: Option<oneshot::Sender<()>>,
}

impl Login {
    fn ended(&self) -> bool {
        self.stop.is_none()
    }
}

/// Why a login was not started or a code not taken.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// There is no login running for that Profile.
    NotRunning,

    /// The login is not waiting for a code: it has not printed its address
    /// yet, it is checking one, or it has ended.
    NotWaiting,
}

impl Logins {
    /// `viewer` opening the modal on `profile`: the login already running for
    /// it, joined, or a new one started.
    ///
    /// A login that has ended is started again: a device opening the modal
    /// wants to log in, whatever the last one came to.
    pub(crate) fn opened(
        &self,
        state: AppState,
        agents: Arc<Agents>,
        profile: store::Profile,
        viewer: String,
    ) -> LoginState {
        let mut held = self
            .held
            .lock()
            .expect("the login register is not poisoned");

        if let Some(login) = held.get_mut(&profile.id)
            && !login.ended()
        {
            login.viewers.insert(viewer);
            return login.state.clone();
        }

        let mut viewers = held
            .remove(&profile.id)
            .map(|ended| ended.viewers)
            .unwrap_or_default();
        viewers.insert(viewer);

        let run = self.runs.fetch_add(1, Ordering::Relaxed);
        let (codes, coded) = mpsc::unbounded_channel();
        let (stop, stopped) = oneshot::channel();

        held.insert(
            profile.id,
            Login {
                run,
                state: LoginState::Starting,
                viewers,
                url: None,
                codes: Some(codes),
                stop: Some(stop),
            },
        );

        tokio::spawn(self.clone().running(
            Run {
                state,
                agents,
                profile,
                run,
            },
            coded,
            stopped,
        ));

        LoginState::Starting
    }

    /// Where `profile`'s login has got to, or `None` where there is none.
    pub(crate) fn reading(&self, profile: i64) -> Option<LoginState> {
        let held = self
            .held
            .lock()
            .expect("the login register is not poisoned");

        held.get(&profile).map(|login| login.state.clone())
    }

    /// The code the login page handed back, for the login waiting for it.
    pub(crate) fn coded(&self, profile: i64, code: &str) -> Result<LoginState, Refusal> {
        let mut held = self
            .held
            .lock()
            .expect("the login register is not poisoned");
        let login = held.get_mut(&profile).ok_or(Refusal::NotRunning)?;

        let (LoginState::Waiting { .. }, Some(codes)) = (&login.state, &login.codes) else {
            return Err(Refusal::NotWaiting);
        };

        codes
            .send(code.trim().to_owned())
            .map_err(|_| Refusal::NotWaiting)?;
        login.state = LoginState::Checking;

        Ok(login.state.clone())
    }

    /// The Profiles found signed out, for the rows read off them — see
    /// [`crate::profiles`].
    pub(crate) fn signed_out(&self) -> HashSet<i64> {
        self.signed_out
            .lock()
            .expect("the signed-out register is not poisoned")
            .clone()
    }

    /// Put down what the account of `profile` was found to be, and say so to
    /// every device where the Profile's row reads differently for it.
    fn found(&self, state: &AppState, profile: i64, logged_in: bool) {
        let moved = {
            let mut signed_out = self
                .signed_out
                .lock()
                .expect("the signed-out register is not poisoned");

            match logged_in {
                true => signed_out.remove(&profile),
                false => signed_out.insert(profile),
            }
        };

        if moved {
            state.nudges.announce_here(Nudge::Profiles);
        }
    }

    /// `viewer` closing the modal on `profile`. The last one to close it ends
    /// the login, killing the harness if it is still running.
    pub(crate) fn closed(&self, profile: i64, viewer: &str) {
        let mut held = self
            .held
            .lock()
            .expect("the login register is not poisoned");

        let Some(login) = held.get_mut(&profile) else {
            return;
        };

        login.viewers.remove(viewer);

        if login.viewers.is_empty()
            && let Some(login) = held.remove(&profile)
            && let Some(stop) = login.stop
        {
            let _ = stop.send(());
        }
    }

    /// `run`'s login has got to `state`, where it is still the Profile's.
    fn moved(&self, run: &Run, state: LoginState) {
        {
            let mut held = self
                .held
                .lock()
                .expect("the login register is not poisoned");

            let Some(login) = held.get_mut(&run.profile.id) else {
                return;
            };

            if login.run != run.run {
                return;
            }

            if let LoginState::Waiting { url, .. } = &state {
                login.url = Some(url.clone());
            }

            login.state = state;
        }

        run.state.nudges.announce_here(Nudge::Login {
            profile: run.profile.id,
        });
    }

    /// Whether `run`'s login is checking a code it was handed.
    fn checking(&self, run: &Run) -> bool {
        let held = self
            .held
            .lock()
            .expect("the login register is not poisoned");

        held.get(&run.profile.id)
            .is_some_and(|login| login.run == run.run && login.state == LoginState::Checking)
    }

    /// The harness asked again for the code it was handed, which was cut
    /// short: back to waiting on the address it already printed.
    fn cut_short(&self, run: &Run) {
        let url = {
            let held = self
                .held
                .lock()
                .expect("the login register is not poisoned");

            held.get(&run.profile.id)
                .filter(|login| login.run == run.run && login.state == LoginState::Checking)
                .and_then(|login| login.url.clone())
        };

        if let Some(url) = url {
            self.moved(
                run,
                LoginState::Waiting {
                    url,
                    refused: Some(CUT_SHORT_SAID.to_owned()),
                },
            );
        }
    }

    /// `run`'s login has ended at `state`: nothing more is written to it and
    /// nothing kills it, and it is kept for the devices still looking.
    fn ended(&self, run: &Run, state: LoginState) {
        {
            let mut held = self
                .held
                .lock()
                .expect("the login register is not poisoned");

            if let Some(login) = held.get_mut(&run.profile.id)
                && login.run == run.run
            {
                login.codes = None;
                login.stop = None;
            }
        }

        self.moved(run, state);
    }

    /// The harness, from its start to its end.
    async fn running(
        self,
        run: Run,
        mut coded: mpsc::UnboundedReceiver<String>,
        mut stopped: oneshot::Receiver<()>,
    ) {
        let mut refused = None;

        let ending = loop {
            match self
                .logging_in(&run, &mut coded, &mut stopped, refused.take())
                .await
            {
                // A fresh harness, now the one that refused the code has gone.
                // The modal reads as checking until it prints its address.
                Ending::Refused => refused = Some(REFUSED_SAID.to_owned()),
                ending => break ending,
            }
        };

        let state = match ending {
            Ending::Stopped | Ending::Refused => None,
            Ending::Failed(reason) => Some(LoginState::Failed { reason }),
            Ending::Exited => Some(
                match status(run.agents.clone(), run.profile.clone()).await {
                    Some(true) => LoginState::LoggedIn,
                    _ => LoginState::Failed {
                        reason: "Claude finished, but the account still reads as logged out. \
                                 Try again."
                            .to_owned(),
                    },
                },
            ),
        };

        // The boundary every harness of this login ran behind, which is held
        // from the first of them to the question asked after the last — see
        // [`crate::sandbox::login_taken_back`]. Taken back before the modal is
        // told, so that a device opening it again starts behind a fresh one.
        let agents = run.agents.clone();
        let profile = run.profile.id;
        let _ = tokio::task::spawn_blocking(move || agents.login_over(profile)).await;

        let Some(state) = state else {
            return;
        };

        if state == LoginState::LoggedIn {
            // The Profile's row says whether its account has a login, and now
            // it has — and it no longer reads as signed out.
            self.signed_out
                .lock()
                .expect("the signed-out register is not poisoned")
                .remove(&run.profile.id);
            run.state.nudges.announce_here(Nudge::Profiles);

            tokio::spawn(crate::signouts::logged_in(
                run.state.clone(),
                run.profile.id,
            ));
        }

        self.ended(&run, state);
    }

    async fn logging_in(
        &self,
        run: &Run,
        coded: &mut mpsc::UnboundedReceiver<String>,
        stopped: &mut oneshot::Receiver<()>,
        refused: Option<String>,
    ) -> Ending {
        let agents = run.agents.clone();
        let profile = run.profile.clone();

        let built = tokio::task::spawn_blocking(move || {
            let (sandbox, argv) = agents.logging_in(&profile, LOGGING_IN)?;
            let (mut rendering, closing) = sandbox.command(&argv).ok()?;
            rendering.set("BROWSER", NO_BROWSER);

            Some((Harness::started(&rendering), closing))
        })
        .await
        .ok()
        .flatten();

        let Some((started, closing)) = built else {
            return Ending::Failed(
                "Verkstead could not build a sandbox to log in from. The server's log says why."
                    .to_owned(),
            );
        };

        let mut child = match started {
            Ok(child) => child,
            Err(error) => {
                tracing::error!(
                    profile_id = run.profile.id,
                    error = ?error,
                    "a Profile's login could not be started"
                );
                closed(closing).await;
                return Ending::Failed(format!("Claude could not be started: {error}"));
            }
        };

        let mut stdin = child.typing.take();

        // The address, off the harness's output as it prints it. Read on a
        // task of its own, because what is printed after it is a prompt with
        // no line ending, which a read waiting for one would wait on for ever.
        if let Some(stdout) = child.printed.take() {
            let logins = self.clone();
            let shown = run.again();

            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(url) = address_in(&line) {
                        logins.moved(
                            &shown,
                            LoginState::Waiting {
                                url,
                                refused: refused.clone(),
                            },
                        );
                    }
                }
            });
        }

        // And what it says on standard error, kept for the reason a failure
        // gives — and watched for a code it is asking for again.
        let said = Arc::new(Mutex::new(Vec::<String>::new()));

        if let Some(stderr) = child.complained.take() {
            let said = said.clone();
            let logins = self.clone();
            let shown = run.again();

            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let line = line.trim().to_owned();

                    if line.starts_with(CUT_SHORT) {
                        logins.cut_short(&shown);
                    }

                    if !line.is_empty() {
                        said.lock().expect("not poisoned").push(line);
                    }
                }
            });
        }

        let limit = tokio::time::sleep(run.agents.login_limit());
        tokio::pin!(limit);

        let ending = loop {
            tokio::select! {
                exited = child.process.wait() => break match exited {
                    Ok(status) if status.success() => Ending::Exited,
                    // Exiting badly with a code in hand is the harness
                    // refusing it — see the module note.
                    Ok(status) if self.checking(run) => {
                        tracing::info!(
                            profile_id = run.profile.id,
                            said = %failure(status, &said),
                            "a login code was refused; starting a fresh login"
                        );
                        Ending::Refused
                    }
                    Ok(status) => Ending::Failed(failure(status, &said)),
                    Err(error) => Ending::Failed(format!("Claude could not be waited on: {error}")),
                },

                Some(code) = coded.recv() => {
                    if let Some(input) = stdin.as_mut() {
                        let written = async {
                            input.write_all(format!("{code}\n").as_bytes()).await?;
                            input.flush().await
                        };

                        if let Err(error) = written.await {
                            tracing::warn!(
                                profile_id = run.profile.id,
                                error = ?error,
                                "a login code could not be handed to the harness"
                            );
                        }
                    }
                }

                _ = &mut *stopped => {
                    child.process.kill().await;
                    break Ending::Stopped;
                }

                () = &mut limit => {
                    child.process.kill().await;
                    break Ending::Failed(
                        "The login ran out of time. Open it again to start over.".to_owned(),
                    );
                }
            }
        };

        drop(stdin);
        closed(closing).await;

        ending
    }
}

/// One login's run: what it was started with, and which run it is.
struct Run {
    state: AppState,
    agents: Arc<Agents>,
    profile: store::Profile,
    run: u64,
}

impl Run {
    /// The same run, for a task of its own to report on.
    fn again(&self) -> Run {
        Run {
            state: self.state.clone(),
            agents: self.agents.clone(),
            profile: self.profile.clone(),
            run: self.run,
        }
    }
}

enum Ending {
    /// The harness exited cleanly, which is the half of success it can say.
    Exited,

    /// It ended without a login, for the reason given.
    Failed(String),

    /// It refused the code it was handed and exited, and a fresh one is due.
    Refused,

    /// The last device closed the modal, and nobody is left to tell.
    Stopped,
}

/// The harness, started over three pipes, whichever of the two ways it was
/// started — what a `tokio::process::Child` is where the standard library
/// started it, and where it did not.
///
/// **Because on Windows it cannot.** A login there runs as the session account
/// (ADR-0014), which is `CreateProcessWithLogonW` — see
/// [`crate::sandbox::starting::held_open`]. What comes back from that is three
/// pipes and a process handle, and this is them made into what the login reads
/// and writes on the runtime: each pipe a file whose reads and writes go to the
/// runtime's blocking threads, and the process waited for on one of its own.
///
/// Killed as it is dropped, on both platforms.
struct Harness {
    /// Its standard input, where a code is written.
    typing: Option<Box<dyn AsyncWrite + Send + Unpin>>,

    /// Its standard output, where the address is printed.
    printed: Option<Box<dyn AsyncRead + Send + Unpin>>,

    /// And its standard error, where a refusal is said.
    complained: Option<Box<dyn AsyncRead + Send + Unpin>>,

    process: Process,
}

impl Harness {
    /// `rendering` started. Blocks on Windows, where the logon is made.
    fn started(rendering: &Rendering) -> io::Result<Harness> {
        #[cfg(windows)]
        if let Some(logon) = rendering.account() {
            let held = crate::sandbox::starting::held_open(rendering, logon)?;
            let running = Arc::new(held.running);

            let (exit, exited) = oneshot::channel();
            let waiting = running.clone();

            tokio::task::spawn_blocking(move || {
                let _ = exit.send(waiting.wait());
            });

            return Ok(Harness {
                typing: Some(Box::new(tokio::fs::File::from_std(held.typing))),
                printed: Some(Box::new(tokio::fs::File::from_std(held.printed))),
                complained: Some(Box::new(tokio::fs::File::from_std(held.complained))),
                process: Process::AsTheAccount { running, exited },
            });
        }

        let mut child = tokio::process::Command::from(std::process::Command::try_from(rendering)?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        Ok(Harness {
            typing: child
                .stdin
                .take()
                .map(|stdin| Box::new(stdin) as Box<dyn AsyncWrite + Send + Unpin>),
            printed: child
                .stdout
                .take()
                .map(|stdout| Box::new(stdout) as Box<dyn AsyncRead + Send + Unpin>),
            complained: child
                .stderr
                .take()
                .map(|stderr| Box::new(stderr) as Box<dyn AsyncRead + Send + Unpin>),
            process: Process::Ordinarily(Box::new(child)),
        })
    }
}

/// The process of a [`Harness`].
enum Process {
    /// Started by the standard library, which is both of the platforms whose
    /// boundary is a wrapper in front of the process.
    Ordinarily(Box<tokio::process::Child>),

    /// And started as the session account, with a blocking thread waiting for
    /// it to end and saying so on `exited`.
    #[cfg(windows)]
    AsTheAccount {
        running: Arc<crate::sandbox::starting::Running>,
        exited: oneshot::Receiver<io::Result<ExitStatus>>,
    },
}

impl Process {
    /// How it ended, once it has. Safe to give up on before it answers, and
    /// asked again; not asked again once it has.
    async fn wait(&mut self) -> io::Result<ExitStatus> {
        match self {
            Process::Ordinarily(child) => child.wait().await,

            #[cfg(windows)]
            Process::AsTheAccount { exited, .. } => exited
                .await
                .unwrap_or_else(|_| Err(io::Error::other("nothing was left waiting for it"))),
        }
    }

    /// Ended. One that has ended already is not an error.
    async fn kill(&mut self) {
        match self {
            Process::Ordinarily(child) => {
                let _ = child.kill().await;
            }

            #[cfg(windows)]
            Process::AsTheAccount { running, .. } => {
                let _ = running.kill();
            }
        }
    }
}

/// What `kill_on_drop` is for the one the standard library started.
#[cfg(windows)]
impl Drop for Process {
    fn drop(&mut self) {
        if let Process::AsTheAccount { running, .. } = self {
            let _ = running.kill();
        }
    }
}

/// The address in a line the harness printed, where it is the line that
/// carries one — see [`VISIT`].
fn address_in(line: &str) -> Option<String> {
    let (_, url) = line.split_once(VISIT)?;
    let url = url.trim();

    url.starts_with("https://").then(|| url.to_owned())
}

/// Ask each of `profiles` that is a Claude account at home here whether it is
/// logged in, before a press starts a session under it, and remember the
/// answer for its row — see the module note.
///
/// **Nothing is refused here.** The answer is put down where every reading of
/// the Profile finds it, and the press reads its Profiles after this, so it is
/// refused by the rule every broken Profile is refused by.
///
/// No answer leaves the reading as it was: a harness that is not on this
/// machine, a sandbox that could not be built, and a harness that did not
/// answer in time. Each of those goes ahead, and a
/// session that then finds itself signed out stops the way any signed-out
/// session does — see [`crate::signouts`].
///
/// **And nothing is asked on Windows.** The question runs behind a boundary
/// there, as the login does, and a boundary is access-control entries written
/// on the account's directories — the slow half of a session's start, paid
/// again in front of every press. A press goes ahead unasked, and a session
/// that finds itself signed out stops the same way.
pub(crate) async fn asked<'a>(
    state: &AppState,
    profiles: impl IntoIterator<Item = &'a store::Profile>,
) {
    let Some(agents) = state.sessions.agents() else {
        return;
    };

    if !agents.checks_logins() || Platform::HERE == Platform::Windows {
        return;
    }

    let mut seen = HashSet::new();

    for profile in profiles {
        if profile.mirror.is_some()
            || profile.agent_type() != store::AgentType::Claude
            || !seen.insert(profile.id)
        {
            continue;
        }

        // A harness that is not on this machine is no answer either, rather
        // than an account signed out: the sandbox would fail to run it, and
        // that failure would read as `auth status` saying no. What is wrong
        // there is the machine, and the dependencies step is what says so.
        let machine = state.onboarding.machine().clone();
        let program = agents.harness(store::AgentType::Claude);

        if !tokio::task::spawn_blocking(move || machine.runs(&program))
            .await
            .unwrap_or(true)
        {
            continue;
        }

        if let Some(logged_in) = status(agents.clone(), profile.clone()).await {
            state.logins.found(state, profile.id, logged_in);
        }
    }
}

/// Whether the account of `profile` reads as logged in, asked of the harness
/// in a fresh root — which is the account's login as the next session would be
/// given it. `None` where nothing could answer: see [`asked`].
async fn status(agents: Arc<Agents>, profile: store::Profile) -> Option<bool> {
    let built = tokio::task::spawn_blocking(move || {
        let (sandbox, argv) = agents.logging_in(&profile, STATUS)?;
        let (rendering, closing) = sandbox.command(&argv).ok()?;

        Some((Harness::started(&rendering), closing))
    })
    .await
    .ok()
    .flatten();

    let (started, closing) = built?;

    let status = match started {
        Ok(mut asking) => {
            // Nothing to type, and what it says is read and thrown away
            // rather than left unread: a harness writing into a pipe nobody
            // reads is one that stops when the pipe is full.
            drop(asking.typing.take());

            for mut said in [asking.printed.take(), asking.complained.take()]
                .into_iter()
                .flatten()
            {
                tokio::spawn(async move {
                    let _ = tokio::io::copy(&mut said, &mut tokio::io::sink()).await;
                });
            }

            // A harness that did not answer in time is killed as it is
            // dropped — see [`Harness`].
            match tokio::time::timeout(STATUS_LIMIT, asking.process.wait()).await {
                Ok(Ok(status)) => Some(status.success()),
                Ok(Err(_)) | Err(_) => None,
            }
        }
        Err(_) => None,
    };

    closed(closing).await;

    status
}

/// What is left to see to once the harness has gone — the write-back that
/// carries the login to the account. Blocks, so off the runtime.
async fn closed(closing: crate::sandbox::Closing) {
    let _ = tokio::task::spawn_blocking(move || closing.close()).await;
}

/// The reason a harness that exited badly gives: the last thing it said on
/// standard error, or its exit status where it said nothing.
fn failure(status: std::process::ExitStatus, said: &Mutex<Vec<String>>) -> String {
    let said = said.lock().expect("not poisoned");

    match said.last() {
        Some(line) => format!("Claude stopped without logging in: {line}"),
        None => format!("Claude stopped without logging in ({status})."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_is_read_off_the_line_that_carries_it() {
        assert_eq!(
            address_in(
                "If the browser didn't open, visit: https://claude.com/cai/oauth/authorize?code=true"
            ),
            Some("https://claude.com/cai/oauth/authorize?code=true".to_owned())
        );
    }

    #[test]
    fn a_line_without_an_address_carries_none() {
        assert_eq!(address_in("Opening browser to sign in…"), None);
        assert_eq!(address_in("Paste code here if prompted > "), None);
        assert_eq!(address_in("visit: somewhere"), None);
    }
}
