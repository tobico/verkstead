//! The hold that stands in front of a stage's finish until every stage already
//! in its roadmap's chain has settled.
//!
//! A roadmap is one chain of branches in the order its stages finish, and a
//! stage **joins** it at its finish: the branch is rebased onto the top, pushed
//! and opened as a pull request — see
//! [ADR-0021](../../../../docs/adr/0021-parallel-stages.md), *The chain*, and
//! `store::stage_chain`, which is the chain read back off the pull requests
//! that were opened.
//!
//! **Nothing rebases onto a branch that is still moving.** A stage below that
//! is still wrapping up — a review finding being fixed, a check going green —
//! is a branch that may yet be pushed again, so a stage whose tasks are all
//! done waits until every stage already in the chain has *settled*. That is
//! what makes the join safe to do once: a stage is rebased before it has a pull
//! request at all, and never after somebody has started reading one.
//!
//! **And one at a time, in the order the tasks finished.** Two stages whose
//! boxes are all ticked would both be released by one settle below them, and
//! both would rebase onto the same top and push. So each takes a place in a
//! queue as it arrives — `store::join_queue`, a stored fact because the wait may
//! be long and a restart must not reorder it — and waits on every stage in front
//! of it *joining and settling*, the join putting it under the rule above. The
//! joins are the one thing a roadmap does in single file, which is the price of
//! never rebasing a branch anybody is reading: a stage whose wrap-up cannot
//! finish holds up every later stage's join, dependent on it or not. Only the
//! join. A waiting stage's own work, its checks, its comments and its review
//! wait on nobody.
//!
//! **The hold is the server's judgement rather than the session's.** It stands
//! in front of the launch rather than inside the finish skill, so a held stage
//! is one where no session has been started: there is no agent sitting in a
//! Worktree waiting to be told it may push, and nothing has been pushed that
//! would have to be taken back.
//!
//! **Only a stage is held.** An ordinary feature's backlog belongs to no roadmap
//! and joins no chain, so its finish is untouched — and so is a roadmap run in
//! order, which is every roadmap until stages start side by side: the stage
//! below settled before this one was ever started, so the first look finds
//! nothing to wait on and nothing is said at all.
//!
//! **Waiting to join is a condition of Implementing rather than a state**,
//! drawn the way *Waiting on checks* is drawn for a wrap-up: a line on the
//! Timeline, a label on the Conversation, nothing on the Lifecycle and no push
//! to the devices. There is nothing for the human to do about it — the stage
//! below is being got on with — so it is a label rather than a phone lighting
//! up.
//!
//! And it is read off the register below at the moment a page is drawn, for the
//! reason [`crate::drivers`] is read that way: the hold is a task of this
//! process, and a server that has just come back is holding nothing at all. The
//! resume that takes the stage up again is what finds it held a second time.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use verkstead_schema::Nudge;

use crate::AppState;
use crate::skills;
use crate::store;
use crate::worktrees;

/// How often a held stage looks at whether it may join yet.
///
/// Three reads of the database and nothing else, so this is not a cost: what it
/// decides is how long after the stage in front of it settles the one behind it
/// gets going, and ten seconds is shorter than the launch that follows it.
pub(crate) const LOOKED_AT_EVERY: Duration = Duration::from_secs(10);

/// Which Conversations are stages held before their finish, waiting for the
/// chain below them to settle.
///
/// Cloned into [`crate::AppState`] the way the drivers register is, and shared
/// by every clone: what is being held is a fact about this process rather than
/// about any one handler.
///
/// A set rather than a count, because unlike a driver there is only ever one
/// hold: a stage reaches its finish step once per run, and the run holds the
/// Conversation while it waits.
#[derive(Clone, Default)]
pub(crate) struct Joins {
    waiting: Arc<Mutex<HashSet<i64>>>,
}

/// One stage's hold, held for as long as it is waiting.
///
/// A guard rather than a pair of calls, for [`crate::drivers::Driving`]'s
/// reason: a run that panicked mid-wait and left the mark behind would be a
/// Conversation drawn as waiting to join for as long as the server stayed up,
/// with nothing left that could ever take it off.
pub(crate) struct Waiting {
    joins: Joins,
    conversation_id: i64,
}

impl Joins {
    /// A register with nothing on it, which is what a server starts with.
    pub(crate) fn new() -> Joins {
        Joins::default()
    }

    /// Say that `conversation_id` is waiting to join, until the guard is
    /// dropped.
    #[must_use = "a stage is held for as long as its hold is, \
                  so dropping one on the spot holds nothing"]
    fn hold(&self, conversation_id: i64) -> Waiting {
        self.waiting
            .lock()
            .expect("the joins register is not poisoned")
            .insert(conversation_id);

        Waiting {
            joins: self.clone(),
            conversation_id,
        }
    }

    /// Whether this Conversation is a stage waiting to join.
    ///
    /// What the Conversation page draws its label from — see [`crate::ui`].
    pub(crate) fn waiting(&self, conversation_id: i64) -> bool {
        self.waiting
            .lock()
            .expect("the joins register is not poisoned")
            .contains(&conversation_id)
    }

    /// And every one of them at once, which is what the sidebar wants: one lock
    /// for the whole list rather than one per row, exactly as the sessions
    /// register is read there.
    pub(crate) fn all_waiting(&self) -> HashSet<i64> {
        self.waiting
            .lock()
            .expect("the joins register is not poisoned")
            .clone()
    }
}

impl Drop for Waiting {
    fn drop(&mut self) {
        self.joins
            .waiting
            .lock()
            .expect("the joins register is not poisoned")
            .remove(&self.conversation_id);
    }
}

/// Hold this Conversation's finish until every stage already in its roadmap's
/// chain has settled, and say on the Timeline what it is waiting on.
///
/// Returns the moment there is nothing left to wait for, which is where the
/// finish session is launched exactly as it is today. A Conversation that is
/// not a stage returns at once and is never marked as waiting: an ordinary
/// feature's backlog joins no chain.
///
/// Called with the run's own driver registration still held, which is what
/// keeps a held stage from reading as one standing still — see
/// [`crate::drivers`]. A Conversation that has stopped while it waited returns
/// too, the launch behind this advancing nothing past a stop.
///
/// A record that cannot be read is not a record saying a stage is unsettled, so
/// it releases and says so in the log: a run held for ever on a database that
/// will not answer is a Conversation nothing could move, and everything else
/// the run is about to do asks the same database.
pub(crate) async fn hold(state: &AppState, conversation_id: i64) {
    let Some(of) = stage(state, conversation_id).await else {
        return;
    };

    // This stage's place in the queue, taken before anything is looked at and
    // taken once: a stage whose tasks finished first joins first, whether or not
    // there is anything to wait for now and whichever of the two launch sites
    // got here — and a restart, or a run that stopped mid-wait, finds the place
    // where it was rather than making a new one. See `store::queue_to_join`.
    if let Err(error) = store::queue_to_join(&state.pool, conversation_id).await {
        tracing::error!(
            error = ?error,
            conversation_id,
            "taking a stage's place in the queue to join failed",
        );
    }

    // The hold itself, taken the first time there is something to wait for and
    // let go as this returns — so a roadmap run in order is never marked at all.
    let mut waiting: Option<Waiting> = None;

    // And what was last said out loud, so that the line is written once per
    // answer rather than once per look: a stage below joining the chain while
    // this one waits changes what it is waiting on and why, and that is worth a
    // line of its own.
    let mut said: Vec<Ahead> = Vec::new();

    loop {
        let ahead = ahead_of(state, &of, conversation_id).await;

        if ahead.is_empty() {
            return;
        }

        if crate::stopping::stopped(state, conversation_id).await {
            tracing::info!(
                conversation_id,
                "the run has stopped, so there is nothing left waiting to join",
            );
            return;
        }

        if waiting.is_none() {
            waiting = Some(state.joins.hold(conversation_id));
        }

        if ahead != said {
            tracing::info!(
                conversation_id,
                roadmap = of.roadmap,
                waiting_on = ?ahead,
                "every task is done and the chain below has not settled, so the finish is held",
            );

            say(state, conversation_id, &of.roadmap, &ahead).await;
            said = ahead;
        }

        tokio::time::sleep(state.sessions.pace().joins).await;
    }
}

/// What the finish session is told the chain is: every branch this stage's own
/// goes on top of, bottom to top, with the branch it joins above last.
///
/// Asked the moment [`hold`] lets the stage in, and handed to the finish session
/// as a section of its prompt — see [`crate::skills::joining`], where the words
/// are. The chain is Verkstead's to say because Verkstead recorded it: which
/// stages joined and in what order is a fact about a pipeline rather than
/// anything a branch could be read for. What is done with it is the session's:
/// `gh stack` is a session's to run and never the server's.
///
/// Two readings put together, and the second is only ever reached at the bottom:
///
/// - **the stages that have joined**, in the order they joined — `store::stage_chain`;
/// - and **the branch this stage was cut from**, where that is not one of them,
///   which is the **foot** of the chain — `store::stacks_on`. A roadmap's chain
///   starts at the roadmap's own branch while its pull request is unmerged, and
///   that branch is no stage, so nothing above reads it back out of the chain.
///   A stage cut from a settled sibling — see [`crate::continuing`], where the
///   base is chosen — was cut from a link instead, and the first reading has
///   it already.
///
/// **A branch already in the default branch is out of it.** A stage that merged
/// is work the default branch holds, and a branch rebased onto a merged tip
/// would be moved back behind the merge that carried it in. The reading is git's
/// and it is the one [`crate::continuing`] makes where a stage's base is chosen,
/// down to what an unresolvable branch means: there is nothing there to stand on.
/// Which is what keeps a roadmap run in order finishing exactly as it does
/// today — a stage whose predecessors have all merged is told no chain at all,
/// and opens the ordinary pull request it always opened.
///
/// An empty answer is a finish told nothing, and there are three ways to it: a
/// Conversation that is not a stage, a chain with nothing unmerged in it, and a
/// record that would not be read. The last of them is [`hold`]'s reasoning
/// again — a database that will not answer is not a database saying there is a
/// chain — and it costs the rebase rather than the finish, which is the safe way
/// round: the branch is where it was cut from, and that is where it was going to
/// be rebased to.
pub(crate) async fn joining(state: &AppState, conversation_id: i64) -> Vec<skills::Link> {
    let Some(of) = stage(state, conversation_id).await else {
        return Vec::new();
    };

    let chain = match store::stage_chain(&state.pool, of.repo_id, &of.roadmap).await {
        Ok(chain) => chain,
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading the chain a stage is joining failed",
            );
            return Vec::new();
        }
    };

    // Two layers of `Option` and both mean nothing to add here: the outer is a
    // Conversation with no stage branch written down, the inner is a branch cut
    // off the default branch, which is a chain with no foot under it.
    let stands_on = match store::stacks_on(&state.pool, conversation_id).await {
        Ok(stands_on) => stands_on.flatten(),
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation_id,
                "reading what a joining stage's branch was cut from failed",
            );
            None
        }
    };

    let named = chain
        .iter()
        .map(|link| link.branch.clone())
        .chain(stands_on.clone())
        .collect();

    below(
        conversation_id,
        &chain,
        stands_on.as_deref(),
        &landed(&of, named).await,
    )
}

/// Which of these branches the default branch already holds — and which of them
/// are not there at all, which comes to the same thing for what is about to be
/// rebased onto them.
///
/// Git's reading rather than the record's. A pull request recorded merged is
/// written by the sweep that runs after a Conversation reaches Done, so a stage
/// merged since the last sweep is one the record still calls open; git is asked
/// about the branch itself, and it is asked at exactly the moment the answer is
/// acted on.
///
/// **Fetched first**, for the reason [`crate::continuing`]'s own reading is: what
/// the default branch holds is what origin is holding rather than wherever this
/// checkout's copy of it was last left, and a machine that has not pulled for a
/// week would read a stage merged a week ago as still in the chain. A fetch that
/// fails is said in the log and the reading goes on off what is here — telling a
/// session no chain at all because the network was down would cost it the rebase
/// it came for.
///
/// Blocking git, on a thread of its own: a fetch has no deadline to answer
/// within, and this one is a step in front of a session launch rather than a
/// page being drawn.
async fn landed(of: &Stage, branches: Vec<String>) -> HashSet<String> {
    let repo = of.repo.clone();
    let default = of.default_branch.clone();

    let read = tokio::task::spawn_blocking(move || {
        if let worktrees::Fetched::Failed(said) = worktrees::fetch(&repo) {
            tracing::error!(
                said,
                repo = %repo.display(),
                "fetching a Repo's remotes failed, so a joining stage's chain is being \
                 read off what this checkout holds",
            );
        }

        let default = worktrees::default_ref(&repo, &default);

        branches
            .into_iter()
            .filter(|branch| match worktrees::resolve(&repo, branch) {
                Some(tip) => worktrees::merged(&repo, &tip, &default) == Some(true),
                None => true,
            })
            .collect()
    })
    .await;

    read.unwrap_or_else(|error| {
        tracing::error!(
            error = ?error,
            "reading which of a chain's branches have merged failed",
        );

        HashSet::new()
    })
}

/// The chain itself, over values rather than over a database and a checkout —
/// for [`ahead`]'s reason: what this takes is three readings, so what a test
/// hands it is three.
///
/// **This stage's own branch is never in it.** A stage at its finish has no pull
/// request yet and so is not in the chain, but one whose ending stopped after
/// the push and was taken up again is — and a branch cannot be rebased onto
/// itself.
///
/// **The foot goes under everything**, where it is not already one of the links
/// above it. It is the branch the stage was cut from, so every commit of it is
/// in this branch already: whatever else the chain holds, this is below it.
///
/// One link per branch. A stage attempted twice is two Conversations answering
/// to one label and two branches in the chain, which is two links and right;
/// one branch named twice is one link.
fn below(
    conversation_id: i64,
    chain: &[store::Joined],
    stands_on: Option<&str>,
    landed: &HashSet<String>,
) -> Vec<skills::Link> {
    let mut links: Vec<skills::Link> = Vec::new();

    for link in chain {
        if link.conversation_id == conversation_id || landed.contains(&link.branch) {
            continue;
        }

        if links.iter().any(|had| had.branch == link.branch) {
            continue;
        }

        links.push(skills::Link {
            stage: Some(link.stage.clone()),
            branch: link.branch.clone(),
        });
    }

    if let Some(stands_on) = stands_on {
        if !landed.contains(stands_on) && !links.iter().any(|link| link.branch == stands_on) {
            links.insert(
                0,
                skills::Link {
                    stage: None,
                    branch: stands_on.to_owned(),
                },
            );
        }
    }

    links
}

/// Which stage of which roadmap this Conversation is, where it is a stage with
/// a label at all.
///
/// The record and nothing else, which is what says whether there is a chain to
/// join — see `store::stage_roadmap`. A row holding a roadmap and no label is
/// the Conversation that *wrote* the roadmap, or a stage from before the label
/// was written down; neither has a place in a chain this can read, and a name
/// guessed off a branch is the guess that record exists to stop.
async fn stage(state: &AppState, conversation_id: i64) -> Option<Stage> {
    let of = match store::stage_roadmap(&state.pool, conversation_id).await {
        Ok(of) => of?,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which stage a Conversation is failed");
            return None;
        }
    };

    let label = of.stage?;

    let repo = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.repo,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Repo of a stage failed");
            return None;
        }
    };

    Some(Stage {
        repo_id: repo.id,
        repo: repo.path,
        default_branch: repo.default_branch,
        roadmap: of.roadmap,
        label,
    })
}

/// The stage doing the waiting: which Repo's roadmap, which roadmap, and which
/// stage of it.
struct Stage {
    /// The registered Repo, which is what the record is asked about.
    repo_id: i64,

    /// And where it is on disk, which is what git is asked about: the chain a
    /// joining stage is told is read off the branches as well as off the
    /// record — see [`landed`].
    repo: PathBuf,

    /// The Repo's default branch, as the registry names it — what a branch
    /// having merged has merged *into*.
    default_branch: String,

    /// The roadmap's directory name under `docs/roadmaps/`.
    roadmap: String,

    /// And which stage of it, as the roadmap's own line labels it.
    label: String,
}

/// One stage this one is waiting on, and which of the two reasons it is waiting
/// on it for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ahead {
    /// Which stage of the roadmap, by the label the roadmap's own line gives it.
    stage: String,

    /// Whether it is in the chain already.
    ///
    /// A stage that has **joined** is waited on until it settles: it is a branch
    /// that may yet be pushed again. One that has not is a stage whose tasks
    /// finished before this one's and whose turn to join therefore comes first —
    /// and it is waited on until it has joined *and* settled, which is the first
    /// case arriving underneath the second.
    joined: bool,
}

/// What this stage is waiting on before it may join, in the order it is waiting
/// on them.
///
/// Three reads of the record and no reading of any branch. `store::stage_chain`
/// says which stages are in the chain and in what order, `store::join_queue`
/// says which of them finished their tasks before this one did, and
/// `store::stage_standings` says what became of each — one rule for *settled*,
/// shared with both of Verkstead's readings of a roadmap, rather than a second
/// one here that could come to disagree with them.
///
/// A record that cannot be read is not a record saying a stage is unsettled, so
/// a failed read is an empty answer and the stage goes on — see [`hold`], where
/// the reason is.
async fn ahead_of(state: &AppState, of: &Stage, conversation_id: i64) -> Vec<Ahead> {
    let chain = match store::stage_chain(&state.pool, of.repo_id, &of.roadmap).await {
        Ok(chain) => chain,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a roadmap's chain failed");
            return Vec::new();
        }
    };

    let queue = match store::join_queue(&state.pool, of.repo_id, &of.roadmap).await {
        Ok(queue) => queue,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a roadmap's queue to join failed");
            return Vec::new();
        }
    };

    let standings = match store::stage_standings(&state.pool, of.repo_id).await {
        Ok(standings) => standings,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a roadmap's stages got to failed");
            return Vec::new();
        }
    };

    ahead(conversation_id, of, &chain, &standings, &queue)
}

/// The reading itself, over values rather than over a database — for
/// `store::StageStandings::from_rows`'s reason: what this takes is three
/// readings of the record, so what a test hands it is three.
///
/// Two rules, and the second is what makes the joins single file:
///
/// - **A stage in the chain that has not settled** is a branch still moving, and
///   nothing rebases onto one.
/// - **A stage whose tasks finished before this one's and which has not joined**
///   has the next turn. It is let in first, and this one goes on waiting through
///   its join — by the rule above, which the join puts it under — until it
///   settles.
///
/// **This stage's own Conversation is never waited on**, in either rule. A stage
/// at its finish has no pull request yet and so is not in the chain, but one
/// whose ending stopped after the push and was taken up again is — and it is in
/// the queue from its first look at the backlog either way.
///
/// One label at most per stage, so a stage attempted twice is named once: what
/// is in the chain is a branch, and what the line says is which stage of the
/// roadmap is holding this one up. And never this stage's own label, where a
/// second Conversation answers to it: that is the abandoned attempt at this very
/// stage, and a stage cannot be what is holding itself up.
///
/// A stage ahead in the queue that **settled** or was **abandoned** is waited on
/// by nobody. Settled is nothing left to wait for, and abandoned is a stage that
/// will never join at all — a queue that went on waiting for one would be a
/// roadmap no later stage could ever finish.
fn ahead(
    conversation_id: i64,
    of: &Stage,
    chain: &[store::Joined],
    standings: &store::StageStandings,
    queue: &[store::Queued],
) -> Vec<Ahead> {
    let mut ahead: Vec<Ahead> = Vec::new();

    let mut put = |stage: &String, joined: bool| {
        if stage != &of.label && !ahead.iter().any(|held| &held.stage == stage) {
            ahead.push(Ahead {
                stage: stage.clone(),
                joined,
            });
        }
    };

    // The chain first, bottom to top, which is the order a stage would be
    // rebased through them.
    for link in chain {
        if link.conversation_id == conversation_id {
            continue;
        }

        if standings.of(&of.roadmap, &link.stage) == Some(store::StageStanding::Settled) {
            continue;
        }

        put(&link.stage, true);
    }

    // Then the queue, as far as this stage's own place in it: the stages whose
    // tasks finished first and which have not joined yet. A stage the record
    // holds no place for is a stage this cannot order itself against, and it
    // waits on the chain alone rather than on everybody.
    let Some(mine) = queue
        .iter()
        .position(|queued| queued.conversation_id == conversation_id)
    else {
        return ahead;
    };

    for queued in &queue[..mine] {
        if chain
            .iter()
            .any(|link| link.conversation_id == queued.conversation_id)
        {
            continue;
        }

        if standings.of(&of.roadmap, &queued.stage) != Some(store::StageStanding::InFlight) {
            continue;
        }

        put(&queued.stage, false);
    }

    ahead
}

/// Say on the Timeline that the stage is waiting to join, and which stage or
/// stages it is waiting on.
///
/// The one thing a held stage says. No device push: the chain below is being
/// got on with by whoever is on it, and there is nothing here for the human to
/// do — see this module's own documentation, where the condition is.
///
/// And it says *why* of each of them, because the two reasons read as different
/// waits to whoever is looking at a queue of stages: one is a branch below still
/// moving, and the other is a stage whose turn simply comes first.
async fn say(state: &AppState, conversation_id: i64, roadmap: &str, ahead: &[Ahead]) {
    let markdown = format!(
        "**Waiting to join.** Every task is done, and the finish is held until the `{roadmap}` \
         roadmap's chain has settled below this stage: {which}. Nothing rebases onto a branch \
         that is still moving, and this branch is rebased once, before it has a pull request \
         anybody has started reading — so the joins are the one thing a roadmap does in single \
         file, in the order the stages' tasks finished.",
        which = listed(&ahead.iter().map(why).collect::<Vec<_>>()),
    );

    match store::note(&state.pool, conversation_id, &markdown).await {
        Ok(true) => state.nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        }),
        Ok(false) => tracing::error!(
            conversation_id,
            "there is no Conversation left to say it is waiting to join",
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "saying that a stage was waiting to join failed")
        }
    }
}

/// One stage being waited on, and what it is being waited on for.
fn why(ahead: &Ahead) -> String {
    let stage = &ahead.stage;

    if ahead.joined {
        format!("stage {stage} has joined the chain and has not settled")
    } else {
        format!("stage {stage} finished its tasks before this one and has not joined yet")
    }
}

/// A few of them in a row, said the way a sentence says them.
fn listed(labels: &[String]) -> String {
    match labels.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stage these are asked about. Any number does; what is being tested
    /// is a register, and a register knows nothing else about a Conversation.
    const CONVERSATION: i64 = 7;

    /// The whole of what the register is for: while the hold is held the stage
    /// is waiting to join, and the moment it is let go it is not.
    #[test]
    fn a_stage_is_waiting_for_as_long_as_its_hold_is_held() {
        let joins = Joins::new();

        assert!(
            !joins.waiting(CONVERSATION),
            "nothing is held on a register nothing has been put on",
        );

        let waiting = joins.hold(CONVERSATION);

        assert!(joins.waiting(CONVERSATION));
        assert_eq!(joins.all_waiting(), HashSet::from([CONVERSATION]));

        drop(waiting);

        assert!(
            !joins.waiting(CONVERSATION),
            "and the hold going takes the label with it",
        );
        assert!(joins.all_waiting().is_empty());
    }

    /// And the reason it is a guard rather than a pair of calls: a run that
    /// died mid-wait would otherwise leave a Conversation drawn as waiting to
    /// join with nothing left that could ever take it off.
    #[tokio::test]
    async fn a_hold_whose_run_panicked_is_off_the_register_too() {
        let joins = Joins::new();

        let run = tokio::spawn({
            let waiting = joins.hold(CONVERSATION);

            async move {
                let _waiting = waiting;

                panic!("a run falling over while it waited to join");
            }
        });

        assert!(run.await.is_err(), "the task panicked");
        assert!(
            !joins.waiting(CONVERSATION),
            "a hold that died is not one still holding anything",
        );
    }

    /// Holds are kept per Conversation, so one stage waiting to join says
    /// nothing about another.
    #[test]
    fn one_stages_hold_holds_only_that_stage() {
        let joins = Joins::new();
        let waiting = joins.hold(CONVERSATION);

        assert!(joins.waiting(CONVERSATION));
        assert!(!joins.waiting(CONVERSATION + 1));

        drop(waiting);
    }

    /// One stage below reads as one, and several read as a list a sentence can
    /// carry.
    #[test]
    fn the_stages_being_waited_on_are_named_the_way_a_sentence_names_them() {
        assert_eq!(listed(&[]), "");
        assert_eq!(listed(&["02".to_owned()]), "02");
        assert_eq!(listed(&["02".to_owned(), "04".to_owned()]), "02 and 04");
        assert_eq!(
            listed(&["02".to_owned(), "03".to_owned(), "04".to_owned()]),
            "02, 03 and 04",
        );
    }

    /// The roadmap these are all stages of, and the Repo it is in. One of each,
    /// because what the rule is about is the order inside one roadmap.
    const ROADMAP: &str = "rate-limiting";
    const REPO: i64 = 1;

    /// The stage doing the waiting, by the label the roadmap gives it.
    ///
    /// The Repo's path and default branch stand for nothing here: what the rules
    /// below are about is three readings of the record, and the one reading that
    /// asks git is handed its answer — see [`below`].
    fn asking(label: &str) -> Stage {
        Stage {
            repo_id: REPO,
            repo: PathBuf::from("/home/tobi/src/verkstead"),
            default_branch: "main".to_owned(),
            roadmap: ROADMAP.to_owned(),
            label: label.to_owned(),
        }
    }

    /// One link of the chain: a stage that has opened a pull request.
    fn joined(conversation_id: i64, stage: &str) -> store::Joined {
        store::Joined {
            conversation_id,
            stage: stage.to_owned(),
            branch: format!("roadmaps/{ROADMAP}/{stage}"),
        }
    }

    /// And one place in the queue: a stage whose tasks have finished.
    fn queued(conversation_id: i64, stage: &str) -> store::Queued {
        store::Queued {
            conversation_id,
            stage: stage.to_owned(),
        }
    }

    /// What the record says became of each stage of the roadmap.
    ///
    /// None of them stopped, for the reason the carry-on's own helper says none
    /// did: a stop is nothing the hold in front of a finish asks about. And no row
    /// names a Conversation either — this reading is handed the ones it weighs, by
    /// the chain and the queue — so every one of them says `0`, which is nobody's.
    fn standings<'a>(
        rows: impl IntoIterator<Item = (&'a str, store::StageStanding)>,
    ) -> store::StageStandings {
        store::StageStandings::from_rows(
            rows.into_iter()
                .map(|(label, standing)| (ROADMAP, label, standing, false, 0)),
        )
    }

    /// Two stages whose boxes are all ticked at once are let in one at a time,
    /// and the one whose tasks finished first goes first.
    ///
    /// Which is the whole of the queue. The chain below them is settled, so the
    /// rule about a branch still moving has nothing to say and both would push —
    /// onto the same top, one of them over the other. The place each took as it
    /// arrived is what tells them apart.
    #[test]
    fn two_stages_at_their_finish_are_let_in_in_the_order_their_tasks_finished() {
        let chain = [joined(1, "01")];
        let queue = [queued(1, "01"), queued(2, "02"), queued(3, "03")];
        let standings = standings([
            ("01", store::StageStanding::Settled),
            ("02", store::StageStanding::InFlight),
            ("03", store::StageStanding::InFlight),
        ]);

        assert_eq!(
            ahead(2, &asking("02"), &chain, &standings, &queue),
            Vec::new(),
            "the first of the two waits on nobody: the chain below it has settled",
        );

        assert_eq!(
            ahead(3, &asking("03"), &chain, &standings, &queue),
            vec![Ahead {
                stage: "02".to_owned(),
                joined: false,
            }],
            "and the second waits on the first, which has not joined yet",
        );
    }

    /// The first of them **joining** does not release the second. Its settling
    /// does.
    ///
    /// A stage that has just joined is a branch still moving — its wrap-up is
    /// where a finding gets fixed and a check goes green — so the queue hands it
    /// straight over to the rule the chain is under, and what the second stage is
    /// waiting on changes from *has not joined* to *has not settled* without
    /// letting go.
    #[test]
    fn the_stage_in_front_joining_does_not_release_the_one_behind_it() {
        let queue = [queued(2, "02"), queued(3, "03")];
        let chain = [joined(2, "02")];

        assert_eq!(
            ahead(
                3,
                &asking("03"),
                &chain,
                &standings([
                    ("02", store::StageStanding::InFlight),
                    ("03", store::StageStanding::InFlight),
                ]),
                &queue,
            ),
            vec![Ahead {
                stage: "02".to_owned(),
                joined: true,
            }],
            "it has joined and is wrapping up, so it is still what is being waited on",
        );

        assert_eq!(
            ahead(
                3,
                &asking("03"),
                &chain,
                &standings([
                    ("02", store::StageStanding::Settled),
                    ("03", store::StageStanding::InFlight),
                ]),
                &queue,
            ),
            Vec::new(),
            "and its settling is what lets the next one in",
        );
    }

    /// A stage in front that will never join holds up nobody.
    ///
    /// Abandoned is a branch nothing is on any more, and settled without a pull
    /// request is a stage that got where it was going another way. Either would
    /// otherwise be a roadmap no later stage could ever finish — and the price
    /// the single file charges is a wrap-up that cannot finish, not a stage
    /// nobody is working.
    #[test]
    fn a_stage_in_front_that_will_never_join_is_waited_on_by_nobody() {
        let queue = [queued(2, "02"), queued(3, "03")];

        for standing in [
            store::StageStanding::Abandoned,
            store::StageStanding::Settled,
        ] {
            assert_eq!(
                ahead(
                    3,
                    &asking("03"),
                    &[],
                    &standings([("02", standing), ("03", store::StageStanding::InFlight)]),
                    &queue,
                ),
                Vec::new(),
                "a stage that is {standing:?} is nobody's turn to wait for",
            );
        }
    }

    /// A stage the queue holds no place for waits on the chain alone.
    ///
    /// Which is the record failing to answer rather than a case of the rule: a
    /// stage takes its place before it looks at anything, so a stage with none
    /// is one whose write did not land. There is nothing to order it against, and
    /// holding it for ever against a queue it is not in would be a stage nothing
    /// could move.
    #[test]
    fn a_stage_with_no_place_in_the_queue_waits_on_the_chain_alone() {
        assert_eq!(
            ahead(
                3,
                &asking("03"),
                &[joined(1, "01")],
                &standings([
                    ("01", store::StageStanding::InFlight),
                    ("02", store::StageStanding::InFlight),
                ]),
                &[queued(2, "02")],
            ),
            vec![Ahead {
                stage: "01".to_owned(),
                joined: true,
            }],
            "the chain below it, and not the stage ahead of it in a queue it is not in",
        );
    }

    /// And a stage never waits on itself, in the queue any more than in the
    /// chain.
    ///
    /// Two Conversations answer to one label where a stage was attempted twice,
    /// and the earlier one took a place in the queue before it was abandoned. A
    /// stage cannot be what is holding itself up, and its own label on the
    /// Timeline would read as one.
    #[test]
    fn a_stage_waits_on_neither_itself_nor_an_earlier_attempt_at_itself() {
        let queue = [queued(2, "03"), queued(3, "03")];
        let standings = standings([("03", store::StageStanding::InFlight)]);

        assert_eq!(
            ahead(3, &asking("03"), &[], &standings, &queue),
            Vec::new(),
            "the abandoned attempt at this very stage is not what it is waiting on",
        );

        assert_eq!(
            ahead(3, &asking("03"), &[joined(3, "03")], &standings, &queue),
            Vec::new(),
            "and neither is its own pull request, where its ending was taken up again",
        );
    }

    /// The two reasons read as two different waits, because to whoever is looking
    /// at a queue of stages they are: one is a branch below still moving, and the
    /// other is a stage whose turn simply comes first.
    #[test]
    fn a_line_says_which_of_the_two_reasons_each_stage_is_waited_on_for() {
        assert_eq!(
            why(&Ahead {
                stage: "02".to_owned(),
                joined: true,
            }),
            "stage 02 has joined the chain and has not settled",
        );
        assert_eq!(
            why(&Ahead {
                stage: "02".to_owned(),
                joined: false,
            }),
            "stage 02 finished its tasks before this one and has not joined yet",
        );
    }

    /// The branch of a stage of this roadmap, named the way [`joined`] names
    /// one: what the chain is a list of.
    fn branch(stage: &str) -> String {
        format!("roadmaps/{ROADMAP}/{stage}")
    }

    /// And the branches git says the default branch already holds.
    fn merged<'a>(branches: impl IntoIterator<Item = &'a str>) -> HashSet<String> {
        branches.into_iter().map(str::to_owned).collect()
    }

    /// The chain a joining stage is told, in the order it is rebased through:
    /// the branch it was cut from at the foot, and every stage that has joined
    /// above it.
    ///
    /// The foot is the roadmap's own branch here, which is where stage 01 of
    /// every roadmap comes off while the roadmap's pull request is unmerged —
    /// and it is no stage, so nothing above ever reads it back out of the chain.
    #[test]
    fn the_chain_runs_from_the_branch_the_stage_was_cut_from_up() {
        let chain = [joined(1, "01"), joined(2, "02")];

        assert_eq!(
            below(3, &chain, Some("rate-limiting"), &merged([])),
            vec![
                skills::Link {
                    stage: None,
                    branch: "rate-limiting".to_owned(),
                },
                skills::Link {
                    stage: Some("01".to_owned()),
                    branch: branch("01"),
                },
                skills::Link {
                    stage: Some("02".to_owned()),
                    branch: branch("02"),
                },
            ],
        );
    }

    /// A stage cut from what is still the top is told a chain ending exactly
    /// there, which is the rebase that moves nothing.
    ///
    /// Every stage of every roadmap run in order. The branch it stands on is
    /// already a link of the chain, so it is named once and as the stage it is
    /// rather than twice.
    #[test]
    fn a_stage_cut_from_the_top_is_told_a_chain_that_ends_at_it() {
        let chain = [joined(1, "01"), joined(2, "02")];

        assert_eq!(
            below(3, &chain, Some(&branch("02")), &merged([])),
            vec![
                skills::Link {
                    stage: Some("01".to_owned()),
                    branch: branch("01"),
                },
                skills::Link {
                    stage: Some("02".to_owned()),
                    branch: branch("02"),
                },
            ],
            "the foot is one of the links above it, so it is not named twice",
        );
    }

    /// A branch the default branch already holds is out of the chain, whether it
    /// is a stage of it or the branch this one was cut from.
    ///
    /// Which is what keeps a roadmap run in order finishing as it does today: a
    /// stage whose predecessors have all merged comes off the default branch,
    /// and a chain of merged branches would rebase it back behind the merge that
    /// carried them in.
    #[test]
    fn a_branch_the_default_branch_holds_is_out_of_the_chain() {
        let chain = [joined(1, "01"), joined(2, "02")];

        assert_eq!(
            below(3, &chain, None, &merged([branch("01").as_str()])),
            vec![skills::Link {
                stage: Some("02".to_owned()),
                branch: branch("02"),
            }],
            "the stage below is still there, and the one under it has merged",
        );

        assert_eq!(
            below(
                3,
                &chain,
                Some("rate-limiting"),
                &merged([
                    "rate-limiting",
                    branch("01").as_str(),
                    branch("02").as_str(),
                ])
            ),
            Vec::new(),
            "and a roadmap merged to the top is a stage told no chain at all",
        );
    }

    /// A stage is never in the chain it is joining.
    ///
    /// A stage at its finish has no pull request yet, so it is usually not in
    /// the chain at all — but one whose ending stopped after the push and was
    /// taken up again is, and a branch cannot be rebased onto itself.
    #[test]
    fn a_stage_is_never_told_to_rebase_onto_its_own_branch() {
        let chain = [joined(1, "01"), joined(2, "02")];

        assert_eq!(
            below(2, &chain, Some(&branch("01")), &merged([])),
            vec![skills::Link {
                stage: Some("01".to_owned()),
                branch: branch("01"),
            }],
        );
    }

    /// And a stage that stands on nothing with nothing in the chain is told
    /// nothing, which is the ordinary unstacked start.
    #[test]
    fn a_stage_standing_on_nothing_joins_no_chain() {
        assert_eq!(below(1, &[], None, &merged([])), Vec::new());
    }
}
