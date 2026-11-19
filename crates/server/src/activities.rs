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
use verkstead_store as store;

use crate::AppState;
use crate::drivers::Driving;
use crate::lending::Lent;
use crate::memos::Building;
use crate::plugins::component::worlds::category::activity::verkstead::plugin::{
    activity_types as world, host,
};
use crate::plugins::component::{self, Ended};
use crate::plugins::manifest::{Activity, Capability, Files, STEERED};

/// Start Activities on this server's Conversations from outside it — the
/// suite's seam, until a Process says which Activity a state runs.
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
                &[],
                Vec::new(),
                &activity.name,
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
) -> Option<Ran> {
    let name = &activity.name;

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
            return None;
        }
    };

    let home = crate::sandbox::Homes::of_the_server(&state.data_dir)
        .map(|homes| homes.servers().to_owned());
    let writing = Arc::new(Mutex::new(None));
    let lent = Hosted {
        lent: Lent::new(state, conversation_id).doing(doing),
        state: state.clone(),
        conversation_id,
        name: name.clone(),
        declared: activity.skills.clone(),
        doing: doing.to_owned(),
        writing: writing.clone(),
    };

    let ran =
        component::run_activity(files, activity, capabilities, home.as_deref(), lent, &facts).await;

    // Whatever it answered, a stop that landed while it ran is the last word:
    // its own `stop`, or the human's.
    if already_stopped(state, conversation_id).await {
        return None;
    }

    let how = match ran {
        Ok(outcome) if outcome == STEERED || activity.outcomes.contains(&outcome) => {
            tracing::info!(
                conversation_id,
                activity = activity.id,
                outcome,
                "an Activity ended"
            );
            let writing = *writing
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            return Some(Ran { outcome, writing });
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

    None
}

/// Stop the Conversation as `name` saying `how`, closing any Set left open.
async fn stopping(state: &AppState, conversation_id: i64, name: &str, how: &str) {
    crate::runner::left_open(state, conversation_id).await;

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
    state: AppState,
    conversation_id: i64,

    /// The Activity's drawn name, which a refused launch is said in.
    name: String,

    /// The skills its block declares: **what an Activity names, it declares**
    /// (Set 1771, Q1), so a launch under any other is refused, as the loader
    /// refuses a block naming a skill it cannot reach.
    declared: Vec<String>,

    /// What the rescue and a stop say it is doing between the steps its
    /// launches name — see [`step`].
    doing: String,

    /// The Event the newest session launched here prints into, read back by
    /// [`run`] once the component has ended.
    writing: Arc<Mutex<Option<i64>>>,
}

impl Hosted {
    /// Trap the component where the Conversation has stopped under it.
    async fn ended_by_a_stop(&self) -> wasmtime::Result<()> {
        if already_stopped(&self.state, self.conversation_id).await {
            return Err(wasmtime::Error::new(Ended(
                "The conversation was stopped.".to_owned(),
            )));
        }

        Ok(())
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
        match step(&self.state, self.conversation_id, &evidence).await {
            Some((building, doing)) => self.lent.now(Some(building), &doing),
            None => self.lent.now(None, &self.doing),
        }

        let launched =
            host::Host::launch(&mut self.lent, skill, role, prompt, evidence, ends).await?;

        match &launched {
            Ok(session) => {
                *self
                    .writing
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                    Some(self.lent.event_id(*session));
            }
            Err(_) => self.ended_by_a_stop().await?,
        }

        Ok(launched)
    }

    /// A session that gave its Done signal and has been quiet since is ended
    /// as the wait comes back, as the runner always ended a step's: whatever
    /// the component does next — another launch, a placement that moves the
    /// Conversation — happens with nothing running.
    async fn wait(&mut self, session: world::Session) -> wasmtime::Result<world::Waited> {
        let waited = host::Host::wait(&mut self.lent, session).await?;

        match waited {
            world::Waited::Done => {
                self.state.sessions.end(self.conversation_id).await;
                self.lent.now(None, &self.doing);
            }
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
