//! The Done signal: a session saying its work is finished, by running
//! `verkstead done`.
//!
//! Every session Verkstead launches is an interactive agent, which idles when
//! its work is over rather than exiting, so something has to end it. What used
//! to was a guess read from outside — the work on the branch plus a few seconds
//! of quiet — and an agent that waits on something in the background wears
//! exactly that shape while it is still at work. So the agent says it is done,
//! and that is what ends it. See ADR-0018.
//!
//! **The repository is still read, at that moment and only then.** What *done*
//! means is the kind's own reading, unchanged — see [`Evidence`] and
//! [`crate::runner::Landing`] — and what it is for now is the check on a signal
//! rather than the trigger for an ending. A signal the evidence does not bear
//! out is refused, naming what is missing, and the session stays alive to put
//! it right in the same turn.
//!
//! **The signal is a bare verb**, because the server already knows which
//! session is running in the Conversation and what it was sent for: whatever is
//! seeing that session out writes down here what it is waiting for — see
//! [`Signals::expecting`] — and the route reads it back. A session nothing has
//! written anything down for is refused by name: one still grilling with no
//! Direction picked has no artifact to have finished.
//!
//! The ending is the driver's rather than the route's. An accepted signal is
//! remembered against the session, and the driver seeing it out ends it once it
//! is next idle — so the closing words an agent prints after the command reach
//! the Transcript. See [`crate::runner`]'s `see_out`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio::sync::watch;
use verkstead_schema::ApiError;

use crate::AppState;
use crate::reply::yaml;
use crate::runner::Landing;
use crate::store::{self, Lifecycle};

/// What each Conversation's running session is being seen out for, where a
/// driver is seeing one out on the Done signal.
///
/// Cloned into [`crate::AppState`] the way the other registers are, and shared
/// by every clone: what a session was sent for is a fact about the process
/// rather than about any one handler.
#[derive(Clone, Default)]
pub(crate) struct Signals {
    expected: Arc<Mutex<HashMap<i64, Expected>>>,

    /// Which registration is which, so that one taken off the register is only
    /// ever the one that put itself there — see [`Expecting`].
    issued: Arc<AtomicU64>,
}

/// One session's entry: which session, what landed looks like for it, and the
/// word that it signalled and was believed.
struct Expected {
    token: u64,
    event_id: i64,
    evidence: Evidence,
    ends: Ends,
    given: watch::Sender<bool>,
}

/// What a session's Done signal is checked against: the kind's own reading of
/// what *done* is, unchanged from when it was the trigger for an ending.
#[derive(Debug, Clone)]
pub(crate) enum Evidence {
    /// A backlog step, the finish, a stage's planning or a grilling's tail after
    /// a pick: `landing` in `worktree` — see [`crate::runner::Landing`].
    Landed { worktree: PathBuf, landing: Landing },

    /// An inline run or an instruction: the Conversation's commits standing past
    /// `already`, where they stood when the session started — see
    /// [`crate::runner::committed_since`]. There is no path to watch for either,
    /// and a commit is the one report an agent cannot half make.
    Committed { already: i64 },

    /// A fix session, which has nothing to show. What judges a fix is the check
    /// on GitHub, asked again once the session is over, and a rule that demanded
    /// a commit would leave a fix with nothing to fix unable to end.
    Nothing,

    /// A follow-up, whose end is the human's to say rather than the session's:
    /// the newest round they answered carries the Nothing-else mark — see
    /// [`crate::runner::marked`]. What it commits is theirs to have asked for,
    /// and a round that was a question and an answer commits nothing at all, so
    /// there is nothing on the branch to read instead.
    NothingElse,
}

/// Whether a session is done with its work alone, or only once its branch is on
/// a pull request as well.
///
/// The second is every session a run ends on: a backlog's finish step, an inline
/// implementation, a roadmap's own session, and the session sent to open the
/// pull request one of those did not. Each commits its work and then pushes and
/// opens the pull request, so each can land everything it was sent for and stop
/// short of the one act that makes the work reviewable. Refused there, that is
/// caught in the same turn rather than by a second session sent afterwards — see
/// [`crate::runner`]'s `to_a_pull_request`, which stays as the net under a
/// session that exits without signalling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ends {
    WithItsWork,
    OnAPullRequest,
}

/// A driver's hold on its entry, for as long as it is seeing the session out.
///
/// A guard rather than a pair of calls, for the reason [`crate::drivers`]'s
/// registration is one: a driver cancelled mid-watch — a later pick superseding
/// the one that armed it, above all — has to take its entry with it, or a
/// signal would be checked against an artifact nobody is waiting for any more.
pub(crate) struct Expecting {
    signals: Signals,
    conversation_id: i64,
    token: u64,
    given: watch::Receiver<bool>,
}

/// Whether a session has given the Done signal, as whoever is waiting on it
/// holds that.
#[derive(Debug, Clone)]
pub(crate) struct Signal {
    given: watch::Receiver<bool>,
}

impl Signals {
    pub(crate) fn new() -> Signals {
        Signals::default()
    }

    /// Write down that the session printing into `event_id` is to be ended on
    /// its Done signal, once `evidence` bears it out — and, where it `ends` on
    /// one, once its branch has a pull request open.
    ///
    /// Replaces whatever the Conversation had written down: one Worktree holds
    /// one session, and the driver seeing it out now is the one that knows what
    /// it was sent for.
    pub(crate) fn expecting(
        &self,
        conversation_id: i64,
        event_id: i64,
        evidence: Evidence,
        ends: Ends,
    ) -> Expecting {
        let token = self.issued.fetch_add(1, Ordering::Relaxed);
        let (given, receiver) = watch::channel(false);

        self.register().insert(
            conversation_id,
            Expected {
                token,
                event_id,
                evidence,
                ends,
                given,
            },
        );

        Expecting {
            signals: self.clone(),
            conversation_id,
            token,
            given: receiver,
        }
    }

    fn register(&self) -> std::sync::MutexGuard<'_, HashMap<i64, Expected>> {
        self.expected
            .lock()
            .expect("the done signals register is not poisoned")
    }
}

impl Expecting {
    /// The signal this entry is waiting on, to be handed to whatever waits.
    pub(crate) fn signal(&self) -> Signal {
        Signal {
            given: self.given.clone(),
        }
    }
}

impl Drop for Expecting {
    fn drop(&mut self) {
        let mut register = self.signals.register();

        if register
            .get(&self.conversation_id)
            .is_some_and(|expected| expected.token == self.token)
        {
            register.remove(&self.conversation_id);
        }
    }
}

impl Signal {
    /// Whether the signal has been given and accepted yet.
    pub(crate) fn given(&self) -> bool {
        *self.given.borrow()
    }

    /// Wait until it has been. Never returns where it never is, which is what
    /// makes it an arm of a `select!`.
    pub(crate) async fn arrived(mut self) {
        if self.given.wait_for(|given| *given).await.is_err() {
            // The entry went without the signal ever being given: nothing will
            // give it now, and nothing is waiting for this to say otherwise.
            std::future::pending::<()>().await;
        }
    }
}

/// What the route answers with, either way.
enum Verdict {
    Accepted,
    Refused(String),
}

/// `POST /conversations/{conversation}/api/v1/done` — the session running in
/// this Conversation says its work is finished.
///
/// 200 where the work has landed by its kind's own reading, and the session is
/// ended once it is next idle. 409 otherwise, with the reason in words the agent
/// can act on in the same turn: what is missing, no session here to end, no
/// Direction picked yet, a follow-up the human has not said is over, or a
/// session a run ends on whose branch has no pull request open. The session is
/// left exactly as it was.
pub(crate) async fn signal(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
) -> Response {
    match verdict(&state, conversation_id).await {
        Verdict::Accepted => {
            tracing::info!(
                conversation_id,
                "a session said it is done and its work bears that out, so it is to be ended"
            );

            (StatusCode::OK, "accepted\n").into_response()
        }
        Verdict::Refused(why) => {
            tracing::info!(
                conversation_id,
                why,
                "a session said it is done and was refused"
            );

            yaml(StatusCode::CONFLICT, &ApiError::new(why))
        }
    }
}

/// Decide a signal: check it against what the session was sent for, and
/// remember it where it holds.
async fn verdict(state: &AppState, conversation_id: i64) -> Verdict {
    let Some(event_id) = state.sessions.writing(conversation_id) else {
        return Verdict::Refused(
            "there is no session running in this Conversation, so there is nothing here to end"
                .to_owned(),
        );
    };

    let Some((token, evidence, ends)) = registered(state, conversation_id, event_id).await else {
        return Verdict::Refused(unexpected(state, conversation_id).await);
    };

    // Not a gap the session can close by itself, so not said as one: whether
    // there is anything else is the human's, and the move that asks them is a Set.
    if matches!(evidence, Evidence::NothingElse)
        && !crate::runner::marked(state, conversation_id).await
    {
        return Verdict::Refused(
            "the human has not said there is nothing else, so this follow-up is not over: put \
             the next round to them as a Set with `verkstead ask`"
                .to_owned(),
        );
    }

    if let Some(missing) = missing(state, conversation_id, &evidence).await {
        return Verdict::Refused(format!(
            "this session is not done yet: {missing}. Put that right, then run `verkstead done` \
             again"
        ));
    }

    if let Some(uncommitted) = uncommitted(state, conversation_id).await {
        return Verdict::Refused(uncommitted);
    }

    // After the commit rather than before it, which is the order the work goes
    // in: a pull request is opened on what was committed and pushed.
    if ends == Ends::OnAPullRequest
        && let Some(unopened) = unopened(state, conversation_id).await
    {
        return Verdict::Refused(unopened);
    }

    // The same entry the check was made for, rather than whichever is there now:
    // one put there in the meantime is a driver waiting on something else.
    let taken = match state
        .signals
        .register()
        .get(&conversation_id)
        .filter(|expected| expected.token == token)
    {
        Some(expected) => {
            expected.given.send_replace(true);
            true
        }
        None => false,
    };

    if !taken {
        return Verdict::Refused(
            "what this session was sent for changed while the signal was being checked, so run \
             `verkstead done` again"
                .to_owned(),
        );
    }

    abandoned(state, conversation_id, event_id).await;

    Verdict::Accepted
}

/// How long a signal from a running session waits for its driver to write down
/// what it is to be checked against, before it is refused as unexpected.
///
/// A driver writes its entry once the session it launched is running, so there
/// is a moment in which a session is on the register and nothing is waiting on
/// it yet. An agent that signals in that moment is not wrong, and refusing it as
/// a session Verkstead ends by itself would send it off believing that. A few
/// seconds covers the moment on a loaded machine.
const REGISTERING: Duration = Duration::from_secs(5);

/// The entry waiting on the session printing into `event_id`, as its token and
/// what it is checked against — waited for up to [`REGISTERING`].
///
/// Read under the lock and checked outside it: the check is git, and a lock
/// held across a process would hold every other Conversation's signal behind it.
async fn registered(
    state: &AppState,
    conversation_id: i64,
    event_id: i64,
) -> Option<(u64, Evidence, Ends)> {
    let deadline = Instant::now() + REGISTERING;

    loop {
        let found = state
            .signals
            .register()
            .get(&conversation_id)
            .filter(|expected| expected.event_id == event_id)
            .map(|expected| (expected.token, expected.evidence.clone(), expected.ends));

        // Given up on once the session is no longer the one running, too: a
        // session that has ended is not one anything will start waiting on.
        if found.is_some()
            || Instant::now() >= deadline
            || state.sessions.writing(conversation_id) != Some(event_id)
        {
            return found;
        }

        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// What `evidence` is still missing, in words an agent can act on, or `None`
/// where it bears the signal out.
async fn missing(state: &AppState, conversation_id: i64, evidence: &Evidence) -> Option<String> {
    match evidence {
        Evidence::Landed { worktree, landing } => crate::runner::missing(worktree, landing).await,
        Evidence::Committed { already } => {
            // Swept first rather than read as the watcher last left it: a
            // session's `verkstead done` usually comes straight after its
            // commit, and the watcher looks at the branch only every few
            // seconds.
            crate::commits::sweep_now(state, conversation_id).await;

            (!crate::runner::committed_since(state, conversation_id, *already).await)
                .then(|| "nothing has been committed since this session began".to_owned())
        }
        // Read in [`verdict`], where its refusal is said in words of its own.
        Evidence::Nothing | Evidence::NothingElse => None,
    }
}

/// How many uncommitted paths a refusal names before it says how many more
/// there are: enough to act on, few enough that the agent reads to the end.
const NAMED: usize = 10;

/// Why a signal is refused over uncommitted changes, or `None` where every
/// repository the session may write in is clean.
///
/// The repositories are the Diff's — see [`crate::diffs::writable`]: the
/// Worktree, then each read-write companion. A read-only companion is not looked
/// at, because nothing a session did can be in it. Read through
/// [`crate::repos::git`], without optional locks, because the session may be
/// committing at this very moment.
///
/// A repository that will not answer refuses the signal, saying so: the session
/// is alive and can simply signal again, where a signal taken over a Worktree
/// nobody could read would end a session over work nobody saw committed.
async fn uncommitted(state: &AppState, conversation_id: i64) -> Option<String> {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading a Conversation to check its session's signal failed"
            );
            return Some(
                "Verkstead could not read this Conversation to check for uncommitted changes, \
                 so run `verkstead done` again in a moment"
                    .to_owned(),
            );
        }
    };

    let readings = crate::diffs::writable(&conversation);

    let found = tokio::task::spawn_blocking(move || {
        readings
            .into_iter()
            .filter_map(|reading| {
                let place = if reading.own {
                    "the Worktree".to_owned()
                } else {
                    format!("the companion repo `{}`", reading.repo)
                };

                match changed(&reading.worktree) {
                    Some(paths) if paths.is_empty() => None,
                    Some(paths) => Some(format!(
                        "{place} has uncommitted changes: {}",
                        listed(&paths)
                    )),
                    None => Some(format!(
                        "git would not say whether {place} has uncommitted changes"
                    )),
                }
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_else(|_| vec!["git would not say whether there are uncommitted changes".to_owned()]);

    (!found.is_empty()).then(|| {
        format!(
            "this session is not done yet: {}. Commit those changes or discard them, then run \
             `verkstead done` again",
            found.join("; ")
        )
    })
}

/// Every path git sees as changed in `worktree` — modified, staged, or untracked
/// and not ignored — or `None` where git will not answer.
fn changed(worktree: &std::path::Path) -> Option<Vec<String>> {
    let status = crate::repos::git(
        worktree,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;

    let mut paths = Vec::new();
    let mut entries = status.split('\0').filter(|entry| !entry.is_empty());

    while let Some(entry) = entries.next() {
        let Some((code, path)) = entry.split_at_checked(3) else {
            continue;
        };

        // A rename or a copy is followed by the path it came from, which is
        // the same change rather than another one.
        if code.starts_with(['R', 'C']) {
            entries.next();
        }

        paths.push(path.to_owned());
    }

    Some(paths)
}

/// `paths` as a refusal names them: in backticks, cut short after [`NAMED`].
fn listed(paths: &[String]) -> String {
    let mut said = paths
        .iter()
        .take(NAMED)
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ");

    if paths.len() > NAMED {
        said.push_str(&format!(" and {} more", paths.len() - NAMED));
    }

    said
}

/// Why a signal is refused over a branch with no pull request open, or `None`
/// where it has one — or where GitHub could not be asked.
///
/// Asked the way the runner asks once one of these sessions is over — see
/// [`crate::wrapping::asked`] — in the Conversation's repository and under the
/// name the branch was pushed as, so that in a stack it is this branch's own
/// pull request that is asked about.
///
/// **GitHub out of reach reads as accepted.** No `gh`, no account, no remote, a
/// rate limit or an answer nobody can read: none of them is GitHub saying there
/// is no pull request, and a session is not to be held hostage by somebody
/// else's outage. What comes after the session asks again either way.
async fn unopened(state: &AppState, conversation_id: i64) -> Option<String> {
    let Some((_, branch, found)) = crate::wrapping::asked(state, conversation_id).await else {
        tracing::warn!(
            conversation_id,
            "whether the branch has a pull request could not be checked, so the Done signal is \
             taken without it"
        );
        return None;
    };

    match found {
        Ok(_) => None,
        Err(crate::github::Trouble::NoPullRequest) => Some(format!(
            "this session is not done yet: the branch `{branch}` has no open pull request. Push \
             it and open one the way the repository's own review process says, then run \
             `verkstead done` again"
        )),
        Err(trouble) => {
            tracing::warn!(
                conversation_id,
                branch,
                why = trouble.why(),
                "whether the branch has a pull request could not be checked, so the Done signal \
                 is taken without it"
            );
            None
        }
    }
}

/// Lock the Sets the session that signalled was idling on, now that nothing will
/// read their Answers.
///
/// Its own, read the way the runner reads them — every Set that landed after the
/// session's Event, one Worktree holding one agent — and the ones it was idling
/// on: a Blocking Ask, and a store-and-nudge one it stood behind. A Deferred Ask
/// is left standing, nothing having ever waited on one, and its Answers reach a
/// later session by design. The locking is [`crate::sets::lock`], the same a
/// relaunched grilling does, so the human sees one kind of locked Set.
async fn abandoned(state: &AppState, conversation_id: i64, event_id: i64) {
    let timeline = match store::timeline(&state.pool, conversation_id).await {
        Ok(timeline) => timeline,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading what a session that said it is done left open failed"
            );
            return;
        }
    };

    let since = timeline
        .into_iter()
        .filter(|event| event.id > event_id)
        .collect::<Vec<_>>();

    crate::sets::lock(
        state,
        conversation_id,
        &crate::sets::open(&since, crate::sets::Open::Idled),
        "the session that asked it said it is done",
    )
    .await;
}

/// Why a session nothing is waiting on a signal from was refused.
///
/// A grilling that has not been picked on is the one said by name: it has no
/// artifact to have finished, and the move that gives it one is the human's. A
/// Direction picked a moment ago is the watcher not yet armed, and says so.
async fn unexpected(state: &AppState, conversation_id: i64) -> String {
    let grilling = matches!(
        store::state(&state.pool, conversation_id).await,
        Ok(Some(Lifecycle::Grilling))
    );

    if !grilling {
        return "Verkstead ends this session by itself once its work is there, so there is \
                nothing for `verkstead done` to say"
            .to_owned();
    }

    match store::picked_direction(&state.pool, conversation_id).await {
        Ok(Some(_)) => "the Direction the human picked is still being taken up, so run \
                        `verkstead done` again in a moment"
            .to_owned(),
        _ => "no Direction has been picked yet, so there is nothing for this session to have \
              finished — propose one with `verkstead ask` and write what the human picks"
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(signals: &Signals, event_id: i64) -> Expecting {
        signals.expecting(
            7,
            event_id,
            Evidence::Landed {
                worktree: PathBuf::from("/nowhere"),
                landing: Landing::Ticked(1),
            },
            Ends::WithItsWork,
        )
    }

    /// Dropping a hold takes its entry off the register, which is what keeps a
    /// cancelled driver's artifact from being checked against.
    #[test]
    fn a_hold_let_go_of_takes_its_entry_with_it() {
        let signals = Signals::new();

        drop(entry(&signals, 1));

        assert!(signals.register().get(&7).is_none());
    }

    /// And never somebody else's: a driver that replaced it owns the entry now.
    #[test]
    fn a_superseded_hold_leaves_the_entry_that_replaced_it() {
        let signals = Signals::new();

        let first = entry(&signals, 1);
        let _second = entry(&signals, 2);

        drop(first);

        assert_eq!(signals.register().get(&7).map(|e| e.event_id), Some(2));
    }

    /// A long list of uncommitted paths is cut short, saying how many more.
    #[test]
    fn a_long_list_of_changes_is_cut_short_saying_how_many_more() {
        let paths = (1..=13).map(|n| format!("{n}.md")).collect::<Vec<_>>();

        let said = listed(&paths);

        assert!(said.starts_with("`1.md`, `2.md`"), "{said}");
        assert!(said.contains("`10.md`"), "{said}");
        assert!(!said.contains("`11.md`"), "{said}");
        assert!(said.ends_with(" and 3 more"), "{said}");
    }

    /// Git's reading of a Worktree: modified, staged, untracked and renamed each
    /// named once, and an ignored file not at all.
    #[test]
    fn every_kind_of_uncommitted_change_is_read_and_an_ignored_file_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path();
        let run = |args: &[&str]| crate::repos::run(at, args).unwrap();

        run(&["init", "--quiet"]);
        std::fs::write(at.join(".gitignore"), "*.log\n").unwrap();
        std::fs::write(at.join("kept.md"), "kept\n").unwrap();
        std::fs::write(at.join("moved.md"), "moved\n").unwrap();
        run(&["add", "-A"]);
        run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "--quiet",
            "-m",
            "first",
        ]);

        assert_eq!(changed(at), Some(Vec::new()), "a clean Worktree has none");

        std::fs::write(at.join("kept.md"), "changed\n").unwrap();
        std::fs::write(at.join("staged.md"), "staged\n").unwrap();
        run(&["add", "staged.md"]);
        run(&["mv", "moved.md", "renamed.md"]);
        std::fs::write(at.join("stray.md"), "stray\n").unwrap();
        std::fs::write(at.join("build.log"), "noise\n").unwrap();

        let mut found = changed(at).unwrap();
        found.sort();

        assert_eq!(found, ["kept.md", "renamed.md", "staged.md", "stray.md"]);
    }

    /// A signal given reaches whoever holds a copy of it.
    #[tokio::test]
    async fn a_given_signal_arrives() {
        let signals = Signals::new();
        let hold = entry(&signals, 1);
        let signal = hold.signal();

        assert!(!signal.given());

        signals.register().get(&7).unwrap().given.send_replace(true);

        assert!(signal.given());
        tokio::time::timeout(std::time::Duration::from_secs(1), signal.arrived())
            .await
            .expect("the signal arrives once given");
    }
}
