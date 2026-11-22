//! The **Process engine**: a Conversation on a Process a plugin provides, run
//! by reading the manifest (ADR-0030, *Process*).
//!
//! **Start enters the first declared state** — see
//! [`crate::conversations::start_grilling`], which writes the state's id to
//! the `conversations.state` column with the rest of what a Start records and
//! hands the Conversation to [`run`].
//!
//! **Each state runs its Activity**, started with the Conversation's facts and
//! the state's options, holding the Conversation's Driving registration for
//! as long as it runs. The Outcome it ends in is mapped by the manifest: to the
//! next declared state, entered the same way; to Done; to Closed, which is the
//! human's Close made by the Process; or to `stopped`, which leaves the
//! Conversation Stopped where it stands with a Notice saying what the Activity
//! ended in, waiting on Resume.
//!
//! **A restart or a Resume enters the current state's Activity again** — see
//! [`crate::resume`] — and the Activity works out where it was from its facts,
//! which is the restart rule every Activity keeps. Nothing is held here
//! between runs.
//!
//! **A Process in trouble stops where it stands.** Its plugin, or a plugin
//! providing one of its Activities, is missing, switched off or in trouble:
//! the Conversation stops with the plugin's trouble as its Notice, which names
//! the dependency wanted, and is carried on from its state once that is back —
//! see [`taken_back`]. A plugin switched off while a Conversation runs on it is
//! the exception: what loaded is kept, and the Conversation runs to the end
//! (Set 1764, Q20).

use std::path::PathBuf;

use crate::AppState;
use crate::drivers::Driving;
use crate::plugins::built_in::BuiltIn;
use crate::plugins::manifest::process::{Process, Runs, State, To};
use crate::plugins::manifest::{Activity, Capability, Files, STEERED};
use crate::store;

/// A Process a plugin loaded here provides, with every Activity its states
/// run found.
#[derive(Debug, Clone)]
pub(crate) struct Loaded {
    /// The providing plugin's uuid.
    pub(crate) plugin: String,

    pub(crate) process: Process,

    /// Whether a new Conversation may be started on it: its plugin is
    /// switched on and in no trouble. A switched-off plugin's Process is kept,
    /// not offered, for the Conversations already on it.
    pub(crate) offered: bool,

    /// Each Activity its states run, by its id.
    pub(crate) activities: Vec<Provided>,
}

/// An Activity a Process's state runs, and where it comes from.
#[derive(Debug, Clone)]
pub(crate) struct Provided {
    pub(crate) activity: Activity,

    /// Where its component is read from.
    pub(crate) files: Source,

    /// What its plugin was granted.
    pub(crate) capabilities: Vec<Capability>,
}

/// Where a plugin's files are.
#[derive(Debug, Clone)]
pub(crate) enum Source {
    BuiltIn(BuiltIn),
    Dir(PathBuf),
}

impl Source {
    fn files(&self) -> Files<'_> {
        match self {
            Source::BuiltIn(built_in) => built_in.files(),
            Source::Dir(dir) => Files::Dir(dir),
        }
    }
}

impl Loaded {
    /// The Activity `id`, where one of its states runs it.
    fn activity(&self, id: &str) -> Option<&Provided> {
        self.activities
            .iter()
            .find(|provided| provided.activity.id == id)
    }

    /// The picker's row for it, and what a Conversation on it is worded
    /// from.
    pub(crate) fn entry(&self) -> verkstead_render::ProcessEntry {
        verkstead_render::ProcessEntry {
            plugin: self.plugin.clone(),
            id: self.process.id.clone(),
            name: self.process.name.clone(),
            description: self.process.description.clone(),
            offered: self.offered,
            states: self
                .process
                .states
                .iter()
                .map(|state| verkstead_render::DeclaredStateEntry {
                    id: state.id.clone(),
                    word: state.word.clone(),
                })
                .collect(),
        }
    }
}

/// Drive `conversation_id` from the declared state it stands in until its
/// Process stops driving it: Done, Closed, stopped, or moved elsewhere by the
/// human. `driving` is held throughout.
///
/// What a Start, a Resume and a restart all hand a declared state to.
pub(crate) async fn run(state: AppState, conversation_id: i64, driving: Driving) {
    loop {
        match step(&state, conversation_id, &driving).await {
            Ok(true) => continue,
            Ok(false) => return,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "running a Conversation's Process failed");
                return;
            }
        }
    }
}

/// Run the Activity of the state the Conversation stands in and follow its
/// Outcome: `true` where that entered another declared state to run.
async fn step(state: &AppState, conversation_id: i64, driving: &Driving) -> anyhow::Result<bool> {
    let Some(conversation) = store::load_conversation(&state.pool, conversation_id).await? else {
        return Ok(false);
    };

    // Closed or steered off it since, or Done: nothing of the Process's left
    // to run.
    let Some(standing) = conversation.declared_state.clone() else {
        return Ok(false);
    };

    let store::Process::Provided { plugin, id } = &conversation.process else {
        return Ok(false);
    };

    // Not before the installed plugins have loaded since the start: until
    // then nothing is held, and a restart reading the Processes in that
    // window would stop every Conversation on one as in trouble.
    if let Some(skills) = state.sessions.skills() {
        skills.settled().await;
    }

    let Some(loaded) = state.plugins.process(plugin, id) else {
        stop(
            state,
            conversation_id,
            crate::stopping::Decided::Nobody,
            &format!("running the {id} process"),
            &trouble(state, plugin, id),
            None,
        )
        .await;
        return Ok(false);
    };

    let Some(declared) = loaded.process.state(&standing) else {
        stop(
            state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            &format!("running {}", loaded.process.name),
            &format!(
                "{} declares no state `{standing}`, which is where this conversation stands. \
                 Update the plugin, or close the conversation.",
                loaded.process.name
            ),
            None,
        )
        .await;
        return Ok(false);
    };

    let doing = doing(declared);

    let Some(provided) = activity(&loaded, declared, &conversation) else {
        stop(
            state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            &doing,
            &format!(
                "{} has no Activity to run in its state `{standing}` for the direction this \
                 conversation was given.",
                loaded.process.name
            ),
            None,
        )
        .await;
        return Ok(false);
    };

    let Some(options) = options(state, conversation_id, declared).await else {
        return Ok(false);
    };

    tracing::info!(
        conversation_id,
        process = loaded.process.id,
        state = standing,
        activity = provided.activity.id,
        "a declared state's Activity is starting"
    );

    let Some(ran) = crate::activities::run(
        state,
        conversation_id,
        driving,
        provided.files.files(),
        &provided.activity,
        &provided.capabilities,
        options,
        &doing,
        crate::continuing::Brought::Settle,
    )
    .await
    else {
        return Ok(false);
    };

    // Taken elsewhere by the human, whose steer says where it goes.
    if ran.outcome == STEERED {
        return Ok(false);
    }

    let Some((_, to)) = declared
        .transitions
        .iter()
        .find(|(outcome, _)| *outcome == ran.outcome)
    else {
        // A detour's Outcomes lead back where it was steered from, which is
        // the steer's to say; and the loader refuses a Process leaving any
        // other unmapped, so this is a manifest changed under a running
        // Activity.
        stop(
            state,
            conversation_id,
            crate::stopping::Decided::Verkstead,
            &doing,
            &format!(
                "{} ended in `{}`, which {} doesn't say where to go from.",
                provided.activity.name, ran.outcome, loaded.process.name
            ),
            ran.writing,
        )
        .await;
        return Ok(false);
    };

    match to {
        To::State(next) => {
            moved(
                state,
                conversation_id,
                &standing,
                store::Destination::Declared(next),
            )
            .await
        }
        To::Done => {
            moved(
                state,
                conversation_id,
                &standing,
                store::Destination::Built(store::Lifecycle::Done),
            )
            .await?;
            state.sessions.end(conversation_id).await;
            Ok(false)
        }
        To::Closed => {
            crate::conversations::close(state, conversation_id).await?;
            Ok(false)
        }
        To::Stopped => {
            stop(
                state,
                conversation_id,
                crate::stopping::Decided::Verkstead,
                &doing,
                &format!(
                    "{} ended in `{}`, and {} stops here until you press Resume.",
                    provided.activity.name, ran.outcome, loaded.process.name
                ),
                ran.writing,
            )
            .await;
            Ok(false)
        }
    }
}

/// Write the move from `from` to `to`, and say the Conversation moved — so
/// the page shows the state it entered. `false` where it had left `from`.
async fn moved(
    state: &AppState,
    conversation_id: i64,
    from: &str,
    to: store::Destination<'_>,
) -> anyhow::Result<bool> {
    let moved = store::transition(&state.pool, conversation_id, from, to).await?;

    if moved == store::Transitioned::Moved {
        state
            .nudges
            .announce(verkstead_schema::Nudge::Conversation {
                conversation: conversation_id,
            });
        state
            .nudges
            .announce(verkstead_schema::Nudge::Conversations);
    }

    Ok(moved == store::Transitioned::Moved)
}

/// What the rescue and a stop say the Conversation was doing in `state`: its
/// word, as the card draws it, in the middle of a sentence.
fn doing(state: &State) -> String {
    let mut letters = state.word.chars();
    match letters.next() {
        Some(first) => first.to_lowercase().collect::<String>() + letters.as_str(),
        None => state.id.clone(),
    }
}

/// The Activity `declared` runs on `conversation`: its one, or the one for
/// the Direction the Conversation was given.
fn activity<'a>(
    loaded: &'a Loaded,
    declared: &State,
    conversation: &store::Conversation,
) -> Option<&'a Provided> {
    let id = match &declared.activity {
        Runs::Activity(id) => id,
        Runs::ByDirection(each) => {
            let direction = conversation.direction?;
            &each
                .iter()
                .find(|(word, _)| *word == direction_word(direction))?
                .1
        }
    };

    loaded.activity(id)
}

/// A Direction as a manifest writes it.
fn direction_word(direction: verkstead_schema::Direction) -> &'static str {
    match direction {
        verkstead_schema::Direction::Inline => "inline",
        verkstead_schema::Direction::TaskList => "task-list",
        verkstead_schema::Direction::Roadmap => "roadmap",
    }
}

/// The options `declared` starts its Activity with: the manifest's, and a
/// `prompt` over the Conversation's documents where the state names a skill.
///
/// **The prompt is Verkstead's to put the documents under**, as every
/// session's is: the manifest's `prompt`, where it writes one, is the opening
/// line over the Brief and what the grilling settled, and where it writes none
/// the opening is *read the skill and do what it says* — see
/// [`crate::skills::declared`]. `None` where the documents couldn't be read,
/// which is said in the log.
async fn options(
    state: &AppState,
    conversation_id: i64,
    declared: &State,
) -> Option<Vec<(String, String)>> {
    let mut options = declared.options.clone();

    let skill = options
        .iter()
        .find(|(key, _)| key == "skill")
        .map(|(_, skill)| skill.clone());

    if let Some(skill) = skill {
        let documents = crate::runner::documents(state, conversation_id).await?;
        let written = options
            .iter()
            .position(|(key, _)| key == "prompt")
            .map(|at| options.remove(at).1);

        options.push((
            "prompt".to_owned(),
            crate::skills::declared(
                documents.skills,
                &skill,
                written.as_deref(),
                &documents.brief,
                documents.handoff(),
            ),
        ));
    }

    Some(options)
}

/// Why `process` of `plugin` can't run here, in the words its Notice says.
fn trouble(state: &AppState, plugin: &str, process: &str) -> String {
    match state.plugins.trouble(plugin) {
        Some(why) => format!(
            "The plugin providing the {process} process can't run it: {why} This \
             conversation carries on from where it stands once that's put right."
        ),
        None => format!(
            "No plugin here provides the {process} process: it isn't installed, or it's \
             switched off. This conversation carries on from where it stands once it's back."
        ),
    }
}

/// Write a stop, over the session `writing` printed into where there is one.
async fn stop(
    state: &AppState,
    conversation_id: i64,
    decided: crate::stopping::Decided<'_>,
    what: &str,
    how: &str,
    writing: Option<i64>,
) {
    crate::runner::stop(state, conversation_id, decided, what, how, writing).await;
}

/// Carry on every Conversation a Process in trouble stopped, now that the
/// Processes loaded here have moved: each one standing in a declared state,
/// stopped by nobody's decision, whose Process now loads.
///
/// What the loader calls once a load has changed what is held — see
/// [`crate::plugins::installed`].
pub(crate) async fn taken_back(state: &AppState) {
    let conversations = match store::conversations(&state.pool, false).await {
        Ok(conversations) => conversations,
        Err(error) => {
            tracing::error!(error = ?error, "listing the Conversations a Process's plugin coming back might carry on failed");
            return;
        }
    };

    for row in conversations {
        let Some(declared) = row.declared else {
            continue;
        };

        if state
            .plugins
            .process(&declared.plugin, &declared.process)
            .is_none()
        {
            continue;
        }

        match store::stopped(&state.pool, row.id).await {
            Ok(Some(stopped)) if !stopped.decision.decided() => {}
            Ok(_) => continue,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id = row.id, "reading whether a Conversation was stopped failed");
                continue;
            }
        }

        tracing::info!(
            conversation_id = row.id,
            process = declared.process,
            "a Process's plugin is back, so the Conversation it stopped carries on"
        );

        if let Err(error) =
            crate::resume::resume(state, row.id, crate::resume::Resuming::Restarted).await
        {
            tracing::error!(error = ?error, conversation_id = row.id, "carrying on a Conversation whose Process is back failed");
        }
    }
}
