//! **The wrap-up** (ADR-0030, *Activity*): one Activity watching every pull
//! request the work is on until Verkstead has finished with it, its watchers
//! declared as options and run side by side inside it (Set 1767, Q32).
//!
//! **Its watchers are options**: `checks`, `comments`, `review` and
//! `mergeable`, each `on` unless the Process turns it `off` — which settles it
//! on every round, the way Fix Merge Issues has its review and comments
//! settled from the start. The checks, the comments and the review are still
//! watched by the host beside this until they move in; what this watches
//! itself is each pull request's landing, through the Forge.
//!
//! **It opens the pull request where none stands**: started with a `prompt`
//! on work whose own Repo has no pull request recorded, it launches one
//! session on `verkstead/submitting` itself — one Activity never calls
//! another — and has the host record what that session opened. A session that
//! opened none is the host's to stop over.
//!
//! **Its Outcomes**: `settled` where everything it waits on is settled, which
//! is Done, the merge being the human's act rather than a step of the
//! wrap-up; `merged` where every pull request has been merged and `closed`
//! where one was closed without it, before then; `given-up` where the host
//! gave up on it, holding the stop that says why; and `steered` where the
//! Conversation went elsewhere or its run stopped.
//!
//! **Stateless across starts.** Everything it goes by is the host's record
//! and the Forge's word, asked afresh every round, so a server restarted
//! mid-wrap-up starts it again and it carries on where the record stands.

wit_bindgen::generate!({
    world: "activity",
    path: "../../../crates/server/wit",
});

use verkstead::plugin::activity_types::{
    Ends, Evidence, PullRequest, PullRequestStands, Waited, Watcher, Wrapped,
};
use verkstead::plugin::host;

struct WrapUp;

/// How long a round waits before it looks again, where the host does not say:
/// the thirty seconds the wrap-up has always asked GitHub on.
const POLL: u64 = 30_000;

impl Guest for WrapUp {
    fn run(facts: Facts) -> String {
        let poll = option(&facts, "poll")
            .and_then(|poll| poll.parse().ok())
            .unwrap_or(POLL);

        if let Some(prompt) = option(&facts, "prompt")
            && !host::pull_requests()
                .iter()
                .any(|pull_request| pull_request.repo == facts.worktree.repo)
            && let Some(outcome) = open(prompt)
        {
            return outcome;
        }

        loop {
            match host::wrap_up() {
                Wrapped::Waiting => {}
                Wrapped::Settled => return "settled".to_owned(),
                Wrapped::GivenUp => return "given-up".to_owned(),
                Wrapped::Over => return "steered".to_owned(),
            }

            let pull_requests = host::pull_requests();

            if off(&facts, "review") {
                host::settle(&Watcher::Review);
            }

            for pull_request in &pull_requests {
                if off(&facts, "comments") {
                    host::settle(&Watcher::Comments(pull_request.clone()));
                }

                if off(&facts, "checks") {
                    host::settle(&Watcher::Checks(pull_request.clone()));
                }

                if off(&facts, "mergeable") {
                    host::settle(&Watcher::Mergeable(pull_request.clone()));
                }
            }

            if let Some(outcome) = landed(&pull_requests) {
                return outcome.to_owned();
            }

            host::wait_any(&[], poll);
        }
    }
}

export!(WrapUp);

/// Launch the session that opens the pull request, and have the host record
/// it: `None` to carry on wrapping up over it, or the Outcome to end in.
fn open(prompt: &str) -> Option<String> {
    let session = match host::launch(
        "verkstead/submitting",
        "implementation",
        prompt,
        &Evidence::Nothing,
        Ends::OnAPullRequest,
    ) {
        Ok(session) => session,
        Err(why) => {
            host::stop(&why);
            return Some(String::new());
        }
    };

    if let Waited::Steered = host::wait(session) {
        return Some("steered".to_owned());
    }

    // Refused, the host has stopped the Conversation over what the session
    // last said, or found it wrapping up no longer: either way nothing here is
    // left to watch.
    match host::record_opened() {
        Ok(_) => None,
        Err(_) => Some("steered".to_owned()),
    }
}

/// Whether the pull requests have landed: `merged` where the Forge says every
/// one of them is, `closed` where it says one was closed unmerged. A pull
/// request it would not answer about concludes nothing.
fn landed(pull_requests: &[PullRequest]) -> Option<&'static str> {
    if pull_requests.is_empty() {
        return None;
    }

    let mut merged = 0;

    for pull_request in pull_requests {
        let Ok(standing) = host::forge_standing(pull_request) else {
            continue;
        };

        match standing.stands {
            PullRequestStands::Merged => merged += 1,
            PullRequestStands::Closed => return Some("closed"),
            PullRequestStands::Open | PullRequestStands::Unknown => {}
        }
    }

    (merged == pull_requests.len()).then_some("merged")
}

/// The option `key` the Activity was started with, where it was.
fn option<'a>(facts: &'a Facts, key: &str) -> Option<&'a str> {
    facts
        .options
        .iter()
        .find(|(named, _)| named == key)
        .map(|(_, value)| value.as_str())
}

/// Whether the watcher `key` is turned off.
fn off(facts: &Facts, key: &str) -> bool {
    option(facts, key) == Some("off")
}
