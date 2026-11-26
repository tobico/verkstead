//! An Agent Profile's account signing out, and the stop that makes the login
//! answerable from a phone (ADR-0022).
//!
//! **The same shape as [`crate::limits`].** A session whose login expired or was
//! refused says so in a line and waits there for somebody at its terminal to
//! type `/login`. Nothing here types it: the run stops, the same as every other
//! stop — one Notice, *blocked on you*, the devices told — and the session is
//! ended, because a session left waiting would pick up again the moment
//! somebody logged in at it, inside a Conversation that reads as stopped. The
//! login is the account's business, and the Notice's Log in press is where it is
//! made: see [`crate::logins`].
//!
//! The same three records are read for the same reasons — what the session
//! printed, the frame that printing drew, and what its backend wrote in its own
//! log — and with the same latch: a display redraws its line for as long as it
//! waits, and one wait is one stop.
//!
//! **Except Claude's, which is read in its log alone.** claude writes the line
//! into its log as the answer to the turn that failed, and draws it again with
//! the rest of a conversation it resumes — Verkstead's `--resume`, or `/resume`
//! typed at it — where on the terminal it is the same line said again. Read
//! there, every resume of a conversation that had once signed out stopped as
//! soon as its past was drawn, whatever the login was by then. The log is
//! followed from where this session began in it, so what is read there is only
//! what this session said. See [`ReadIn`].
//!
//! **And a conversation carried on begins again in the log.** A session resumed
//! with the harness's own resume is watched by a [`Watch`] of its own, and the
//! following of the log it is appending to opens at the length the launch
//! measured before it started — see [`crate::transcript::Carried`]. So the line
//! a run stopped on the first time sits behind the reading rather than in front
//! of it, however many times the display draws it again. Where it *is* drawn
//! again is the terminal and the frame, and for claude neither of those is read;
//! the Capture is read by nothing that stops a run.
//!
//! **The phrase rule.** A session has signed out when one of its lines *ends*
//! with its harness's phrase — [`CLAUDE_SIGNED_OUT`], [`AGY_SIGNED_OUT`] — once
//! the terminal's decoration is off both ends of it. Claude says it as
//! `<reason> · Please run /login`, the reason first and the instruction last,
//! so what has to be anchored is the end; agy greets before it says so. Anchored, a line
//! that merely mentions the phrase — an agent reading ADR-0022 aloud, a grep of
//! this file — goes on: the phrase is in the middle of it, or a quotation mark
//! or a backtick comes after it, and ASCII punctuation is not decoration. See
//! [`says_so`].
//!
//! **And agy's needs its picker under it.** agy greets every start with the
//! same sentence, signed in or not, and only then looks for its login: signed
//! in, the next thing it draws is `Signing in...`, and the line is drawn over a
//! moment later; signed out, it is its login picker. So agy's sentence counts
//! only with [`AGY_PICKER`] as the next line drawn — see [`Phrase::then`].
//!
//! **A login resumes what it unblocks.** Once a login on the Profile has
//! landed, every run stopped as Signed out on it is started again through
//! Resume — see [`logged_in`]. Each one builds a fresh root, which is what
//! picks the new login up: on Linux the old session's bind still holds the old
//! file. And the members are told, because a run under a Profile not bound on
//! its device is stopped and recorded there — see [`hearing`].
//!
//! **And each of them waits for its session to go.** A sign-out writes the
//! stop and ends the session on the word that it was written, in that order,
//! so a resume that lands in between is refused as already driven and starts
//! nothing. Both of the resumes made here are Verkstead's own press with no
//! second one behind it, so both wait — see [`let_go`].
//!
//! **A sign-out after a fresh login resumes by itself.** A session can sign
//! out after the human has already logged in again: its root was built before
//! the login, and on Linux it still holds the old file. So the account's login
//! is read as the root is built — see [`Login`] — and where it has been
//! replaced by the time the session signs out, the stop is one nobody chose
//! and nobody is woken for. Its Notice says why, and the run is started again
//! the moment the session has gone — see [`carrying_on`].
//!
//! **Claude and the Antigravity CLI only.** The other harnesses log in
//! differently, and none of them has a login Verkstead can run yet — so they
//! have no phrase, and a session on one is never stopped here.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use sqlx::SqlitePool;
use tokio::sync::mpsc;
use verkstead_schema::{Nudge, Nudged};

use crate::AppState;
use crate::limits::{DECORATION, Held};
use crate::nudge::Nudges;
use crate::store;

/// What claude 2.1.283 ends a line with when its account's login is gone:
/// `Not logged in · Please run /login`, `Login expired · Please run /login`,
/// `OAuth token revoked · Please run /login`, and
/// `API Error: 401 Invalid API key · Please run /login`. Read off the binary.
///
/// Matched without regard to case. The reasons in front of it are claude's and
/// will move; the instruction is the part they all share.
const CLAUDE_SIGNED_OUT: &str = "please run /login";

/// And what agy 1.4.12 ends a line with when it has no login to run on:
/// `Welcome to the Antigravity CLI. You are currently not signed in.`, over the
/// sign-in picker it then waits at.
///
/// Drawn the same for an account that never had a login and for one whose
/// token was refused when agy went to refresh it — both read off the real
/// binary, the second with Google's token endpoint answered by a stand-in.
/// **And drawn by a session signed in too**, on its way up: measured on a Mac,
/// where the Keychain agy is refused first holds it long enough to be seen,
/// the sentence is followed by `Signing in...` and drawn over once the login
/// file is read. What tells the two apart is [`AGY_PICKER`].
const AGY_SIGNED_OUT: &str = "you are currently not signed in.";

/// The line agy 1.4.12 draws next when it really has no login: the head of
/// its sign-in picker, `Select login method:`, over the methods to pick from.
const AGY_PICKER: &str = "select login method:";

/// What a session of one harness says when its account has signed out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Phrase {
    /// The sentence a line ends with.
    ends: &'static str,

    /// The line that has to be the next one drawn after it, blank lines
    /// aside, where the sentence alone is also said by a session signed in.
    then: Option<&'static str>,

    /// Which of the session's records it is read in.
    read_in: ReadIn,
}

/// Which of the three records a session leaves a sign-out is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadIn {
    /// What it printed, the frame it drew, and its backend's own log.
    Every,

    /// Its backend's own log, and nothing it puts on the terminal — for a
    /// harness that draws its conversation so far again on the screen, where a
    /// sign-out it stopped on before is a line drawn rather than said.
    Log,
}

/// What a session of `agent_type` says when its account has signed out — or
/// nothing, where Verkstead cannot run that harness's login.
fn signed_out_phrase(agent_type: store::AgentType) -> Option<Phrase> {
    match agent_type {
        // Read in its log alone. claude 2.1.329 writes the line into its log as
        // the turn's answer — an assistant message marked as an API error — and
        // draws every message of a conversation it resumes, that one among them:
        // `--resume` as a Conversation carries on, and `/resume` typed at it by
        // the human. On the terminal the two are the same line, so a resumed
        // conversation that had signed out once was stopped on that old line as
        // soon as it was drawn, every time, whatever the login now was. The log
        // is followed from where this session began in it, so it holds only what
        // this session said.
        store::AgentType::Claude => Some(Phrase {
            ends: CLAUDE_SIGNED_OUT,
            then: None,
            read_in: ReadIn::Log,
        }),
        store::AgentType::Agy => Some(Phrase {
            ends: AGY_SIGNED_OUT,
            then: Some(AGY_PICKER),
            read_in: ReadIn::Every,
        }),
        store::AgentType::Codex | store::AgentType::Grok | store::AgentType::OpenCode => None,
    }
}

/// The line in `text` that says the account signed out, tidied of the
/// terminal's own sequences — or `None`.
fn saying(text: &str, phrase: Option<Phrase>) -> Option<String> {
    let phrase = phrase?;
    let lines: Vec<String> = text.lines().map(crate::capture::plain).collect();

    lines.iter().enumerate().find_map(|(at, line)| {
        let said = says_so(line, phrase.ends)
            && phrase
                .then
                .is_none_or(|then| followed_by(&lines[at + 1..], then));

        said.then(|| line.clone())
    })
}

/// Whether the first line of `after` that is not blank is `then`, once the
/// decoration is off both ends of it.
fn followed_by(after: &[String], then: &str) -> bool {
    after
        .iter()
        .map(|line| {
            line.trim_start_matches(DECORATION)
                .trim_end_matches(DECORATION)
        })
        .find(|line| !line.is_empty())
        .is_some_and(|line| line.eq_ignore_ascii_case(then))
}

/// Whether one tidied line *says* the account signed out, rather than
/// mentioning the phrase.
///
/// The phrase has to end the line once the decoration is off the back — the
/// box border, the spaces — and there has to be something in front of it once
/// the decoration is off the front: claude always gives a reason, and a line
/// that is the phrase and nothing else is somebody typing it.
fn says_so(line: &str, phrase: &str) -> bool {
    let line = line
        .trim_start_matches(DECORATION)
        .trim_end_matches(DECORATION);

    let Some(reason) = line.len().checked_sub(phrase.len()) else {
        return false;
    };

    line.is_char_boundary(reason)
        && reason > 0
        && line[reason..].eq_ignore_ascii_case(phrase)
        && line[..reason].ends_with(char::is_whitespace)
}

/// Where the Profile a session runs under is bound, which decides what the
/// Notice offers: a Log in press here, or the name of the device to log in on.
#[derive(Debug, Clone)]
pub(crate) enum Home {
    /// Bound on this device, so its login is made here.
    Here,

    /// Not bound here: its account was fetched from the device the lending
    /// names, which is where its login is made — and which is told when a
    /// session here finds it signed out, see
    /// [`crate::mirroring::account::Lending::signed_out`].
    Away(Box<crate::mirroring::account::Lending>),
}

/// The account's login file as it was when a session's root was built, so that
/// a sign-out can tell a login replaced since from the one it was given.
///
/// Only a Claude or an Antigravity CLI account bound here: one not bound here
/// runs from a copy fetched before each launch, and the other harnesses are
/// never stopped here at all.
#[derive(Debug, Clone)]
pub(crate) struct Login {
    path: PathBuf,

    /// When it was last written, or `None` where there was no login then.
    built: Option<SystemTime>,
}

impl Login {
    /// Read the login of `profile`'s account now, just before its root is
    /// built — or `None` where a sign-out on it cannot resume by itself.
    pub(crate) fn before_building(profile: &store::Profile) -> Option<Login> {
        if profile.unbound.is_some() || signed_out_phrase(profile.agent_type()).is_none() {
            return None;
        }

        let path = crate::sandbox::root::login_at(&profile.account);
        let built = written(&path);

        Some(Login { path, built })
    }

    /// Whether the login has been replaced since the root was built.
    fn replaced(&self) -> bool {
        newer(self.built, written(&self.path))
    }
}

/// When the file at `path` was last written, or `None` where there is none.
fn written(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// Whether a login written at `now` is newer than the one written at `built`:
/// one that has appeared since, or one written later.
fn newer(built: Option<SystemTime>, now: Option<SystemTime>) -> bool {
    match (built, now) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(built), Some(now)) => now > built,
    }
}

/// The runs stopped because their session signed out on a login that had
/// already been replaced, on their way to [`carrying_on`].
///
/// Held by the sessions register, which is where a session's relay can reach
/// it from; read by the one task that has the whole of the server to resume
/// with. Every clone is the same channel.
#[derive(Clone)]
pub(crate) struct Late {
    sender: mpsc::UnboundedSender<i64>,
    receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<i64>>>>,
}

impl Default for Late {
    fn default() -> Late {
        let (sender, receiver) = mpsc::unbounded_channel();

        Late {
            sender,
            receiver: Arc::new(Mutex::new(Some(receiver))),
        }
    }
}

impl Late {
    /// Say that `conversation_id` is to carry on once its session has gone.
    fn carry_on(&self, conversation_id: i64) {
        // A send that fails is a server with nothing listening, which is one
        // running no sessions: the stop stays, and a restart takes it up.
        let _ = self.sender.send(conversation_id);
    }

    /// The reading end, to the first that asks.
    fn taken(&self) -> Option<mpsc::UnboundedReceiver<i64>> {
        self.receiver
            .lock()
            .expect("the late sign-outs are not poisoned")
            .take()
    }
}

/// One session, watched for the line that says its account signed out.
///
/// Held by the relay beside [`crate::limits::Watch`] and fed the same records.
pub(crate) struct Watch {
    conversation_id: i64,
    event_id: i64,

    /// The Profile this session runs under, by its uuid: what the stop records
    /// a login as waiting on.
    profile_id: String,

    /// And what it is called, which is what the Notice and the push name.
    profile: String,

    /// And where it is bound — here, or the device its account was fetched from.
    home: Home,

    /// The phrase, off the agent the Profile runs — see [`signed_out_phrase`].
    phrase: Option<Phrase>,

    /// The account's login as the root was built, where a sign-out may find it
    /// replaced since — see [`Login`].
    login: Option<Login>,

    /// And where a run stopped on a replaced login is sent to carry on.
    late: Late,

    printed: Held,

    /// Whether the line on screen now has already been stopped on — see
    /// [`crate::limits::Watch`], whose latch this is.
    raised: bool,
}

impl Watch {
    /// Watch the session printing into `event_id`, run under the Profile
    /// `profile_id`, called `profile`, which runs `agent_type`. `login` is the
    /// account's login as the session's root was built.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn on(
        conversation_id: i64,
        event_id: i64,
        profile_id: String,
        profile: String,
        home: Home,
        agent_type: store::AgentType,
        login: Option<Login>,
        late: Late,
    ) -> Watch {
        Watch {
            conversation_id,
            event_id,
            profile_id,
            profile,
            home,
            phrase: signed_out_phrase(agent_type),
            login,
            late,
            printed: Held::default(),
            raised: false,
        }
    }

    /// Take a chunk of what the session printed.
    pub(crate) fn printed(&mut self, text: &str) {
        self.printed.push(text);
    }

    /// The line in the records its harness is read in that says the account
    /// signed out — see [`ReadIn`].
    fn found(&self, drawn: &str, said: Option<&str>) -> Option<String> {
        let in_the_log = || saying(said.unwrap_or_default(), self.phrase);

        match self.phrase.map(|phrase| phrase.read_in) {
            Some(ReadIn::Log) => in_the_log(),

            _ => saying(self.printed.text(), self.phrase)
                .or_else(|| saying(drawn, self.phrase))
                .or_else(in_the_log),
        }
    }

    /// Look at what has arrived, and stop the run if it says the account signed
    /// out.
    ///
    /// `true` is *this session is to end*, for [`crate::limits::Watch::look`]'s
    /// reason: the relay is what ends it.
    #[must_use = "a stop for a sign-out ends the session it stopped"]
    pub(crate) async fn look(
        &mut self,
        pool: &SqlitePool,
        nudges: &Nudges,
        drawn: &str,
        said: Option<&str>,
    ) -> bool {
        let printed_something = !self.printed.text().is_empty();

        let found = self.found(drawn, said);

        self.printed.looked();

        let Some(found) = found else {
            if printed_something {
                self.raised = false;
            }

            return false;
        };

        if self.raised {
            return false;
        }

        self.raised = true;

        signed_out(pool, nudges, self, &found).await
    }
}

/// Stop the run: write the stop and its Notice, tell the devices, and say that
/// the session is to end. `true` on every way of returning but a stop that
/// could not be written — see [`crate::limits`], whose rule this is.
async fn signed_out(pool: &SqlitePool, nudges: &Nudges, watch: &Watch, said: &str) -> bool {
    let conversation_id = watch.conversation_id;

    let lifecycle = match store::load_conversation(pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state,
        Ok(None) => return false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation whose account signed out failed");
            return false;
        }
    };

    // Read now rather than as the line arrived: a login that lands while the
    // session is saying so is one the next session is given.
    if watch.login.as_ref().is_some_and(Login::replaced) {
        return late(pool, nudges, watch, said, lifecycle).await;
    }

    let home = match &watch.home {
        Home::Here => None,
        Home::Away(lending) => Some(home_named(pool, lending.at()).await),
    };

    // And the device the login is made on is told, which is the one device
    // that cannot see this: its file is still there, and without the word its
    // card goes on reading *Logged in* with nothing to press. Off to one side,
    // a dial being no reason to hold the stop up.
    if let Home::Away(lending) = &watch.home {
        let lending = lending.as_ref().clone();

        tokio::spawn(async move { lending.signed_out().await });
    }

    let stopped = crate::stopping::stop(
        pool,
        nudges,
        conversation_id,
        crate::stopping::Decided::SignedOut {
            id: &watch.profile_id,
            profile: &watch.profile,
        },
        crate::stalls::driving(lifecycle),
        &crate::stopping::signed_out(&watch.profile, said, home.as_deref()),
        Some(watch.event_id),
    )
    .await;

    match stopped {
        Ok(None) => {
            tracing::info!(
                conversation_id,
                session = watch.event_id,
                "an account signed out on a run that had already stopped"
            );
            true
        }
        Ok(Some(notice)) => {
            tracing::warn!(
                conversation_id,
                notice,
                session = watch.event_id,
                profile = watch.profile,
                "an account signed out, so the run has stopped"
            );
            true
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "a run could not be stopped on a sign-out");
            false
        }
    }
}

/// Stop the run whose session signed out on a login that has since been
/// replaced, and send it on to carry on with the new one.
///
/// A stop nobody chose, so nobody is woken and a restart takes it up unasked:
/// what it is waiting for is the session to go, not the human. The Notice says
/// why the run went on by itself. `true` on every way of returning but a stop
/// that could not be written, as [`signed_out`].
async fn late(
    pool: &SqlitePool,
    nudges: &Nudges,
    watch: &Watch,
    said: &str,
    lifecycle: store::Lifecycle,
) -> bool {
    let conversation_id = watch.conversation_id;

    let stopped = crate::stopping::stop(
        pool,
        nudges,
        conversation_id,
        crate::stopping::Decided::Nobody,
        crate::stalls::driving(lifecycle),
        &crate::stopping::signed_out_since(&watch.profile, said),
        Some(watch.event_id),
    )
    .await;

    match stopped {
        Ok(None) => {
            tracing::info!(
                conversation_id,
                session = watch.event_id,
                "an account signed out on a run that had already stopped"
            );
            true
        }
        Ok(Some(notice)) => {
            tracing::info!(
                conversation_id,
                notice,
                session = watch.event_id,
                profile = watch.profile,
                "a session signed out on a login since replaced, so the run carries on"
            );
            watch.late.carry_on(conversation_id);
            true
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "a run could not be stopped on a sign-out");
            false
        }
    }
}

/// How long a run being started again waits for its session, and for whatever
/// was driving it, to let go before it is started again anyway.
const LETTING_GO: Duration = Duration::from_secs(30);

/// How often it looks.
const LOOKING_EVERY: Duration = Duration::from_millis(100);

/// Start again every run a late sign-out stopped, from now until the process
/// stops — see [`Late`].
///
/// Nothing on a server that runs no sessions, which has no sign-out to hear.
pub(crate) fn carrying_on(state: &AppState) {
    let Some(mut late) = state.sessions.late().taken() else {
        return;
    };

    let state = state.clone();

    tokio::spawn(async move {
        while let Some(conversation_id) = late.recv().await {
            tokio::spawn(carry_on(state.clone(), conversation_id));
        }
    });
}

/// Start `conversation_id` again through Resume, once the session that signed
/// out has gone and the driver that launched it has let go — see [`let_go`].
async fn carry_on(state: AppState, conversation_id: i64) {
    let_go(&state, conversation_id).await;

    match crate::resume::resume(&state, conversation_id, crate::resume::Resuming::LoggedIn).await {
        Ok(verkstead_render::Resumed::Resumed) => tracing::info!(
            conversation_id,
            "a run that signed out on a login since replaced is being driven again"
        ),
        Ok(refusal) => tracing::warn!(
            conversation_id,
            ?refusal,
            "a run that signed out on a login since replaced could not be started again"
        ),
        Err(error) => tracing::error!(
            error = ?error,
            conversation_id,
            "starting a run that signed out on a login since replaced failed"
        ),
    }
}

/// Wait for the session that stopped `conversation_id`, and for whatever was
/// driving it, to let go — or for [`LETTING_GO`] to run out.
///
/// **What a resume nobody is going to press again has to wait for.** A
/// sign-out's stop is written while its session is still running: the stop
/// goes down first and the relay ends the session on the word that it was
/// written — see [`Watch::look`]. A Resume landing in between is refused as
/// already driven, which takes the stop away and launches nothing, and that is
/// right for a press somebody makes again a moment later and no use at all
/// here, where the press is Verkstead's own and there is no second one. So the
/// run would sit with its badge gone and nothing driving it until the human
/// found it and pressed Resume themselves.
///
/// And a driver still holding on is one that may yet launch its own next step
/// over the stop Resume has just taken away. A wrap-up's watchers may hold on
/// long after, which is what the limit is for: Resume over a driver that is
/// still registered is what the press does too.
async fn let_go(state: &AppState, conversation_id: i64) {
    let deadline = Instant::now() + LETTING_GO;

    while (state.sessions.working().contains(&conversation_id)
        || state.drivers.registered(conversation_id))
        && Instant::now() < deadline
    {
        tokio::time::sleep(LOOKING_EVERY).await;
    }
}

/// A login on `profile` has landed: resume every run stopped as Signed out
/// waiting on it.
///
/// Through Resume, the same as the press, so what each one starts is what
/// ought to be running now. A run stopped for any other reason, or waiting on
/// another Profile, is left as it is. A refusal leaves its run stopped where
/// the human will find it, and pressing Resume there says why.
///
/// Each one waits for its session to go first — see [`let_go`], which is why.
pub(crate) async fn logged_in(state: AppState, profile: String) {
    let waiting = match store::signed_out_on(&state.pool, &profile).await {
        Ok(waiting) => waiting,
        Err(error) => {
            tracing::error!(error = ?error, profile_id = %profile, "reading the runs a login unblocks failed");
            return;
        }
    };

    for conversation_id in waiting {
        let_go(&state, conversation_id).await;

        match crate::resume::resume(&state, conversation_id, crate::resume::Resuming::LoggedIn)
            .await
        {
            Ok(verkstead_render::Resumed::Resumed) => tracing::info!(
                conversation_id,
                profile_id = %profile,
                "a login unblocked a run, so it is being driven again"
            ),
            Ok(refusal) => tracing::warn!(
                conversation_id,
                profile_id = %profile,
                ?refusal,
                "a login unblocked a run, but nothing could be started for it"
            ),
            Err(error) => tracing::error!(
                error = ?error,
                conversation_id,
                profile_id = %profile,
                "starting a run a login unblocked failed"
            ),
        }
    }
}

/// Resume the runs here that a member's login unblocks, from now until the
/// process stops.
///
/// A run under a Profile not bound here runs from a copy of the account,
/// fetched from a device it is bound on, and its stop is recorded here. So a
/// login made over there resumes nothing here by itself: [`logged_in`] reads
/// the runs of the device it runs on. This is what hears that device's
/// [`Nudge::LoggedIn`] and resumes them, each fetching the new login as it
/// launches.
///
/// **Only where the Profile is not bound here.** A run under one bound here
/// runs from this device's own login, which a login on another device has not
/// changed: resumed, it would only sign out again. A Nudge that never lands
/// leaves the run stopped, the way it was before, for Resume to start.
///
/// Nothing on a server with no members or no sessions.
pub(crate) fn hearing(state: &AppState) {
    if state.devices.is_none() || state.sessions.agents().is_none() {
        return;
    }

    let state = state.clone();
    let mut moved = state.nudges.subscribe();

    tokio::spawn(async move {
        loop {
            match moved.recv().await {
                Ok(Nudged {
                    device: Some(device),
                    moved: Nudge::LoggedIn { profile },
                    ..
                }) => {
                    tokio::spawn(logged_in_on(state.clone(), device, profile));
                }
                Ok(_) => {}

                // What was missed is left stopped for Resume, as a Nudge that
                // never landed is.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                    tracing::debug!(missed, "the sign-outs fell behind the Nudges");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            }
        }
    });
}

/// A login on `profile` has landed on the member `device`: resume the runs
/// here stopped on it, where this device runs it from a copy.
async fn logged_in_on(state: AppState, device: String, profile: String) {
    match store::load_profile(&state.pool, &profile).await {
        Ok(Some(held)) if held.unbound.is_some() => {
            tracing::info!(
                device = %device,
                profile_id = %profile,
                "a member logged in on a Profile this device runs from a copy",
            );
            logged_in(state, profile).await;
        }
        Ok(_) => {}
        Err(error) => {
            tracing::error!(error = ?error, profile_id = %profile, "reading the Profile a member logged in on failed");
        }
    }
}

/// What the device with `device` for its Device Id is shown under, or words
/// that stand in for it where this device has never heard of it.
async fn home_named(pool: &SqlitePool, device: &str) -> String {
    match store::members(pool).await {
        Ok(members) => members
            .into_iter()
            .find(|member| member.device == device)
            .map(|member| member.name),
        Err(error) => {
            tracing::error!(error = ?error, device, "reading the device a Profile is bound on failed");
            None
        }
    }
    .unwrap_or_else(|| "the device it is bound on".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    use store::AgentType::{Agy, Claude, Codex, Grok, OpenCode};

    fn signed_out(text: &str) -> Option<String> {
        saying(text, signed_out_phrase(Claude))
    }

    fn watching() -> Watch {
        watching_on(Claude)
    }

    fn watching_on(agent_type: store::AgentType) -> Watch {
        Watch::on(
            1,
            2,
            "3".to_owned(),
            "fable".to_owned(),
            Home::Here,
            agent_type,
            None,
            Late::default(),
        )
    }

    /// A login is newer where it has appeared since the root was built, or was
    /// written later; one that is unchanged, or gone, is not.
    #[test]
    fn a_login_is_newer_where_it_was_written_since() {
        let built = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let later = built + Duration::from_secs(1);

        assert!(newer(None, Some(built)), "appeared since");
        assert!(newer(Some(built), Some(later)), "written since");
        assert!(!newer(Some(built), Some(built)), "unchanged");
        assert!(!newer(None, None), "never there");
        assert!(!newer(Some(built), None), "gone");
    }

    /// The lines claude 2.1.283 ends with the phrase, as it draws them.
    #[test]
    fn the_lines_claude_draws_when_its_login_is_gone_are_recognised() {
        for line in [
            "Not logged in · Please run /login",
            "Login expired · Please run /login",
            "OAuth token revoked · Please run /login",
            "API Error: 401 Invalid API key · Please run /login",
            "  ⎿  Login expired · Please run /login",
            "│ Not logged in · Please run /login │",
            "\u{1b}[31mLogin expired · Please run /login\u{1b}[0m",
        ] {
            assert!(signed_out(line).is_some(), "{line:?} was not recognised");
        }
    }

    /// The line is kept as it was printed, tidied of the terminal's sequences.
    #[test]
    fn the_line_is_kept_as_it_was_printed() {
        assert_eq!(
            signed_out("\u{1b}[2m⎿\u{1b}[0m  Login expired · Please run /login\n"),
            Some("⎿  Login expired · Please run /login".to_owned()),
        );
    }

    /// The phrase in the middle of a line is a mention, and so is a line that
    /// quotes it: the end is anchored, and ASCII punctuation is not decoration.
    #[test]
    fn a_line_that_mentions_the_phrase_is_not_a_sign_out() {
        for line in [
            "Please run /login · Your session has expired",
            "Your session has expired. Please run /login to sign in again.",
            "A session has signed out when a line ends with `Please run /login`.",
            r#"const CLAUDE: &str = "Login expired · Please run /login";"#,
            r#"            "Login expired · Please run /login","#,
            "Claude says it as `<reason> · Please run /login`, once terminal",
            "- Login expired · Please run /login.",
            "Please run /login",
            "  │ Please run /login",
            "Please run /loginnow",
            "",
        ] {
            assert_eq!(signed_out(line), None, "{line:?} stopped a run");
        }
    }

    /// What agy 1.4.12 draws over its sign-in picker, as it draws it.
    #[test]
    fn the_line_agy_draws_when_it_has_no_login_is_recognised() {
        for line in [
            " Welcome to the Antigravity CLI. You are currently not signed in.",
            "\u{1b}[1mWelcome to the Antigravity CLI.\u{1b}[0m You are currently not signed in.",
        ] {
            let drawn = format!("{line}\r\n\n Select login method:\n > 1. Google OAuth\n");

            assert_eq!(
                saying(&drawn, signed_out_phrase(Agy)),
                Some(crate::capture::plain(line)),
                "{drawn:?} was not recognised"
            );
        }

        for line in [
            "You are currently not signed in.",
            "agy says `You are currently not signed in.` over its picker",
            "Welcome to the Antigravity CLI. You are currently not signed in. Pick one.",
            "Not logged in · Please run /login",
        ] {
            let drawn = format!("{line}\n Select login method:\n");

            assert_eq!(saying(&drawn, signed_out_phrase(Agy)), None, "{line:?}");
        }
    }

    /// agy greets a start signed in with the same sentence, under which it
    /// draws `Signing in...` rather than its picker — as a session on a Mac
    /// drew it, the Keychain refused first. That is no sign-out, and nor is the
    /// sentence with nothing yet under it.
    #[test]
    fn agy_signing_in_is_not_signed_out() {
        for drawn in [
            "\n Welcome to the \u{1b}[38;5;69mAntigravity CLI\u{1b}[m. You are currently not signed in.\r\n\n \u{28f7}  Signing in...\u{1b}[0 q\r",
            " Welcome to the Antigravity CLI. You are currently not signed in.\n",
            " Welcome to the Antigravity CLI. You are currently not signed in.",
        ] {
            assert_eq!(saying(drawn, signed_out_phrase(Agy)), None, "{drawn:?}");
        }
    }

    /// Only the sessions of a harness whose login Verkstead runs are read.
    #[test]
    fn only_a_session_whose_login_verkstead_runs_is_read() {
        for agent_type in [Codex, Grok, OpenCode] {
            assert_eq!(
                saying(
                    "Login expired · Please run /login",
                    signed_out_phrase(agent_type)
                ),
                None,
            );
        }
    }

    /// A line split across two chunks is still one line.
    #[test]
    fn a_line_split_across_two_chunks_is_still_one_line() {
        let mut watch = watching_on(Agy);

        watch.printed("Welcome to the Antigravity CLI. You are currently not sig");
        watch.printed.looked();
        assert_eq!(watch.found("", None), None, "half a line is not one");

        watch.printed("ned in.\nSelect login method:\n");
        assert!(watch.found("", None).is_some());
    }

    /// And the frame is read as well as the terminal, for a harness read in
    /// every record.
    #[test]
    fn the_frame_is_read_too() {
        let watch = watching_on(Agy);

        assert!(
            watch
                .found(
                    "Welcome to the Antigravity CLI. You are currently not signed in.\n\n\
                     Select login method:\n",
                    None
                )
                .is_some(),
            "the frame",
        );
    }

    /// Claude's is read in its log and nowhere else: on the terminal and in the
    /// frame the line is also drawn again, with the rest of a conversation it
    /// resumes, long after it was said.
    #[test]
    fn claude_is_read_in_its_log_alone() {
        let mut watch = watching();
        let line = "● Login expired · Please run /login";

        watch.printed(&format!("{line}\n"));
        assert_eq!(watch.found(line, None), None, "the terminal and the frame");

        assert!(
            watch
                .found("", Some("Login expired · Please run /login"))
                .is_some(),
            "the log",
        );
    }
}
