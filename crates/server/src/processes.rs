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
                    interviews: state
                        .activity
                        .activities()
                        .iter()
                        .any(|(_, id)| *id == INTERVIEW),
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
            target: self.process.target,
            narrowed: self.narrows_the_wrap_up(),
            filled_from_a_pull_request: self.whose()
                == crate::conversations::filled_from_a_pull_request(),
        }
    }

    /// Which Process it is, as the record holds it and the viewer receives it.
    pub(crate) fn whose(&self) -> store::Process {
        store::Process {
            plugin: self.plugin.clone(),
            id: self.process.id.clone(),
        }
    }

    /// Whether its wrap-up is narrowed to what GitHub refuses a merge for:
    /// the options of its `wrapping` state turn the Wrap-up's `review` and
    /// `comments` off. Read off the options alone, as the Wrap-up itself reads
    /// them — never off which Process it is.
    ///
    /// By the state's id, which is the word the record holds for Verkstead's
    /// own wrap-up: what reads this is the Steer form's note under
    /// Verkstead's own **Wrapping** row, and a Process that declares no state
    /// by that name has no such row to say anything under.
    fn narrows_the_wrap_up(&self) -> bool {
        let Some(wrapping) = self.process.state(WRAPPING) else {
            return false;
        };
        let off = |key: &str| {
            wrapping
                .options
                .iter()
                .any(|(named, value)| named == key && value == "off")
        };

        off("review") && off("comments")
    }

    /// And whether its wrap-up is over a link of a chain rather than over one
    /// pull request: the `stack` option of its wrapping state, which *Fix
    /// merge issues* turns on.
    ///
    /// Read off the options as the narrowing is, and for the same reason: what
    /// a state does is what its Activity is told to do, never which Process it
    /// belongs to. A conflict low in a stack moves every branch above it, so a
    /// wrap-up told this walks the chain on GitHub both ways from what it was
    /// pointed at and records every pull request of it.
    fn walks_the_stack(&self) -> bool {
        self.process.state(WRAPPING).is_some_and(|wrapping| {
            wrapping
                .options
                .iter()
                .any(|(named, value)| named == "stack" && value == "on")
        })
    }
}

/// The Process `process` names, where a plugin here provides it: an installed
/// plugin's, or one of the five Verkstead ships — `tinker` with no plugin
/// beside it is the *Tinker* plugin's, which is how a Conversation stored
/// before Processes were data finds its manifest. Held whether or not it is
/// offered, so a Conversation on a plugin unticked since runs on. `None` for a
/// plugin that isn't held here.
pub(crate) fn of(state: &AppState, process: &store::Process) -> Option<Loaded> {
    state
        .plugins
        .process(&process.plugin, &process.id)
        .or_else(|| {
            built_ins()
                .iter()
                .find(|loaded| loaded.plugin == process.plugin && loaded.process.id == process.id)
                .cloned()
        })
}

/// Every Process the built-in plugins provide, each with the Activities its
/// states run found — read out of this binary once, so a press arriving before
/// the installed plugins have loaded still finds them. `offered` here is only
/// a placeholder: whether one is offered is its tick, see [`offered`].
///
/// In the order the picker draws their rows, which is
/// [`crate::plugins::built_in::PROCESSES`]' and not the directory order the
/// built-ins are embedded in: this list *is* the picker's list now that the
/// viewer keeps none of its own (Set 1765, Q29).
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

        let mut held: Vec<Loaded> = read
            .iter()
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
            .collect();

        let order = |loaded: &Loaded| {
            crate::plugins::built_in::PROCESSES
                .iter()
                .position(|uuid| *uuid == loaded.plugin)
                .unwrap_or(usize::MAX)
        };
        held.sort_by_key(order);
        held
    });

    &HELD
}

/// Whether a new Conversation may be started on `process`: a plugin's while
/// it is loaded, switched on and in no trouble; a built-in plugin's while its
/// tick stands — Develop's included, since stage 11 wrote it as data.
pub(crate) async fn offered(state: &AppState, process: &store::Process) -> bool {
    // Each of the five is a built-in plugin of its own, so what is read for
    // one of them is its tick; an installed plugin's is read off what loaded.
    match process.built_in() {
        true => ticked(state, &process.plugin).await,
        false => state.plugins.offers(&process.plugin, &process.id),
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

/// Which Activity the state `standing` of `process`'s manifest runs, where it
/// runs one that does not depend on the Conversation's Direction.
///
/// What a host driver dispatches on: a Resume reads the standing state's
/// Activity and starts that Activity again, which is the restart rule read off
/// the manifest rather than off a list of states (ADR-0030, *Process*). A state
/// whose Activity is the Direction's own answers with every one it may run, so
/// a caller asking whether it builds gets a yes whichever Direction was picked.
pub(crate) fn activities(
    state: &AppState,
    process: &store::Process,
    standing: &str,
) -> Vec<String> {
    of(state, process)
        .and_then(|loaded| {
            let declared = loaded.process.state(standing)?.activity.clone();
            Some(
                declared
                    .activities()
                    .iter()
                    .map(|(_, id)| (*id).to_owned())
                    .collect(),
            )
        })
        .unwrap_or_default()
}

/// The state of `process` that runs the Activity `activity`, by its id — the
/// first its manifest declares, where more than one does.
///
/// What a host driver reads to find where a Conversation goes next without an
/// Outcome to map: a pull request being recorded lands in the state that wraps
/// up, whatever that Process calls it.
pub(crate) fn running(
    state: &AppState,
    process: &store::Process,
    activity: &str,
) -> Option<store::State> {
    let loaded = of(state, process)?;

    loaded
        .process
        .states
        .iter()
        .find(|declared| {
            declared
                .activity
                .activities()
                .iter()
                .any(|(_, id)| *id == activity)
        })
        .map(|declared| store::State::named(&declared.id))
}

/// The state a stage Conversation is built in: the one its Process's manifest
/// maps the task-list Direction to, a stage being worked a task at a time.
///
/// A Process whose manifest this build cannot read lands in `implementing`, the
/// word a stage has always been started in — which is what Develop declares,
/// and Develop is what every stage runs under.
pub(crate) async fn staging(state: &AppState, conversation_id: i64) -> store::State {
    let process = match store::process(&state.pool, conversation_id).await {
        Ok(process) => Some(process),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which Process a stage is on failed");
            None
        }
    };

    process
        .and_then(|process| directed(state, &process, verkstead_schema::Direction::TaskList))
        .unwrap_or_else(|| store::State::named("implementing"))
}

/// And the state `process`'s interview sends a Conversation to once the human
/// has picked `direction`, by its id.
///
/// The manifest's `directions` map, which is where the Outcomes the interview
/// ends a round in lead (Set 1765, Q27) — Develop maps all three to the one
/// state that builds. `None` where it offers that Direction nowhere, which is
/// an interview whose chooser never put it up.
pub(crate) fn directed(
    state: &AppState,
    process: &store::Process,
    direction: verkstead_schema::Direction,
) -> Option<store::State> {
    let loaded = of(state, process)?;
    let word = direction_word(direction);

    loaded
        .process
        .directions
        .iter()
        .find(|(named, _)| named == word)
        .map(|(_, to)| store::State::named(to))
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
/// for, as its manifest says — see [`Loaded::narrows_the_wrap_up`], which is
/// the same reading sent to the picker.
pub(crate) fn narrows_the_wrap_up(state: &AppState, process: &store::Process) -> bool {
    of(state, process).is_some_and(|loaded| loaded.narrows_the_wrap_up())
}

/// And whether its wrap-up is over a whole stack, as its manifest says — see
/// [`Loaded::walks_the_stack`].
pub(crate) fn walks_the_stack(state: &AppState, process: &store::Process) -> bool {
    of(state, process).is_some_and(|loaded| loaded.walks_the_stack())
}

/// The state the five Processes Verkstead ships wrap up in, by the word the
/// record holds.
///
/// Read for the **Wrapping** row on the Steer form and the note under it, which
/// are Verkstead's own rows on that form while the five are steered by its own
/// targets — see [`crate::steering`]. Everything else finds the wrap-up by the
/// Activity it runs, see [`running`].
pub(crate) const WRAPPING: &str = "wrapping";

/// The id the Wrap-up Activity has in the built-in Activities plugin's
/// manifest.
pub(crate) const WRAP_UP: &str = "wrap-up";

/// The ids the two Activities Verkstead reads a Conversation's own drawing off
/// have in the built-in Activities plugin's manifest.
///
/// **What is running decides, not what the state is called** (Set 1765, Q29):
/// a Set asked from Follow-up rounds carries the Nothing-else box and a Set
/// asked from the Interview honours a `proposal`, whatever the state a Process
/// declared for it is named.
pub(crate) const INTERVIEW: &str = "interview";
pub(crate) const FOLLOW_UP_ROUNDS: &str = "follow-up-rounds";

/// And the three a state that builds may run, one per Direction — see
/// Develop's `implementing`, whose Activity is the Direction's own.
///
/// What a host driver dispatches on where it used to match the state: a Resume
/// on a state running any of these starts the run again, and the run reads the
/// Direction for itself.
pub(crate) const BUILDERS: [&str; 3] = ["one-session", "backlog-runner", "stage-runner"];

/// Whether what is running on `conversation_id` is the **Interview**, so a
/// `proposal` on its Set is one there is still something to inform about —
/// see [`crate::conversations::settle_a_proposal`].
pub(crate) async fn interviewing(state: &AppState, conversation_id: i64) -> bool {
    match standing_under(state, conversation_id).await {
        Some((word, process)) => runs(state, &process, &word, INTERVIEW),
        None => false,
    }
}

/// Whether `conversation_id` stands in a state whose Activity is the Wrap-up,
/// whatever its Process calls that state.
///
/// What the wrap-up's own watchers ask before they go to the network, and what
/// a pull request being recorded is the move into: a Conversation that has been
/// closed, steered or finished out from under them is no longer wrapping
/// anything up.
pub(crate) async fn wrapping_up(state: &AppState, conversation_id: i64) -> bool {
    match standing_under(state, conversation_id).await {
        Some((word, process)) => runs(state, &process, &word, WRAP_UP),
        None => false,
    }
}

/// And which ending the Nothing-else box on a Set of `conversation_id` would
/// bring about, or `None` where what is running takes no such mark.
///
/// **Follow-up rounds is the Activity that takes it**, and what it is told to
/// keep as evidence is what says which ending it is: `investigated` is an
/// investigation, which ends where it was asked from with nothing committed,
/// and anything else is a follow-up, which ends in the wrap-up. The same two
/// facts the component itself reads off its options — see the Activity's own
/// `plugins/activities/follow-up-rounds`.
pub(crate) async fn ending(
    state: &AppState,
    conversation_id: i64,
) -> Option<verkstead_render::Ending> {
    let (word, process) = standing_under(state, conversation_id).await?;

    if !runs(state, &process, &word, FOLLOW_UP_ROUNDS) {
        return None;
    }

    let investigating = options_of(state, &process, &word)
        .iter()
        .any(|(key, value)| key == "evidence" && value == "investigated");

    Some(match investigating {
        true => verkstead_render::Ending::Investigation,
        false => verkstead_render::Ending::FollowUp,
    })
}

/// The word `conversation_id` stands in with the Process it reads against, or
/// nothing at all where the record will not say.
async fn standing_under(
    state: &AppState,
    conversation_id: i64,
) -> Option<(String, store::Process)> {
    match store::state_under(&state.pool, conversation_id).await {
        Ok(standing) => standing,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading where a Conversation stands failed");
            None
        }
    }
}

/// The id of the state a Conversation stands in, as its Process's manifest
/// names it — which is the word its `state` column holds.
///
/// The five Verkstead ships name their states with the words that column has
/// always held, so a Grilling answers `grilling` here and is read against
/// Develop's manifest — see [`crate::plugins::built_in`].
pub(crate) fn standing_in(conversation: &store::Conversation) -> &str {
    conversation.state.stored()
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
///
/// And, beside it, the Steer Event that brought the Conversation here, which is
/// where what its checkouts already held is written down — the pair
/// [`crate::investigations::Landing`] is for an investigation, read for the
/// same reason by the same function. `None` where no steer put it here, which
/// is a detour entered some other way and so one with no scratch recorded.
async fn homeward(
    state: &AppState,
    conversation_id: i64,
    loaded: &Loaded,
    standing: &str,
) -> (To, Option<i64>) {
    let steer = match store::latest_steer(&state.pool, conversation_id).await {
        Ok(Some(steer)) if steer.into == standing => Some(steer),
        Ok(_) => None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading where a detour was steered from failed");
            None
        }
    };

    let steered = steer.as_ref().map(|steer| steer.event);

    let to = match steer.and_then(|steer| steer.from) {
        Some(from) if from != standing && loaded.process.state(&from).is_some() => To::State(from),
        _ => To::Done,
    };

    (to, steered)
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
    let Some(standing) = conversation.state.declared().map(str::to_owned) else {
        return Ok(false);
    };

    // The five Verkstead ships run on the host's own drivers, their states
    // being the ones it words itself — see the module above.
    if conversation.process.built_in() {
        return Ok(false);
    }
    let store::Process { plugin, id } = &conversation.process;

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

    // A detour goes back where it was steered from, whatever it ended in —
    // and its scratch comes out of the checkouts on the way, which is what
    // `detoured` is carried down to the move for.
    let back;
    let mut detoured = None;
    let to = if declared.detour {
        let (homeward, steered) = homeward(state, conversation_id, &loaded, &standing).await;
        back = homeward;
        detoured = steered;
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
            let moved = moved(
                state,
                conversation_id,
                &standing,
                &store::State::named(next),
            )
            .await?;

            // A detour hands the checkouts back to a state that will be worked
            // in, so what it wrote in them comes out first — after the move,
            // which is what says this detour is the one that ended, and before
            // the loop below starts the next state's Activity, which is the
            // first thing that would work in there. `detoured` is `None` for
            // every other move, which is nothing to put back. See
            // [`crate::investigations::tidied`].
            if moved {
                crate::investigations::tidied(state, conversation_id, detoured).await;
            }

            Ok(moved)
        }
        To::Done => {
            moved(state, conversation_id, &standing, &store::State::done()).await?;
            state.sessions.end(conversation_id).await;
            // Nothing is put back for a detour that ended Done, as nothing is
            // for an investigation that did: the Worktree goes with the close,
            // and nothing is going to work in there before then — see
            // [`crate::activities::follow_up_rounds`].
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
        Ok(Some(conversation)) => conversation.state.declared() == Some(standing),
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
    to: &store::State,
) -> anyhow::Result<bool> {
    let moved =
        store::transition(&state.pool, conversation_id, &store::State::named(from), to).await?;

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

/// The options `declared` starts its Activity with: the host's own facts where
/// the Activity reads any, the manifest's, and a `prompt` over the
/// Conversation's documents where the state names a skill.
///
/// **What only the host can answer, the host answers** — the Wrap-up's `poll`,
/// `attempts` and `review-after`, which are this server's pace and budget
/// rather than anything a manifest shared between devices could say. Written
/// first, so the manifest cannot move this machine off them; the host driver
/// that runs the five writes the same three the same way round, see
/// [`crate::activities::wrap_up::pacing`].
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
    // The host's own facts about this machine and this run come first, where
    // the Activity is one that reads them: the Wrap-up's pace, the goes a red
    // check gets, the review window. A manifest cannot know any of the three
    // — see [`crate::activities::wrap_up::pacing`].
    let mut options = match declared
        .activity
        .activities()
        .iter()
        .any(|(_, id)| *id == WRAP_UP)
    {
        true => crate::activities::wrap_up::pacing(state),
        false => Vec::new(),
    };
    options.extend(declared.options.clone());

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
/// Processes loaded here have moved: each one standing in a state a plugin's
/// Process declares, stopped by nobody's decision, whose Process now loads.
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
        if row.state.declared().is_none() || row.process.built_in() {
            continue;
        }
        let store::Process { plugin, id } = &row.process;

        if state.plugins.process(plugin, id).is_none() {
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
            process = id,
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
                ("stack".to_owned(), "on".to_owned()),
            ]
        );
        assert!(options("review").is_empty());

        assert!(built_in("fix-merge-issues").narrows_the_wrap_up());
        assert!(!built_in("review").narrows_the_wrap_up());
    }

    /// And it is the one whose wrap-up is over a whole stack, which is its
    /// `stack` option and nothing about which Process it is: the chain the
    /// pull request is a link of is walked on GitHub and recorded whole.
    #[test]
    fn fix_merge_issues_walks_the_stack_by_its_options_alone() {
        assert!(built_in("fix-merge-issues").walks_the_stack());

        for id in ["develop", "tinker", "investigate", "review"] {
            assert!(
                !built_in(id).walks_the_stack(),
                "{id} is pointed at work on the one branch",
            );
        }
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
                !store::State::named(first).verksteads(),
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
