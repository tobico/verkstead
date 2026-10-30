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
//! **Where no device will do, the task runs here**, and a Notice says why — see
//! [`Unplaced`], one case for each reason, each its own sentence, because the
//! platform is a preference rather than a condition of the work. Written at
//! every launch that meets it, a Resume's included, since each launch asked
//! again.
//!
//! **A roadmap stage's tasks are held** (Set 1543, Q4): see [`held`], whose
//! one check is what lifting the hold deletes.
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
}

/// What is being placed, which the Notice and both Timelines name.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Work {
    /// A task of the backlog, by its label as the list writes it.
    Task(String),

    /// The inline run, its implementation session about to start.
    Inline,
}

impl Work {
    /// The work as the start of a sentence names it.
    fn named(&self) -> String {
        match self {
            Work::Task(label) => format!("Task {label}"),
            Work::Inline => "The inline run".to_owned(),
        }
    }

    /// Who the move is asked by: Verkstead, for this work.
    fn asking(&self, wants: Platform) -> store::AskedBy {
        let wants = platform::os_word(wants, None);

        match self {
            Work::Task(label) => store::AskedBy::Task {
                label: label.clone(),
                wants,
            },
            Work::Inline => store::AskedBy::Inline { wants },
        }
    }
}

/// Why work that wants a platform runs here all the same, each the reason one
/// Notice gives — and each its own case, because stage 04 asks the human about
/// some of them rather than telling them.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Unplaced {
    /// No member of that platform is linked, this Verkstead being in no cluster
    /// included. Nothing to ask anybody: there is nowhere to go.
    NoneLinked,

    /// Members of it are linked and none is permitted: their names.
    NonePermitted(Vec<String>),

    /// Permitted members of it, none of which can take the work: what each
    /// one's preflight found, in the Transfer dialog's words, which name it.
    NoneCanTake(Vec<String>),

    /// A roadmap stage's Conversation, which is not placed — see [`held`].
    Held { roadmap: String },
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
            Unplaced::NonePermitted(names) => {
                let mut said = format!(
                    "{named} wants {wants}, but this conversation may not be transferred to {}, \
                     so it's running here.",
                    either(names),
                );

                if let Work::Task(_) = work {
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

            Unplaced::Held { roadmap } => format!(
                "{named} wants {wants}, but it's running here: this conversation is a stage \
                 of the {roadmap} roadmap, and a stage moved off this device drops out of its \
                 roadmap's sight."
            ),
        }
    }
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

    place(state, conversation_id, Work::Task(label), wants).await
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

    let placed = match held(state, conversation_id).await {
        Some(roadmap) => Err(Unplaced::Held { roadmap }),
        None => placing(state, conversation_id, &work, wants).await,
    };

    match placed {
        Ok(placed) => placed,

        Err(unplaced) => {
            tracing::info!(
                conversation_id,
                work = ?work,
                wants = ?wants,
                unplaced = ?unplaced,
                "work that wants a platform runs here",
            );

            if let Err(error) =
                store::note(&state.pool, conversation_id, &unplaced.said(&work, wants)).await
            {
                tracing::error!(error = ?error, conversation_id, "saying why work runs here failed");
            }

            Placed::Here
        }
    }
}

/// The roadmap `conversation_id` is a stage of, where it is one: **its tasks are
/// not placed** (Set 1543, Q4).
///
/// Placing a task moves its Conversation, and a stage's Conversation moved off
/// its roadmap's home device goes dark to that roadmap's scheduler until stages
/// can be moved (stage 05). Stage 06 lifts this, and lifting it is deleting this
/// check and the one line that asks it.
///
/// Any row is a stage: the Conversation that *wrote* a roadmap has one too, and
/// it works no backlog to ask this of.
async fn held(state: &AppState, conversation_id: i64) -> Option<String> {
    match store::stage_roadmap(&state.pool, conversation_id).await {
        Ok(of) => of.map(|of| of.roadmap),
        // The same store the placing reads next, which fails the same way and
        // runs the task here.
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a Conversation is a roadmap stage failed");
            None
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
                .map(|placed| placed.member.name)
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
                "other-machine".to_owned(),
                "unticked-machine".to_owned()
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
            Unplaced::NoneLinked.said(&Work::Task("02".to_owned()), Platform::Windows),
            "Task 02 wants Windows, but no Windows device is linked, so it's running here.",
        );

        assert_eq!(
            Unplaced::NonePermitted(vec!["Ada".to_owned(), "Grace".to_owned(), "Kit".to_owned()])
                .said(&Work::Task("02".to_owned()), Platform::MacOs),
            "Task 02 wants macOS, but this conversation may not be transferred to Ada, Grace or \
             Kit, so it's running here. Tick a macOS device under **May be transferred to** to \
             have tasks like it moved there.",
        );

        assert_eq!(
            Unplaced::NoneCanTake(vec![
                "Ada is unreachable.".to_owned(),
                "The repo app is not on Grace.".to_owned(),
            ])
            .said(&Work::Task("03".to_owned()), Platform::Linux),
            "Task 03 wants Linux, but no Linux device it may be transferred to can take it, so \
             it's running here.\n\n- Ada is unreachable.\n- The repo app is not on Grace.",
        );

        assert_eq!(
            Unplaced::Held {
                roadmap: "platform-placement".to_owned()
            }
            .said(&Work::Task("01".to_owned()), Platform::Windows),
            "Task 01 wants Windows, but it's running here: this conversation is a stage of the \
             platform-placement roadmap, and a stage moved off this device drops out of its \
             roadmap's sight.",
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
            Unplaced::NonePermitted(vec!["Ada".to_owned()]).said(&Work::Inline, Platform::Windows),
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
}
