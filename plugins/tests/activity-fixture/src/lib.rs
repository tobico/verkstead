//! An Activity that reaches every host function a wrap-up needs, and answers
//! what each came to as its Outcome, for the server's suites to read.
//!
//! **Waiting on several sessions.** It launches a first session and waits on
//! it for a moment, which runs out: a `tick`. It launches a second, which ends
//! the first, and waits on both: the first answers, having ended. Then it waits
//! on the second until its Done signal is taken.
//!
//! **The Forge and the wrap-up's records**, over pull request #41 of the
//! Worktree's Repo: its standing, its checks — which the suite says the Forge
//! lacks, so the `checks` watcher comes back settled at once — and then a
//! watcher settled, a fix attempt counted and a comment marked addressed, each
//! read back.
//!
//! The Outcome is every answer in order, joined by spaces: `tick
//! first:steered second:done standing:cleanly:open checks:lacking attempts:1
//! addressed:PRRC_1` where all went as the suite means it to.

wit_bindgen::generate!({
    path: "../../../crates/server/wit",
    world: "activity",
});

use verkstead::plugin::activity_types::{
    Checked, Ends, Evidence, Mergeable, PullRequest, PullRequestStands, Unforged, Waited, Watcher, Woke,
};
use verkstead::plugin::host;

/// How long the first wait lasts, which the first session outlives.
const MOMENT: u64 = 100;

/// How long the waits that are to be answered may last: far longer than the
/// suite's patience.
const LONG: u64 = 600_000;

struct Fixture;

impl Guest for Fixture {
    fn run(facts: Facts) -> String {
        let mut saw = Vec::new();

        let Some(first) = launch("The fixture Activity's first session waits.") else {
            return "unlaunched".to_owned();
        };

        saw.push(match host::wait_any(&[first], MOMENT) {
            Woke::Tick => "tick".to_owned(),
            Woke::Waited((_, waited)) => format!("woke:{}", word(&waited)),
        });

        let Some(second) = launch("The fixture Activity's second session signals.") else {
            return "unlaunched".to_owned();
        };

        saw.push(waited(
            "first",
            first,
            second,
            host::wait_any(&[first, second], LONG),
        ));
        saw.push(waited(
            "second",
            first,
            second,
            host::wait_any(&[second], LONG),
        ));

        let pull_request = PullRequest {
            repo: facts.worktree.repo.clone(),
            number: 41,
        };

        saw.push(match host::forge_standing(&pull_request) {
            Ok(standing) => format!(
                "standing:{}:{}",
                match standing.mergeable {
                    Mergeable::Cleanly => "cleanly",
                    Mergeable::Conflicting => "conflicting",
                    Mergeable::Unknown => "unknown",
                },
                match standing.stands {
                    PullRequestStands::Open => "open",
                    PullRequestStands::Merged => "merged",
                    PullRequestStands::Closed => "closed",
                    PullRequestStands::Unknown => "unknown",
                }
            ),
            Err(unforged) => format!("standing:{}", unforged_word(&unforged)),
        });

        saw.push(match host::forge_checks(&pull_request) {
            Ok(suite) => format!(
                "checks:{}",
                suite
                    .checks
                    .iter()
                    .map(|check| match check.how {
                        Checked::Passed => "passed",
                        Checked::Failed => "failed",
                        Checked::Running => "running",
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Err(unforged) => format!("checks:{}", unforged_word(&unforged)),
        });

        host::settle(&Watcher::Mergeable(pull_request.clone()));

        saw.push(match host::count_fix_attempt(&pull_request, "Rust") {
            Ok(_) => match host::fix_attempts(&pull_request, "Rust") {
                Ok(attempts) => format!("attempts:{attempts}"),
                Err(_) => "attempts:unread".to_owned(),
            },
            Err(_) => "attempts:uncounted".to_owned(),
        });

        saw.push(
            match host::mark_addressed(&pull_request.repo, &["PRRC_1".to_owned()]) {
                Ok(()) => match host::addressed(&pull_request.repo) {
                    Ok(addressed) => format!("addressed:{}", addressed.join(",")),
                    Err(_) => "addressed:unread".to_owned(),
                },
                Err(_) => "addressed:unmarked".to_owned(),
            },
        );

        saw.join(" ")
    }
}

/// A session under `verkstead/implementing` on `prompt`, expecting nothing.
fn launch(prompt: &str) -> Option<u64> {
    host::launch(
        "verkstead/implementing",
        "implementation",
        prompt,
        &Evidence::Nothing,
        Ends::WithItsWork,
    )
    .ok()
}

/// What `woke` came to, said as `<which>:<word>`, which session it answered
/// for being named by which of the two it was — `expected` where it is the one
/// the caller expected.
fn waited(expected: &str, first: u64, second: u64, woke: Woke) -> String {
    match woke {
        Woke::Tick => format!("{expected}:tick"),
        Woke::Waited((session, waited)) => {
            let which = match session {
                session if session == first => "first",
                session if session == second => "second",
                _ => "neither",
            };

            format!("{which}:{}", word(&waited))
        }
    }
}

fn word(waited: &Waited) -> &'static str {
    match waited {
        Waited::Done => "done",
        Waited::Gone(_) => "gone",
        Waited::Stalled => "stalled",
        Waited::Steered => "steered",
        Waited::Marked => "marked",
        Waited::Picked(_) => "picked",
    }
}

fn unforged_word(unforged: &Unforged) -> String {
    match unforged {
        Unforged::Lacking => "lacking".to_owned(),
        Unforged::Unasked(why) => format!("unasked({why})"),
    }
}

export!(Fixture);
