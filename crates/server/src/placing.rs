//! Moving a task of the backlog to the platform its line declares, before its
//! session starts (ADR-0029, *Placing declared work*).
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
//! **Where no device will do, the task runs here**, said in the log and nowhere
//! else for now.

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

    let Some(devices) = state.devices.as_ref() else {
        tracing::info!(
            conversation_id,
            task = label,
            wants = ?wants,
            "the task wants a platform and this device is in no cluster, so it runs here",
        );
        return Placed::Here;
    };

    if !wants_a_move(&devices.os(), wants) {
        return Placed::Here;
    }

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

        Ok((.., None)) => return Placed::Here,

        Err(error) => {
            tracing::error!(error = ?error, conversation_id, task = label, "reading where a task that wants a platform could go failed, so it runs here");
            return Placed::Here;
        }
    };

    let candidates = candidates(
        devices.id(),
        born.as_ref().map(|born| born.device.as_str()),
        &ticked,
        members,
        wants,
    );

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
                Ok(reading) if reading.lacks.is_empty() => true,

                Ok(reading) => {
                    tracing::info!(
                        conversation_id,
                        device = member.device,
                        lacks = ?reading.lacks,
                        "a device a task could go to cannot take it, so the next is asked",
                    );
                    false
                }

                Err(refusal) => {
                    tracing::error!(
                        conversation_id,
                        device = member.device,
                        "the preflight of a device a task could go to failed: {}",
                        refusal.saying,
                    );
                    false
                }
            }
        }
    })
    .await;

    let Some(member) = chosen else {
        tracing::info!(
            conversation_id,
            task = label,
            wants = ?wants,
            candidates = candidates.len(),
            "no permitted device of the platform a task wants can take it, so it runs here",
        );
        return Placed::Here;
    };

    let asked = crate::transfers::asked(
        state,
        conversation_id,
        &member.device,
        store::AskedBy::Task {
            label: label.clone(),
            wants: platform::os_word(wants, None),
        },
    )
    .await;

    match asked {
        Ok(()) => {
            tracing::info!(
                conversation_id,
                task = label,
                device = member.device,
                "a task wants another platform, so the Conversation is being moved before it is \
                 launched",
            );
            Placed::Moving
        }

        Err(error) => {
            tracing::error!(error = ?error, conversation_id, task = label, "asking for a task's move failed, so it runs here");
            Placed::Here
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
fn candidates(
    here: &str,
    born: Option<&str>,
    ticked: &[String],
    members: Vec<store::Member>,
    wants: Platform,
) -> Vec<store::Member> {
    crate::transfers::placed_among(here, born, ticked, members)
        .into_iter()
        .filter(|placed| placed.permitted && platform::os_mark(&placed.member.os) == Some(wants))
        .map(|placed| placed.member)
        .collect()
}

/// The first of `candidates` that `clean` says can take the work, asked in turn
/// rather than all at once: the first clean answer is the whole of what is
/// wanted, and a device later in the list is no reason to wake one earlier.
async fn first_clean<'a, F, Answering>(
    candidates: &'a [store::Member],
    mut clean: F,
) -> Option<&'a store::Member>
where
    F: FnMut(&'a store::Member) -> Answering,
    Answering: Future<Output = bool>,
{
    for member in candidates {
        if clean(member).await {
            return Some(member);
        }
    }

    None
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
        );

        assert_eq!(devices(&found), ["born", "ticked"]);
    }

    /// A member nobody ticked is never a candidate, whatever it runs.
    #[test]
    fn an_unticked_device_is_never_a_candidate() {
        let found = candidates(
            HERE,
            Some(HERE),
            &[],
            vec![member("unticked", "Windows")],
            Platform::Windows,
        );

        assert!(found.is_empty());
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
            devices(&candidates(HERE, None, &ticked, members(), Platform::Linux)),
            ["wsl"],
        );
        assert_eq!(
            devices(&candidates(
                HERE,
                None,
                &ticked,
                members(),
                Platform::Windows
            )),
            ["windows"],
        );
        assert_eq!(
            devices(&candidates(HERE, None, &ticked, members(), Platform::MacOs)),
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
            async move { member.device == "clean" || member.device == "after" }
        })
        .await;

        assert_eq!(chosen.map(|member| member.device.as_str()), Some("clean"));
        assert_eq!(asked, ["asleep", "lacking", "clean"]);
    }

    /// And none clean is nowhere to go.
    #[tokio::test]
    async fn no_clean_device_is_nowhere_to_go() {
        let candidates = vec![member("asleep", "Windows")];

        assert!(
            first_clean(&candidates, |_| async { false })
                .await
                .is_none()
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
