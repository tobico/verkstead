//! The viewer's own JSON namespace: everything the human's browser asks of the
//! server, and nothing an agent ever calls.
//!
//! Kept under `/api/ui/` rather than mixed into `/api/v1/` because the two are
//! different promises. The agent contract is versioned and public to whatever is
//! installed out there, so it changes only by adding; this namespace is private
//! to the viewer that ships in the same binary, and the two of them may be
//! rearranged together whenever it suits.
//!
//! What crosses it is JSON, where the agents' half speaks YAML: the agents' side
//! is read and written by humans in a terminal, and this side is read by a
//! browser. Everything the agent wrote arrives already rendered to sanitized HTML
//! — see [`verkstead_render`] — so the viewer needs no markdown parser, no diff
//! highlighter and no sanitizer of its own.
//!
//! The two mutations answer 200 with a named outcome rather than a status code,
//! including for the Set that has gone: every one of those outcomes is something
//! the viewer has to say to the human in words, and a Set answered from another
//! device is not an error to be retried. What does get a status code is a Set
//! that cannot be read at all — a 404, because that is a page the viewer draws
//! differently.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as HttpResponse};
use axum::routing::{get, post};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use verkstead_render::{
    Adopted, Author, BaseBranchChoice, BranchRename, BriefEdit, CheckRollup, CompanionAdded,
    CompanionBaseRecorded, CompanionBranchRenamed, CompanionMode, CompanionModeChoice,
    CompanionModeChosen, CompanionRemoved, CompanionView, ConversationArchived, ConversationClosed,
    ConversationEntry, ConversationSteered, ConversationStopped, ConversationUnarchived,
    ConversationView, Cursor, GrillingStarted, Lifecycle, Locked, NewAdoption, NewCompanion,
    NewConversation, NewOrder, ProfileChoice, ProfileEdit, ProfileEntry, PushKey, Registration,
    RepoEntry, Resumed, SetReading, SetView, SettingsEdit, SettingsSaved, SettingsView,
    ShowingArchived, Standing, SteerOpened, SteerSubmission, Submitted, Subscribed, Subscription,
    TokenEdit, TokenSaved, UnreadableSet, Unsubscribe, UpdateNotice, Verified,
};
use verkstead_schema::{ApiError, Nudge, Response};

use crate::settings::{Config, GitAuthor, Secrets};
use crate::{AppState, store};

/// The viewer's routes, over the state the agent API is already holding: a
/// submit from the browser has to reach an agent waiting on the REST endpoint,
/// so both halves settle Sets through the same channel and read Liveness out of
/// the same registry of held waits.
pub(crate) fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/ui/sets/{id}", get(set))
        .route("/api/ui/sets/{id}/response", post(submit_response))
        .route("/api/ui/sets/{id}/lock", post(lock_set))
        .route("/api/ui/repos", get(repos).post(register_repo))
        // What one Repo's branches are, which is what a drafting Conversation
        // picks the one it comes off out of. Under the Repo rather than under
        // the Conversation: the branches are the repository's, and two
        // Conversations against one Repo are looking at the same list.
        .route("/api/ui/repos/{id}/branches", get(branches))
        .route(
            "/api/ui/conversations",
            get(conversations).post(start_conversation),
        )
        // The order the human dragged that list into. A path of its own under
        // the list rather than a field on anything in it: what it says is about
        // the sidebar rather than about any one Conversation, and the whole
        // order is what a drag produces.
        .route("/api/ui/conversations/order", post(place_conversations))
        // And whether that list is drawing what has been archived, which is
        // about the sidebar in exactly the same way — the human's standing
        // choice rather than this device's, so it is read back here on every
        // load rather than kept where the toggle was flipped.
        .route(
            "/api/ui/conversations/archived",
            get(showing_archived).post(show_archived),
        )
        // The roadmaps in the registered Repos that nothing is driving, drawn
        // as a notice under the new-conversation box. Beside the Conversations
        // rather than under a Repo, because that is where it is read: what it
        // offers is another way to start work.
        .route("/api/ui/abandoned-roadmaps", get(abandoned_roadmaps))
        // And starting one to adopt a roadmap with, which is what clicking a
        // roadmap in that notice does. Its own endpoint rather than a field on
        // the one above: adopting is the other way into the pipeline, and what
        // it starts is a Conversation with no Brief to write.
        .route("/api/ui/adoptions", post(start_adoption))
        .route("/api/ui/conversations/{id}", get(conversation))
        // One Event's full self, fetched by the pane that shows it rather than
        // carried by the Conversation — see [`capture`].
        .route("/api/ui/conversations/{id}/capture/{event}", get(capture))
        // And the same session's own record of what it said, which is what that
        // pane draws wherever there is one — see [`transcript`].
        .route(
            "/api/ui/conversations/{id}/transcript/{event}",
            get(transcript),
        )
        // And how it looked while it was saying it: the grid those bytes leave
        // on a terminal — see [`screen`].
        .route("/api/ui/conversations/{id}/screen/{event}", get(screen))
        // And the same Screen watched as it is drawn, where the session is still
        // running: the one socket in the codebase — see
        // [`crate::screen::attach`]. Beside the fetch rather than instead of it,
        // because a session that has ended has no socket and its last screen is
        // still worth showing.
        .route(
            "/api/ui/conversations/{id}/screen/{event}/attach",
            get(crate::screen::attach),
        )
        // And one commit — its summary and its diff — fetched the same way and
        // for the same reason; see [`commit_pane`].
        .route(
            "/api/ui/conversations/{id}/commit/{event}",
            get(commit_pane),
        )
        // And the backlog opened, which is every task document `.tasks/` holds.
        // Named by the Conversation alone: a backlog is read off the Worktree
        // rather than remembered, so there is no Event id to reach it by — see
        // [`backlog`].
        .route("/api/ui/conversations/{id}/backlog", get(backlog))
        // And the roadmap opened, which is every stage brief one of them holds.
        // Named by the roadmap rather than by the Conversation, which is the one
        // place this parts company with the backlog above: a Worktree holds one
        // `.tasks/` and may hold any number of roadmaps — see [`roadmap`].
        .route("/api/ui/conversations/{id}/roadmap/{name}", get(roadmap))
        // And what is on the pull request the finish step opened, fetched by the
        // pane that shows it — see [`pull_request`]. Fetched rather than
        // remembered, and the reason is stronger here than for either of the two
        // above: reading it is asking GitHub over the network.
        .route(
            "/api/ui/conversations/{id}/pull-request/{event}",
            get(pull_request),
        )
        .route("/api/ui/conversations/{id}/brief", post(save_brief))
        .route("/api/ui/conversations/{id}/branch", post(rename_branch))
        .route("/api/ui/conversations/{id}/base", post(set_base_branch))
        // And the other registered Repos the work runs alongside, added and
        // taken away on the same card and for as long as the same card is
        // drawn. Named in the path rather than in the verb, as everything
        // around it is: the viewer speaks one method, so taking one away is a
        // route rather than a `DELETE`.
        .route("/api/ui/conversations/{id}/companions", post(add_companion))
        .route(
            "/api/ui/conversations/{id}/companions/{repo}/remove",
            post(remove_companion),
        )
        // And what each of those rows is configured with — the same three
        // things the Conversation's own branch row settles, about a companion
        // instead: how far in the work may reach, what its checkout comes off,
        // and what its branch is called.
        .route(
            "/api/ui/conversations/{id}/companions/{repo}/mode",
            post(set_companion_mode),
        )
        .route(
            "/api/ui/conversations/{id}/companions/{repo}/base",
            post(set_companion_base),
        )
        .route(
            "/api/ui/conversations/{id}/companions/{repo}/branch",
            post(rename_companion_branch),
        )
        // The two that make and unmake what a Conversation works in. Named in
        // the path rather than in the verb, as closing a Set unanswered is: the
        // viewer speaks one method. Nothing here opens a second round on one
        // Verkstead has finished with: a steer into Grilling is that, and it
        // goes through the modal below like every other steer.
        .route("/api/ui/conversations/{id}/grill", post(start_grilling))
        // And the press that adopts a roadmap's next stage, which is the
        // grilling start's sibling: what the human presses on an adopting
        // Conversation, there being no Brief to write and no grilling to run.
        .route("/api/ui/conversations/{id}/adopt", post(adopt))
        .route("/api/ui/conversations/{id}/close", post(close))
        // And the two of those joined, which is one row of the menu rather than
        // two pressed in turn: the close and the archive are one intention often
        // enough to be worth a press of their own.
        .route(
            "/api/ui/conversations/{id}/close-and-archive",
            post(close_and_archive),
        )
        // And the one that puts a closed Conversation away, which is the row
        // beside Close in the same menu. Named in the path like everything
        // around it, and with no body for the same reason: which Conversation
        // it is is the whole of what it says.
        .route("/api/ui/conversations/{id}/archive", post(archive))
        // And the way back out of it, which is the same row saying the other
        // word: archiving is reversible, and this is what reverses it.
        .route("/api/ui/conversations/{id}/unarchive", post(unarchive))
        // And the browser saying the human has now looked at one, which takes
        // the mark off the sidebar row. A press of its own rather than
        // something the read of the Conversation does on the way past: a GET
        // that wrote would be a GET a retry or a prefetch could spend, and what
        // is being recorded is a person having looked rather than a page having
        // fetched.
        .route("/api/ui/conversations/{id}/seen", post(seen))
        // No route for how the work gets built: the direction rides the closing
        // Question Set, and answering one is answering a Set — see
        // [`store::submit_response`].
        //
        // And no route for a run waiting an account's window out. That was a
        // press per Event, on the one card that carried its own; a run stopped
        // for a window is stopped like everything else now, and what starts it
        // again is the one Resume below.
        //
        // And no route for the one thing the human used to set going by hand
        // beside the work: a steer into Implementing carries the instruction
        // now, and the session it starts drives the Conversation rather than
        // standing next to it — see [`steer`].
        //
        // What there is is the press that gets a stopped Conversation going
        // again: what Verkstead itself should be doing, worked out again from
        // where the work now stands. Per Conversation rather than per Event, and
        // with no body at all — there is nothing to say about it beyond which
        // Conversation it is.
        .route("/api/ui/conversations/{id}/resume", post(resume))
        // And the two presses that stop it, which are Resume's opposite number
        // and take a body for the same reason it does: none. Which Conversation
        // it is is the whole of what either says, and which press it was is the
        // route — a Stop that waits for the step it is on, and one that does
        // not.
        .route("/api/ui/conversations/{id}/stop", post(stop))
        .route("/api/ui/conversations/{id}/force-stop", post(force_stop))
        // And the two presses that steer it, which are the row beside those in
        // the same menu. Two rather than one because the click is an act of its
        // own: it stops the drive so that nothing launches while the human
        // composes, and answers with what it found running — see
        // [`crate::steering`]. The submit under it carries what the modal
        // settled, which is the only body of the four.
        .route("/api/ui/conversations/{id}/steer", post(steer))
        .route(
            "/api/ui/conversations/{id}/steer/submit",
            post(steer_submit),
        )
        .route(
            "/api/ui/conversations/{id}/grilling-pairing",
            post(choose_grilling_pairing),
        )
        .route(
            "/api/ui/conversations/{id}/implementation-pairing",
            post(choose_implementation_pairing),
        )
        .route("/api/ui/profiles", get(profiles).post(create_profile))
        .route("/api/ui/profiles/{id}", post(edit_profile))
        // Removing is a POST to a route of its own, as closing a Set unanswered
        // is: the viewer speaks one method, and the thing being done is named in
        // the path rather than in the verb.
        .route("/api/ui/profiles/{id}/delete", post(delete_profile))
        // Not a thing to fetch but a thing to listen on — see [`crate::nudge`].
        .route("/api/ui/nudges", get(crate::nudge::nudges))
        .route("/api/ui/push/key", get(push_key))
        .route("/api/ui/push/subscribe", post(subscribe))
        .route("/api/ui/push/unsubscribe", post(unsubscribe))
        // What Verkstead has been told, and telling it. One route for both, the
        // read and the save being the same page's two halves — and one save for
        // the author and the token together, because the page has one button.
        .route("/api/ui/settings", get(settings).post(save_settings))
        .route("/api/ui/update", get(update))
}

/// `GET /api/ui/sets/{id}` — one Set, rendered, with where it stands.
///
/// Where it stands travels with it rather than being asked for afterwards:
/// it decides whether the viewer draws a form or a record, and a Set answered
/// days ago must not flash a form first.
async fn set(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    // An id that is not a number cannot name a Set, so it gets the same answer
    // as one that names no Set: there isn't one. The viewer's own routes are
    // this permissive too — the id comes out of a URL the human may have typed.
    let Ok(id) = id.parse::<i64>() else {
        return no_such_set(&id);
    };

    let stored = match store::load_set(&state.pool, id).await {
        Ok(Some(stored)) => stored,
        Ok(None) => return no_such_set(&id.to_string()),
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "loading a Question Set failed");
            return unavailable("the Question Set could not be read");
        }
    };

    let settlement = match store::settlement(&state.pool, id).await {
        Ok(settlement) => settlement,
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "reading how a Set stands failed");
            return unavailable("the Question Set could not be read");
        }
    };

    // Which Conversation it was asked from, which is where this page leads back
    // to. A stored Set always has one — it and its Timeline Event are written in
    // the one transaction — so a Set without one is a record this server cannot
    // make a whole page out of rather than a Set that is simply somewhere else.
    let conversation = match store::asked_from(&state.pool, id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(set_id = id, "a stored Question Set is on no Timeline");
            return unavailable("the Question Set could not be read");
        }
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "reading which Conversation a Set was asked from failed");
            return unavailable("the Question Set could not be read");
        }
    };

    // A stored body this build's schema will not take is a page all the same,
    // and the same page whether it is reached by its own URL or opened from the
    // Timeline: the record, said to be unreadable, with the body itself under
    // it. Not a failure — the Set is there, and this is what there is of it.
    let set = match stored.set {
        store::Asked::Set(set) => set,
        store::Asked::Unreadable(unreadable) => {
            tracing::warn!(
                set_id = id,
                why = unreadable.why,
                "a stored Question Set cannot be read"
            );

            return Json(SetReading::Unreadable(UnreadableSet {
                id: stored.id,
                conversation,
                body: unreadable.body,
                why: unreadable.why,
            }))
            .into_response();
        }
    };

    let standing = standing(
        &state,
        id,
        settlement,
        stored.deferred,
        &stored.created_at,
        OffsetDateTime::now_utc(),
    );

    // Whether the closing section carries the Nothing-else option, which is a
    // fact about the Conversation rather than about the Set: a follow-up's
    // rounds are ordinary Sets, and what makes one a follow-up's is where the
    // work stands while it is being answered. A Conversation that cannot be read
    // draws no option, which is what every state but Follow-up gets anyway.
    let follow_up = match store::state(&state.pool, conversation).await {
        Ok(state) => state == Some(store::Lifecycle::FollowUp),
        Err(error) => {
            tracing::error!(
                error = ?error,
                conversation,
                "reading where a Set's Conversation stands failed"
            );
            false
        }
    };

    // Everything the agent wrote, rendered — which is the whole of what is left
    // to do, and none of it this crate's.
    //
    // On the blocking pool, for the Diff inside it: a Set is asked with the whole
    // of a dirty working tree attached, and parsing and colouring that is not
    // work to do on an async worker thread while other requests wait behind it.
    let set_id = stored.id;
    let view = tokio::task::spawn_blocking(move || {
        verkstead_render::set_view(set_id, conversation, set, standing, follow_up)
    })
    .await;

    let view: SetView = match view {
        Ok(view) => view,
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "rendering a Question Set failed");
            return unavailable("the Question Set could not be read");
        }
    };

    Json(SetReading::Set(Box::new(view))).into_response()
}

/// Where a Set stands, as both its own page and its row on a Timeline read it.
///
/// The Liveness comes out of the registry of held waits, which is the same
/// registry either way: whichever of the two the human is looking at, it is the
/// page they act on.
///
/// Except for a Deferred Ask, which the registry has nothing to say about: no
/// wait was ever held on one, so ageing it against the clock would report an
/// agent that had gone where none was ever there. `deferred` is what the record
/// says about how it was asked, and it is the whole verdict where it is true.
fn standing(
    state: &AppState,
    set_id: i64,
    settlement: Option<store::Settlement>,
    deferred: bool,
    created_at: &str,
    now: OffsetDateTime,
) -> Standing {
    match settlement {
        Some(store::Settlement::Answered(answered)) => {
            Standing::Answered(verkstead_render::Answered {
                submitted_at: answered.submitted_at,
                response: answered.response,
            })
        }
        Some(store::Settlement::LockedUnanswered(locked)) => {
            Standing::LockedUnanswered(locked.locked_at)
        }
        None if deferred => Standing::Waiting(verkstead_schema::Liveness::Deferred),
        None => Standing::Waiting(state.waits.liveness(set_id, created_at, now)),
    }
}

/// `POST /api/ui/sets/{id}/response` — answer a Set.
///
/// Goes through the same store call the agent-facing endpoint does, so a submit
/// from the browser wakes a waiting agent exactly as `curl` would.
async fn submit_response(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(response): Json<Response>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(Submitted::NoSuchSet).into_response();
    };

    let submission =
        match store::submit_response(&state.pool, &state.settlements, id, &response).await {
            Ok(submission) => submission,
            Err(error) => {
                tracing::error!(error = ?error, set_id = id, "taking a Response failed");
                return unavailable("the Response could not be taken");
            }
        };

    Json(match submission {
        store::Submission::Accepted(taken) => {
            crate::conversations::settle_a_proposal(&state, id, taken.proposed).await;

            Submitted::Accepted
        }
        store::Submission::AlreadyAnswered => Submitted::AlreadyAnswered,
        store::Submission::NoSuchSet => Submitted::NoSuchSet,
        store::Submission::Locked => Submitted::Locked,
        store::Submission::Invalid(invalid) => {
            Submitted::Rejected(invalid.violations.iter().map(ToString::to_string).collect())
        }
    })
    .into_response()
}

/// `POST /api/ui/sets/{id}/lock` — close a Set unanswered.
///
/// The human declaring that nobody is ever going to answer it, so it stops being
/// something that is waiting on them. Only ever reached from a browser
/// (ADR-0001) — the agent API has no route for it, because a disconnected agent
/// is not evidence: the CLI reconnects through transient drops.
async fn lock_set(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(Locked::NoSuchSet).into_response();
    };

    let locking = match store::lock_set(&state.pool, &state.settlements, id).await {
        Ok(locking) => locking,
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "locking a Set failed");
            return unavailable("the Question Set could not be locked");
        }
    };

    Json(match locking {
        store::Locking::Locked(_) => Locked::Closed,
        store::Locking::AlreadyAnswered => Locked::AlreadyAnswered,
        store::Locking::AlreadyLocked => Locked::AlreadyLocked,
        store::Locking::NoSuchSet => Locked::NoSuchSet,
    })
    .into_response()
}

/// `GET /api/ui/repos` — the Repos Verkstead has been told about, by name.
async fn repos(State(state): State<AppState>) -> HttpResponse {
    let repos = match store::registered_repos(&state.pool).await {
        Ok(repos) => repos,
        Err(error) => {
            tracing::error!(error = ?error, "reading the registered Repos failed");
            return unavailable("the registered Repos could not be read");
        }
    };

    let rows: Vec<RepoEntry> = repos
        .into_iter()
        .map(|repo| RepoEntry {
            id: repo.id,
            name: repo.name,
            // Stored as UTF-8 in the first place — a path that is not cannot be
            // registered — so nothing is lost putting it back on the wire.
            path: repo.path.to_string_lossy().into_owned(),
            default_branch: repo.default_branch,
        })
        .collect();

    Json(rows).into_response()
}

/// `GET /api/ui/repos/{id}/branches` — every branch of one registered Repo,
/// local and remote-tracking, for the dropdown that picks what the work comes
/// off.
///
/// Read out of git every time rather than kept anywhere: branches are the
/// repository's own and move without Verkstead hearing about it, so a stored
/// list would be one more thing to be wrong.
async fn branches(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return no_such_repo(&id);
    };

    match crate::repos::branches(&state.pool, id).await {
        Ok(Some(branches)) => Json(branches).into_response(),
        Ok(None) => no_such_repo(&id.to_string()),
        Err(error) => {
            tracing::error!(error = ?error, repo_id = id, "listing a Repo's branches failed");
            unavailable("the Repo's branches could not be read")
        }
    }
}

/// `POST /api/ui/repos` — take on the repository at a path.
///
/// Every refusal is the server's: the Watched Paths are a security boundary, and
/// a boundary a request could reach around by not going through the form would
/// not be one. See [`crate::repos`] for what is checked.
async fn register_repo(
    State(state): State<AppState>,
    Json(registration): Json<Registration>,
) -> HttpResponse {
    match crate::repos::register(&state.pool, &state.watched, &registration.path).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "registering a Repo failed");
            unavailable("the Repo could not be registered")
        }
    }
}

/// `GET /api/ui/abandoned-roadmaps` — the registered Repos holding roadmaps
/// nothing is driving, one notice each.
///
/// Read from the repositories every time it is asked for, like the pinned task
/// and stage lists: the boxes and the branches are the repository's own answer
/// about its roadmaps, and a list Verkstead kept would be a second opinion that
/// went wrong the moment somebody ticked a box.
///
/// A Repo that cannot be read contributes nothing rather than failing the list.
/// What this decides is whether to draw a notice, and a git that was briefly
/// busy is no reason to say a roadmap has been abandoned.
async fn abandoned_roadmaps(State(state): State<AppState>) -> HttpResponse {
    let repos = match store::registered_repos(&state.pool).await {
        Ok(repos) => repos,
        Err(error) => {
            tracing::error!(error = ?error, "reading the registered Repos failed");
            return unavailable("the registered Repos could not be read");
        }
    };

    Json(crate::stages::abandoned(repos).await).into_response()
}

/// `GET /api/ui/conversations` — the sidebar, newest first.
///
/// Three facts ride out on every row beyond what the store holds: whether a
/// session is running on it, whether that session has gone quiet, and whether
/// it is waiting on the human. All three are read here at the moment the list
/// is drawn, and none of them is stored — a running session is a process this
/// server holds, how long it has been silent is a clock on that process, and
/// what is waiting is an `OR` the store computes over rows that move on their
/// own. Which mark they come out as is the viewer's, and the rule there is one
/// line: waiting wins over both of the others.
async fn conversations(State(state): State<AppState>) -> HttpResponse {
    let conversations = match store::conversations(&state.pool).await {
        Ok(conversations) => conversations,
        Err(error) => {
            tracing::error!(error = ?error, "reading the Conversations failed");
            return unavailable("the Conversations could not be read");
        }
    };

    // Read once for the whole list rather than per row: which Conversations are
    // running is one lock away, and asking it per row would take that lock as
    // many times as there are Conversations for an answer that cannot change
    // between them any more meaningfully than it changes between reads.
    let working = state.sessions.working();

    // And which of those have gone quiet, which is the other half of what the
    // card's mark says. A second read of the same register rather than one
    // answer: `working` is what the whole sidebar is drawn from and this is a
    // fact about the few rows in it.
    let quiet = state.sessions.quiet();

    let rows: Vec<ConversationEntry> = conversations
        .into_iter()
        .map(|conversation| {
            let working = working.contains(&conversation.id);

            ConversationEntry {
                id: conversation.id,
                branch: conversation.branch,
                repo: conversation.repo,
                state: row_state(conversation.id, conversation.state),
                working,
                // Idle is a thing a running session is, and the two sets are
                // read a moment apart — so the pair is made consistent here
                // rather than left to the page that draws it.
                idle: working && quiet.contains(&conversation.id),
                waiting: conversation.waiting,
                // And the same pairing again for the wrap-up that has got down
                // to its checks: the settle facts came out of the query above,
                // and whether anything is running on it is this register's to
                // say. A fix session working a red check draws as plain
                // Wrapping — waiting is what a wrap-up with nobody in it does.
                waiting_on_checks: conversation.narrowed_to_checks && !working,
                // And whether Verkstead has told the human something about it
                // they have not looked at yet, which is the store's alone: it is
                // written down rather than read off anything here, being a fact
                // about the person rather than about the work.
                unseen: conversation.unseen,
            }
        })
        .collect();

    Json(rows).into_response()
}

/// `POST /api/ui/conversations` — start one against a registered Repo.
async fn start_conversation(
    State(state): State<AppState>,
    Json(new): Json<NewConversation>,
) -> HttpResponse {
    match crate::conversations::start(&state, new.repo_id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "starting a Conversation failed");
            unavailable("the Conversation could not be started")
        }
    }
}

/// `POST /api/ui/conversations/order` — the sidebar, in the order the human just
/// dragged it into.
///
/// Refused for nothing. Every id is either a Conversation, which is placed, or
/// not one, which is passed over — a viewer sends the list it drew, and by the
/// time it lands a row may have been started or closed. There is nothing to
/// answer with beyond that it was taken, so it answers with nothing.
///
/// The Nudge is what carries it to the other devices: an order is the list
/// having moved, which is the one thing every open sidebar has to read again.
async fn place_conversations(
    State(state): State<AppState>,
    Json(placed): Json<NewOrder>,
) -> HttpResponse {
    match store::place_conversations(&state.pool, &placed.order).await {
        Ok(()) => {
            state.nudges.announce(Nudge::Conversations);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => {
            tracing::error!(error = ?error, "placing the Conversations failed");
            unavailable("the order could not be saved")
        }
    }
}

/// `POST /api/ui/adoptions` — start a Conversation to adopt a roadmap with.
///
/// What clicking a roadmap in the abandoned-roadmaps notice does. It records
/// and opens; nothing about the repository is touched, and nothing is adopted
/// until the human presses Adopt on the page this puts them on.
async fn start_adoption(
    State(state): State<AppState>,
    Json(new): Json<NewAdoption>,
) -> HttpResponse {
    match crate::conversations::start_adopting(&state, new.repo_id, &new.roadmap).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "starting a Conversation to adopt a roadmap failed");
            unavailable("the Conversation could not be started")
        }
    }
}

/// `GET /api/ui/conversations/{id}` — one Conversation with its Timeline.
///
/// The Timeline travels with it rather than being fetched beside it: it is what
/// the middle pane *is*, and a Conversation whose Timeline arrived a moment
/// later would draw an empty pane first every time one is opened.
async fn conversation(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    // An id that is not a number cannot name a Conversation, so it gets the same
    // answer as one that names none — the id comes out of a URL the human may
    // have typed.
    let Ok(id) = id.parse::<i64>() else {
        return no_such_conversation(&id);
    };

    let conversation = match store::load_conversation(&state.pool, id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return no_such_conversation(&id.to_string()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "loading a Conversation failed");
            return unavailable("the Conversation could not be read");
        }
    };

    // Which of this Conversation's output Events is still being written into,
    // which is a question about a process rather than about the record: a
    // restarted server has no sessions, and every Capture it holds is of one
    // that is over.
    //
    // Asked *before* the Timeline it is read against, and the order is the whole
    // of what makes the answer safe. A relay takes itself off this register only
    // once it has flushed the last of what it printed, so a session this says has
    // ended is one whose output is already in the store — and a Timeline read
    // after that is a Timeline with all of it. Read the other way round, a
    // session that finished in between would leave its Event drawn as stopped
    // with the Capture from before it flushed: `0 lines`, nothing printed,
    // nothing running, and a page saying the session never said anything when it
    // had.
    //
    // The other order costs nothing: an Event drawn as running that has just
    // stopped is right again on the next read, a second later.
    let writing = state.sessions.writing(id);

    // And whether that session has stopped printing, which is the other half of
    // what the mark on its row says. Read here beside the register above rather
    // than per Event: there is at most one session running on a Conversation,
    // so the answer cannot differ between the Events it is drawn against.
    let idling = state.sessions.idling(id);

    let timeline = match store::timeline(&state.pool, id).await {
        Ok(timeline) => timeline,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading a Timeline failed");
            return unavailable("the Conversation could not be read");
        }
    };

    // The two Pairings are read as rows rather than as ids: what the pane says
    // about a Profile, and whether it can still be run under, is the same
    // reading the Profile list gets.
    let grilling_pairing = match crate::profiles::pairing(
        &state.watched,
        conversation.grilling_pairing,
    )
    .await
    {
        Ok(pairing) => pairing,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading a grilling Pairing failed");
            return unavailable("the Conversation could not be read");
        }
    };

    let implementation_pairing = match crate::profiles::pairing(
        &state.watched,
        conversation.implementation_pairing,
    )
    .await
    {
        Ok(pairing) => pairing,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading an implementation Pairing failed");
            return unavailable("the Conversation could not be read");
        }
    };

    // What the worktree holds rather than what the record remembers: the
    // backlog, as `.tasks/` stands right now. Read off the filesystem for the
    // reason the worktree's own missing-ness is — the repository owns those
    // files, and a row remembering what they said would be one more thing to be
    // wrong.
    let backlog = crate::tasks::showing(conversation.worktree.clone()).await;

    // And the roadmap this branch is about, read the same way and for the same
    // reason — `docs/roadmaps/` is the repository's too. Which of a repository's
    // roadmaps is this one's is asked of git against the base commit: a
    // repository keeps its finished roadmaps, and a Conversation is about the
    // one its branch has written to. See [`crate::stages`].
    let roadmaps = crate::stages::showing(
        conversation.worktree.clone(),
        conversation.base_commit.clone(),
    )
    .await;

    // Each of those goes in two places — pinned above the record, and on the
    // record at the row that says it landed — and this is the one reading behind
    // both. The rows are stamped where the runner sees the landing; a
    // Conversation from before there were rows to stamp keeps the pinned card
    // alone, which is what it has always had.
    let mut pinned: Vec<verkstead_render::PinnedEvent> = backlog
        .clone()
        .map(verkstead_render::task_list_event)
        .into_iter()
        .collect();

    pinned.extend(
        roadmaps
            .iter()
            .cloned()
            .map(verkstead_render::stage_list_event),
    );

    // And how the pull request's checks were the last time anything asked, which
    // is the one thing about a pull request that is written down and moves. Read
    // once for the two cards drawn from it below, both being the one card in the
    // two places a pull request is drawn.
    //
    // The Conversation's own repository's pull request only. How the checks are
    // is written down per Conversation rather than per pull request, so it is the
    // one that moved this Conversation into Wrapping that it belongs to — see
    // [`own_checks`]. A companion's card draws no icon rather than this one's.
    //
    // Stale on a Conversation nothing is watching any more, the watcher stopping
    // when the wrap-up is over — which is a card an hour behind rather than a
    // card that is wrong: the last thing anybody asked GitHub is the honest
    // thing to draw.
    let checks = match store::check_rollup(&state.pool, id).await {
        Ok(checks) => checks.map(rollup),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading how a pull request's checks are failed");
            None
        }
    };

    // And every pull request the work ended up on, which are pinned beside them.
    // These *are* on the record — the Conversation's own repository's is what
    // moved the Conversation into Wrapping, and a companion's is that wrap-up
    // covering the repository it also committed in — so they are read off the
    // Timeline for the reason the Brief is: they are already here. All of them
    // rather than the last one found: a Conversation ends on one pull request per
    // repository it was worked in, and the human wraps up all of them at once.
    // What is not read here is what a PR holds, which is a request of its own;
    // see [`pull_request`].
    pinned.extend(timeline.iter().filter_map(|event| match &event.event {
        store::Event::PullRequest(opened) => Some(verkstead_render::pull_request_event(
            event.id,
            event.at.clone(),
            verkstead_render::PullRequestSummary {
                number: opened.number,
                title: opened.title.clone(),
                url: opened.url.clone(),
                repo: opened.repo.clone(),
                checks: own_checks(&opened.repo, checks),
            },
        )),
        _ => None,
    }));

    // Whether the worktree is still on disk, which is a look at the filesystem
    // rather than anything the store knows.
    let worktree = match crate::conversations::worktree(conversation.worktree.clone()).await {
        Ok(worktree) => worktree,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading a worktree failed");
            return unavailable("the Conversation could not be read");
        }
    };

    // And each companion's, which is the same look at the filesystem: a
    // Conversation with three companions has four checkouts, and every one of
    // them is a directory somebody could have deleted.
    let mut companions = Vec::new();

    for reading in conversation.companions {
        match companion(reading).await {
            Ok(companion) => companions.push(companion),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id = id, "reading a companion worktree failed");
                return unavailable("the Conversation could not be read");
            }
        }
    }

    // The Brief decides whether the Conversation is ready to grill, so it is
    // read off the Timeline before the Timeline is spent building the view. The
    // newest of them: a Conversation gets one Brief per round, and what a
    // grilling would start from is the round nobody has grilled yet.
    let brief = timeline
        .iter()
        .rev()
        .find_map(|event| match &event.event {
            store::Event::Brief(markdown) => Some(markdown.as_str()),
            _ => None,
        })
        .unwrap_or_default();

    let ready_to_grill = crate::conversations::ready_to_grill(
        conversation.state,
        grilling_pairing.as_ref(),
        implementation_pairing.as_ref(),
        brief,
    );

    // Which Brief is still being written, where one is. A Brief freezes when its
    // round's grilling starts, so the one open is the newest — and only while the
    // Conversation is drafting. An adopting Conversation's first Brief is nobody
    // here's to write at all: it is the stage brief, and it arrives when the stage
    // is adopted.
    let briefs: Vec<i64> = timeline
        .iter()
        .filter_map(|event| match &event.event {
            store::Event::Brief(_) => Some(event.id),
            _ => None,
        })
        .collect();

    let open_brief = (conversation.state == store::Lifecycle::Draft)
        .then(|| briefs.last().copied())
        .flatten()
        .filter(|open| !(conversation.adopting.is_some() && briefs.first() == Some(open)));

    // And what this Conversation is adopting, where it is adopting anything and
    // has not adopted it yet: the roadmap named and the stage the Adopt press
    // would start, read off the Repo at the base commit rather than out of any
    // row. Only the roadmap's name is stored — see [`crate::stages::adopting`].
    //
    // A worktree is what says the adoption has happened, adoption being what
    // makes one. What follows it is the stage's work and, if the human steers it
    // into a second round when that work is done, a Brief of their own — never
    // the stage brief again.
    let adopting = match conversation.adopting.clone() {
        Some(roadmap) if worktree.is_none() => Some(
            crate::stages::adopting(
                conversation.repo.clone(),
                conversation.base_commit.clone(),
                roadmap,
            )
            .await,
        ),
        _ => None,
    };

    // Whether driving has stopped, however it stopped: the stop says the
    // Conversation is stopped now, and the Notice it points at says what stopped
    // and why. One question about one thing — an account out of window stops a
    // run the same way a session falling over does.
    let stopped = match store::stopped(&state.pool, id).await {
        Ok(stopped) => stopped,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading whether a Conversation had stopped failed");
            None
        }
    };

    // And whether there is driving to start again, which is the same kind of
    // fact about the other end of the ladder: the Conversation says it is being
    // worked on and nothing is working on it. Asked of the running server as
    // much as of the record — see [`crate::resume::ready`].
    let ready_to_resume = crate::resume::ready(&state, id, conversation.state, stopped.is_some());

    // And whether there is driving to stop, which is the same fact read the
    // other way: a Conversation that says it is being worked on and has not
    // stopped is one the human may pull the brake on. Nothing about the register
    // here — a run between two steps is as much a run to stop as a busy one. See
    // [`crate::stops::ready`].
    let ready_to_stop = crate::stops::ready(conversation.state, stopped.is_some());

    // And whether the press has already been made and is waiting for the step
    // the run is on to finish, which is what takes Stop off the menu: the
    // decision is recorded, and asking for it again is Verkstead asking for one
    // it has. Force stop is drawn on `ready_to_stop` alone, being the escalation
    // from here rather than the same press repeated.
    //
    // A read that fails reads as *not asked*, which is the way round that leaves
    // the press offered: a menu short of a row the human wanted is worse than
    // one carrying a row that answers `Stopping` again.
    let stop_asked = match store::asked_to_stop(&state.pool, id).await {
        Ok(asked) => asked,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading whether a stop was asked for failed");
            false
        }
    };

    // And whether a steer into Implementing would have anything to carry on: a
    // backlog with work left in it, or a roadmap the branch has written. What
    // the modal draws the *carrying on* by, the target itself being offered
    // wherever an instruction can be written, which is everywhere. Off the
    // Worktree as it stands, which is where the pinned Events above are read
    // from and for the same reason — the repository owns those files. See
    // [`crate::steering::standing`], which is the rule the submit refuses by,
    // and which reads a Worktree that has gone as *cannot tell* rather than as
    // nothing standing: the steer makes one out of the branch before anything
    // runs in it.
    let ready_to_continue = crate::steering::standing(
        conversation.direction,
        conversation.worktree.clone(),
        conversation.base_commit.clone(),
    )
    .await
    .offerable();

    // The stop the header draws a mark for, which is every stop but the one on a
    // Conversation the human has closed. Closing is them saying the work is over
    // wherever it had got to, so whatever it stopped on stopped being something
    // to come back to — a Conversation they closed themselves is the last place
    // a mark saying *look here* belongs. The stop record itself is untouched: it
    // is history, and the Notice it points at is still on the Timeline.
    let marked = stopped
        .as_ref()
        .filter(|_| conversation.state != store::Lifecycle::Closed);

    // And the mark points at the stop's own Notice, whatever wrote it: a run
    // that has stopped is stopped, and a mark with nowhere to go would be one
    // the human could not act on.
    let blocked_on = marked.map(|stopped| stopped.notice);

    // Which mark it is, decided here so the browser never weighs a stored word:
    // Verkstead's brake and a driver a crash took away are things that happened
    // without the human, so those get the accent badge; their own press gets the
    // quiet label. See [`store::Decision::waits_on_the_human`], which is the
    // same rule the sidebar's own `waiting` is folded by.
    let stopped_by_hand = marked.is_some_and(|stopped| !stopped.decision.waits_on_the_human());

    // With the words about the account coming back beside it, where the stop
    // carries any: the one thing that tells a run stopped by an exhausted window
    // from a run stopped by anything else. Drawn beside Resume rather than acted
    // on — no stop resumes itself, so every one of them waits for the same press.
    let resets = stopped.and_then(|stopped| stopped.resets);

    // And whether the wrap-up has narrowed to its checks, which is a label
    // beside the state rather than a state of its own: the review and the
    // comments settled, the checks not. Half of the condition — the other half
    // is that nothing is running in the Worktree, which is `writing` below.
    let narrowed_to_checks = match store::narrowed_to_checks(&state.pool, id).await {
        Ok(narrowed) => narrowed,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading whether a wrap-up was down to its checks failed");
            false
        }
    };

    // And whether the human has put this Conversation away, which is what the
    // actions menu offers Unarchive by. Read here rather than carried by the
    // Conversation the store loaded: it is a fact about the sidebar, and the
    // page is the one other place that has anything to say about it.
    let archived = match store::archived(&state.pool, id).await {
        Ok(archived) => archived,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading whether a Conversation was archived failed");
            false
        }
    };

    // One clock for the whole Timeline: every Set on it is aged against the same
    // moment, so two rows written a millisecond apart cannot come back reading as
    // if they were read at different times.
    let now = OffsetDateTime::now_utc();

    let view = ConversationView {
        id: conversation.id,
        repo: RepoEntry {
            id: conversation.repo.id,
            name: conversation.repo.name,
            // Stored as UTF-8 in the first place — a path that is not cannot be
            // registered — so nothing is lost putting it back on the wire.
            path: conversation.repo.path.to_string_lossy().into_owned(),
            default_branch: conversation.repo.default_branch,
        },
        branch: conversation.branch,
        base_commit: conversation.base_commit,
        companions,
        state: lifecycle(conversation.state),
        ready_to_grill,
        ready_to_resume,
        ready_to_stop,
        stop_asked,
        ready_to_continue,
        adopting,
        grilling_pairing,
        implementation_pairing,
        worktree,
        direction: conversation.direction,
        pinned,
        blocked_on,
        stopped_by_hand,
        // A fix session actively working a red check is a wrap-up getting on
        // with it, so the label is drawn only where nothing is running — the
        // same reading `working` below is.
        waiting_on_checks: narrowed_to_checks && writing.is_none(),
        resets,
        archived,
        // The same reading the Events above are drawn against, said as a fact
        // about the Conversation: the Timeline offers Force stop exactly where
        // something is running, and one Event of a session's is not the question
        // — a Conversation whose session has ended is not working, whichever
        // Event it was writing into.
        working: writing.is_some(),
        // And the register beside it, read raw: what is holding this
        // Conversation as of now, whatever state it is in. The rule about which
        // states ought to have one is `ready_to_resume`'s a few lines up — this
        // is the register itself, which is the half a reader outside the process
        // cannot see any other way.
        driven: state.drivers.registered(id),
        timeline: timeline
            .into_iter()
            // Every kind in, none held back: the record is the whole of what
            // happened, and the one Event that is pinned as well — the pull
            // request, see `pinned` above — is handed over twice rather than
            // moved out of here.
            .map(|event| {
                match event.event {
                    // Rendered on the way out where there is markdown to render —
                    // see [`verkstead_render`]. A move has none: it is one state.
                    store::Event::Brief(markdown) => verkstead_render::brief_event(
                        event.id,
                        event.at,
                        markdown,
                        Some(event.id) != open_brief,
                    ),
                    store::Event::Moved(state) => {
                        verkstead_render::moved_event(event.id, event.at, lifecycle(state))
                    }
                    // The summary and not the Capture: a Timeline is read every
                    // time an open page hears the world moved, and a session's
                    // output is megabytes the middle pane never shows.
                    store::Event::AgentOutput(summary) => verkstead_render::agent_output_event(
                        event.id,
                        event.at,
                        summary.lines,
                        summary.turns,
                        summary.latest,
                        writing == Some(event.id),
                        idling,
                    ),
                    // The table of what was asked against what was decided, and no
                    // more: the whole document is what the details pane fetches,
                    // from the endpoint one Set has always been read through.
                    //
                    // The Event's own stamp is what the Liveness verdict is aged
                    // against. It is the Set's creation time — both are written in
                    // the one transaction that puts a Set on a Timeline.
                    store::Event::QuestionSet(asked) => match &asked.set {
                        store::Asked::Set(set) => {
                            let standing = standing(
                                &state,
                                asked.set_id,
                                asked.settlement,
                                asked.deferred,
                                &event.at,
                                now,
                            );

                            verkstead_render::question_set_event(
                                event.id,
                                event.at,
                                asked.set_id,
                                set,
                                standing,
                            )
                        }
                        // A row of its own rather than an omission: the ask
                        // happened, and a Timeline that quietly left it out
                        // would be this build deciding a decision never
                        // occurred. What it opens is the stored body — see the
                        // Set endpoint above.
                        store::Asked::Unreadable(unreadable) => {
                            verkstead_render::unreadable_set_event(
                                event.id,
                                event.at,
                                asked.set_id,
                                unreadable.why.clone(),
                            )
                        }
                    },
                    // Rendered like the Brief, and inline like it: a document to
                    // read, with nothing of it a details pane would add.
                    store::Event::Handoff(markdown) => {
                        verkstead_render::handoff_event(event.id, event.at, &markdown)
                    }
                    // The counts, the subject and what the commit said about
                    // itself, and not the diff: the diff is in the repository,
                    // and what fetches it is the pane that shows it. The summary
                    // goes over whole and comes out as the snippet the card
                    // clamps — the renderer's, so that the cutting of a commit's
                    // own words happens where every other rendering of them does.
                    store::Event::Commit(commit) => verkstead_render::commit_event(
                        event.id,
                        event.at,
                        verkstead_render::CommitRecord {
                            sha: commit.sha,
                            subject: commit.subject,
                            files: commit.files,
                            insertions: commit.insertions,
                            deletions: commit.deletions,
                            summary: commit.summary,
                            // Which repository it came out of, where that is not
                            // this Conversation's own. The store decides that,
                            // because it is the store that knows both.
                            repo: commit.repo,
                        },
                    ),
                    // A wait a Verkstead of before put on a Timeline, said in
                    // the sentence a stop for a window is said in now — see
                    // [`crate::stopping::out_of_window`]. Nothing writes another
                    // and nothing rewrote these: what changes is that the record
                    // reads as the one kind of stopped thing, drawn as the line
                    // it always carried with nothing to press on it.
                    //
                    // A Notice by the time it is on the wire, which is what a
                    // migrated database needs it to be: an open Pause was read
                    // onto its Conversation as the stop it is, and the Event the
                    // *blocked on you* badge points at is this one.
                    store::Event::Pause(pause) => verkstead_render::notice_event(
                        event.id,
                        event.at,
                        &crate::stopping::out_of_window(&pause.profile, &pause.said),
                    ),
                    // Rendered like the handoff and inline like it, being the
                    // other kind of sentence somebody has to be able to read
                    // back — and the one nobody wrote for a human to press
                    // anything about. What a stop's Notice says is what stopped,
                    // why, and the evidence — see [`crate::stopping`], which writes
                    // the markdown.
                    store::Event::Notice(markdown) => {
                        verkstead_render::notice_event(event.id, event.at, &markdown)
                    }
                    // And a Manual Task a Verkstead of before set going by hand.
                    // Nothing writes another — a steer into Implementing carries
                    // the instruction now — and nothing rewrote these: the
                    // instruction is what was asked for, and it is drawn as the
                    // line it is with nothing to press on it.
                    store::Event::ManualTask(instruction) => {
                        verkstead_render::manual_task_event(event.id, event.at, &instruction)
                    }
                    // And where the human said the work goes, which is the one
                    // Event that stands beside another: the move it wrote is
                    // right under it, and the pair is the whole record of a
                    // steer — who decided, and what became of it.
                    store::Event::Steer(target, instruction) => verkstead_render::steer_event(
                        event.id,
                        event.at,
                        lifecycle(target),
                        instruction.as_deref(),
                    ),
                    // The one kind that is pinned as well as listed, handed
                    // over twice for the page to draw twice: the sticky block
                    // above the record keeps it in view, and here is the moment
                    // it happened. Both copies are the same card made the same
                    // way — see [`verkstead_render::pull_request_reached`].
                    store::Event::PullRequest(opened) => verkstead_render::pull_request_reached(
                        event.id,
                        event.at,
                        verkstead_render::PullRequestSummary {
                            number: opened.number,
                            title: opened.title,
                            url: opened.url,
                            repo: opened.repo.clone(),
                            checks: own_checks(&opened.repo, checks),
                        },
                    ),
                    // And the two rows that carry nothing of their own: what is
                    // drawn at them is the live reading above, handed over a
                    // second time. The row says when the branch first carried a
                    // backlog — or a roadmap — and the card at it says what that
                    // list holds now, which is the same card the pinned block is
                    // showing.
                    store::Event::TaskList => {
                        verkstead_render::task_list_reached(event.id, event.at, backlog.clone())
                    }
                    store::Event::StageList => {
                        verkstead_render::stage_list_reached(event.id, event.at, roadmaps.clone())
                    }
                }
            })
            .collect(),
    };

    Json(view).into_response()
}

/// `GET /api/ui/conversations/{id}/capture/{event}` — what one session
/// printed, whole.
///
/// Its own request rather than a field on the Conversation, because of the two
/// sizes involved. A session prints megabytes over an hour and the Timeline is
/// re-read every time an open page hears the world moved; the Capture is read
/// when somebody opens the one Event it belongs to.
///
/// Byte for byte, control sequences and all — what a terminal was sent is what
/// the session said.
async fn capture(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
) -> HttpResponse {
    // Two ids out of a URL a human may have typed, and neither of them naming a
    // number cannot name a Capture — the same permissiveness every other id
    // here is read with.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return no_such_capture();
    };

    match store::capture(&state.pool, id, event).await {
        Ok(Some(text)) => Json(verkstead_render::Capture { text }).into_response(),
        Ok(None) => no_such_capture(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading a Capture failed");
            unavailable("the Capture could not be read")
        }
    }
}

/// Where a reading of a Transcript is carrying on from, where it is carrying on
/// from anywhere.
#[derive(Debug, serde::Deserialize)]
struct Resuming {
    /// The cursor the reader's last reading ended at. Absent on the first one,
    /// which is what makes a whole read the default rather than something to
    /// ask for.
    after: Option<String>,
}

/// `GET /api/ui/conversations/{id}/transcript/{event}` — what one session
/// said, as a conversation. With `?after=<cursor>`, only what it has said since
/// that reading stopped.
///
/// Its own request rather than a field on the Conversation, for the Capture's
/// reason and to the same size: this is an hour of talking, and the Timeline is
/// re-read every time an open page hears the world moved.
///
/// And incremental for the same reason one step further on. An open pane
/// re-reads a running session's record on every batch of lines, which is twice
/// a second while it talks — so a reading says where it stopped and the next one
/// begins there, and what crosses the wire is the new turns rather than the hour
/// before them (ADR 0009).
///
/// Every way of failing to carry on ends in the whole record: a cursor that was
/// never written here, one naming a place this Transcript has not reached, and
/// a reader that names none at all. The whole record is always a correct answer
/// to any of them, and a gap in what somebody is reading never is.
///
/// The lines were stored verbatim and are read here, on the way out, which is
/// what keeps the coupling to somebody else's file format to the one crate that
/// has the parsers in it (ADR 0006). An empty Transcript is an ordinary answer
/// and not a failure: it is every session that left no log, and the pane's
/// answer to one is to show the Capture instead.
async fn transcript(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
    Query(resuming): Query<Resuming>,
) -> HttpResponse {
    // Read as permissively as every other pair of ids here: neither of them
    // naming a number cannot name a Transcript.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return no_such_transcript();
    };

    if let Some(from) = resuming.after.and_then(|at| at.parse::<Cursor>().ok()) {
        match store::transcript_after(&state.pool, id, event, i64::from(from.lines)).await {
            Ok(Some(lines)) => {
                return Json(verkstead_render::transcript_after(from, &lines)).into_response();
            }
            // The cursor names a place this record has not been, or there is no
            // such Transcript at all. Which of the two is settled by reading it
            // whole, below.
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading the rest of a Transcript failed");
                return unavailable("the Transcript could not be read");
            }
        }
    }

    match store::transcript(&state.pool, id, event).await {
        Ok(Some(lines)) => Json(verkstead_render::transcript_view(&lines)).into_response(),
        Ok(None) => no_such_transcript(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading a Transcript failed");
            unavailable("the Transcript could not be read")
        }
    }
}

/// `GET /api/ui/conversations/{id}/screen/{event}` — how one session looked:
/// the grid its Capture leaves on a terminal.
///
/// The same Event read a third way. The Transcript is what the session said and
/// the Capture is the bytes it sent a terminal; this is the terminal at the
/// other end of them — the grid, with the cursor where the session left it,
/// handed over as the escape sequences that would paint it.
///
/// Replayed here rather than kept, because the Capture is the record and a
/// second copy of the same thing in a different shape is a second thing to keep
/// in step. A session that has ended replays to the screen it last stood on; a
/// live one replays to wherever its Capture has got to.
///
/// The parsing is the server's, which is what makes this the exception to the
/// rule that the browser never parses rather than a hole in it (ADR 0007): what
/// crosses the wire is a repaint to feed a terminal, and the terminal that
/// decided it stays here.
async fn screen(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
) -> HttpResponse {
    // Read as permissively as every other pair of ids here: neither of them
    // naming a number cannot name a Screen.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return no_such_screen();
    };

    match store::capture(&state.pool, id, event).await {
        Ok(Some(text)) => {
            let replayed = crate::screen::replay(&text);
            let (columns, rows) = replayed.size();

            Json(verkstead_render::Screen {
                repaint: replayed.repaint(),
                columns,
                rows,
            })
            .into_response()
        }
        Ok(None) => no_such_screen(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading a Screen failed");
            unavailable("the Screen could not be read")
        }
    }
}

/// `GET /api/ui/conversations/{id}/commit/{event}` — one commit's summary and
/// its diff, rendered.
///
/// Its own request rather than a field on the Conversation, exactly as a
/// Capture is: a Timeline is read every time an open page hears the world
/// moved, and a commit is worth reading whole when somebody opens the one Event
/// it belongs to.
///
/// The diff is read out of the repository rather than out of the store. The
/// commit is in git — that is what a commit *is* — and keeping a second copy of
/// every patch would be a database growing with the work rather than with the
/// record of it. The summary is the other way about: the sweep kept it when it
/// recorded the commit, so it comes off the Event beside the line the Timeline
/// draws.
async fn commit_pane(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
) -> HttpResponse {
    // Two ids out of a URL a human may have typed, read as permissively as every
    // other id here: neither of them naming a number cannot name a commit.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return no_such_commit();
    };

    let commit = match store::commit(&state.pool, id, event).await {
        Ok(Some(commit)) => commit,
        Ok(None) => return no_such_commit(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading a commit failed");
            return unavailable("the commit could not be read");
        }
    };

    // Which repository to read it out of, which is the one the commit was
    // recorded against rather than the Conversation's own: a companion's commit
    // is in the companion's repository, and the Conversation's would know
    // nothing about it.
    //
    // A commit whose repository can no longer say anything about it — taken off
    // the registry, moved out from under Verkstead — is the *gone* a collected
    // commit already is, and answers the same way.
    let repo = match store::commit_repo(&state.pool, id, event).await {
        Ok(Some(repo)) => repo.path,
        Ok(None) => return no_such_commit(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading the repository of a commit failed");
            return unavailable("the commit could not be read");
        }
    };

    // Read and rendered in the one blocking task. Parsing a patch and colouring
    // every line of it is as much work as running the `git` that produced it, and
    // an async worker thread is the wrong place for either: a large diff run
    // inline here would hold up every other request sharing that thread.
    let rendered = tokio::task::spawn_blocking(move || {
        crate::commits::patch(&repo, &commit.sha)
            .as_deref()
            .map(|patch| verkstead_render::commit_pane(commit.summary.as_deref(), patch))
    })
    .await;

    let rendered = match rendered {
        Ok(rendered) => rendered,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading a commit's diff failed");
            return unavailable("the commit could not be read");
        }
    };

    // A commit the repository will not say anything about is one that has gone —
    // collected, or on a branch somebody rewrote. There is nothing to draw a pane
    // about, which is what a 404 means everywhere else here.
    let Some(rendered) = rendered else {
        return no_such_commit();
    };

    Json(rendered).into_response()
}

/// `GET /api/ui/conversations/{id}/backlog` — every task document the
/// Conversation's Worktree holds, rendered.
///
/// Its own request rather than a field on the Conversation, exactly as a
/// commit's diff is: a Timeline is read every time an open page hears the world
/// moved, and what the entries *say* is worth reading when somebody opens the
/// card.
///
/// No Event id in the path, unlike the three panes around it. The backlog is a
/// reading of the Worktree rather than a record, so the row that says where it
/// landed fixes a position and names nothing: there is one backlog per
/// Conversation, and this is it.
///
/// A Conversation with no Worktree, no `.tasks/` or nothing readable in it is
/// one there is no pane to draw about, which is a 404 for the reason a commit
/// the repository has lost is one.
async fn backlog(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    // Read as permissively as every other id here: one that names no number
    // names no Conversation.
    let Ok(id) = id.parse::<i64>() else {
        return no_such_backlog();
    };

    let worktree = match store::load_conversation(&state.pool, id).await {
        Ok(Some(conversation)) => conversation.worktree,
        Ok(None) => return no_such_backlog(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "loading a Conversation failed");
            return unavailable("the backlog could not be read");
        }
    };

    match crate::tasks::documents(worktree).await {
        Some(pane) => Json(pane).into_response(),
        None => no_such_backlog(),
    }
}

/// `GET /api/ui/conversations/{id}/roadmap/{name}` — every stage brief the named
/// roadmap holds, rendered.
///
/// Its own request for the backlog's reason, and read off the Worktree the same
/// way. What is different is the name in the path: a Worktree holds one
/// `.tasks/` and may hold any number of roadmaps, so the card that opens this
/// says which of them it is.
///
/// The name is a directory name off a card the server itself drew, and it is
/// checked against the roadmaps this branch has written to before anything is
/// joined onto a path — see [`crate::stages::documents`].
///
/// A Conversation with no Worktree, no roadmap of that name, or nothing readable
/// in it is one there is no pane to draw about, which is a 404 for the reason
/// the backlog's is.
async fn roadmap(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return no_such_roadmap();
    };

    let (worktree, base) = match store::load_conversation(&state.pool, id).await {
        Ok(Some(conversation)) => (conversation.worktree, conversation.base_commit),
        Ok(None) => return no_such_roadmap(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "loading a Conversation failed");
            return unavailable("the roadmap could not be read");
        }
    };

    match crate::stages::documents(worktree, base, name).await {
        Some(pane) => Json(pane).into_response(),
        None => no_such_roadmap(),
    }
}

/// `GET /api/ui/conversations/{id}/pull-request/{event}` — what is on the pull
/// request the finish step opened: its commit list and its comments.
///
/// Its own request rather than a field on the Conversation, as a Capture and
/// a diff are — and more so than either, because reading it is asking GitHub
/// through the host's `gh`. A Timeline that carried this would make an API call
/// every time an open page heard the world moved.
///
/// A `gh` that will not answer is refused with the reason it gave, which is the
/// one thing the human can act on: what the pane then shows is "there is no `gh`
/// on this machine's PATH" rather than a spinner.
///
/// The same question carries back how the checks are, so opening this is also
/// what freshens the rollup the card draws — see [`crate::checks::remember`].
/// The checks watcher keeps that fresh while a wrap-up is running and stops when
/// the wrap-up is over, so on a Conversation carried to Done the pane is the one
/// thing left that asks.
async fn pull_request(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
) -> HttpResponse {
    // Two ids out of a URL a human may have typed, read as permissively as every
    // other pair here.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return no_such_pull_request();
    };

    // Which PR, off the Conversation's own record: the Event says which pull
    // request, and an Event id belonging to another Conversation names nothing
    // here.
    let timeline = match store::timeline(&state.pool, id).await {
        Ok(timeline) => timeline,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading a Timeline failed");
            return unavailable("the pull request could not be read");
        }
    };

    let opened = timeline.into_iter().find_map(|on| match on.event {
        store::Event::PullRequest(opened) if on.id == event => Some(opened),
        _ => None,
    });

    let Some(opened) = opened else {
        return no_such_pull_request();
    };

    // And which repository to ask about it in, which is the repository that pull
    // request was opened in rather than the Conversation's own. A number means
    // something else in another repository, or nothing at all — so a companion's
    // pull request asked about in the work's own repo would come back as
    // somebody else's work or as a 404. See [`store::pull_request_repo`].
    let repo = match store::pull_request_repo(&state.pool, id, event).await {
        Ok(Some(repo)) => repo.path,
        Ok(None) => return no_such_pull_request(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading the repository of a pull request failed");
            return unavailable("the pull request could not be read");
        }
    };

    let gh = state.github.clone();

    let asked =
        tokio::task::spawn_blocking(move || crate::github::details(&gh, &repo, opened.number))
            .await;

    match asked {
        Ok(Ok(read)) => {
            // Written down before the answer goes out, so a page that redraws
            // the card on the Nudge this sends draws what the pane is about to
            // show it.
            crate::checks::remember(&state, id, &read.checks).await;

            Json(read.pane).into_response()
        }
        // GitHub could not be asked, or would not say. Refused with `gh`'s own
        // reason rather than a bare failure: every one of those reasons is
        // something different for the human to go and do.
        Ok(Err(trouble)) => {
            tracing::warn!(
                conversation_id = id,
                event_id = event,
                why = trouble.why(),
                "a pull request could not be read through the host gh",
            );

            refused(StatusCode::BAD_GATEWAY, ApiError::new(trouble.why()))
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "asking gh about a pull request failed");
            unavailable("the pull request could not be read")
        }
    }
}

/// `POST /api/ui/conversations/{id}/brief` — save what the human has written.
async fn save_brief(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(edit): Json<BriefEdit>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::BriefSaved::NoSuchConversation).into_response();
    };

    match crate::conversations::save_brief(&state.pool, id, &edit.markdown).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "saving a Brief failed");
            unavailable("the Brief could not be saved")
        }
    }
}

/// `POST /api/ui/conversations/{id}/branch` — name the branch the work will be
/// done on.
async fn rename_branch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(rename): Json<BranchRename>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::BranchRenamed::NoSuchConversation).into_response();
    };

    match crate::conversations::rename_branch(&state.pool, id, &rename.branch).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "renaming a branch failed");
            unavailable("the branch could not be named")
        }
    }
}

/// `POST /api/ui/conversations/{id}/base` — choose the branch the work comes
/// off, or put the Conversation back on the default-branch rule.
async fn set_base_branch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(choice): Json<BaseBranchChoice>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::BaseRecorded::NoSuchConversation).into_response();
    };

    match crate::conversations::set_base_branch(&state.pool, id, choice.branch.as_deref()).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "recording a base branch failed");
            unavailable("the base branch could not be recorded")
        }
    }
}

/// `POST /api/ui/conversations/{id}/companions` — work alongside another
/// registered Repo.
async fn add_companion(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(added): Json<NewCompanion>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(CompanionAdded::NoSuchConversation).into_response();
    };

    match crate::conversations::add_companion(&state.pool, id, added.repo_id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "adding a companion repo failed");
            unavailable("the companion repo could not be added")
        }
    }
}

/// `POST /api/ui/conversations/{id}/companions/{repo}/remove` — and stop working
/// alongside one.
///
/// Which Repo is in the path and there is no body at all: the id is the whole of
/// what a removal says.
async fn remove_companion(
    State(state): State<AppState>,
    Path((id, repo)): Path<(String, String)>,
) -> HttpResponse {
    let (Ok(id), Ok(repo)) = (id.parse::<i64>(), repo.parse::<i64>()) else {
        return Json(CompanionRemoved::NoSuchConversation).into_response();
    };

    match crate::conversations::remove_companion(&state.pool, id, repo).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "removing a companion repo failed");
            unavailable("the companion repo could not be removed")
        }
    }
}

/// `POST /api/ui/conversations/{id}/companions/{repo}/mode` — how far into one
/// of them the work may reach.
async fn set_companion_mode(
    State(state): State<AppState>,
    Path((id, repo)): Path<(String, String)>,
    Json(choice): Json<CompanionModeChoice>,
) -> HttpResponse {
    let (Ok(id), Ok(repo)) = (id.parse::<i64>(), repo.parse::<i64>()) else {
        return Json(CompanionModeChosen::NoSuchConversation).into_response();
    };

    match crate::conversations::set_companion_mode(&state.pool, id, repo, choice.mode).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "setting a companion's mode failed");
            unavailable("the companion repo's mode could not be set")
        }
    }
}

/// `POST /api/ui/conversations/{id}/companions/{repo}/base` — the branch of the
/// companion's own repository its checkout comes off, or the default-branch
/// rule.
async fn set_companion_base(
    State(state): State<AppState>,
    Path((id, repo)): Path<(String, String)>,
    Json(choice): Json<BaseBranchChoice>,
) -> HttpResponse {
    let (Ok(id), Ok(repo)) = (id.parse::<i64>(), repo.parse::<i64>()) else {
        return Json(CompanionBaseRecorded::NoSuchConversation).into_response();
    };

    match crate::conversations::set_companion_base(&state.pool, id, repo, choice.branch.as_deref())
        .await
    {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "recording a companion's base branch failed");
            unavailable("the companion repo's base branch could not be recorded")
        }
    }
}

/// `POST /api/ui/conversations/{id}/companions/{repo}/branch` — what a
/// read-write companion's branch is called, or nothing at all for mirroring.
async fn rename_companion_branch(
    State(state): State<AppState>,
    Path((id, repo)): Path<(String, String)>,
    Json(rename): Json<BranchRename>,
) -> HttpResponse {
    let (Ok(id), Ok(repo)) = (id.parse::<i64>(), repo.parse::<i64>()) else {
        return Json(CompanionBranchRenamed::NoSuchConversation).into_response();
    };

    match crate::conversations::rename_companion_branch(&state.pool, id, repo, &rename.branch).await
    {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "naming a companion's branch failed");
            unavailable("the companion repo's branch could not be named")
        }
    }
}

/// `POST /api/ui/conversations/{id}/grill` — give a Conversation somewhere to
/// work and set it grilling.
///
/// Every precondition is checked here whatever the page believed a moment ago:
/// `ready_to_grill` is what decides whether the button is offered, and a Profile
/// can be deleted or a base commit lost between the page reading that and the
/// human pressing it.
async fn start_grilling(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(GrillingStarted::NoSuchConversation).into_response();
    };

    match crate::conversations::start_grilling(&state, id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "starting a grilling failed");
            unavailable("the grilling could not be started")
        }
    }
}

/// `POST /api/ui/conversations/{id}/adopt` — take a roadmap's next stage and set
/// it working.
///
/// The grilling start's sibling, and checked the same way: the page names the
/// stage it read a moment ago, and a roadmap somebody has ticked, taken or
/// finished since is answered here rather than there.
async fn adopt(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(Adopted::NoSuchConversation).into_response();
    };

    match crate::conversations::adopt(&state, id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "adopting a roadmap stage failed");
            unavailable("the stage could not be adopted")
        }
    }
}

/// `POST /api/ui/conversations/{id}/resume` — start driving it again.
///
/// What should be running is recomputed from the state the Conversation is in
/// and what its branch has written, rather than being read off whatever stopped:
/// a stop is answered whenever the human gets to it, and the work moves on in
/// the meantime.
///
/// Answered as soon as the decision is made rather than once the session is up.
/// What Resume starts takes as long as it takes — a grilling relaunch waits for
/// the Worktree, and a wrap-up is five watchers — and the browser is waiting for
/// *whether* it started, which is what the named outcomes say.
async fn resume(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(Resumed::NoSuchConversation).into_response();
    };

    match crate::resume::resume(&state, id, crate::resume::Resuming::Pressed).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "starting to drive a Conversation again failed");
            unavailable("the conversation could not be resumed")
        }
    }
}

/// `POST /api/ui/conversations/{id}/stop` — pause after the current task.
///
/// Answered as soon as the decision is made, like Resume: what the browser is
/// waiting for is *whether* the run is stopping, and a session left to reach its
/// own end takes as long as the step takes.
async fn stop(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ConversationStopped::NoSuchConversation).into_response();
    };

    match crate::stops::stop(&state, id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "stopping a Conversation failed");
            unavailable("the conversation could not be stopped")
        }
    }
}

/// `POST /api/ui/conversations/{id}/force-stop` — stop now, and end what is
/// running.
async fn force_stop(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ConversationStopped::NoSuchConversation).into_response();
    };

    match crate::stops::force(&state, id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "stopping a Conversation where it stood failed");
            unavailable("the conversation could not be stopped")
        }
    }
}

/// `POST /api/ui/conversations/{id}/steer` — stop the drive and open the modal.
///
/// The click rather than the move. What comes back says the modal may open and
/// whether a session is still running, which is what the **Interrupt current
/// task** checkbox is offered against. Cancelling from here leaves the
/// Conversation stopped with Resume on offer, which is what the click is for.
async fn steer(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(SteerOpened::NoSuchConversation).into_response();
    };

    match crate::steering::click(&state, id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "stopping a Conversation to steer it failed");
            unavailable("the conversation could not be stopped to steer it")
        }
    }
}

/// `POST /api/ui/conversations/{id}/steer/submit` — move it where the human
/// said.
///
/// Answered as soon as the move is recorded rather than once anything the target
/// needs is up, the way Resume is: what the browser is waiting for is *whether*
/// it went, and the targets that launch something take as long as a launch
/// takes.
async fn steer_submit(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(submission): Json<SteerSubmission>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ConversationSteered::NoSuchConversation).into_response();
    };

    match crate::steering::submit(&state, id, &submission).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "steering a Conversation failed");
            unavailable("the conversation could not be steered")
        }
    }
}

/// `POST /api/ui/conversations/{id}/close` — stop it wherever it has got to.
async fn close(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ConversationClosed::NoSuchConversation).into_response();
    };

    match crate::conversations::close(&state, id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "closing a Conversation failed");
            unavailable("the conversation could not be closed")
        }
    }
}

/// `POST /api/ui/conversations/{id}/close-and-archive` — end it and put it away
/// in one press.
///
/// The menu's two rows joined, because they are one intention often enough: a
/// Conversation the human is finished with is usually one they are finished
/// looking at. Joined on this side rather than in the browser so that a
/// connection dropped between the two cannot leave the pair half made.
///
/// Answered with what became of the close, that being the half that has
/// anything to refuse: archiving a Conversation just closed is either the
/// archiving asked for or one already made, and the browser reads the
/// Conversation back either way. What comes back as a failure names which half
/// it was, because *closed but still on the list* is a different thing to be
/// told than *not closed*.
async fn close_and_archive(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ConversationClosed::NoSuchConversation).into_response();
    };

    let closed = match crate::conversations::close(&state, id).await {
        Ok(outcome) => outcome,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "closing a Conversation failed");
            return unavailable("the conversation could not be closed");
        }
    };

    // Nothing to put away where there was nothing to close. The other outcomes
    // are both a Conversation that is closed now, which is what archiving wants.
    if closed == ConversationClosed::NoSuchConversation {
        return Json(closed).into_response();
    }

    match store::archive_conversation(&state.pool, id).await {
        // Whichever it says, the Conversation is off the list — and the close is
        // what the browser is told about, as it is for the row that only closes.
        Ok(_) => {
            state.nudges.announce(Nudge::Conversations);
            Json(closed).into_response()
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "putting a just-closed Conversation away failed");
            unavailable("the conversation was closed, but could not be put away")
        }
    }
}

/// `POST /api/ui/conversations/{id}/archive` — take a closed one off the list.
///
/// Straight to the store rather than through [`crate::conversations`], as the
/// sidebar's order is: there is no worktree to remove and no session to end.
/// Archiving is a fact about the list, and writing it down is the whole of it.
async fn archive(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ConversationArchived::NoSuchConversation).into_response();
    };

    match store::archive_conversation(&state.pool, id).await {
        Ok(outcome) => {
            state.nudges.announce(Nudge::Conversations);
            Json(archived(outcome)).into_response()
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "archiving a Conversation failed");
            unavailable("the conversation could not be archived")
        }
    }
}

/// The store's word for what became of it, as the wire says it.
fn archived(outcome: store::Archiving) -> ConversationArchived {
    match outcome {
        store::Archiving::Archived => ConversationArchived::Archived,
        store::Archiving::AlreadyArchived => ConversationArchived::AlreadyArchived,
        store::Archiving::NotClosed => ConversationArchived::NotClosed,
        store::Archiving::NoSuchConversation => ConversationArchived::NoSuchConversation,
    }
}

/// `POST /api/ui/conversations/{id}/unarchive` — put it back on the list.
///
/// Archiving's mirror in every way, the store included: one row taken away,
/// and no state to be in the wrong one of. The Nudge is what carries it to the
/// other devices, a Conversation arriving on the list being the one thing every
/// open sidebar has to read again.
async fn unarchive(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ConversationUnarchived::NoSuchConversation).into_response();
    };

    match store::unarchive_conversation(&state.pool, id).await {
        Ok(outcome) => {
            state.nudges.announce(Nudge::Conversations);
            Json(unarchived(outcome)).into_response()
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "unarchiving a Conversation failed");
            unavailable("the conversation could not be unarchived")
        }
    }
}

/// And its word for what became of that.
fn unarchived(outcome: store::Unarchiving) -> ConversationUnarchived {
    match outcome {
        store::Unarchiving::Unarchived => ConversationUnarchived::Unarchived,
        store::Unarchiving::NotArchived => ConversationUnarchived::NotArchived,
        store::Unarchiving::NoSuchConversation => ConversationUnarchived::NoSuchConversation,
    }
}

/// `POST /api/ui/conversations/{id}/seen` — the human has looked at this one.
///
/// Takes the unseen mark off, which is the whole of it: the mark is one row or
/// none, and there is nothing to be refused for. An id naming no Conversation
/// clears nothing and says so the same way one that was never marked does —
/// looking at something is not a claim that it is still there.
///
/// The Nudge goes out only where there was a mark to take away. The ordinary
/// case is a Conversation opened for the second time in a session of reading,
/// and every other device re-reading its sidebar because a page was scrolled
/// past would be a cost paid for nothing.
async fn seen(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return StatusCode::NO_CONTENT.into_response();
    };

    match store::see_conversation(&state.pool, id).await {
        Ok(cleared) => {
            if cleared {
                state.nudges.announce(Nudge::Conversations);
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "clearing the unseen mark on a Conversation failed");
            unavailable("the conversation could not be marked as seen")
        }
    }
}

/// `GET /api/ui/conversations/archived` — whether the sidebar is drawing what
/// has been put away.
async fn showing_archived(State(state): State<AppState>) -> HttpResponse {
    match store::showing_archived(&state.pool).await {
        Ok(showing) => Json(ShowingArchived { showing }).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "reading whether the archived Conversations are shown failed");
            unavailable("the setting could not be read")
        }
    }
}

/// `POST /api/ui/conversations/archived` — and saying that it is, or is not.
///
/// The position the switch has been put in rather than a flip, so two devices
/// pressing at once land on a state one of them asked for rather than on
/// whichever order they arrived in. Refused for nothing, and there is nothing
/// to answer with beyond that it was taken.
async fn show_archived(
    State(state): State<AppState>,
    Json(showing): Json<ShowingArchived>,
) -> HttpResponse {
    match store::show_archived(&state.pool, showing.showing).await {
        Ok(()) => {
            state.nudges.announce(Nudge::Conversations);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => {
            tracing::error!(error = ?error, "saying whether the archived Conversations are shown failed");
            unavailable("the setting could not be saved")
        }
    }
}

/// `POST /api/ui/conversations/{id}/grilling-pairing` — which account and model
/// the grilling session runs under.
async fn choose_grilling_pairing(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(choice): Json<ProfileChoice>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::ProfileChosen::NoSuchConversation).into_response();
    };

    match crate::profiles::choose_grilling(&state.pool, id, &choice).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "choosing a grilling Pairing failed");
            unavailable("the grilling pairing could not be chosen")
        }
    }
}

/// `POST /api/ui/conversations/{id}/implementation-pairing` — and the one the
/// implementation runs under, which is a separate choice.
async fn choose_implementation_pairing(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(choice): Json<ProfileChoice>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::ProfileChosen::NoSuchConversation).into_response();
    };

    match crate::profiles::choose_implementation(&state.pool, id, &choice).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "choosing an implementation Pairing failed");
            unavailable("the implementation pairing could not be chosen")
        }
    }
}

/// `GET /api/ui/profiles` — the Agent Profiles, by name, each saying whether its
/// pair is still where it was left.
async fn profiles(State(state): State<AppState>) -> HttpResponse {
    match crate::profiles::listed(&state.pool, &state.watched).await {
        Ok(rows) => Json::<Vec<ProfileEntry>>(rows).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "reading the Agent Profiles failed");
            unavailable("the agent profiles could not be read")
        }
    }
}

/// `POST /api/ui/profiles` — take on an account, named by the pair that is
/// mounted for it.
///
/// Every refusal is the server's, as a registration's is: the Watched Paths are
/// a security boundary, and one a request could reach around by not going
/// through the form would not be one.
async fn create_profile(
    State(state): State<AppState>,
    Json(edit): Json<ProfileEdit>,
) -> HttpResponse {
    match crate::profiles::create(&state.pool, &state.watched, &edit).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "saving an Agent Profile failed");
            unavailable("the agent profile could not be saved")
        }
    }
}

/// `POST /api/ui/profiles/{id}` — rewrite one, whole.
async fn edit_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(edit): Json<ProfileEdit>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::ProfileSaved::NoSuchProfile).into_response();
    };

    match crate::profiles::edit(&state.pool, &state.watched, id, &edit).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, profile_id = id, "rewriting an Agent Profile failed");
            unavailable("the agent profile could not be saved")
        }
    }
}

/// `POST /api/ui/profiles/{id}/delete` — remove one nobody is running under.
async fn delete_profile(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::ProfileDeleted::NoSuchProfile).into_response();
    };

    match crate::profiles::remove(&state.pool, id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, profile_id = id, "removing an Agent Profile failed");
            unavailable("the agent profile could not be removed")
        }
    }
}

/// A stored companion as the viewer receives it: the Repo in the shape every
/// other Repo crosses this wire in, the three facts about how the work will use
/// it, and where it was checked out once it has been.
///
/// The checkout is read off the filesystem the Conversation's own is —
/// [`crate::conversations::worktree`] — because whether a directory is still
/// there is not something a database knows, and a companion deleted by hand
/// should read as a Conversation with a problem rather than as an obscure
/// failure from whatever next works in it.
async fn companion(companion: store::Companion) -> Result<CompanionView, anyhow::Error> {
    let worktree = crate::conversations::worktree(companion.worktree).await?;

    Ok(CompanionView {
        repo: RepoEntry {
            id: companion.repo.id,
            name: companion.repo.name,
            // Stored as UTF-8 in the first place — a path that is not cannot be
            // registered — so nothing is lost putting it back on the wire.
            path: companion.repo.path.to_string_lossy().into_owned(),
            default_branch: companion.repo.default_branch,
        },
        mode: match companion.mode {
            store::CompanionMode::ReadOnly => CompanionMode::ReadOnly,
            store::CompanionMode::ReadWrite => CompanionMode::ReadWrite,
        },
        base_ref: companion.base_ref,
        branch: companion.branch,
        worktree,
        base_commit: companion.base_commit,
    })
}

/// And how a pull request's checks are, the same way: the store's word for a
/// whole suite, as the card that draws an icon of it receives it.
fn rollup(checks: store::Rollup) -> CheckRollup {
    match checks {
        store::Rollup::Passed => CheckRollup::Passed,
        store::Rollup::Running => CheckRollup::Running,
        store::Rollup::Failed => CheckRollup::Failed,
    }
}

/// How the checks are, but only on the pull request they were written down
/// about.
///
/// A rollup is recorded per Conversation, and a Conversation now ends on one
/// pull request per repository it was worked in — so the word belongs to the one
/// that moved it into Wrapping, which is the Conversation's own repository's.
/// That is the pull request whose `repo` reads back unlabeled: a companion's
/// carries the name of the repository it was opened in.
///
/// A companion's card draws no icon rather than this one's. Drawing it there
/// would be saying something about a suite nobody asked GitHub about.
fn own_checks(repo: &Option<String>, checks: Option<CheckRollup>) -> Option<CheckRollup> {
    match repo {
        None => checks,
        Some(_) => None,
    }
}

/// A sidebar row's state as the viewer receives it, including the row whose
/// stored word this Verkstead has never heard of.
///
/// Such a row is drawn as a **Draft**, and the word it really held goes in the
/// log. Draft rather than anything else because of what the row is *for* here:
/// it is the way to the Conversation's own page, and the page's own read still
/// refuses a word it cannot parse — so what the human lands on is the pane's
/// error state and the escape hatch in it. That hatch offers Archive on a state
/// of Closed and Close-and-archive on anything else, and Archive on a row like
/// this would answer `NotClosed` and go nowhere. So the fallback has to read as
/// *not closed*, and Draft is the one that does while drawing harmlessly.
fn row_state(id: i64, state: store::RowState) -> Lifecycle {
    match state {
        store::RowState::Known(state) => lifecycle(state),
        store::RowState::Unknown(word) => {
            tracing::warn!(
                conversation_id = id,
                state = word,
                "a Conversation's stored state is a word this Verkstead does not know, so its \
                 row is drawn as a draft"
            );

            Lifecycle::Draft
        }
    }
}

/// The store's lifecycle state as the viewer receives it. One word either side,
/// and this is where the two vocabularies are held to each other.
fn lifecycle(state: store::Lifecycle) -> Lifecycle {
    match state {
        store::Lifecycle::Draft => Lifecycle::Draft,
        store::Lifecycle::Grilling => Lifecycle::Grilling,
        store::Lifecycle::Implementing => Lifecycle::Implementing,
        store::Lifecycle::Wrapping => Lifecycle::Wrapping,
        store::Lifecycle::FollowUp => Lifecycle::FollowUp,
        store::Lifecycle::Done => Lifecycle::Done,
        store::Lifecycle::Closed => Lifecycle::Closed,
    }
}

/// `GET /api/ui/push/key` — the public half of the server's VAPID keypair.
async fn push_key(State(state): State<AppState>) -> HttpResponse {
    match store::vapid_keys(&state.pool).await {
        Ok(keys) => Json(PushKey {
            key: keys.public_key,
        })
        .into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "reading the VAPID keypair failed");
            unavailable("the push key could not be read")
        }
    }
}

/// `POST /api/ui/push/subscribe` — take a device's subscription, so a Set
/// arriving can reach it.
async fn subscribe(
    State(state): State<AppState>,
    Json(subscription): Json<Subscription>,
) -> HttpResponse {
    let subscribing = store::store_subscription(
        &state.pool,
        &store::PushSubscription {
            endpoint: subscription.endpoint,
            p256dh: subscription.p256dh,
            auth: subscription.auth,
        },
    )
    .await;

    match subscribing {
        Ok(store::Subscribing::Stored) => Json(Subscribed::Stored).into_response(),
        Ok(store::Subscribing::Incomplete) => Json(Subscribed::Incomplete).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "storing a push subscription failed");
            unavailable("the subscription could not be stored")
        }
    }
}

/// `POST /api/ui/push/unsubscribe` — forget a device, because it asked not to be
/// told any more.
///
/// An endpoint the server never stored is not an error: a browser can drop its
/// own subscription without the server having heard of it, and afterwards what
/// was asked for holds either way — nothing is sent there.
async fn unsubscribe(
    State(state): State<AppState>,
    Json(unsubscribe): Json<Unsubscribe>,
) -> HttpResponse {
    match store::forget_subscription(&state.pool, &unsubscribe.endpoint).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "forgetting a push subscription failed");
            unavailable("the subscription could not be forgotten")
        }
    }
}

/// `GET /api/ui/settings` — what Verkstead has been told: the git author, and
/// that there is a GitHub token.
///
/// Read off the two files at the moment it is asked for, like everything else
/// that reads them: the files are the source of truth, so a token or an author
/// somebody hand-edited into place is what this comes back with.
async fn settings(State(state): State<AppState>) -> HttpResponse {
    Json(as_told(&state.settings)).into_response()
}

/// `POST /api/ui/settings` — write the author down, and set or clear the token.
///
/// Both files in one request, because the page has one button. The token's half
/// is an action rather than a value — see [`TokenEdit`]: most saves are about
/// the author, and a blank write-only field read as *clear this* would take the
/// credentials away every time somebody corrected their own email address.
///
/// A token that was set is then tried against GitHub, and what GitHub said rides
/// back with the save. Tried after the writing and never before it: a token is
/// pasted once, out of a page that will not show it again, and a verification
/// that failed on the network is no reason to make the human go back for
/// another one.
async fn save_settings(
    State(state): State<AppState>,
    Json(edit): Json<SettingsEdit>,
) -> HttpResponse {
    let settings = state.settings.clone();
    let gh = state.github.clone();

    // One blocking hop for the whole save: two files written and, where a token
    // was set, a `gh` run. Everything here is the filesystem or a process, and
    // none of it belongs on the runtime's threads.
    let saved = tokio::task::spawn_blocking(move || {
        settings.save_config(&Config::of_author(GitAuthor::of(
            Some(edit.git_author.name),
            Some(edit.git_author.email),
        )))?;

        let verifying = match &edit.github_token {
            TokenEdit::Keep => None,
            TokenEdit::Set { token } => {
                settings.save_secrets(&Secrets::of_token(Some(token.clone())))?;

                // Read back rather than taken from the request: a token that was
                // only whitespace is nothing configured, and verifying what was
                // typed would announce an account for a token no session will
                // ever be given.
                settings.secrets().github_token().map(str::to_owned)
            }
            TokenEdit::Clear => {
                settings.save_secrets(&Secrets::of_token(None))?;

                None
            }
        };

        let verified = verifying.map(|token| match crate::github::authenticates_as(&gh, &token) {
            Ok(login) => Verified::Account { login },
            Err(trouble) => Verified::Refused { why: trouble.why() },
        });

        Ok::<_, std::io::Error>(SettingsSaved {
            settings: as_told(&settings),
            verified,
        })
    })
    .await;

    match saved {
        Ok(Ok(saved)) => Json(saved).into_response(),
        // The one way this fails: a file that would not be written. Something to
        // try again rather than something to read, so it is a status code and
        // not a named outcome — and worth saying loudly, because a settings page
        // that quietly saved nothing is how credentials go missing.
        Ok(Err(error)) => {
            tracing::error!(error = ?error, "writing the settings files failed");
            unavailable("the settings could not be saved")
        }
        Err(error) => {
            tracing::error!(error = ?error, "saving the settings failed");
            unavailable("the settings could not be saved")
        }
    }
}

/// How the settings stand, read off the files.
///
/// The token comes back as its last four characters and the moment the file was
/// written, and never as itself — see [`verkstead_render::SettingsView`]. A
/// token nobody can read back out is a token that cannot leak through a page,
/// and the four characters are the whole of what the human needs to tell one
/// from another.
fn as_told(settings: &crate::settings::Settings) -> SettingsView {
    let secrets = settings.secrets();
    let config = settings.config();
    let author = config.git_author();

    SettingsView {
        git_author: Author {
            name: author.name().unwrap_or_default().to_owned(),
            email: author.email().unwrap_or_default().to_owned(),
        },
        github_token: secrets.github_token().map(|token| TokenSaved {
            last_four: last_four(token),
            at: settings
                .secrets_written_at()
                .and_then(|at| at.format(&Rfc3339).ok())
                .unwrap_or_default(),
        }),
    }
}

/// The last four characters of a token, or the fewer there are.
///
/// By character rather than by byte: a hand-edited file may hold anything at
/// all, and slicing a string in the middle of a character is a panic in a
/// settings page.
fn last_four(token: &str) -> String {
    let characters: Vec<char> = token.chars().collect();
    let from = characters.len().saturating_sub(4);

    characters[from..].iter().collect()
}

/// `GET /api/ui/update` — whether a newer Verkstead has been released than
/// the one serving this page.
///
/// Answered out of memory and never a request made while the browser waits: the
/// server asks GitHub on its own schedule (see [`crate::updates`]) and this
/// hands over whatever it last concluded. A server that could not find out says
/// there is nothing to update to, which is also what a current one says — there
/// is nothing for the human to do about either.
async fn update(State(state): State<AppState>) -> HttpResponse {
    let notice: UpdateNotice = state.updates.notice();

    Json(notice).into_response()
}

/// There is no such Set to read. Worded with whatever was asked for, which for a
/// typed URL is not a number at all.
fn no_such_set(id: &str) -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new(format!("there is no Question Set {id}")),
    )
}

/// There is no such Conversation to read. Worded like the Set's, and for the
/// same reason: what was asked for is what a typed URL held.
fn no_such_conversation(id: &str) -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new(format!("there is no Conversation {id}")),
    )
}

/// There is no such Repo to read the branches of. Worded like the two above,
/// and for their reason: what was asked for is what a URL held.
fn no_such_repo(id: &str) -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new(format!("there is no Repo {id}")),
    )
}

/// There is no such Capture on that Conversation's Timeline. Worded without
/// either id, unlike the two above: what was asked for is a pair, and a pair
/// that names nothing names nothing for more than one reason.
fn no_such_capture() -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new("there is no such Capture on that Conversation"),
    )
}

/// And no such Transcript, which for the Capture's reason is worded without
/// either id — and for the Capture's reason again is about the Event rather
/// than about the lines: a session that said nothing has a Transcript with
/// nothing on it, and that is an answer rather than a refusal.
fn no_such_transcript() -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new("there is no such Transcript on that Conversation"),
    )
}

/// And no such Screen, which is the same question as no such Capture and
/// worded apart from it all the same: what was asked for is the terminal the
/// bytes were addressed to, and a reader who asked for that should be told
/// there is no such thing rather than about a record they did not ask about.
///
/// Shared with the socket a live one is watched over — see
/// [`crate::screen::attach`] — which refuses the same way for the same reason:
/// a session that is not running has no Screen to attach to.
pub(crate) fn no_such_screen() -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new("there is no such Screen on that Conversation"),
    )
}

/// And no such commit — either the Conversation has no such Event, or the
/// repository no longer has the commit it names. Worded without either id for
/// the Capture's reason, and without telling the two apart because there is
/// nothing different for the human to do about them: the Event is not one this
/// server can show a diff for.
fn no_such_commit() -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new("there is no such commit on that Conversation"),
    )
}

/// And no backlog to open — no Conversation of that id, no Worktree left, or
/// nothing in it this can read as a list. Worded without telling them apart for
/// the commit's reason: there is nothing different for the human to do about
/// any of them.
fn no_such_backlog() -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new("there is no backlog on that Conversation"),
    )
}

/// And no roadmap of that name to open — no Conversation, no Worktree left, or
/// nothing this branch wrote under that name that reads as a roadmap. Worded
/// without telling them apart for the backlog's reason.
fn no_such_roadmap() -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new("there is no roadmap of that name on that Conversation"),
    )
}

/// And no such pull request — either the Conversation has no such Event, or the
/// Event is not one. Worded without either id for the Capture's reason.
fn no_such_pull_request() -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new("there is no such pull request on that Conversation"),
    )
}

fn unavailable(message: &str) -> HttpResponse {
    refused(StatusCode::INTERNAL_SERVER_ERROR, ApiError::new(message))
}

/// A refusal, in the same shape the agent API refuses in: the viewer's fetches
/// then have one thing to read whichever half of the server answered.
fn refused(status: StatusCode, error: ApiError) -> HttpResponse {
    (status, Json(error)).into_response()
}
