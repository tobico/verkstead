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
//! says — and a Response to a Blocking Ask types nothing either, because the
//! wait the CLI is holding open is what delivers it. And a session that has gone
//! is the folding rule's case rather than a nudge that failed: its Answers go
//! into the next session's prompt of that Conversation, exactly as a Deferred
//! Ask's do — see [`crate::deferrals`], which is untouched by any of this.
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

use tokio::sync::broadcast::error::RecvError;
use verkstead_store::SettledSet;

use crate::AppState;
use crate::store;

/// What is typed in, for the Set that has just been answered.
///
/// Written to the agent as the human would write it, because that is what it is:
/// a line arriving at the session's own terminal, indistinguishable from one
/// somebody watching had typed.
///
/// **It names the Set and the command.** An agent reading this has ended its
/// turn and may have stored more than one Set, so a line that only said *your
/// answers are in* would send it back to the Guide to find out which and how.
/// The id it was stored under is the one thing it needs and the one thing only
/// Verkstead has.
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
/// it to.
///
/// Read off the record rather than off the announcement, which carries the Set
/// and where it was asked from and nothing else: whether anybody is idling on it
/// is what [`store::asked_as`] says, and whether the human answered it or closed
/// it unanswered is what [`store::settlement`] says. A Set locked unanswered has
/// no Answers to fetch, so nothing is typed and the session is left to the quiet
/// grace, which now sees nothing open on it.
async fn about(state: &AppState, settled: SettledSet) {
    let set_id = settled.set_id;

    // Every Set is asked from a Conversation, in one transaction with the Event
    // that puts it on that Conversation's Timeline — so this is a record that
    // has been got at rather than something a Set can be. Said and left: the
    // viewer's own listener says the same thing about the same settlement.
    let Some(conversation_id) = settled.conversation_id else {
        return;
    };

    match store::asked_as(&state.pool, set_id).await {
        Ok(store::Ask::StoreAndNudge) => {}
        // A Blocking Ask's wait is what delivers it, and nobody is idling on a
        // Deferred one at all.
        Ok(_) => return,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, set_id, "reading how a settled Question Set was asked failed");
            return;
        }
    }

    match store::settlement(&state.pool, set_id).await {
        Ok(Some(store::Settlement::Answered(_))) => {}
        Ok(_) => return,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, set_id, "reading what became of a settled Question Set failed");
            return;
        }
    }

    // Whatever session is running for the Conversation the Set was asked from,
    // which is the session that stored it in every case that matters: one idling
    // on a stored ask is not ended on quiet and not rescued, so it is there until
    // it goes of its own accord. Where it has gone and something has started
    // another — a run picked up again after a session died — the line reaches
    // that one, which is the right end of the same choice: its prompt was built
    // before there was an Answer to fold into it, so being told is the only way
    // these Answers reach anybody before the session after it.
    let Some(event_id) = state.sessions.writing(conversation_id) else {
        tracing::info!(
            conversation_id,
            set_id,
            "the session that stored the Question Set has gone, so its Answers go into the \
             next session's prompt rather than into a terminal",
        );
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
