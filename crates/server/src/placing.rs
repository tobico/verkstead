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
//! and the runner stands down. Nothing waits on it here; the answer is acted on
//! as it is taken — see [`answered`] — and a launch after *Run it here* runs
//! the work here without asking again.
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
    /// Verkstead's own, and their answer launches it — see [`answered`].
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
}

impl Work {
    /// The work as the start of a sentence names it.
    fn named(&self) -> String {
        match self {
            Work::Task { label, .. } => format!("Task {label}"),
            Work::Inline => "The inline run".to_owned(),
        }
    }

    /// Who the move is asked by: Verkstead, for this work.
    fn asking(&self, wants: Platform) -> store::AskedBy {
        let wants = platform::os_word(wants, None);

        match self {
            Work::Task { label, .. } => store::AskedBy::Task {
                label: label.clone(),
                wants,
            },
            Work::Inline => store::AskedBy::Inline { wants },
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
    /// order they would be tried. The one case the human is asked about rather
    /// than told — see [`ask`].
    NonePermitted(Vec<Device>),

    /// Permitted members of it, none of which can take the work: what each
    /// one's preflight found, in the Transfer dialog's words, which name it.
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
    // is where it runs — the answer's own Resume reaching this launch included.
    if run_here(state, conversation_id, &work).await {
        return Placed::Here;
    }

    match placing(state, conversation_id, &work, wants).await {
        Ok(placed) => placed,

        Err(Unplaced::NonePermitted(devices)) => {
            match ask(state, conversation_id, &work, wants, &devices).await {
                Ok(()) => {
                    tracing::info!(
                        conversation_id,
                        work = ?work,
                        wants = ?wants,
                        "work wants a platform whose devices are linked and not permitted, so the \
                         human is asked where it runs",
                    );
                    Placed::Asked
                }

                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, work = ?work, "asking where work runs failed, so it runs here");
                    unplaced(
                        state,
                        conversation_id,
                        &work,
                        wants,
                        Unplaced::NonePermitted(devices),
                    )
                    .await
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

/// Put the Question Set of Verkstead's own that asks where `work` runs, its
/// platform's `devices` being linked and none of them permitted (ADR-0029,
/// *Verkstead asks with a Question Set of its own*): one *Move to* for each, in
/// the order they would be tried, and *Run it here*.
///
/// On the Timeline, on the badge and on a phone as any Set is. Nothing waits
/// on it here: the answer is what launches the work — see [`answered`].
async fn ask(
    state: &AppState,
    conversation_id: i64,
    work: &Work,
    wants: Platform,
    devices: &[Device],
) -> anyhow::Result<()> {
    let conversation = store::load_conversation(&state.pool, conversation_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Conversation {conversation_id} is not there"))?;

    let set = asked(
        work,
        wants,
        devices,
        &conversation.branch,
        &conversation.repo.name,
    );

    let about = store::OwnAsking::Placing {
        work: work.placed(),
        wants: platform::os_word(wants, None),
        moves: devices.iter().map(|device| device.id.clone()).collect(),
    };

    let created = store::ask_on_our_own(&state.pool, conversation_id, &set, &about)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Conversation {conversation_id} is not there"))?;

    crate::push::announce(&state.pool, created.id, &set);
    state.nudges.announce(verkstead_schema::Nudge::Set {
        conversation: conversation_id,
    });

    Ok(())
}

/// The Set that asks where `work` runs, in ADR-0029's words: what wants the
/// platform and why its devices cannot be used yet, a verb on each button, and
/// a title — which is what a phone is told — naming the branch and the device.
fn asked(
    work: &Work,
    wants: Platform,
    devices: &[Device],
    branch: &str,
    project: &str,
) -> verkstead_schema::QuestionSet {
    use verkstead_schema::{Question, QuestionOption, QuestionSet};

    let wants = platform::os_word(wants, None);
    let names = names(devices);

    let linked = match devices {
        [_] => "is linked but not permitted",
        _ => "are linked but not permitted",
    };

    let options = devices
        .iter()
        .map(|device| format!("Move to {}", device.name))
        .chain(["Run it here".to_owned()])
        .zip(1..)
        .map(|(text, n)| QuestionOption {
            n,
            text,
            recommended: false,
            cells: Vec::new(),
        })
        .collect();

    QuestionSet {
        title: format!("Move {branch} to {}?", either(&names)),
        preface: None,
        questions: vec![Question {
            label: QUESTION.to_owned(),
            text: format!("{} wants {wants}. {} {linked}.", work.named(), both(&names)),
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

/// Whether the human, asked where `work` runs, said to run it here: the newest
/// Set of Verkstead's own about it answered with *Run it here*.
async fn run_here(state: &AppState, conversation_id: i64, work: &Work) -> bool {
    let own = match store::own_sets(&state.pool, conversation_id).await {
        Ok(own) => own,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Sets Verkstead asked failed");
            return false;
        }
    };

    let newest = own.into_iter().rev().find_map(|own| match &own.asking {
        store::OwnAsking::Placing {
            work: placed,
            moves,
            ..
        } if Work::from_placed(placed) == *work => Some((own.response, moves.len())),
        store::OwnAsking::Placing { .. } => None,
    });

    matches!(
        newest,
        Some((Some(response), moves)) if picked(&response) == Some(moves as u32 + 1)
    )
}

/// The Option picked on a Set of Verkstead's, where one was.
fn picked(response: &verkstead_schema::Response) -> Option<u32> {
    response
        .answers
        .iter()
        .find(|answer| answer.label.trim() == QUESTION)
        .and_then(|answer| answer.selected)
}

/// Act on the answer to a Set of Verkstead's own asking where work runs, in
/// `conversation_id`: *Move to* ticks that device under **May be transferred
/// to** and moves the work there by the ordinary **Transfer**, on Verkstead's
/// own account, the far end's arrival launching it; *Run it here* launches it
/// here, by the Resume a press would make.
///
/// An answer picking neither is the human not deciding, and nothing is guessed
/// at: the work is left waiting, a Notice saying how it stands.
pub(crate) async fn answered(
    state: &AppState,
    conversation_id: i64,
    asking: &store::OwnAsking,
    response: &verkstead_schema::Response,
) {
    let store::OwnAsking::Placing { work, wants, moves } = asking;
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

            let by = match &work {
                Work::Task { label, .. } => store::AskedBy::Task {
                    label: label.clone(),
                    wants: wants.clone(),
                },
                Work::Inline => store::AskedBy::Inline {
                    wants: wants.clone(),
                },
            };

            if let Err(error) = crate::transfers::asked(state, conversation_id, device, by).await {
                tracing::error!(error = ?error, conversation_id, device, "asking for the move the human picked failed");
            }
        }

        Some(n) if n == moves.len() + 1 => {
            match crate::resume::resume(state, conversation_id, crate::resume::Resuming::Pressed)
                .await
            {
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
            let said = match work {
                Work::Task { .. } => format!(
                    "{} hasn't started. Press **Resume** to be asked again.",
                    work.named()
                ),
                Work::Inline => {
                    "The inline run hasn't started. Press **Resume** to start it here.".to_owned()
                }
            };

            if let Err(error) = store::note(&state.pool, conversation_id, &said).await {
                tracing::error!(error = ?error, conversation_id, "saying the work is still waiting failed");
            }
        }
    }
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

    let candidates = candidates(
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
                Ok(reading) if reading.lacks.is_empty() => None,

                Ok(reading) => Some(
                    reading
                        .lacks
                        .iter()
                        .map(|lacking| crate::transfers::lacks(lacking, &reading.device))
                        .collect(),
                ),

                Err(refusal) => {
                    tracing::error!(
                        conversation_id,
                        device = member.device,
                        "the preflight of a device work could go to failed: {}",
                        refusal.saying,
                    );
                    Some(vec![format!(
                        "{} couldn't be asked what it has: {}",
                        member.name, refusal.saying,
                    )])
                }
            }
        }
    })
    .await?;

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
/// `Err` where there are none, saying which: no member of that platform at
/// all, or members of it that nobody permitted.
fn candidates(
    here: &str,
    born: Option<&str>,
    ticked: &[String],
    members: Vec<store::Member>,
    wants: Platform,
) -> Result<Vec<store::Member>, Unplaced> {
    let (permitted, unpermitted): (Vec<_>, Vec<_>) =
        crate::transfers::placed_among(here, born, ticked, members)
            .into_iter()
            .filter(|placed| platform::os_mark(&placed.member.os) == Some(wants))
            .partition(|placed| placed.permitted);

    match (permitted.is_empty(), unpermitted.is_empty()) {
        (false, _) => Ok(permitted.into_iter().map(|placed| placed.member).collect()),
        (true, true) => Err(Unplaced::NoneLinked),
        (true, false) => Err(Unplaced::NonePermitted(
            unpermitted
                .into_iter()
                .map(|placed| Device {
                    id: placed.member.device,
                    name: placed.member.name,
                })
                .collect(),
        )),
    }
}

/// The first of `candidates` that `found` finds nothing in the way of, asked in
/// turn rather than all at once: the first clean answer is the whole of what is
/// wanted, and a device later in the list is no reason to wake one earlier.
///
/// `Err` where every one found something, saying what.
async fn first_clean<'a, F, Answering>(
    candidates: &'a [store::Member],
    mut found: F,
) -> Result<&'a store::Member, Unplaced>
where
    F: FnMut(&'a store::Member) -> Answering,
    Answering: Future<Output = Option<Vec<String>>>,
{
    let mut findings = Vec::new();

    for member in candidates {
        match found(member).await {
            None => return Ok(member),
            Some(said) => findings.extend(said),
        }
    }

    Err(Unplaced::NoneCanTake(findings))
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
        .unwrap();

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
            devices(&candidates(HERE, None, &ticked, members(), Platform::Linux).unwrap()),
            ["wsl"],
        );
        assert_eq!(
            devices(&candidates(HERE, None, &ticked, members(), Platform::Windows).unwrap()),
            ["windows"],
        );
        assert_eq!(
            devices(&candidates(HERE, None, &ticked, members(), Platform::MacOs).unwrap()),
            ["mac"],
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
                    .then(|| vec![format!("{} lacks", member.device)])
            }
        })
        .await;

        assert_eq!(chosen.map(|member| member.device.as_str()), Ok("clean"));
        assert_eq!(asked, ["asleep", "lacking", "clean"]);
    }

    /// And none clean is nowhere to go, with what each one found.
    #[tokio::test]
    async fn no_clean_device_is_nowhere_to_go() {
        let candidates = vec![member("asleep", "Windows"), member("lacking", "Windows")];

        assert_eq!(
            first_clean(&candidates, |member| async move {
                Some(vec![format!("{} found", member.device)])
            })
            .await
            .unwrap_err(),
            Unplaced::NoneCanTake(vec!["asleep found".to_owned(), "lacking found".to_owned()]),
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
