//! An Activity's component, started on a Conversation and run to its Outcome
//! (ADR-0030, *Activity*).
//!
//! **The same host as every in-process driver.** The component's imports are
//! linked to a [`Lent`], so a session it launches is launched, waited on and
//! checked exactly as one of [`crate::runner`]'s is: the Pairing its role
//! names, the Turn, the Deferred Answers folded under its prompt, and `verkstead
//! done` held to the evidence it registered. What a component writes is its
//! own prompt; what every session is told is the host's.
//!
//! **Started with facts, and keeping nothing.** [`run`] reads the start facts
//! afresh — the Worktree and Companions, the Pairings, the Brief and handoff,
//! the Timeline as it is held — and a component started again after a restart
//! is handed the same, plus whatever has happened since.
//!
//! **It holds the Conversation's Driving registration** for as long as it runs,
//! as every driver does, so the stall sweep never finds it undriven.
//!
//! **It ends in an Outcome, or it stops.** What the caller is handed is a word
//! the block declared, or `steered`; the caller maps it to where the
//! Conversation goes, which a Process does as data from stage 11 on. A
//! component that cannot be started, traps, or answers a word it never
//! declared stops the Conversation with a Notice saying so, and is never a
//! server panic. A stop that landed while it ran — a Force stop, say — ends it
//! at the next host function it calls, and what it would have answered is not
//! read.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use verkstead_schema::Direction;
use verkstead_store as store;

use crate::AppState;
use crate::continuing::Brought;
use crate::drivers::Driving;
use crate::lending::Lent;
use crate::memos::Building;
use crate::plugins::component::worlds::category::activity::verkstead::plugin::{
    activity_types as world, host,
};
use crate::plugins::component::{self, Ended};
use crate::plugins::manifest::{Activity, Capability, Files, STEERED};

pub(crate) mod follow_up_rounds;
pub(crate) mod interview;
pub(crate) mod one_session;
pub(crate) mod wrap_up;

/// The built-in Activities plugin, and its Activity whose id is `id`: how
/// the Conversation's states call Verkstead's own Activities by name, until a
/// Process names them (stage 11).
pub(crate) fn built_in(
    id: &str,
) -> Option<(
    crate::plugins::built_in::BuiltIn,
    crate::plugins::manifest::Activity,
)> {
    let shipped = crate::plugins::built_in::activities();
    let activity = shipped
        .manifest(crate::platform::Platform::HERE)
        .ok()?
        .activities
        .into_iter()
        .find(|activity| activity.id == id)?;

    Some((shipped, activity))
}

/// Start Activities on this server's Conversations from outside it — the
/// suite's seam, for an Activity no Process of this build's names.
///
/// Handed out beside the router it was built with: see
/// [`crate::router_hosting_activities`].
#[derive(Clone)]
pub struct Hosting {
    state: AppState,
}

impl Hosting {
    pub(crate) fn over(state: AppState) -> Hosting {
        Hosting { state }
    }

    /// Say that `conversation_id`'s interview is to offer `offers` alone,
    /// whatever its Process declares (Set 1765, Q27). Said before the
    /// grilling starts, as a declaration is.
    ///
    /// What a Process declares is its manifest's `directions` — see
    /// [`crate::processes::directions`] — and one of the five Verkstead runs
    /// itself has no Process of its own to swap for a narrower one, so this is
    /// how the suite narrows a Develop's interview.
    pub fn offering(&self, conversation_id: i64, offers: Vec<Direction>) {
        self.state.followers.declare(conversation_id, offers);
    }

    /// Say that the Forge lacks `capability` from now on — the declaration a
    /// Forge makes from stage 12, which GitHub, with every capability, never
    /// does. A watcher asking over it is settled at once: see
    /// [`crate::forge`].
    pub fn lacking(&self, capability: crate::forge::Capability) {
        self.state.forge.lacking(capability);
    }

    /// Start `activity`, whose component is `component`, on
    /// `conversation_id`, granted nothing. The Conversation counts as driven
    /// from before this returns; the task answers the Outcome, or `None` where
    /// it stopped instead.
    pub fn start(
        &self,
        conversation_id: i64,
        component: Vec<u8>,
        activity: Activity,
    ) -> tokio::task::JoinHandle<Option<String>> {
        self.start_granted(conversation_id, component, activity, Vec::new())
    }

    /// The same, granted `capabilities`, as a run brought about by a settle
    /// is.
    pub fn start_granted(
        &self,
        conversation_id: i64,
        component: Vec<u8>,
        activity: Activity,
        capabilities: Vec<Capability>,
    ) -> tokio::task::JoinHandle<Option<String>> {
        let state = self.state.clone();
        let driving = state.drivers.driving(conversation_id);

        tokio::spawn(async move {
            let bytes: &[u8] = &component;
            let file = activity.component.to_string_lossy().into_owned();
            let files = [(file.as_str(), bytes)];

            run(
                &state,
                conversation_id,
                &driving,
                Files::Embedded(&files),
                &activity,
                &capabilities,
                Vec::new(),
                &activity.name,
                Brought::Settle,
            )
            .await
            .map(|ran| ran.outcome)
        })
    }
}

/// How a component's run ended, where it ended in an Outcome.
pub(crate) struct Ran {
    /// The word it answered: one the block declares, or `steered`.
    pub(crate) outcome: String,

    /// The Timeline Event the newest session it launched printed into, where
    /// it launched one — what a stop over what that session left behind
    /// carries the tail of.
    pub(crate) writing: Option<i64>,
}

/// Run `activity` on `conversation_id` to its Outcome, `driving` held
/// throughout and handed back to the caller to hold across whatever the Outcome
/// sets going: the word it answered, where it was one the block declares or
/// `steered`, and `None` where the Conversation stopped instead.
///
/// `doing` is what the rescue and a stop the component asks for say it was
/// doing — the words the state it runs in is judged by, *following the work
/// up*, rather than the Activity's name.
///
/// `brought` is what brought the run about, which a Conversation the component
/// starts is held to the limits by — see [`crate::continuing::begin`].
///
/// **The `building` option is the host's**: where it names a kind — see
/// [`Building::named`] — every session the component launches builds that,
/// which is what permits its Memos and previews and stamps the Memos it
/// leaves.
///
/// **So is `offers`**: the Directions an interview offers, as their words
/// joined by commas — `inline,task-list`. Where it is given, the session
/// launched expecting `picked` offers those alone: its proposals are stamped
/// with them and a pick of any other is not heard. Where it is not, all three.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run(
    state: &AppState,
    conversation_id: i64,
    _driving: &Driving,
    files: Files<'_>,
    activity: &Activity,
    capabilities: &[Capability],
    options: Vec<(String, String)>,
    doing: &str,
    brought: Brought,
) -> Option<Ran> {
    let (outcome, seen) = run_seeing(
        state,
        conversation_id,
        files,
        activity,
        capabilities,
        options,
        Lent::new(state, conversation_id)
            .doing(doing)
            .brought(brought),
    )
    .await;

    outcome.map(|outcome| Ran {
        outcome,
        writing: seen.event_id,
    })
}

/// What the newest session a component launched came to, read back by an
/// in-process caller where the world's `waited` says only *gone* — as
/// [`Lent::see`] reads it back for a driver of its own.
#[derive(Default)]
pub(crate) struct Seen {
    /// The Timeline Event it printed into, where one was launched.
    pub(crate) event_id: Option<i64>,

    /// How it ended, where a wait saw it end without its Done signal taken.
    pub(crate) ended: Option<crate::sessions::Ended>,

    /// The Direction a wait answered `picked`, its artifact written — which
    /// stands where a stop that landed beside it took the Outcome away.
    pub(crate) picked: Option<String>,

    /// The registration the host took as the human picked a Direction, handed
    /// on with the wait's `picked` to whatever the Direction sets going — see
    /// [`Lent::wait`].
    pub(crate) driving: Option<Driving>,

    /// Whether a host function ended the component for a stop it found
    /// standing — see [`Hosted::ended_by_a_stop`]. The stop is whoever's put it
    /// there to say, so the run writes none of its own over it: a move that is
    /// failing forgets its request before it writes its stop, and a run that
    /// ended inside that gap would otherwise say the Activity stopped.
    pub(crate) stopped_under: bool,
}

/// [`run`], and what the newest session the component launched came to — lent
/// `lent`, which carries what only an in-process caller can say to the host:
/// what the rescue says the session is doing, a Turn already held, the Sets a
/// relaunched grilling holds open, a registration to let go once its session
/// is launched.
///
/// The Driving registration is the caller's to hold across the run, as for
/// [`run`] — or the [`Lent`]'s, for the interview, whose grilling is driven by
/// its session until the human picks.
pub(crate) async fn run_seeing(
    state: &AppState,
    conversation_id: i64,
    files: Files<'_>,
    activity: &Activity,
    capabilities: &[Capability],
    options: Vec<(String, String)>,
    lent: Lent,
) -> (Option<String>, Seen) {
    let name = &activity.name;
    let seen = Arc::new(Mutex::new(Seen::default()));
    let building = options
        .iter()
        .find(|(key, _)| key == "building")
        .and_then(|(_, kind)| Building::named(kind));
    let offers = options
        .iter()
        .find(|(key, _)| key == "offers")
        .map(|(_, words)| offered(words))
        .unwrap_or_else(|| Direction::ALL.to_vec());

    let facts = match facts(state, conversation_id, activity, options).await {
        Ok(facts) => facts,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, activity = activity.id, "reading what an Activity starts with failed");
            stopping(
                state,
                conversation_id,
                name,
                &format!("{name} couldn't be started: the conversation couldn't be read."),
            )
            .await;
            return (None, Seen::default());
        }
    };

    let home = crate::sandbox::Homes::of_the_server(&state.data_dir)
        .map(|homes| homes.servers().to_owned());
    let lent = Hosted {
        doing: lent.doing_now().to_owned(),
        lent: lent.building(building).offering(offers),
        seen: seen.clone(),
        state: state.clone(),
        conversation_id,
        name: name.clone(),
        declared: activity.skills.clone(),
        starts: capabilities.contains(&Capability::Conversations),
        building,
        planning: None,
        beside: false,
    };

    let ran =
        component::run_activity(files, activity, capabilities, home.as_deref(), lent, &facts).await;

    let seen = std::mem::take(&mut *seen.lock().expect("nothing panics holding it"));

    // Whatever it answered, a stop that landed while it ran is the last word:
    // its own `stop`, or the human's — or a move's, whose stop may still be on
    // its way when the component it ended has already returned.
    if seen.stopped_under || already_stopped(state, conversation_id).await {
        return (None, seen);
    }

    let how = match ran {
        Ok(outcome) if outcome == STEERED || activity.outcomes.contains(&outcome) => {
            tracing::info!(
                conversation_id,
                activity = activity.id,
                outcome,
                "an Activity ended"
            );
            return (Some(outcome), seen);
        }
        Ok(outcome) => {
            format!("{name} ended in `{outcome}`, which isn't one of the outcomes it declares.")
        }
        Err(trouble) => trouble.why,
    };

    tracing::warn!(
        conversation_id,
        activity = activity.id,
        how,
        "an Activity stopped the Conversation"
    );
    stopping(state, conversation_id, name, &how).await;

    (None, seen)
}

/// Run `activity` **beside** `conversation_id` rather than driving it: the
/// word it answered, where it was one the block declares, and `None` where it
/// could not be run or answered anything else — which is said in the log
/// alone.
///
/// What the **Stage runner** runs as — see [`crate::continuing`]. The
/// Conversation it is handed is one that has settled, or the foot of a roadmap's
/// chain, which nothing here drives: so it holds no Driving registration, a stop
/// standing on the Conversation does not end it, and nothing it does or fails
/// to do stops the Conversation. What it notices is said as `brought` says it
/// is — see [`crate::continuing::Brought::noticed`].
#[allow(clippy::too_many_arguments)]
pub(crate) async fn beside(
    state: &AppState,
    conversation_id: i64,
    files: Files<'_>,
    activity: &Activity,
    capabilities: &[Capability],
    options: Vec<(String, String)>,
    brought: Brought,
) -> Option<String> {
    let name = &activity.name;

    let facts = match facts(state, conversation_id, activity, options).await {
        Ok(facts) => facts,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, activity = activity.id, "reading what an Activity starts with failed");
            return None;
        }
    };

    let home = crate::sandbox::Homes::of_the_server(&state.data_dir)
        .map(|homes| homes.servers().to_owned());
    let lent = Hosted {
        lent: Lent::new(state, conversation_id)
            .doing(name)
            .brought(brought),
        state: state.clone(),
        conversation_id,
        name: name.clone(),
        declared: activity.skills.clone(),
        starts: capabilities.contains(&Capability::Conversations),
        doing: name.clone(),
        building: None,
        seen: Arc::new(Mutex::new(Seen::default())),
        planning: None,
        beside: true,
    };

    match component::run_activity(files, activity, capabilities, home.as_deref(), lent, &facts)
        .await
    {
        Ok(outcome) if activity.outcomes.contains(&outcome) => {
            tracing::info!(
                conversation_id,
                activity = activity.id,
                outcome,
                "an Activity run beside a Conversation ended"
            );
            Some(outcome)
        }
        Ok(outcome) => {
            tracing::error!(
                conversation_id,
                activity = activity.id,
                outcome,
                "an Activity run beside a Conversation ended in an outcome it doesn't declare"
            );
            None
        }
        Err(trouble) => {
            tracing::error!(
                conversation_id,
                activity = activity.id,
                why = trouble.why,
                "an Activity run beside a Conversation couldn't be run"
            );
            None
        }
    }
}

/// The Directions `words` names, joined by commas, in the design's order and
/// each once — all three where it names none of them.
fn offered(words: &str) -> Vec<Direction> {
    let named: Vec<Direction> = words
        .split(',')
        .filter_map(|word| Direction::named(word.trim()))
        .collect();

    match Direction::ALL
        .into_iter()
        .filter(|direction| named.contains(direction))
        .collect::<Vec<_>>()
    {
        offered if offered.is_empty() => Direction::ALL.to_vec(),
        offered => offered,
    }
}

/// Stop the Conversation as `name` saying `how`, closing any Set left open.
async fn stopping(state: &AppState, conversation_id: i64, name: &str, how: &str) {
    crate::activities::follow_up_rounds::left_open(state, conversation_id).await;

    crate::runner::stop(
        state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        name,
        how,
        None,
    )
    .await;
}

/// Whether a stop stands on the Conversation, read and nothing written — a
/// Stop the human asked for after the step is the session's to land, at its
/// next launch.
///
/// **Or nothing is to drive it any more**: a steer put it in a state nothing
/// drives — into Done, its session left to finish — or the work is being moved
/// to another device. Either refuses the component's next launch with no stop
/// behind it, and a stop written over that refusal would put a badge on work
/// the human has finished with or sent elsewhere. See
/// [`crate::stopping::stopped`], which asks the same two of every launch.
async fn already_stopped(state: &AppState, conversation_id: i64) -> bool {
    if crate::stopping::undriven(state, conversation_id).await
        || crate::transfers::going(state, conversation_id).await
    {
        return true;
    }

    match store::stopped(&state.pool, conversation_id).await {
        Ok(stopped) => stopped.is_some(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a Conversation had stopped under its Activity failed");
            true
        }
    }
}

/// What an Activity starts with, read off the Conversation as it stands.
async fn facts(
    state: &AppState,
    conversation_id: i64,
    activity: &Activity,
    options: Vec<(String, String)>,
) -> Result<world::Facts> {
    let conversation = store::load_conversation(&state.pool, conversation_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("there is no Conversation {conversation_id}"))?;
    let (brief, handoff) = crate::conversations::documents(&state.pool, conversation_id).await?;
    let timeline = store::timeline_as_held(&state.pool, conversation_id).await?;

    let worktree = world::Checkout {
        repo: conversation.repo.name.clone(),
        path: shown(conversation.worktree.as_deref()),
        branch: conversation.branch.clone(),
        base: conversation.base_commit.clone().unwrap_or_default(),
        writable: true,
    };

    let companions = conversation
        .companions
        .iter()
        .map(|companion| world::Checkout {
            repo: companion.repo.name.clone(),
            path: shown(companion.worktree.as_deref()),
            branch: companion
                .branch_for(&conversation.branch)
                .unwrap_or_else(|| companion.branch.clone()),
            base: companion.base_commit.clone().unwrap_or_default(),
            writable: companion.mode == store::CompanionMode::ReadWrite,
        })
        .collect();

    let pairings = [
        store::Role::Grilling,
        store::Role::Implementation,
        store::Role::Review,
    ]
    .into_iter()
    .filter_map(|role| {
        let pairing = match role {
            store::Role::Grilling => conversation.grilling_pairing.clone(),
            role => crate::runner::under(
                role,
                &conversation.review_pairing,
                conversation.implementation_pairing.as_ref(),
            ),
        }?;

        Some(world::Pairing {
            role: crate::lending::role_named(role).to_owned(),
            profile: pairing.profile.id.clone(),
            model: pairing.runs_on().unwrap_or_default().to_owned(),
        })
    })
    .collect();

    let skills = match state.sessions.skills() {
        Some(skills) => activity
            .skills
            .iter()
            .map(|skill| world::Skill {
                name: skill.clone(),
                path: skills.named(&format!("{skill}/SKILL.md")),
            })
            .collect(),
        None => Vec::new(),
    };

    Ok(world::Facts {
        worktree,
        companions,
        pairings,
        brief,
        handoff,
        timeline: timeline
            .into_iter()
            .map(|(id, kind, body)| world::Entry { id, kind, body })
            .collect(),
        options,
        skills,
    })
}

fn shown(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_default()
}

/// The host lent to a component: [`Lent`]'s every call, held to the skills
/// its block declares, and a stop that landed while it ran ending the
/// component at the next one.
struct Hosted {
    lent: Lent,

    /// What the newest session launched came to, for [`run_seeing`]'s caller.
    seen: Arc<Mutex<Seen>>,

    state: AppState,
    conversation_id: i64,

    /// The Activity's drawn name, which a refused launch is said in.
    name: String,

    /// The skills its block declares: **what an Activity names, it declares**
    /// (Set 1771, Q1), so a launch under any other is refused, as the loader
    /// refuses a block naming a skill it cannot reach.
    declared: Vec<String>,

    /// Whether its plugin was granted `conversations: start`, without which
    /// every host function about starting Conversations is refused.
    starts: bool,

    /// What the rescue and a stop say it is doing between the steps its
    /// launches name — see [`step`].
    doing: String,

    /// What the sessions its launches name no step for build, from its
    /// `building` option — see [`run`].
    building: Option<Building>,

    /// The session launched to plan a roadmap stage, where the newest was one:
    /// its backlog landing is the host's to record once it has been waited on.
    planning: Option<world::Session>,

    /// Whether it runs beside the Conversation rather than driving it — see
    /// [`beside`] — so that a stop standing on it ends nothing.
    beside: bool,
}

impl Hosted {
    /// Trap the component where the Conversation has stopped under it.
    async fn ended_by_a_stop(&self) -> wasmtime::Result<()> {
        if !self.beside && already_stopped(&self.state, self.conversation_id).await {
            self.seen
                .lock()
                .expect("nothing panics holding it")
                .stopped_under = true;

            return Err(wasmtime::Error::new(Ended(
                "The conversation was stopped.".to_owned(),
            )));
        }

        Ok(())
    }
}

impl Hosted {
    /// The sentence a call about starting Conversations is refused in, where
    /// the plugin was not granted it.
    fn ungranted(&self) -> Option<String> {
        if self.starts {
            return None;
        }

        tracing::error!(
            conversation_id = self.conversation_id,
            "an Activity asked about starting Conversations without the capability",
        );

        Some(format!(
            "{} doesn't ask to start conversations, so it can't start or read one.",
            self.name
        ))
    }

    /// Whether `.tasks/TODO.md` is in the Worktree, committed as it stands.
    async fn backlog_committed(&self) -> bool {
        let worktree = store::load_conversation(&self.state.pool, self.conversation_id)
            .await
            .ok()
            .flatten()
            .and_then(|conversation| conversation.worktree);

        match worktree {
            Some(worktree) => crate::runner::backlog_landed(&worktree).await,
            None => false,
        }
    }
}

impl world::Host for Hosted {}

impl host::Host for Hosted {
    async fn launch(
        &mut self,
        skill: String,
        role: String,
        prompt: String,
        evidence: world::Evidence,
        ends: world::Ends,
    ) -> wasmtime::Result<Result<world::Session, String>> {
        if !self.declared.contains(&skill) {
            tracing::error!(
                conversation_id = self.conversation_id,
                skill,
                "an Activity asked to launch a session under a skill its block doesn't declare",
            );

            return Ok(Err(format!(
                "{} doesn't declare the `{skill}` skill, so it can't launch a session under it.",
                self.name
            )));
        }

        // What the session builds, and what it is said to be doing, read off
        // what it is expected to leave — see [`step`].
        let building = step(&self.state, self.conversation_id, &evidence).await;
        let planning = matches!(building, Some((Building::Stage, _)));

        match building {
            Some((building, doing)) => self.lent.now(Some(building), &doing),
            None => self.lent.now(self.building, &self.doing),
        }

        let launched =
            host::Host::launch(&mut self.lent, skill, role, prompt, evidence, ends).await?;

        match &launched {
            Ok(session) => {
                self.planning = planning.then_some(*session);

                *self.seen.lock().expect("nothing panics holding it") = Seen {
                    event_id: Some(self.lent.event_id(*session)),
                    ..Seen::default()
                }
            }
            Err(_) => self.ended_by_a_stop().await?,
        }

        Ok(launched)
    }

    /// A session that gave its Done signal and has been quiet since is ended
    /// as the wait comes back, as the runner always ended a step's: whatever
    /// the component does next — another launch, a placement that moves the
    /// Conversation — happens with nothing running.
    ///
    /// **And a stage's backlog landing is recorded** once its planning session
    /// is waited on and the list is committed, however the session ended, as
    /// the runner recorded it after seeing the planning out. A second sighting
    /// writes nothing — see [`crate::conversations::backlog_landed`].
    async fn wait(&mut self, session: world::Session) -> wasmtime::Result<world::Waited> {
        let waited = host::Host::wait(&mut self.lent, session).await?;

        {
            let mut seen = self.seen.lock().expect("nothing panics holding it");
            seen.ended = self.lent.ended(session);

            if let world::Waited::Picked(direction) = &waited {
                seen.picked = Some(direction.clone());
                seen.driving = self.lent.driving();
            }
        }

        if self.planning == Some(session) && self.backlog_committed().await {
            crate::conversations::backlog_landed(&self.state, self.conversation_id).await;
        }

        match waited {
            world::Waited::Done => {
                self.state.sessions.end(self.conversation_id).await;
                self.lent.now(self.building, &self.doing);
            }
            world::Waited::Picked(_) => {}
            _ => self.ended_by_a_stop().await?,
        }

        Ok(waited)
    }

    async fn git_facts(
        &mut self,
        repo: String,
    ) -> wasmtime::Result<Result<world::BranchFacts, String>> {
        host::Host::git_facts(&mut self.lent, repo).await
    }

    async fn notice(&mut self, text: String) -> wasmtime::Result<()> {
        host::Host::notice(&mut self.lent, text).await
    }

    async fn stop(&mut self, reason: String) -> wasmtime::Result<()> {
        host::Host::stop(&mut self.lent, reason).await
    }

    async fn read_file(
        &mut self,
        repo: String,
        at: host::Revision,
        path: String,
    ) -> wasmtime::Result<Result<Option<Vec<u8>>, String>> {
        host::Host::read_file(&mut self.lent, repo, at, path).await
    }

    async fn list_directory(
        &mut self,
        repo: String,
        at: host::Revision,
        path: String,
    ) -> wasmtime::Result<Result<Option<Vec<String>>, String>> {
        host::Host::list_directory(&mut self.lent, repo, at, path).await
    }

    async fn place_task(
        &mut self,
        task: u32,
        label: String,
        platform: String,
    ) -> wasmtime::Result<host::Placed> {
        host::Host::place_task(&mut self.lent, task, label, platform).await
    }

    /// The chain's wait, and a stop that landed while it waited ending the
    /// component rather than letting it launch the finish.
    async fn join_chain(&mut self) -> wasmtime::Result<Vec<host::Link>> {
        let chain = host::Host::join_chain(&mut self.lent).await?;

        self.ended_by_a_stop().await?;

        Ok(chain)
    }

    async fn start_conversation(
        &mut self,
        stage: host::Stage,
    ) -> wasmtime::Result<Result<host::Started, host::NotStarted>> {
        if let Some(why) = self.ungranted() {
            return Ok(Err(host::NotStarted::Halted(why)));
        }

        self.ended_by_a_stop().await?;

        host::Host::start_conversation(&mut self.lent, stage).await
    }

    /// A stop that landed ends the component here too, as an Activity reading
    /// a Conversation over and over until it settles has no launch or wait to
    /// be ended at.
    async fn conversation_state(
        &mut self,
        conversation: i64,
    ) -> wasmtime::Result<Result<host::Stands, String>> {
        if let Some(why) = self.ungranted() {
            return Ok(Err(why));
        }

        self.ended_by_a_stop().await?;

        host::Host::conversation_state(&mut self.lent, conversation).await
    }

    async fn stage_standings(
        &mut self,
        roadmap: String,
    ) -> wasmtime::Result<Result<host::Standings, String>> {
        if let Some(why) = self.ungranted() {
            return Ok(Err(why));
        }

        host::Host::stage_standings(&mut self.lent, roadmap).await
    }

    async fn stage_chain(
        &mut self,
        roadmap: String,
    ) -> wasmtime::Result<Result<Vec<host::Link>, String>> {
        if let Some(why) = self.ungranted() {
            return Ok(Err(why));
        }

        host::Host::stage_chain(&mut self.lent, roadmap).await
    }

    async fn wait_any(
        &mut self,
        sessions: Vec<world::Session>,
        timeout: u64,
    ) -> wasmtime::Result<world::Woke> {
        let woke = host::Host::wait_any(&mut self.lent, sessions, timeout).await?;

        if let world::Woke::Waited((session, waited)) = &woke {
            {
                let mut seen = self.seen.lock().expect("nothing panics holding it");

                if seen.event_id == Some(self.lent.event_id(*session)) {
                    seen.ended = self.lent.ended(*session);
                }
            }

            if !matches!(waited, world::Waited::Done) {
                self.ended_by_a_stop().await?;
            }
        }

        Ok(woke)
    }

    async fn pull_requests(&mut self) -> wasmtime::Result<Vec<world::PullRequest>> {
        host::Host::pull_requests(&mut self.lent).await
    }

    async fn forge_standing(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<world::PullRequestStanding, world::Unforged>> {
        host::Host::forge_standing(&mut self.lent, pull_request).await
    }

    async fn forge_checks(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<world::Suite, world::Unforged>> {
        host::Host::forge_checks(&mut self.lent, pull_request).await
    }

    async fn forge_comments(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<Vec<world::Comment>, world::Unforged>> {
        host::Host::forge_comments(&mut self.lent, pull_request).await
    }

    async fn forge_reply(
        &mut self,
        pull_request: world::PullRequest,
        markdown: String,
    ) -> wasmtime::Result<Result<String, world::Unforged>> {
        host::Host::forge_reply(&mut self.lent, pull_request, markdown).await
    }

    async fn forge_opened(
        &mut self,
        repo: String,
        branch: String,
    ) -> wasmtime::Result<Result<Option<world::Opened>, world::Unforged>> {
        host::Host::forge_opened(&mut self.lent, repo, branch).await
    }

    async fn wrap_up(&mut self) -> wasmtime::Result<world::Wrapped> {
        host::Host::wrap_up(&mut self.lent).await
    }

    async fn record_opened(&mut self) -> wasmtime::Result<Result<world::PullRequest, String>> {
        host::Host::record_opened(&mut self.lent).await
    }

    async fn settle(&mut self, watcher: world::Watcher) -> wasmtime::Result<()> {
        host::Host::settle(&mut self.lent, watcher).await
    }

    async fn unsettle(&mut self, watcher: world::Watcher) -> wasmtime::Result<()> {
        host::Host::unsettle(&mut self.lent, watcher).await
    }

    async fn fix_attempts(
        &mut self,
        pull_request: world::PullRequest,
        check: String,
    ) -> wasmtime::Result<Result<i64, String>> {
        host::Host::fix_attempts(&mut self.lent, pull_request, check).await
    }

    async fn count_fix_attempt(
        &mut self,
        pull_request: world::PullRequest,
        check: String,
    ) -> wasmtime::Result<Result<i64, String>> {
        host::Host::count_fix_attempt(&mut self.lent, pull_request, check).await
    }

    async fn addressed(&mut self, repo: String) -> wasmtime::Result<Result<Vec<String>, String>> {
        host::Host::addressed(&mut self.lent, repo).await
    }

    async fn mark_addressed(
        &mut self,
        repo: String,
        comments: Vec<String>,
    ) -> wasmtime::Result<Result<(), String>> {
        host::Host::mark_addressed(&mut self.lent, repo, comments).await
    }

    async fn pushed_head(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Option<String>> {
        host::Host::pushed_head(&mut self.lent, pull_request).await
    }

    async fn conflict_attempts(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<i64, String>> {
        host::Host::conflict_attempts(&mut self.lent, pull_request).await
    }

    async fn count_conflict_attempt(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<i64, String>> {
        host::Host::count_conflict_attempt(&mut self.lent, pull_request).await
    }

    async fn resolution(
        &mut self,
        pull_request: world::PullRequest,
    ) -> wasmtime::Result<Result<String, String>> {
        host::Host::resolution(&mut self.lent, pull_request).await
    }

    async fn give_up(
        &mut self,
        pull_request: world::PullRequest,
        what: String,
        how: String,
    ) -> wasmtime::Result<bool> {
        host::Host::give_up(&mut self.lent, pull_request, what, how).await
    }

    async fn ignored(
        &mut self,
        repo: String,
        comments: Vec<world::Comment>,
    ) -> wasmtime::Result<Vec<String>> {
        host::Host::ignored(&mut self.lent, repo, comments).await
    }

    async fn settled(&mut self, watcher: world::Watcher) -> wasmtime::Result<bool> {
        host::Host::settled(&mut self.lent, watcher).await
    }

    async fn cover(&mut self) -> wasmtime::Result<()> {
        host::Host::cover(&mut self.lent).await
    }

    async fn review_wanted(&mut self, afresh: bool) -> wasmtime::Result<bool> {
        host::Host::review_wanted(&mut self.lent, afresh).await
    }

    async fn reviewing(
        &mut self,
        said: Option<String>,
    ) -> wasmtime::Result<Result<String, String>> {
        host::Host::reviewing(&mut self.lent, said).await
    }

    async fn review_over(&mut self, how: Option<String>) -> wasmtime::Result<()> {
        host::Host::review_over(&mut self.lent, how).await
    }

    async fn batch_over(
        &mut self,
        pull_request: world::PullRequest,
        comments: Vec<String>,
        how: Option<String>,
    ) -> wasmtime::Result<()> {
        host::Host::batch_over(&mut self.lent, pull_request, comments, how).await
    }

    async fn left_behind(&mut self) -> wasmtime::Result<bool> {
        host::Host::left_behind(&mut self.lent).await
    }
}

/// What a session launched expecting `evidence` builds, and what it is said to
/// be doing, where it builds: the host's reading of the evidence rather than
/// anything the component says, so no launch needs a word for it (Set 1839).
/// A box ticked is that task, `.tasks/TODO.md` gone is the finish, and the
/// list arriving is a stage's planning — the three a backlog's sessions are
/// launched expecting. See [`crate::memos`].
async fn step(
    state: &AppState,
    conversation_id: i64,
    evidence: &world::Evidence,
) -> Option<(Building, String)> {
    let world::Evidence::Landed(landing) = evidence else {
        return None;
    };

    let todo = Path::new(crate::tasks::BACKLOG).join(crate::tasks::TODO);

    match landing {
        world::Landing::Ticked(number) => {
            let worktree = store::load_conversation(&state.pool, conversation_id)
                .await
                .ok()
                .flatten()
                .and_then(|conversation| conversation.worktree);

            let file = worktree.and_then(|worktree| {
                crate::tasks::files(&worktree.join(crate::tasks::BACKLOG)).remove(number)
            });

            let doing = match file {
                Some(file) => format!(
                    "the task in {}",
                    Path::new(crate::tasks::BACKLOG).join(file).display()
                ),
                None => format!("task {number} of the backlog"),
            };

            Some((Building::Task(*number), doing))
        }
        world::Landing::Gone(path) if Path::new(path) == todo => {
            Some((Building::Finish, "finishing the feature".to_owned()))
        }
        world::Landing::Arrived(path) if Path::new(path) == todo => Some((
            Building::Stage,
            "planning the roadmap stage into a backlog".to_owned(),
        )),
        _ => None,
    }
}
