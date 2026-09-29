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
//! **What crosses is the Conversation and its whole record**: the row first —
//! the Repo the matching settled, the branch, the lifecycle, the Pairings as
//! ids of the far end's own, the **Rank** verbatim and the **birth key** — and
//! then the **slice**, which is the Timeline and everything hanging off it,
//! renumbered as it lands. See [`store::slice`] for what a slice is and
//! [`record`] for the leg that carries it. The branch and the Worktree are the
//! tasks after this one.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use verkstead_render::{
    Arrived, BirthKey, ConversationAcross, PairingAcross, PickedAcross, ProfileAcross, Transferring,
};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::attachments::Attachments;
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

/// The most a record may be on the wire: **twice what the rows themselves may
/// weigh**.
///
/// The bound that matters is [`store::MOST_A_SLICE_IS`], which is over the bytes
/// of the record — the Transcript lines and the Capture chunks and the attached
/// files, counted as they are read. This is the same bound said again over what
/// goes down the wire, which is bigger than what it carries: JSON escapes what
/// it quotes and base64 is four bytes for every three. Twice is comfortably past
/// either, and it is here for the reason every body limit on this listener is —
/// a caller that starts writing and does not stop.
pub(crate) const MOST_A_RECORD_IS: usize = 2 * store::MOST_A_SLICE_IS;

/// A Conversation's whole record, as it crosses.
///
/// **Not a viewer type**, for [`ConversationAcross`]'s reason: the two ends of it
/// are two Verksteads. And not a `verkstead_render` type either, because what it
/// carries is the store's own rows — a slice is read out of one database and
/// written into another, and nothing between the two renders it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecordAcross {
    /// Every row of the record, with the ids as they stand on the device it came
    /// off — see [`store::Slice`].
    pub(crate) slice: store::Slice,

    /// Which of **the receiving device's** Repos each of those is, as the match
    /// settled it: a row landing under the wrong repository is the failure that
    /// whole rule exists to prevent, so nothing is guessed at the other end.
    pub(crate) repos: Vec<(i64, i64)>,

    /// And every Agent Profile the record names, by the device each is at home
    /// on and the id it has there — which is the one name for an account that
    /// means the same thing on both machines.
    pub(crate) profiles: Vec<(i64, ProfileAcross)>,

    /// The attached files' bytes, beside the rows that name them. Each lands in
    /// the far end's own attachments directory, under its own Conversation id,
    /// so a session there is given the paths its own sandbox expects.
    pub(crate) files: Vec<AttachedFile>,
}

/// One attached file on the way across.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AttachedFile {
    /// What it is called in the Conversation's directory, which is the name on
    /// the row — the counted-up one where the directory already had that name,
    /// so the row and the file agree on both machines by construction.
    pub(crate) name: String,

    /// Its bytes, base64 — the way a memory store's files cross, and for the
    /// same reason: the envelope is JSON and an attachment is a screenshot.
    pub(crate) bytes: String,
}

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

    // And the record itself, which is the leg with everything about the work in
    // it. A failure from here on has a copy over there to take back.
    if let Err(saying) = record(state, device, conversation_id, arrived.id).await {
        return Err(Went {
            saying,
            landed: Some(arrived.id),
        });
    }

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

/// Put the Conversation's whole record over: the Timeline and everything hanging
/// off it, and the attached files beside the rows that name them.
///
/// **A leg of its own, after the row.** The row is what the far end numbers, and
/// the number is what everything here is written against — so the record cannot
/// go until there is something for it to land beside. Which also makes this the
/// one part of a move that can fail with a copy already over there, and the
/// caller sweeps it: see [`across`], where the commit point is.
///
/// **The two ids the store cannot renumber for itself are settled here**: which
/// of the far end's Repos each of ours is, by the match the whole cluster runs
/// on, and what each Agent Profile is called across a cluster, which is the
/// device it is at home on and its id there. A repository with nowhere to land
/// refuses the move by name; an account the far end has never heard of leaves
/// its column empty over there, that being history rather than a Pairing
/// anything is going to be launched under.
///
/// `Err` is the sentence the Notice carries.
async fn record(
    state: &AppState,
    device: &str,
    conversation_id: i64,
    there: i64,
) -> Result<(), String> {
    let slice = store::slice(&state.pool, conversation_id)
        .await
        .map_err(|why| format!("{why:#}"))?;

    let repos = matched(state, device, &slice).await?;
    let profiles = accounts(state, &slice).await?;
    let files = attached(state, conversation_id, slice.weight()).await?;

    let saying = serde_json::to_vec(&RecordAcross {
        slice,
        repos,
        profiles,
        files,
    })
    .map_err(|why| format!("the Conversation's record could not be written down to send: {why}"))?;

    relaying::put_to(
        state.devices.as_ref(),
        device,
        Call {
            method: reqwest::Method::POST,
            onwards: format!("{TRANSFERS}/{there}/record"),
            headers: as_json(),
            body: Streamed::saying(saying),
        },
    )
    .await
    .map(|_| ())
    .map_err(|refusal| refusal.saying)
}

/// Which of `device`'s Repos each of the ones this record names is.
///
/// Refused by name where one of them is nothing over there — the preflight asks
/// the same question of the Conversation's own repository and each Companion,
/// and this is every repository the *record* names, which is those and whatever
/// a commit was once recorded in.
async fn matched(
    state: &AppState,
    device: &str,
    slice: &store::Slice,
) -> Result<Vec<(i64, i64)>, String> {
    let ids: Vec<i64> = slice.repos().into_iter().collect();

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let Some(devices) = state.devices.as_ref() else {
        return Err(
            "this server holds no device identity to match repositories through".to_owned(),
        );
    };

    let mut ours = Vec::with_capacity(ids.len());

    for id in &ids {
        match store::load_repo(&state.pool, *id).await {
            Ok(Some(repo)) => ours.push(repo),
            Ok(None) => {
                return Err(format!(
                    "the record names repository {id}, which is not registered on this device \
                     any more",
                ));
            }
            Err(why) => {
                return Err(format!(
                    "a repository the record names could not be read: {why:#}"
                ));
            }
        }
    }

    let theirs = crate::matching::each_across(devices, device, &ours)
        .await
        .map_err(|refusal| refusal.saying)?;

    let mut across = Vec::with_capacity(ids.len());

    for ((id, repo), there) in ids.into_iter().zip(&ours).zip(theirs) {
        let Some(there) = there else {
            return Err(format!(
                "no repository there is {name}, which this Conversation's record names",
                name = repo.name,
            ));
        };

        across.push((id, there.id));
    }

    Ok(across)
}

/// And what each Agent Profile the record names is called across a cluster.
///
/// One read of the registry for all of them, the way an arriving Conversation's
/// three Pairings are resolved against one. A Profile that is not in it any more
/// is left out: what names one here is a Steer somebody ran a year ago, and an
/// account deleted since is a name the record cannot keep on either machine.
async fn accounts(
    state: &AppState,
    slice: &store::Slice,
) -> Result<Vec<(i64, ProfileAcross)>, String> {
    let ids = slice.profiles();

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let profiles = store::profiles(&state.pool)
        .await
        .map_err(|why| format!("this device's Agent Profiles could not be read: {why:#}"))?;

    Ok(ids
        .into_iter()
        .filter_map(|id| {
            let profile = profiles.iter().find(|profile| profile.id == id)?;

            Some((id, profile_across(profile, &state.device)))
        })
        .collect())
}

/// The attached files' bytes, read off this device's attachments directory.
///
/// **Counted towards the same bound the rows are**, which is what makes it one
/// bound over a record rather than two over halves of one — see
/// [`store::MOST_A_SLICE_IS`]. Past it the move is refused whole, naming the
/// Conversation: half a record is worse than a move that did not happen.
///
/// A row whose file has gone from the directory is a row and no file, which is
/// what it already was here: the record crosses and the bytes do not, and the
/// far end draws exactly what this one drew.
///
/// Blocking, off the runtime's threads: these are as much as thirty-two
/// megabytes apiece.
async fn attached(
    state: &AppState,
    conversation_id: i64,
    weighs: usize,
) -> Result<Vec<AttachedFile>, String> {
    let rows = store::attachments(&state.pool, conversation_id)
        .await
        .map_err(|why| {
            format!("the files attached to this Conversation could not be read: {why:#}")
        })?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let directories = Attachments::under(&state.data_dir);

    tokio::task::spawn_blocking(move || {
        let mut held = weighs;
        let mut files = Vec::with_capacity(rows.len());

        for row in rows {
            let path = directories.file(conversation_id, &row.name);

            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                    tracing::warn!(
                        conversation_id,
                        name = %row.name,
                        "an attached file is not in the Conversation's directory, so the row \
                         crossed without it",
                    );

                    continue;
                }
                Err(why) => {
                    return Err(format!(
                        "the attached file {name} could not be read: {why}",
                        name = row.name,
                    ));
                }
            };

            held += bytes.len();

            if held > store::MOST_A_SLICE_IS {
                return Err(format!(
                    "the record of Conversation {conversation_id} is larger than the {most} \
                     bytes one may be to cross a link",
                    most = store::MOST_A_SLICE_IS,
                ));
            }

            files.push(AttachedFile {
                name: row.name,
                bytes: STANDARD.encode(&bytes),
            });
        }

        Ok(files)
    })
    .await
    .map_err(|why| format!("the attached files could not be read: {why}"))?
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

/// And one Pairing: the Profile named the way a cluster names one — see
/// [`profile_across`] — and the model beside it.
fn pairing_across(pairing: &store::Pairing, here: &str) -> PairingAcross {
    PairingAcross {
        profile: profile_across(&pairing.profile, here),
        model: pairing.model.clone(),
    }
}

/// One Agent Profile, named the way a cluster names one.
///
/// A mirror carries both already — it is a row of this device's marked with
/// where the account lives. One of this device's own is at home here, so the
/// pair is this device's id and the row's own: which is exactly what the far end
/// turns back into a mirror of its own, the work having moved to the machine
/// that was the member.
fn profile_across(profile: &store::Profile, here: &str) -> ProfileAcross {
    match &profile.mirror {
        Some(mirror) => ProfileAcross {
            device: mirror.device.clone(),
            id: mirror.id,
        },
        None => ProfileAcross {
            device: here.to_owned(),
            id: profile.id,
        },
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
