//! Moving a Conversation onto another device of the cluster: the press, the
//! wait for the turn to end, and the move itself (ADR-0020, *Transfer*).
//!
//! **The press records a request rather than making a move.** It is
//! `verkstead done`'s pattern and Stop's: the session running for this
//! Conversation is part way through a turn, and a move that cut across it would
//! leave the work wherever the agent had got to. So what the press writes down
//! is *this Conversation is going to that device* — see [`store::ask_to_transfer`]
//! — and from then until it lands the Timeline reads **Transferring to** that
//! machine.
//!
//! **What sees the session out is what Stop's press leaves behind.** Whatever is
//! running runs to its own end, nothing new is started — that half is
//! [`crate::stopping::stopped`], which reads a Conversation on its way somewhere
//! as one nothing may be launched in — and the move runs once the session is
//! gone. Pressed with nothing running there is nothing to see out, and the move
//! runs where it stands.
//!
//! **And it runs after the session's ending has finished happening.** A session
//! away from home writes its account's login and this Repo's memory entries back
//! to the device they are at home on as it ends, and the word that it is over is
//! the last thing that ending sends — so a mover waiting on that word is waiting
//! on the write-back too, which is the order a transfer needs and gets for
//! nothing. See [`crate::sessions::Session::ended`].
//!
//! **The far end's answer is the commit point.** The move asks that device to
//! take the Conversation, it writes a row of its own and answers with the id it
//! gave it, and only then does this device mark its own copy transferred. What
//! that buys is a failure that is safe in both directions: a move that falls
//! over before the answer leaves the Conversation here live and **stopped**, with
//! a Notice naming what failed, and anything that did reach the far end is swept
//! rather than left as a half-record somebody has to find. The Worktree here is
//! left exactly where it is either way — this stage moves the record and the
//! next ones move the work.
//!
//! **What crosses in this stage is the Conversation itself**: the Repo the
//! matching settled, the branch, the lifecycle, the Pairings as ids of the far
//! end's own, the **Rank** verbatim and the **birth key**. No Timeline, no branch
//! and no Worktree — see the tasks after this one.

use verkstead_render::{
    Arrived, BirthKey, ConversationAcross, PairingAcross, PickedAcross, ProfileAcross, Transferring,
};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::peer::transfers::TRANSFERS;
use crate::relaying::{self, Call, Refusal, Streamed, as_json};
use crate::store::{self, Lifecycle};

/// The most the far end's answer may be: **one kilobyte**.
///
/// What comes back is one id. The bound is the one every read across a link has,
/// and for the reason they all have one — a machine that answers and then writes
/// without stopping. See [`crate::preflight::MOST_THE_HARNESSES_ARE`], the same
/// bound one reading along.
const MOST_AN_ANSWER_IS: usize = 1024;

/// Press *Transfer to…*: write down that this Conversation is going to `device`.
///
/// **Everything the dialog decided is decided again here**, which is the order
/// every press in this codebase is made in: the page was drawn a moment ago and
/// the record is what decides. So the state is asked again, and so is the
/// preflight — a machine that has gone to sleep between the drawing and the
/// press is the ordinary way for this one to be refused.
///
/// `Err` is this device's own trouble, or a machine that would not answer the
/// preflight at all: both are sentences for the human rather than outcomes of
/// the press.
pub(crate) async fn transfer(
    state: &AppState,
    conversation_id: i64,
    device: &str,
) -> Result<Transferring, Refusal> {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return Ok(Transferring::NoSuchConversation),
        Err(why) => {
            return Err(Refusal::ours(format!(
                "the Conversation could not be read: {why:#}"
            )));
        }
    };

    // A tombstone is not this device's to move: what it holds is a copy of work
    // that is being done somewhere else, and the press that moves it is over
    // there. Its URL leads there, so the human pressing here has somewhere to
    // go without being told anything clever.
    if conversation.transferred.is_some() {
        return Ok(Transferring::Elsewhere);
    }

    if !movable(conversation.state) {
        return Ok(Transferring::NotTransferable);
    }

    let reading =
        crate::preflight::of(state.devices.as_ref(), &state.pool, &conversation, device).await?;

    if !reading.lacks.is_empty() {
        return Ok(Transferring::Lacking(reading));
    }

    store::ask_to_transfer(&state.pool, conversation_id, device)
        .await
        .map_err(|why| Refusal::ours(format!("the transfer could not be written down: {why:#}")))?;

    tracing::info!(
        conversation_id,
        device,
        state = ?conversation.state,
        "the human asked for a Conversation to be moved onto another device",
    );

    // The pane says *Transferring to* from here, and the sidebar rows with it.
    state.nudges.announce(Nudge::Conversation {
        conversation: conversation_id,
    });

    see_out(state.clone(), conversation_id, device.to_owned());

    Ok(Transferring::Transferring)
}

/// Which states a Conversation may be moved out of: every one but Draft and
/// Closed.
///
/// A Draft is moved by the device select on its own composer — there is a Brief
/// and no work to carry — and a Closed Conversation has none left to move. The
/// same rule the actions menu draws the row by, asked again here for the reason
/// every press re-asks what drew it.
pub(crate) fn movable(lifecycle: Lifecycle) -> bool {
    !matches!(lifecycle, Lifecycle::Draft | Lifecycle::Closed)
}

/// Whether this Conversation is on its way to another device, or has already
/// gone.
///
/// **What nothing may launch past**, which is why it reads the two together: a
/// session started behind a transfer would be a turn nobody could see out, and
/// one started on a tombstone would be work done on a copy of a record. Asked in
/// front of every launch through [`crate::stopping::stopped`], beside the stop
/// the human asked for.
///
/// A store that will not answer reads as *going*, for the reason its neighbour
/// reads its own failures that way: what is on the other side of this is
/// spending an account, and something that cannot tell whether the work is
/// leaving should wait.
pub(crate) async fn going(state: &AppState, conversation_id: i64) -> bool {
    match store::transfer_asked(&state.pool, conversation_id).await {
        Ok(Some(device)) => {
            tracing::info!(
                conversation_id,
                device,
                "the Conversation is being moved onto another device, so nothing was launched",
            );
            return true;
        }
        Ok(None) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a Conversation was being transferred failed");
            return true;
        }
    }

    match store::transferred(&state.pool, conversation_id).await {
        Ok(Some(to)) => {
            tracing::info!(
                conversation_id,
                device = to.device,
                "the live record is on another device, so nothing was launched",
            );
            true
        }
        Ok(None) => false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether a Conversation had been transferred away failed");
            true
        }
    }
}

/// Whether the stall sweep should leave this Conversation alone: its work is on
/// its way to another device, or it has already gone.
///
/// **Neither of them is a Conversation standing still.** One is being seen out
/// so that it can be moved, and the other is a tombstone — a copy of work being
/// done somewhere else, which nothing here was ever supposed to be driving and
/// which a stall's Notice would be written all over for nothing.
///
/// **And it is where a move nobody is making is taken up.** Nothing survives a
/// process, so a request written before a restart has no mover behind it — and
/// the sweep is exactly the thing that notices a Conversation nobody is driving.
/// The mover this starts holds a driver's registration, so the next sweep leaves
/// it alone, and so does this one: the register is asked before anything is
/// started.
///
/// See [`crate::stalls`], and [`crate::stops::asked`], which is the same shape
/// one press along.
pub(crate) async fn moving(state: &AppState, conversation_id: i64) -> bool {
    if matches!(
        store::transferred(&state.pool, conversation_id).await,
        Ok(Some(_))
    ) {
        return true;
    }

    let Ok(Some(device)) = store::transfer_asked(&state.pool, conversation_id).await else {
        return false;
    };

    if !state.drivers.registered(conversation_id) {
        tracing::info!(
            conversation_id,
            device,
            "nothing is seeing out a Conversation that was asked to move, so the move is taken up",
        );

        see_out(state.clone(), conversation_id, device);
    }

    true
}

/// See the session out, and then move the work.
///
/// **A driver's registration for the whole of it**, which is what it is: from
/// the press until the move has landed, the thing seeing this Conversation along
/// is this task — so the stall sweep leaves it alone and the page says something
/// is holding it.
///
/// The session is waited on rather than ended: Stop after the current task is
/// exactly the promise here, and what stops anything being launched behind it is
/// [`going`], asked in front of every launch. Waited on in a loop because a
/// launch decided before the request landed is a session that appears after the
/// first wait — one lap, and then there is nothing running.
///
/// **And the same session is never waited on twice.** What tells one from
/// another is the Timeline Event it is printing into; a register still naming
/// the one that has just ended is a relay that died without taking itself off,
/// and a loop that waited on it again would be one with nothing to end it.
fn see_out(state: AppState, conversation_id: i64, device: String) {
    let driving = state.drivers.driving(conversation_id);

    tokio::spawn(async move {
        let _driving = driving;
        let mut seen: Option<i64> = None;

        while let Some(mut session) = state.sessions.following(conversation_id) {
            let writing = state.sessions.writing(conversation_id);

            if writing.is_some() && writing == seen {
                tracing::warn!(
                    conversation_id,
                    device,
                    "a session that has ended is still on the register, so the move runs \
                     without waiting on it again",
                );
                break;
            }

            tracing::info!(
                conversation_id,
                device,
                "a Conversation is to be moved, so its session is being seen out",
            );

            seen = writing;
            session.ended().await;
        }

        move_it(&state, conversation_id, &device).await;
    });
}

/// The move itself, once there is nothing left running.
///
/// Every failure comes back here as a sentence and goes through [`failed`],
/// which is what leaves the human with a Conversation they can resume and a
/// Notice saying why they have to.
async fn move_it(state: &AppState, conversation_id: i64, device: &str) {
    // The request as it stands now rather than as it stood at the press. Two
    // presses in the minutes before a session ends are the human changing their
    // mind about where the work goes: the second one's mover has the request,
    // and the first one's stands down rather than racing it.
    match store::transfer_asked(&state.pool, conversation_id).await {
        Ok(Some(asked)) if asked == device => {}

        Ok(Some(asked)) => {
            tracing::info!(
                conversation_id,
                asked,
                superseded = device,
                "the Conversation is going somewhere else now, so this move stands down",
            );
            return;
        }

        Ok(None) => {
            tracing::info!(
                conversation_id,
                device,
                "the move was taken back before it ran",
            );
            return;
        }

        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the transfer to make failed");
            return;
        }
    }

    match across(state, conversation_id, device).await {
        Ok(there) => {
            tracing::info!(
                conversation_id,
                device,
                there,
                "a Conversation has been moved onto another device",
            );

            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });
            state.nudges.announce(Nudge::Conversations);
        }

        Err(Went { saying, landed }) => {
            failed(state, conversation_id, device, saying, landed).await
        }
    }
}

/// What a move that did not finish has to say, and what it left behind.
struct Went {
    /// Why it stopped, in the words the Notice carries.
    saying: String,

    /// And the id of the copy that reached the far end, where one did: what the
    /// sweep takes back.
    landed: Option<i64>,
}

/// Put the Conversation to the far end, and mark this copy transferred once that
/// device has confirmed it.
///
/// The id the far end gave its copy, or what went wrong and what is left over
/// there to sweep.
async fn across(state: &AppState, conversation_id: i64, device: &str) -> Result<i64, Went> {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            return Err(Went {
                saying: "the Conversation is no longer on this device".to_owned(),
                landed: None,
            });
        }
        Err(why) => {
            return Err(Went {
                saying: format!("this device could not read the Conversation: {why:#}"),
                landed: None,
            });
        }
    };

    let named = relaying::called(state.devices.as_ref(), device).await;

    let Some(devices) = state.devices.as_ref() else {
        return Err(Went {
            saying: "this server holds no device identity to move work through".to_owned(),
            landed: None,
        });
    };

    // Which of that device's Repos is this repository, asked again at the moment
    // the work goes: the preflight settled it when the press was made, and what
    // is acted on is what git says now. Refused rather than guessed — a branch
    // landing in the wrong directory is the one failure this rule exists to
    // prevent.
    let repo = match crate::matching::across(devices, device, &conversation.repo).await {
        Ok(Some(there)) => there.id,

        Ok(None) => {
            return Err(Went {
                saying: format!(
                    "no repository on {named} is {repo} any more",
                    repo = conversation.repo.name,
                ),
                landed: None,
            });
        }

        Err(refusal) => {
            return Err(Went {
                saying: refusal.saying,
                landed: None,
            });
        }
    };

    let Some(born) = birth(state, conversation_id).await else {
        return Err(Went {
            saying: "this Conversation has no birth key, so no device could tell its copy of \
                     the work from another's"
                .to_owned(),
            landed: None,
        });
    };

    let rank = match store::conversation_rank(&state.pool, conversation_id).await {
        Ok(rank) => rank,
        Err(why) => {
            return Err(Went {
                saying: format!("this device could not read the Conversation's rank: {why:#}"),
                landed: None,
            });
        }
    };

    let Some(rank) = rank else {
        return Err(Went {
            saying: "this Conversation has no rank, so no list could say where its copy sits"
                .to_owned(),
            landed: None,
        });
    };

    let here = state.device.clone();

    let going = ConversationAcross {
        born: BirthKey {
            device: born.device,
            id: born.id,
        },
        repo,
        branch: conversation.branch.clone(),
        branch_named: conversation.branch_named,
        naming: conversation.naming,
        state: crate::ui::lifecycle(conversation.state),
        rank,
        grilling: conversation
            .grilling_pairing
            .as_ref()
            .map_or(PickedAcross::Nothing, |pairing| {
                PickedAcross::Under(pairing_across(pairing, &here))
            }),
        implementation: conversation
            .implementation_pairing
            .as_ref()
            .map_or(PickedAcross::Nothing, |pairing| {
                PickedAcross::Under(pairing_across(pairing, &here))
            }),
        review: picked(&conversation.review_pairing, &here),
    };

    let saying = serde_json::to_vec(&going).map_err(|why| Went {
        saying: format!("the Conversation could not be written down to send: {why}"),
        landed: None,
    })?;

    let arrived: Arrived = relaying::word_from(
        state.devices.as_ref(),
        device,
        Call {
            method: reqwest::Method::POST,
            onwards: TRANSFERS.to_owned(),
            headers: as_json(),
            body: Streamed::saying(saying),
        },
        MOST_AN_ANSWER_IS,
    )
    .await
    .map_err(|refusal| Went {
        saying: refusal.saying,
        landed: None,
    })?;

    // The commit point is behind us: that device has the work. What is left is
    // saying so here, and a failure at this one step is the one that has
    // something over there to take back.
    store::transfer_away(
        &state.pool,
        conversation_id,
        &store::Transferred {
            device: device.to_owned(),
            id: arrived.id,
        },
    )
    .await
    .map_err(|why| Went {
        saying: format!("this device could not record where the work went: {why:#}"),
        landed: Some(arrived.id),
    })?;

    // And the request is spent. After the mark rather than before it, so that
    // nothing between the two reads this Conversation as one still standing
    // still with a session to launch.
    if let Err(error) = store::forget_transfer(&state.pool, conversation_id).await {
        tracing::error!(error = ?error, conversation_id, "the transfer that has been made could not be forgotten");
    }

    Ok(arrived.id)
}

/// A move that did not finish: sweep what reached the far end, take the request
/// away, and stop the Conversation with a Notice naming what failed.
///
/// **Live and stopped**, which is the pair that makes a failure safe: nothing
/// here writes the mark, so this copy is still the record and the work is still
/// where it was — and the stop is what says so on the page and waits for the
/// human's press. The Worktree is untouched, as it is on the way through.
async fn failed(
    state: &AppState,
    conversation_id: i64,
    device: &str,
    saying: String,
    landed: Option<i64>,
) {
    let named = relaying::called(state.devices.as_ref(), device).await;

    tracing::error!(
        conversation_id,
        device,
        "moving a Conversation onto another device failed: {saying}",
    );

    if let Some(there) = landed {
        swept(state, device, there).await;
    }

    if let Err(error) = store::forget_transfer(&state.pool, conversation_id).await {
        tracing::error!(error = ?error, conversation_id, "a transfer that failed could not be forgotten");
    }

    let lifecycle = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation.state,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading a Conversation whose move failed came to nothing");
            return;
        }
    };

    let stopped = crate::stopping::stop(
        &state.pool,
        &state.nudges,
        conversation_id,
        crate::stopping::Decided::Verkstead,
        crate::stalls::driving(lifecycle),
        &format!("moving it onto {named} failed: {saying}"),
        crate::stalls::said_last(state, conversation_id).await,
    )
    .await;

    if let Err(error) = stopped {
        tracing::error!(error = ?error, conversation_id, "a transfer that failed could not be recorded as a stop");
    }
}

/// Take back the copy that reached the far end.
///
/// Said in the log and nowhere else: what the human is owed about a failed move
/// is the Notice on the Conversation they still have, and whether the machine
/// that took a copy managed to let go of it again is a thing for this device to
/// keep saying rather than for them to act on.
async fn swept(state: &AppState, device: &str, there: i64) {
    let swept = relaying::put_to(
        state.devices.as_ref(),
        device,
        Call {
            method: reqwest::Method::DELETE,
            onwards: format!("{TRANSFERS}/{there}"),
            headers: as_json(),
            body: Streamed::Nothing,
        },
    )
    .await;

    match swept {
        Ok(_) => tracing::info!(
            device,
            there,
            "the copy a failed move left on a member was swept",
        ),

        Err(refusal) => tracing::error!(
            device,
            there,
            "the copy a failed move left on a member could not be swept: {}",
            refusal.saying,
        ),
    }
}

/// The key this Conversation was born under, where it has one.
///
/// `None` is a database written before there were any and not yet stamped,
/// which no running server holds: a serve stamps every Conversation before it
/// answers anything. It is a refusal rather than a key invented here, because a
/// key invented on the way out is a copy the Merged List would draw twice.
async fn birth(state: &AppState, conversation_id: i64) -> Option<store::Birth> {
    match store::birth(&state.pool, conversation_id).await {
        Ok(born) => born,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the key a Conversation was born under failed");
            None
        }
    }
}

/// One role's choice, as it crosses.
fn picked(picked: &store::Picked, here: &str) -> PickedAcross {
    match picked {
        store::Picked::Nothing => PickedAcross::Nothing,
        store::Picked::Skipped => PickedAcross::Skipped,
        store::Picked::Under(pairing) => PickedAcross::Under(pairing_across(pairing, here)),
    }
}

/// And one Pairing, with its Profile named the way a cluster names one: the
/// device it is at home on, and the id it has **there**.
///
/// A mirror carries both already — it is a row of this device's marked with
/// where the account lives. One of this device's own is at home here, so the
/// pair is this device's id and the row's own: which is exactly what the far end
/// turns back into a mirror of its own, the work having moved to the machine
/// that was the member.
fn pairing_across(pairing: &store::Pairing, here: &str) -> PairingAcross {
    let profile = match &pairing.profile.mirror {
        Some(mirror) => ProfileAcross {
            device: mirror.device.clone(),
            id: mirror.id,
        },
        None => ProfileAcross {
            device: here.to_owned(),
            id: pairing.profile.id,
        },
    };

    PairingAcross {
        profile,
        model: pairing.model.clone(),
    }
}

/// What the Timeline says while a Conversation is on its way: the name of the
/// machine it is going to, or nothing where it is going nowhere.
///
/// The name rather than the Device Id, because it is drawn in a sentence — see
/// [`verkstead_render::ConversationView::transferring`]. A read that fails reads
/// as *going nowhere*, which is the way round that leaves the page saying less
/// rather than saying something wrong.
pub(crate) async fn transferring(state: &AppState, conversation_id: i64) -> Option<String> {
    let device = match store::transfer_asked(&state.pool, conversation_id).await {
        Ok(device) => device?,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which device a Conversation is going to failed");
            return None;
        }
    };

    Some(relaying::called(state.devices.as_ref(), &device).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every state but the two, which is the rule the row is drawn by and the
    /// rule the press is refused by — one rule, said here.
    #[test]
    fn a_draft_and_a_closed_conversation_are_the_two_that_do_not_move() {
        for state in [
            Lifecycle::Grilling,
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
            Lifecycle::FollowUp,
            Lifecycle::Done,
        ] {
            assert!(movable(state), "{state:?} is work on a machine");
        }

        assert!(
            !movable(Lifecycle::Draft),
            "a Draft is moved by its own composer",
        );
        assert!(
            !movable(Lifecycle::Closed),
            "a Closed Conversation has no work left to move",
        );
    }
}
