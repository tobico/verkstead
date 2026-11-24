//! Opening a pull request: a session running `verkstead pull-request open`,
//! which is how every session opens one since the skills stopped naming a
//! Forge's own tool (ADR-0030, *Forge*).
//!
//! **The server opens it, through the Repo's Forge, as the Repo's Author.** A
//! session knows what the pull request should say and nothing about where the
//! Repo's pull requests live: which Forge that is is read off the remote at the
//! moment of the call — see [`crate::forges::of`] — and the token is the
//! Author's for that Forge, read at the same moment. So one subcommand reads
//! the same on every Forge, and a skill that said `gh pr create` would have
//! been a skill about GitHub.
//!
//! **The branch is asked where it went.** A pull request's head is a branch on
//! the Forge, and which branch that is is whatever the push named — the push
//! being the target repository's own review process rather than Verkstead's.
//! So the head is the name the remote is carrying this branch under, read the
//! way the wrap-up reads it — see [`crate::wrapping`] — and then asked of
//! origin: a branch origin has never heard of is a branch nothing has pushed,
//! which is the first refusal, because a Forge asked to open one would answer
//! in its own words about a ref nobody has heard of.
//!
//! **And it is recorded, the way a found one is.** The row and the Timeline
//! Event [`crate::wrapping`] writes when it finds a pull request, written here
//! the moment the Forge opens one — without the move, which stays the
//! Conversation's ending to make. So the human sees the pull request as soon as
//! it exists, and the ending that follows finds the row already there rather
//! than writing a second one.
//!
//! **No capability gates it.** Every Forge holds a branch's work in something
//! for review, that being what makes it a Forge, so there is nothing here for a
//! declaration to be short of: the five capabilities are what a Forge may
//! *lack* beside that (ADR-0030, *Forge*), and `create` among them is making a
//! repository rather than opening a pull request.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use verkstead_schema::{ApiError, Nudge, PullRequestOpened, PullRequestOpening};

use crate::AppState;
use crate::forges::{Matched, Opening, Trouble};
use crate::reply::yaml;
use crate::store;

/// `POST /conversations/{conversation}/api/v1/pull-requests` — the session
/// running in this Conversation asks the Repo's Forge to open one as a draft,
/// saying what it should hold in the [`PullRequestOpening`] in the body.
///
/// 200 with where it is. 409 where there is no session running here, where the
/// Repo is on no Forge, where the branch is not pushed, where the Author holds
/// no token for this Forge, or where the Forge itself refused.
///
/// A draft, always: merging is the human's act, and nothing here is allowed to
/// look like it was theirs.
pub(crate) async fn open(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
    body: String,
) -> Response {
    let asked = match body.trim().is_empty() {
        true => PullRequestOpening::default(),
        false => match serde_saphyr::from_str::<PullRequestOpening>(&body) {
            Ok(asked) => asked,
            Err(error) => {
                return refused(
                    conversation_id,
                    format!("what the pull request should say isn't YAML: {error}"),
                );
            }
        },
    };

    if state.sessions.writing(conversation_id).is_none() {
        return refused(
            conversation_id,
            "there is no session running in this Conversation, so there is nobody here to open \
             a pull request"
                .to_owned(),
        );
    }

    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            return refused(
                conversation_id,
                "there is no Conversation left to open a pull request for".to_owned(),
            );
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation to open a pull request for failed");

            return yaml(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiError::new("The Conversation couldn't be read, so run it again"),
            );
        }
    };

    let repo = conversation.repo;

    // Read now, off the remote: a Repo on no Forge has nobody to ask, and the
    // sentence names the host so that the human can add it to a Forge's hosts.
    let forge = match crate::forges::of(&state, &repo.path).await {
        Matched::On(forge) => forge,
        Matched::Unmatched { host } => {
            return refused(
                conversation_id,
                match host {
                    Some(host) => format!(
                        "the repo's remote is on {host}, which no enabled forge answers for, so \
                         there is nowhere to open a pull request"
                    ),
                    None => {
                        "no forge is enabled, so there is nowhere to open a pull request".to_owned()
                    }
                },
            );
        }
    };

    let asking = state.github.as_author(repo.author.as_deref());

    // The name the remote is carrying the branch under, and then whether
    // origin is carrying it at all — see the module's notes.
    let pushed = {
        let path = repo.path.clone();
        let branch = conversation.branch.clone();
        let credential = asking.credential();

        // Off the runtime's threads: git is a process, and the second of these
        // goes to the network.
        tokio::task::spawn_blocking(move || {
            let head =
                crate::worktrees::pushed_to(&path, &branch).unwrap_or_else(|| branch.clone());

            crate::checks::pushed_head(&path, Some(&head), &credential).map(|_| head)
        })
        .await
    };

    let head = match pushed {
        Ok(Some(head)) => head,
        Ok(None) => {
            return refused(
                conversation_id,
                format!(
                    "the branch `{}` isn't on origin, so there is nothing on {} to open a pull \
                     request from. Push it — `git push -u origin HEAD` — and run this again",
                    conversation.branch, forge.name
                ),
            );
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading whether the branch is pushed failed");

            return yaml(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiError::new("Whether the branch is pushed couldn't be read, so run it again"),
            );
        }
    };

    // The base the Conversation was cut against where the session named none,
    // and the Repo's own default branch where the record holds neither — which
    // is the rule every other reading of that column is under.
    let base = asked
        .base
        .map(|base| base.trim().to_owned())
        .filter(|base| !base.is_empty())
        .or_else(|| conversation.base_ref.clone())
        .unwrap_or_else(|| repo.default_branch.clone());

    // The branch's own name where the session named no title, which is what a
    // pull request opened with nothing said about it is called.
    let title = asked
        .title
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| conversation.branch.clone());

    let opening = Opening {
        head: head.clone(),
        base: base.clone(),
        title,
        body: asked.body.unwrap_or_default(),
    };

    let path = repo.path.clone();

    // Off the runtime's threads: a Forge goes to the network.
    let opened = match tokio::task::spawn_blocking(move || {
        asking.open_pull_request(&path, &opening)
    })
    .await
    {
        Ok(Ok(opened)) => opened,
        // Nothing falls back on the host's own login (Set 1878, Q3), so an
        // Author with no token for this Forge is said as that and sent to
        // where one is set rather than left with the Forge's own words.
        Ok(Err(Trouble::NotLoggedIn)) => {
            return refused(
                conversation_id,
                format!(
                    "there is no {} token to open a pull request with. Set one for this \
                         repo's Author under Git settings and run this again",
                    forge.name
                ),
            );
        }
        Ok(Err(trouble)) => return refused(conversation_id, trouble.why()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking the Forge to open a pull request failed");

            return yaml(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiError::new("The forge couldn't be asked, so run it again"),
            );
        }
    };

    let url = opened.url.clone();

    // The row and the Event, and no move: what the Conversation does about
    // being on a pull request is its ending's — see the module's notes.
    let recorded = store::record_another_pull_request(
        &state.pool,
        conversation_id,
        repo.id,
        &store::PullRequest {
            number: opened.number,
            title: opened.title,
            url: opened.url,
            head: Some(opened.head),
            base: Some(opened.base),
            // Which repository it is in is the caller's to know, and this one
            // is the Conversation's own — see [`store::PullRequest`].
            repo: None,
        },
    )
    .await;

    match recorded {
        Ok(true) => {
            tracing::info!(
                conversation_id,
                number = opened.number,
                url,
                head,
                base,
                forge = forge.name,
                "a session opened a pull request through the Repo's forge",
            );

            // Something new is pinned above the Timeline, and an open page
            // should say so without being reloaded.
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });
        }
        // Opened and not recorded: the pull request is on the Forge either way,
        // so the session is told where it is rather than told to open a second
        // one, and the ending asks the Forge about the branch regardless.
        Ok(false) => tracing::info!(
            conversation_id,
            number = opened.number,
            "there is no Conversation left to record the pull request against",
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "recording a pull request a session opened failed");
        }
    }

    yaml(StatusCode::OK, &PullRequestOpened { url })
}

fn refused(conversation_id: i64, why: String) -> Response {
    tracing::info!(conversation_id, why, "a session's pull request was refused");

    yaml(StatusCode::CONFLICT, &ApiError::new(why))
}
