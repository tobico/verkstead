//! One of this device's Agent Profiles as a member is handed the **memory store**
//! of its account — what it remembers of one repository, and what one
//! Conversation wrote — and is given back what a session away from home left in
//! it (ADR-0020, *Shared Profiles*).
//!
//! **The answering half of the memory sync.** A device launching under a Profile
//! whose account lives here syncs this Repo's entries into its own mirror of that
//! account before launch and back after, so this is what they are read from and
//! written into. [`crate::mirroring::memory`] is the end that asks, and the whole
//! of the reasoning is there.
//!
//! **This device names its own paths.** What arrives is three facts about the
//! asking device's side — which of *this* device's Repos this repository is, what
//! that Conversation's Worktree is called, and every session id it has had — and
//! the parts of the store are named off this machine's own directories from them:
//! its Repo's path, and its own worktrees directory under its Data Directory with
//! the same stem on the end, which is the path this device *would* have used for
//! that work. Nothing that arrives is ever joined onto a directory here as a path.
//!
//! **A press rather than a path**, both ways, because the question does not fit
//! in one: it is three facts, one of them a list. So the read is a `POST` whose
//! body is the question and whose answer is the files.
//!
//! **A member's own call rather than the viewer's**, gated to members with
//! everything else on this listener, and **only this device's own Profiles**: a
//! row that is itself a mirror is an account on a third machine, and this device
//! has no store of it to hand over. Beside [`super::account`], which carries the
//! same account's login and configuration, for that module's reasons.
//!
//! **And only where the Profile's memory switch is on.** Off, the account's store
//! is not a session's at home either, so there is nothing here to lend and nothing
//! to take back — answered as an empty store rather than refused, which is what a
//! Profile with the switch off *has*.

use axum::Json;
use axum::Router;
use axum::extract::{DefaultBodyLimit, Path as At, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use std::path::{Path, PathBuf};
use verkstead_render::{MemoryLeft, MemoryWanted};

use crate::mirroring::memory::{MOST_A_STORE_IS, carried, gathered, written_down};
use crate::sandbox::root::{Part, Root};
use crate::{AppState, store};

/// Where a member asks after the memory store of one of this device's Profiles,
/// by the id it has **here** — which is the id the mirror row records for it.
///
/// Under the Profile beside its account rather than inside it: what an account
/// hands over is what a root is *made* from, composed fresh each launch, and a
/// memory store is joined rather than composed and is synced on its own terms.
pub const MEMORY: &str = "/api/peer/v1/profiles/{profile}/memory";

/// And where a device away from home puts what its session **left** in that
/// store.
pub const LEFT: &str = "/api/peer/v1/profiles/{profile}/memory/left";

/// `POST /api/peer/v1/profiles/{profile}/memory` — every file of that Profile's
/// memory store that belongs to the repository and the Conversation the body
/// names, each under the part of the store it is in.
///
/// **Read at the moment it is asked for**, as the account beside it is: a memory
/// written here since the last session away is the one the next session away has
/// to be given.
///
/// **A Profile that is not one of this device's own is `404`**, and so is one that
/// is not there at all — a row removed between a mirror's last refresh and this
/// call.
pub(crate) async fn held(
    State(state): State<AppState>,
    At(profile): At<i64>,
    Json(wanted): Json<MemoryWanted>,
) -> Response {
    let Some((account, parts)) = named(&state, profile, &wanted).await else {
        return (
            StatusCode::NOT_FOUND,
            "this device holds no Agent Profile of its own under that id\n",
        )
            .into_response();
    };

    match tokio::task::spawn_blocking(move || gathered(&account, &parts)).await {
        Ok(Ok(files)) => Json(files).into_response(),

        Ok(Err(why)) => {
            tracing::error!(
                profile,
                "the memory store a member asked after could not be read off the disk: {why:#}",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own memory store\n",
            )
                .into_response()
        }

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile,
                "reading the memory store a member asked after ended badly",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own memory store\n",
            )
                .into_response()
        }
    }
}

/// `POST /api/peer/v1/profiles/{profile}/memory/left` — what a session away from
/// home wrote to that store, written into it here.
///
/// **Written into the parts this device names**, out of the same question the read
/// was made with: a file under a label this device holds no part for is dropped,
/// and so is one whose path inside would land anywhere but under that part — see
/// [`crate::mirroring::memory::written_down`], which is the same write a mirror is
/// made by.
///
/// **Nothing is deleted.** What arrives is only what the session wrote or made,
/// and everything else in that store is the human's own work in other
/// repositories.
pub(crate) async fn written(
    State(state): State<AppState>,
    At(profile): At<i64>,
    Json(left): Json<MemoryLeft>,
) -> Response {
    let MemoryLeft { wanted, files } = left;

    let Some((account, parts)) = named(&state, profile, &wanted).await else {
        return (
            StatusCode::NOT_FOUND,
            "this device holds no Agent Profile of its own under that id\n",
        )
            .into_response();
    };

    let carrying = files.len();

    match tokio::task::spawn_blocking(move || written_down(&account, &parts, &files)).await {
        Ok(Ok(_)) => {
            tracing::debug!(
                profile,
                carrying,
                "what a member's session wrote to its memory store was written into this \
                 device's account",
            );

            StatusCode::NO_CONTENT.into_response()
        }

        Ok(Err(why)) => {
            tracing::error!(
                profile,
                "what a member's session wrote to its memory store could not be written into \
                 this device's account: {why:#}",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not write into its own memory store\n",
            )
                .into_response()
        }

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile,
                "writing what a member's session left in its memory store ended badly",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not write into its own memory store\n",
            )
                .into_response()
        }
    }
}

/// The account of this device's own Profile `profile`, and the parts of its
/// memory store that `wanted` names — each at a path of **this** machine's own.
///
/// `None` is a Profile that is not this device's own, or not there at all. A
/// Profile whose memory switch is off is no parts rather than no Profile: the
/// switch says its store is nobody's session's, here or away, and an empty
/// answer is what that is.
///
/// Where a Repo is named it is this device's registry that says where it is, and
/// where a Worktree is named it is this device's own worktrees directory the stem
/// is joined onto — the path this device would have cut that work in. Where
/// neither is named, the part it would have been is left out on both ends — see
/// [`carried`].
async fn named(
    state: &AppState,
    profile: i64,
    wanted: &MemoryWanted,
) -> Option<(PathBuf, Vec<Part>)> {
    let held = match store::load_profile(&state.pool, profile).await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile,
                "a member asked after one of this device's memory stores and the Profile could \
                 not be read",
            );

            return None;
        }
    };

    // Its own rows and nothing it is itself mirroring, which is the reading
    // [`super::account`] makes: the store of a Profile at home on a third machine
    // is that machine's to hand over.
    let held = held.filter(|held| held.mirror.is_none())?;

    // Nothing at all where the switch is off. Answered rather than refused: a
    // Profile whose sessions are given no store has none to lend, which is not
    // the same as a Profile that is not here.
    if !held.memory {
        return Some((PathBuf::new(), Vec::new()));
    }

    let repo = match wanted.repo {
        Some(repo) => match store::load_repo(&state.pool, repo).await {
            Ok(repo) => repo.map(|repo| repo.path),

            Err(why) => {
                tracing::error!(
                    error = ?why,
                    profile,
                    "the Repo a member named of this device's own could not be read",
                );

                None
            }
        },

        None => None,
    };

    // And the path this device would have cut that work in, which is its own
    // worktrees directory with the stem the other machine's carries — a Worktree
    // lives under the Data Directory rather than under the Repo, so there is
    // nothing to match it by and naming it this machine's way is the whole of the
    // rewrite.
    let worktree = wanted
        .worktree
        .as_deref()
        .filter(|stem| {
            Path::new(stem)
                .file_name()
                .is_some_and(|name| name == *stem)
        })
        .map(|stem| crate::worktrees::directory(&state.data_dir).join(stem));

    let platform = crate::platform::Platform::HERE;
    let account = held.account.clone();
    let wanted = wanted.clone();

    let named = tokio::task::spawn_blocking(move || {
        let root = match &account {
            store::Account::Claude { claude_dir, .. } => Root::claude(
                platform,
                claude_dir,
                // The main checkout is what a root reads off a Worktree's own
                // `.git`, so a Repo's path with `.git` on it is the same entry a
                // session here would have named. Nothing where no Repo of this
                // device's is that repository, which is a part `carried` then
                // leaves out.
                &repo.unwrap_or_default().join(".git"),
                worktree.as_deref().unwrap_or(Path::new("")),
            ),

            store::Account::Codex { home } => Root::codex(home),
            store::Account::Grok { home } => Root::grok(home),
            store::Account::OpenCode { home } => Root::opencode(home),
        };

        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            &wanted,
        );

        (root.account().to_owned(), parts)
    })
    .await;

    match named {
        Ok(named) => Some(named),

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile,
                "naming the parts of this device's own memory store ended badly",
            );

            None
        }
    }
}

/// The routes, over the state the workbench answers out of.
///
/// **Built here and mounted by [`super::workbench::served`]**, for the reason
/// [`super::account`]'s are: they answer out of this device's own Profiles and
/// this device's own Repos.
pub(crate) fn route() -> Router<AppState> {
    Router::new()
        .route(
            MEMORY,
            post(held).layer(DefaultBodyLimit::max(MOST_A_QUESTION_IS)),
        )
        .route(
            LEFT,
            post(written).layer(DefaultBodyLimit::max(MOST_A_STORE_IS)),
        )
}

/// The most the question about a store may be: **sixty-four kilobytes**.
///
/// It is two ids and a list of session names, which is some hundreds of bytes for
/// a Conversation that has run all week. What a bound is for is what every bound
/// across a link is for.
const MOST_A_QUESTION_IS: usize = 64 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    /// Both are a member's calls, so they go where a member's calls go rather than
    /// through any of the three the gate stands aside for: what one hands over is
    /// a human's transcripts.
    #[test]
    fn the_memory_is_not_one_of_the_un_gated_three() {
        for path in [MEMORY, LEFT] {
            assert_ne!(path, super::super::IDENTITY);
            assert_ne!(path, super::super::joining::JOIN);
            assert_ne!(path, super::super::exchange::SETTLED);
        }
    }

    /// And both are outside the three prefixes this device keeps to itself, which
    /// is what they have to be: a route served on the one namespace a member is
    /// never served would be a route nothing could ever reach.
    #[test]
    fn the_memory_is_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            for path in [MEMORY, LEFT] {
                assert!(
                    !path.starts_with(prefix),
                    "{path} would be held back with {prefix}/",
                );
            }
        }
    }

    /// And each names the Profile by the id it has on the device that answers,
    /// which is what a mirror row records for it there.
    #[test]
    fn the_memory_names_the_profile() {
        for path in [MEMORY, LEFT] {
            assert!(
                path.contains("{profile}"),
                "the path names the Profile it is about: {path}",
            );
        }
    }

    /// And what the session left is one thing the store holds rather than a route
    /// beside it, the two being the two directions of one thing.
    #[test]
    fn what_was_left_is_under_the_store() {
        assert_eq!(LEFT, format!("{MEMORY}/left"));
    }
}
