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
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as HttpResponse};
use axum::routing::{get, post};
use time::OffsetDateTime;
use verkstead_render::{
    ArchiveEntry, Archived, BaseCommitOverride, BriefEdit, BranchRename, ConversationEntry,
    ConversationView, Lifecycle, NewConversation, PendingEntry, PushKey, Registration, RepoEntry,
    SetView, Standing, Submitted, Subscribed, Subscription, Unsubscribe, UpdateNotice,
};
use verkstead_schema::{ApiError, Response};

use crate::{AppState, store};

/// The viewer's routes, over the state the agent API is already holding: a
/// submit from the browser has to reach an agent waiting on the REST endpoint,
/// so both halves settle Sets through the same channel and read Liveness out of
/// the same registry of held waits.
pub(crate) fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/ui/pending", get(pending))
        .route("/api/ui/archive", get(archive))
        .route("/api/ui/sets/{id}", get(set))
        .route("/api/ui/sets/{id}/response", post(submit_response))
        .route("/api/ui/sets/{id}/archive", post(archive_set))
        .route("/api/ui/repos", get(repos).post(register_repo))
        .route(
            "/api/ui/conversations",
            get(conversations).post(start_conversation),
        )
        .route("/api/ui/conversations/{id}", get(conversation))
        .route("/api/ui/conversations/{id}/brief", post(save_brief))
        .route("/api/ui/conversations/{id}/branch", post(rename_branch))
        .route("/api/ui/conversations/{id}/base", post(set_base_commit))
        // Not a thing to fetch but a thing to listen on — see [`crate::nudge`].
        .route("/api/ui/nudges", get(crate::nudge::nudges))
        .route("/api/ui/push/key", get(push_key))
        .route("/api/ui/push/subscribe", post(subscribe))
        .route("/api/ui/push/unsubscribe", post(unsubscribe))
        .route("/api/ui/update", get(update))
}

/// `GET /api/ui/pending` — the Sets still waiting on the human, newest first.
async fn pending(State(state): State<AppState>) -> HttpResponse {
    let now = OffsetDateTime::now_utc();

    let pending = match store::pending_sets(&state.pool).await {
        Ok(pending) => pending,
        Err(error) => {
            tracing::error!(error = ?error, "reading the pending Sets failed");
            return unavailable("the pending Sets could not be read");
        }
    };

    let rows: Vec<PendingEntry> = pending
        .into_iter()
        .map(|set| PendingEntry {
            id: set.id,
            title: set.title,
            project: set.project,
            branch: set.branch,
            // All three already decided here rather than sent as timestamps:
            // this is the side with the clock and with the registry of held
            // waits.
            age: verkstead_render::relative_age(&set.created_at, now),
            created_stamp: verkstead_render::utc_stamp(&set.created_at),
            liveness: state.waits.liveness(set.id, &set.created_at, now),
        })
        .collect();

    Json(rows).into_response()
}

/// `GET /api/ui/archive` — the Sets that have been settled, newest first.
async fn archive(State(state): State<AppState>) -> HttpResponse {
    let now = OffsetDateTime::now_utc();

    let archived = match store::archived_sets(&state.pool).await {
        Ok(archived) => archived,
        Err(error) => {
            tracing::error!(error = ?error, "reading the Archive failed");
            return unavailable("the Archive could not be read");
        }
    };

    let rows: Vec<ArchiveEntry> = archived
        .into_iter()
        .map(|set| ArchiveEntry {
            id: set.id,
            title: set.title,
            project: set.project,
            branch: set.branch,
            settled_at: verkstead_render::settled_age(&set.settled_at, now),
            settled_stamp: verkstead_render::utc_stamp(&set.settled_at),
            unanswered: set.settled == store::Settled::ArchivedUnanswered,
        })
        .collect();

    Json(rows).into_response()
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

    let standing = match settlement {
        Some(store::Settlement::Answered(answered)) => {
            Standing::Answered(verkstead_render::Answered {
                submitted_at: answered.submitted_at,
                response: answered.response,
            })
        }
        Some(store::Settlement::ArchivedUnanswered(archived)) => {
            Standing::ArchivedUnanswered(archived.archived_at)
        }
        // The same verdict the pending list's row carries, from the same
        // registry: this is the page it is acted on.
        None => Standing::Waiting(state.waits.liveness(
            id,
            &stored.created_at,
            OffsetDateTime::now_utc(),
        )),
    };

    // Everything the agent wrote, rendered — which is the whole of what is left
    // to do, and none of it this crate's.
    let view: SetView = verkstead_render::set_view(stored.id, stored.set, standing);

    Json(view).into_response()
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
        store::Submission::Accepted(_) => Submitted::Accepted,
        store::Submission::AlreadyAnswered => Submitted::AlreadyAnswered,
        store::Submission::NoSuchSet => Submitted::NoSuchSet,
        store::Submission::Archived => Submitted::Archived,
        store::Submission::Invalid(invalid) => {
            Submitted::Rejected(invalid.violations.iter().map(ToString::to_string).collect())
        }
    })
    .into_response()
}

/// `POST /api/ui/sets/{id}/archive` — close a Set unanswered.
///
/// The human declaring that nobody is ever going to answer it, so it stops being
/// something that is waiting on them. Only ever reached from a browser
/// (ADR-0001) — the agent API has no route for it, because a disconnected agent
/// is not evidence: the CLI reconnects through transient drops.
async fn archive_set(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(Archived::NoSuchSet).into_response();
    };

    let archiving = match store::archive_set(&state.pool, &state.settlements, id).await {
        Ok(archiving) => archiving,
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "archiving a Set failed");
            return unavailable("the Question Set could not be archived");
        }
    };

    Json(match archiving {
        store::Archiving::Archived(_) => Archived::Closed,
        store::Archiving::AlreadyAnswered => Archived::AlreadyAnswered,
        store::Archiving::AlreadyArchived => Archived::AlreadyArchived,
        store::Archiving::NoSuchSet => Archived::NoSuchSet,
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

/// `GET /api/ui/conversations` — the sidebar, newest first.
async fn conversations(State(state): State<AppState>) -> HttpResponse {
    let conversations = match store::conversations(&state.pool).await {
        Ok(conversations) => conversations,
        Err(error) => {
            tracing::error!(error = ?error, "reading the Conversations failed");
            return unavailable("the Conversations could not be read");
        }
    };

    let rows: Vec<ConversationEntry> = conversations
        .into_iter()
        .map(|conversation| ConversationEntry {
            id: conversation.id,
            branch: conversation.branch,
            repo: conversation.repo,
            state: lifecycle(conversation.state),
        })
        .collect();

    Json(rows).into_response()
}

/// `POST /api/ui/conversations` — start one against a registered Repo.
async fn start_conversation(
    State(state): State<AppState>,
    Json(new): Json<NewConversation>,
) -> HttpResponse {
    match crate::conversations::start(&state.pool, new.repo_id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, "starting a Conversation failed");
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

    let timeline = match store::timeline(&state.pool, id).await {
        Ok(timeline) => timeline,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading a Timeline failed");
            return unavailable("the Conversation could not be read");
        }
    };

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
        state: lifecycle(conversation.state),
        timeline: timeline
            .into_iter()
            .map(|event| match event.event {
                // The one kind there is. Rendered on the way out like everything
                // else made of markdown — see [`verkstead_render`].
                store::Event::Brief(markdown) => {
                    verkstead_render::brief_event(event.id, event.at, markdown)
                }
            })
            .collect(),
    };

    Json(view).into_response()
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

/// `POST /api/ui/conversations/{id}/base` — override the base commit, or put the
/// Conversation back on the default-branch rule.
async fn set_base_commit(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(override_): Json<BaseCommitOverride>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::BaseRecorded::NoSuchConversation).into_response();
    };

    match crate::conversations::set_base_commit(&state.pool, id, override_.commit.as_deref()).await
    {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "recording a base commit failed");
            unavailable("the base commit could not be recorded")
        }
    }
}

/// The store's lifecycle state as the viewer receives it. One word either side,
/// and this is where the two vocabularies are held to each other.
fn lifecycle(state: store::Lifecycle) -> Lifecycle {
    match state {
        store::Lifecycle::Draft => Lifecycle::Draft,
        store::Lifecycle::Grilling => Lifecycle::Grilling,
        store::Lifecycle::Direction => Lifecycle::Direction,
        store::Lifecycle::Implementing => Lifecycle::Implementing,
        store::Lifecycle::Wrapping => Lifecycle::Wrapping,
        store::Lifecycle::Done => Lifecycle::Done,
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

fn unavailable(message: &str) -> HttpResponse {
    refused(StatusCode::INTERNAL_SERVER_ERROR, ApiError::new(message))
}

/// A refusal, in the same shape the agent API refuses in: the viewer's fetches
/// then have one thing to read whichever half of the server answered.
fn refused(status: StatusCode, error: ApiError) -> HttpResponse {
    (status, Json(error)).into_response()
}
