//! **The wrap-up** (ADR-0030, *Activity*): one Activity watching every pull
//! request the work is on until Verkstead has finished with it, its watchers
//! declared as options and run side by side inside it (Set 1767, Q32).
//!
//! **Its watchers are options**: `checks`, `comments`, `review` and
//! `mergeable`, each `on` unless the Process turns it `off` — which settles it
//! on every round, the way Fix Merge Issues has its review and comments
//! settled from the start. The comments and the review are still watched by
//! the host beside this until they move in; what this watches itself is each
//! pull request's checks, whether it merges, and its landing, through the
//! Forge.
//!
//! **The checks**, every round, of every pull request on the record — a
//! Companion's found after this started included. Green settles them, which a
//! suite only is where it is about what origin holds the branch at and where
//! a pull request that has reported checks still reports some: GitHub runs
//! behind a push for a while, and a green rollup of the commit before it would
//! carry the work to Done over work nothing has checked. A red one dispatches
//! a **fix session** inside the addressing skill under the implementation
//! role, told which checks are red, where their runs are and which checkout to
//! work in, with its goes counted per check name in the host's record as it is
//! dispatched — so a restarted server does not count afresh. Once every red
//! check has had its `attempts`, the wrap-up is given up over that pull
//! request, which the host holds off while another pull request still has a go
//! coming. A Forge that lacks checks settles the watcher at once, and that
//! pull request's checks are never asked about again.
//!
//! **Whether it merges** rides the same answer. *Cleanly* settles it,
//! *conflicting* puts it back to waiting and dispatches a **resolution
//! session** instead of any fix — a branch nothing can land is not one worth
//! getting a check green on — told what the host words for it, with the
//! Repo's goes counted the same way; *unknown*, GitHub still working it out,
//! does neither.
//!
//! **One session at a time.** A dispatch tries for the Worktree rather than
//! waiting on it, and holds it for as long as the session runs: what else is
//! in there is the review or a batch of comments, both of which take as long
//! as they take, and a fix queued behind one would be about a suite nobody had
//! looked at since. The next round looks again. The pull request a session is
//! about is not looked at while it runs, and once it is over a round passes
//! before that one is looked at or anything is dispatched again, so that its
//! push has been asked about.
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
//! where one was closed without it, before then; `given-up` where it or the
//! host gave up on it, the host holding the stop that says why; and `steered`
//! where the Conversation went elsewhere or its run stopped.
//!
//! **Stateless across starts.** Everything it goes by is the host's record
//! and the Forge's word, asked afresh every round, so a server restarted
//! mid-wrap-up starts it again and it carries on where the record stands.
//! What it does keep — which pull requests have reported a check, and which
//! the Forge has no checks for — it learns again.

wit_bindgen::generate!({
    world: "activity",
    path: "../../../crates/server/wit",
});

use std::collections::HashSet;

use verkstead::plugin::activity_types::{
    Check, Checked, Ends, Evidence, Mergeable, PullRequest, PullRequestStanding,
    PullRequestStands, Session, Suite, Unforged, Waited, Watcher, Woke, Wrapped,
};
use verkstead::plugin::host;

struct WrapUp;

/// How long a round waits before it looks again, where the host does not say:
/// the thirty seconds the wrap-up has always asked GitHub on.
const POLL: u64 = 30_000;

/// How many fix sessions a check gets, and resolution sessions a conflict,
/// where the host does not say.
const ATTEMPTS: i64 = 2;

/// What the host refuses a launch with where something else holds the
/// Worktree: nothing is launched, and the next round looks again.
const BUSY: &str = "Something else is working in the Worktree.";

/// The skill a fix or a resolution session runs inside.
const ADDRESSING: &str = "verkstead/addressing";

impl Guest for WrapUp {
    fn run(facts: Facts) -> String {
        let poll = option(&facts, "poll")
            .and_then(|poll| poll.parse().ok())
            .unwrap_or(POLL);

        if let Some(prompt) = option(&facts, "prompt")
            && !host::pull_requests()
                .iter()
                .any(|pull_request| pull_request.repo == facts.worktree.repo)
            && let Some(outcome) = open(prompt, poll)
        {
            return outcome;
        }

        let mut watching = Watching::new(&facts);

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

            let standings: Vec<Option<PullRequestStanding>> = pull_requests
                .iter()
                .map(|pull_request| host::forge_standing(pull_request).ok())
                .collect();

            if let Some(outcome) = landed(&standings) {
                return outcome.to_owned();
            }

            watching.round(&pull_requests, &standings);

            let fixing: Vec<Session> = watching.fixing.into_iter().collect();

            if let Woke::Waited((session, _)) = host::wait_any(&fixing, poll)
                && watching.fixing == Some(session)
            {
                watching.fixing = None;
            }
        }
    }
}

export!(WrapUp);

/// The checks and the merge, watched across rounds.
struct Watching<'a> {
    facts: &'a Facts,
    attempts: i64,

    /// The fix or resolution session dispatched and not yet over.
    fixing: Option<Session>,

    /// The pull request it was dispatched about, which is not looked at while
    /// it runs — what the Forge says of it then is about the work before the
    /// fix — nor for the round after, so that what it pushed has had a poll's
    /// time to be asked about.
    held: Option<(String, i64)>,

    /// The pull requests that have reported a check, which is what tells a
    /// repository with no CI from a run not created yet.
    reported: HashSet<(String, i64)>,

    /// The pull requests the Forge has no checks for: settled at once, and
    /// never asked about again.
    lacking: HashSet<(String, i64)>,
}

/// What one look at a pull request came to.
enum Look {
    /// Nothing to dispatch about.
    Nothing,
    /// These checks are red.
    Failed(Vec<Check>),
    /// It will not merge into its base.
    Conflicting,
}

impl<'a> Watching<'a> {
    fn new(facts: &'a Facts) -> Watching<'a> {
        Watching {
            facts,
            attempts: option(facts, "attempts")
                .and_then(|attempts| attempts.parse().ok())
                .unwrap_or(ATTEMPTS),
            fixing: None,
            held: None,
            reported: HashSet::new(),
            lacking: HashSet::new(),
        }
    }

    /// Look at every pull request, settle the wrap-up on what each came to,
    /// and dispatch at most one session about the first that needs one — where
    /// none is running and a round has passed since the last.
    fn round(&mut self, pull_requests: &[PullRequest], standings: &[Option<PullRequestStanding>]) {
        let held = self.held.clone();

        let looks: Vec<(&PullRequest, Look)> = pull_requests
            .iter()
            .zip(standings)
            .filter(|(pull_request, _)| {
                held.as_ref() != Some(&(pull_request.repo.clone(), pull_request.number))
            })
            .map(|(pull_request, standing)| (pull_request, self.look(pull_request, standing)))
            .collect();

        // Nothing dispatched beside a session, nor in the round after one.
        if self.fixing.is_some() || self.held.take().is_some() {
            return;
        }

        for (pull_request, look) in looks {
            let dispatched = match look {
                Look::Nothing => continue,
                Look::Failed(failed) => self.fix(pull_request, &failed),
                Look::Conflicting => self.resolve(pull_request),
            };

            if dispatched {
                return;
            }
        }
    }

    /// Ask how `pull_request`'s checks are, and whether it merges, and settle
    /// the wrap-up on both. A Forge that couldn't be asked concludes nothing.
    fn look(&mut self, pull_request: &PullRequest, standing: &Option<PullRequestStanding>) -> Look {
        let key = (pull_request.repo.clone(), pull_request.number);
        let checks_off = off(self.facts, "checks");

        // Settled again every round rather than once: a review that ends puts
        // every pull request's checks back to waiting, for a push nobody here
        // can be told the checks of.
        if self.lacking.contains(&key) {
            host::settle(&Watcher::Checks(pull_request.clone()));
        }

        let suite = match checks_off || self.lacking.contains(&key) {
            true => None,
            false => match host::forge_checks(pull_request) {
                Ok(suite) => Some(suite),
                // Settled by the host as it answered.
                Err(Unforged::Lacking) => {
                    self.lacking.insert(key.clone());
                    None
                }
                Err(Unforged::Unasked(_)) => return Look::Nothing,
            },
        };

        // Off the same answer as the checks where they were asked, and off the
        // pull request's standing where there are none to ask.
        let mergeable = match &suite {
            Some(suite) => suite.mergeable,
            None => standing
                .as_ref()
                .map_or(Mergeable::Unknown, |standing| standing.mergeable),
        };

        let conflicting = !off(self.facts, "mergeable") && self.merging(pull_request, mergeable);

        let failed = match &suite {
            Some(suite) => self.checking(pull_request, suite),
            None => Vec::new(),
        };

        match (conflicting, failed.is_empty()) {
            (true, _) => Look::Conflicting,
            (false, false) => Look::Failed(failed),
            (false, true) => Look::Nothing,
        }
    }

    /// Settle or unsettle the merge on what the Forge said, and say whether it
    /// conflicts.
    fn merging(&self, pull_request: &PullRequest, mergeable: Mergeable) -> bool {
        let watcher = Watcher::Mergeable(pull_request.clone());

        match mergeable {
            Mergeable::Cleanly => host::settle(&watcher),
            Mergeable::Conflicting => host::unsettle(&watcher),
            Mergeable::Unknown => {}
        }

        mergeable == Mergeable::Conflicting
    }

    /// Settle or unsettle the checks on `suite`, answering the red ones.
    fn checking(&mut self, pull_request: &PullRequest, suite: &Suite) -> Vec<Check> {
        let key = (pull_request.repo.clone(), pull_request.number);
        let watcher = Watcher::Checks(pull_request.clone());

        if !suite.checks.is_empty() {
            self.reported.insert(key.clone());
        }

        let failed: Vec<Check> = suite
            .checks
            .iter()
            .filter(|check| check.how == Checked::Failed)
            .cloned()
            .collect();

        let running = suite
            .checks
            .iter()
            .any(|check| check.how == Checked::Running);

        if !failed.is_empty() || running {
            host::unsettle(&watcher);
            return failed;
        }

        // Green, a pull request with no checks at all included — but not one
        // that has had checks and reports none now, which is a run for the last
        // push not created yet, nor one about a commit origin isn't holding.
        let gone = suite.checks.is_empty() && self.reported.contains(&key);
        let stale = !suite.head.is_empty()
            && host::pushed_head(pull_request).is_some_and(|pushed| pushed != suite.head);

        match gone || stale {
            true => host::unsettle(&watcher),
            false => host::settle(&watcher),
        }

        Vec::new()
    }

    /// Dispatch a fix session for the red checks with goes left, or give the
    /// wrap-up up where none has. Whether anything was dispatched or given up.
    fn fix(&mut self, pull_request: &PullRequest, failed: &[Check]) -> bool {
        let mut fixable = Vec::new();

        for check in failed {
            match host::fix_attempts(pull_request, &check.name) {
                Ok(spent) if spent < self.attempts => fixable.push(check.clone()),
                Ok(_) => {}
                Err(_) => return false,
            }
        }

        let named = named(pull_request);

        if fixable.is_empty() {
            let how = format!(
                "{} on {named} after {} fix sessions each:\n\n{}",
                match failed.len() {
                    1 => "A check is still failing".to_owned(),
                    many => format!("{many} checks are still failing"),
                },
                self.attempts,
                listed(failed),
            );

            return host::give_up(
                pull_request,
                &format!("getting the checks green on {named}"),
                &how,
            );
        }

        let feedback = format!(
            "These checks are failing on {named}. Work in that repository's worktree, at `{}` \
             — both `git` and `gh` read the repository from wherever they are run, so a check \
             asked about anywhere else is a different repository's.\n\nFind out what each of \
             them is actually complaining about, fix the cause, and push the fix so they run \
             again.\n\n{}",
            worktree(self.facts, &pull_request.repo),
            listed(&fixable),
        );

        let Some(session) = self.dispatch(&feedback) else {
            return false;
        };

        // Counted as it is dispatched rather than as it ends, so that a go
        // spent by a server that then restarted is not spent again.
        for check in &fixable {
            let _ = host::count_fix_attempt(pull_request, &check.name);
        }

        self.fixing = Some(session);
        self.held = Some((pull_request.repo.clone(), pull_request.number));
        true
    }

    /// Dispatch a resolution session at a pull request that will not merge, or
    /// give the wrap-up up where its Repo has had its goes.
    fn resolve(&mut self, pull_request: &PullRequest) -> bool {
        let Ok(spent) = host::conflict_attempts(pull_request) else {
            return false;
        };

        let named = named(pull_request);

        if spent >= self.attempts {
            let how = format!(
                "{} still conflicts with its base branch after {} resolution sessions. \
                 Nothing lands until the conflict is resolved by hand.",
                opening(&named),
                self.attempts,
            );

            return host::give_up(
                pull_request,
                &format!("resolving the conflicts on {named}"),
                &how,
            );
        }

        // Refused where it is a stack nothing here can sync, which the host has
        // stopped the run over: the next round finds it over.
        let Ok(told) = host::resolution(pull_request) else {
            return true;
        };

        let Some(session) = self.dispatch(&told) else {
            return false;
        };

        let _ = host::count_conflict_attempt(pull_request);

        self.fixing = Some(session);
        self.held = Some((pull_request.repo.clone(), pull_request.number));
        true
    }

    /// Launch a session inside the addressing skill about `feedback`, or
    /// nothing where something else holds the Worktree — or it was refused,
    /// which the host has said in its log — for the next round to try again.
    fn dispatch(&self, feedback: &str) -> Option<Session> {
        host::launch(
            ADDRESSING,
            "implementation",
            &addressing(self.facts, feedback),
            &Evidence::Nothing,
            Ends::WithItsWork,
        )
        .ok()
    }
}

/// Launch the session that opens the pull request, and have the host record
/// it: `None` to carry on wrapping up over it, or the Outcome to end in.
fn open(prompt: &str, poll: u64) -> Option<String> {
    let session = loop {
        match host::launch(
            "verkstead/submitting",
            "implementation",
            prompt,
            &Evidence::Nothing,
            Ends::OnAPullRequest,
        ) {
            Ok(session) => break session,
            // Whatever is in the Worktree finishes first, as a launch that
            // waited for it would have let it.
            Err(why) if why == BUSY => {
                if let Wrapped::Over = host::wrap_up() {
                    return Some("steered".to_owned());
                }

                host::wait_any(&[], poll);
            }
            Err(why) => {
                host::stop(&why);
                return Some(String::new());
            }
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

/// Whether the pull requests have landed, by their standings: `merged` where
/// the Forge says every one of them is, `closed` where it says one was closed
/// unmerged. A pull request it would not answer about concludes nothing.
fn landed(standings: &[Option<PullRequestStanding>]) -> Option<&'static str> {
    if standings.is_empty() {
        return None;
    }

    let mut merged = 0;

    for standing in standings.iter().flatten() {
        match standing.stands {
            PullRequestStands::Merged => merged += 1,
            PullRequestStands::Closed => return Some("closed"),
            PullRequestStands::Open | PullRequestStands::Unknown => {}
        }
    }

    (merged == standings.len()).then_some("merged")
}

/// What a fix or resolution session is started on: the documents the work is
/// written down in, and under them the feedback to address — in the words the
/// server's own builder has always used.
fn addressing(facts: &Facts, feedback: &str) -> String {
    let skill = facts
        .skills
        .iter()
        .find(|skill| skill.name == ADDRESSING)
        .map_or(ADDRESSING, |skill| skill.path.as_str());

    let mut prompt = format!(
        "Read {skill} and address the feedback at the end of this prompt, the way it says. \
         Nothing else in this session tells you how to reach me.\n\
         \n# The Brief this started from\n\n{}\n",
        facts.brief,
    );

    if let Some(handoff) = &facts.handoff {
        prompt.push_str(&format!(
            "\n# What the grilling settled, in its own words\n\n{handoff}\n"
        ));
    }

    format!(
        "{prompt}\n# The feedback to address\n\n{}\n",
        feedback.trim()
    )
}

/// Where the Repo named `repo` is checked out: the Worktree, or a Companion's.
fn worktree<'f>(facts: &'f Facts, repo: &str) -> &'f str {
    std::iter::once(&facts.worktree)
        .chain(&facts.companions)
        .find(|checkout| checkout.repo == repo)
        .map_or("", |checkout| checkout.path.as_str())
}

/// The pull request in words: its number, and the Repo it is in.
fn named(pull_request: &PullRequest) -> String {
    format!(
        "pull request #{} of `{}`",
        pull_request.number, pull_request.repo
    )
}

/// `what` opening a sentence.
fn opening(what: &str) -> String {
    let mut letters = what.chars();

    match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    }
}

/// One check per line: what it is called, and where its run is.
fn listed(checks: &[Check]) -> String {
    checks
        .iter()
        .map(|check| match check.link.is_empty() {
            true => format!("- {}", check.name),
            false => format!("- {} — {}", check.name, check.link),
        })
        .collect::<Vec<String>>()
        .join("\n")
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
