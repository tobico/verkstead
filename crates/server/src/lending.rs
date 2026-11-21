//! The host side of the `activity` world: what Verkstead lends whatever drives a
//! Conversation, in the shape `wit/plugin.wit` gives it (ADR-0030, *Activity*).
//!
//! **One way to launch a session for a Conversation, and one way to wait on
//! it.** [`Lent`] implements the trait `bindgen!` generates for the world's
//! imports, and every driver in [`crate::runner`], the grilling's first launch
//! and its relaunch included, starts and waits on its sessions through it. A
//! component hosted later reaches the server through the same calls, so the
//! interface is exercised by the built-ins long before a stranger's Activity
//! needs it.
//!
//! **The driver writes the prompt and the host adds the rest.** What a session
//! is told about its own work is the caller's, built with the same
//! [`crate::skills`] builder it always was; what every session is told —
//! Memos, previews, packs, companions, transfer, naming — is
//! [`crate::sessions::Sessions::start`]'s, and the Answers folded from Deferred
//! Asks are added here, under it all, as they always were.
//!
//! **What only an in-process driver can say is said to the [`Lent`] itself**,
//! rather than widened into the world: whether the caller already holds the
//! Turn, the Sets a relaunched grilling holds open, the building kind a
//! session's Memos are left under, and what the rescue says the session is
//! doing. And what a wait came to is read back in full — how a session ended,
//! and the Event it printed into — through [`Lent::see`] and
//! [`Lent::event_id`], where the world's `waited` says only *gone*.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::sync::watch;
use verkstead_schema::{Direction, Nudge};
use verkstead_store as store;

use crate::AppState;
use crate::continuing::{Began, Brought};
use crate::done::{Ends, Evidence};
use crate::drivers::Driving;
use crate::memos::Building;
use crate::plugins::component::worlds::category::activity::verkstead::plugin::{
    activity_types as world, host,
};
use crate::runner::Landing;
use crate::sessions::{Ended, Session};

/// What the host lends one Conversation: its sessions, and what an in-process
/// driver has said about the next of them.
pub(crate) struct Lent {
    state: AppState,
    conversation_id: i64,

    /// Whether a launch waits for the Turn and asks after a stop, or the caller
    /// is holding the Turn already — see [`Lent::turn_held`].
    turn: Turn,

    /// The Sets a relaunched grilling leaves open rather than locking.
    held: Vec<i64>,

    /// What a session launched here is building, where it is — which is what
    /// says it may retire a Memo, and what one it leaves is left under.
    building: Option<Building>,

    /// The Directions a session launched expecting `picked` offers — see
    /// [`Lent::offering`].
    offers: Vec<Direction>,

    /// What the rescue says the session is doing, where it goes idle without a
    /// word.
    doing: String,

    /// Dropped once the next launch is over, which is what wakes a caller
    /// waiting on it — see [`Lent::telling`].
    telling: Option<tokio::sync::oneshot::Sender<()>>,

    /// A registration held across the next launch alone — see
    /// [`Lent::launching`].
    launching: Option<Driving>,

    /// The registration a pick took, held from then on — see
    /// [`Lent::wait_for_the_pick`].
    picked: Option<Driving>,

    /// Every session launched here, by the handle the world names it
    /// by.
    watched: HashMap<world::Session, Watched>,
    issued: world::Session,

    /// The Timeline Event the newest session launched here prints into, which
    /// is what a stop asked for through the world carries the tail of.
    writing: Option<i64>,

    /// Whether the newest session launched here was building — see
    /// [`Lent::building`] — so that what it asked and left standing is
    /// answered into the next session's prompt rather than closed by a stop.
    folding: bool,

    /// What brought the run about, which a Conversation it starts is held to
    /// the limits by and says what it says by — see [`Lent::brought`].
    brought: Brought,

    /// The sentences a start was answered *no place* with for want of a place
    /// on the server, which a look says out loud where the places are full —
    /// see [`Brought::noticed`].
    waiting: Vec<String>,

    /// How many starts asked for here were weighed, had a place and halted:
    /// places spent, for the rest of the run — see [`crate::continuing::begin`].
    spent: usize,
}

/// Whether a launch waits for the Conversation's Turn.
///
/// Every launch takes it but the ones whose caller already holds it — a fix
/// session and the review, both dispatched by a wrap-up that took it before it
/// decided anything — and a grilling's, which nothing else is driving yet.
///
/// The wait is what keeps a launch from displacing a session somebody else put
/// there. [`crate::sessions::Sessions::start`] ends whatever is registered, which
/// is exactly what a run relaunching its own step wants and exactly what must
/// not happen to a session a steer set going — the human steers in a quiet
/// moment between steps, and a run that reached the next one a second later
/// would kill it mid-sentence. A steer holds the Turn for as long as its session
/// runs, so this waits for it rather than ending it.
///
/// Held across the launch alone rather than across the session, because that is
/// the whole of what it is protecting: once a session is registered, everything
/// else that might start one can see it there.
///
/// And a stop lands here, which is why it is asked after the wait rather than
/// before: *nothing new starts* has to be true of the moment a session would be
/// started, and the wait is however long the session in front of this one took.
/// Every launch a run makes goes through here, so this is the one place that can
/// say it of all of them — which is why what it asks is
/// [`crate::stopping::stopped`] and not [`crate::stops::asked`] alone. A press that
/// is still waiting is only one of the two ways a run stops: a Force stop writes
/// its stop outright, and so does a Stop pressed in one of the quiet moments
/// between sessions. A launch that looked for the waiting press alone would walk
/// straight past both of them.
enum Turn {
    Take,
    Held,
    /// Taken by the caller and handed over, to be held across the next launch
    /// and let go once it is over — a relaunched grilling, which waited for
    /// whatever was running to finish before it decided anything.
    Handed(crate::sessions::Turn),
    /// Tried rather than waited for, and held for as long as the session it
    /// was taken for runs — the wrap-up's dispatches, see [`Lent::trying`].
    Try,
}

/// What a launch tried for the Turn is refused with where something else holds
/// it: the Wrap-up component looks again on its next round.
pub(crate) const BUSY: &str = "Something else is working in the Worktree.";

/// One session the host is watching, and what its Done signal is checked
/// against while it is waited on.
struct Watched {
    session: Session,
    expected: Expected,
    ends: Ends,

    /// How it ended, once a wait has seen it end.
    ended: Option<Ended>,

    /// Its Done signal checked and its silences rescued across the ticks of a
    /// `wait-any` — see [`Seeing`].
    seeing: Option<Seeing>,

    /// The Turn it was launched under, where it was tried for — held until a
    /// wait sees it out. See [`Lent::trying`].
    turn: Option<crate::sessions::Turn>,
}

/// A session's Done signal held to its evidence, and the session spoken to
/// where it goes quiet, for as long as a `wait-any` keeps coming back to it —
/// across its ticks rather than within one wait, so a signal given between two
/// waits is not lost, and neither the grace after a signal nor the span a
/// rescue waits out starts again at every tick.
///
/// Dropped as the wait answers for the session, or with the [`Lent`], which
/// takes both away.
struct Seeing {
    _expecting: crate::done::Expecting,

    /// Over once the signal was given and taken and the session has been idle
    /// since, the rescue running beside it until then.
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Seeing {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// What a session's Done signal is checked against: evidence known at its
/// launch, or the artifact of whichever Direction the human picks.
enum Expected {
    Evidence(Evidence),

    /// The latest pick, as [`crate::followers`] carries it.
    Picked(watch::Receiver<Option<Direction>>),
}

impl Lent {
    /// The host lent to `conversation_id`'s driver: a launch takes the Turn,
    /// nothing is held, nothing is building, and the rescue says the session is
    /// working.
    pub(crate) fn new(state: &AppState, conversation_id: i64) -> Lent {
        Lent {
            state: state.clone(),
            conversation_id,
            turn: Turn::Take,
            held: Vec::new(),
            building: None,
            offers: Direction::ALL.to_vec(),
            doing: "working".to_owned(),
            telling: None,
            launching: None,
            picked: None,
            watched: HashMap::new(),
            issued: 0,
            writing: None,
            folding: false,
            brought: Brought::Settle,
            waiting: Vec::new(),
            spent: 0,
        }
    }

    /// What brought the run about, where it may start Conversations: a look
    /// says less — see
    /// [`crate::continuing::begin`]. A settle where nobody says.
    pub(crate) fn brought(mut self, brought: Brought) -> Lent {
        self.brought = brought;
        self
    }

    /// The caller holds the Turn already — a wrap-up takes it before it decides
    /// what to dispatch — or, a grilling, is the only thing that could be
    /// starting a session: a launch neither waits for it nor asks after a stop.
    pub(crate) fn turn_held(mut self) -> Lent {
        self.turn = Turn::Held;
        self
    }

    /// Hold `turn`, which the caller took, across the next launch and let it
    /// go once that is over: neither waiting for it nor asking after a stop.
    pub(crate) fn turn_handed(mut self, turn: crate::sessions::Turn) -> Lent {
        self.turn = Turn::Handed(turn);
        self
    }

    /// A launch tries for the Turn rather than waiting, is refused with
    /// [`BUSY`] where it is taken, and holds it for as long as its session
    /// runs, ending the session once its Done signal is taken and it idles.
    ///
    /// What the wrap-up's dispatches always were. A fix session queued behind
    /// the review would be dispatched about a suite nobody has looked at since,
    /// and one not holding the Turn would be ended by the review or a batch of
    /// comments starting over it mid-commit — see
    /// [`crate::sessions::Sessions::try_turn`].
    pub(crate) fn trying(mut self) -> Lent {
        self.turn = Turn::Try;
        self
    }

    /// Wake whoever holds the other half of `launched` once the next launch is
    /// over, whichever way it went — or once this host is gone without one.
    /// What a press that runs its session under a component spawned elsewhere
    /// waits on, so that it answers with the session there, as it always has.
    pub(crate) fn telling(mut self, launched: tokio::sync::oneshot::Sender<()>) -> Lent {
        self.telling = Some(launched);
        self
    }

    /// Hold `driving`, the caller's registration, across the next launch and
    /// let it go once that is over — a grilling, which its session drives from
    /// then on until the human picks.
    pub(crate) fn launching(mut self, driving: Driving) -> Lent {
        self.launching = Some(driving);
        self
    }

    /// The registration a pick took, handed on to whatever the Direction sets
    /// going.
    pub(crate) fn driving(&mut self) -> Option<Driving> {
        self.picked.take()
    }

    /// Leave `sets` open across the launch rather than locking them — a
    /// grilling relaunched over an arrival. See [`crate::sessions::Held`].
    pub(crate) fn holding(mut self, sets: Vec<i64>) -> Lent {
        self.held = sets;
        self
    }

    /// What the sessions launched here are building, where they are.
    ///
    /// Two rules hang off it, and they are the one rule: a session that builds
    /// is the one told how to retire a Memo, and the kind is what a Memo it
    /// leaves is left under — see [`crate::memos`].
    pub(crate) fn building(mut self, building: Option<Building>) -> Lent {
        self.building = building;
        self
    }

    /// The Directions a session launched expecting `picked` offers: what its
    /// proposals are stamped with, and the only picks it hears. All three
    /// where nothing narrows them — see [`crate::followers`].
    pub(crate) fn offering(mut self, offers: Vec<Direction>) -> Lent {
        self.offers = offers;
        self
    }

    /// What the rescue says the session is doing — "following the work up".
    pub(crate) fn doing(mut self, doing: &str) -> Lent {
        self.doing = doing.to_owned();
        self
    }

    /// What the rescue is to say the session is doing, as [`Lent::doing`] last
    /// set it.
    pub(crate) fn doing_now(&self) -> &str {
        &self.doing
    }

    /// The same two, changed between one launch and the next — a component's
    /// launches, which each say what they build through the evidence they
    /// expect. See [`crate::activities`].
    pub(crate) fn now(&mut self, building: Option<Building>, doing: &str) {
        self.building = building;
        self.doing = doing.to_owned();
    }

    /// Launch a session through the world's *launch a session*, saying nothing
    /// of a refusal beyond what [`host::Host::launch`] has already logged.
    pub(crate) async fn start(
        &mut self,
        skill: &str,
        role: store::Role,
        prompt: String,
        evidence: Evidence,
        ends: Ends,
    ) -> Option<world::Session> {
        let launched = host::Host::launch(
            self,
            skill.to_owned(),
            role_named(role).to_owned(),
            prompt,
            lent_evidence(&evidence),
            lent_ends(ends),
        )
        .await;

        match launched {
            Ok(Ok(session)) => Some(session),
            Ok(Err(_)) | Err(_) => None,
        }
    }

    /// The Timeline Event `session` prints into.
    pub(crate) fn event_id(&self, session: world::Session) -> i64 {
        self.watched
            .get(&session)
            .map(|watched| watched.session.event_id)
            .expect("a session is only asked after by the handle it was launched under")
    }

    /// Wait on `session` through the world's *wait*, and say how it ended —
    /// `None` where it gave its Done signal, had it taken, and has been idle
    /// since.
    pub(crate) async fn see(&mut self, session: world::Session) -> Option<Ended> {
        match host::Host::wait(self, session).await {
            Ok(world::Waited::Done) => None,
            _ => Some(
                self.watched
                    .get(&session)
                    .and_then(|watched| watched.ended.clone())
                    .unwrap_or(Ended::Unknown),
            ),
        }
    }

    /// How `session` ended, once a wait has seen it end without its Done
    /// signal taken — `None` before then, and where the signal was taken.
    pub(crate) fn ended(&self, session: world::Session) -> Option<Ended> {
        self.watched
            .get(&session)
            .and_then(|watched| watched.ended.clone())
    }

    fn watch(
        &mut self,
        session: Session,
        expected: Expected,
        ends: Ends,
        turn: Option<crate::sessions::Turn>,
    ) -> world::Session {
        self.issued += 1;
        self.writing = Some(session.event_id);

        self.watched.insert(
            self.issued,
            Watched {
                session,
                expected,
                ends,
                ended: None,
                seeing: None,
                turn,
            },
        );

        self.issued
    }

    /// Wait on a session launched expecting `picked`: a grilling, whose Done
    /// signal is checked against nothing until the human picks a Direction on
    /// its proposal, and against that Direction's artifact from then on — the
    /// latest pick's, a later one moving the wait onto its own. See
    /// [`crate::runner::tail`] for which artifact each is.
    ///
    /// **Before a pick nothing is held to anything**: no signal is expected, so
    /// one is refused saying nothing has been picked, and nobody speaks to the
    /// session, whose questions are the work. Its ending is `gone` with how it
    /// went — nothing else is written, and a grilling session dying is left to
    /// the stall sweep as it always was.
    ///
    /// **After one, the session is seen out the way any step's is**: spoken to
    /// where it goes quiet without its artifact, and answered `picked` once it
    /// has signalled over it and been idle since — or once it ended, where the
    /// artifact was its last act. A session that ended without it stops the
    /// Conversation in the host's own sentence, which the wait then answers as
    /// `gone` with nothing to add.
    async fn wait_for_the_pick(
        &mut self,
        session: world::Session,
    ) -> wasmtime::Result<world::Waited> {
        let state = self.state.clone();
        let conversation_id = self.conversation_id;
        let pace = state.sessions.pace();

        let Some(watched) = self.watched.get_mut(&session) else {
            return Err(wasmtime::Error::msg(format!(
                "there is no session {session} to wait on"
            )));
        };

        let Expected::Picked(picks) = &mut watched.expected else {
            return Err(wasmtime::Error::msg(format!(
                "session {session} was not launched expecting a pick"
            )));
        };

        let event_id = watched.session.event_id;
        let idle = watched.session.idle.clone();
        let mut picked = *picks.borrow_and_update();

        let (ended, tail) = loop {
            let tail = match picked {
                Some(direction) => crate::runner::tail(&state, conversation_id, direction)
                    .await
                    .map(|tail| (direction, tail)),
                None => None,
            };

            // A grilling picked on is driven by this wait from now on, rather
            // than by its session alone: the moves the Direction makes go on
            // after the session is ended, and a roadmap's pull request moves
            // the Conversation on while it still runs. See [`crate::drivers`].
            if picked.is_some() && self.picked.is_none() {
                self.picked = Some(state.drivers.driving(conversation_id));
            }

            let Some((direction, tail)) = tail else {
                tokio::select! {
                    ended = watched.session.ended() => break (ended, None),
                    next = next_pick(picks) => {
                        picked = Some(next);
                        continue;
                    }
                }
            };

            let expecting = state.signals.expecting(
                conversation_id,
                event_id,
                tail.evidence.clone(),
                tail.ends,
                None,
            );
            let signal = expecting.signal();

            tokio::select! {
                ended = watched.session.ended() => break (ended, Some((direction, tail))),
                () = crate::runner::signalled_and_idle(signal.clone(), &idle, pace) => {
                    return Ok(world::Waited::Picked(direction.word().to_owned()));
                }
                next = next_pick(picks) => {
                    picked = Some(next);
                    continue;
                }
                never = crate::rescues::watched(
                    &state,
                    conversation_id,
                    event_id,
                    &idle,
                    pace,
                    signal,
                    &tail.doing,
                ) => match never {},
            }
        };

        watched.ended = Some(ended.clone());

        let Some((direction, tail)) = tail else {
            return Ok(self.gone(&ended, &Evidence::Nothing).await);
        };

        // It may have written the artifact as its last act and exited before a
        // poll caught it, which is the ordinary shape of a session that
        // finishes rather than idles — so the Worktree is asked once more,
        // whichever way it ended.
        if crate::runner::check(&tail.worktree, &tail.landing).await {
            return Ok(world::Waited::Picked(direction.word().to_owned()));
        }

        // Stopped or steered from outside, which has said so already.
        if ended.on_purpose() {
            return Ok(self.gone(&ended, &tail.evidence).await);
        }

        tracing::warn!(
            conversation_id,
            event_id,
            ?direction,
            "a grilling session ended without writing what the pick asked for, so the run stops here",
        );

        let how = ended
            .badly()
            .unwrap_or_else(|| "The session ended without finishing the step.".to_owned());

        crate::runner::stop(
            &state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            &tail.doing,
            &how,
            Some(event_id),
        )
        .await;

        Ok(world::Waited::Gone(None))
    }

    /// What a session that ended without its Done signal taken is to the one
    /// waiting on it.
    ///
    /// **Ended on purpose, with no stop standing, is the human taking the
    /// Conversation elsewhere** — a steer ends the session it moves past, and
    /// so does a transfer, after which the work carries on at the far end —
    /// so it is `steered`, and whatever was waiting has nothing left to do.
    /// Under a stop it is `gone` with nothing said, the stop's own Notice being
    /// the whole of it; otherwise `gone` carries how it ended, where that was
    /// the trouble — see [`Ended::badly`].
    ///
    /// **And a session whose rounds the human had already marked over is
    /// `marked`**, where its evidence is that mark: an agent that finishes its
    /// turn and exits before the grace runs out is the ordinary shape of a
    /// follow-up that is over, rather than one nobody is left to have. A Set
    /// still standing is asked about first — the human holding a question is
    /// worth a stop whatever the newest answer said — and both read the safe
    /// way round: a store that will not answer reads as open and not marked.
    async fn gone(&self, ended: &Ended, evidence: &Evidence) -> world::Waited {
        let state = &self.state;
        let conversation_id = self.conversation_id;

        if ended.on_purpose() {
            return match store::stopped(&state.pool, conversation_id).await {
                Ok(None) => world::Waited::Steered,
                Ok(Some(_)) => world::Waited::Gone(None),
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, "reading whether a Conversation had stopped under its session failed");
                    world::Waited::Gone(None)
                }
            };
        }

        let within = match evidence {
            Evidence::NothingElse => store::Lifecycle::FollowUp,
            Evidence::Investigated => store::Lifecycle::Investigating,
            _ => return world::Waited::Gone(ended.badly()),
        };

        if !crate::runner::open(state, conversation_id).await
            && crate::activities::follow_up_rounds::marked(state, conversation_id, within).await
        {
            return world::Waited::Marked;
        }

        world::Waited::Gone(ended.badly())
    }

    /// Wait on `sessions` for at most `timeout`: see the world's `wait-any`.
    ///
    /// **The newest is the one running**, a launch having ended whatever ran
    /// before it, so it alone has its Done signal checked and its silences
    /// rescued — by a [`Seeing`] that outlives the wait — and every one of them
    /// is waited on to end. One already seen to end answers at once, as `wait`
    /// answers it.
    async fn wait_any(
        &mut self,
        sessions: Vec<world::Session>,
        timeout: u64,
    ) -> wasmtime::Result<world::Woke> {
        for session in &sessions {
            match self.watched.get(session) {
                None => {
                    return Err(wasmtime::Error::msg(format!(
                        "there is no session {session} to wait on"
                    )));
                }
                Some(Watched {
                    expected: Expected::Picked(_),
                    ..
                }) => {
                    return Err(wasmtime::Error::msg(format!(
                        "session {session} was launched expecting a pick, so it is waited on alone"
                    )));
                }
                Some(Watched {
                    ended: Some(ended), ..
                }) => {
                    return Ok(world::Woke::Waited((
                        *session,
                        world::Waited::Gone(ended.badly()),
                    )));
                }
                Some(_) => {}
            }
        }

        let timeout = std::time::Duration::from_millis(timeout);

        // Nothing to wait on but the time, or a stop the host's watchers have
        // handed a wrap-up to answer, which is looked at on the next round.
        let Some(newest) = sessions.iter().max().copied() else {
            tokio::select! {
                () = tokio::time::sleep(timeout) => {}
                () = self.state.wrap_ups.handed() => {}
            }
            return Ok(world::Woke::Tick);
        };

        self.seeing(newest);

        let woke = {
            let mut endings = Vec::new();
            let mut signalled = None;

            for (handle, watched) in self.watched.iter_mut() {
                if !sessions.contains(handle) {
                    continue;
                }

                let Watched {
                    session, seeing, ..
                } = watched;

                if *handle == newest {
                    signalled = seeing.as_mut().map(|seeing| &mut seeing.task);
                }

                let ending: std::pin::Pin<Box<dyn Future<Output = Ended> + Send + '_>> =
                    Box::pin(session.ended());
                endings.push((*handle, ending));
            }

            let first_to_end = std::future::poll_fn(|context| {
                for (handle, ending) in endings.iter_mut() {
                    if let std::task::Poll::Ready(ended) = ending.as_mut().poll(context) {
                        return std::task::Poll::Ready((*handle, ended));
                    }
                }

                std::task::Poll::Pending
            });

            // A task that was aborted or panicked has given no signal.
            let signalled = async {
                match signalled {
                    Some(task) => {
                        if task.await.is_err() {
                            std::future::pending::<()>().await;
                        }
                    }
                    None => std::future::pending().await,
                }
            };

            tokio::select! {
                (handle, ended) = first_to_end => Some((handle, Some(ended))),
                () = signalled => Some((newest, None)),
                () = tokio::time::sleep(timeout) => None,
            }
        };

        let Some((handle, ended)) = woke else {
            return Ok(world::Woke::Tick);
        };

        let watched = self
            .watched
            .get_mut(&handle)
            .expect("it was found above, and nothing is taken out");
        watched.seeing = None;

        if let Some(ended) = &ended {
            watched.ended = Some(ended.clone());
        }

        let evidence = match &watched.expected {
            Expected::Evidence(evidence) => evidence.clone(),
            Expected::Picked(_) => unreachable!("a session expecting a pick was refused above"),
        };

        self.seen_out(handle, ended.is_none()).await;

        let Some(ended) = ended else {
            return Ok(world::Woke::Waited((handle, world::Waited::Done)));
        };

        Ok(world::Woke::Waited((
            handle,
            self.gone(&ended, &evidence).await,
        )))
    }

    /// Let go of the Turn `session` was launched holding, once a wait has seen
    /// it out — ending it first where it gave its Done signal and idles since,
    /// as a fix session dispatched by a wrap-up always was. See
    /// [`Lent::trying`].
    async fn seen_out(&mut self, session: world::Session, done: bool) {
        let Some(turn) = self
            .watched
            .get_mut(&session)
            .and_then(|watched| watched.turn.take())
        else {
            return;
        };

        if done {
            tracing::info!(
                conversation_id = self.conversation_id,
                "a session the wrap-up dispatched said it is done and has gone quiet since, so it \
                 is being ended",
            );

            self.state.sessions.end(self.conversation_id).await;
        }

        drop(turn);
    }

    /// Start `session`'s [`Seeing`], where it has none standing.
    fn seeing(&mut self, session: world::Session) {
        let state = self.state.clone();
        let conversation_id = self.conversation_id;
        let building = self.building;
        let doing = self.doing.clone();

        let Some(watched) = self.watched.get_mut(&session) else {
            return;
        };

        let Expected::Evidence(evidence) = &watched.expected else {
            return;
        };

        if watched.seeing.is_some() {
            return;
        }

        let event_id = watched.session.event_id;
        let idle = watched.session.idle.clone();
        let pace = state.sessions.pace();

        let expecting = state.signals.expecting(
            conversation_id,
            event_id,
            evidence.clone(),
            watched.ends,
            building,
        );
        let signal = expecting.signal();

        let task = tokio::spawn(async move {
            tokio::select! {
                () = crate::runner::signalled_and_idle(signal.clone(), &idle, pace) => {}
                never = crate::rescues::watched(
                    &state,
                    conversation_id,
                    event_id,
                    &idle,
                    pace,
                    signal,
                    &doing,
                ) => match never {},
            }
        });

        watched.seeing = Some(Seeing {
            _expecting: expecting,
            task,
        });
    }

    /// Whatever a launch is refused over is logged here, where the reason is
    /// known, and handed back as the sentence the world refuses with.
    fn refused(&self, why: &str) -> Result<world::Session, String> {
        tracing::error!(
            conversation_id = self.conversation_id,
            why,
            "no session was started"
        );

        Err(why.to_owned())
    }
}

impl world::Host for Lent {}

impl host::Host for Lent {
    /// Start a session the way every driver always started one: under the
    /// Pairing the role names, once the Turn is the caller's and nothing has
    /// stopped the run, with the Answers nobody has been told about folded under
    /// the prompt and the session before it ended — see [`crate::runner`]'s
    /// drivers, which say why each of those holds.
    async fn launch(
        &mut self,
        skill: String,
        role: String,
        prompt: String,
        evidence: world::Evidence,
        ends: world::Ends,
    ) -> wasmtime::Result<Result<world::Session, String>> {
        let state = self.state.clone();
        let conversation_id = self.conversation_id;

        // Let go however this returns: see [`Lent::telling`] and
        // [`Lent::launching`].
        let _telling = self.telling.take();
        let _launching = self.launching.take();

        if !named_in_a_pack(&skill) {
            return Ok(self.refused(&format!(
                "`{skill}` isn't a skill named as `<pack>/<skill>`."
            )));
        }

        let Some(role) = role_called(&role) else {
            return Ok(self.refused(&format!("There's no role called `{role}`.")));
        };

        // See [`Turn`]: held across the launch alone, and a stop asked after
        // the wait for it rather than before.
        let turn = match std::mem::replace(&mut self.turn, Turn::Held) {
            Turn::Take => {
                self.turn = Turn::Take;

                let turn = state.sessions.turn(conversation_id).await;

                if crate::stopping::stopped(&state, conversation_id).await {
                    return Ok(Err("The run was stopped, so nothing new starts.".to_owned()));
                }

                Some(turn)
            }
            Turn::Held => None,
            Turn::Handed(turn) => Some(turn),
            Turn::Try => {
                self.turn = Turn::Try;

                let Some(turn) = state.sessions.try_turn(conversation_id) else {
                    tracing::debug!(
                        conversation_id,
                        "something else is working in the Worktree, so nothing is launched yet",
                    );
                    return Ok(Err(BUSY.to_owned()));
                };

                if crate::stopping::stopped(&state, conversation_id).await {
                    return Ok(Err("The run was stopped, so nothing new starts.".to_owned()));
                }

                Some(turn)
            }
        };

        // Read back every time rather than held: where an agent is about to be
        // let loose is the one thing that must not be guessed at.
        let conversation = match store::load_conversation(&state.pool, conversation_id).await {
            Ok(Some(conversation)) => conversation,
            Ok(None) => return Ok(self.refused("There is no Conversation left to work in.")),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading the Conversation to work in failed");
                return Ok(self.refused("The Conversation couldn't be read."));
            }
        };

        let paired = match role {
            store::Role::Grilling => conversation.grilling_pairing.clone(),
            role => crate::runner::under(
                role,
                &conversation.review_pairing,
                conversation.implementation_pairing.as_ref(),
            ),
        };

        let Some(pairing) = paired else {
            return Ok(self.refused(&format!(
                "The Pairing the {} role runs under is gone.",
                role_named(role)
            )));
        };

        // A pick is listened for from before the session exists, so none on
        // its proposal can arrive with nothing to hear it.
        let expected = match evidence {
            world::Evidence::Picked => {
                Expected::Picked(state.followers.listen(conversation_id, self.offers.clone()))
            }
            evidence => match taken_evidence(evidence, conversation.worktree.as_deref()) {
                Some(evidence) => Expected::Evidence(evidence),
                None => {
                    return Ok(
                        self.refused("The Conversation has no Worktree left to land anything in.")
                    );
                }
            },
        };

        // And whatever the human has since answered on this Conversation's
        // Deferred Asks that no session has been told about — under everything
        // above. Never into a grilling, which is primed with every Set already:
        // see [`crate::deferrals`].
        let folding = match role {
            store::Role::Grilling => None,
            _ => Some(crate::deferrals::unfolded(&state, conversation_id).await),
        };

        let prompt = match &folding {
            Some(folding) => folding.under(&prompt),
            None => prompt,
        };

        // One Worktree holds one agent: a register still holding a relay that has
        // not finished unwinding would be two agents editing each other's files.
        state.sessions.end(conversation_id).await;

        let started = state
            .sessions
            .start(
                &state.pool,
                &state.nudges,
                state.devices.as_ref(),
                &conversation,
                &pairing,
                &prompt,
                self.building.is_some(),
                crate::sessions::Held {
                    sets: &self.held,
                    settlements: &state.settlements,
                },
            )
            .await;

        match started {
            Ok(Some(session)) => {
                // Once there is a session reading them, and only then: a launch
                // that came to nothing would otherwise cost the human the one
                // session their Answers were folded into.
                if let Some(folding) = folding {
                    folding.recorded(&state.pool).await;
                }

                self.folding = self.building.is_some();

                let held = matches!(self.turn, Turn::Try).then_some(turn).flatten();

                Ok(Ok(self.watch(session, expected, taken_ends(ends), held)))
            }
            Ok(None) => Ok(self.refused("The session couldn't be started.")),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "a session could not be started");
                Ok(self.refused("The session couldn't be started."))
            }
        }
    }

    /// Wait until the session gives its Done signal, has it taken and is idle
    /// since, or ends — what its signal is checked against written down for
    /// exactly as long as it is waited on, and taken away with the wait if the
    /// driver is cancelled. A session that goes idle with nothing open and no
    /// signal given is spoken to, and the human told where it will not answer:
    /// see [`crate::rescues`]. Never an ending of its own. A session that ends
    /// is `gone`, `steered` or `marked` as [`Lent::gone`] reads it.
    async fn wait(&mut self, session: world::Session) -> wasmtime::Result<world::Waited> {
        let conversation_id = self.conversation_id;

        let Some(watched) = self.watched.get_mut(&session) else {
            return Err(wasmtime::Error::msg(format!(
                "there is no session {session} to wait on"
            )));
        };

        if let Some(ended) = &watched.ended {
            return Ok(world::Waited::Gone(ended.badly()));
        }

        // Waited on here alone from now on: what a `wait-any` left checking it
        // would be a second check, and a second rescue.
        watched.seeing = None;

        let evidence = match &watched.expected {
            Expected::Evidence(evidence) => evidence.clone(),
            Expected::Picked(_) => return self.wait_for_the_pick(session).await,
        };

        let event_id = watched.session.event_id;
        let idle = watched.session.idle.clone();
        let pace = self.state.sessions.pace();

        let expecting = self.state.signals.expecting(
            conversation_id,
            event_id,
            evidence.clone(),
            watched.ends,
            self.building,
        );
        let signal = expecting.signal();

        let ended = tokio::select! {
            ended = watched.session.ended() => Some(ended),
            () = crate::runner::signalled_and_idle(signal.clone(), &idle, pace) => None,
            never = crate::rescues::watched(
                &self.state,
                conversation_id,
                event_id,
                &idle,
                pace,
                signal,
                &self.doing,
            ) => match never {},
        };

        drop(expecting);

        if let Some(ended) = &ended {
            watched.ended = Some(ended.clone());
        }

        self.seen_out(session, ended.is_none()).await;

        match ended {
            Some(ended) => Ok(self.gone(&ended, &evidence).await),
            None => Ok(world::Waited::Done),
        }
    }

    /// What the Conversation's own branch, or a Companion's, holds past its
    /// base, and whether anything in its Worktree is uncommitted — read-only, a
    /// rename followed first, as [`crate::runner`] reads its own branch.
    async fn git_facts(
        &mut self,
        repo: String,
    ) -> wasmtime::Result<Result<world::BranchFacts, String>> {
        let found = match self.found(&repo).await {
            Ok(found) => found,
            Err(why) => return Ok(Err(why)),
        };

        let Found {
            path,
            worktree,
            branch,
            base,
        } = found;

        let (Some(worktree), Some(base)) = (worktree, base) else {
            return Ok(Err(format!(
                "`{repo}` has no Worktree or no base commit to read against."
            )));
        };

        let read =
            tokio::task::spawn_blocking(move || read_branch(&path, &worktree, &base, branch)).await;

        Ok(match read {
            Ok(Some(facts)) => Ok(facts),
            Ok(None) | Err(_) => Err(format!("git wouldn't say what `{repo}`'s branch holds.")),
        })
    }

    /// Put `text` on the Timeline as a Notice, and tell open pages it is there.
    /// Put a notice on the Timeline — or, a run a look or a report brought
    /// about, wherever that says what it noticed: see [`Brought::noticed`].
    async fn notice(&mut self, text: String) -> wasmtime::Result<()> {
        let conversation_id = self.conversation_id;

        if self.brought != Brought::Settle {
            let waiting = self.waiting.contains(&text);

            self.brought
                .noticed(&self.state, conversation_id, &text, waiting)
                .await;

            return Ok(());
        }

        match store::note(&self.state.pool, conversation_id, &text).await {
            Ok(true) => self.state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            }),
            Ok(false) => {
                tracing::error!(
                    conversation_id,
                    "there is no Conversation left to put a notice on"
                )
            }
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "putting a notice on the Timeline failed")
            }
        }

        Ok(())
    }

    /// Stop where the Conversation stands, closing any Set left open first, with
    /// `reason` as what the stop's Notice says happened and the newest
    /// session's last words as its evidence.
    ///
    /// **Except a building session's Sets**, which stay standing: a backlog
    /// step's ask is an ordinary one, whose Answers open the prompt of the
    /// session the human's Resume starts — see [`crate::deferrals`] — as the
    /// runner always left one when it stopped over its step.
    async fn stop(&mut self, reason: String) -> wasmtime::Result<()> {
        if !self.folding {
            crate::activities::follow_up_rounds::left_open(&self.state, self.conversation_id).await;
        }

        crate::runner::stop(
            &self.state,
            self.conversation_id,
            crate::stopping::Decided::Verkstead,
            &self.doing,
            &reason,
            self.writing,
        )
        .await;

        Ok(())
    }

    /// The file at `path` as `repo` committed it at `at`, read by git rather
    /// than off the disk, so what a session has written and not committed is
    /// not there yet.
    async fn read_file(
        &mut self,
        repo: String,
        at: host::Revision,
        path: String,
    ) -> wasmtime::Result<Result<Option<Vec<u8>>, String>> {
        Ok(self
            .reading(repo, at, path, Reading::File)
            .await
            .map(|read| read.and_then(Read::file)))
    }

    /// The entries of the directory at `path` as `repo` committed it at `at`.
    async fn list_directory(
        &mut self,
        repo: String,
        at: host::Revision,
        path: String,
    ) -> wasmtime::Result<Result<Option<Vec<String>>, String>> {
        Ok(self
            .reading(repo, at, path, Reading::Directory)
            .await
            .map(|read| read.and_then(Read::directory)))
    }

    /// Place a backlog task whose line wants `platform`, as the runner always
    /// placed one before launching it — see [`crate::placing`]. Moving, or
    /// asking the human, is `elsewhere`; a word naming no platform is `here`.
    async fn place_task(
        &mut self,
        task: u32,
        label: String,
        platform: String,
    ) -> wasmtime::Result<host::Placed> {
        let Some(wants) = crate::declarations::matching(&platform) else {
            return Ok(host::Placed::Here);
        };

        let placed =
            crate::placing::wanting(&self.state, self.conversation_id, task, label, wants).await;

        Ok(match placed.here() {
            true => host::Placed::Here,
            false => host::Placed::Elsewhere,
        })
    }

    /// Hold the finish until the stage may join its roadmap's chain, and
    /// answer that chain, as the runner always held a stage's finish — see
    /// [`crate::joins::let_in`]. Not a stage, nothing is held or answered.
    async fn join_chain(&mut self) -> wasmtime::Result<Vec<host::Link>> {
        Ok(crate::joins::let_in(&self.state, self.conversation_id)
            .await
            .into_iter()
            .map(|link| host::Link {
                stage: link.stage,
                branch: link.branch,
            })
            .collect())
    }

    /// Start a roadmap stage as a Conversation of its own, as the carry-on
    /// starts each stage it finds ready — see [`crate::continuing::begin`],
    /// which holds it to the limits and says what it says.
    async fn start_conversation(
        &mut self,
        stage: host::Stage,
    ) -> wasmtime::Result<Result<host::Started, host::NotStarted>> {
        let (stage, base, stacks) = match staged(stage) {
            Ok(staged) => staged,
            Err(why) => return Ok(Err(host::NotStarted::Halted(why))),
        };

        let began = crate::continuing::begin(
            &self.state,
            self.conversation_id,
            stage,
            base,
            stacks,
            self.brought,
            self.spent,
        )
        .await;

        Ok(match began {
            Began::Started { id, branch } => Ok(host::Started {
                conversation: id,
                branch,
            }),
            Began::NoPlace { said, server } => {
                if server {
                    self.waiting.push(said.clone());
                }

                Err(host::NotStarted::NoPlace(said))
            }
            Began::Halted(why) => {
                self.spent += 1;

                Err(host::NotStarted::Halted(why))
            }
        })
    }

    /// Where `conversation` is, and whether it has settled, as the record
    /// reads a stage — one of this Conversation's Repo's.
    async fn conversation_state(
        &mut self,
        conversation: i64,
    ) -> wasmtime::Result<Result<host::Stands, String>> {
        let repo = match self.repo().await {
            Ok(repo) => repo,
            Err(why) => return Ok(Err(why)),
        };

        let read = store::conversation_standing(&self.state.pool, repo, conversation).await;

        Ok(match read {
            Ok(Some((state, standing))) => Ok(host::Stands {
                state: state.stored().to_owned(),
                settled: standing == store::StageStanding::Settled,
            }),
            Ok(None) => Err(format!(
                "There is no conversation {conversation} on this conversation's repo."
            )),
            Err(error) => {
                tracing::error!(error = ?error, conversation, "reading where a Conversation stands failed");
                Err(format!(
                    "Where conversation {conversation} is couldn't be read."
                ))
            }
        })
    }

    /// What the record holds of `roadmap` in this Conversation's Repo — see
    /// [`store::stage_standings`].
    async fn stage_standings(
        &mut self,
        roadmap: String,
    ) -> wasmtime::Result<Result<host::Standings, String>> {
        let repo = match self.repo().await {
            Ok(repo) => repo,
            Err(why) => return Ok(Err(why)),
        };

        let record = match store::stage_standings(&self.state.pool, repo).await {
            Ok(record) => record,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id = self.conversation_id, "reading what a roadmap's stages had got to failed");
                return Ok(Err(format!(
                    "What the `{roadmap}` roadmap's stages have got to couldn't be read."
                )));
            }
        };

        Ok(Ok(host::Standings {
            planner: record.planner(&roadmap).map(standing),
            stages: record
                .stages(&roadmap)
                .into_iter()
                .map(|(label, held, stopped, conversation)| host::StageStanding {
                    label: label.to_owned(),
                    standing: standing(held),
                    stopped,
                    conversation,
                })
                .collect(),
        }))
    }

    /// The stages of `roadmap` that have joined its chain, bottom to top —
    /// see [`store::stage_chain`].
    async fn stage_chain(
        &mut self,
        roadmap: String,
    ) -> wasmtime::Result<Result<Vec<host::Link>, String>> {
        let repo = match self.repo().await {
            Ok(repo) => repo,
            Err(why) => return Ok(Err(why)),
        };

        Ok(
            match store::stage_chain(&self.state.pool, repo, &roadmap).await {
                Ok(chain) => Ok(chain
                    .into_iter()
                    .map(|joined| host::Link {
                        stage: Some(joined.stage),
                        branch: joined.branch,
                    })
                    .collect()),
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id = self.conversation_id, "reading a roadmap's chain failed");
                    Err(format!("The `{roadmap}` roadmap's chain couldn't be read."))
                }
            },
        )
    }

    async fn wait_any(
        &mut self,
        sessions: Vec<world::Session>,
        timeout: u64,
    ) -> wasmtime::Result<world::Woke> {
        Lent::wait_any(self, sessions, timeout).await
    }

    async fn pull_requests(&mut self) -> wasmtime::Result<Vec<world::PullRequest>> {
        Ok(crate::forge::recorded(&self.state, self.conversation_id).await)
    }

    async fn forge_standing(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<world::PullRequestStanding, world::Unforged>> {
        Ok(crate::forge::standing(&self.state, self.conversation_id, pull_request).await)
    }

    async fn forge_checks(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<world::Suite, world::Unforged>> {
        Ok(crate::forge::checks(&self.state, self.conversation_id, pull_request).await)
    }

    async fn forge_comments(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<Vec<world::Comment>, world::Unforged>> {
        Ok(crate::forge::comments(&self.state, self.conversation_id, pull_request).await)
    }

    async fn forge_reply(
        &mut self,
        pull_request: world::PullRequest,
        markdown: String,
    ) -> wasmtime::Result<Result<String, world::Unforged>> {
        Ok(crate::forge::reply(&self.state, self.conversation_id, pull_request, markdown).await)
    }

    async fn forge_opened(
        &mut self,
        repo: String,
        branch: String,
    ) -> wasmtime::Result<Result<Option<world::Opened>, world::Unforged>> {
        Ok(crate::forge::opened(&self.state, self.conversation_id, repo, branch).await)
    }

    async fn wrap_up(&mut self) -> wasmtime::Result<world::Wrapped> {
        use crate::settling::Standing;

        Ok(
            match crate::settling::standing(&self.state, self.conversation_id).await {
                Standing::Waiting => world::Wrapped::Waiting,
                Standing::Settled => world::Wrapped::Settled,
                Standing::GivenUp => world::Wrapped::GivenUp,
                Standing::Over => world::Wrapped::Over,
            },
        )
    }

    /// Record what the `submitting` session launched here opened, as a
    /// take-up over a bare branch always has: the session ended where it said
    /// it was done and is idle, GitHub asked, and the pull request written
    /// beside the wrap-up with the host's watchers started over it — or the
    /// run stopped over what the session last said. See
    /// [`crate::wrapping::opened_beside`].
    async fn record_opened(&mut self) -> wasmtime::Result<Result<world::PullRequest, String>> {
        let conversation_id = self.conversation_id;

        self.state.sessions.end(conversation_id).await;

        crate::wrapping::opened_beside(&self.state, conversation_id, self.writing).await;

        let own = match store::load_conversation(&self.state.pool, conversation_id).await {
            Ok(Some(conversation)) => conversation.repo.name,
            Ok(None) => return Ok(Err("There is no conversation left.".to_owned())),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading the Conversation a pull request was opened for failed");
                return Ok(Err("The conversation couldn't be read.".to_owned()));
            }
        };

        Ok(crate::forge::recorded(&self.state, conversation_id)
            .await
            .into_iter()
            .find(|pull_request| pull_request.repo == own)
            .ok_or_else(|| "No pull request was opened.".to_owned()))
    }

    /// Settle what `watcher` waits on, in the store the viewer draws the
    /// wrap-up's standing from (Set 1838, Q3): the host's record, not the
    /// Activity's memory.
    async fn settle(&mut self, watcher: world::Watcher) -> wasmtime::Result<()> {
        let conversation_id = self.conversation_id;

        if let Some(waiting_on) = self.waiting_on(&watcher).await
            && let Err(error) =
                store::settle_wrap_up(&self.state.pool, conversation_id, waiting_on).await
        {
            tracing::error!(error = ?error, conversation_id, ?waiting_on, "settling something a wrap-up waits on failed");
        }

        Ok(())
    }

    async fn unsettle(&mut self, watcher: world::Watcher) -> wasmtime::Result<()> {
        let conversation_id = self.conversation_id;

        if let Some(waiting_on) = self.waiting_on(&watcher).await
            && let Err(error) =
                store::unsettle_wrap_up(&self.state.pool, conversation_id, waiting_on).await
        {
            tracing::error!(error = ?error, conversation_id, ?waiting_on, "putting something a wrap-up waits on back to waiting failed");
        }

        Ok(())
    }

    async fn fix_attempts(
        &mut self,
        pull_request: world::PullRequest,
        check: String,
    ) -> wasmtime::Result<Result<i64, String>> {
        let conversation_id = self.conversation_id;
        let named = match crate::forge::named(&self.state, conversation_id, &pull_request).await {
            Ok(named) => named,
            Err(why) => return Ok(Err(why)),
        };

        Ok(store::fix_attempts(
            &self.state.pool,
            conversation_id,
            named.repo.id,
            named.number,
            &check,
        )
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, conversation_id, check, "reading a check's fix attempts failed");
            "The fix attempts couldn't be read.".to_owned()
        }))
    }

    async fn count_fix_attempt(
        &mut self,
        pull_request: world::PullRequest,
        check: String,
    ) -> wasmtime::Result<Result<i64, String>> {
        let conversation_id = self.conversation_id;
        let named = match crate::forge::named(&self.state, conversation_id, &pull_request).await {
            Ok(named) => named,
            Err(why) => return Ok(Err(why)),
        };

        Ok(store::record_fix_attempt(
            &self.state.pool,
            conversation_id,
            named.repo.id,
            named.number,
            &check,
        )
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, conversation_id, check, "counting a check's fix attempt failed");
            "The fix attempt couldn't be counted.".to_owned()
        }))
    }

    async fn addressed(&mut self, repo: String) -> wasmtime::Result<Result<Vec<String>, String>> {
        let conversation_id = self.conversation_id;
        let repo = match crate::forge::repo_named(&self.state, conversation_id, &repo).await {
            Ok(repo) => repo,
            Err(why) => return Ok(Err(why)),
        };

        Ok(
            store::addressed_comments(&self.state.pool, conversation_id, repo.id)
                .await
                .map_err(|error| {
                    tracing::error!(error = ?error, conversation_id, repo = repo.name, "reading which comments were addressed failed");
                    "Which comments were addressed couldn't be read.".to_owned()
                }),
        )
    }

    async fn mark_addressed(
        &mut self,
        repo: String,
        comments: Vec<String>,
    ) -> wasmtime::Result<Result<(), String>> {
        let conversation_id = self.conversation_id;
        let repo = match crate::forge::repo_named(&self.state, conversation_id, &repo).await {
            Ok(repo) => repo,
            Err(why) => return Ok(Err(why)),
        };

        Ok(store::record_addressed_comments(
            &self.state.pool,
            conversation_id,
            repo.id,
            &comments,
        )
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, conversation_id, repo = repo.name, "marking comments addressed failed");
            "The comments couldn't be marked addressed.".to_owned()
        }))
    }

    async fn pushed_head(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Option<String>> {
        Ok(crate::checks::pushed(&self.state, self.conversation_id, pull_request).await)
    }

    async fn conflict_attempts(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<i64, String>> {
        Ok(crate::checks::conflict_attempts(&self.state, self.conversation_id, pull_request).await)
    }

    async fn count_conflict_attempt(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<i64, String>> {
        Ok(
            crate::checks::count_conflict_attempt(&self.state, self.conversation_id, pull_request)
                .await,
        )
    }

    async fn resolution(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<String, String>> {
        Ok(crate::checks::resolution(
            &self.state,
            self.conversation_id,
            pull_request,
            self.writing,
        )
        .await)
    }

    /// Give the wrap-up up with the tail of the newest session launched here,
    /// which is the last fix or resolution the component dispatched.
    async fn give_up(
        &mut self,
        pull_request: world::PullRequest,
        what: String,
        how: String,
    ) -> wasmtime::Result<bool> {
        Ok(crate::checks::give_up(
            &self.state,
            self.conversation_id,
            pull_request,
            what,
            how,
            self.writing,
        )
        .await)
    }

    async fn ignored(
        &mut self,
        repo: String,
        comments: Vec<world::Comment>,
    ) -> wasmtime::Result<Vec<String>> {
        let conversation_id = self.conversation_id;
        let Ok(repo) = crate::forge::repo_named(&self.state, conversation_id, &repo).await else {
            return Ok(Vec::new());
        };

        let said: Vec<crate::forges::Comment> = comments
            .into_iter()
            .map(|comment| crate::forges::Comment {
                which: comment.which,
                author: comment.author,
                at: comment.at,
                about: comment.about,
                markdown: comment.markdown,
            })
            .collect();

        Ok(crate::comments::not_to_answer(&self.state, conversation_id, &repo, &said).await)
    }

    /// Whether `watcher` is settled — a store that will not answer reading as
    /// *not yet*, which is the right way round for what it decides: on the
    /// other side of it is an agent let loose on feedback nobody was asked
    /// about.
    async fn settled(&mut self, watcher: world::Watcher) -> wasmtime::Result<bool> {
        let conversation_id = self.conversation_id;

        let Some(waiting_on) = self.waiting_on(&watcher).await else {
            return Ok(false);
        };

        Ok(
            match store::wrap_up_settled(&self.state.pool, conversation_id).await {
                Ok(settled) => settled.contains(&waiting_on),
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, ?waiting_on, "reading what a wrap-up had settled failed");
                    false
                }
            },
        )
    }

    async fn cover(&mut self) -> wasmtime::Result<()> {
        crate::wrapping::covering(self.state.clone(), self.conversation_id).await;
        Ok(())
    }

    async fn review_wanted(&mut self, afresh: bool) -> wasmtime::Result<bool> {
        Ok(crate::review::wanted_now(&self.state, self.conversation_id, afresh).await)
    }

    async fn reviewing(
        &mut self,
        said: Option<String>,
    ) -> wasmtime::Result<Result<String, String>> {
        Ok(crate::review::prompt(&self.state, self.conversation_id, said).await)
    }

    /// See the review session out, with the tail of the newest session
    /// launched here — the review itself — where it ended badly.
    async fn review_over(&mut self, how: Option<String>) -> wasmtime::Result<()> {
        let ended_badly = how.map(|how| (how, self.writing.unwrap_or_default()));

        crate::review::over(&self.state, self.conversation_id, ended_badly).await;
        Ok(())
    }

    /// The same for a batch session.
    async fn batch_over(
        &mut self,
        pull_request: world::PullRequest,
        comments: Vec<String>,
        how: Option<String>,
    ) -> wasmtime::Result<()> {
        let conversation_id = self.conversation_id;
        let named = match crate::forge::named(&self.state, conversation_id, &pull_request).await {
            Ok(named) => named,
            Err(why) => {
                tracing::error!(
                    conversation_id,
                    why,
                    "a batch session was over on a pull request the Conversation isn't on"
                );
                return Ok(());
            }
        };

        let ended_badly = how.map(|how| (how, self.writing.unwrap_or_default()));

        crate::responding::batch_over(
            &self.state,
            conversation_id,
            named.repo.id,
            named.number,
            &comments,
            ended_badly,
        )
        .await;

        Ok(())
    }

    async fn left_behind(&mut self) -> wasmtime::Result<bool> {
        Ok(crate::responding::unattended(&self.state, self.conversation_id).await)
    }
}

impl Lent {
    /// The id of this Conversation's Repo, or a sentence saying it couldn't
    /// be read.
    async fn repo(&self) -> Result<i64, String> {
        match store::load_conversation(&self.state.pool, self.conversation_id).await {
            Ok(Some(conversation)) => Ok(conversation.repo.id),
            Ok(None) => Err("There is no Conversation left to read.".to_owned()),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id = self.conversation_id, "reading the Conversation for its Repo failed");
                Err("The Conversation couldn't be read.".to_owned())
            }
        }
    }
}

/// What the record's standing is called in the world.
fn standing(standing: store::StageStanding) -> host::Standing {
    match standing {
        store::StageStanding::Settled => host::Standing::Settled,
        store::StageStanding::InFlight => host::Standing::InFlight,
        store::StageStanding::Abandoned => host::Standing::Abandoned,
    }
}

/// A stage an Activity asked to start, read into the host's: the stage, the
/// branch it is cut from, and whether the Repo records stacking. Refused, in a
/// sentence, where its roadmap is no directory name or its brief is not inside
/// that roadmap's directory, since the branch is named for both.
fn staged(stage: host::Stage) -> Result<(crate::stages::Stage, String, bool), String> {
    let host::Stage {
        roadmap,
        label,
        title,
        brief_path,
        brief,
        base,
        platform,
        stacks,
    } = stage;

    let named =
        !roadmap.is_empty() && roadmap != "." && roadmap != ".." && !roadmap.contains(['/', '\\']);

    if !named {
        return Err(format!(
            "`{roadmap}` isn't a roadmap's directory name, so no stage of it can start."
        ));
    }

    let within = format!("{}/{roadmap}/", crate::stages::ROADMAPS);
    let file = brief_path.strip_prefix(&within).unwrap_or_default();

    if file.is_empty() || file.contains('/') || inside(&brief_path).is_none() {
        return Err(format!(
            "Stage {label} of the `{roadmap}` roadmap names `{brief_path}` as its brief, which \
             isn't a file in `{within}`, so it can't start."
        ));
    }

    if label.is_empty() || base.is_empty() {
        return Err(format!(
            "A stage of the `{roadmap}` roadmap needs its number and the branch it's cut from \
             to start."
        ));
    }

    let stage = crate::stages::Stage {
        roadmap,
        label,
        title,
        brief_path,
        brief,
        platform: crate::declarations::matching(&platform),
    };

    Ok((stage, base, stacks))
}

/// A Repo of the Conversation's, found by name: where it is, its Worktree, and
/// what its branch was cut from.
struct Found {
    path: PathBuf,
    worktree: Option<PathBuf>,
    branch: String,
    base: Option<String>,
}

impl Lent {
    /// The Conversation's own Repo, or the Companion called `repo` — a rename
    /// of the Conversation's branch followed first, as [`crate::runner`] reads
    /// its own branch. A sentence saying why, where it is neither.
    async fn found(&self, repo: &str) -> Result<Found, String> {
        let state = &self.state;
        let conversation_id = self.conversation_id;

        let conversation = match store::load_conversation(&state.pool, conversation_id).await {
            Ok(Some(conversation)) => conversation,
            Ok(None) => return Err("There is no Conversation left to read.".to_owned()),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading the Conversation for one of its Repos failed");
                return Err("The Conversation couldn't be read.".to_owned());
            }
        };

        if conversation.repo.name == repo {
            let branch = match conversation.worktree.as_deref() {
                Some(worktree) => crate::renames::follow(
                    &state.pool,
                    conversation_id,
                    &conversation.repo.path,
                    worktree,
                    &conversation.branch,
                )
                .await
                .unwrap_or_else(|| conversation.branch.clone()),
                None => conversation.branch.clone(),
            };

            Ok(Found {
                path: conversation.repo.path.clone(),
                worktree: conversation.worktree.clone(),
                branch,
                base: conversation.base_commit.clone(),
            })
        } else if let Some(companion) = conversation
            .companions
            .iter()
            .find(|companion| companion.repo.name == repo)
        {
            Ok(Found {
                path: companion.repo.path.clone(),
                worktree: companion.worktree.clone(),
                branch: companion
                    .branch_for(&conversation.branch)
                    .unwrap_or_else(|| companion.branch.clone()),
                base: companion.base_commit.clone(),
            })
        } else {
            Err(format!(
                "`{repo}` is neither this Conversation's repo nor one of its companions."
            ))
        }
    }

    /// `path` in `repo` at `at`, read as `reading` says, off the runtime's
    /// threads: the Worktree's HEAD read in the Worktree, which has a HEAD of
    /// its own, and a named revision in the Repo, where every branch is.
    async fn reading(
        &self,
        repo: String,
        at: host::Revision,
        path: String,
        reading: Reading,
    ) -> Result<Option<Read>, String> {
        let found = self.found(&repo).await?;

        let (dir, revision) = match at {
            host::Revision::Head => match found.worktree {
                Some(worktree) => (worktree, "HEAD".to_owned()),
                None => return Err(format!("`{repo}` has no Worktree to read the HEAD of.")),
            },
            host::Revision::Named(revision) => (found.path, revision),
        };

        let named = repo.clone();
        tokio::task::spawn_blocking(move || read_at(&dir, &named, &revision, &path, reading))
            .await
            .unwrap_or_else(|_| Err(format!("git wouldn't read `{repo}`.")))
    }
}

/// Which of the two reads a [`read_at`] is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reading {
    File,
    Directory,
}

/// What a [`read_at`] found, which is what it was asked for.
#[derive(Debug, PartialEq, Eq)]
enum Read {
    File(Vec<u8>),
    Directory(Vec<String>),
}

impl Read {
    fn file(self) -> Option<Vec<u8>> {
        match self {
            Read::File(bytes) => Some(bytes),
            Read::Directory(_) => None,
        }
    }

    fn directory(self) -> Option<Vec<String>> {
        match self {
            Read::Directory(names) => Some(names),
            Read::File(_) => None,
        }
    }
}

/// `path` at `revision` in the repository at `dir` — called `repo` in what it
/// says — as `reading` asks: `None` where nothing is there, and a sentence
/// where the revision names no commit, the path leads out of the Repo, or what
/// is there is the other of a file and a directory.
///
/// The revision is resolved to its commit first, so that one naming nothing is
/// told apart from a path naming nothing, and every git here is run with
/// `--no-optional-locks` by [`crate::repos::bytes`], so a read never trips a
/// session's own `git add`.
fn read_at(
    dir: &Path,
    repo: &str,
    revision: &str,
    path: &str,
    reading: Reading,
) -> Result<Option<Read>, String> {
    let Some(path) = inside(path) else {
        return Err(format!("`{path}` leads out of `{repo}`."));
    };

    let commit = crate::repos::git(
        dir,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &format!("{revision}^{{commit}}"),
        ],
    )
    .map(|commit| commit.trim().to_owned())
    .filter(|commit| !commit.is_empty())
    .ok_or_else(|| format!("`{revision}` isn't a branch or a commit of `{repo}`."))?;

    let object = format!("{commit}:{path}");
    let Some(kind) = crate::repos::git(dir, &["cat-file", "-t", "--end-of-options", &object])
    else {
        return Ok(None);
    };

    let shown = if path.is_empty() { "." } else { &path };
    match (kind.trim(), reading) {
        ("blob", Reading::File) => crate::repos::bytes(
            dir,
            &["cat-file", "blob", "--end-of-options", &object],
            &[0],
        )
        .map(|bytes| Some(Read::File(bytes)))
        .ok_or_else(|| format!("git wouldn't read `{shown}` of `{repo}`.")),
        ("tree", Reading::Directory) => crate::repos::bytes(
            dir,
            &["ls-tree", "-z", "--name-only", "--end-of-options", &object],
            &[0],
        )
        .map(|listed| {
            Some(Read::Directory(
                listed
                    .split(|byte| *byte == 0)
                    .filter(|name| !name.is_empty())
                    .map(|name| String::from_utf8_lossy(name).into_owned())
                    .collect(),
            ))
        })
        .ok_or_else(|| format!("git wouldn't list `{shown}` of `{repo}`.")),
        ("tree", Reading::File) => Err(format!("`{shown}` is a directory in `{repo}`.")),
        ("blob", Reading::Directory) => Err(format!("`{shown}` is a file in `{repo}`.")),
        // A submodule's commit, say: nothing a read can answer with.
        _ => Ok(None),
    }
}

/// `path` said from the Repo's root as git names it — `.` and empty pieces
/// dropped, `""` the root itself — or `None` where it is absolute or climbs
/// out with a `..`.
fn inside(path: &str) -> Option<String> {
    if path.starts_with('/') {
        return None;
    }

    let mut pieces = Vec::new();
    for piece in path.split('/') {
        match piece {
            "" | "." => {}
            ".." => return None,
            piece => pieces.push(piece),
        }
    }

    Some(pieces.join("/"))
}

impl Lent {
    /// What `watcher` is in the store's words — `None`, said in the log, where
    /// it names a pull request in a Repo the Conversation is not in.
    async fn waiting_on(&self, watcher: &world::Watcher) -> Option<store::WaitingOn> {
        let (pull_request, waiting_on): (_, fn(i64, i64) -> store::WaitingOn) = match watcher {
            world::Watcher::Review => return Some(store::WaitingOn::Review),
            world::Watcher::Checks(pull_request) => (pull_request, |repo_id, number| {
                store::WaitingOn::Checks { repo_id, number }
            }),
            world::Watcher::Comments(pull_request) => (pull_request, |repo_id, number| {
                store::WaitingOn::Comments { repo_id, number }
            }),
            world::Watcher::Mergeable(pull_request) => (pull_request, |repo_id, number| {
                store::WaitingOn::Mergeable { repo_id, number }
            }),
        };

        match crate::forge::named(&self.state, self.conversation_id, pull_request).await {
            Ok(named) => Some(waiting_on(named.repo.id, named.number)),
            Err(why) => {
                tracing::error!(
                    conversation_id = self.conversation_id,
                    why,
                    "a watcher named a pull request the Conversation isn't on"
                );
                None
            }
        }
    }
}

/// The branch's facts, by git, off the runtime's threads. `None` where git
/// would not answer.
fn read_branch(
    repo: &Path,
    worktree: &Path,
    base: &str,
    branch: String,
) -> Option<world::BranchFacts> {
    let counted = crate::repos::git(
        repo,
        &[
            "rev-list",
            "--count",
            "--end-of-options",
            &format!("{base}..{branch}"),
        ],
    )?;

    let commits = counted.trim().parse().ok()?;
    let uncommitted = !crate::diffs::changed(worktree)?.is_empty();

    Some(world::BranchFacts {
        branch,
        commits,
        uncommitted,
    })
}

/// The next Direction the human picks, however many listeners ago; never, once
/// the channel is another interview's.
async fn next_pick(picks: &mut watch::Receiver<Option<Direction>>) -> Direction {
    loop {
        if picks.changed().await.is_err() {
            std::future::pending::<()>().await;
        }

        if let Some(direction) = *picks.borrow_and_update() {
            return direction;
        }
    }
}

/// Whether `skill` is named `<pack>/<skill>`, as stage 07 names every skill.
fn named_in_a_pack(skill: &str) -> bool {
    skill
        .split_once('/')
        .is_some_and(|(pack, name)| !pack.is_empty() && !name.is_empty() && !name.contains('/'))
}

/// A role as the world spells it, which is as the store does.
pub(crate) fn role_named(role: store::Role) -> &'static str {
    match role {
        store::Role::Grilling => "grilling",
        store::Role::Implementation => "implementation",
        store::Role::Review => "review",
    }
}

fn role_called(role: &str) -> Option<store::Role> {
    [
        store::Role::Grilling,
        store::Role::Implementation,
        store::Role::Review,
    ]
    .into_iter()
    .find(|named| role_named(*named) == role)
}

/// `evidence` in the world's words. A landing's Worktree goes unsaid: the host
/// reads it off the Conversation as the session is launched.
fn lent_evidence(evidence: &Evidence) -> world::Evidence {
    match evidence {
        Evidence::Landed { landing, .. } => world::Evidence::Landed(match landing {
            Landing::Gone(path) => world::Landing::Gone(path.to_string_lossy().into_owned()),
            Landing::Arrived(path) => world::Landing::Arrived(path.to_string_lossy().into_owned()),
            Landing::Ticked(number) => world::Landing::Ticked(*number),
            Landing::Roadmap(base) => world::Landing::Roadmap(base.clone()),
            Landing::Handoff(path) => world::Landing::Handoff(path.to_string_lossy().into_owned()),
        }),
        Evidence::Committed { already } => world::Evidence::CommittedSince(*already),
        Evidence::Nothing => world::Evidence::Nothing,
        Evidence::NothingElse => world::Evidence::NothingElse,
        Evidence::Investigated => world::Evidence::Investigated,
    }
}

/// `evidence` as the **One session** Activity's `evidence` option spells it: a
/// bare word, or a word, a colon and what it is of — `committed-since:3`. A
/// landing's Worktree goes unsaid, as it does across the world.
pub(crate) fn evidence_option(evidence: &Evidence) -> String {
    match lent_evidence(evidence) {
        world::Evidence::Landed(landing) => match landing {
            world::Landing::Gone(path) => format!("gone:{path}"),
            world::Landing::Arrived(path) => format!("arrived:{path}"),
            world::Landing::Ticked(number) => format!("ticked:{number}"),
            world::Landing::Roadmap(base) => format!("roadmap:{base}"),
            world::Landing::Handoff(path) => format!("handoff:{path}"),
        },
        world::Evidence::CommittedSince(already) => format!("committed-since:{already}"),
        world::Evidence::Nothing => "nothing".to_owned(),
        world::Evidence::NothingElse => "nothing-else".to_owned(),
        world::Evidence::Investigated => "investigated".to_owned(),
        world::Evidence::Picked => "picked".to_owned(),
    }
}

/// `ends` as the **One session** Activity's `ends` option spells it.
pub(crate) fn ends_option(ends: Ends) -> &'static str {
    match ends {
        Ends::WithItsWork => "with-its-work",
        Ends::OnAPullRequest => "on-a-pull-request",
    }
}

/// The world's evidence as [`crate::done`] checks it, a landing in `worktree` —
/// `None` where it is a landing and there is no Worktree to land it in, and for
/// `picked`, which is no evidence until there is a pick: see
/// [`Lent::wait_for_the_pick`].
fn taken_evidence(evidence: world::Evidence, worktree: Option<&Path>) -> Option<Evidence> {
    Some(match evidence {
        world::Evidence::Landed(landing) => Evidence::Landed {
            worktree: worktree?.to_owned(),
            landing: match landing {
                world::Landing::Gone(path) => Landing::Gone(PathBuf::from(path)),
                world::Landing::Arrived(path) => Landing::Arrived(PathBuf::from(path)),
                world::Landing::Ticked(number) => Landing::Ticked(number),
                world::Landing::Roadmap(base) => Landing::Roadmap(base),
                world::Landing::Handoff(path) => Landing::Handoff(PathBuf::from(path)),
            },
        },
        world::Evidence::CommittedSince(already) => Evidence::Committed { already },
        world::Evidence::Nothing => Evidence::Nothing,
        world::Evidence::NothingElse => Evidence::NothingElse,
        world::Evidence::Investigated => Evidence::Investigated,
        world::Evidence::Picked => return None,
    })
}

fn lent_ends(ends: Ends) -> world::Ends {
    match ends {
        Ends::WithItsWork => world::Ends::WithItsWork,
        Ends::OnAPullRequest => world::Ends::OnAPullRequest,
    }
}

fn taken_ends(ends: world::Ends) -> Ends {
    match ends {
        world::Ends::WithItsWork => Ends::WithItsWork,
        world::Ends::OnAPullRequest => Ends::OnAPullRequest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_is_named_in_a_pack() {
        assert!(named_in_a_pack("verkstead/following-up"));
        assert!(!named_in_a_pack("following-up"));
        assert!(!named_in_a_pack("verkstead/"));
        assert!(!named_in_a_pack("/following-up"));
        assert!(!named_in_a_pack("verkstead/following-up/SKILL.md"));
    }

    #[test]
    fn a_role_is_called_what_the_store_calls_it() {
        for role in [
            store::Role::Grilling,
            store::Role::Implementation,
            store::Role::Review,
        ] {
            assert_eq!(role_called(role_named(role)), Some(role));
        }

        assert_eq!(role_called("reviewer"), None);
    }

    /// Every piece of evidence a driver registers crosses the world and comes
    /// back as it went, its landing in the Worktree the host reads.
    #[test]
    fn evidence_crosses_the_world_and_back() {
        let worktree = Path::new("/work");

        for evidence in [
            Evidence::Landed {
                worktree: worktree.to_owned(),
                landing: Landing::Ticked(3),
            },
            Evidence::Landed {
                worktree: worktree.to_owned(),
                landing: Landing::Arrived(PathBuf::from(".tasks/TODO.md")),
            },
            Evidence::Landed {
                worktree: worktree.to_owned(),
                landing: Landing::Handoff(PathBuf::from("/data/handoffs/4.md")),
            },
            Evidence::Committed { already: 7 },
            Evidence::Nothing,
            Evidence::NothingElse,
            Evidence::Investigated,
        ] {
            let crossed = taken_evidence(lent_evidence(&evidence), Some(worktree)).unwrap();

            assert_eq!(format!("{crossed:?}"), format!("{evidence:?}"));
        }

        assert!(taken_evidence(world::Evidence::Landed(world::Landing::Ticked(1)), None).is_none());
    }

    /// Run git in `dir`, where a test is building the repository it reads.
    fn run(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(args)
            .current_dir(dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    /// A repository with `.tasks/TODO.md` committed on `main`, a branch
    /// `elsewhere` holding one more file, and the list changed on disk and not
    /// committed.
    fn repository() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        run(path, &["init", "--quiet", "--initial-branch=main"]);
        std::fs::create_dir(path.join(".tasks")).unwrap();
        std::fs::write(path.join(".tasks/TODO.md"), "- [ ] 01: One\n").unwrap();
        std::fs::write(path.join(".tasks/01-one.md"), "# One\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "--quiet", "-m", "plan"]);

        run(path, &["switch", "--quiet", "-c", "elsewhere"]);
        std::fs::write(path.join("note.md"), "elsewhere\n").unwrap();
        run(path, &["add", "-A"]);
        run(path, &["commit", "--quiet", "-m", "note"]);
        run(path, &["switch", "--quiet", "main"]);

        std::fs::write(path.join(".tasks/TODO.md"), "- [x] 01: One\n").unwrap();

        dir
    }

    #[test]
    fn a_read_answers_what_is_committed_and_not_what_is_written() {
        let dir = repository();

        assert_eq!(
            read_at(dir.path(), "app", "HEAD", ".tasks/TODO.md", Reading::File),
            Ok(Some(Read::File(b"- [ ] 01: One\n".to_vec()))),
        );
        assert_eq!(
            read_at(dir.path(), "app", "elsewhere", "./note.md", Reading::File),
            Ok(Some(Read::File(b"elsewhere\n".to_vec()))),
        );
        assert_eq!(
            read_at(dir.path(), "app", "main", "note.md", Reading::File),
            Ok(None),
            "a path that is not there is none",
        );
    }

    #[test]
    fn a_listing_names_a_directorys_entries() {
        let dir = repository();

        assert_eq!(
            read_at(dir.path(), "app", "HEAD", ".tasks/", Reading::Directory),
            Ok(Some(Read::Directory(vec![
                "01-one.md".to_owned(),
                "TODO.md".to_owned()
            ]))),
        );
        assert_eq!(
            read_at(dir.path(), "app", "elsewhere", "", Reading::Directory),
            Ok(Some(Read::Directory(vec![
                ".tasks".to_owned(),
                "note.md".to_owned()
            ]))),
            "the root is the empty path",
        );
        assert_eq!(
            read_at(dir.path(), "app", "HEAD", "docs", Reading::Directory),
            Ok(None),
        );
    }

    #[test]
    fn a_read_is_refused_in_a_sentence() {
        let dir = repository();

        for (revision, path, reading, said) in [
            (
                "nowhere",
                "TODO.md",
                Reading::File,
                "`nowhere` isn't a branch or a commit of `app`.",
            ),
            (
                "--output=x",
                "TODO.md",
                Reading::File,
                "`--output=x` isn't a branch or a commit of `app`.",
            ),
            (
                "HEAD",
                "../outside",
                Reading::File,
                "`../outside` leads out of `app`.",
            ),
            (
                "HEAD",
                "/etc/passwd",
                Reading::File,
                "`/etc/passwd` leads out of `app`.",
            ),
            (
                "HEAD",
                ".tasks",
                Reading::File,
                "`.tasks` is a directory in `app`.",
            ),
            (
                "HEAD",
                ".tasks/TODO.md",
                Reading::Directory,
                "`.tasks/TODO.md` is a file in `app`.",
            ),
        ] {
            assert_eq!(
                read_at(dir.path(), "app", revision, path, reading),
                Err(said.to_owned()),
            );
        }
    }

    #[test]
    fn a_path_is_said_from_the_root() {
        assert_eq!(
            inside("./docs//roadmaps/"),
            Some("docs/roadmaps".to_owned())
        );
        assert_eq!(inside(""), Some(String::new()));
        assert_eq!(inside("docs/../.."), None);
        assert_eq!(inside("/docs"), None);
    }
}
