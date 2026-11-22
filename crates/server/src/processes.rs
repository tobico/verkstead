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
use crate::plugins::manifest::process::{Brief, Process, Role, Runs, State, To};
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
            roles: self
                .process
                .roles
                .iter()
                .map(|role| verkstead_render::RoleEntry {
                    role: role.name.clone(),
                    label: role.label.clone(),
                    building: role.building,
                    away: role.away,
                })
                .collect(),
            built_in: crate::plugins::built_in::stood_for(&self.plugin, &self.process.id)
                .map(|process| crate::ui::process(&process)),
        }
    }
}

/// The Process `process` runs by, where it is data: a plugin's own, or the
/// built-in plugin one of the record's bare words stands for — `tinker` is
/// the *Tinker* plugin's, and `develop` the *Develop* plugin's. Held whether
/// or not it is offered, so a Conversation on a plugin unticked since runs on.
/// `None` for a plugin that isn't held here.
pub(crate) fn of(state: &AppState, process: &store::Process) -> Option<Loaded> {
    match process {
        store::Process::Provided { plugin, id } => state.plugins.process(plugin, id),
        built_in => {
            let (plugin, id) = crate::plugins::built_in::standing_for(built_in)?;
            built_ins()
                .iter()
                .find(|loaded| loaded.plugin == plugin && loaded.process.id == id)
                .cloned()
        }
    }
}

/// Every Process the built-in plugins provide, each with the Activities its
/// states run found — read out of this binary once, so a press arriving before
/// the installed plugins have loaded still finds them. `offered` here is only
/// a placeholder: whether one is offered is its tick, see [`offered`].
fn built_ins() -> &'static [Loaded] {
    use crate::plugins::built_in::{BUILT_INS, BuiltIn};
    use crate::plugins::manifest::Manifest;
    use std::sync::LazyLock;

    static HELD: LazyLock<Vec<Loaded>> = LazyLock::new(|| {
        let read: Vec<(BuiltIn, Manifest)> = BUILT_INS
            .iter()
            .filter_map(|built_in| {
                Some((
                    *built_in,
                    built_in.manifest(crate::platform::Platform::HERE).ok()?,
                ))
            })
            .collect();
        let source = |uuid: &str| {
            read.iter()
                .find(|(_, manifest)| manifest.uuid == uuid)
                .map(|(built_in, manifest)| (Source::BuiltIn(*built_in), manifest))
        };

        read.iter()
            .filter(|(_, manifest)| !crate::plugins::built_in::unlisted(&manifest.uuid))
            .flat_map(|(_, manifest)| {
                crate::plugins::installed::reaching(manifest, &source)
                    .into_iter()
                    .map(|(process, activities)| Loaded {
                        plugin: manifest.uuid.clone(),
                        process,
                        offered: true,
                        activities,
                    })
            })
            .collect()
    });

    &HELD
}

/// Whether a new Conversation may be started on `process`: a plugin's while
/// it is loaded, switched on and in no trouble; a built-in plugin's while its
/// tick stands — Develop's included, since stage 11 wrote it as data.
pub(crate) async fn offered(state: &AppState, process: &store::Process) -> bool {
    match process {
        store::Process::Provided { plugin, id } => state.plugins.offers(plugin, id),
        // Each of the five is a built-in plugin of its own, so every arm here
        // reads a tick. A word none of them stands for could only be a
        // record this build does not understand, and nothing is refused over
        // one.
        built_in => match crate::plugins::built_in::standing_for(built_in) {
            Some((plugin, _)) => ticked(state, plugin).await,
            None => true,
        },
    }
}

/// Whether the built-in `plugin`'s tick stands: it does until somebody
/// unticks it. A store that won't say reads as ticked, so nothing is refused
/// over a database nobody could read.
async fn ticked(state: &AppState, plugin: &str) -> bool {
    match store::replicated::live_records::<crate::replicating::plugins::Plugins>(&state.pool).await
    {
        Ok(records) => records
            .iter()
            .find(|record| record.uuid == plugin)
            .is_none_or(|record| record.fields.enabled),
        Err(error) => {
            tracing::error!(error = ?error, plugin, "reading whether a built-in Process is ticked failed");
            true
        }
    }
}

/// Every Process the picker may offer and every Conversation is worded from:
/// the built-ins', each offered while ticked, then the installed plugins'.
pub(crate) async fn listed(state: &AppState) -> Vec<Loaded> {
    let mut listed = Vec::new();
    for loaded in built_ins() {
        listed.push(Loaded {
            offered: ticked(state, &loaded.plugin).await,
            ..loaded.clone()
        });
    }
    listed.extend(state.plugins.processes());
    listed
}

/// Where `outcome` leads from the state `standing`, as `process`'s manifest
/// says — `None` where it is not data, or says nothing about that state,
/// which is the caller's own reading to fall back on.
pub(crate) fn leads(
    state: &AppState,
    process: &store::Process,
    standing: &str,
    outcome: &str,
) -> Option<To> {
    of(state, process)?
        .process
        .state(standing)?
        .transitions
        .iter()
        .find(|(mapped, _)| mapped == outcome)
        .map(|(_, to)| to.clone())
}

/// Whether the state `standing` of `process`'s manifest runs the Activity
/// `activity`.
pub(crate) fn runs(
    state: &AppState,
    process: &store::Process,
    standing: &str,
    activity: &str,
) -> bool {
    of(state, process)
        .and_then(|loaded| {
            let declared = loaded.process.state(standing)?.activity.clone();
            Some(declared.activities().iter().any(|(_, id)| *id == activity))
        })
        .unwrap_or(false)
}

/// What a host driver does with an Outcome of one of Verkstead's own
/// Activities, as the Conversation's Process's manifest says where it leads:
/// the landing that host already knows for each kind of place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Leaving {
    /// The Process is not data, or says nothing of that state: the host's
    /// own reading, which is what Develop's manifest will say.
    AsEver,
    /// Into a state that wraps up.
    ToTheWrapUp,
    Done,
    Closed,
    Stopped,
    /// Into a state no host driver enters yet, by its id.
    Elsewhere(String),
}

/// Where `outcome` of the state `standing` takes a Conversation on `process`,
/// as a host driver acts on it — see [`Leaving`].
pub(crate) fn leaving(
    state: &AppState,
    process: &store::Process,
    standing: &str,
    outcome: &str,
) -> Leaving {
    match leads(state, process, standing, outcome) {
        None => Leaving::AsEver,
        Some(To::Done) => Leaving::Done,
        Some(To::Closed) => Leaving::Closed,
        Some(To::Stopped) => Leaving::Stopped,
        Some(To::State(next)) if runs(state, process, &next, "wrap-up") => Leaving::ToTheWrapUp,
        Some(To::State(next)) => Leaving::Elsewhere(next),
    }
}

/// The stop a host driver writes where a Process sends an Outcome somewhere
/// it can't take a Conversation: `stopped`, or a state it has no way into.
pub(crate) async fn stopped_by(
    state: &AppState,
    conversation_id: i64,
    doing: &str,
    activity: &str,
    outcome: &str,
    leaving: &Leaving,
) {
    let how = match leaving {
        Leaving::Elsewhere(next) => format!(
            "{activity} ended in `{outcome}`, which leads to `{next}`, and Verkstead can't \
             enter that state from here yet."
        ),
        _ => format!(
            "{activity} ended in `{outcome}`, and this process stops here until you press Resume."
        ),
    };

    stop(
        state,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        doing,
        &how,
        None,
    )
    .await;
}

/// The options `process`'s manifest gives the state `standing`, or none.
pub(crate) fn options_of(
    state: &AppState,
    process: &store::Process,
    standing: &str,
) -> Vec<(String, String)> {
    of(state, process)
        .and_then(|loaded| Some(loaded.process.state(standing)?.options.clone()))
        .unwrap_or_default()
}

/// The Directions the interview of a Conversation on `process` offers, in the
/// order the chooser draws them: the ones its manifest maps to a state
/// (Set 1765, Q27) — all three for Develop, which maps each of them to
/// `implementing`.
///
/// All three where the Process is not data or maps none, which is the reading
/// a chooser has always had: a Process that offers nothing to pick between is
/// one whose interview could put no proposal at all.
pub(crate) fn directions(
    state: &AppState,
    process: &store::Process,
) -> Vec<verkstead_schema::Direction> {
    let mapped: Vec<verkstead_schema::Direction> = of(state, process)
        .map(|loaded| {
            loaded
                .process
                .directions
                .iter()
                .filter_map(|(word, _)| direction_named(word))
                .collect()
        })
        .unwrap_or_default();

    match mapped.is_empty() {
        true => verkstead_schema::Direction::ALL.to_vec(),
        false => mapped,
    }
}

/// Whether a Draft of `process` takes a Target, as its manifest says.
pub(crate) fn takes_a_target(state: &AppState, process: &store::Process) -> bool {
    of(state, process).is_some_and(|loaded| loaded.process.target)
}

/// Whether `process`'s wrap-up is narrowed to what GitHub refuses a merge
/// for: the options of the state it wraps up in turn the Wrap-up's `review`
/// and `comments` off. Read off the options alone, as the Wrap-up itself
/// reads them — never off which Process it is.
pub(crate) fn narrows_the_wrap_up(state: &AppState, process: &store::Process) -> bool {
    let options = options_of(state, process, WRAPPING);
    let off = |key: &str| {
        options
            .iter()
            .any(|(named, value)| named == key && value == "off")
    };

    off("review") && off("comments")
}

/// The state a built-in Process wraps up in, by the word the record holds.
pub(crate) const WRAPPING: &str = "wrapping";

/// The word of the state a Conversation stands in, as its Process's manifest
/// names it.
///
/// A declared state carries its id; one of Verkstead's own carries the word the
/// `conversations.state` column holds, which is the id the five built-in
/// plugins give their states — see [`crate::plugins::built_in`].
pub(crate) fn standing_in(conversation: &store::Conversation) -> &str {
    conversation
        .declared_state
        .as_deref()
        .unwrap_or_else(|| conversation.state.stored())
}

/// The roles `process` declares, in the order its manifest writes them: what
/// the Agent control draws a picker for, and what a Start waits on (Set 1765,
/// Q26).
///
/// Empty for a Process whose plugin is not loaded, which is a Conversation in
/// trouble — see [`trouble`].
pub(crate) fn roles(state: &AppState, process: &store::Process) -> Vec<Role> {
    of(state, process).map_or_else(Vec::new, |loaded| loaded.process.roles.clone())
}

/// The roles `process` declares, each as its name and the label drawn for it —
/// what a page or a sentence names a role by.
pub(crate) fn labels(state: &AppState, process: &store::Process) -> Vec<(String, String)> {
    roles(state, process)
        .into_iter()
        .map(|role| (role.name, role.label))
        .collect()
}

/// The role `process` marks as building: what the Done signal, Memos and
/// previews assume, and what a state declaring no roles of its own runs under.
pub(crate) fn building(state: &AppState, process: &store::Process) -> Option<Role> {
    of(state, process).map(|loaded| loaded.process.building().clone())
}

/// The roles the state `standing` of `process` runs its sessions under, in the
/// order written — the state's own `roles`, or the building role where it
/// declares none.
///
/// What a steer into that state settles, and what the Pairings a session is
/// launched under are looked up by.
pub(crate) fn roles_in(state: &AppState, process: &store::Process, standing: &str) -> Vec<Role> {
    of(state, process).map_or_else(Vec::new, |loaded| {
        loaded
            .process
            .roles_in(standing)
            .into_iter()
            .cloned()
            .collect()
    })
}

/// Which of the state `standing`'s roles a session an Activity launches naming
/// `word` runs under — see [`Process::under`].
pub(crate) fn under(
    state: &AppState,
    process: &store::Process,
    standing: &str,
    word: &str,
) -> Option<Role> {
    of(state, process).and_then(|loaded| loaded.process.under(standing, word).cloned())
}

/// What a steer into the state `standing` of a Process takes, where its
/// manifest marks it as taking one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mark {
    /// The brief the form draws and the Activity is started with.
    pub(crate) brief: Brief,
    /// Whether it goes back to where it was steered from once its Activity
    /// ends.
    pub(crate) detour: bool,
}

impl Mark {
    fn of(declared: &State) -> Option<Mark> {
        Some(Mark {
            brief: declared.steer?,
            detour: declared.detour,
        })
    }
}

/// The steer mark of the state `standing` of `loaded`, where it takes one.
pub(crate) fn mark(loaded: &Loaded, standing: &str) -> Option<Mark> {
    Mark::of(loaded.process.state(standing)?)
}

/// Each state of `loaded` a steer may go into, in the order written, with
/// its mark.
pub(crate) fn steerable(loaded: &Loaded) -> Vec<(&State, Mark)> {
    loaded
        .process
        .states
        .iter()
        .filter_map(|declared| Some((declared, Mark::of(declared)?)))
        .collect()
}

/// What the human wrote under the steer that put the Conversation in
/// `standing`, where it is still there and they wrote something.
///
/// Read off the record rather than carried in, so a restart or a Resume
/// starts the Activity with it again: the newest Steer went into this state,
/// and nothing has moved the Conversation since.
async fn steered_with(state: &AppState, conversation_id: i64, standing: &str) -> Option<String> {
    let steer = match store::latest_steer(&state.pool, conversation_id).await {
        Ok(steer) => steer?,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a steer was written with failed");
            return None;
        }
    };

    (steer.into == standing && steer.moves_since <= 1)
        .then_some(steer.written)
        .flatten()
}

/// Where a detour of `loaded`, standing in `standing`, goes back to once its
/// Activity ends: the state the steer into it found the Conversation in, or
/// Done where that is one nothing returns to — Draft, Done, Closed, the
/// detour itself — or a state the Process doesn't declare.
async fn homeward(state: &AppState, conversation_id: i64, loaded: &Loaded, standing: &str) -> To {
    let from = match store::latest_steer(&state.pool, conversation_id).await {
        Ok(Some(steer)) if steer.into == standing => steer.from,
        Ok(_) => None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading where a detour was steered from failed");
            None
        }
    };

    match from {
        Some(from) if from != standing && loaded.process.state(&from).is_some() => To::State(from),
        _ => To::Done,
    }
}

/// The heading a brief of kind `brief` goes under in a session's prompt.
fn heading(brief: Brief) -> Option<&'static str> {
    match brief {
        Brief::Round => Some("The brief for this round"),
        Brief::Instruction => Some("What I want done"),
        Brief::FollowUp => Some("What I want followed up"),
        Brief::Question => Some("What I want found out"),
        Brief::None => None,
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

    // The newest steer as the Activity starts: one after it is the human
    // taking the Conversation elsewhere while it ran.
    let steered = newest_steer(state, conversation_id).await;

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

    let Some(options) = options(state, conversation_id, &standing, declared).await else {
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

    // Taken elsewhere by the human, whose steer says where it goes — and so
    // is an Activity whose session a steer ended as it moved the Conversation
    // on, whatever that ending read as: what it ended in is no longer this
    // state's to follow.
    if ran.outcome == STEERED || !still_in(state, conversation_id, &standing, steered).await {
        return Ok(false);
    }

    // A detour goes back where it was steered from, whatever it ended in.
    let back;
    let to = if declared.detour {
        back = homeward(state, conversation_id, &loaded, &standing).await;
        tracing::info!(
            conversation_id,
            state = standing,
            to = ?back,
            "a detour's Activity ended, so it goes back where it was steered from"
        );
        &back
    } else {
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
        to
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

/// Whether `conversation_id` still stands in the declared state `standing`
/// with no steer since the one that was newest as its Activity started,
/// `steered` — a steer back into the same state included.
async fn still_in(
    state: &AppState,
    conversation_id: i64,
    standing: &str,
    steered: Option<i64>,
) -> bool {
    let here = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.declared_state.as_deref() == Some(standing),
        Ok(None) => false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading where a Conversation stands failed");
            false
        }
    };

    here && newest_steer(state, conversation_id).await == steered
}

/// The newest Steer Event of `conversation_id`, by its id.
async fn newest_steer(state: &AppState, conversation_id: i64) -> Option<i64> {
    match store::latest_steer(&state.pool, conversation_id).await {
        Ok(steer) => steer.map(|steer| steer.event),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a Conversation's newest steer failed");
            None
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

/// And the Direction a manifest's word names, where it names one — the
/// reader refuses any other, so this is only ever `None` for a manifest read
/// by a Verkstead that knows a Direction this one does not.
fn direction_named(word: &str) -> Option<verkstead_schema::Direction> {
    verkstead_schema::Direction::ALL
        .into_iter()
        .find(|direction| direction_word(*direction) == word)
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
    standing: &str,
    declared: &State,
) -> Option<Vec<(String, String)>> {
    let mut options = declared.options.clone();

    // What the human wrote under the steer into it, as the option its kind
    // names — `instruction`, `follow-up`, `question` — and under its heading
    // in the prompt a skill's session is started on.
    let steered = match declared.steer {
        Some(brief @ (Brief::Instruction | Brief::FollowUp | Brief::Question)) => {
            steered_with(state, conversation_id, standing)
                .await
                .map(|written| (brief, written))
        }
        _ => None,
    };
    if let Some((brief, written)) = &steered {
        options.push((brief.word().to_owned(), written.clone()));
    }

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

        let mut prompt = crate::skills::declared(
            documents.skills,
            &skill,
            written.as_deref(),
            &documents.brief,
            documents.handoff(),
        );
        if let Some((brief, written)) = &steered
            && let Some(heading) = heading(*brief)
        {
            prompt = format!(
                "{}\n\n# {heading}\n\n{}\n",
                prompt.trim_end(),
                written.trim()
            );
        }
        options.push(("prompt".to_owned(), prompt));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The built-in Process `id`, as this binary carries it.
    fn built_in(id: &str) -> Loaded {
        built_ins()
            .iter()
            .find(|loaded| loaded.process.id == id)
            .unwrap_or_else(|| panic!("the built-in Process {id} is carried"))
            .clone()
    }

    /// Fix Merge Issues narrows its wrap-up by its wrapping state's options
    /// alone — the Wrap-up's review and comments off — and Review, the same
    /// wrap-up otherwise, turns neither off.
    #[test]
    fn fix_merge_issues_narrows_its_wrap_up_by_its_options_alone() {
        let options = |id: &str| {
            built_in(id)
                .process
                .state(WRAPPING)
                .unwrap()
                .options
                .clone()
        };

        assert_eq!(
            options("fix-merge-issues"),
            [
                ("comments".to_owned(), "off".to_owned()),
                ("review".to_owned(), "off".to_owned()),
            ]
        );
        assert!(options("review").is_empty());
    }

    /// Each of the five starts in the state word the record already holds for
    /// it, with every Activity it runs found in the built-in Activities.
    #[test]
    fn the_five_start_in_the_words_already_stored() {
        for (id, first) in [
            ("develop", "grilling"),
            ("tinker", "follow-up"),
            ("investigate", "investigating"),
            ("review", "wrapping"),
            ("fix-merge-issues", "wrapping"),
        ] {
            let loaded = built_in(id);
            assert_eq!(loaded.process.states[0].id, first, "{id}");
            assert!(
                store::Lifecycle::named(first).is_some(),
                "{first} is one of the words the record holds"
            );
            for state in &loaded.process.states {
                for (_, activity) in state.activity.activities() {
                    assert!(loaded.activity(activity).is_some(), "{id} runs {activity}");
                }
            }
        }
    }

    /// Develop's five states, in the words a Conversation's record already
    /// held: the interview offering all three Directions, each leading to the
    /// one `implementing` state, whose Activity is the Direction's own; then
    /// the wrap-up, the follow-up that hangs off it, and `investigating`, the
    /// detour a question takes.
    #[test]
    fn develop_is_the_ladder_written_as_data() {
        use crate::plugins::manifest::process::{Brief, Runs, To};

        let loaded = built_in("develop");
        let develop = &loaded.process;

        assert!(!develop.target, "Develop is pointed at nothing");
        assert_eq!(develop.building().name, "implementation");
        assert_eq!(
            develop
                .roles
                .iter()
                .map(|role| role.name.as_str())
                .collect::<Vec<&str>>(),
            ["grilling", "implementation", "review"],
        );

        assert_eq!(
            develop.directions,
            [
                ("inline".to_owned(), "implementing".to_owned()),
                ("task-list".to_owned(), "implementing".to_owned()),
                ("roadmap".to_owned(), "implementing".to_owned()),
            ],
            "all three, each leading to the one state that builds",
        );
        assert_eq!(
            develop
                .states
                .iter()
                .map(|state| state.id.as_str())
                .collect::<Vec<&str>>(),
            [
                "grilling",
                "implementing",
                "wrapping",
                "follow-up",
                "investigating"
            ],
        );

        let state = |id: &str| develop.state(id).expect("Develop declares it").clone();

        assert_eq!(
            state("grilling").activity,
            Runs::Activity("interview".to_owned())
        );
        assert_eq!(state("grilling").steer, Some(Brief::Round));
        assert_eq!(
            state("grilling").transitions,
            [("gone".to_owned(), To::Stopped)],
            "the Directions are `directions`' to map, and `gone` is the only other way out",
        );

        assert_eq!(
            state("implementing").activity,
            Runs::ByDirection(vec![
                ("inline".to_owned(), "one-session".to_owned()),
                ("task-list".to_owned(), "backlog-runner".to_owned()),
                ("roadmap".to_owned(), "stage-runner".to_owned()),
            ]),
        );
        assert_eq!(state("implementing").steer, Some(Brief::Instruction));
        assert!(
            state("implementing")
                .transitions
                .contains(&("done".to_owned(), To::State("wrapping".to_owned()))),
            "an inline session that is done wraps up",
        );
        assert!(
            state("implementing")
                .transitions
                .contains(&("refused".to_owned(), To::Stopped)),
        );
        for outcome in ["finished", "continued"] {
            assert!(
                state("implementing")
                    .transitions
                    .contains(&(outcome.to_owned(), To::State("wrapping".to_owned()))),
                "{outcome} wraps up too, as a backlog's and a roadmap's ending do",
            );
        }

        assert_eq!(state("wrapping").steer, Some(Brief::None));
        assert!(state("wrapping").options.is_empty(), "nothing is narrowed");
        assert_eq!(state("follow-up").steer, Some(Brief::FollowUp));
        assert_eq!(
            state("follow-up").transitions,
            [
                ("committed".to_owned(), To::State("wrapping".to_owned())),
                ("nothing".to_owned(), To::State("wrapping".to_owned())),
            ],
            "a follow-up is steered into over a pull request, so either ending leaves \
             that pull request and a wrap-up to see it out",
        );

        assert!(state("investigating").detour);
        assert_eq!(state("investigating").steer, Some(Brief::Question));
        assert!(
            state("investigating").transitions.is_empty(),
            "a detour goes back where it was steered from, whatever it ends in",
        );
    }
}
