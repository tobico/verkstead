//! **One session**, the plain Activity (ADR-0030, *Activity*): one session
//! launched under a skill, as a role, with a prompt, expecting the evidence of
//! its Done signal, and waited on until it is over.
//!
//! **Configured entirely by options.** The inline build is this Activity under
//! `verkstead/implementing` expecting a commit and ending on a pull request;
//! the `submitting` step is it under `verkstead/submitting` expecting nothing
//! but the pull request; an `instruction` steer is it under
//! `verkstead/instruction` expecting a commit. The prompt is handed in whole,
//! in the words the server's builders have always used, so a session is told
//! exactly what it was told before. The building kind its Memos and previews
//! are permitted by is an option too, which the host reads rather than this.
//!
//! **It ends in an Outcome, and never decides what one means.** `done` where
//! the Done signal was given and taken, `refused` where the session ended
//! without one, and `steered` where the human took the Conversation
//! elsewhere. What a `refused` comes to — a stop, or a pull request asked
//! after first — is its caller's, which reads how the session ended through
//! the host. A launch the host refuses stops the Conversation in the host's
//! sentence for why.

wit_bindgen::generate!({
    world: "activity",
    path: "../../../crates/server/wit",
});

use verkstead::plugin::activity_types::{Ends, Evidence, Landing, Waited};
use verkstead::plugin::host;

struct OneSession;

impl Guest for OneSession {
    fn run(facts: Facts) -> String {
        let (Some(skill), Some(prompt)) = (option(&facts, "skill"), option(&facts, "prompt"))
        else {
            host::stop("One session was started without a skill or a prompt to start it on.");
            return String::new();
        };

        let role = option(&facts, "role").unwrap_or("implementation");

        let Some(evidence) = evidence(option(&facts, "evidence").unwrap_or("nothing")) else {
            host::stop("One session was started expecting evidence it doesn't know.");
            return String::new();
        };

        let ends = match option(&facts, "ends") {
            Some("on-a-pull-request") => Ends::OnAPullRequest,
            _ => Ends::WithItsWork,
        };

        let session = match host::launch(skill, role, prompt, &evidence, ends) {
            Ok(session) => session,
            Err(why) => {
                host::stop(&why);
                return String::new();
            }
        };

        match host::wait(session) {
            Waited::Done | Waited::Marked => "done",
            Waited::Steered => "steered",
            Waited::Gone(_) | Waited::Stalled => "refused",
        }
        .to_owned()
    }
}

export!(OneSession);

/// The option `key` the Activity was started with, where it was.
fn option<'a>(facts: &'a Facts, key: &str) -> Option<&'a str> {
    facts
        .options
        .iter()
        .find(|(named, _)| named == key)
        .map(|(_, value)| value.as_str())
}

/// The evidence an option names: a bare word — `nothing`, `nothing-else`,
/// `investigated` — or a word, a colon and what it is of —
/// `committed-since:3`, `ticked:2`, `handoff:/path`.
fn evidence(option: &str) -> Option<Evidence> {
    let (kind, of) = option.split_once(':').unwrap_or((option, ""));

    Some(match kind {
        "nothing" => Evidence::Nothing,
        "nothing-else" => Evidence::NothingElse,
        "investigated" => Evidence::Investigated,
        "committed-since" => Evidence::CommittedSince(of.parse().ok()?),
        "gone" => Evidence::Landed(Landing::Gone(of.to_owned())),
        "arrived" => Evidence::Landed(Landing::Arrived(of.to_owned())),
        "ticked" => Evidence::Landed(Landing::Ticked(of.parse().ok()?)),
        "roadmap" => Evidence::Landed(Landing::Roadmap(of.to_owned())),
        "handoff" => Evidence::Landed(Landing::Handoff(of.to_owned())),
        _ => return None,
    })
}
