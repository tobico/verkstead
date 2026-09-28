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
use std::sync::{Arc, Mutex};
use std::time::Duration;

use verkstead_schema::Nudge;

use crate::AppState;
use crate::store;

/// How often a held stage looks at whether the chain below it has settled.
///
/// Two reads of the database and nothing else, so this is not a cost: what it
/// decides is how long after the stage below settles the one behind it gets
/// going, and ten seconds is shorter than the launch that follows it.
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

    // The hold itself, taken the first time there is something to wait for and
    // let go as this returns — so a roadmap run in order is never marked at all.
    let mut waiting: Option<Waiting> = None;

    // And what was last said out loud, so that the line is written once per
    // answer rather than once per look: a second stage joining the chain below
    // while this one waits changes what it is waiting on, and that is worth a
    // line of its own.
    let mut said: Vec<String> = Vec::new();

    loop {
        let unsettled = unsettled(state, &of, conversation_id).await;

        if unsettled.is_empty() {
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

        if unsettled != said {
            tracing::info!(
                conversation_id,
                roadmap = of.roadmap,
                waiting_on = ?unsettled,
                "every task is done and the chain below has not settled, so the finish is held",
            );

            say(state, conversation_id, &of.roadmap, &unsettled).await;
            said = unsettled;
        }

        tokio::time::sleep(state.sessions.pace().joins).await;
    }
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

    let repo_id = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.repo.id,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Repo of a stage failed");
            return None;
        }
    };

    Some(Stage {
        repo_id,
        roadmap: of.roadmap,
        label,
    })
}

/// The stage doing the waiting: which Repo's roadmap, which roadmap, and which
/// stage of it.
struct Stage {
    repo_id: i64,
    roadmap: String,
    label: String,
}

/// The stages already in this roadmap's chain that have not settled, bottom to
/// top, by the labels the roadmap gives them.
///
/// Two reads of the record and no reading of any branch: `store::stage_chain`
/// says which stages are in the chain and in what order, and
/// `store::stage_standings` says what became of each — one rule for *settled*,
/// shared with both of Verkstead's readings of a roadmap, rather than a second
/// one here that could come to disagree with them.
///
/// **This stage's own Conversation is never waited on.** A stage at its finish
/// has no pull request yet and so is not in the chain, but one whose ending
/// stopped after the push and was taken up again is — and a stage waiting for
/// itself to settle would never finish.
///
/// One label at most per stage, so a stage attempted twice is named once: what
/// is in the chain is a branch, and what the line says is which stage of the
/// roadmap is holding this one up.
async fn unsettled(state: &AppState, of: &Stage, conversation_id: i64) -> Vec<String> {
    let chain = match store::stage_chain(&state.pool, of.repo_id, &of.roadmap).await {
        Ok(chain) => chain,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a roadmap's chain failed");
            return Vec::new();
        }
    };

    if chain.is_empty() {
        return Vec::new();
    }

    let standings = match store::stage_standings(&state.pool, of.repo_id).await {
        Ok(standings) => standings,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a roadmap's stages got to failed");
            return Vec::new();
        }
    };

    let mut unsettled: Vec<String> = Vec::new();

    for link in chain {
        if link.conversation_id == conversation_id {
            continue;
        }

        if standings.of(&of.roadmap, &link.stage) == Some(store::StageStanding::Settled) {
            continue;
        }

        if !unsettled.contains(&link.stage) {
            unsettled.push(link.stage);
        }
    }

    // And the stage's own label where two Conversations answer to it: the other
    // one is the abandoned attempt at this very stage, and a stage cannot be
    // what is holding itself up.
    unsettled.retain(|label| label != &of.label);

    unsettled
}

/// Say on the Timeline that the stage is waiting to join, and which stage or
/// stages it is waiting on.
///
/// The one thing a held stage says. No device push: the chain below is being
/// got on with by whoever is on it, and there is nothing here for the human to
/// do — see this module's own documentation, where the condition is.
async fn say(state: &AppState, conversation_id: i64, roadmap: &str, unsettled: &[String]) {
    let (which, have) = match unsettled {
        [one] => (format!("stage {one}"), "has"),
        many => (format!("stages {}", listed(many)), "have"),
    };

    let markdown = format!(
        "**Waiting to join.** Every task is done, and {which} of the `{roadmap}` roadmap {have} \
         joined the chain below this one and {have} not settled — so the finish is held until \
         {have_they}. Nothing rebases onto a branch that is still moving, and this branch is \
         rebased once, before it has a pull request anybody has started reading.",
        have_they = match unsettled {
            [_] => "it does",
            _ => "they do",
        },
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

/// A few labels in a row, said the way a sentence says them.
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
}
