//! The look that starts what waited for a place on the server.
//!
//! A ready stage that fits under its roadmap's limit and not under the server's
//! is held rather than started — see [`crate::settings::AtOnce::conversations`],
//! which is how many places there are, and [`crate::continuing`], which spends
//! them. That hold is only half a limit on its own. **Nothing that frees a place
//! is a settle**: a Conversation reaches Done, a run stops, a grilling ends, the
//! human closes something — and the carry-on runs at settles alone. So a stage
//! held for a place would wait until something else of *its own* roadmap
//! settled, which may be nothing at all, and a server that had gone quiet with
//! work left in it would stay quiet.
//!
//! What closes that loop is this: a look of the server's own at the roadmaps
//! Verkstead is driving, made **when a place comes free** rather than on a clock.
//! Each of them is read exactly as a settle reads it, and what the free places
//! hold is started.
//!
//! **Nothing about the waiting is stored.** What is ready is read afresh from the
//! declarations, the record and the boxes at every look — see
//! [`crate::stages::ready`] — so a look after a restart finds exactly what the
//! last one did, and a restart loses nothing that was waiting. A second record
//! saying *this stage is waiting* could only come to disagree with a readiness
//! that is worked out afresh every time: a dependency settling under it, a box
//! ticked by hand, a stage added to the index.
//!
//! **A start with no settle behind it needs a predecessor.** The carry-on takes
//! a stage's Pairings, its companions, its base and the Timeline it says things
//! on from the Conversation that settled, and here nothing has. It takes them
//! from the **roadmap's chain** instead: the highest settled stage of it, and the
//! roadmap's own Conversation where no stage of it has settled yet. Which is the
//! foot of the chain either way, and already what the base is chosen off — see
//! the `continuing` module's own `cut_from`, which walks the chain for the same
//! link. The notice that a stage started goes on that Conversation's Timeline.
//!
//! **Which is the reading's own limit, inherited whole.** A foot with no Worktree
//! left — one the human archived and the cleanup has since let go of — is one
//! [`crate::continuing::reading`] stops at, exactly as it stops at a settle on
//! the same Conversation. Such a roadmap is the press's from then on, and it was
//! before this too.
//!
//! **And the look is silent unless it starts something.** It runs for as long as
//! the server is up and over every roadmap there is, so a sentence said about a
//! roadmap with nothing ready is a sentence said again at the next place to
//! change hands and for ever. Each of those is said once, by the settle that
//! found it — see [`crate::continuing::Brought`], which is where the two readings
//! part company and where the one exception is checked.
//!
//! **When it looks.** When a place comes free, and every [`crate::Pace::places`]
//! behind that whether one has or not.
//!
//! The wake-up is [`crate::drivers::Drivers::letting_go`]: the last driver of a
//! Conversation going out of scope, which is what a run ending, a wrap-up
//! finishing and a Conversation being closed all come to. A look costs a reading
//! of every roadmap the server has ever driven — several database reads and a
//! handful of git reads apiece — and that bill grows for ever with the number of
//! roadmaps a machine has run, so a server where nothing has happened for days
//! should pay it none of the time rather than twice a minute. Which is what
//! waiting on the thing itself buys: no places change hands, no looks.
//!
//! The slow one behind it is what makes every case eventual rather than only the
//! common one. A place can come free without a driver letting go of anything — a
//! grilling nobody has picked on yet is a session with no driver — and the limit
//! itself can move under a look, the human raising it on the settings page while
//! the machine is full. Neither wakes anything, and both are waited out.
//!
//! And never before the startup resume is done: both registers the places are
//! counted off are facts about this process, so a server that has just come back
//! holds no places at all until the resume takes its runs back up. A look that got
//! there first would read four free places on a server running four Conversations
//! and start over the top of them. Which is the reading [`crate::stalls::sweeping`]
//! waits for too, and for the mirror image of the reason.
//!
//! And never on a server that runs no sessions, for the reason the stall sweep
//! never runs there: nothing it started could run.

use std::time::Duration;

use tokio::sync::watch;

use crate::AppState;
use crate::continuing::{self, Brought};
use crate::store;

/// How long the roadmaps being driven go unlooked-at when **nothing frees a
/// place**, as [`crate::Pace`] has it by default.
///
/// The backstop rather than the pace: what ordinarily brings a look about is a
/// place coming free — see [`crate::drivers::Drivers::letting_go`] — and this is
/// the wait behind that, for the ways a place can come free without waking
/// anything. Half a minute, which is what the pace was when it was the only
/// thing: short enough that the cases the wake-up misses are still measured in
/// seconds, and no longer a cost a quiet server pays, because a quiet server is
/// one where every one of these expires having found the places exactly as it
/// left them.
pub(crate) const LOOKED_AT_EVERY: Duration = Duration::from_secs(30);

/// Look for a free place from now until the process stops: once as soon as
/// `resumed` says the startup resume is done, and after that whenever a place
/// comes free or [`crate::Pace::places`] goes by without one.
///
/// `resumed` is the signal [`crate::resume::at_startup`] is awaited through —
/// waited for rather than raced, because a look that counted the places before
/// the resume had taken its runs back up would count none of them. See
/// [`crate::resume::taken_up`], which is the one waiter both sweeps hear.
pub(crate) fn looking(state: &AppState, mut resumed: watch::Receiver<bool>) {
    // Nothing for this to do on a server that runs no sessions — see
    // [`crate::sessions::Sessions::runs_sessions`] and
    // [`crate::stalls::sweeping`], which stands down for the same reason. Only
    // the tests' routers are ever built that way, and there a look would be a
    // stage started into a server with nothing to run it.
    if !state.sessions.runs_sessions() {
        return;
    }

    let state = state.clone();

    tokio::spawn(async move {
        if resumed.wait_for(|done| *done).await.is_err() {
            tracing::error!(
                "the signal saying what was left running had been taken up again is gone, so \
                 the look for a free place stands down",
            );

            return;
        }

        loop {
            look(&state).await;

            // Whichever comes first, and neither of them is a settle. A look
            // woken by the first is a place that has genuinely just changed
            // hands; one woken by the second is the backstop under everything
            // that frees a place without saying so — see this module's own
            // documentation, where the pair is set out.
            tokio::select! {
                () = state.drivers.letting_go() => {}
                () = tokio::time::sleep(state.sessions.pace().places) => {}
            }
        }
    });
}

/// One look: start what the free places hold, across every roadmap being driven.
///
/// **Nothing at all while the places are full**, which is the ordinary answer on
/// a busy server and costs a file read and two register reads to reach. Every
/// roadmap below it would be read only to be told the same thing.
///
/// Which is *not* on its own what makes a stage held for a place worth saying out
/// loud inside a look. A reading spends a place on every stage it puts up to
/// start, and a start can still refuse after that — so the places being full when
/// a look began is no promise that a roadmap it passes over is genuinely waiting
/// on the machine. What decides that is a count taken after the starts, once per
/// reading: see [`crate::continuing::Brought::held`].
///
/// **Oldest roadmap first**, which is the order the free places are handed out
/// in. A roadmap's age is the age of the Conversation that wrote it rather than
/// how long any stage of it has been waiting — see
/// [`store::driven_roadmaps`], which is where the order comes from and why
/// nothing about the waiting is stored for it. Within a roadmap the order is the
/// roadmap's own: [`crate::stages::ready`] answers lowest-numbered first, and a
/// place is spent by a stage that starts and by nothing else.
///
/// **A roadmap already at its own limit is passed over**, however long it has
/// been waiting and however many places the server has free. The two limits are
/// both in force and the roadmap's is the stricter one there, so nothing of it
/// starts — and because a place is spent by a stage that starts, the place it
/// could not use is still free for the next roadmap below it rather than
/// standing empty. Which is the whole of why nothing is carried between the
/// readings here: each of them counts the places itself, so what an earlier
/// roadmap took is gone and what it could not take is not.
///
/// Nothing is refused for and nothing is returned. This runs unattended with
/// nobody watching, and what it has to say it says on a Timeline when it starts
/// something and in the log when it does not.
async fn look(state: &AppState) {
    // How many places there are, off `config.yaml` as it stands this moment — four
    // where nobody has said. Read afresh at every look rather than held from
    // startup, the way the carry-on reads it at every settle: a number changed on
    // the settings page is in force at the next look, and a place that a lowered
    // limit has taken away is one nothing already running is stopped over.
    let places = state.settings.config().at_once().conversations();
    let taking = state.drivers.taking(&state.sessions.working());

    if taking.len() >= places {
        tracing::debug!(
            taking = taking.len(),
            places,
            "every place on the server is taken, so there is nothing for a look to start",
        );

        return;
    }

    let driving = match store::driven_roadmaps(&state.pool).await {
        Ok(driving) => driving,
        Err(error) => {
            tracing::error!(error = ?error, "listing the roadmaps to look for a waiting stage among failed");
            return;
        }
    };

    // Oldest roadmap first, which is the order the free places go out in — the
    // read comes back that way, and what walks it in that order is this loop.
    for driven in driving {
        // The foot of this roadmap's chain, which is what a start with no settle
        // behind it inherits from. A roadmap with none is one whose own
        // Conversation has gone: there is nothing to take Pairings from and
        // nothing to say anything on, so it is left to the press.
        let Some(foot) = foot(state, &driven).await else {
            continue;
        };

        // The same reading a settle makes, and it counts the places itself: what
        // was free when this look began is not what is free after the roadmap
        // before this one took some, so nothing is carried between them.
        continuing::reading(state.clone(), foot, Brought::Look).await;
    }
}

/// The **foot** of one roadmap's chain: the Conversation a start with no settle
/// behind it takes its Pairings, its companions, its base and its Timeline from.
///
/// The highest **settled** stage of the chain, and the roadmap's own Conversation
/// where no stage of it has settled yet. Which is the same walk the `continuing`
/// module's own `cut_from` makes to choose a base, asked one field along: the
/// branch it picks is this Conversation's branch, so a stage started off a look
/// is cut from exactly where a stage started off that Conversation's own settle
/// would have been.
///
/// **The highest rather than the newest.** The chain is in the order its stages
/// joined, and what is wanted is the last of them the record calls settled — a
/// link still wrapping up is a branch that may yet be pushed again, and a stage
/// cut from one would be cut from a commit that is about to stop existing. Which
/// is that same `cut_from`'s rule, asked here of the same chain.
///
/// `None` where the roadmap has no settled stage *and* no Conversation of its own
/// on the record — a roadmap adopted stage by stage from before any of this was
/// written down, or one whose planning Conversation has been unregistered out
/// from under it. Nothing is started for one: there is nowhere to take a Pairing
/// from, and *Continue a roadmap* is the press that gives it one.
async fn foot(state: &AppState, driven: &store::Driven) -> Option<i64> {
    let record = match store::stage_standings(&state.pool, driven.repo_id).await {
        Ok(record) => record,
        Err(error) => {
            tracing::error!(error = ?error, roadmap = driven.roadmap, "reading what a roadmap's stages had got to failed");
            return None;
        }
    };

    let chain = match store::stage_chain(&state.pool, driven.repo_id, &driven.roadmap).await {
        Ok(chain) => chain,
        Err(error) => {
            tracing::error!(error = ?error, roadmap = driven.roadmap, "reading a roadmap's chain failed");
            return None;
        }
    };

    let settled = chain
        .iter()
        .rev()
        .find(|link| record.of(&driven.roadmap, &link.stage) == Some(store::StageStanding::Settled))
        .map(|link| link.conversation_id);

    if let Some(settled) = settled {
        return Some(settled);
    }

    match store::roadmap_planner(&state.pool, driven.repo_id, &driven.roadmap).await {
        Ok(planned) => planned.map(|planned| planned.conversation_id),
        Err(error) => {
            tracing::error!(error = ?error, roadmap = driven.roadmap, "reading the Conversation a roadmap was written in failed");
            None
        }
    }
}
