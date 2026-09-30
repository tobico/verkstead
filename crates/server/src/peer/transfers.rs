//! Taking a Conversation in from another device, and letting go of one whose
//! move fell over (ADR-0020, *Transfer*).
//!
//! **The receiving half of a move.** The device holding the work does the
//! deciding — which of this device's Repos is that repository, which of its
//! Agent Profiles each Pairing means — because that is the end that acts on the
//! answers, and [`crate::preflight`] is where it asks them. What is left here is
//! the writing: a row of this device's own, under ids this device issued,
//! carrying the two things that are the cluster's rather than anybody's — the
//! **Rank** and the **birth key**.
//!
//! **And a copy coming home lands on the row it left.** The second time work
//! moves it is very often coming back — drafted on the laptop, worked on the
//! desktop, and home again — and the birth key is what says the arriving
//! Conversation and the copy this device kept are one piece of work. So the copy
//! is written over, under the id it already has: what the sending device holds is
//! the live record and what is here is a stale copy of it, so nothing is merged
//! and nothing is reconciled. The id is the whole point of doing it that way —
//! every bookmark, Timeline reference and Answer to a Set asked months ago names
//! it, and a second Conversation beside the first would leave all of them
//! pointing at a copy of work that had come back. The record lands over the old
//! one and the Worktree this device cut the first time is the one the branch is
//! brought back into — see [`super::checkouts`].
//!
//! **And the answer is the commit point of the whole move.** The id this device
//! numbered its copy is what the sending device writes on its own copy as the
//! mark saying where the live record now is; until that answer arrives nothing
//! has changed over there and the work is still being done. Which is what makes
//! a failure safe, and what the sweep below is for: a move that fell over after
//! this wrote and before that mark landed leaves a copy nobody is going to use,
//! and the device that sent it takes it back rather than leaving a half-record
//! on a machine the work never reached.
//!
//! **Which is why nothing here starts anything.** Everything the sending device
//! sends before its own copy is marked is a thing it may take back, and a session
//! launched in a checkout that was about to be swept would be an agent working in
//! a directory nobody could account for. So there is a last call — see
//! [`arrived`] — which the sending device makes once the mark is written, and
//! *that* is the one this device acts on: a Notice saying which machine the work
//! came from, the conversation the agent was part way through written down for
//! whatever is launched next to carry on — see [`carries_on`] — and **Resume**
//! pressed by this device for itself.
//!
//! **Gated to members like everything else on this listener.** Writing a
//! Conversation into somebody's database is not a stranger's press:
//! [`super::router`] puts [`super::members_only`] over this, and a caller whose
//! certificate this device holds no membership for never reaches it.

use std::collections::HashMap;

use axum::Json;
use axum::Router;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, post};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use verkstead_render::{Arrived, CameFrom, ConversationAcross, PickedAcross, ProfileAcross};

use crate::attachments::Attachments;
use crate::transfers::{AttachedFile, MOST_A_RECORD_IS, RecordAcross};
use crate::{AppState, store};

/// Where a member puts a Conversation it is moving onto this device.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace: no browser makes
/// this call and no page draws its answer. The Repos and the harnesses beside it
/// are readings of this machine; this is the one press a member makes on this
/// listener that writes to the record.
pub const TRANSFERS: &str = "/api/peer/v1/transfers";

/// And where it takes one back, the id being the one this device answered with.
pub const ONE_TRANSFER: &str = "/api/peer/v1/transfers/{id}";

/// And where the record itself lands, once that row is there to land beside it.
///
/// **A leg of its own rather than more of the first call.** The ids everything
/// in a record points at are this device's, and the first of them is the
/// Conversation's own — which is what the first call answered with. So the row
/// goes, this device numbers it, and the record follows against that number.
pub const ONE_TRANSFERS_RECORD: &str = "/api/peer/v1/transfers/{id}/record";

/// And where the device that sent it says the move is over, which is the one call
/// this device may act on — see [`arrived`].
pub const ONE_TRANSFERS_ARRIVAL: &str = "/api/peer/v1/transfers/{id}/arrival";

/// `POST /api/peer/v1/transfers` — write the arriving Conversation down, and
/// answer with the id it goes by here.
///
/// **Or the id it already went by**, where this device holds a copy of the very
/// work that is arriving: a Conversation drafted on the laptop, worked on the
/// desktop and coming home is one piece of work whichever way it is travelling,
/// and what says so is the **birth key** it carries. The copy kept here when the
/// work left is written over rather than a second Conversation being made beside
/// it — see [`store::replace`], and this module's own header. Which is the whole
/// of what a transfer back is at this end: every leg after it is the same leg.
///
/// The Repo is refused where this registry no longer holds it and each Pairing
/// where no Profile here is the one it names — both by name, because both are
/// the sending device acting on a reading that has gone stale and both are worth
/// saying out loud on the Timeline the failure ends up on.
pub(crate) async fn take(
    State(state): State<AppState>,
    Json(arriving): Json<ConversationAcross>,
) -> Response {
    let born = store::Birth {
        device: arriving.born.device.clone(),
        id: arriving.born.id,
    };

    let (grilling, implementation, review) = match resolved(&state, &arriving).await {
        Ok(picked) => picked,
        Err(refusal) => return refusal,
    };

    let arrival = store::Arrival {
        born: born.clone(),
        repo_id: arriving.repo,
        branch: arriving.branch.clone(),
        branch_named: arriving.branch_named,
        naming: arriving.naming,
        state: crate::ui::state_of(arriving.state),
        rank: arriving.rank.clone(),
        grilling,
        implementation,
        review,
    };

    // Whether this device already holds a copy of this work, asked before
    // anything is written: the answer is the difference between a Conversation
    // arriving and one coming back.
    let held = match store::born_as(&state.pool, &born).await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(error = ?why, "looking for the copy a Conversation a member is transferring here would replace failed");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not look for its own copy of the work\n",
            )
                .into_response();
        }
    };

    let written = match held {
        Some(id) => back(&state, id, &arrival).await,
        None => fresh(&state, &arrival).await,
    };

    let id = match written {
        Ok(id) => id,
        Err(refusal) => return refusal,
    };

    tracing::info!(
        conversation_id = id,
        born = arriving.born.id,
        on = arriving.born.device,
        back = held.is_some(),
        "a Conversation transferred from a member landed here",
    );

    // The sidebar of every browser open on this device, and of every member
    // holding this device's list: there is a Conversation here that was not here
    // a moment ago, or one that is about to be the live copy again.
    state
        .nudges
        .announce(verkstead_schema::Nudge::Conversations);

    Json(Arrived { id }).into_response()
}

/// A Conversation this device has never held: a row of its own, and the id it
/// was given.
async fn fresh(state: &AppState, arrival: &store::Arrival) -> Result<i64, Response> {
    match store::arrive(&state.pool, arrival).await {
        Ok(Some(id)) => Ok(id),

        Ok(None) => Err((
            StatusCode::BAD_REQUEST,
            format!(
                "no repository is registered on this device under the id {repo} the transfer \
                 named\n",
                repo = arrival.repo_id,
            ),
        )
            .into_response()),

        Err(why) => {
            tracing::error!(error = ?why, "a Conversation a member is transferring here could not be written down");

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "the Conversation could not be written down on this device\n",
            )
                .into_response())
        }
    }
}

/// And one coming home: the copy held here written over, under the id it already
/// has.
///
/// **The id is the whole point.** A bookmark, a Timeline reference on another
/// Conversation, a URL in a Question Set answered months ago — all of them name
/// the id this device issued the first time the work landed, and all of them go
/// on working because the return lands on it. A second Conversation beside the
/// first would leave every one of them pointing at a stale copy of work that had
/// come back.
async fn back(state: &AppState, id: i64, arrival: &store::Arrival) -> Result<i64, Response> {
    match store::replace(&state.pool, id, arrival).await {
        Ok(store::Replacing::Replaced) => Ok(id),

        Ok(store::Replacing::NoSuchRepo) => Err((
            StatusCode::BAD_REQUEST,
            format!(
                "no repository is registered on this device under the id {repo} the transfer \
                 named\n",
                repo = arrival.repo_id,
            ),
        )
            .into_response()),

        // The copy was deleted between the lookup and the write, which is a
        // human clearing out a machine at the moment the work came back to it.
        // Refused rather than written afresh: the sending device reads this as
        // the move not happening, and the work stays where it is.
        Ok(store::Replacing::NoSuchConversation) => Err((
            StatusCode::BAD_REQUEST,
            "the copy of this Conversation held on this device is no longer here\n",
        )
            .into_response()),

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "a Conversation a member is transferring back here could not be written down");

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "the Conversation could not be written down on this device\n",
            )
                .into_response())
        }
    }
}

/// `POST /api/peer/v1/transfers/{id}/record` — the Timeline and everything
/// hanging off it, written down against the Conversation this device numbered a
/// moment ago.
///
/// **Every id is this device's by the time it is written.** The rows arrive
/// carrying the numbers they had on the machine they came off — every Verkstead
/// has a Conversation 1 — and the store renumbers them as they land, so an Event
/// referenced by a Capture, a Transcript, a session Pairing or a Question Set
/// comes out pointing at the Event it actually landed as. What the store cannot
/// work out for itself is the two ids that are not this Conversation's: the
/// Repos, which the sending device matched against this registry before it sent
/// anything, and the Agent Profiles, which are named across a cluster by the
/// device each is at home on — see [`resolved`], which is the same reading one
/// call along.
///
/// **The files first, then the rows.** An attachment is a row and bytes both,
/// and the row is what names the file: written the other way round there would
/// be a moment in which a session on this device could be told about a path that
/// is not there yet. A landing that then fails takes the directory back with it,
/// the copy being one the sending device is about to sweep.
///
/// **And the record lands over whatever was here**, which is what a Conversation
/// coming home does to the copy this device kept: the rows go in the very
/// transaction the new ones land in — see [`store::land`] — and the directory
/// their files are in is moved aside first, so each file keeps the name its row
/// says it was stored under rather than being counted up past the copy of itself
/// that was already there.
///
/// **Aside rather than away, because a return that fails leaves the copy here
/// standing.** What the sending device sweeps on a failure is a copy that has only
/// ever been arriving; a copy that was here before the move began is this device's
/// own row from the first time the work was here, and it keeps it — see
/// [`sweep`]. So a directory *deleted* in front of a landing that then failed
/// would leave every attachment row of that copy naming a file nothing could put
/// back. It goes back where it was instead, and is only let go of once the record
/// has landed over it — see [`Attachments::set_aside`].
pub(crate) async fn written(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(record): Json<RecordAcross>,
) -> Response {
    let profiles = match accounts(&state, &record).await {
        Ok(profiles) => profiles,
        Err(refusal) => return refusal,
    };

    let renaming = store::Renaming {
        repos: record.repos.iter().copied().collect(),
        profiles,
    };

    let directories = Attachments::under(&state.data_dir);

    let aside = match directories.set_aside(id) {
        Ok(aside) => aside,

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "the files of the copy a Conversation a member is transferring here would land over could not be held aside");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the attached files already here could not be held aside\n",
            )
                .into_response();
        }
    };

    // Whatever this landing wrote taken back, and the copy that was here put back
    // where it was — which is the one thing every failure below does. A fresh
    // arrival held nothing aside, so what it gives back is the files this landing
    // wrote and nothing else.
    let given_back = || match &aside {
        Some(held) => directories.put_back(id, held),
        None => directories.remove(id),
    };

    if let Err(why) = kept(&state, id, &record.files).await {
        tracing::error!(error = ?why, conversation_id = id, "the files on a Conversation a member is transferring here could not be written");

        given_back();

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "the attached files could not be written down on this device\n",
        )
            .into_response();
    }

    match store::land(&state.pool, id, &record.slice, &renaming).await {
        Ok(()) => {
            // The record has landed over the old one, so what was held aside is
            // the attached files of a copy nothing names any more.
            if let Some(held) = &aside {
                directories.let_go_of(id, held);
            }

            tracing::info!(
                conversation_id = id,
                tables = record.slice.tables.len(),
                files = record.files.len(),
                "the record of a Conversation transferred from a member landed here",
            );

            // The Timeline on every browser open on this device, and the sidebar
            // of every member holding its list: there is work here now rather
            // than the empty row the first leg left.
            state
                .nudges
                .announce(verkstead_schema::Nudge::Conversation { conversation: id });
            state
                .nudges
                .announce(verkstead_schema::Nudge::Conversations);

            StatusCode::OK.into_response()
        }

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "the record of a Conversation a member is transferring here could not be written down");

            given_back();

            (StatusCode::BAD_REQUEST, format!("{why:#}\n")).into_response()
        }
    }
}

/// `POST /api/peer/v1/transfers/{id}/arrival` — the device that sent the work
/// saying the move is over, and this device taking it up.
///
/// **The one leg after the commit point, and the only one this device may act
/// on.** Every call before it is in front of the mark the sending device writes:
/// a move that falls over there is swept — see [`sweep`] — and a session started
/// in a checkout that was about to be taken back would be an agent working in a
/// directory nobody could account for. By the time this arrives the sending
/// device has let go, so the work is *here*, and two things follow from that.
///
/// **A Notice saying where it came from**, so the record reads as one story
/// across the two machines rather than as a Conversation that appeared from
/// nowhere. By the name this device knows that machine by, the way every sentence
/// about a member is written — see [`crate::relaying::called`].
///
/// **And Resume, pressed by this device for itself.** Resume is the one standing
/// way in: it asks what *ought* to be running now, from the lifecycle the
/// Conversation is in and what the branch has written, which is exactly the
/// question a Conversation that has just arrived poses. See
/// [`crate::resume::on_arrival`], where the refusals are written down.
///
/// **With the conversation the agent was having written down in front of it**, so
/// that the session Resume starts carries it on rather than opening one of its own
/// — see [`carries_on`]. Written whether or not anything starts here: a
/// Conversation that arrives stopped by a decision waits for a press, and the
/// press the human makes the next morning is still the one that picks this
/// conversation up.
///
/// **And where the work has come home, this is where it stops being a copy.** A
/// Conversation this device had handed on wears the mark saying where the live
/// record is, and it goes on wearing it through every leg of the return — each of
/// those being in front of the commit point, so a return that falls over leaves
/// the tombstone it started as, still pointing at the machine still doing the
/// work. The word that the move is over is what takes it off: see
/// [`store::live_here`]. Nothing to take off is the ordinary arrival, which never
/// wore one.
///
/// **In that order, too.** A Conversation wearing the mark is one nothing may be
/// launched in — see [`crate::transfers::going`] — so the mark comes off before
/// the Resume below, or the press this device makes for itself would be refused
/// on the grounds that the work is somewhere else.
///
/// **`Ok` whatever the Resume decided**, because there is nothing the sending
/// device could do with the answer: its copy is already the tombstone and the
/// work is already here. What a refusal leaves is a stop on this device's own
/// Timeline, which is where the human who has to act on it is looking.
pub(crate) async fn arrived(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(from): Json<CameFrom>,
) -> Response {
    if !matches!(store::load_conversation(&state.pool, id).await, Ok(Some(_))) {
        return (
            StatusCode::BAD_REQUEST,
            "no Conversation on this device has that id\n",
        )
            .into_response();
    }

    let named = crate::relaying::called(state.devices.as_ref(), &from.device).await;

    tracing::info!(
        conversation_id = id,
        device = from.device,
        "a member says the Conversation it moved here has finished arriving",
    );

    if let Err(error) = store::live_here(&state.pool, id).await {
        tracing::error!(error = ?error, conversation_id = id, "recording that the work is on this device again failed");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "this device could not record that the work is here\n",
        )
            .into_response();
    }

    let noted = store::note(
        &state.pool,
        id,
        &format!(
            "Transferred to **{here}** at {whose} request: this Conversation was moved onto \
             this device from **{named}**. Its branch, its Worktree and whatever was \
             uncommitted in it are here, and the record above it came across with them.",
            here = state
                .devices
                .as_ref()
                .map_or_else(crate::platform::hostname, |devices| devices.name()),
            whose = from.asked.whose(),
        ),
    )
    .await;

    if let Err(error) = noted {
        tracing::error!(error = ?error, conversation_id = id, "saying on the Timeline where a Conversation arrived from failed");
    }

    state
        .nudges
        .announce(verkstead_schema::Nudge::Conversation { conversation: id });

    carries_on(&state, id).await;

    crate::resume::on_arrival(&state, id, &named).await;

    StatusCode::OK.into_response()
}

/// Write down the conversation the work arrived part way through, so that the
/// session started here carries it on rather than opening one of its own
/// (ADR-0020, *Transfer*).
///
/// **The newest session on the record is the one the agent was having.** The
/// slice landed a leg ago, so every name Verkstead gave this Conversation's
/// sessions is here in order, and the last of them is the one that was running
/// when the work moved — see [`store::the_last_session`].
///
/// **Here rather than inside the Resume**, because a Conversation may arrive with
/// nothing started for it: one stopped by a decision waits for a press, and the
/// press the human makes the next morning is still the one that carries this
/// conversation on. So what says the conversation is there to be taken up is
/// written down, and the first launch that reads it is the one that takes it —
/// see [`crate::sessions`].
///
/// **And whether it can be taken up is the launch's question rather than this
/// one.** Whether the harness has a resume line, whether the Pairing about to run
/// is the harness that session ran on, and whether the log came across with the
/// memory sync are all facts about the launch — and the last of them is not
/// settled until a member's account has answered. So this writes down what there
/// is, and nothing here refuses.
///
/// A Conversation with no named session is one there is nothing to carry on:
/// every backend Verkstead names no session for, and every Conversation whose
/// work had not started when it moved. **Said on the Timeline**, because it is one
/// of the reasons the human is owed for a session that starts from the record
/// rather than mid-turn — and the one of them known here rather than at the launch.
/// Which is also why it is worded about whatever is started rather than about a
/// session: an arrival into a state nothing drives starts nothing at all, and the
/// press somebody makes the next morning is the one this is about. The two ways
/// the writing itself can fail are the same answer read a moment later: nothing
/// was written down, so nothing will be carried on. See [`crate::carrying`].
async fn carries_on(state: &AppState, id: i64) {
    let written = match store::the_last_session(&state.pool, id).await {
        Ok(Some(continued)) => {
            match store::continue_on_arrival(&state.pool, id, &continued).await {
                Ok(()) => true,

                Err(error) => {
                    tracing::error!(error = ?error, conversation_id = id, "recording the conversation the arriving work was part way through failed, so the session started here will be re-primed instead");
                    false
                }
            }
        }

        Ok(None) => false,

        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading the newest session of an arriving Conversation failed, so the session started here will be re-primed instead");
            false
        }
    };

    if !written {
        crate::carrying::instead(
            &state.pool,
            &state.nudges,
            id,
            &crate::carrying::Instead::NoSession,
        )
        .await;
    }
}

/// Which of this device's Agent Profiles each one the record names is, where
/// this device has one at all.
///
/// [`resolved`]'s reading over a whole record rather than over three Pairings,
/// and with the missing ones left out rather than refused: what names a Profile
/// here is a Steer somebody ran a year ago, and an account deleted since is a
/// name no machine can keep. The column lands empty and the rest of the Steer
/// stands — see `store::slices`.
async fn accounts(state: &AppState, record: &RecordAcross) -> Result<HashMap<i64, i64>, Response> {
    if record.profiles.is_empty() {
        return Ok(HashMap::new());
    }

    let profiles = match store::profiles(&state.pool).await {
        Ok(profiles) => profiles,
        Err(why) => {
            tracing::error!(error = ?why, "this device's Profiles could not be read for an arriving record");

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device's Agent Profiles could not be read\n",
            )
                .into_response());
        }
    };

    Ok(record
        .profiles
        .iter()
        .filter_map(|(there, named)| Some((*there, here(&profiles, &state.device, named)?)))
        .collect())
}

/// The attached files, written into this device's own directory for this
/// Conversation.
///
/// Under the id this device gave it, which is what makes the path one its own
/// sessions are given — see [`crate::attachments`]. Each keeps the name the row
/// says it was stored under: the directory is one nothing has written to yet, so
/// nothing is counted up and the row and the file agree.
///
/// Blocking, off the runtime's threads: an attachment is as much as thirty-two
/// megabytes.
async fn kept(state: &AppState, id: i64, files: &[AttachedFile]) -> anyhow::Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let directories = Attachments::under(&state.data_dir);
    let files: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|file| {
            Ok((
                file.name.clone(),
                STANDARD.decode(&file.bytes).map_err(|why| {
                    anyhow::anyhow!("the bytes of {name} did not read: {why}", name = file.name)
                })?,
            ))
        })
        .collect::<anyhow::Result<_>>()?;

    tokio::task::spawn_blocking(move || {
        for (name, bytes) in files {
            directories.keep(id, &name, &bytes)?;
        }

        anyhow::Ok(())
    })
    .await?
}

/// `DELETE /api/peer/v1/transfers/{id}` — the device that sent this copy taking
/// it back, its move having failed before it was finished.
///
/// **Whatever state it is in, and without the archive behind it** — see
/// [`store::sweep_arrival`], where that is argued. A copy that never finished
/// arriving was never the human's to look at, and the alternative to sweeping it
/// is a Conversation nobody can account for.
///
/// **Except a copy that was here before the move began**, which is a transfer
/// back whose move fell over: what the sender is taking back is the arriving
/// record, and the row under it is this device's own from the first time the work
/// was here — the id every link anybody kept still names. So it keeps the row and
/// goes on wearing the mark it never took off, which leaves exactly the tombstone
/// it was before the return started: its URL leads to the machine still doing the
/// work, and the Merged List draws that machine's copy. What tells the two cases
/// apart is the mark — a copy that has only ever been arriving never wore one.
///
/// An id naming nothing answers as well as one naming something: the sender is
/// asking for this not to be here, and it is not here.
pub(crate) async fn sweep(State(state): State<AppState>, Path(id): Path<i64>) -> Response {
    match store::transferred(&state.pool, id).await {
        Ok(Some(live)) => {
            tracing::info!(
                conversation_id = id,
                device = live.device,
                "a member took back a Conversation it was moving back here, so the copy this \
                 device kept stays as the tombstone it was",
            );

            state
                .nudges
                .announce(verkstead_schema::Nudge::Conversations);

            return StatusCode::OK.into_response();
        }

        Ok(None) => {}

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "reading whether a half-transferred Conversation was a copy this device already held failed");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not tell what it holds of that Conversation\n",
            )
                .into_response();
        }
    }

    // Where its checkout went, read before the rows that name it are taken away:
    // a directory nothing points at is one nobody would ever find again. Nothing
    // is there for a copy swept before the git leg ran, which is most of them.
    let checkout = match store::closable(&state.pool, id).await {
        Ok(checkout) => checkout,

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "reading what a half-transferred Conversation had checked out failed");

            None
        }
    };

    match store::sweep_arrival(&state.pool, id).await {
        Ok(swept) => {
            tracing::info!(
                conversation_id = id,
                outcome = ?swept,
                "a member took back a Conversation whose transfer here failed",
            );

            // And the files that came with it, which are bytes rather than rows
            // and so are no part of the walk above. A copy that never finished
            // arriving was never the human's to look at, and a directory of
            // somebody else's attachments under an id nothing names is exactly
            // the leftover this sweep is for.
            Attachments::under(&state.data_dir).remove(id);

            // And the checkouts, for that reason again: a Worktree is a directory
            // this device made for work that turned out never to have arrived.
            // The Companions' among them, each being as much this device's making
            // as the Conversation's own. The branches stay — they are the work
            // itself, and a bundle that landed is history this repository now has
            // whatever became of the move.
            if let Some(checkout) = checkout {
                let mut removing: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

                if let Some(worktree) = checkout.worktree {
                    removing.push((checkout.repo, worktree));
                }

                for companion in checkout.companions {
                    if let Some(worktree) = companion.worktree {
                        removing.push((companion.repo, worktree));
                    }
                }

                let _ = tokio::task::spawn_blocking(move || {
                    for (repo, worktree) in removing {
                        crate::worktrees::remove(&repo, &worktree);
                    }
                })
                .await;
            }

            state
                .nudges
                .announce(verkstead_schema::Nudge::Conversations);

            StatusCode::OK.into_response()
        }

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "a half-transferred Conversation could not be swept");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the Conversation could not be swept from this device\n",
            )
                .into_response()
        }
    }
}

/// The three roles, each read into an id of this device's own.
///
/// `Err` is the refusal to answer with, naming the Profile that has no row here:
/// the sending device picked the pair off its own rows, and a mirror it holds
/// that this device does not is a cluster that has not finished agreeing with
/// itself.
async fn resolved(
    state: &AppState,
    arriving: &ConversationAcross,
) -> Result<
    (
        store::ArrivingPicked,
        store::ArrivingPicked,
        store::ArrivingPicked,
    ),
    Response,
> {
    // One read of the registry for all three, rather than a lookup apiece: a
    // Conversation names three Pairings and they are very often one account.
    let profiles = match store::profiles(&state.pool).await {
        Ok(profiles) => profiles,
        Err(why) => {
            tracing::error!(error = ?why, "this device's Profiles could not be read for an arriving Conversation");

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device's Agent Profiles could not be read\n",
            )
                .into_response());
        }
    };

    let mut read = [
        store::ArrivingPicked::Nothing,
        store::ArrivingPicked::Nothing,
        store::ArrivingPicked::Nothing,
    ];

    for (at, picked) in [
        &arriving.grilling,
        &arriving.implementation,
        &arriving.review,
    ]
    .into_iter()
    .enumerate()
    {
        read[at] = match picked {
            PickedAcross::Nothing => store::ArrivingPicked::Nothing,
            PickedAcross::Skipped => store::ArrivingPicked::Skipped,

            PickedAcross::Under(pairing) => {
                let Some(profile_id) = here(&profiles, &state.device, &pairing.profile) else {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "this device holds no Agent Profile for the one at home on {device} \
                             as {id}\n",
                            device = pairing.profile.device,
                            id = pairing.profile.id,
                        ),
                    )
                        .into_response());
                };

                store::ArrivingPicked::Under {
                    profile_id,
                    model: pairing.model.clone(),
                }
            }
        };
    }

    let [grilling, implementation, review] = read;

    Ok((grilling, implementation, review))
}

/// Which row of this device's `profiles` is the Profile a transfer names, where
/// one is.
///
/// Two ways to be it, and they are the same fact read from the two sides: the
/// account is at home **here**, in which case the pair names this device and one
/// of its own rows; or it is at home on some other machine, in which case what
/// this device holds is a **mirror** of it, marked with that machine and the id
/// it has there — see [`crate::mirroring`].
///
/// One of this device's own rows is never matched by the mirror arm and a mirror
/// is never matched by the first, which is what keeps a Profile from being read
/// as its own mirror on the machine it is at home on.
fn here(profiles: &[store::Profile], device: &str, named: &ProfileAcross) -> Option<i64> {
    profiles
        .iter()
        .find(|profile| match &profile.mirror {
            None => named.device == device && named.id == profile.id,
            Some(mirror) => mirror.device == named.device && mirror.id == named.id,
        })
        .map(|profile| profile.id)
}

/// The routes, over the state the workbench answers out of — mounted by
/// [`super::workbench::served`], where the Repos and the harnesses beside them
/// are mounted and for their reason: what they write is *this* device's record.
pub(crate) fn route() -> Router<AppState> {
    Router::new()
        .route(TRANSFERS, post(take))
        .route(ONE_TRANSFER, delete(sweep))
        .route(
            ONE_TRANSFERS_RECORD,
            // Over the router's own default, the way the Question Set route and
            // the memory store's raise theirs: a record is a Conversation's
            // whole Timeline, and the bound over it is the record's own.
            post(written).layer(DefaultBodyLimit::max(MOST_A_RECORD_IS)),
        )
        .route(ONE_TRANSFERS_ARRIVAL, post(arrived))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It is a member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for: a stranger does not
    /// write Conversations into somebody's database.
    #[test]
    fn the_transfers_are_not_one_of_the_un_gated_three() {
        for path in [
            TRANSFERS,
            ONE_TRANSFER,
            ONE_TRANSFERS_RECORD,
            ONE_TRANSFERS_ARRIVAL,
        ] {
            assert_ne!(path, super::super::IDENTITY);
            assert_ne!(path, super::super::joining::JOIN);
            assert_ne!(path, super::super::exchange::SETTLED);
        }
    }

    /// And outside the three prefixes this device keeps to itself, which a
    /// route served on the one namespace a member never reaches would be
    /// nothing at all.
    #[test]
    fn the_transfers_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            for path in [
                TRANSFERS,
                ONE_TRANSFER,
                ONE_TRANSFERS_RECORD,
                ONE_TRANSFERS_ARRIVAL,
            ] {
                assert!(
                    !path.starts_with(prefix),
                    "{path} would be held back with {prefix}/",
                );
            }
        }
    }
}
