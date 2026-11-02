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
//! start a session under a Claude Profile bound here asks `claude auth
//! status` of its account first — see [`asked`]. An account with no login is
//! remembered as signed out, so the Profile reads as broken on its row and the
//! press is refused naming it, before a session comes up with nothing to run
//! on. A login from the card clears it; the press is then the human's to make
//! again, there being no run to resume.
//!
//! **And the Antigravity CLI the same way**, with three differences read off
//! agy 1.4.12 — see [`AGY`]. Its login is behind any command that needs one,
//! so what runs is `agy -p /help`, which signs in and then prints the commands
//! without asking a model anything. It offers the address only to a standard
//! input that is a terminal, so that is the one stream it is given a terminal
//! for — and on Windows, where it reads the code off its console whatever its
//! standard input is, the whole of it runs on one (see `Harness::on_a_console`).
//! And it waits a minute for the code, then gives up; while a device is still
//! looking, a fresh address takes the place of the one that ran out.
//!
//! Claude and the Antigravity CLI only, and only a Profile bound on this device:
//! one not bound here runs from a copy of its account fetched before every
//! launch, and a login written into the copy is one the next fetch writes over —
//! its login is made on a device it is bound on.

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

/// How one harness's login goes: what runs it, what it prints, and what it
/// says back — the whole of what tells one harness's login from another's.
struct Speaking {
    /// What the modal and its refusals call the harness.
    name: &'static str,

    /// The words of the harness's line a login runs.
    logging_in: &'static [&'static str],

    /// And the words that ask whether the account is logged in.
    status: &'static [&'static str],

    /// What a failing answer to those says where the account is signed out —
    /// or `None` where the exit status is the whole answer. Where it is said,
    /// a failure that does not say it is no answer, rather than a sign-out.
    signed_out: Option<&'static str>,

    /// What the harness prints in front of the address on the line that
    /// carries it — or `None` where the address is a line of its own.
    visit: Option<&'static str>,

    /// What the harness says to a code cut short, before it goes on waiting
    /// on the same address — or `None` where it says nothing of the kind.
    cut_short: Option<&'static str>,

    /// What it says as it gives up waiting for a code — or `None` where it
    /// waits for as long as it is left.
    expired: Option<&'static str>,

    /// Whether the code is read off a terminal rather than a pipe.
    typed_on_a_terminal: bool,
}

/// Claude Code 2.1.283, checked against the real harness with a throwaway HOME.
///
/// It prints `If the browser didn't open, visit: <url>` on a line of its own,
/// and then the prompt for the code, `Paste code here if prompted > `, with no
/// line ending after it. A code with no `#` in it gets `Invalid code` on
/// standard error, and the harness waits on — see the module note. `auth
/// status` answers by its exit status: `0` logged in, `1` not. An API key, in
/// the environment or from an `apiKeyHelper`, reads as logged in.
const CLAUDE: Speaking = Speaking {
    name: "Claude",
    logging_in: &["auth", "login", "--claudeai"],
    status: &["auth", "status"],
    signed_out: None,
    visit: Some("visit: "),
    cut_short: Some("Invalid code"),
    expired: None,
    typed_on_a_terminal: false,
};

/// The Antigravity CLI 1.4.12, checked against the real binary with a
/// throwaway HOME and its token exchange answered by a stand-in for Google.
///
/// **`-p /help` is the login.** agy has no command that only signs in: every
/// one that needs the account signs it in first. `/help` in print mode is
/// answered by agy itself, so what follows the sign-in is a list of commands
/// and no model is asked anything.
///
/// **On standard error, and only to a terminal.** agy prints
/// `Authentication required. Please visit the URL to log in:`, the address
/// indented on the next line, then `Waiting for authentication (timeout
/// 60s)...` and `Or, paste the authorization code here and press Enter:` — all
/// on standard error — and reads the code off standard input. Only where that
/// is a terminal, though: given a pipe it says `Error: authentication
/// required. Run 'agy' to log in, then retry.` and exits `1`.
///
/// **A wrong code and a cut one are the same to it**: Google refuses either,
/// and agy says `Error: authentication failed: token exchange failed: …` and
/// exits `1`. An empty line it ignores, which the modal never sends. And after
/// a minute with no code it says `Error: authentication timed out.` and exits
/// `1`.
///
/// **`models` asks the account**, and signed out says `Error: Please sign in
/// to view available models. Launch the CLI without arguments to sign in.` and
/// exits `1`. It fails other ways too — no network, Google not answering — so
/// only that sentence reads as signed out.
const AGY: Speaking = Speaking {
    name: "Antigravity",
    logging_in: &["-p", "/help"],
    status: &["models"],
    signed_out: Some("Please sign in"),
    visit: None,
    cut_short: None,
    expired: Some("Error: authentication timed out."),
    typed_on_a_terminal: true,
};

/// How a login of `agent_type` goes, or `None` for a harness whose login
/// Verkstead does not run.
fn speaking(agent_type: store::AgentType) -> Option<&'static Speaking> {
    match agent_type {
        store::AgentType::Claude => Some(&CLAUDE),
        store::AgentType::Agy => Some(&AGY),
        store::AgentType::Codex | store::AgentType::Grok | store::AgentType::OpenCode => None,
    }
}

/// Whether Verkstead runs the login of a Profile of `agent_type` — see
/// [`crate::ui`], which refuses to open one for any other.
pub(crate) fn logs_in(agent_type: store::AgentType) -> bool {
    speaking(agent_type).is_some()
}

/// What the modal says when the harness asked again for a code cut short.
const CUT_SHORT_SAID: &str =
    "That isn't the whole code. Copy all of it from the page and paste it again.";

/// What the modal says when the harness refused a code and a fresh login has
/// taken its place.
fn refused_said(speaking: &Speaking) -> String {
    format!(
        "{} didn't accept that code. Open the new address below, log in again, and paste \
         the code it gives you.",
        speaking.name
    )
}

/// And when the harness gave up waiting for a code and a fresh login has taken
/// its place.
const EXPIRED_SAID: &str = "That address ran out. Open the new one below, log in, and paste \
                            the code within a minute.";

/// How long that question is waited on before a press goes ahead without the
/// answer. It reads files and answers at once; one that hangs is a harness
/// in trouble that the session will show more plainly than a refusal could.
///
/// **Longer on Windows**, where it is only asked once a login has ended (see
/// [`asked`]), so nothing is held up by it but the modal reading as checking:
/// there the harness is a fresh process of the session account's, and a
/// `powershell.exe` started that way has taken half a minute on a runner
/// before it said a word — past twenty seconds, which read the login that had
/// just landed as one that had not.
const STATUS_LIMIT: Duration = match Platform::HERE {
    Platform::Windows => Duration::from_secs(120),
    Platform::Linux | Platform::MacOs => Duration::from_secs(20),
};

/// And how long it is waited on once a login has exited, to say whether the
/// login landed.
///
/// **Longer, because nothing but the modal is waiting on it**, and what an
/// answer that came too late reads as is a login that failed — the human told
/// to start again over a login that is sitting in the account. On Windows this
/// is also the only time the question is asked at all (see [`asked`]), and
/// there it is a logon as the session account and a cold start of the harness
/// behind it, which on a slow machine can outlast [`STATUS_LIMIT`] on its own.
const LANDED_LIMIT: Duration = Duration::from_secs(120);

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
    held: Arc<Mutex<HashMap<String, Login>>>,

    /// What tells an entry from the one that replaced it, so that a run that
    /// was killed does not write its ending over its successor's.
    runs: Arc<AtomicU64>,

    /// The Profiles whose account `claude auth status` last found with no
    /// login — see [`asked`]. Held here rather than in the store: it is a
    /// reading of the account, and a restart asks again at the next press.
    signed_out: Arc<Mutex<HashSet<String>>>,
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
        let Some(speaking) = speaking(profile.agent_type()) else {
            return LoginState::Failed {
                reason: "Verkstead doesn't run this harness's login.".to_owned(),
            };
        };

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
            profile.id.clone(),
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
                speaking,
                run,
            },
            coded,
            stopped,
        ));

        LoginState::Starting
    }

    /// Where `profile`'s login has got to, or `None` where there is none.
    pub(crate) fn reading(&self, profile: &str) -> Option<LoginState> {
        let held = self
            .held
            .lock()
            .expect("the login register is not poisoned");

        held.get(profile).map(|login| login.state.clone())
    }

    /// The code the login page handed back, for the login waiting for it.
    pub(crate) fn coded(&self, profile: &str, code: &str) -> Result<LoginState, Refusal> {
        let mut held = self
            .held
            .lock()
            .expect("the login register is not poisoned");
        let login = held.get_mut(profile).ok_or(Refusal::NotRunning)?;

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
    pub(crate) fn signed_out(&self) -> HashSet<String> {
        self.signed_out
            .lock()
            .expect("the signed-out register is not poisoned")
            .clone()
    }

    /// Put down what the account of `profile` was found to be, and say so to
    /// every device where the Profile's row reads differently for it.
    fn found(&self, state: &AppState, profile: &str, logged_in: bool) {
        let moved = {
            let mut signed_out = self
                .signed_out
                .lock()
                .expect("the signed-out register is not poisoned");

            match logged_in {
                true => signed_out.remove(profile),
                false => signed_out.insert(profile.to_owned()),
            }
        };

        if moved {
            state.nudges.announce_here(Nudge::Profiles);
        }
    }

    /// `viewer` closing the modal on `profile`. The last one to close it ends
    /// the login, killing the harness if it is still running.
    pub(crate) fn closed(&self, profile: &str, viewer: &str) {
        let mut held = self
            .held
            .lock()
            .expect("the login register is not poisoned");

        let Some(login) = held.get_mut(profile) else {
            return;
        };

        login.viewers.remove(viewer);

        if login.viewers.is_empty()
            && let Some(login) = held.remove(profile)
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
            profile: run.profile.id.clone(),
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

        // One limit for every harness of the login rather than one apiece, so
        // that a modal nobody closed is not kept going for ever by a harness
        // that gives up on its own, minute after minute.
        let deadline = tokio::time::Instant::now() + run.agents.login_limit();

        let ending = loop {
            match self
                .logging_in(&run, &mut coded, &mut stopped, refused.take(), deadline)
                .await
            {
                // A fresh harness, now the one that refused the code has gone.
                // The modal reads as checking until it prints its address.
                Ending::Refused => refused = Some(refused_said(run.speaking)),
                // Or now the one whose address ran out has.
                Ending::Expired => refused = Some(EXPIRED_SAID.to_owned()),
                ending => break ending,
            }
        };

        let state = match ending {
            Ending::Stopped | Ending::Refused | Ending::Expired => None,
            Ending::Failed(reason) => Some(LoginState::Failed { reason }),
            Ending::Exited => Some(
                match status(run.agents.clone(), run.profile.clone(), LANDED_LIMIT).await {
                    Some(true) => LoginState::LoggedIn,
                    _ => LoginState::Failed {
                        reason: format!(
                            "{} finished, but the account still reads as logged out. Try again.",
                            run.speaking.name
                        ),
                    },
                },
            ),
        };

        // The boundary every harness of this login ran behind, which is held
        // from the first of them to the question asked after the last — see
        // [`crate::sandbox::login_taken_back`]. Taken back before the modal is
        // told, so that a device opening it again starts behind a fresh one.
        let agents = run.agents.clone();
        let profile = run.profile.id.clone();
        let _ = tokio::task::spawn_blocking(move || agents.login_over(&profile)).await;

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
                run.profile.id.clone(),
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
        deadline: tokio::time::Instant,
    ) -> Ending {
        let speaking = run.speaking;
        let agents = run.agents.clone();
        let profile = run.profile.clone();

        let built = tokio::task::spawn_blocking(move || {
            let (sandbox, argv) = agents.logging_in(&profile, speaking.logging_in)?;
            let (mut rendering, closing) = sandbox.command(&argv).ok()?;
            rendering.set("BROWSER", NO_BROWSER);

            Some((rendering, closing))
        })
        .await
        .ok()
        .flatten();

        let Some((rendering, closing)) = built else {
            return Ending::Failed(
                "Verkstead couldn't build a sandbox to log in from. The server's log says why."
                    .to_owned(),
            );
        };

        let mut child = match Harness::start(rendering, speaking.typed_on_a_terminal).await {
            Ok(child) => child,
            Err(error) => {
                tracing::error!(
                    profile_id = run.profile.id,
                    error = ?error,
                    "a Profile's login could not be started"
                );
                closed(closing).await;
                return Ending::Failed(format!("{} couldn't be started: {error}.", speaking.name));
            }
        };

        let mut stdin = child.typing.take();

        // What it says on standard error, kept for the reason a failure gives.
        let said = Arc::new(Mutex::new(Vec::<String>::new()));

        // The address, off whichever of its two outputs the harness prints it
        // on — Claude's standard output, agy's standard error — and the rest
        // of what it says read for a code it asks for again and an address
        // that has run out. Each read on a task of its own, because what is
        // printed after the address is a prompt with no line ending, which a
        // read waiting for one would wait on for ever.
        let reading: Vec<_> = [
            (child.printed.take(), false),
            (child.complained.take(), true),
        ]
        .into_iter()
        .filter_map(|(output, kept)| Some((output?, kept)))
        .map(|(output, kept)| {
            let said = said.clone();
            let logins = self.clone();
            let shown = run.again();
            let refused = refused.clone();

            tokio::spawn(async move {
                let mut lines = BufReader::new(output).lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(url) = address_in(&line, speaking) {
                        logins.moved(
                            &shown,
                            LoginState::Waiting {
                                url,
                                refused: refused.clone(),
                            },
                        );
                    }

                    let line = line.trim().to_owned();

                    if speaking.cut_short.is_some_and(|cut| line.starts_with(cut)) {
                        logins.cut_short(&shown);
                    }

                    if kept && !line.is_empty() {
                        said.lock().expect("not poisoned").push(line);
                    }
                }
            })
        })
        .collect();

        let limit = tokio::time::sleep_until(deadline);
        tokio::pin!(limit);

        let ending = loop {
            tokio::select! {
                exited = child.process.wait() => {
                    // What it said on its way out read to the end before it is
                    // judged on, which is what says an address ran out. Not
                    // waited on for ever: a process of its own can still be
                    // holding the pipe.
                    for read in reading {
                        let _ = tokio::time::timeout(Duration::from_secs(2), read).await;
                    }

                    break match exited {
                        Ok(status) if status.success() => Ending::Exited,
                        // Exiting badly with a code in hand is the harness
                        // refusing it — see the module note.
                        Ok(status) if self.checking(run) => {
                            tracing::info!(
                                profile_id = run.profile.id,
                                said = %failure(speaking, status, &said),
                                "a login code was refused; starting a fresh login"
                            );
                            Ending::Refused
                        }
                        // And without one, having said it was done waiting, is
                        // the address running out.
                        Ok(_) if expired(speaking, &said) => Ending::Expired,
                        Ok(status) => Ending::Failed(failure(speaking, status, &said)),
                        Err(error) => Ending::Failed(format!(
                            "{} couldn't be waited on: {error}.",
                            speaking.name
                        )),
                    };
                }

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
    speaking: &'static Speaking,
    run: u64,
}

impl Run {
    /// The same run, for a task of its own to report on.
    fn again(&self) -> Run {
        Run {
            state: self.state.clone(),
            agents: self.agents.clone(),
            profile: self.profile.clone(),
            speaking: self.speaking,
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

    /// It gave up waiting for a code and exited, and a fresh one is due.
    Expired,

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
    /// `rendering` started, from a thread that outlives it.
    ///
    /// **Not from a blocking thread on Linux.** `bwrap --die-with-parent` kills
    /// its child when the thread that started it ends, not the process, and
    /// the runtime ends a blocking thread once it has idled for ten seconds —
    /// so a login started from one died long before a human could hand it a
    /// code. On Windows the logon blocks, and what ends the harness with the
    /// server is a Job rather than the thread — see
    /// [`crate::sandbox::outliving`].
    ///
    /// **With a terminal for its standard input** where `on_a_terminal` says
    /// the harness reads its code off one, and pipes for the other two all the same: the harness
    /// asks only whether its input is a terminal, and the address is still
    /// read off what it prints. On Windows the whole of it is on a console
    /// instead — see [`Harness::on_a_console`].
    async fn start(rendering: Rendering, on_a_terminal: bool) -> io::Result<Harness> {
        match Platform::HERE {
            Platform::Windows => {
                tokio::task::spawn_blocking(move || Harness::started(&rendering, on_a_terminal))
                    .await
                    .map_err(io::Error::other)?
            }
            Platform::Linux | Platform::MacOs => Harness::started(&rendering, on_a_terminal),
        }
    }

    /// `rendering` started. Blocks on Windows, where the logon is made.
    #[cfg_attr(windows, allow(unused_variables))]
    fn started(rendering: &Rendering, on_a_terminal: bool) -> io::Result<Harness> {
        #[cfg(windows)]
        if let Some(logon) = rendering.account() {
            if on_a_terminal {
                return Harness::on_a_console(rendering);
            }

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

        let mut command =
            tokio::process::Command::from(std::process::Command::try_from(rendering)?);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // The end a code is written to, where it is a terminal rather than the
        // pipe above.
        #[cfg(unix)]
        let keys = match on_a_terminal {
            true => {
                let (keys, keyboard) = keyboard()?;
                command.stdin(Stdio::from(keyboard));
                Some(keys)
            }
            false => None,
        };
        #[cfg(not(unix))]
        let keys: Option<tokio::fs::File> = None;

        let mut child = command.spawn()?;

        Ok(Harness {
            typing: match keys {
                Some(keys) => Some(Box::new(keys) as Box<dyn AsyncWrite + Send + Unpin>),
                None => child
                    .stdin
                    .take()
                    .map(|stdin| Box::new(stdin) as Box<dyn AsyncWrite + Send + Unpin>),
            },
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

    /// `rendering` started as the account on a console, for a harness that
    /// reads its code off one.
    ///
    /// **Because on Windows that is not its standard input.** agy 1.4.12 reads
    /// the code from `CONIN$`, the console's own input, whatever its standard
    /// input is; and the console `CreateProcessWithLogonW` gives a process is a
    /// hidden one nothing can type into. Handed the code down a pipe, it went
    /// on waiting until its minute was up — measured on Windows 11 build 26200.
    /// So the login runs where a session does, on a pseudoconsole a launcher
    /// makes as the account (see [`crate::terminal::launcher`]): the code is
    /// typed into it, its line ending made the Enter a console reads, and what
    /// it draws is read back as lines — see [`Drawn`]. Everything the harness
    /// says is on the console, so all of it comes back as what it complained.
    #[cfg(windows)]
    fn on_a_console(rendering: &Rendering) -> io::Result<Harness> {
        use tokio::io::AsyncReadExt;

        let mut terminal = crate::terminal::Terminal::open()?;
        terminal.resize(DRAWN_WIDE, crate::terminal::ROWS)?;

        let child = terminal.spawn(rendering)?;
        let terminal = Arc::new(terminal);

        let (typing, mut typed) = tokio::io::duplex(1024);
        let (mut drawing, drawn) = tokio::io::duplex(64 * 1024);

        tokio::spawn({
            let terminal = terminal.clone();

            async move {
                let mut buffer = [0u8; 1024];

                while let Ok(read @ 1..) = typed.read(&mut buffer).await {
                    let keys: Vec<u8> = buffer[..read]
                        .iter()
                        .map(|&key| if key == b'\n' { b'\r' } else { key })
                        .collect();

                    if terminal.write(&keys).await.is_err() {
                        return;
                    }
                }
            }
        });

        // Read to the end whether or not anybody is still reading what it
        // comes to: a console nobody reads is one whose harness stops.
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 16 * 1024];
            let mut seen = Drawn::default();

            while let Ok(read @ 1..) = terminal.read(&mut buffer).await {
                let _ = drawing.write_all(&seen.plain(&buffer[..read])).await;
            }
        });

        Ok(Harness {
            typing: Some(Box::new(typing)),
            printed: None,
            complained: Some(Box::new(drawn)),
            process: Process::OnAConsole(child),
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

    /// Or as the account on a console — see [`Harness::on_a_console`]. Its Job
    /// ends it as it is dropped.
    #[cfg(windows)]
    OnAConsole(crate::terminal::Child),
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

            #[cfg(windows)]
            Process::OnAConsole(child) => child.wait().await,
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

            #[cfg(windows)]
            Process::OnAConsole(child) => {
                let _ = child.start_kill();
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

/// A terminal for a harness to read its code off: the end a code is written
/// to, and the end that is its standard input.
///
/// **Nothing typed is echoed**, there being nobody to read it back: an echo
/// left unread is one that fills the terminal's buffer, and a terminal whose
/// buffer is full takes no more typing. Both ends close on exec, so nothing
/// else the server starts holds either, and the one that is handed to the
/// harness is its standard input by the time it runs.
#[cfg(unix)]
fn keyboard() -> io::Result<(tokio::fs::File, std::os::fd::OwnedFd)> {
    use rustix::fs::{Mode, OFlags};
    use rustix::pty::OpenptFlags;
    use rustix::termios::{LocalModes, OptionalActions};

    let held = rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    rustix::io::fcntl_setfd(&held, rustix::io::FdFlags::CLOEXEC)?;
    rustix::pty::grantpt(&held)?;
    rustix::pty::unlockpt(&held)?;

    let name = rustix::pty::ptsname(&held, Vec::new())?;
    let keyboard = rustix::fs::open(
        name.as_c_str(),
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;

    let mut modes = rustix::termios::tcgetattr(&keyboard)?;
    modes.local_modes.remove(LocalModes::ECHO);
    rustix::termios::tcsetattr(&keyboard, OptionalActions::Now, &modes)?;

    Ok((
        tokio::fs::File::from_std(std::fs::File::from(held)),
        keyboard,
    ))
}

/// What a pseudoconsole draws, made back into the plain lines a pipe would have
/// carried — see [`Harness::on_a_console`].
///
/// **A console draws a screen rather than printing lines**: colours, a cursor
/// that is hidden and shown, a title, and a cursor moved to where the next line
/// goes as often as a line ending. So every sequence is dropped, a move of the
/// cursor to a place is read as a line ending, a move of it forward as a space,
/// and a carriage return as nothing. Kept from one read to the next, because a
/// sequence can be cut between two of them.
#[derive(Default)]
#[cfg_attr(not(windows), allow(dead_code))]
struct Drawn {
    within: Within,
}

/// Where in a sequence the last read left off.
#[derive(Default, Clone, Copy, PartialEq)]
#[cfg_attr(not(windows), allow(dead_code))]
enum Within {
    /// Text.
    #[default]
    Nothing,
    /// An escape, and nothing yet after it.
    Escape,
    /// An escape and one byte more to come — a character set chosen.
    Chosen,
    /// A control sequence, up to its final byte.
    Control,
    /// An operating system command, up to its bell or its terminator.
    Command,
    /// An escape inside one of those, which is the first half of its
    /// terminator.
    Ending,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl Drawn {
    /// `bytes` as plain text. Bytes rather than a string, because a character
    /// can be cut between two reads as well.
    fn plain(&mut self, bytes: &[u8]) -> Vec<u8> {
        const ESCAPE: u8 = 0x1b;
        const BELL: u8 = 0x07;

        let mut plain = Vec::with_capacity(bytes.len());

        for &byte in bytes {
            self.within = match (self.within, byte) {
                (Within::Nothing, ESCAPE) => Within::Escape,
                (Within::Nothing, b'\n' | b'\t') => {
                    plain.push(byte);
                    Within::Nothing
                }
                (Within::Nothing, byte) if byte < 0x20 || byte == 0x7f => Within::Nothing,
                (Within::Nothing, byte) => {
                    plain.push(byte);
                    Within::Nothing
                }

                (Within::Escape, b'[') => Within::Control,
                (Within::Escape, b']') => Within::Command,
                (Within::Escape, b'(' | b')' | b'*' | b'+') => Within::Chosen,
                (Within::Escape | Within::Chosen, _) => Within::Nothing,

                // The final byte, which says what the sequence was.
                (Within::Control, 0x40..=0x7e) => {
                    match byte {
                        b'H' | b'f' => plain.push(b'\n'),
                        b'C' => plain.push(b' '),
                        _ => {}
                    }
                    Within::Nothing
                }
                (Within::Control, _) => Within::Control,

                (Within::Command, BELL) => Within::Nothing,
                (Within::Command, ESCAPE) => Within::Ending,
                (Within::Command, _) => Within::Command,
                (Within::Ending, _) => Within::Nothing,
            };
        }

        plain
    }
}

/// How wide the console a login is drawn on is: wider than the address agy
/// 1.4.12 prints, 704 characters, so that it is drawn as one line of it rather
/// than cut where a window of [`crate::terminal::COLUMNS`] would wrap it.
#[cfg(windows)]
const DRAWN_WIDE: u16 = 2048;

/// The address in a line the harness printed, where it is the line that
/// carries one — see [`Speaking::visit`].
fn address_in(line: &str, speaking: &Speaking) -> Option<String> {
    let url = match speaking.visit {
        Some(visit) => line.split_once(visit)?.1,
        None => line,
    }
    .trim();

    url.starts_with("https://").then(|| url.to_owned())
}

/// Whether the harness said, on its way out, that it had given up waiting for
/// a code — see [`Speaking::expired`].
fn expired(speaking: &Speaking, said: &Mutex<Vec<String>>) -> bool {
    speaking.expired.is_some_and(|expired| {
        said.lock()
            .expect("not poisoned")
            .iter()
            .any(|line| line.starts_with(expired))
    })
}

/// Ask each of `profiles` that is an account bound here whose login Verkstead
/// runs whether it is
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
        if profile.unbound.is_some()
            || !logs_in(profile.agent_type())
            || !seen.insert(profile.id.clone())
        {
            continue;
        }

        // A harness that is not on this machine is no answer either, rather
        // than an account signed out: the sandbox would fail to run it, and
        // that failure would read as `auth status` saying no. What is wrong
        // there is the machine, and the dependencies step is what says so.
        let machine = state.onboarding.machine().clone();
        let program = agents.harness(profile.agent_type());

        if !tokio::task::spawn_blocking(move || machine.runs(&program))
            .await
            .unwrap_or(true)
        {
            continue;
        }

        if let Some(logged_in) = status(agents.clone(), profile.clone(), STATUS_LIMIT).await {
            state.logins.found(state, &profile.id, logged_in);
        }
    }
}

/// Whether the account of `profile` reads as logged in, asked of the harness
/// in a fresh root — which is the account's login as the next session would be
/// given it, waited on for `limit`. `None` where nothing could answer: see
/// [`asked`].
async fn status(agents: Arc<Agents>, profile: store::Profile, limit: Duration) -> Option<bool> {
    let speaking = speaking(profile.agent_type())?;

    let built = tokio::task::spawn_blocking(move || {
        let (sandbox, argv) = agents.logging_in(&profile, speaking.status)?;
        sandbox.command(&argv).ok()
    })
    .await
    .ok()
    .flatten();

    let (rendering, closing) = built?;

    // Asked over pipes whatever the login reads its code off: nothing is typed.
    let status = match Harness::start(rendering, false).await {
        Ok(mut asking) => {
            // Nothing to type, and what it says is read rather than left
            // unread: a harness writing into a pipe nobody reads is one that
            // stops when the pipe is full. Kept, where what it says is half
            // the answer.
            drop(asking.typing.take());

            let reading: Vec<_> = [asking.printed.take(), asking.complained.take()]
                .into_iter()
                .flatten()
                .map(|mut said| {
                    tokio::spawn(async move {
                        let mut kept = Vec::new();
                        let _ = tokio::io::copy(&mut said, &mut kept).await;
                        String::from_utf8_lossy(&kept).into_owned()
                    })
                })
                .collect();

            // A harness that did not answer in time is killed as it is
            // dropped — see [`Harness`].
            match tokio::time::timeout(limit, asking.process.wait()).await {
                Ok(Ok(status)) if status.success() => Some(true),
                Ok(Ok(_)) => match speaking.signed_out {
                    None => Some(false),
                    Some(signed_out) => {
                        let mut said = String::new();

                        for read in reading {
                            if let Ok(Ok(read)) =
                                tokio::time::timeout(Duration::from_secs(2), read).await
                            {
                                said.push_str(&read);
                            }
                        }

                        said.contains(signed_out).then_some(false)
                    }
                },
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
fn failure(
    speaking: &Speaking,
    status: std::process::ExitStatus,
    said: &Mutex<Vec<String>>,
) -> String {
    let said = said.lock().expect("not poisoned");

    match said.last() {
        Some(line) => format!(
            "{} stopped without logging in: {}",
            speaking.name,
            crate::stopping::ended(line)
        ),
        None => format!("{} stopped without logging in ({status}).", speaking.name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_is_read_off_the_line_that_carries_it() {
        assert_eq!(
            address_in(
                "If the browser didn't open, visit: https://claude.com/cai/oauth/authorize?code=true",
                &CLAUDE
            ),
            Some("https://claude.com/cai/oauth/authorize?code=true".to_owned())
        );
    }

    #[test]
    fn a_line_without_an_address_carries_none() {
        assert_eq!(address_in("Opening browser to sign in…", &CLAUDE), None);
        assert_eq!(address_in("Paste code here if prompted > ", &CLAUDE), None);
        assert_eq!(address_in("visit: somewhere", &CLAUDE), None);
    }

    /// agy 1.4.12 prints the address indented on a line of its own, under the
    /// line that asks for it.
    #[test]
    fn agy_prints_its_address_on_a_line_of_its_own() {
        assert_eq!(
            address_in(
                "  https://accounts.google.com/o/oauth2/auth?access_type=offline&state=x",
                &AGY
            ),
            Some(
                "https://accounts.google.com/o/oauth2/auth?access_type=offline&state=x".to_owned()
            )
        );
        assert_eq!(
            address_in(
                "Authentication required. Please visit the URL to log in:",
                &AGY
            ),
            None
        );
        assert_eq!(
            address_in(
                "Or, paste the authorization code here and press Enter:",
                &AGY
            ),
            None
        );
    }

    /// What a pseudoconsole draws, read back as the lines a pipe would have
    /// carried: its colours and its cursor gone, a move of the cursor read as
    /// a line ending, and a sequence cut between two reads still taken whole.
    #[test]
    fn what_a_console_draws_is_read_as_lines() {
        let mut drawn = Drawn::default();

        let first = drawn.plain(
            b"\x1b[?25l\x1b[2J\x1b[m\x1b[HAuthentication required. Please visit the URL to log in:\
              \r\n\x1b[2Chttps://accounts.google.com/o/oauth2/auth?state=x\x1b[K\r\n\x1b]0;agy\x07\
              \x1b[4;1HWaiting for authentication (timeout 60s)...\x1b",
        );
        let second = drawn.plain(b"[KOr,\x1b[1Cpaste the authorization code here:\x1b[?25h");

        let lines: Vec<String> = String::from_utf8([first, second].concat())
            .expect("what was drawn is text")
            .lines()
            .map(str::to_owned)
            .collect();

        assert_eq!(
            lines,
            [
                "",
                "Authentication required. Please visit the URL to log in:",
                " https://accounts.google.com/o/oauth2/auth?state=x",
                "",
                "Waiting for authentication (timeout 60s)...Or, paste the authorization code here:",
            ]
        );
        assert_eq!(
            address_in(&lines[2], &AGY),
            Some("https://accounts.google.com/o/oauth2/auth?state=x".to_owned())
        );
    }

    #[test]
    fn an_address_that_ran_out_is_told_from_a_login_that_failed() {
        let said =
            |lines: &[&str]| Mutex::new(lines.iter().map(|line| (*line).to_owned()).collect());

        assert!(expired(
            &AGY,
            &said(&[
                "Error: authentication timed out.",
                "error: authentication failed or timed out"
            ])
        ));
        assert!(!expired(
            &AGY,
            &said(&[
                "Error: authentication failed: token exchange failed: oauth2: \"invalid_grant\"",
                "error: authentication failed or timed out"
            ])
        ));
        assert!(!expired(
            &CLAUDE,
            &said(&["Error: authentication timed out."])
        ));
    }
}
