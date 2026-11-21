//! **Follow-up rounds**, the first built-in Activity (ADR-0030, *Activity*): a
//! session on the following-up skill, relaunched by nobody, asking the human in
//! rounds until they mark one *Nothing else*, and a reading of the branches at
//! the end.
//!
//! **It reaches the server through the host functions and nothing else.** What
//! the session is started on is written here, off the start facts — the Brief,
//! the handoff, and the newest way into Follow-up on the Timeline — in the words
//! the server's own builder has always used. The session is launched expecting
//! the Nothing-else mark as its Done signal, so a follow-up never ends itself.
//!
//! **It ends in an Outcome, and never names a state.** `committed` where the
//! Conversation's branch or any read-write Companion's holds commits past its
//! base, `nothing` where none does, and `steered` where the human took the
//! Conversation elsewhere. Where the Conversation goes on each is the host's to
//! say: back to the wrap-up over a pull request, a pull request sent for, or
//! Done.
//!
//! **Started again rather than remembered.** Nothing is kept for it between
//! starts, so a follow-up picked up again — a Resume, or a server restarted
//! mid-rounds — is this Activity started over the Timeline as it now stands.
//! The host hands it two readings of that Timeline it has no other way to make:
//! `settled`, the rounds already answered inside this follow-up as one digest,
//! which the fresh session is primed with; and `marked`, where the newest of
//! them carries the Nothing-else mark and the work is on no pull request. A
//! marked round with work standing is then a follow-up whose ending got as far
//! as owing a pull request, so it ends `committed` at once rather than asking
//! the human for another round.
//!
//! **Configured rather than copied.** The skill and the evidence are options,
//! defaulting to the follow-up's, so investigating rounds are this Activity
//! started with `skill: verkstead/investigating` and `evidence: investigated`.
//! Its reading of the branches is the same: an investigation that committed
//! nothing ends `nothing`, and one that did ends `committed`, which is the word
//! its Process holds it to. What an investigation is asked is the host's to
//! write — the question it was opened with is read off a steer into
//! Investigating or the Brief of an Investigate, which a follow-up's reading of
//! the Timeline knows nothing of — so it is handed in whole as the `prompt`
//! option, and a session that goes is said to have left the question
//! unfinished rather than the follow-up.

wit_bindgen::generate!({
    world: "activity",
    path: "../../../crates/server/wit",
});

use verkstead::plugin::activity_types::{Ends, Entry, Evidence, Waited};
use verkstead::plugin::host;

/// What a gone session's stop says beyond how it went, where the human never
/// said there was nothing else: the server's own sentence for it.
const NOBODY: &str = "Nobody is left to ask you anything or to act on what you say. Any \
     question the session put to you has been closed unanswered.";

/// How a session that finished went, where the host had nothing worse to say.
const FINISHED: &str = "The follow-up session finished.";

/// The same two for an investigation: what is left unfinished is the question
/// rather than the follow-up, and there is nothing on the branch either way.
const NOBODY_INVESTIGATING: &str = "Nobody is left to find anything out or to ask you about \
     it. Any question the session put to you has been closed unanswered.";

const FINISHED_INVESTIGATING: &str = "The investigating session finished.";

struct Rounds;

impl Guest for Rounds {
    fn run(facts: Facts) -> String {
        let skill = option(&facts, "skill").unwrap_or("verkstead/following-up");
        let (evidence, nobody, finished) = match option(&facts, "evidence") {
            Some("investigated") => (
                Evidence::Investigated,
                NOBODY_INVESTIGATING,
                FINISHED_INVESTIGATING,
            ),
            _ => (Evidence::NothingElse, NOBODY, FINISHED),
        };

        let Some(path) = facts.skills.iter().find(|named| named.name == skill) else {
            host::stop(&format!(
                "There's no `{skill}` skill to start the follow-up session in."
            ));
            return String::new();
        };

        // The rounds are over and the work is standing: the ending was made
        // already, and what it sent for is what is still owed. Read as the
        // ending reads, so the two cannot disagree about which this is.
        if option(&facts, "marked") == Some("true") && ending(&facts) == "committed" {
            return "committed".to_owned();
        }

        let prompt = match option(&facts, "prompt") {
            Some(prompt) => prompt.to_owned(),
            None => {
                let Some(follow_up) = opened(&facts.timeline) else {
                    host::stop("There's nothing on the timeline to follow up on.");
                    return String::new();
                };

                prompt(&path.path, &facts, &follow_up)
            }
        };

        let session = match host::launch(
            skill,
            "implementation",
            &prompt,
            &evidence,
            Ends::WithItsWork,
        ) {
            Ok(session) => session,
            Err(why) => {
                host::stop(&why);
                return String::new();
            }
        };

        match host::wait(session) {
            Waited::Done | Waited::Marked | Waited::Picked(_) => ending(&facts),
            Waited::Steered => "steered".to_owned(),
            // How it ended, where the ending itself was the trouble; otherwise
            // the ending is the whole of it, a session that has finished with
            // the follow-up still running being exactly as gone as one that
            // fell over.
            Waited::Gone(how) => {
                host::stop(&format!("{} {nobody}", how.as_deref().unwrap_or(finished)));
                String::new()
            }
            Waited::Stalled => {
                host::stop(&format!("{finished} {nobody}"));
                String::new()
            }
        }
    }
}

export!(Rounds);

/// What the human wants followed up on, and whether it is the Conversation's
/// own Brief.
struct FollowUp<'a> {
    brief: &'a str,
    from_the_brief: bool,
}

/// The option `key` the Activity was started with, where it was.
fn option<'a>(facts: &'a Facts, key: &str) -> Option<&'a str> {
    facts
        .options
        .iter()
        .find(|(named, _)| named == key)
        .map(|(_, value)| value.as_str())
}

/// The newest way into Follow-up: a steer with a brief under it, or — where
/// there is none — a **Tinker** start, whose follow-up is the Brief itself and
/// which wrote a move into Follow-up rather than a steer.
fn opened(timeline: &[Entry]) -> Option<FollowUp<'_>> {
    let steered = timeline
        .iter()
        .rev()
        .filter(|entry| entry.kind == "steer")
        .find_map(|entry| match entry.body.split_once('\n') {
            Some(("follow-up", brief)) => Some(brief),
            _ => None,
        });

    if let Some(brief) = steered {
        return Some(FollowUp {
            brief,
            from_the_brief: false,
        });
    }

    timeline
        .iter()
        .any(|entry| entry.kind == "moved" && entry.body == "follow-up")
        .then_some(())?;

    let brief = timeline.iter().rev().find(|entry| entry.kind == "brief")?;

    Some(FollowUp {
        brief: &brief.body,
        from_the_brief: true,
    })
}

/// The follow-up session's prompt: the documents the work is written down in,
/// and under them what the human wants followed up — or, where the follow-up
/// *is* the Brief, that alone, said once under the heading that says act on it —
/// and last the rounds already answered, where there were any.
fn prompt(skill: &str, facts: &Facts, follow_up: &FollowUp<'_>) -> String {
    let prompt = match follow_up.from_the_brief || facts.brief.trim().is_empty() {
        true => alone(&format!(
            "Read {skill} and follow up on the work on this branch, the way it says."
        )),
        false => {
            let mut prompt = format!(
                "{}\n# The Brief this started from\n\n{}\n",
                alone(&format!(
                    "Read {skill} and follow up on this branch's pull request, the way it says."
                )),
                facts.brief,
            );

            if let Some(handoff) = &facts.handoff {
                prompt.push_str(&format!(
                    "\n# What the grilling settled, in its own words\n\n{handoff}\n"
                ));
            }

            prompt
        }
    };

    let prompt = format!(
        "{prompt}\n# What I want to follow up on\n\n{}\n",
        follow_up.brief.trim()
    );

    // The rounds already answered, where this is a follow-up picked up again:
    // a heading over nothing would tell a fresh session something had been said.
    match option(facts, "settled").map(str::trim) {
        Some(settled) if !settled.is_empty() => {
            format!("{prompt}\n# What you have already asked, and what I said\n\n{settled}\n")
        }
        _ => prompt,
    }
}

fn alone(opening: &str) -> String {
    format!("{opening} Nothing else in this session tells you how to reach me.\n")
}

/// The Outcome the branches read as: `committed` where the Conversation's own
/// branch or any read-write Companion's holds commits past its base, and
/// `nothing` where none does. A branch git will not read holds nothing, as the
/// server has always read one.
fn ending(facts: &Facts) -> String {
    let committed = std::iter::once(&facts.worktree)
        .chain(
            facts
                .companions
                .iter()
                .filter(|companion| companion.writable),
        )
        .any(|checkout| host::git_facts(&checkout.repo).is_ok_and(|branch| branch.commits > 0));

    match committed {
        true => "committed",
        false => "nothing",
    }
    .to_owned()
}
