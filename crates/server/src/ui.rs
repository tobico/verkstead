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
    Adopted, Archived, BaseCommitOverride, BranchRename, BriefEdit, ConversationAborted,
    ConversationEntry, ConversationView, GrillingStarted, HandedBack, Lifecycle, ManualTaskStarted,
    ManualTaskSubmission, NewAdoption, NewConversation, ProfileChoice, ProfileEdit, ProfileEntry,
    PushKey, Registration, RemedyChoice, RemedySettled, RepoEntry, SetView, Standing, Submitted,
    Subscribed, Subscription, Unsubscribe, UpdateNotice,
};
use verkstead_schema::{ApiError, Nudge, Response};

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
        // And one commit's diff, fetched the same way and for the same reason —
        // see [`commit_diff`].
        .route(
            "/api/ui/conversations/{id}/commit/{event}",
            get(commit_diff),
        )
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
        .route("/api/ui/conversations/{id}/base", post(set_base_commit))
        // The two that make and unmake what a Conversation works in. Named in
        // the path rather than in the verb, as closing a Set unanswered is: the
        // viewer speaks one method.
        .route("/api/ui/conversations/{id}/grill", post(start_grilling))
        // And the press that adopts a roadmap's next stage, which is the
        // grilling start's sibling: what the human presses on an adopting
        // Conversation, there being no Brief to write and no grilling to run.
        .route("/api/ui/conversations/{id}/adopt", post(adopt))
        .route("/api/ui/conversations/{id}/abort", post(abort))
        // And the one press that ends a Hold. Per Conversation rather than per
        // Event, because a Conversation has one keyboard: which of its sessions
        // the human took is the Conversation's own answer, and a route that made
        // them name it would be one they could get wrong.
        //
        // A press rather than anything the socket does. A Hold ends only by
        // being handed back — not by the socket dropping, not by the tab closing
        // — so what ends one is a request of its own, which any device on the
        // tailnet can make and which outlives whatever was watching.
        .route("/api/ui/conversations/{id}/hand-back", post(hand_back))
        // No route for how the work gets built: the direction rides the closing
        // Question Set, and answering one is answering a Set — see
        // [`store::submit_response`].
        //
        // And what the human does about a run that stopped. Per Event rather
        // than per Conversation, because that is what is being answered: the
        // Timeline is where the question was put, and a route that took only
        // the Conversation would answer whichever Interruption happened to be
        // open when it arrived.
        .route(
            "/api/ui/conversations/{id}/interruption/{event}",
            post(settle_interruption),
        )
        // And what the human sets going by hand, wherever nothing is running.
        // Per Conversation rather than per Event, unlike settling an
        // Interruption: a Manual Task answers nothing on the Timeline — it is a
        // new thing to do, and the Event it becomes is written by this.
        .route(
            "/api/ui/conversations/{id}/manual-task",
            post(start_manual_task),
        )
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
        store::Submission::Accepted(taken) => {
            crate::conversations::settle_a_proposal(&state, id, taken.proposed).await;

            if let Some(reviewed) = taken.reviewed {
                crate::review::answered(&state, reviewed);
            }

            Submitted::Accepted
        }
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
/// Two facts ride out on every row beyond what the store holds: whether a
/// session is running on it, and whether it is waiting on the human. Both are
/// read here at the moment the list is drawn, and neither is stored — a running
/// session is a process this server holds, and what is waiting is an `OR` the
/// store computes over rows that move on their own. Which mark either one comes
/// out as is the viewer's, and the rule there is one line: waiting wins.
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

    let rows: Vec<ConversationEntry> = conversations
        .into_iter()
        .map(|conversation| ConversationEntry {
            id: conversation.id,
            branch: conversation.branch,
            repo: conversation.repo,
            state: lifecycle(conversation.state),
            working: working.contains(&conversation.id),
            waiting: conversation.waiting,
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

/// `POST /api/ui/adoptions` — start a Conversation to adopt a roadmap with.
///
/// What clicking a roadmap in the abandoned-roadmaps notice does. It records
/// and opens; nothing about the repository is touched, and nothing is adopted
/// until the human presses Adopt on the page this puts them on.
async fn start_adoption(
    State(state): State<AppState>,
    Json(new): Json<NewAdoption>,
) -> HttpResponse {
    match crate::conversations::start_adopting(&state.pool, new.repo_id, &new.roadmap).await {
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

    // What the worktree holds that is not a moment in the record: the backlog,
    // as `.tasks/` stands right now. Read off the filesystem for the reason the
    // worktree's own missing-ness is — the repository owns those files, and a
    // row remembering what they said would be one more thing to be wrong.
    let mut pinned = crate::tasks::pinned(conversation.worktree.clone()).await;

    // And the roadmap this branch is about, read the same way and for the same
    // reason — `docs/roadmaps/` is the repository's too. Which of a repository's
    // roadmaps is this one's is asked of git against the base commit: a
    // repository keeps its finished roadmaps, and a Conversation is about the
    // one its branch has written to. See [`crate::stages`].
    pinned.extend(
        crate::stages::pinned(
            conversation.worktree.clone(),
            conversation.base_commit.clone(),
        )
        .await,
    );

    // And the pull request the work ended up on, which is pinned beside it. This
    // one *is* on the record — it is what moved the Conversation into Wrapping —
    // so it is read off the Timeline for the reason the Brief is: it is already
    // here. What is not read here is what the PR holds, which is a request of its
    // own; see [`pull_request`].
    pinned.extend(timeline.iter().rev().find_map(|event| match &event.event {
        store::Event::PullRequest(opened) => Some(verkstead_render::pull_request_event(
            event.id,
            event.at.clone(),
            verkstead_render::PullRequestSummary {
                number: opened.number,
                title: opened.title.clone(),
                url: opened.url.clone(),
            },
        )),
        _ => None,
    }));

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

    // And what this Conversation is adopting, where it is adopting anything:
    // the roadmap named and the stage the Adopt press would start, read off the
    // Repo at the base commit rather than out of any row. Only the roadmap's
    // name is stored — see [`crate::stages::adopting`].
    let adopting = match conversation.adopting.clone() {
        None => None,
        Some(roadmap) => Some(
            crate::stages::adopting(
                conversation.repo.clone(),
                conversation.base_commit.clone(),
                roadmap,
            )
            .await,
        ),
    };

    // What the work has stopped on, read off the Timeline for the reason the
    // Brief is: it is already here. The store's index makes at
    // most one open, so the last one that is unsettled is the one — and it is
    // the *only* one, which is what makes *the run stops here* a fact rather
    // than a promise.
    let stopped_at = timeline.iter().rev().find_map(|event| match &event.event {
        store::Event::Interruption(interruption) if interruption.settled.is_none() => {
            Some(event.id)
        }
        _ => None,
    });

    // And what the human has taken the keyboard of, which is the other thing the
    // work can be stopped on and the one that is nowhere on the Timeline: a Hold
    // leaves no Event, because the Timeline records the work rather than the
    // watching. Read off the running server, which is the only thing that knows.
    let held = state.sessions.holding(id);

    // The badge points at whichever of the two is in force, and at the Hold
    // first. The two cannot really be open together — an Interruption is raised
    // about a session that is over, and nothing raises one behind a Hold — and
    // where they somehow are, the keyboard is the thing the human is holding
    // right now.
    let blocked_on = held.or(stopped_at);

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
        adopting,
        grilling_profile,
        implementation_profile,
        worktree,
        direction: conversation.direction,
        pinned,
        blocked_on,
        held,
        // The same reading the Events above are drawn against, said as a fact
        // about the Conversation: the Timeline offers the Manual Task composer
        // exactly where nothing is running, and one Event of a session's is not
        // the question — a Conversation whose session has ended is not working,
        // whichever Event it was writing into.
        working: writing.is_some(),
        timeline: timeline
            .into_iter()
            // `filter_map` rather than `map`, for the one Event that is on the
            // record and not in the list: the pull request is drawn pinned above
            // the Timeline — see `pinned` above — and what says on the record
            // that it arrived is the move into Wrapping right beside it.
            .filter_map(|event| {
                Some(match event.event {
                    // Rendered on the way out where there is markdown to render —
                    // see [`verkstead_render`]. A move has none: it is one state.
                    store::Event::Brief(markdown) => {
                        verkstead_render::brief_event(event.id, event.at, markdown)
                    }
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
                        let standing =
                            standing(&state, asked.set_id, asked.settlement, &event.at, now);

                        verkstead_render::question_set_event(
                            event.id,
                            event.at,
                            asked.set_id,
                            &asked.set,
                            standing,
                        )
                    }
                    // Rendered like the Brief, and inline like it: a document to
                    // read, with nothing of it a details pane would add.
                    store::Event::Handoff(markdown) => {
                        verkstead_render::handoff_event(event.id, event.at, &markdown)
                    }
                    // The counts and the subject, and not the diff: the diff is in
                    // the repository, and what fetches it is the pane that shows it.
                    store::Event::Commit(commit) => verkstead_render::commit_event(
                        event.id,
                        event.at,
                        verkstead_render::CommitSummary {
                            sha: commit.sha,
                            subject: commit.subject,
                            files: commit.files,
                            insertions: commit.insertions,
                            deletions: commit.deletions,
                        },
                    ),
                    // Whole, evidence and all, unlike the three above it. The
                    // evidence was bounded when it was gathered — see the server's
                    // `interruptions` module — and the remedies are chosen against
                    // it, so a page that had to fetch it separately could draw the
                    // buttons before it could say what they were for.
                    store::Event::Interruption(interruption) => {
                        verkstead_render::interruption_event(
                            event.id,
                            event.at,
                            stopped(*interruption),
                        )
                    }
                    // Rendered like the handoff and inline like it, being the
                    // other kind of sentence somebody has to be able to read
                    // back — and the one nobody wrote for a human to press
                    // anything about.
                    store::Event::Notice(markdown) => {
                        verkstead_render::notice_event(event.id, event.at, &markdown)
                    }
                    // And what the human asked for by hand, rendered like the
                    // handoff and inline like it: the instruction is the whole
                    // of what a Manual Task is on the record, and what its
                    // session did lands beside it as its own Events.
                    store::Event::ManualTask(instruction) => {
                        verkstead_render::manual_task_event(event.id, event.at, &instruction)
                    }
                    // The one kind that is not in the list: it is drawn pinned
                    // above the Timeline instead. Dropped by name rather than by
                    // a catch-all, so a kind added later has to be decided about
                    // rather than silently disappearing.
                    store::Event::PullRequest(_) => return None,
                })
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

/// `GET /api/ui/conversations/{id}/transcript/{event}` — what one session
/// said, as a conversation.
///
/// Its own request rather than a field on the Conversation, for the Capture's
/// reason and to the same size: this is an hour of talking, and the Timeline is
/// re-read every time an open page hears the world moved.
///
/// The lines were stored verbatim and are read here, on the way out, which is
/// what keeps the coupling to somebody else's file format to the one crate that
/// has the parsers in it (ADR 0006). An empty Transcript is an ordinary answer
/// and not a failure: it is every session that left no log, and the pane's
/// answer to one is to show the Capture instead.
async fn transcript(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
) -> HttpResponse {
    // Read as permissively as every other pair of ids here: neither of them
    // naming a number cannot name a Transcript.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return no_such_transcript();
    };

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

/// `GET /api/ui/conversations/{id}/commit/{event}` — one commit's diff,
/// rendered.
///
/// Its own request rather than a field on the Conversation, exactly as a
/// Capture is: a Timeline is read every time an open page hears the world
/// moved, and a diff is worth reading when somebody opens the one Event it
/// belongs to.
///
/// Read out of the repository rather than out of the store. The commit is in git
/// — that is what a commit *is* — and keeping a second copy of every patch would
/// be a database growing with the work rather than with the record of it. What
/// the store holds is the line the Timeline draws.
async fn commit_diff(
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

    // Which repository to read it out of, which is the Conversation's own. A
    // Conversation that has a commit on its Timeline and no row of its own is a
    // record that has been got at.
    let repo = match store::load_conversation(&state.pool, id).await {
        Ok(Some(conversation)) => conversation.repo.path,
        Ok(None) => return no_such_commit(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "loading a Conversation failed");
            return unavailable("the commit could not be read");
        }
    };

    let patch = match tokio::task::spawn_blocking(move || crate::commits::patch(&repo, &commit.sha))
        .await
    {
        Ok(patch) => patch,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "reading a commit's diff failed");
            return unavailable("the commit could not be read");
        }
    };

    // A commit the repository will not say anything about is one that has gone —
    // collected, or on a branch somebody rewrote. There is nothing to draw a pane
    // about, which is what a 404 means everywhere else here.
    let Some(patch) = patch else {
        return no_such_commit();
    };

    Json(verkstead_render::commit_diff(&patch)).into_response()
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
async fn pull_request(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
) -> HttpResponse {
    // Two ids out of a URL a human may have typed, read as permissively as every
    // other pair here.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return no_such_pull_request();
    };

    // Which PR, and which repository to ask about it in. Both come off the
    // Conversation's own record: the Event says which pull request, and an Event
    // id belonging to another Conversation names nothing here.
    let conversation = match store::load_conversation(&state.pool, id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return no_such_pull_request(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "loading a Conversation failed");
            return unavailable("the pull request could not be read");
        }
    };

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

    let gh = state.github.clone();
    let repo = conversation.repo.path;

    let asked =
        tokio::task::spawn_blocking(move || crate::github::details(&gh, &repo, opened.number))
            .await;

    match asked {
        Ok(Ok(details)) => Json(details).into_response(),
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

/// `POST /api/ui/conversations/{id}/interruption/{event}` — what the human is
/// doing about a run that stopped.
///
/// One press for the choice and the doing: retry
/// launches a fresh session for the same step, taking whatever was written
/// alongside; take over stops Verkstead driving; abort ends the run. In every
/// case the repository is left as the session left it.
///
/// `AlreadySettled` is an outcome rather than an error, for the reason every
/// other named outcome here is one: the human answers from whichever device is to
/// hand, and the second press of a button is something to say in words rather
/// than something to retry.
async fn settle_interruption(
    State(state): State<AppState>,
    Path((id, event)): Path<(String, String)>,
    Json(choice): Json<RemedyChoice>,
) -> HttpResponse {
    // Two ids out of a URL a human may have typed, read as permissively as every
    // other pair here: neither of them naming a number cannot name an
    // Interruption.
    let (Ok(id), Ok(event)) = (id.parse::<i64>(), event.parse::<i64>()) else {
        return Json(RemedySettled::NoSuchInterruption).into_response();
    };

    match crate::interruptions::settle(&state, id, event, choice.remedy, &choice.note).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, event_id = event, "settling an Interruption failed");
            unavailable("the interruption could not be settled")
        }
    }
}

/// `POST /api/ui/conversations/{id}/manual-task` — do this one thing by hand.
///
/// The instruction goes on the Timeline and a one-off session starts on it under
/// the Profile the human picked beside it. Nothing about the Conversation moves:
/// a Manual Task is outside the pipeline, and what it leaves behind is its
/// instruction, what its session printed and whatever that committed.
///
/// `AlreadyRunning` is an outcome rather than an error, for the reason every
/// other named outcome here is one: the composer that was pressed was drawn a
/// moment ago, and an agent having started since is something to say in words
/// rather than something to retry.
async fn start_manual_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(submission): Json<ManualTaskSubmission>,
) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(ManualTaskStarted::NoSuchConversation).into_response();
    };

    match crate::manual::submit(&state, id, &submission.instruction, submission.profile_id).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id = id, "starting a manual task failed");
            unavailable("the manual task could not be started")
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

/// `POST /api/ui/conversations/{id}/hand-back` — the human gives the keyboard
/// back, and the Hold is over.
///
/// The only way one ends. Nothing here judges anything: what the human left is
/// judged by whichever driver was waiting at the gate, by the ordinary
/// end-of-session rules — the Step's commit landed so the run goes on, or it did
/// not and there is an Interruption. Handing back is what lets that question be
/// asked, not what answers it.
///
/// Refused for nothing. A Conversation that is not held is one already handed
/// back, which is the same answer arriving twice rather than a mistake.
async fn hand_back(State(state): State<AppState>, Path(id): Path<String>) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return Json(HandedBack::NotHeld).into_response();
    };

    if !state.sessions.hand_back(id) {
        return Json(HandedBack::NotHeld).into_response();
    }

    tracing::info!(
        conversation_id = id,
        "the keyboard has been handed back, so Verkstead has this Conversation again",
    );

    // The badge goes with it, and it is drawn off the Conversation.
    state
        .nudges
        .announce(Nudge::Conversation { conversation: id });

    Json(HandedBack::HandedBack).into_response()
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

/// An Interruption as the viewer receives it: the evidence, and how it was
/// settled if it was.
///
/// The one Event whose whole self rides on the Timeline. Held to the viewer's
/// vocabulary here, as a lifecycle state is, because the two enums are the same
/// three remedies said in two crates that do not depend on each other.
fn stopped(interruption: store::Interruption) -> verkstead_render::Stopped {
    verkstead_render::Stopped {
        what: interruption.evidence.what,
        how: interruption.evidence.how,
        git_status: interruption.evidence.git_status,
        tail: interruption.evidence.tail,
        settled: interruption
            .settled
            .map(|settled| verkstead_render::RemedyTaken {
                remedy: remedy(settled.remedy),
                note: settled.note,
                at: settled.at,
            }),
    }
}

/// The store's word for a remedy as the viewer receives it, the other way round
/// from [`crate::interruptions`]'s: this is what was chosen, and that is what to
/// choose.
fn remedy(remedy: store::Remedy) -> verkstead_render::Remedy {
    match remedy {
        store::Remedy::Retry => verkstead_render::Remedy::Retry,
        store::Remedy::TakeOver => verkstead_render::Remedy::TakeOver,
        store::Remedy::Abort => verkstead_render::Remedy::Abort,
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
