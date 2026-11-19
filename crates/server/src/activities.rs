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

use anyhow::Result;
use verkstead_store as store;

use crate::AppState;
use crate::drivers::Driving;
use crate::lending::Lent;
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
        })
    }
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
) -> Option<String> {
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
    let lent = Hosted {
        lent: Lent::new(state, conversation_id).doing(doing),
        state: state.clone(),
        conversation_id,
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
            return Some(outcome);
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
async fn already_stopped(state: &AppState, conversation_id: i64) -> bool {
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

/// The host lent to a component: [`Lent`]'s every call, and a stop that
/// landed while it ran ending the component at the next one.
struct Hosted {
    lent: Lent,
    state: AppState,
    conversation_id: i64,
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
        let launched =
            host::Host::launch(&mut self.lent, skill, role, prompt, evidence, ends).await?;

        if launched.is_err() {
            self.ended_by_a_stop().await?;
        }

        Ok(launched)
    }

    async fn wait(&mut self, session: world::Session) -> wasmtime::Result<world::Waited> {
        let waited = host::Host::wait(&mut self.lent, session).await?;

        if !matches!(waited, world::Waited::Done) {
            self.ended_by_a_stop().await?;
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
}
