//! **The wrap-up** (ADR-0030, *Activity*): one Activity watching every pull
//! request the work is on until Verkstead has finished with it, its watchers
//! declared as options and run side by side inside it (Set 1767, Q32).
//!
//! **Its watchers are options**: `checks`, `comments`, `review` and
//! `mergeable`, each `on` unless the Process turns it `off` — which settles it
//! on every round, the way Fix Merge Issues has its review and comments
//! settled from the start. The review may also be `afresh`, which is a Resume
//! asking for the branch read again whatever the last review left standing.
//! All four run here, side by side, reaching the pull requests through the
//! Forge alone.
//!
//! **The companions first.** Before anything reads the work whole, the host is
//! asked to find the pull requests the work's Companions were carried to and
//! record each beside the Conversation's own — which gives the wrap-up up
//! where one was committed in and left without any.
//!
//! **The review**, once, across every pull request: a session inside the
//! reviewing skill under the Review role, launched where the host says the
//! branch is to be read — not where it has been, where there is to be no
//! review, or where a review's own proposal is standing with nobody behind it,
//! which the host stops the run over. What is already standing on the pull
//! requests is folded into its prompt and marked addressed as it launches, so
//! nothing is later sent to do ungated what its Set is about to propose. How
//! it ended is the host's to see to: the review settled, the work sent back to
//! build what it split out, or the run stopped. `review-after` holds it back
//! that many milliseconds, a window the suite holds open.
//!
//! **What is said after it**, on every pull request every round. A comment
//! nobody has been sent to deal with — not one a share left, nor one the
//! human's ignore rules match, both the host's to say — puts that pull
//! request's comments back to waiting, and nothing is settled or answered
//! until the review is over: until then, whatever is said is the review's to
//! fold in. After it, a pull request with nothing outstanding settles, where
//! nothing is running, and one with something dispatches a **batch session**
//! about it inside the responding skill, told which pull request and where to
//! work, its comments marked addressed as it launches. A batch session's
//! proposal standing with nobody behind it is the host's to stop over, and
//! nothing said is settled or answered while it stands. A Forge that lacks
//! review settles a pull request's comments at once, and the review with them.
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
//! waiting on it, and holds it for as long as the session runs, the wait on
//! the human included: a fix queued behind the review or a batch would be
//! about a suite nobody had looked at since. The next round looks again, and
//! with the Worktree free it dispatches the review first, then a fix or a
//! resolution, then a batch. The pull request a session is
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
//! the Forge has no checks or no review for — it learns again.

wit_bindgen::generate!({
    world: "activity",
    path: "../../../crates/server/wit",
});

use std::collections::HashSet;
use std::time::{Duration, Instant};

use verkstead::plugin::activity_types::{
    Check, Checked, Comment, Ends, Evidence, Mergeable, PullRequest, PullRequestStanding,
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

/// The skill the review runs inside.
const REVIEWING: &str = "verkstead/reviewing";

/// The skill a batch session runs inside.
const RESPONDING: &str = "verkstead/responding";

/// What separates one comment from the next, and one pull request's comments
/// from another's: a rule, which is what a human reading markdown on a phone
/// sees a break as.
const BETWEEN: &str = "\n\n---\n\n";

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

        // Before anything reads the work whole: which pull requests there are
        // is what the review reads, and what is said on each of them.
        host::cover();

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

            let running: Vec<Session> = watching
                .running
                .iter()
                .map(|(session, _)| *session)
                .collect();

            if let Woke::Waited((session, waited)) = host::wait_any(&running, poll) {
                watching.over(session, waited);
            }
        }
    }
}

export!(WrapUp);

/// The watchers, across rounds.
struct Watching<'a> {
    facts: &'a Facts,
    attempts: i64,

    /// The session dispatched and not yet over, and what it was dispatched
    /// for.
    running: Option<(Session, Dispatched)>,

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

    /// The pull requests the Forge has no review for: their comments settled
    /// at once and never asked about again, and the review with them.
    silent: HashSet<(String, i64)>,

    /// Whether the review reads past what the last one left standing, until a
    /// review has been launched.
    afresh: bool,

    /// Whether this run's review is behind it: launched, or found settled.
    /// One run reads the branch once, as its wrap's review. A settle that
    /// goes again under it is a wrap of its own begun since — the work sent
    /// back to be built and a pull request recorded again in the moment
    /// between two rounds — whose review is the run that wrap starts.
    reviewed: bool,

    /// When this started, and how long after it the review is held back.
    started: Instant,
    review_after: Duration,
}

/// What a session was dispatched for, which is what is seen to once it is
/// over.
enum Dispatched {
    /// A fix or a resolution: nothing to see to, the next round looking again.
    Fix,
    /// The review, which the host sees out.
    Review,
    /// A batch about these comments on this pull request, which the host sees
    /// out.
    Batch(PullRequest, Vec<String>),
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
            running: None,
            held: None,
            reported: HashSet::new(),
            lacking: HashSet::new(),
            silent: HashSet::new(),
            afresh: option(facts, "review") == Some("afresh"),
            reviewed: false,
            started: Instant::now(),
            review_after: Duration::from_millis(
                option(facts, "review-after")
                    .and_then(|after| after.parse().ok())
                    .unwrap_or(0),
            ),
        }
    }

    /// Look at every pull request, settle the wrap-up on what each came to,
    /// and dispatch at most one session — the review, or about the first pull
    /// request that needs one — where none is running and a round has passed
    /// since a fix.
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

        let said = self.reading(pull_requests);

        // Nothing dispatched beside a session, nor in the round after a fix.
        if self.running.is_some() || self.held.take().is_some() {
            return;
        }

        if self.review(pull_requests) {
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

        if let Some((pull_request, fresh)) = said.into_iter().next() {
            self.answer(&pull_request, &fresh);
        }
    }

    /// See to a session that is over, by what it was dispatched for: the host
    /// sees the review and a batch out — `how` it ended, where that was the
    /// trouble — and a fix needs nothing. One the human steered past has
    /// nothing left to see to.
    fn over(&mut self, session: Session, waited: Waited) {
        if self.running.as_ref().map(|(running, _)| *running) != Some(session) {
            return;
        }

        let Some((_, dispatched)) = self.running.take() else {
            return;
        };

        let how = match waited {
            Waited::Steered => return,
            Waited::Gone(how) => how,
            _ => None,
        };

        match dispatched {
            Dispatched::Fix => {}
            Dispatched::Review => host::review_over(how.as_deref()),
            Dispatched::Batch(pull_request, which) => {
                host::batch_over(&pull_request, &which, how.as_deref())
            }
        }
    }

    /// Read what is said on every pull request and settle the wrap-up on it,
    /// answering with what is outstanding on each — once the review is over,
    /// until when nothing said is anybody's but the review's.
    fn reading(&mut self, pull_requests: &[PullRequest]) -> Vec<(PullRequest, Vec<Comment>)> {
        if off(self.facts, "comments") {
            return Vec::new();
        }

        // Read before the comments are, which is what makes the two agree: the
        // review marks what it folds in as it launches, so a reading of the
        // comments from before that and of the review from after it would
        // answer the very comments the review was given.
        let reviewed = host::settled(&Watcher::Review);

        // A batch's proposal standing with nobody behind it is a question with
        // nobody to read the answer — and its comments marked addressed. Nothing
        // is settled or answered over it.
        if reviewed && host::left_behind() {
            return Vec::new();
        }

        let mut outstanding = Vec::new();

        for pull_request in pull_requests {
            // Nothing is touched where the Forge couldn't be asked.
            let Some(fresh) = self.unaddressed(pull_request) else {
                continue;
            };

            let watcher = Watcher::Comments(pull_request.clone());

            // Not until the review is over, nor while something is working —
            // a batch just dispatched reads exactly as one that is over, its
            // comments addressed and nothing asked yet.
            if fresh.is_empty() {
                if reviewed && self.running.is_none() {
                    host::settle(&watcher);
                }

                continue;
            }

            host::unsettle(&watcher);

            if reviewed {
                outstanding.push((pull_request.clone(), fresh));
            }
        }

        outstanding
    }

    /// What is said on `pull_request` that nobody has been sent to deal with
    /// yet — not what was marked addressed, nor what the host says is nobody's
    /// to answer — or `None` where the Forge could not be asked, which is
    /// neither *nothing was said* nor *something was*.
    fn unaddressed(&mut self, pull_request: &PullRequest) -> Option<Vec<Comment>> {
        let key = (pull_request.repo.clone(), pull_request.number);

        // Settled again every round, as the checks are where there are none.
        if self.silent.contains(&key) {
            host::settle(&Watcher::Comments(pull_request.clone()));
            return None;
        }

        let said = match host::forge_comments(pull_request) {
            Ok(said) => said,
            // Settled by the host as it answered.
            Err(Unforged::Lacking) => {
                self.silent.insert(key);
                return None;
            }
            Err(Unforged::Unasked(_)) => return None,
        };

        let already = host::addressed(&pull_request.repo).ok()?;

        let new: Vec<Comment> = said
            .into_iter()
            .filter(|comment| !already.contains(&comment.which))
            .collect();

        let ignored = host::ignored(&pull_request.repo, &new);

        Some(
            new.into_iter()
                .filter(|comment| !ignored.contains(&comment.which))
                .collect(),
        )
    }

    /// Launch the review, where the host says the branch is to be read, with
    /// whatever is standing on the pull requests folded in. Whether it was.
    fn review(&mut self, pull_requests: &[PullRequest]) -> bool {
        if off(self.facts, "review") {
            return false;
        }

        // A Forge with no review has nothing for it to wait on.
        if !self.silent.is_empty() {
            host::settle(&Watcher::Review);
            return false;
        }

        if self.reviewed {
            return false;
        }

        if host::settled(&Watcher::Review) {
            self.reviewed = true;
            return false;
        }

        if self.started.elapsed() < self.review_after || !host::review_wanted(self.afresh) {
            return false;
        }

        let mut said = Vec::new();
        let mut folded = Vec::new();

        for pull_request in pull_requests {
            // One that couldn't be asked about costs the review that one's
            // comments, which the batch after it picks up.
            let Some(fresh) = self.unaddressed(pull_request) else {
                if !self.silent.is_empty() {
                    host::settle(&Watcher::Review);
                    return false;
                }

                continue;
            };

            if fresh.is_empty() {
                continue;
            }

            said.push(said_by(pull_request, &fresh));
            folded.push((pull_request.clone(), which(&fresh)));
        }

        let said = (!said.is_empty()).then(|| said.join(BETWEEN));

        let Ok(told) = host::reviewing(said.as_deref()) else {
            return false;
        };

        let Ok(session) = host::launch(
            REVIEWING,
            "review",
            &told,
            &Evidence::Nothing,
            Ends::WithItsWork,
        ) else {
            return false;
        };

        // Marked as it launches rather than as it ends, so that a server that
        // restarted over it does not hand them to a batch as well.
        for (pull_request, which) in folded {
            let _ = host::mark_addressed(&pull_request.repo, &which);
        }

        self.afresh = false;
        self.reviewed = true;
        self.running = Some((session, Dispatched::Review));
        true
    }

    /// Dispatch a batch session about what is outstanding on `pull_request`.
    /// Whether it was.
    fn answer(&mut self, pull_request: &PullRequest, fresh: &[Comment]) -> bool {
        let told = format!(
            "These comments were left on {}. Work in that repository's worktree, at `{}` — both \
             `git` and `gh` read the repository from wherever they are run, so a diff read or a \
             commit made anywhere else is a different repository's.\n\n{}",
            named(pull_request),
            worktree(self.facts, &pull_request.repo),
            said_by(pull_request, fresh),
        );

        let Ok(session) = host::launch(
            RESPONDING,
            "implementation",
            &responding(self.facts, &told),
            &Evidence::Nothing,
            Ends::WithItsWork,
        ) else {
            return false;
        };

        // Marked as it launches, so a batch a server dispatched and then
        // restarted over is not dispatched twice.
        let which = which(fresh);
        let _ = host::mark_addressed(&pull_request.repo, &which);

        self.running = Some((session, Dispatched::Batch(pull_request.clone(), which)));
        true
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

        self.running = Some((session, Dispatched::Fix));
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

        self.running = Some((session, Dispatched::Fix));
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
    let skill = skill(facts, ADDRESSING);

    format!(
        "{}\n# The feedback to address\n\n{}\n",
        on_the_documents(
            facts,
            &format!(
                "Read {skill} and address the feedback at the end of this prompt, the way it says."
            ),
        ),
        feedback.trim()
    )
}

/// What a batch session is started on: the documents, and under them what has
/// just been said.
fn responding(facts: &Facts, said: &str) -> String {
    let skill = skill(facts, RESPONDING);

    format!(
        "{}\n# What has just been said on the pull request\n\n{}\n",
        on_the_documents(
            facts,
            &format!(
                "Read {skill} and answer what has just been said on the pull request named at \
                 the end of this prompt, the way it says. The work described below is what it \
                 was meant to be."
            ),
        ),
        said.trim()
    )
}

/// The opening line naming the skill, and the Brief and the handoff under it.
fn on_the_documents(facts: &Facts, opening: &str) -> String {
    let mut prompt = format!(
        "{opening} Nothing else in this session tells you how to reach me.\n\
         \n# The Brief this started from\n\n{}\n",
        facts.brief,
    );

    if let Some(handoff) = &facts.handoff {
        prompt.push_str(&format!(
            "\n# What the grilling settled, in its own words\n\n{handoff}\n"
        ));
    }

    prompt
}

/// Where a session reads the skill `name`'s `SKILL.md`.
fn skill<'f>(facts: &'f Facts, name: &'f str) -> &'f str {
    facts
        .skills
        .iter()
        .find(|skill| skill.name == name)
        .map_or(name, |skill| skill.path.as_str())
}

/// What was said, in the order it was said in, whole and in the markdown it
/// was written in, each with the pull request it was said on and — for one
/// left on the diff — the file and the line: "this is the wrong way round" is
/// an instruction with them and a riddle without.
fn said_by(pull_request: &PullRequest, fresh: &[Comment]) -> String {
    fresh
        .iter()
        .map(|comment| {
            let who = match comment.author.is_empty() {
                true => "Somebody".to_owned(),
                false => format!("**{}**", comment.author),
            };

            let where_said = match comment.about.is_empty() {
                true => String::new(),
                false => format!(", on {}", comment.about),
            };

            format!(
                "{who} said on {}{where_said}:\n\n{}",
                named(pull_request),
                comment.markdown.trim(),
            )
        })
        .collect::<Vec<String>>()
        .join(BETWEEN)
}

/// What each of the comments is, which is what it is marked addressed by.
fn which(comments: &[Comment]) -> Vec<String> {
    comments
        .iter()
        .map(|comment| comment.which.clone())
        .collect()
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
