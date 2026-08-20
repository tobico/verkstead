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
    Archived, BaseCommitOverride, BranchRename, BriefEdit, ConversationAborted, ConversationEntry,
    ConversationView, GrillingStarted, Lifecycle, NewConversation, ProfileChoice, ProfileEdit,
    ProfileEntry, PushKey, Registration, RepoEntry, SetView, Standing, Submitted, Subscribed,
    Subscription, Unsubscribe, UpdateNotice,
};
use verkstead_schema::{ApiError, Response};

use crate::{AppState, store};

/// The viewer's routes, over the state the agent API is already holding: a
/// submit from the browser has to reach an agent waiting on the REST endpoint,
/// so both halves settle Sets through the same channel and read Liveness out of
/// the same registry of held waits.
pub(crate) fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/ui/sets/{id}", get(set))
        .route("/api/ui/sets/{id}/response", post(submit_response))
        .route("/api/ui/sets/{id}/archive", post(archive_set))
        .route("/api/ui/repos", get(repos).post(register_repo))
        .route(
            "/api/ui/conversations",
            get(conversations).post(start_conversation),
        )
        .route("/api/ui/conversations/{id}", get(conversation))
        // One Event's full self, fetched by the pane that shows it rather than
        // carried by the Conversation — see [`transcript`].
        .route(
            "/api/ui/conversations/{id}/transcript/{event}",
            get(transcript),
        )
        .route("/api/ui/conversations/{id}/brief", post(save_brief))
        .route("/api/ui/conversations/{id}/branch", post(rename_branch))
        .route("/api/ui/conversations/{id}/base", post(set_base_commit))
        // The two that make and unmake what a Conversation works in. Named in
        // the path rather than in the verb, as closing a Set unanswered is: the
        // viewer speaks one method.
        .route("/api/ui/conversations/{id}/grill", post(start_grilling))
        .route("/api/ui/conversations/{id}/abort", post(abort))
        .route(
            "/api/ui/conversations/{id}/grilling-profile",
            post(choose_grilling_profile),
        )
        .route(
            "/api/ui/conversations/{id}/implementation-profile",
            post(choose_implementation_profile),
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

    let standing = standing(
        &state,
        id,
        settlement,
        &stored.created_at,
        OffsetDateTime::now_utc(),
    );

    // Everything the agent wrote, rendered — which is the whole of what is left
    // to do, and none of it this crate's.
    let view: SetView = verkstead_render::set_view(stored.id, conversation, stored.set, standing);

    Json(view).into_response()
}

/// Where a Set stands, as both its own page and its row on a Timeline read it.
///
/// The Liveness comes out of the registry of held waits, which is the same
/// registry either way: whichever of the two the human is looking at, it is the
/// page they act on.
fn standing(
    state: &AppState,
    set_id: i64,
    settlement: Option<store::Settlement>,
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
        Some(store::Settlement::ArchivedUnanswered(archived)) => {
            Standing::ArchivedUnanswered(archived.archived_at)
        }
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

    // The two Profiles are read as rows rather than as ids: what the pane says
    // about one, and whether it can still be run under, is the same reading the
    // Profile list gets.
    let grilling_profile = match crate::profiles::entry(
        &state.watched,
        conversation.grilling_profile,
    )
    .await
    {
        Ok(profile) => profile,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading a grilling Profile failed");
            return unavailable("the Conversation could not be read");
        }
    };

    let implementation_profile = match crate::profiles::entry(
        &state.watched,
        conversation.implementation_profile,
    )
    .await
    {
        Ok(profile) => profile,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading an implementation Profile failed");
            return unavailable("the Conversation could not be read");
        }
    };

    // Whether the worktree is still on disk, which is a look at the filesystem
    // rather than anything the store knows.
    let worktree = match crate::conversations::worktree(conversation.worktree).await {
        Ok(worktree) => worktree,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "reading a worktree failed");
            return unavailable("the Conversation could not be read");
        }
    };

    // The Brief decides whether the Conversation is ready to grill, so it is
    // read off the Timeline before the Timeline is spent building the view.
    let brief = timeline
        .iter()
        .find_map(|event| match &event.event {
            store::Event::Brief(markdown) => Some(markdown.as_str()),
            _ => None,
        })
        .unwrap_or_default();

    let ready_to_grill = crate::conversations::ready_to_grill(
        conversation.state,
        grilling_profile.as_ref(),
        implementation_profile.as_ref(),
        brief,
    );

    // Which of this Conversation's output Events is still being written into,
    // which is a question about a process rather than about the record: a
    // restarted server has no sessions, and every transcript it holds is of one
    // that is over.
    let writing = state.sessions.writing(id);

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
        state: lifecycle(conversation.state),
        ready_to_grill,
        grilling_profile,
        implementation_profile,
        worktree,
        timeline: timeline
            .into_iter()
            .map(|event| match event.event {
                // Rendered on the way out where there is markdown to render —
                // see [`verkstead_render`]. A move has none: it is one state.
                store::Event::Brief(markdown) => {
                    verkstead_render::brief_event(event.id, event.at, markdown)
                }
                store::Event::Moved(state) => {
                    verkstead_render::moved_event(event.id, event.at, lifecycle(state))
                }
                // The summary and not the transcript: a Timeline is read every
                // time an open page hears the world moved, and a session's
                // output is megabytes the middle pane never shows.
                store::Event::AgentOutput(summary) => verkstead_render::agent_output_event(
                    event.id,
                    event.at,
                    summary.lines,
                    summary.latest,
                    writing == Some(event.id),
                ),
                // The table of what was asked against what was decided, and no
                // more: the whole document is what the details pane fetches,
                // from the endpoint one Set has always been read through.
                //
                // The Event's own stamp is what the Liveness verdict is aged
                // against. It is the Set's creation time — both are written in
                // the one transaction that puts a Set on a Timeline.
                store::Event::QuestionSet(asked) => {
                    let standing = standing(&state, asked.set_id, asked.settlement, &event.at, now);

                    verkstead_render::question_set_event(
                        event.id,
                        event.at,
                        asked.set_id,
                        &asked.set,
                        standing,
                    )
                }
            })
            .collect(),
    };

    Json(view).into_response()
}

/// `GET /api/ui/conversations/{id}/transcript/{event}` — what one session
/// printed, whole.
///
/// Its own request rather than a field on the Conversation, because of the two
/// sizes involved. A session prints megabytes over an hour and the Timeline is
/// re-read every time an open page hears the world moved; the transcript is read
/// when somebody opens the one Event it belongs to.
///
/// Byte for byte, control sequences and all — what a terminal was sent is what
/// the session said.
async fn transcript(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
) -> HttpResponse {
    // Two ids out of a URL a human may have typed, and neither of them naming a
    // number cannot name a transcript — the same permissiveness every other id
    // here is read with.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return no_such_transcript();
    };

    match store::transcript(&state.pool, id, event).await {
        Ok(Some(text)) => Json(verkstead_render::Transcript { text }).into_response(),
        Ok(None) => no_such_transcript(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading a transcript failed");
            unavailable("the transcript could not be read")
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

/// `POST /api/ui/conversations/{id}/abort` — stop it wherever it has got to.
async fn abort(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ConversationAborted::NoSuchConversation).into_response();
    };

    match crate::conversations::abort(&state, id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "aborting a Conversation failed");
            unavailable("the conversation could not be aborted")
        }
    }
}

/// `POST /api/ui/conversations/{id}/grilling-profile` — which account and model
/// the grilling session runs under.
async fn choose_grilling_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(choice): Json<ProfileChoice>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::ProfileChosen::NoSuchConversation).into_response();
    };

    match crate::profiles::choose_grilling(&state.pool, id, choice.profile_id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "choosing a grilling Profile failed");
            unavailable("the grilling profile could not be chosen")
        }
    }
}

/// `POST /api/ui/conversations/{id}/implementation-profile` — and the one the
/// implementation runs under, which is a separate choice.
async fn choose_implementation_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(choice): Json<ProfileChoice>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(verkstead_render::ProfileChosen::NoSuchConversation).into_response();
    };

    match crate::profiles::choose_implementation(&state.pool, id, choice.profile_id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "choosing an implementation Profile failed");
            unavailable("the implementation profile could not be chosen")
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
        store::Lifecycle::Aborted => Lifecycle::Aborted,
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

/// There is no such transcript on that Conversation's Timeline. Worded without
/// either id, unlike the two above: what was asked for is a pair, and a pair
/// that names nothing names nothing for more than one reason.
fn no_such_transcript() -> HttpResponse {
    refused(
        StatusCode::NOT_FOUND,
        ApiError::new("there is no such transcript on that Conversation"),
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
