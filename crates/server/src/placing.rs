//! Moving a task of the backlog to the platform its line declares, or an inline
//! run to the one platform its proposal wants, before its session starts
//! (ADR-0029, *Placing declared work*).
//!
//! **Verkstead moves declared work; the agent never calls `verkstead transfer`
//! for it.** A line of `.tasks/TODO.md` may say `on windows` after its link —
//! see [`crate::declarations::wants`] — and the runner, about to launch that
//! task, asks here first. Where this device is already of that platform nothing
//! happens. Where it is not, the Conversation is moved by the ordinary
//! **Transfer**, asked on Verkstead's own account for the task, and the runner
//! stands down: nothing is running, so the move runs where it stands, and the far
//! end's arrival presses its own Resume, which reaches this same question there
//! and finds the task on its platform.
//!
//! **Asked at every launch of a declared task, a Resume's included.** The
//! declaration is what the task wants wherever the Conversation happens to
//! stand when it is launched, so a task moved back by hand is moved again by the
//! next launch of it. And nothing ever moves the work home: an undeclared task,
//! the finish and the wrap-up run wherever the last move left it.
//!
//! **The device is the first permitted one of that platform that answers
//! cleanly.** Permitted by the rule the agent's own call is held to — the
//! drafting device and then the ticks, in that order, never this device; see
//! [`crate::transfers::cluster`] — and of that platform by its member's OS word
//! read through [`platform::os_mark`], never matched as a string: *Linux (WSL)*
//! is a Linux device, so `on linux` may pick a Windows machine's WSL and `on
//! windows` never does. Clean is the **Preflight** holding nothing: a device
//! that is asleep, or that answers and lacks the Repo, the harness or the
//! Author, is passed over for the next.
//!
//! **Where devices of the platform are linked and none is permitted, the human
//! is asked** (ADR-0029, *Verkstead asks with a Question Set of its own*): a
//! Set of Verkstead's own offering *Move to* each of them against *Run it here*,
//! and the runner stands down. The open Set is the wait: it holds the
//! Conversation on the drivers register until it settles — see
//! [`crate::own_sets::wait`] — and any launch reaching the same work while it is
//! open, a restart's included, waits on it again rather than asking twice. The
//! answer is acted on as it is taken — see [`answered`] — and a launch after
//! *Run it here* runs the work here without asking again.
//!
//! **Where no device will do otherwise, the task runs here**, and a Notice says
//! why — see [`Unplaced`], one case for each reason, each its own sentence,
//! because the platform is a preference rather than a condition of the work.
//! Written at every launch that meets it, a Resume's included, since each
//! launch asked again.
//!
//! **A roadmap stage's tasks are placed like any other.** Moving a stage's
//! Conversation off its roadmap's home device leaves it in that roadmap's sight:
//! the copy reports to home and asks it before joining (ADR-0029, *Home drives a
//! roadmap across devices*), whoever asked for the move.
//!
//! **The agent's own call to a device that cannot be used yet is asked about
//! the same way** (ADR-0029, *Verkstead asks with a Question Set of its own*):
//! `verkstead transfer` naming a device not ticked, or ticked and asleep, puts
//! the same Set with *Keep it here* in place of *Run it here* — see [`called`]
//! — and *Keep it here* carries the session on where it is: see [`kept`].
//!
//! **An inline run is placed only where it starts** (Set 1543, Q5b): see
//! [`inline`], asked once, as the grilling hands off. A proposal wanting
//! several platforms, or none, starts where it is and the agent moves itself
//! as the work needs; and a Resume of the run never asks, because once it has
//! started, where it is is the agent's and the human's to decide.

use std::future::Future;
use std::path::Path;

use crate::AppState;
use crate::platform::{self, Platform};
use crate::store;

/// What the runner is to do about the task it was about to launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placed {
    /// Launch it here: it wants no platform, this device is of the one it
    /// wants, or no device of that platform can take it.
    Here,

    /// Stand down: the move is asked for, and the far end's Resume launches it.
    Moving,

    /// Stand down: the human is asked where it runs, with a Question Set of
    /// Verkstead's own — or was already, and has not answered — and their answer
    /// launches it — see [`answered`].
    Asked,
}

/// What is being placed, which the Notice and both Timelines name.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Work {
    /// A task of the backlog, by its number and its label as the list writes
    /// it.
    Task { number: u32, label: String },

    /// The inline run, its implementation session about to start.
    Inline,

    /// The session's own call to move the work, to a device that is linked and
    /// not permitted, or permitted and asleep — see [`called`]. What it "wants"
    /// is that device's platform, and its last Option is *Keep it here*.
    Called,
}

impl Work {
    /// The work as the start of a sentence names it.
    fn named(&self) -> String {
        match self {
            Work::Task { label, .. } => format!("Task {label}"),
            Work::Inline => "The inline run".to_owned(),
            Work::Called => "The work".to_owned(),
        }
    }

    /// Who the move is asked by: Verkstead, for this work.
    fn asking(&self, wants: Platform) -> store::AskedBy {
        self.asked_for(&platform::os_word(wants, None))
    }

    /// The same, the platform written as the human writes it.
    fn asked_for(&self, wants: &str) -> store::AskedBy {
        let wants = wants.to_owned();

        match self {
            Work::Task { label, .. } => store::AskedBy::Task {
                label: label.clone(),
                wants,
            },
            Work::Inline => store::AskedBy::Inline { wants },

            // The human's answer, or the device's waking, is the consent; the
            // move is still the one the session asked for.
            Work::Called => store::AskedBy::Session,
        }
    }

    /// The work as the record of a Set of Verkstead's own keeps it.
    fn placed(&self) -> store::PlacedWork {
        match self {
            Work::Task { number, label } => store::PlacedWork::Task {
                number: *number,
                label: label.clone(),
            },
            Work::Inline => store::PlacedWork::Inline,
            Work::Called => store::PlacedWork::Called,
        }
    }

    /// And the work that record names.
    fn from_placed(placed: &store::PlacedWork) -> Work {
        match placed {
            store::PlacedWork::Task { number, label } => Work::Task {
                number: *number,
                label: label.clone(),
            },
            store::PlacedWork::Inline => Work::Inline,
            store::PlacedWork::Called => Work::Called,
        }
    }
}

/// A member of the cluster as a Set names it and an answer moves the work to.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Device {
    /// Its Device Id.
    id: String,

    /// Its name, as the human gave it.
    name: String,
}

/// Why work that wants a platform cannot be moved to it, each the reason one
/// Notice gives — and each its own case, because the human is asked about one
/// of them rather than told: see [`Unplaced::NonePermitted`].
///
/// Every case is a way of there being *none*, which is what each name says.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Unplaced {
    /// No member of that platform is linked, this Verkstead being in no cluster
    /// included. Nothing to ask anybody: there is nowhere to go.
    NoneLinked,

    /// Members of it are linked and none is permitted: which they are, in the
    /// order they would be tried. One of the two cases the human is asked about
    /// rather than told — see [`ask`].
    NonePermitted(Vec<Device>),

    /// Permitted members of it, none able to take the work and at least one of
    /// them asleep: those, in the order they are tried, and the members of it
    /// nobody permitted, which the same Set asks about. The other case the
    /// human is asked about — and the one that settles itself, the first of
    /// the sleeping ones to wake taking the work.
    Asleep {
        asleep: Vec<Device>,
        unpermitted: Vec<Device>,
    },

    /// Permitted members of it, every one awake and none of which can take the
    /// work: what each one's preflight found, in the Transfer dialog's words,
    /// which name it.
    NoneCanTake(Vec<String>),
}

impl Unplaced {
    /// The Notice, naming the work — a task by its label — and the platform as
    /// the human writes it.
    fn said(&self, work: &Work, wants: Platform) -> String {
        let wants = platform::os_word(wants, None);
        let named = work.named();

        match self {
            Unplaced::NoneLinked => format!(
                "{named} wants {wants}, but no {wants} device is linked, so it's running here."
            ),

            // A tick is advice for the tasks after this one; an inline run is
            // placed only as it starts, so there is no later one for it to help.
            Unplaced::NonePermitted(devices) => {
                let mut said = format!(
                    "{named} wants {wants}, but this conversation may not be transferred to {}, \
                     so it's running here.",
                    either(&names(devices)),
                );

                if let Work::Task { .. } = work {
                    said.push_str(&format!(
                        " Tick a {wants} device under **May be transferred to** to have tasks \
                         like it moved there."
                    ));
                }

                said
            }

            Unplaced::Asleep { asleep, .. } => {
                let awake = match asleep.as_slice() {
                    [_] => "isn't awake",
                    _ => "aren't awake",
                };

                format!(
                    "{named} wants {wants}, but {} {awake}, so it's running here.",
                    either(&names(asleep)),
                )
            }

            Unplaced::NoneCanTake(found) => {
                let mut said = format!(
                    "{named} wants {wants}, but no {wants} device it may be transferred to can \
                     take it, so it's running here.\n"
                );

                for finding in found {
                    said.push_str(&format!("\n- {finding}"));
                }

                said
            }
        }
    }
}

/// The devices' names, in their order.
fn names(devices: &[Device]) -> Vec<String> {
    devices.iter().map(|device| device.name.clone()).collect()
}

/// Names joined as a sentence says them: `X`, `X or Y`, `X, Y or Z`.
fn either(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// Place task `number` of the backlog in `worktree` before it is launched.
pub(crate) async fn task(
    state: &AppState,
    conversation_id: i64,
    worktree: &Path,
    number: u32,
) -> Placed {
    let backlog = worktree.join(crate::tasks::BACKLOG);

    let Some((label, wants)) = crate::tasks::entries(&backlog)
        .into_iter()
        .flatten()
        .find(|entry| entry.number == number)
        .and_then(|entry| Some((entry.label, entry.platform?)))
    else {
        return Placed::Here;
    };

    place(state, conversation_id, Work::Task { number, label }, wants).await
}

/// Place the inline run about to start in `conversation_id`, where the proposal
/// the human picked it on wants exactly one platform.
///
/// Asked once, as the grilling hands off, and never by a Resume of the run —
/// see the module's *placed only where it starts*.
pub(crate) async fn inline(state: &AppState, conversation_id: i64) -> Placed {
    let wanted = match store::picked_wants(&state.pool, conversation_id).await {
        Ok(wanted) => wanted,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what the picked proposal wants failed, so the inline run starts here");
            return Placed::Here;
        }
    };

    let Some(wants) = the_one(&wanted) else {
        return Placed::Here;
    };

    place(state, conversation_id, Work::Inline, wants).await
}

/// The platform a proposal wanting exactly one wants. Several, or none, is
/// work that starts where it is, the agent moving itself as it needs.
fn the_one(wanted: &[verkstead_schema::WantedPlatform]) -> Option<Platform> {
    use verkstead_schema::WantedPlatform;

    match wanted {
        [WantedPlatform::Linux] => Some(Platform::Linux),
        [WantedPlatform::Macos] => Some(Platform::MacOs),
        [WantedPlatform::Windows] => Some(Platform::Windows),
        _ => None,
    }
}

/// Place `work`, which wants `wants`: here where this device is of it, moved
/// where a device of it can take the work, and here with a Notice saying why
/// where none can.
async fn place(state: &AppState, conversation_id: i64, work: Work, wants: Platform) -> Placed {
    // What this device is: its member row's word in a cluster, and the word it
    // would join one with where it is in none.
    let here = match state.devices.as_ref() {
        Some(devices) => devices.os(),
        None => platform::os_word(Platform::HERE, platform::kernel_release().as_deref()),
    };

    if !wants_a_move(&here, wants) {
        return Placed::Here;
    }

    // Where the human was asked about this work and said to run it here, that
    // is where it runs — the answer's own launch reaching here included. And
    // where they were asked and have not answered, the open Set is the wait.
    match asked_before(state, conversation_id, &work).await {
        Before::RunHere => return Placed::Here,
        Before::Open(set_id) => {
            crate::own_sets::wait(state, conversation_id, set_id).await;
            return Placed::Asked;
        }
        Before::Nothing => {}
    }

    match placing(state, conversation_id, &work, wants).await {
        Ok(placed) => placed,

        Err(why @ (Unplaced::NonePermitted(_) | Unplaced::Asleep { .. })) => {
            let (asleep, unpermitted) = match &why {
                Unplaced::NonePermitted(unpermitted) => (&[][..], unpermitted.as_slice()),
                Unplaced::Asleep {
                    asleep,
                    unpermitted,
                } => (asleep.as_slice(), unpermitted.as_slice()),
                _ => unreachable!("matched as one of the two above"),
            };

            match ask(state, conversation_id, &work, wants, asleep, unpermitted).await {
                Ok(()) => {
                    tracing::info!(
                        conversation_id,
                        work = ?work,
                        wants = ?wants,
                        unplaced = ?why,
                        "work wants a platform none of whose devices can be used yet, so the \
                         human is asked where it runs",
                    );
                    Placed::Asked
                }

                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, work = ?work, "asking where work runs failed, so it runs here");
                    unplaced(state, conversation_id, &work, wants, why).await
                }
            }
        }

        Err(why) => unplaced(state, conversation_id, &work, wants, why).await,
    }
}

/// Run `work` here, with a Notice saying why.
async fn unplaced(
    state: &AppState,
    conversation_id: i64,
    work: &Work,
    wants: Platform,
    unplaced: Unplaced,
) -> Placed {
    tracing::info!(
        conversation_id,
        work = ?work,
        wants = ?wants,
        unplaced = ?unplaced,
        "work that wants a platform runs here",
    );

    if let Err(error) = store::note(&state.pool, conversation_id, &unplaced.said(work, wants)).await
    {
        tracing::error!(error = ?error, conversation_id, "saying why work runs here failed");
    }

    Placed::Here
}

/// Put the Question Set of Verkstead's own that asks where `work` runs, none of
/// its platform's devices able to take it yet (ADR-0029, *Verkstead asks with a
/// Question Set of its own*): the permitted ones `asleep`, and the `unpermitted`
/// ones linked. One *Move to* for each unpermitted one, in the order they would
/// be tried, and *Run it here*; nothing for a sleeping one, which the work moves
/// to by itself once it wakes.
///
/// On the Timeline, on the badge and on a phone as any Set is, and held on the
/// drivers register while it is open — see [`crate::own_sets::wait`], which is
/// also what watches for a sleeping one waking. The answer is what launches the
/// work otherwise — see [`answered`].
async fn ask(
    state: &AppState,
    conversation_id: i64,
    work: &Work,
    wants: Platform,
    asleep: &[Device],
    unpermitted: &[Device],
) -> anyhow::Result<()> {
    let conversation = store::load_conversation(&state.pool, conversation_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Conversation {conversation_id} is not there"))?;

    let set = asked(
        work,
        wants,
        asleep,
        unpermitted,
        &conversation.branch,
        &conversation.repo.name,
    );

    let ids = |devices: &[Device]| devices.iter().map(|device| device.id.clone()).collect();

    let about = store::OwnAsking::Placing {
        work: work.placed(),
        wants: platform::os_word(wants, None),
        moves: ids(unpermitted),
        waiting_on: ids(asleep),
    };

    let created = store::ask_on_our_own(&state.pool, conversation_id, &set, &about)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Conversation {conversation_id} is not there"))?;

    crate::own_sets::wait(state, conversation_id, created.id).await;

    crate::push::announce(&state.pool, created.id, &set);
    state.nudges.announce(verkstead_schema::Nudge::Set {
        conversation: conversation_id,
    });

    Ok(())
}

/// The Set that asks where `work` runs, in ADR-0029's words: what wants the
/// platform and why its devices cannot be used yet, a verb on each button, and
/// a title — which is what a phone is told — naming the branch and the device.
///
/// The sleeping devices first, then the unpermitted ones, where it is both.
fn asked(
    work: &Work,
    wants: Platform,
    asleep: &[Device],
    unpermitted: &[Device],
    branch: &str,
    project: &str,
) -> verkstead_schema::QuestionSet {
    use verkstead_schema::{Question, QuestionOption, QuestionSet};

    let (text, title) = match work {
        Work::Called => called_saying(asleep, unpermitted, branch),
        _ => placing_saying(work, wants, asleep, unpermitted, branch),
    };

    // *Keep it here* where the session asked for the move, the work being part
    // way through here already; *Run it here* where Verkstead is placing work
    // that has not started.
    let here = match work {
        Work::Called => "Keep it here",
        _ => "Run it here",
    };

    let options = unpermitted
        .iter()
        .map(|device| format!("Move to {}", device.name))
        .chain([here.to_owned()])
        .zip(1..)
        .map(|(text, n)| QuestionOption {
            n,
            text,
            recommended: false,
            cells: Vec::new(),
        })
        .collect();

    QuestionSet {
        title,
        preface: None,
        questions: vec![Question {
            label: QUESTION.to_owned(),
            text,
            columns: Vec::new(),
            options,
            subquestions: Vec::new(),
        }],
        postscript: None,
        proposal: None,
        project: Some(project.to_owned()),
        branch: Some(branch.to_owned()),
        diff: None,
        diffs: Vec::new(),
    }
}

/// The sentence above the Options and the title, for work Verkstead is placing.
fn placing_saying(
    work: &Work,
    wants: Platform,
    asleep: &[Device],
    unpermitted: &[Device],
    branch: &str,
) -> (String, String) {
    let wants = platform::os_word(wants, None);

    let mut text = format!("{} wants {wants}.", work.named());

    if !asleep.is_empty() {
        let names = names(asleep);

        text.push_str(&match asleep {
            [_] => format!(
                " {} is not awake, and the work moves there by itself once it is.",
                names[0],
            ),
            _ => format!(
                " {} are not awake, and the work moves by itself to the first of them that is.",
                both(&names),
            ),
        });
    }

    if !unpermitted.is_empty() {
        let linked = match unpermitted {
            [_] => "is linked but not permitted",
            _ => "are linked but not permitted",
        };

        text.push_str(&format!(" {} {linked}.", both(&names(unpermitted))));
    }

    // Naming the device a press would move the work to, where there is one, and
    // the one it is waiting on otherwise.
    let title = if unpermitted.is_empty() {
        format!("Run {branch} here, or wait for {}?", either(&names(asleep)))
    } else {
        format!("Move {branch} to {}?", either(&names(unpermitted)))
    };

    (text, title)
}

/// The same for the session's own call, in the ADR's agent rows: the device it
/// asked for and why the work cannot move there yet. One device, either way —
/// a call names one — but said of each where an answer chained more.
fn called_saying(asleep: &[Device], unpermitted: &[Device], branch: &str) -> (String, String) {
    if !unpermitted.is_empty() {
        let names = either(&names(unpermitted));

        return (
            format!("The agent asked to move to {names}, which is not permitted."),
            format!("Move {branch} to {names}?"),
        );
    }

    let names = either(&names(asleep));

    (
        format!(
            "The agent asked to move to {names}, which is not awake, and the work moves there by \
             itself once it is."
        ),
        format!("Keep {branch} here, or wait for {names}?"),
    )
}

/// The label of the one Question a Set of Verkstead's asks.
const QUESTION: &str = "Q1";

/// Names joined as a list of all of them: `X`, `X and Y`, `X, Y, and Z`.
fn both(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// What the human has already been asked about some work, as the newest Set
/// of Verkstead's own about it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Before {
    /// Answered *Run it here*.
    RunHere,

    /// Neither answered nor locked: the work is waiting on it.
    Open(i64),

    /// Never asked, or answered otherwise, or locked unanswered: asked afresh.
    Nothing,
}

/// How the newest Set of Verkstead's own about `work` stands.
///
/// **A task's only within the round of building now under way.** A task is
/// known by its number, and a Conversation sent back to be built works a fresh
/// backlog numbered from `01` again: task 03 answered *Run it here* last round
/// is not this round's task 03 — see [`store::building_since`].
async fn asked_before(state: &AppState, conversation_id: i64, work: &Work) -> Before {
    let own = match store::own_sets(&state.pool, conversation_id).await {
        Ok(own) => own,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Sets Verkstead asked failed");
            return Before::Nothing;
        }
    };

    let since = match work {
        Work::Task { .. } => match store::building_since(&state.pool, conversation_id).await {
            Ok(since) => since,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading when this round of building began failed");
                return Before::Nothing;
            }
        },
        Work::Inline | Work::Called => None,
    };

    let newest = own.into_iter().rev().find(|own| {
        since.is_none_or(|since| own.event_id > since)
            && match &own.asking {
                store::OwnAsking::Placing { work: placed, .. } => {
                    Work::from_placed(placed) == *work
                }
            }
    });

    let Some(newest) = newest else {
        return Before::Nothing;
    };

    let store::OwnAsking::Placing { moves, .. } = &newest.asking;

    match (&newest.response, newest.locked) {
        (Some(response), _) if picked(response) == Some(moves.len() as u32 + 1) => Before::RunHere,
        (None, false) => Before::Open(newest.set_id),
        _ => Before::Nothing,
    }
}

/// Whether the inline run of `conversation_id` is waiting on a Set of
/// Verkstead's own asking where it runs — and if so, wait on it.
///
/// What a Resume of the run asks before launching anything, a restart's
/// included: a Resume never places the run, but one the human has not yet
/// answered about has not started, and starting it here would be answering for
/// them.
pub(crate) async fn inline_waiting(state: &AppState, conversation_id: i64) -> bool {
    match asked_before(state, conversation_id, &Work::Inline).await {
        Before::Open(set_id) => {
            crate::own_sets::wait(state, conversation_id, set_id).await;
            true
        }
        Before::RunHere | Before::Nothing => false,
    }
}

/// The Option picked on a Set of Verkstead's, where one was.
fn picked(response: &verkstead_schema::Response) -> Option<u32> {
    response
        .answers
        .iter()
        .find(|answer| answer.label.trim() == QUESTION)
        .and_then(|answer| answer.selected)
}

/// Act on the answer to Set `set_id` of Verkstead's own asking where work runs,
/// in `conversation_id`: *Move to* ticks that device under **May be
/// transferred to** and moves the work there by the ordinary **Transfer**, on
/// Verkstead's own account, the far end's arrival launching it; *Run it here*
/// launches it here, by the Resume a press would make.
///
/// **A device ticked by the answer and not answering is then waited on**
/// (ADR-0029, *A device both unticked and asleep is asked about as unticked*):
/// a line beside the answer says it was permitted, and the asleep row's Set
/// goes up as the wait, *Run it here* still there to press and the device's
/// waking settling it — see [`woke`].
///
/// `held` is the Set's hold on the drivers register, where it had one: a
/// launch here is handed it, so the place passes to the run without coming
/// free in between, and a move lets it go once it is asked for.
///
/// An answer picking neither is the human not deciding, and nothing is guessed
/// at: the work is left waiting, a Notice saying how it stands.
pub(crate) async fn answered(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
    asking: &store::OwnAsking,
    response: &verkstead_schema::Response,
    held: Option<crate::drivers::Driving>,
) {
    let store::OwnAsking::Placing {
        work,
        wants,
        moves,
        waiting_on,
    } = asking;
    let work = Work::from_placed(work);

    let picked = picked(response).and_then(|n| usize::try_from(n).ok());

    match picked {
        Some(n) if (1..=moves.len()).contains(&n) => {
            let device = &moves[n - 1];

            match crate::transfers::permit(state, conversation_id, device, true).await {
                Ok(permitting) => tracing::info!(
                    conversation_id,
                    device,
                    ?permitting,
                    "the human picked a device for work that wants a platform, so it is ticked",
                ),
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, device, "ticking the device the human picked failed");
                }
            }

            if let Some(asleep) = not_awake(state, conversation_id, device).await {
                let asleep = WaitedOn {
                    already: waiting_on,
                    ticked: asleep,
                };

                waited_on(state, conversation_id, set_id, &work, wants, asleep, held).await;
                return;
            }

            let by = work.asked_for(wants);

            if let Err(error) = crate::transfers::asked(state, conversation_id, device, by).await {
                tracing::error!(error = ?error, conversation_id, device, "asking for the move the human picked failed");
            }
        }

        Some(n) if n == moves.len() + 1 && work == Work::Called => {
            kept(state, conversation_id, set_id, held).await;
        }

        Some(n) if n == moves.len() + 1 => {
            // The launch takes the Set's registration over where it held one,
            // which is also what keeps the press's own check from reading that
            // registration as something already driving the work.
            let resumed = match held {
                Some(held) => crate::resume::landed(state, conversation_id, held).await,
                None => {
                    crate::resume::resume(state, conversation_id, crate::resume::Resuming::Pressed)
                        .await
                }
            };

            match resumed {
                Ok(resumed) => tracing::info!(
                    conversation_id,
                    ?resumed,
                    "the human said to run work that wants a platform here",
                ),
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, "starting work the human said to run here failed");
                }
            }
        }

        _ => {
            if let Err(error) =
                store::note(&state.pool, conversation_id, &still_waiting(&work)).await
            {
                tracing::error!(error = ?error, conversation_id, "saying the work is still waiting failed");
            }
        }
    }
}

/// Which way a session's call to move the work cannot be made yet, which is
/// which row of ADR-0029's table its Set says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Request {
    /// The device is linked, and the human has not ticked it.
    Unpermitted,

    /// The device is permitted, and did not answer its preflight.
    Asleep,
}

/// Put the Set of Verkstead's own that asks about the session's call to move
/// the work to `device` (ADR-0029, *Verkstead asks with a Question Set of its
/// own*): *Move to* it against *Keep it here* where it is not permitted, and
/// *Keep it here* alone where it is permitted and asleep, the device's waking
/// moving the work — see [`woke`].
///
/// The Set's wait holds the place from here — see [`crate::own_sets::wait`] —
/// and the session that called is ended at its turn's end by the caller. Its
/// answer is acted on as any of these is — see [`answered`] — *Keep it here*
/// carrying the session on where it is: see [`kept`].
pub(crate) async fn called(
    state: &AppState,
    conversation_id: i64,
    device: &store::Member,
    request: Request,
) -> anyhow::Result<()> {
    let asked = [Device {
        id: device.device.clone(),
        name: device.name.clone(),
    }];

    let (asleep, unpermitted): (&[Device], &[Device]) = match request {
        Request::Unpermitted => (&[], &asked),
        Request::Asleep => (&asked, &[]),
    };

    // The platform is the device's own, which nothing on the Set says: it is
    // what an answer ticking a device asleep asks again with.
    let wants = platform::os_mark(&device.os).unwrap_or(Platform::HERE);

    ask(
        state,
        conversation_id,
        &Work::Called,
        wants,
        asleep,
        unpermitted,
    )
    .await
}

/// The open Set of Verkstead's own about the session's call, where there is one:
/// the newest one about a call, neither answered nor locked.
pub(crate) async fn called_open(state: &AppState, conversation_id: i64) -> Option<i64> {
    match asked_before(state, conversation_id, &Work::Called).await {
        Before::Open(set_id) => Some(set_id),
        Before::RunHere | Before::Nothing => None,
    }
}

/// Carry the session that called on where it is, the human having answered
/// *Keep it here* to Set `set_id` (ADR-0029): the Carried Conversation's resume
/// with a note saying the work stays — an addition's carrying-on, on another
/// occasion (ADR-0028; see [`crate::carrying::Occasion::Kept`]).
///
/// What the next launch carries on is written down first, and the session that
/// called is ended where it is still running — an answer that beat its turn's
/// end. The launch is handed the Set's hold, so the place passes to it without
/// coming free in between.
async fn kept(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
    held: Option<crate::drivers::Driving>,
) {
    let driving = held.unwrap_or_else(|| state.drivers.driving(conversation_id));

    let written = match store::the_last_session(&state.pool, conversation_id).await {
        Ok(Some(continued)) => {
            let continued = store::Continued {
                widened_by: None,
                kept_here_by: Some(set_id),
                ..continued
            };

            match store::continue_on_arrival(&state.pool, conversation_id, &continued).await {
                Ok(()) => true,
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, "recording the conversation Keep it here carries on failed, so the session started for it will be re-primed instead");
                    false
                }
            }
        }
        Ok(None) => false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the session Keep it here carries on failed, so the session started for it will be re-primed instead");
            false
        }
    };

    if !written {
        crate::carrying::instead(
            &state.pool,
            &state.nudges,
            conversation_id,
            crate::carrying::Occasion::Kept(set_id),
            &crate::carrying::Instead::NoSession,
        )
        .await;
    }

    state.sessions.end(conversation_id).await;

    tracing::info!(
        conversation_id,
        set_id,
        "the human kept the work here, so the session that asked to move it is carried on",
    );

    crate::resume::widened(state.clone(), conversation_id, set_id, driving).await;
}

/// The device `device`, just ticked for work in `conversation_id`, where it
/// did not answer its preflight: asleep, and so waited on rather than moved to.
///
/// `None` where it answered, lacking something or not, and where it could not
/// be asked: the Transfer is asked for, and says what stands in its way.
async fn not_awake(state: &AppState, conversation_id: i64, device: &str) -> Option<Device> {
    let devices = state.devices.as_ref()?;

    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(conversation) => conversation?,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the work a device was ticked for failed");
            return None;
        }
    };

    let config = state.settings.config();

    match crate::preflight::of(Some(devices), &state.pool, &config, &conversation, device).await {
        Ok(reading) => (unclean(&reading) == Some(Unclean::Asleep)).then(|| Device {
            id: device.to_owned(),
            name: reading.device,
        }),
        Err(refusal) => {
            tracing::error!(
                conversation_id,
                device,
                "asking a device just ticked whether it is awake failed: {}",
                refusal.saying,
            );
            None
        }
    }
}

/// The sleeping devices the Set put after an answer waits on.
struct WaitedOn<'a> {
    /// Those the answered Set was waiting on already, by Device Id, where it
    /// asked about an unticked device beside them.
    already: &'a [String],

    /// The one the answer ticked, which did not answer.
    ticked: Device,
}

/// Wait on `asleep.ticked`, the device the answer to Set `set_id` ticked, for
/// `work`: the answer's line says it was permitted, and the asleep row's Set
/// goes up in the answered one's place, holding the work's place on the server
/// from the answered one's `held` without a gap. Any the answered Set was
/// waiting on already are waited on still, ahead of it, as they would be tried.
///
/// Where the Set cannot be put the work is left as an unanswered Set would
/// leave it, a Notice saying so: a launch here would be answering for them.
async fn waited_on(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
    work: &Work,
    wants: &str,
    asleep: WaitedOn<'_>,
    held: Option<crate::drivers::Driving>,
) {
    let line = format!("{} was permitted, and isn't awake yet.", asleep.ticked.name);

    if let Err(error) = store::answered_saying(&state.pool, set_id, &line).await {
        tracing::error!(error = ?error, conversation_id, set_id, "saying the device picked was permitted failed");
    }

    let Some(platform) = platform::os_mark(wants) else {
        tracing::error!(
            conversation_id,
            wants,
            "the platform a Set of Verkstead's asked about is not one this build knows, so the work waits"
        );
        return;
    };

    let mut asleep_now = named(state, asleep.already).await;
    asleep_now.push(asleep.ticked);

    match ask(state, conversation_id, work, platform, &asleep_now, &[]).await {
        Ok(()) => tracing::info!(
            conversation_id,
            work = ?work,
            "the device the human picked is permitted now and asleep, so the work waits on it",
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, work = ?work, "asking again while the device picked is asleep failed");

            if let Err(error) =
                store::note(&state.pool, conversation_id, &still_waiting(work)).await
            {
                tracing::error!(error = ?error, conversation_id, "saying the work is still waiting failed");
            }
        }
    }

    drop(held);
}

/// The members `ids` names, as a Set names them, in that order: one no longer
/// in the cluster is left out.
async fn named(state: &AppState, ids: &[String]) -> Vec<Device> {
    if ids.is_empty() {
        return Vec::new();
    }

    let Some(devices) = state.devices.as_ref() else {
        return Vec::new();
    };

    let members = match devices.membership().rows().await {
        Ok(members) => members,
        Err(error) => {
            tracing::error!(error = ?error, "reading the members a Set of Verkstead's waits on failed");
            return Vec::new();
        }
    };

    ids.iter()
        .filter_map(|id| {
            members
                .iter()
                .find(|member| member.device == *id)
                .map(|member| Device {
                    id: member.device.clone(),
                    name: member.name.clone(),
                })
        })
        .collect()
}

/// The Notice for work an answer left waiting.
fn still_waiting(work: &Work) -> String {
    match work {
        Work::Task { .. } => format!(
            "{} hasn't started. Press **Resume** to be asked again.",
            work.named()
        ),
        Work::Inline => {
            "The inline run hasn't started. Press **Resume** to start it here.".to_owned()
        }
        Work::Called => "The work hasn't moved, and nothing is running. Press **Resume** to \
                         carry it on here."
            .to_owned(),
    }
}

/// Whether a sleeping device Set `set_id` of Verkstead's is waiting on has
/// woken and taken the work, settling the Set: one look, made by the Set's
/// wait — see [`crate::own_sets::wait`].
///
/// The devices are asked in the order they were tried, and the first whose
/// preflight comes back clean takes the work (ADR-0029, *A device that answers
/// before the human does settles the Set itself*): the Set is locked with a
/// line saying that device answered, and the move is asked for by the ordinary
/// **Transfer** on Verkstead's account, the tick already being the consent. The
/// Set's hold on the drivers register is let go once it has been.
///
/// `true` where the Set is settled, by this or by anything else — answered or
/// locked before the lock here could be taken — and there is nothing left to
/// wait on. A device still asleep, or awake and lacking something, is `false`:
/// the human may yet wake it or put it right, and *Run it here* is still there
/// to press.
pub(crate) async fn woke(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
    asking: &store::OwnAsking,
) -> bool {
    let store::OwnAsking::Placing {
        work,
        wants,
        waiting_on,
        ..
    } = asking;
    let work = Work::from_placed(work);

    let Some(devices) = state.devices.as_ref() else {
        return false;
    };

    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return true,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the work a sleeping device is awaited for failed");
            return false;
        }
    };

    let config = state.settings.config();

    let mut answered = None;

    for device in waiting_on {
        match crate::preflight::of(Some(devices), &state.pool, &config, &conversation, device).await
        {
            Ok(reading) if reading.lacks.is_empty() => {
                answered = Some((device, reading.device));
                break;
            }
            Ok(_) => {}
            Err(refusal) => tracing::error!(
                conversation_id,
                device,
                "asking a sleeping device whether it is awake failed: {}",
                refusal.saying,
            ),
        }
    }

    let Some((device, name)) = answered else {
        return false;
    };

    let line = format!("{name} answered, so the work moved there.");

    match store::lock_set_saying(&state.pool, &state.settlements, set_id, &line).await {
        Ok(store::Locking::Locked(_)) => {}

        // Settled some other way first — the human's answer is acted on where it
        // was taken, and their lock is theirs.
        Ok(_) => return true,

        Err(error) => {
            tracing::error!(error = ?error, conversation_id, set_id, "locking a Set the device it waited on answered failed, so the work stays waiting");
            return false;
        }
    }

    state.nudges.announce(verkstead_schema::Nudge::Set {
        conversation: conversation_id,
    });

    let held = state.own_holds.take(set_id);

    match crate::transfers::asked(state, conversation_id, device, work.asked_for(wants)).await {
        Ok(()) => tracing::info!(
            conversation_id,
            work = ?work,
            device,
            "a sleeping device the work was waiting on answered, so the work is being moved there",
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, device, "asking for the move to a device that woke failed");
        }
    }

    drop(held);

    true
}

/// Move the Conversation to the first permitted device of `wants` that can take
/// it, or say why there is none.
async fn placing(
    state: &AppState,
    conversation_id: i64,
    work: &Work,
    wants: Platform,
) -> Result<Placed, Unplaced> {
    let Some(devices) = state.devices.as_ref() else {
        return Err(Unplaced::NoneLinked);
    };

    let read = async {
        anyhow::Ok((
            store::birth(&state.pool, conversation_id).await?,
            store::permitted_devices(&state.pool, conversation_id).await?,
            devices.membership().rows().await?,
            store::load_conversation(&state.pool, conversation_id).await?,
        ))
    };

    let (born, ticked, members, conversation) = match read.await {
        Ok((born, ticked, members, Some(conversation))) => (born, ticked, members, conversation),

        Ok((.., None)) => return Ok(Placed::Here),

        Err(error) => {
            tracing::error!(error = ?error, conversation_id, work = ?work, "reading where work that wants a platform could go failed, so it runs here");
            return Ok(Placed::Here);
        }
    };

    let (candidates, unpermitted) = candidates(
        devices.id(),
        born.as_ref().map(|born| born.device.as_str()),
        &ticked,
        members,
        wants,
    )?;

    let config = state.settings.config();

    let chosen = first_clean(&candidates, |member| {
        let config = &config;
        let conversation = &conversation;

        async move {
            match crate::preflight::of(
                Some(devices),
                &state.pool,
                config,
                conversation,
                &member.device,
            )
            .await
            {
                Ok(reading) => unclean(&reading),

                Err(refusal) => {
                    tracing::error!(
                        conversation_id,
                        device = member.device,
                        "the preflight of a device work could go to failed: {}",
                        refusal.saying,
                    );
                    Some(Unclean::Lacks(vec![format!(
                        "{} couldn't be asked what it has: {}",
                        member.name, refusal.saying,
                    )]))
                }
            }
        }
    })
    .await
    .map_err(|found| none_clean(found, unpermitted))?;

    let asked =
        crate::transfers::asked(state, conversation_id, &chosen.device, work.asking(wants)).await;

    match asked {
        Ok(()) => {
            tracing::info!(
                conversation_id,
                work = ?work,
                device = chosen.device,
                "work wants another platform, so the Conversation is being moved before it is \
                 launched",
            );
            Ok(Placed::Moving)
        }

        Err(error) => {
            tracing::error!(error = ?error, conversation_id, work = ?work, "asking for the move failed, so the work runs here");
            Ok(Placed::Here)
        }
    }
}

/// Whether a task wanting `wants` has to move off a device whose OS word is
/// `here`: only where the word's mark is another platform's, or none.
fn wants_a_move(here: &str, wants: Platform) -> bool {
    platform::os_mark(here) != Some(wants)
}

/// The members a task wanting `wants` may be moved to, in the order they are
/// tried: the permitted ones — the drafting device, then the ticks — whose OS
/// word is that platform's, never `here`.
///
/// And beside them the members of that platform nobody permitted, which a Set
/// asking about the permitted ones being asleep asks about too.
///
/// `Err` where there are none, saying which: no member of that platform at
/// all, or members of it that nobody permitted.
fn candidates(
    here: &str,
    born: Option<&str>,
    ticked: &[String],
    members: Vec<store::Member>,
    wants: Platform,
) -> Result<(Vec<store::Member>, Vec<Device>), Unplaced> {
    let (permitted, unpermitted): (Vec<_>, Vec<_>) =
        crate::transfers::placed_among(here, born, ticked, members)
            .into_iter()
            .filter(|placed| platform::os_mark(&placed.member.os) == Some(wants))
            .partition(|placed| placed.permitted);

    let unpermitted: Vec<Device> = unpermitted
        .into_iter()
        .map(|placed| Device {
            id: placed.member.device,
            name: placed.member.name,
        })
        .collect();

    match (permitted.is_empty(), unpermitted.is_empty()) {
        (false, _) => Ok((
            permitted.into_iter().map(|placed| placed.member).collect(),
            unpermitted,
        )),
        (true, true) => Err(Unplaced::NoneLinked),
        (true, false) => Err(Unplaced::NonePermitted(unpermitted)),
    }
}

/// Why a permitted device cannot take the work, as its preflight reads.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Unclean {
    /// It did not answer: asleep, which settles itself by its waking.
    Asleep,

    /// It answered, lacking these, in the Transfer dialog's words.
    Lacks(Vec<String>),
}

/// What a preflight's `reading` stands in the way of the work, or `None` where
/// nothing does.
fn unclean(reading: &verkstead_render::Preflight) -> Option<Unclean> {
    use verkstead_render::Lacking;

    match reading.lacks.as_slice() {
        [] => None,
        [Lacking::Unreachable] => Some(Unclean::Asleep),
        lacks => Some(Unclean::Lacks(
            lacks
                .iter()
                .map(|lacking| crate::transfers::lacks(lacking, &reading.device))
                .collect(),
        )),
    }
}

/// Why none of the permitted devices can take the work, out of what each one
/// `found`: asleep where any of them is, the human being asked about those and
/// the `unpermitted` ones together; and otherwise what each one lacks.
fn none_clean(found: Vec<(&store::Member, Unclean)>, unpermitted: Vec<Device>) -> Unplaced {
    let asleep: Vec<Device> = found
        .iter()
        .filter(|(_, unclean)| *unclean == Unclean::Asleep)
        .map(|(member, _)| Device {
            id: member.device.clone(),
            name: member.name.clone(),
        })
        .collect();

    if !asleep.is_empty() {
        return Unplaced::Asleep {
            asleep,
            unpermitted,
        };
    }

    Unplaced::NoneCanTake(
        found
            .into_iter()
            .flat_map(|(_, unclean)| match unclean {
                Unclean::Asleep => Vec::new(),
                Unclean::Lacks(lacks) => lacks,
            })
            .collect(),
    )
}

/// The first of `candidates` that `found` finds nothing in the way of, asked in
/// turn rather than all at once: the first clean answer is the whole of what is
/// wanted, and a device later in the list is no reason to wake one earlier.
///
/// `Err` where every one found something, saying what of each.
async fn first_clean<'a, F, Answering>(
    candidates: &'a [store::Member],
    mut found: F,
) -> Result<&'a store::Member, Vec<(&'a store::Member, Unclean)>>
where
    F: FnMut(&'a store::Member) -> Answering,
    Answering: Future<Output = Option<Unclean>>,
{
    let mut findings = Vec::new();

    for member in candidates {
        match found(member).await {
            None => return Ok(member),
            Some(unclean) => findings.push((member, unclean)),
        }
    }

    Err(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HERE: &str = "aa00";

    fn member(device: &str, os: &str) -> store::Member {
        store::Member {
            device: device.to_owned(),
            name: format!("{device}-machine"),
            os: os.to_owned(),
            addresses: vec!["192.168.1.24".to_owned()],
            fingerprint: format!("{device}-print"),
            last_seen: "2026-10-30T00:00:00Z".to_owned(),
            reachable: true,
            renewing_from: None,
            acknowledged: None,
            kept_by: None,
            direct: None,
            version: None,
            install: None,
        }
    }

    fn task(label: &str) -> Work {
        Work::Task {
            number: label.parse().unwrap(),
            label: label.to_owned(),
        }
    }

    fn device(name: &str) -> Device {
        Device {
            id: format!("{name}-id"),
            name: name.to_owned(),
        }
    }

    fn devices(found: &[store::Member]) -> Vec<&str> {
        found.iter().map(|member| member.device.as_str()).collect()
    }

    /// The drafting device is permitted without a tick, and is tried before the
    /// ticks.
    #[test]
    fn the_drafting_device_counts_as_permitted_and_comes_first() {
        let found = candidates(
            HERE,
            Some("born"),
            &["ticked".to_owned()],
            vec![member("ticked", "Windows"), member("born", "Windows 11")],
            Platform::Windows,
        )
        .unwrap()
        .0;

        assert_eq!(devices(&found), ["born", "ticked"]);
    }

    /// A member nobody ticked is never a candidate, whatever it runs — and
    /// where every member of the platform is one, they are named.
    #[test]
    fn an_unticked_device_is_never_a_candidate() {
        let found = candidates(
            HERE,
            Some(HERE),
            &[],
            vec![member("unticked", "Windows"), member("other", "Windows 11")],
            Platform::Windows,
        );

        assert_eq!(
            found.unwrap_err(),
            Unplaced::NonePermitted(vec![
                Device {
                    id: "other".to_owned(),
                    name: "other-machine".to_owned(),
                },
                Device {
                    id: "unticked".to_owned(),
                    name: "unticked-machine".to_owned(),
                },
            ]),
        );
    }

    /// And with no member of the platform at all — whatever else is ticked —
    /// none is linked.
    #[test]
    fn no_member_of_the_platform_is_none_linked() {
        let found = candidates(
            HERE,
            None,
            &["mac".to_owned()],
            vec![member("mac", "macOS 15.1"), member("wsl", "Linux (WSL)")],
            Platform::Windows,
        );

        assert_eq!(found.unwrap_err(), Unplaced::NoneLinked);
        assert_eq!(
            candidates(HERE, None, &[], Vec::new(), Platform::Windows).unwrap_err(),
            Unplaced::NoneLinked,
        );
    }

    /// *Linux (WSL)* is a Linux device: a candidate for `on linux`, and never
    /// for `on windows`, though the Windows machine it runs on is.
    #[test]
    fn a_wsl_member_is_a_linux_candidate_and_never_a_windows_one() {
        let members = || {
            vec![
                member("wsl", "Linux (WSL)"),
                member("windows", "Windows"),
                member("mac", "macOS 15.1"),
            ]
        };
        let ticked = ["wsl".to_owned(), "windows".to_owned(), "mac".to_owned()];

        assert_eq!(
            devices(
                &candidates(HERE, None, &ticked, members(), Platform::Linux)
                    .unwrap()
                    .0
            ),
            ["wsl"],
        );
        assert_eq!(
            devices(
                &candidates(HERE, None, &ticked, members(), Platform::Windows)
                    .unwrap()
                    .0
            ),
            ["windows"],
        );
        assert_eq!(
            devices(
                &candidates(HERE, None, &ticked, members(), Platform::MacOs)
                    .unwrap()
                    .0
            ),
            ["mac"],
        );
    }

    /// And the members of the platform nobody ticked come back beside the
    /// permitted ones, for a Set about a sleeping one to ask about too.
    #[test]
    fn the_unticked_members_come_back_beside_the_candidates() {
        let (found, unticked) = candidates(
            HERE,
            None,
            &["ticked".to_owned()],
            vec![member("ticked", "Windows"), member("unticked", "Windows")],
            Platform::Windows,
        )
        .unwrap();

        assert_eq!(devices(&found), ["ticked"]);
        assert_eq!(
            unticked,
            [Device {
                id: "unticked".to_owned(),
                name: "unticked-machine".to_owned(),
            }],
        );
    }

    /// A device that is asleep, or that answers and lacks something, is passed
    /// over for the next — and the one after the first clean answer is never
    /// asked.
    #[tokio::test]
    async fn a_device_that_cannot_take_the_work_is_passed_over_for_the_next() {
        let candidates = vec![
            member("asleep", "Windows"),
            member("lacking", "Windows"),
            member("clean", "Windows"),
            member("after", "Windows"),
        ];
        let mut asked = Vec::new();

        let chosen = first_clean(&candidates, |member| {
            asked.push(member.device.clone());
            async move {
                (member.device != "clean" && member.device != "after")
                    .then(|| Unclean::Lacks(vec![format!("{} lacks", member.device)]))
            }
        })
        .await;

        assert_eq!(chosen.map(|member| member.device.as_str()), Ok("clean"));
        assert_eq!(asked, ["asleep", "lacking", "clean"]);
    }

    /// And none clean, every one awake, is nowhere to go, with what each one
    /// found.
    #[tokio::test]
    async fn no_clean_device_awake_is_nowhere_to_go() {
        let candidates = vec![member("one", "Windows"), member("other", "Windows")];

        let found = first_clean(&candidates, |member| async move {
            Some(Unclean::Lacks(vec![format!("{} found", member.device)]))
        })
        .await
        .unwrap_err();

        assert_eq!(
            none_clean(found, vec![device("unticked")]),
            Unplaced::NoneCanTake(vec!["one found".to_owned(), "other found".to_owned()]),
        );
    }

    /// **One of them asleep is the human asked** (Set 1547, Q4): the sleeping
    /// ones, whatever the awake ones lack, and the unticked ones beside them.
    #[tokio::test]
    async fn a_sleeping_device_among_none_clean_is_asked_about() {
        let candidates = vec![member("lacking", "Windows"), member("asleep", "Windows")];

        let found = first_clean(&candidates, |member| async move {
            Some(match member.device.as_str() {
                "asleep" => Unclean::Asleep,
                _ => Unclean::Lacks(vec!["lacking found".to_owned()]),
            })
        })
        .await
        .unwrap_err();

        assert_eq!(
            none_clean(found, vec![device("unticked")]),
            Unplaced::Asleep {
                asleep: vec![Device {
                    id: "asleep".to_owned(),
                    name: "asleep-machine".to_owned(),
                }],
                unpermitted: vec![device("unticked")],
            },
        );
    }

    /// Each reason a task runs here is its own sentence, naming the task and
    /// the platform as the human writes it.
    #[test]
    fn each_reason_names_the_task_and_the_platform() {
        assert_eq!(
            Unplaced::NoneLinked.said(&task("02"), Platform::Windows),
            "Task 02 wants Windows, but no Windows device is linked, so it's running here.",
        );

        assert_eq!(
            Unplaced::NonePermitted(vec![device("Ada"), device("Grace"), device("Kit")])
                .said(&task("02"), Platform::MacOs),
            "Task 02 wants macOS, but this conversation may not be transferred to Ada, Grace or \
             Kit, so it's running here. Tick a macOS device under **May be transferred to** to \
             have tasks like it moved there.",
        );

        assert_eq!(
            Unplaced::NoneCanTake(vec![
                "Ada is unreachable.".to_owned(),
                "The repo app is not on Grace.".to_owned(),
            ])
            .said(&task("03"), Platform::Linux),
            "Task 03 wants Linux, but no Linux device it may be transferred to can take it, so \
             it's running here.\n\n- Ada is unreachable.\n- The repo app is not on Grace.",
        );
    }

    /// The inline run is named as itself, and is given no advice about ticking
    /// for a later run it will never have.
    #[test]
    fn the_inline_runs_notice_names_the_inline_run() {
        assert_eq!(
            Unplaced::NoneLinked.said(&Work::Inline, Platform::Windows),
            "The inline run wants Windows, but no Windows device is linked, so it's running here.",
        );

        assert_eq!(
            Unplaced::NonePermitted(vec![device("Ada")]).said(&Work::Inline, Platform::Windows),
            "The inline run wants Windows, but this conversation may not be transferred to Ada, \
             so it's running here.",
        );
    }

    /// Only a proposal wanting exactly one platform is placed.
    #[test]
    fn only_a_proposal_wanting_one_platform_is_placed() {
        use verkstead_schema::WantedPlatform;

        assert_eq!(the_one(&[WantedPlatform::Windows]), Some(Platform::Windows));
        assert_eq!(the_one(&[WantedPlatform::Macos]), Some(Platform::MacOs));
        assert_eq!(the_one(&[WantedPlatform::Linux]), Some(Platform::Linux));

        assert_eq!(the_one(&[]), None);
        assert_eq!(
            the_one(&[WantedPlatform::Linux, WantedPlatform::Windows]),
            None
        );
    }

    /// A device already of the platform is never moved off it — a WSL being of
    /// Linux — and one of another platform, or of none this build knows, is.
    #[test]
    fn a_device_already_of_the_platform_is_never_moved_off() {
        assert!(!wants_a_move("Windows", Platform::Windows));
        assert!(!wants_a_move("Linux (WSL)", Platform::Linux));
        assert!(!wants_a_move("macOS 15.1", Platform::MacOs));

        assert!(wants_a_move("Linux (WSL)", Platform::Windows));
        assert!(wants_a_move("Linux", Platform::Windows));
        assert!(wants_a_move("FreeBSD", Platform::Linux));
    }

    /// **Verkstead's own Set names the work, the platform and the device**, in
    /// ADR-0029's words: a *Move to* for each device in the order they would be
    /// tried, then *Run it here*, under a title naming the branch and the
    /// device for the phone it reaches.
    #[test]
    fn the_set_names_the_work_the_platform_and_each_device() {
        let set = asked(
            &task("03"),
            Platform::Windows,
            &[],
            &[device("nuc-win")],
            "rate-limiting",
            "app",
        );

        assert_eq!(set.title, "Move rate-limiting to nuc-win?");
        assert_eq!(set.branch.as_deref(), Some("rate-limiting"));
        assert_eq!(set.project.as_deref(), Some("app"));
        assert_eq!(
            set.questions[0].text,
            "Task 03 wants Windows. nuc-win is linked but not permitted.",
        );
        assert_eq!(
            set.questions[0]
                .options
                .iter()
                .map(|option| (option.n, option.text.as_str()))
                .collect::<Vec<_>>(),
            [(1, "Move to nuc-win"), (2, "Run it here")],
        );
        set.validate().expect("the Set keeps the question grammar");

        let set = asked(
            &Work::Inline,
            Platform::Windows,
            &[],
            &[device("nuc-win"), device("nuc-2")],
            "rate-limiting",
            "app",
        );

        assert_eq!(set.title, "Move rate-limiting to nuc-win or nuc-2?");
        assert_eq!(
            set.questions[0].text,
            "The inline run wants Windows. nuc-win and nuc-2 are linked but not permitted.",
        );
        assert_eq!(
            set.questions[0]
                .options
                .iter()
                .map(|option| option.text.as_str())
                .collect::<Vec<_>>(),
            ["Move to nuc-win", "Move to nuc-2", "Run it here"],
        );
        set.validate().expect("the Set keeps the question grammar");
    }

    /// **A sleeping device is waited on, not offered** (ADR-0029, the asleep
    /// row): the Set says the work moves there by itself once it is awake, and
    /// *Run it here* is its one Option.
    #[test]
    fn the_asleep_set_offers_run_it_here_alone() {
        let set = asked(
            &task("03"),
            Platform::Windows,
            &[device("nuc-win")],
            &[],
            "rate-limiting",
            "app",
        );

        assert_eq!(set.title, "Run rate-limiting here, or wait for nuc-win?");
        assert_eq!(
            set.questions[0].text,
            "Task 03 wants Windows. nuc-win is not awake, and the work moves there by itself \
             once it is.",
        );
        assert_eq!(
            set.questions[0]
                .options
                .iter()
                .map(|option| (option.n, option.text.as_str()))
                .collect::<Vec<_>>(),
            [(1, "Run it here")],
        );
        set.validate().expect("the Set keeps the question grammar");
    }

    /// **Asleep and unticked together are one Set saying both** (Set 1547, Q3):
    /// the sleeping one moving by itself, the other not permitted, with *Move
    /// to* the unticked one against *Run it here*.
    #[test]
    fn asleep_and_unticked_together_are_one_set_saying_both() {
        let set = asked(
            &task("03"),
            Platform::Windows,
            &[device("nuc-win")],
            &[device("nuc-2")],
            "rate-limiting",
            "app",
        );

        assert_eq!(set.title, "Move rate-limiting to nuc-2?");
        assert_eq!(
            set.questions[0].text,
            "Task 03 wants Windows. nuc-win is not awake, and the work moves there by itself \
             once it is. nuc-2 is linked but not permitted.",
        );
        assert_eq!(
            set.questions[0]
                .options
                .iter()
                .map(|option| option.text.as_str())
                .collect::<Vec<_>>(),
            ["Move to nuc-2", "Run it here"],
        );
        set.validate().expect("the Set keeps the question grammar");
    }

    /// **The agent's call says the ADR's agent rows** (ADR-0029): the device it
    /// asked for and why the work cannot move there yet, with *Keep it here* in
    /// place of *Run it here* — beside *Move to* where the device is not
    /// permitted, and alone where it is asleep.
    #[test]
    fn the_agents_call_asks_with_keep_it_here() {
        let unticked = asked(
            &Work::Called,
            Platform::Windows,
            &[],
            &[device("nuc-win")],
            "rate-limiting",
            "app",
        );

        assert_eq!(unticked.title, "Move rate-limiting to nuc-win?");
        assert_eq!(
            unticked.questions[0].text,
            "The agent asked to move to nuc-win, which is not permitted.",
        );
        assert_eq!(
            unticked.questions[0]
                .options
                .iter()
                .map(|option| option.text.as_str())
                .collect::<Vec<_>>(),
            ["Move to nuc-win", "Keep it here"],
        );
        unticked
            .validate()
            .expect("the Set keeps the question grammar");

        let asleep = asked(
            &Work::Called,
            Platform::Windows,
            &[device("nuc-win")],
            &[],
            "rate-limiting",
            "app",
        );

        assert_eq!(
            asleep.title,
            "Keep rate-limiting here, or wait for nuc-win?"
        );
        assert_eq!(
            asleep.questions[0].text,
            "The agent asked to move to nuc-win, which is not awake, and the work moves there \
             by itself once it is.",
        );
        assert_eq!(
            asleep.questions[0]
                .options
                .iter()
                .map(|option| option.text.as_str())
                .collect::<Vec<_>>(),
            ["Keep it here"],
        );
        asleep
            .validate()
            .expect("the Set keeps the question grammar");
    }

    /// Three or more names take the serial comma, as every list on a screen
    /// does.
    #[test]
    fn a_list_of_every_device_takes_the_serial_comma() {
        let names = |names: &[&str]| {
            names
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        };

        assert_eq!(both(&names(&["Ada"])), "Ada");
        assert_eq!(both(&names(&["Ada", "Kit"])), "Ada and Kit");
        assert_eq!(
            both(&names(&["Ada", "Grace", "Kit"])),
            "Ada, Grace, and Kit"
        );
    }
}
