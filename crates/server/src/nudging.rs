//! The nudge: telling a session idling on a stored ask that its Answers are
//! there to fetch.
//!
//! The far end of a **Store-and-nudge Ask**. The session asked, the Set was
//! stored, `verkstead ask` came back at once and the turn ended there — so when
//! the human answers there is nothing on the wire to hand the Response to and
//! nothing on that end listening for it. What there is is a terminal, and
//! Verkstead types one line into it saying the Answers are there and what
//! fetches them. See [`crate::typing`], which is the channel the rescue already
//! uses, and [`LINE`], which is what is typed.
//!
//! **Only where there is a session to nudge, and only for a Set it is idling
//! on.** A Response to a Deferred Ask types nothing whatever backend it was
//! asked on — nobody is idling on one, which is the whole of what `--deferred`
//! says. And a *stored* ask whose session has gone is the folding rule's case
//! rather than a nudge that failed: its Answers go into the next session's
//! prompt of that Conversation, exactly as a Deferred Ask's do — see
//! [`crate::deferrals`], which is untouched by any of this.
//!
//! A blocking ask has no such second chance, and this is where that shows: it
//! has no folding record, because it never needed one — the wait delivered. So
//! a killed wait *and* a session already gone is the one arrangement where the
//! human's Answers reach nobody at all, and all this can do about it is say so
//! in the log rather than claim a prompt will carry them.
//!
//! **And the same line rescues a Blocking Ask whose wait has gone** — see
//! [`after_the_wait`]. A blocking ask is delivered by the wait the CLI is
//! holding open, so ordinarily there is nothing to type and nothing to type it
//! at. But the wait is a shell command a harness is running in the background,
//! and a harness may stop one: what that leaves is a Set answered, a session
//! alive, and nothing between the two — a Conversation nobody can move, which
//! is the very thing the rescue exists to prevent and the one shape of it the
//! rescue cannot see, because there *is* something open on the Conversation.
//! So the delivery is waited out and then read, and a Response that reached
//! nobody is said out loud in the one channel that reaches a session.
//!
//! Which is a fact read rather than a presence guessed at. Whether a wait is
//! held right now says nothing here — a CLI that took its Response and exited
//! holds none either, and the two endings must not be confused — so what is
//! read is [`verkstead_store::delivered`], which is written where the Response
//! is handed over and nowhere else.
//!
//! **One place, however the Response arrived.** The human answers from the
//! viewer and an agent could answer over the agent API; both store it on the one
//! path and both announce it on the one channel, so this hangs off that
//! announcement rather than off either caller — see [`listening`]. A nudge sent
//! from one namespace and silently not from the other would be a session waiting
//! for a line nobody was going to type.
//!
//! **Nothing goes on the Timeline for it.** It is Verkstead speaking to an agent
//! rather than anything the work has got to, and the line is in the session's own
//! Capture — the same account the rescue gives of itself.
//!
//! Not to be read as the viewer's [`Nudge`](verkstead_schema::Nudge), which is
//! the data-free signal telling an open page the world moved. This one is a
//! sentence typed at an agent.

use std::time::Duration;

use tokio::sync::broadcast::error::RecvError;
use verkstead_store::SettledSet;

use crate::AppState;
use crate::store;

/// How long a Blocking Ask's own wait is given to come back for the Response
/// before the nudge is typed instead — see [`after_the_wait`].
///
/// **Long enough that a wait which is merely struggling is not spoken over.**
/// The CLI reopens the moment a hold closes, and where something transient is in
/// the way it backs off to a ceiling of ten seconds — so a wait that is alive at
/// all collects the Response within a few seconds of its landing, and a minute
/// is half a dozen of its retries. What a window this size costs, in the case it
/// is for, is a minute of a session sitting on Answers it has not been told
/// about; what a shorter one would cost is a line typed at a session that was
/// about to be handed them anyway.
///
/// And nothing worse than that if it is spent wrongly: the line says to fetch
/// Answers, and fetching Answers that have already arrived hands back the same
/// Response again.
///
/// **With a ceiling it has to stay under, which is the rescue's.** Answering the
/// Set leaves nothing open on the Conversation, so the session becomes the
/// rescue's business at that moment — and what the rescue would say is that a
/// Set is the whole of how the human is spoken to, which is an agent's cue to
/// ask one. A second copy of the Questions is the very thing being prevented
/// here, so this has to reach the session first. The rescue gives a stirred
/// session [`crate::runner::Pace::waking`] to say its first word, which is five
/// minutes; a minute is comfortably inside it, and the line typed here is itself
/// the stir that puts the rescue back to waiting.
const RECONNECT: Duration = Duration::from_secs(60);

/// What is typed in, for the Set that has just been answered.
///
/// Written to the agent as the human would write it, because that is what it is:
/// a line arriving at the session's own terminal, indistinguishable from one
/// somebody watching had typed.
///
/// **It names the Set and the command.** An agent reading this has no wait in
/// front of it and may have asked more than one Set, so a line that only said
/// *your answers are in* would send it back to the Guide to find out which and
/// how. The id it was asked under is the one thing it needs and the one thing
/// only Verkstead has.
pub(crate) fn line(set_id: i64) -> String {
    format!(
        "I've answered Question Set {set_id}. Fetch my answers with \
         `verkstead answers {set_id}`, and carry on with the work from there."
    )
}

/// Nudge about every Set settled from now until the process stops.
///
/// Subscribed to the store's settlement channel, which is the one moment a Set
/// is settled however it was: the browser's submit and the agent API's both
/// reach it, and both reach it after the Response is stored.
///
/// Nothing at all on a server that runs no sessions — there is no terminal to
/// type into on one, now or ever, which is the same reading [`crate::stalls`]
/// takes of the same register.
pub(crate) fn listening(state: &AppState) {
    if !state.sessions.runs_sessions() {
        return;
    }

    let mut settlements = state.settlements.subscribe();
    let state = state.clone();

    tokio::spawn(async move {
        loop {
            match settlements.recv().await {
                Ok(settled) => about(&state, settled).await,
                // A burst bigger than the channel holds, which is a settlement
                // this may have been the only reader of. What it costs is a
                // session left idling on Answers nobody told it about, and what
                // catches that is the same thing that catches a session idling
                // on anything else: the Set is settled now, so the quiet grace
                // and the rescue see nothing open and take it in hand.
                Err(RecvError::Lagged(missed)) => {
                    tracing::error!(
                        missed,
                        "settlements were announced faster than the nudge could read them, so a \
                         session idling on one of them was not told its Answers had landed",
                    );
                }
                // The server itself is going, which is the only way this ends.
                Err(RecvError::Closed) => return,
            }
        }
    });
}

/// Say the Answers are there, where there is a session idling on this Set to say
/// it to — now, or once a Blocking Ask's wait has had its chance.
///
/// Read off the record rather than off the announcement, which carries the Set
/// and where it was asked from and nothing else: how it was asked is what
/// [`store::asked_as`] says, and whether the human answered it or closed it
/// unanswered is what [`store::settlement`] says. A Set locked unanswered has
/// no Answers to fetch, so nothing is typed and the session is left to the quiet
/// grace, which now sees nothing open on it.
///
/// Which of the three kinds it is decides only *when* — see [`after_the_wait`]
/// for the one that waits, and [`tell`] for the typing the two of them share.
async fn about(state: &AppState, settled: SettledSet) {
    let set_id = settled.set_id;

    // Every Set is asked from a Conversation, in one transaction with the Event
    // that puts it on that Conversation's Timeline — so this is a record that
    // has been got at rather than something a Set can be. Said and left: the
    // viewer's own listener says the same thing about the same settlement.
    let Some(conversation_id) = settled.conversation_id else {
        return;
    };

    let ask = match store::asked_as(&state.pool, set_id).await {
        Ok(ask) => ask,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, set_id, "reading how a settled Question Set was asked failed");
            return;
        }
    };

    match store::settlement(&state.pool, set_id).await {
        Ok(Some(store::Settlement::Answered(_))) => {}
        Ok(_) => return,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, set_id, "reading what became of a settled Question Set failed");
            return;
        }
    }

    match ask {
        store::Ask::StoreAndNudge => tell(state, conversation_id, set_id, ask).await,
        // Whose wait is what delivers it — so this stands back and looks again
        // once the wait has had its chance.
        store::Ask::Blocking => after_the_wait(state, conversation_id, set_id),
        // Nobody is idling on a Deferred Ask at all.
        store::Ask::Deferred => {}
    }
}

/// Give a Blocking Ask's own wait its chance at the Response, and say the
/// Answers are there where it never took them.
///
/// **A task of its own, because it waits.** The settlements are read one at a
/// time, and a minute spent here is a minute no other Set is nudged about — so
/// this returns at once and the looking-again happens beside it. Nothing is lost
/// if the server goes in between: a CLI that is still there reconnects on its
/// own and is handed the Response the ordinary way, which is the same ending
/// this was watching for.
///
/// **What it reads is the delivery and not the liveness.** A Set with no wait
/// held on it is a Set whose agent took its Response and exited just as much as
/// one whose agent was killed, and the badge that tells a human those apart —
/// see [`verkstead_store::Waits`] — cannot, which is why it says *display only*
/// and stays that way. [`verkstead_store::delivered`] is the fact itself, and
/// is written where the handing over happens.
fn after_the_wait(state: &AppState, conversation_id: i64, set_id: i64) {
    let state = state.clone();

    tokio::spawn(async move {
        tokio::time::sleep(RECONNECT).await;

        match store::delivered(&state.pool, set_id).await {
            // The wait came back for it, which is what a Blocking Ask is and
            // what all but a handful of them do.
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, set_id, "reading whether a Question Set's Response was delivered failed");
                return;
            }
        }

        tracing::info!(
            conversation_id,
            set_id,
            "the wait on the Question Set never took the Response, so the session is being told \
             its Answers are there to fetch",
        );

        tell(&state, conversation_id, set_id, store::Ask::Blocking).await;
    });
}

/// Type the line, at whatever session the Conversation has.
///
/// Shared by the two callers, because what is typed and where it goes are the
/// same question however the session came to be waiting on a line: one that
/// ended its turn on a stored ask, and one whose wait was killed under it.
async fn tell(state: &AppState, conversation_id: i64, set_id: i64, ask: store::Ask) {
    // Whatever session is running for the Conversation the Set was asked from,
    // which is the session that asked it in every case that matters: one idling
    // on a stored ask is not ended on quiet and not rescued, so it is there until
    // it goes of its own accord, and one whose wait was killed is still inside
    // the window the rescue gives a stirred session to speak — see [`RECONNECT`].
    // Where it has gone and something has started another — a run picked up again
    // after a session died — the line reaches that one, which is the right end of
    // the same choice: its prompt was built before there was an Answer to fold
    // into it, so being told is the only way these Answers reach anybody before
    // the session after it.
    let Some(event_id) = state.sessions.writing(conversation_id) else {
        // Said two ways, because two things are true. A stored ask that reaches
        // nobody is the folding rule's case and loses nothing. A blocking ask
        // has no folding record at all — it never needed one, because its wait
        // delivered — so a killed wait *and* a gone session is the one way the
        // human's Answers reach nobody at all. Nothing here can mend that; what
        // it can do is not claim otherwise.
        match ask {
            store::Ask::Blocking => tracing::warn!(
                conversation_id,
                set_id,
                "the wait on the Question Set never took the Response and the session that \
                 asked it has gone, so the human's Answers reached nobody: a blocking ask has \
                 no folding record to carry them into the next session's prompt",
            ),
            _ => tracing::info!(
                conversation_id,
                set_id,
                "the session that asked the Question Set has gone, so its Answers go into the \
                 next session's prompt rather than into a terminal",
            ),
        }
        return;
    };

    if crate::typing::typed(state, conversation_id, event_id, &line(set_id)).await {
        tracing::info!(
            conversation_id,
            event_id,
            set_id,
            "the session idling on the Question Set was told its Answers are there to fetch",
        );
    } else {
        tracing::info!(
            conversation_id,
            event_id,
            set_id,
            "the session idling on the Question Set had already ended, so nothing was typed \
             into it and its Answers go into the next session's prompt",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The line names the Set, and it names what fetches it. Both, because an
    /// agent that has ended its turn has neither in front of it.
    #[test]
    fn the_line_names_the_set_and_the_command_that_fetches_it() {
        let line = line(42);

        assert!(
            line.contains("Question Set 42"),
            "the Set it is about is named: {line:?}",
        );
        assert!(
            line.contains("verkstead answers 42"),
            "and what fetches it, with the id it was stored under: {line:?}",
        );
        assert!(
            !line.contains('\n'),
            "and it is one line: the Enter is the typing's, and a line broken \
             over two would be submitted half-written: {line:?}",
        );
    }
}
