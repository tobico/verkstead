//! This device's Repos as a member is told of them: the row, and the `origin`
//! remote's URL git gives for it (ADR-0020, *Repos across devices*).
//!
//! **The answering half of the match.** Which of a member's Repos is this
//! repository is the question a cluster has to settle before anything keyed by a
//! path crosses between machines — the memory sync's *whose entry do I pull*,
//! and transfer's *where does this branch land*. This is what a device answers
//! it out of, and [`crate::matching`] is the end that asks and decides.
//!
//! **A member's own call rather than the viewer's**, so it is here beside
//! [`super::news`] rather than in [`crate::ui::routes`]: no page draws a list of
//! origins, and the one that would — the Repo dropdown — is drawn on a name and
//! a path and is asked for often enough that shelling out to git per row on
//! every load would be a cost paid for nothing. So the reading lives where the
//! one caller is, on the one listener that caller reaches.
//!
//! **Gated to members like everything else on this listener.** What a Repo's
//! origin says is where the human's work is hosted and what their directories
//! are called, which is not a stranger's business: [`super::router`] puts
//! [`super::members_only`] over this, and a caller whose certificate this device
//! holds no membership for never reaches it.
//!
//! **And every origin is read afresh**, off the repository rather than off the
//! registration, which records a path, a name and a default branch and no origin
//! at all — see [`crate::repos::origin`], where that is said and why. Both ends
//! of a match do it, which is what makes the comparison one about two
//! repositories rather than about two rows.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_render::RepoAcross;

use crate::{AppState, store};

/// Where a member asks after this device's Repos.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace and is drawn by a
/// browser; and it names nobody, there being nothing to fill in — a device asks
/// for the lot and matches its own repository against them. The same shape the
/// news and the renewal beside it take.
pub const REPOS: &str = "/api/peer/v1/repos";

/// `GET /api/peer/v1/repos` — every Repo on this device's registry, each with
/// the URL its `origin` remote has right now.
///
/// **The whole registry rather than the one that matches.** The rule that says
/// which of them is the caller's repository is one rule, and it runs on the
/// device that is asking: a device that sent its own origin over for this one to
/// judge would be the same rule written twice, once on each side of a version
/// skew, and two Verksteads that disagreed about a trailing `.git` would
/// disagree about which repository the work is in. So this answers facts and
/// nothing else — see [`crate::matching::across`], which is where the deciding
/// happens.
///
/// **A Repo that has been unregistered is not on it**, this being the registry
/// as every other reading of it sees one: a repository the human has told this
/// device to stop offering is not one to land somebody else's work in.
///
/// The git reads go in one blocking task rather than one apiece: they are
/// `git remote get-url` against a handful of directories, and a match is asked
/// for once before a launch rather than on every page.
pub(crate) async fn held(State(state): State<AppState>) -> Response {
    let repos = match store::registered_repos(&state.pool).await {
        Ok(repos) => repos,

        Err(why) => {
            tracing::error!(
                error = ?why,
                "a member asked after this device's Repos and the registry could not be read",
            );

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own Repos\n",
            )
                .into_response();
        }
    };

    let answered = tokio::task::spawn_blocking(move || {
        repos
            .into_iter()
            .map(|repo| RepoAcross {
                id: repo.id,
                origin: crate::repos::origin(&repo.path),
                // Stored as UTF-8 in the first place — a path that is not cannot
                // be registered — so nothing is lost putting it back on the wire.
                path: repo.path.to_string_lossy().into_owned(),
                name: repo.name,
            })
            .collect::<Vec<_>>()
    })
    .await;

    match answered {
        Ok(answered) => Json(answered).into_response(),

        Err(why) => {
            tracing::error!(
                error = ?why,
                "the Repos a member asked after could not be read off the disk",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own repositories\n",
            )
                .into_response()
        }
    }
}

/// The route, over the state the workbench answers out of.
///
/// **Built here and mounted by [`super::workbench::served`]**, which is where
/// the rest of what a member reaches is built and where the news beside it is
/// mounted: it answers out of this device's own registry, and a router over a
/// second state would be answering for a second device's Repos.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(REPOS, get(held))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It is a member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for: what a repository's
    /// origin says about somebody's work is not a stranger's business.
    #[test]
    fn the_repos_are_not_one_of_the_un_gated_three() {
        assert_ne!(REPOS, super::super::IDENTITY);
        assert_ne!(REPOS, super::super::joining::JOIN);
        assert_ne!(REPOS, super::super::exchange::SETTLED);
    }

    /// And it names no device: a device asks for the whole registry and matches
    /// its own repository against it, so there is nothing for a caller to fill
    /// in.
    #[test]
    fn the_repos_name_no_device() {
        assert!(
            !REPOS.contains('{'),
            "the Repos path carries no segment somebody fills in: {REPOS}",
        );
    }

    /// And it is outside the prefixes this device keeps to itself, which is
    /// what it has to be: a reading served on the one namespace a member is never
    /// served would be a route nothing could ever reach.
    #[test]
    fn the_repos_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !REPOS.starts_with(prefix),
                "the Repos would be held back with {prefix}/",
            );
        }
    }
}
