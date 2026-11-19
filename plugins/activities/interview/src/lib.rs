//! **The interview** (ADR-0030, *Activity*): the grilling session, launched on
//! `verkstead/grilling` under the Grilling role and waited on until it has
//! written what the human picked.
//!
//! **One session, whatever is picked.** Breaking the work down and staging it
//! go on inside the same session, as the grilling skill says, so the interview
//! launches nothing after it. The proposals the session puts are the host's to
//! settle and write down; what the interview waits for is the artefact of the
//! Direction picked on the latest of them — the handoff, the plan commit, or
//! the roadmap commit and its pull request — with the session's Done signal
//! accepted over it, which the host checks as the evidence `picked`.
//!
//! **Its Outcomes are the Directions**: `direction:inline`,
//! `direction:task-list` and `direction:roadmap`, and `steered` where the human
//! took the Conversation elsewhere. A Process maps each to a state. And `gone`
//! where the session went before anything was picked: nothing is driving the
//! Conversation then, and the host says so as it says of any Conversation left
//! standing still.
//!
//! **Handed its prompt whole**, as an option: a fresh grilling's words are the
//! Brief under the line sending it into the skill, and a relaunch's add what
//! the human already settled, both written by the server's builders so the
//! session is told exactly what it always was. A launch the host refuses stops
//! the Conversation in the host's sentence for why.

wit_bindgen::generate!({
    world: "activity",
    path: "../../../crates/server/wit",
});

use verkstead::plugin::activity_types::{Ends, Evidence, Waited};
use verkstead::plugin::host;

struct Interview;

impl Guest for Interview {
    fn run(facts: Facts) -> String {
        let Some(prompt) = facts
            .options
            .iter()
            .find(|(key, _)| key == "prompt")
            .map(|(_, prompt)| prompt.as_str())
        else {
            host::stop("The interview was started without a prompt to start it on.");
            return String::new();
        };

        let session = match host::launch(
            "verkstead/grilling",
            "grilling",
            prompt,
            &Evidence::Picked,
            Ends::WithItsWork,
        ) {
            Ok(session) => session,
            Err(why) => {
                host::stop(&why);
                return String::new();
            }
        };

        match host::wait(session) {
            Waited::Picked(direction) => format!("direction:{direction}"),
            Waited::Steered => "steered".to_owned(),
            // Nothing else ends a wait on a pick: a session gone before one, or
            // gone after it in a stop the host has already written.
            Waited::Gone(_) | Waited::Done | Waited::Marked | Waited::Stalled => "gone".to_owned(),
        }
    }
}

export!(Interview);
